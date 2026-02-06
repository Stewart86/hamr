//! Plugin dispatch methods for `HamrCore`.
//!
//! Handles sending messages to plugins (initial, search, action, slider, switch)
//! and managing plugin communication lifecycle.

use super::{HamrCore, ID_DISMISS, InputMode, generate_session_id, process};
use crate::Error;
use crate::plugin::{PluginInput, PluginProcess, SelectedItem, Step};
use hamr_types::CoreUpdate;
use tracing::{debug, error, info};

impl HamrCore {
    /// Start a background daemon for a plugin.
    pub(super) fn start_daemon(&mut self, plugin_id: &str) -> crate::Result<()> {
        let plugin = self
            .plugins
            .get(plugin_id)
            .ok_or_else(|| Error::PluginNotFound(plugin_id.to_string()))?;

        // Skip stdio communication for socket plugins (they're handled by the daemon)
        if plugin.is_socket() {
            debug!("Skipping stdio spawn for socket plugin: {}", plugin_id);
            return Ok(());
        }

        let mut process = PluginProcess::spawn(plugin_id, &plugin.handler_path, &plugin.path)?;

        let sender = process.sender();
        let receiver = process.take_receiver().expect("Receiver should exist");

        self.daemons
            .insert(plugin_id.to_string(), (process, sender));

        process::spawn_response_listener(plugin_id.to_string(), receiver, self.update_tx.clone());

        info!("Started daemon for plugin: {}", plugin_id);
        Ok(())
    }

    /// Activate a plugin for multi-step flow (called when plugin response has activate: true)
    /// Unlike `handle_open_plugin`, this doesn't send initial step - the plugin already responded
    pub fn activate_plugin_for_multistep(&mut self, plugin_id: &str) {
        let Some(plugin) = self.plugins.get(plugin_id) else {
            tracing::warn!("Cannot activate unknown plugin: {}", plugin_id);
            return;
        };

        // Don't activate if already active
        if self.state.active_plugin.is_some() {
            debug!(
                "Plugin already active, skipping activation for {}",
                plugin_id
            );
            return;
        }

        let session = generate_session_id();

        self.state.active_plugin = Some(super::ActivePlugin {
            id: plugin_id.to_string(),
            name: plugin.manifest.name.clone(),
            icon: plugin.manifest.icon.clone(),
            session,
            last_selected_item: None,
            context: None,
        });
        self.state.navigation_depth = 0;

        // Determine input mode
        self.state.input_mode = match plugin.manifest.input_mode {
            Some(crate::plugin::InputMode::Submit) => InputMode::Submit,
            _ => InputMode::Realtime,
        };

        debug!("Activated plugin {} for multi-step flow", plugin_id);

        self.send_update(CoreUpdate::PluginActivated {
            id: plugin_id.to_string(),
            name: plugin.manifest.name.clone(),
            icon: plugin.manifest.icon.clone(),
        });
    }

    /// Set the context for the active plugin
    /// Called by daemon when `ContextChanged` comes from plugin response
    pub fn set_plugin_context(&mut self, context: Option<String>) {
        if let Some(ref mut active) = self.state.active_plugin {
            active.context = context;
        }
    }

    /// Send the initial step to a plugin.
    pub(super) async fn send_plugin_initial(&mut self, plugin_id: &str, session: &str) {
        let input = PluginInput {
            step: Step::Initial,
            query: None,
            selected: None,
            action: None,
            session: Some(session.to_string()),
            context: None,
            value: None,
            form_data: None,
            source: None,
        };

        self.send_to_plugin(plugin_id, &input).await;
    }

    /// Send a search query to the active plugin.
    pub(super) async fn send_plugin_search(&mut self, query: &str) {
        let Some(ref active) = self.state.active_plugin else {
            return;
        };

        let plugin_id = active.id.clone();
        let session = active.session.clone();

        let input = PluginInput {
            step: Step::Search,
            query: Some(query.to_string()),
            selected: active.last_selected_item.as_ref().map(|id| SelectedItem {
                id: id.clone(),
                extra: None,
            }),
            action: None,
            session: Some(session),
            context: active.context.clone(),
            value: None,
            form_data: None,
            source: None,
        };

        self.send_update(CoreUpdate::Busy { busy: true });
        self.send_to_plugin(&plugin_id, &input).await;
    }

    /// Send an action to the active plugin.
    pub(super) async fn send_plugin_action(&mut self, item_id: &str, action: Option<String>) {
        let Some(ref mut active) = self.state.active_plugin else {
            return;
        };

        active.last_selected_item = Some(item_id.to_string());
        let plugin_id = active.id.clone();
        let session = active.session.clone();
        let context = active.context.clone();

        let input = PluginInput {
            step: Step::Action,
            query: Some(self.state.query.clone()),
            selected: Some(SelectedItem {
                id: item_id.to_string(),
                extra: None,
            }),
            action,
            session: Some(session),
            context,
            value: None,
            form_data: None,
            source: None,
        };

        // Note: navigation_depth is NOT incremented here
        // It's managed by the TUI based on NavigateForward updates from plugins
        // (Plugins signal forward navigation via navigate_forward: true in response)
        self.send_update(CoreUpdate::Busy { busy: true });
        self.send_to_plugin(&plugin_id, &input).await;
    }

    /// Send a slider value change to a plugin.
    pub(super) async fn send_plugin_slider_change(
        &mut self,
        plugin_id: &str,
        item_id: &str,
        value: f64,
    ) {
        if let Some(plugin) = self.plugins.get(plugin_id)
            && plugin.is_daemon()
            && !self.daemons.contains_key(plugin_id)
            && let Err(e) = self.start_daemon(plugin_id)
        {
            error!("Failed to start daemon for {}: {}", plugin_id, e);
            return;
        }

        let session = self
            .state
            .active_plugin
            .as_ref()
            .map_or_else(generate_session_id, |p| p.session.clone());

        let input = PluginInput {
            step: Step::Action,
            query: None,
            selected: Some(SelectedItem {
                id: item_id.to_string(),
                extra: None,
            }),
            action: Some("slider".to_string()),
            session: Some(session),
            context: None,
            value: Some(value),
            form_data: None,
            source: None,
        };

        self.send_to_plugin(plugin_id, &input).await;
    }

    /// Send a switch toggle to a plugin.
    pub(super) async fn send_plugin_switch_toggle(
        &mut self,
        plugin_id: &str,
        item_id: &str,
        value: bool,
    ) {
        if let Some(plugin) = self.plugins.get(plugin_id)
            && plugin.is_daemon()
            && !self.daemons.contains_key(plugin_id)
            && let Err(e) = self.start_daemon(plugin_id)
        {
            error!("Failed to start daemon for {}: {}", plugin_id, e);
            return;
        }

        let session = self
            .state
            .active_plugin
            .as_ref()
            .map_or_else(generate_session_id, |p| p.session.clone());

        let input = PluginInput {
            step: Step::Action,
            query: None,
            selected: Some(SelectedItem {
                id: item_id.to_string(),
                extra: None,
            }),
            action: Some("switch".to_string()),
            session: Some(session),
            context: None,
            value: Some(if value { 1.0 } else { 0.0 }),
            form_data: None,
            source: None,
        };

        self.send_to_plugin(plugin_id, &input).await;
    }

    /// Handle an ambient action (triggered from ambient mode, e.g., notifications).
    pub(super) async fn handle_ambient_action(
        &mut self,
        plugin_id: String,
        item_id: String,
        action: Option<String>,
    ) {
        let session = generate_session_id();

        let input = PluginInput {
            step: Step::Action,
            query: None,
            selected: Some(SelectedItem {
                id: item_id,
                extra: None,
            }),
            action,
            session: Some(session),
            context: None,
            value: None,
            form_data: None,
            source: Some("ambient".to_string()),
        };

        self.send_update(CoreUpdate::Busy { busy: true });
        self.send_to_plugin(&plugin_id, &input).await;
    }

    /// Handle dismissing an ambient item.
    pub(super) async fn handle_dismiss_ambient(&mut self, plugin_id: String, item_id: String) {
        let session = generate_session_id();

        let input = PluginInput {
            step: Step::Action,
            query: None,
            selected: Some(SelectedItem {
                id: item_id,
                extra: None,
            }),
            action: Some(ID_DISMISS.to_string()),
            session: Some(session),
            context: None,
            value: None,
            form_data: None,
            source: Some("ambient".to_string()),
        };

        self.send_to_plugin(&plugin_id, &input).await;
    }

    /// Send a replay action to a plugin (one-shot execution, no UI).
    /// Used when executing indexed items from main search.
    pub(super) async fn send_replay_action(
        &mut self,
        plugin_id: &str,
        item_id: &str,
        action: Option<String>,
    ) {
        let session = generate_session_id();

        let input = PluginInput {
            step: Step::Action,
            query: None,
            selected: Some(SelectedItem {
                id: item_id.to_string(),
                extra: None,
            }),
            action,
            session: Some(session),
            context: None,
            value: None,
            form_data: None,
            source: None,
        };

        self.send_update(CoreUpdate::Busy { busy: true });
        self.send_to_plugin(plugin_id, &input).await;
    }

    /// Send input to a plugin, handling daemon vs on-demand spawning.
    pub(super) async fn send_to_plugin(&mut self, plugin_id: &str, input: &PluginInput) {
        if let Some((_, sender)) = self.daemons.get(plugin_id) {
            if let Err(e) = sender.send(input).await {
                error!("Failed to send to daemon {}: {}", plugin_id, e);
            }
            return;
        }

        if let Some(plugin) = self.plugins.get(plugin_id)
            && plugin.is_socket()
        {
            debug!("Skipping stdio send for socket plugin: {}", plugin_id);
            return;
        }

        if let Some((mut process, _)) = self.active_process.take() {
            let _ = process.kill().await;
        }

        if let Some(plugin) = self.plugins.get(plugin_id) {
            match PluginProcess::spawn(plugin_id, &plugin.handler_path, &plugin.path) {
                Ok(mut process) => {
                    if let Err(e) = process.send_and_close(input).await {
                        error!("Failed to send to plugin {}: {}", plugin_id, e);
                        return;
                    }

                    if let Some(receiver) = process.take_receiver() {
                        process::spawn_response_listener(
                            plugin_id.to_string(),
                            receiver,
                            self.update_tx.clone(),
                        );
                    }

                    let sender = process.sender();
                    self.active_process = Some((process, sender));
                }
                Err(e) => {
                    error!("Failed to spawn plugin {}: {}", plugin_id, e);
                    self.send_update(CoreUpdate::Error {
                        message: format!("Failed to start plugin '{plugin_id}': {e}"),
                    });
                }
            }
        }
    }
}
