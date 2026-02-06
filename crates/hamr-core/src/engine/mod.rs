mod plugins;
mod process;
mod suggestions;

use crate::Result;
use crate::config::{Config, Directories};
use crate::frecency::{ExecutionContext, FrecencyScorer, MatchType};
use crate::index::IndexStore;
use crate::plugin::{
    FrecencyMode, PluginInput, PluginManager, PluginProcess, PluginResponse, PluginSender,
    SelectedItem, Step, invoke_match,
};
use crate::search::{SearchEngine, SearchMatch, Searchable, SearchableSource};
use crate::utils::now_millis;
use hamr_types::{CoreEvent, CoreUpdate, ResultType, SearchResult};
use std::collections::HashMap;
use std::path::Path;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tracing::{debug, error, info, warn};

pub(crate) const ID_PLUGIN_ENTRY: &str = "__plugin__";
pub(crate) const ID_BACK: &str = "__back__";
pub(crate) const ID_FORM_CANCEL: &str = "__form_cancel__";
pub(crate) const PREFIX_PATTERN_MATCH: &str = "__pattern_match__:";
pub(crate) const PREFIX_MATCH_PREVIEW: &str = "__match_preview__:";
pub(crate) const ID_DISMISS: &str = "__dismiss__";

pub(crate) const DEFAULT_PLUGIN_ICON: &str = "extension";
pub(crate) const DEFAULT_VERB_OPEN: &str = "Open";
pub(crate) const DEFAULT_VERB_SELECT: &str = "Select";
pub(crate) const DEFAULT_ICON_TYPE: &str = "material";

/// Core hamr engine
pub struct HamrCore {
    dirs: Directories,
    config: Config,
    plugins: PluginManager,
    index: IndexStore,
    search: SearchEngine,
    state: LauncherState,

    /// Running daemon processes (kept alive) and their senders
    daemons: HashMap<String, (PluginProcess, PluginSender)>,

    /// Active plugin process (non-daemon) and sender
    active_process: Option<(PluginProcess, PluginSender)>,

    /// Channel to send updates to UI
    update_tx: UnboundedSender<CoreUpdate>,

    /// Throttle state for continuous control recording (sliders, switches)
    control_throttle: ControlThrottle,
}

/// Throttle state for recording continuous control interactions
/// Only records once per control until 2 seconds of inactivity
#[derive(Debug, Default)]
struct ControlThrottle {
    /// Key of last recorded control: `plugin_id/item_id`
    last_control_key: Option<String>,
    /// Timestamp (ms) when last control was recorded
    last_record_time: u64,
}

/// Idle threshold for control recording (2 seconds)
const CONTROL_IDLE_THRESHOLD_MS: u64 = 2000;

/// State restore window in milliseconds (5 seconds)
const STATE_RESTORE_WINDOW_MS: u128 = 5000;

/// Current launcher state
#[derive(Debug, Clone, Default)]
pub struct LauncherState {
    /// Whether launcher is open
    pub is_open: bool,

    /// Current search query
    pub query: String,

    /// Currently active plugin
    pub active_plugin: Option<ActivePlugin>,

    /// Navigation depth in plugin
    pub navigation_depth: u32,

    /// Current input mode
    pub input_mode: InputMode,

    /// Whether we're busy waiting for plugin response
    pub busy: bool,

    /// Timestamp when launcher was last closed (for state restore)
    pub last_close_time: Option<std::time::Instant>,

    /// Cached results for state restoration
    pub last_results: Vec<SearchResult>,

    /// Cached placeholder for state restoration
    pub last_placeholder: Option<String>,

    /// Cached context for state restoration
    pub last_context: Option<String>,

    /// Pre-built recent/suggestions list (rebuilt on launcher close)
    pub cached_recent: Vec<SearchResult>,

    /// Whether we're in plugin management mode (showing only plugins via "/" prefix)
    pub plugin_management: bool,

    /// Pending initial query to send to plugin after it opens
    /// Used to handle the race between `ClearInput` and the next `query_changed`
    pub pending_initial_query: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ActivePlugin {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub session: String,
    pub last_selected_item: Option<String>,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    Realtime,
    Submit,
}

impl HamrCore {
    /// Create a new `HamrCore` instance with a channel for updates.
    /// Returns the core and a receiver for updates.
    ///
    /// # Errors
    ///
    /// Returns an error if directories cannot be created, config fails to load,
    /// or plugin discovery fails.
    pub fn new() -> Result<(Self, UnboundedReceiver<CoreUpdate>)> {
        let dirs = Directories::new()?;
        dirs.ensure_exists()?;

        let config = Config::load(&dirs.config_file)?;
        let mut plugins = PluginManager::new(&dirs);
        plugins.discover()?;

        debug!("Loading index from {}", dirs.index_cache.display());
        let index = match IndexStore::load(&dirs.index_cache) {
            Ok(store) => {
                debug!("Index loaded successfully");
                store
            }
            Err(e) => {
                warn!("Failed to load index: {}", e);
                IndexStore::default()
            }
        };
        let search = SearchEngine::new();

        let (update_tx, update_rx) = mpsc::unbounded_channel();

        Ok((
            Self {
                dirs,
                config,
                plugins,
                index,
                search,
                state: LauncherState::default(),
                daemons: HashMap::new(),
                active_process: None,
                update_tx,
                control_throttle: ControlThrottle::default(),
            },
            update_rx,
        ))
    }

    /// Initialize and start background daemons.
    ///
    /// # Errors
    ///
    /// Returns an error if the core fails to initialize or start daemons.
    pub fn start(&mut self) -> Result<()> {
        info!("Starting hamr core...");

        // Collect daemon plugin IDs first to avoid borrow conflict
        let daemon_ids: Vec<String> = self
            .plugins
            .background_daemons()
            .map(|p| p.id.clone())
            .collect();

        for plugin_id in daemon_ids {
            if let Err(e) = self.start_daemon(&plugin_id) {
                warn!("Failed to start daemon for {}: {}", plugin_id, e);
            }
        }

        self.load_static_indexes();

        info!("Hamr core started");
        Ok(())
    }

    fn try_late_platform_init(&mut self) {
        if let Ok(true) = self.plugins.retry_platform_detection() {
            let ids: Vec<_> = self
                .plugins
                .all()
                .filter(|p| p.is_daemon() && p.is_background_daemon())
                .map(|p| p.id.clone())
                .collect();

            for id in ids {
                if !self.daemons.contains_key(&id)
                    && let Err(e) = self.start_daemon(&id)
                {
                    warn!("Failed to start daemon for {}: {}", id, e);
                }
            }
        }
    }

    /// Process an event - updates are sent via channel
    pub async fn process(&mut self, event: CoreEvent) {
        self.try_late_platform_init();

        match event {
            CoreEvent::QueryChanged { query } => {
                self.handle_query_changed(query).await;
            }
            CoreEvent::QuerySubmitted { query, context } => {
                if let Some(active) = &mut self.state.active_plugin
                    && context.is_some()
                {
                    active.context.clone_from(&context);
                }
                self.handle_query_submitted(query).await;
            }
            CoreEvent::ItemSelected {
                id,
                action,
                plugin_id,
            } => {
                self.handle_item_selected(id, action, plugin_id).await;
            }
            CoreEvent::AmbientAction {
                plugin_id,
                item_id,
                action,
            } => {
                self.handle_ambient_action(plugin_id, item_id, action).await;
            }
            CoreEvent::DismissAmbient { plugin_id, item_id } => {
                self.handle_dismiss_ambient(plugin_id, item_id).await;
            }
            CoreEvent::SliderChanged {
                id,
                value,
                plugin_id,
            } => {
                self.handle_slider_changed(id, value, plugin_id).await;
            }
            CoreEvent::SwitchToggled {
                id,
                value,
                plugin_id,
            } => {
                self.handle_switch_toggled(id, value, plugin_id).await;
            }
            CoreEvent::Back => {
                self.handle_back().await;
            }
            CoreEvent::Cancel => {
                self.handle_cancel().await;
            }
            CoreEvent::OpenPlugin { plugin_id } => {
                self.handle_open_plugin(plugin_id).await;
            }
            CoreEvent::ClosePlugin => {
                self.handle_close_plugin().await;
            }
            CoreEvent::LauncherOpened => {
                self.handle_launcher_opened().await;
            }
            CoreEvent::LauncherClosed => {
                self.handle_launcher_closed();
            }
            CoreEvent::RefreshIndex { plugin_id } => {
                debug!("Refresh index requested for {}", plugin_id);
            }
            CoreEvent::FormSubmitted { form_data, context } => {
                self.handle_form_submitted(form_data, context).await;
            }
            CoreEvent::FormCancelled => {
                self.handle_form_cancelled().await;
            }
            CoreEvent::SetContext { context } => {
                if let Some(ref mut active) = self.state.active_plugin {
                    active.context = context;
                }
            }
            CoreEvent::FormFieldChanged {
                field_id,
                value,
                form_data,
                context,
            } => {
                self.handle_form_field_changed(field_id, value, form_data, context)
                    .await;
            }
            CoreEvent::PluginActionTriggered { action_id } => {
                self.handle_plugin_action(action_id).await;
            }
        }
    }

    fn serialize_form_data(form_data: &HashMap<String, String>) -> serde_json::Value {
        match serde_json::to_value(form_data) {
            Ok(v) => v,
            Err(e) => {
                warn!("Failed to serialize form data: {e}");
                serde_json::Value::default()
            }
        }
    }

    fn active_plugin_info(&self) -> Option<(String, String)> {
        self.state
            .active_plugin
            .as_ref()
            .map(|a| (a.id.clone(), a.session.clone()))
    }

    /// Send an update to the UI
    fn send_update(&self, update: CoreUpdate) {
        if let Err(e) = self.update_tx.send(update) {
            error!("Failed to send update: {}", e);
        }
    }

    /// Send an update and cache it for state restoration
    fn send_update_cached(&mut self, update: CoreUpdate) {
        match &update {
            CoreUpdate::Results { results, .. } => {
                self.state.last_results.clone_from(results);
            }
            CoreUpdate::Placeholder { placeholder } => {
                self.state.last_placeholder = Some(placeholder.clone());
            }
            CoreUpdate::ContextChanged { context } => {
                self.state.last_context.clone_from(context);
                // Also update active plugin's context so ID_BACK uses the correct context
                if let Some(ref mut active) = self.state.active_plugin {
                    active.context.clone_from(context);
                }
            }
            _ => {}
        }
        self.send_update(update);
    }

    async fn handle_query_changed(&mut self, query: String) {
        debug!(
            "handle_query_changed: query='{}', active_plugin={:?}",
            query,
            self.state.active_plugin.as_ref().map(|p| &p.id)
        );

        // Implicitly mark launcher as open when receiving queries
        // This handles cases where UI sends query_changed without explicit launcher_opened
        if !self.state.is_open {
            debug!("Implicitly marking launcher as open (received query_changed)");
            self.state.is_open = true;
        }

        self.state.query.clone_from(&query);

        // Handle pending initial query from pattern match
        // This prevents the empty query (from ClearInput) from overwriting the actual query
        if self.state.active_plugin.is_some() {
            let query_to_use = if query.is_empty() && self.state.pending_initial_query.is_some() {
                let pending = self.state.pending_initial_query.take();
                debug!("Using pending initial query: {:?}", pending);
                pending.unwrap_or_default()
            } else {
                query
            };

            self.state.query.clone_from(&query_to_use);

            if self.state.input_mode == InputMode::Realtime {
                self.send_plugin_search(&query_to_use).await;
            }
        } else if self.state.plugin_management {
            if query.is_empty() {
                let results = self.get_plugin_list();
                self.send_update_cached(CoreUpdate::Results {
                    results,
                    placeholder: Some("Search plugins...".to_string()),
                    clear_input: None,
                    input_mode: None,
                    context: None,
                    navigate_forward: None,
                    display_hint: None,
                });
            } else {
                let results = self.perform_main_search(&query).await;
                self.send_update_cached(CoreUpdate::Results {
                    results,
                    placeholder: Some("Search plugins...".to_string()),
                    clear_input: None,
                    input_mode: None,
                    context: None,
                    navigate_forward: None,
                    display_hint: None,
                });
            }
        } else {
            // Reserved "/" prefix for plugin list mode (non-configurable)
            if query == "/" {
                debug!("Entering plugin list mode via '/' prefix");
                self.enter_plugin_management().await;
                return;
            }

            if let Some(plugin_id) = self.find_action_bar_hint_match(&query) {
                debug!(
                    "Action bar hint match: '{}' -> plugin '{}'",
                    query, plugin_id
                );
                self.record_plugin_open(&plugin_id);
                self.handle_open_plugin(plugin_id).await;
                self.send_update(CoreUpdate::ClearInput);
                return;
            }

            if let Some((plugin, remaining)) = self.plugins.find_matching(&query)
                && remaining.is_empty()
            {
                let plugin_id = plugin.id.clone();
                debug!(
                    "Plugin prefix exact match: '{}' -> plugin '{}'",
                    query, plugin_id
                );
                self.record_plugin_open(&plugin_id);
                self.handle_open_plugin(plugin_id).await;
                self.send_update(CoreUpdate::ClearInput);
                return;
            }

            let results = self.perform_main_search(&query).await;
            debug!("handle_query_changed: produced {} results", results.len());
            self.send_update_cached(CoreUpdate::results(results));
        }
    }

    /// Find plugin ID from `action_bar_hints` that matches the query exactly
    fn find_action_bar_hint_match(&self, query: &str) -> Option<String> {
        for hint in self.config.action_bar_hints() {
            if query == hint.prefix {
                return Some(hint.plugin.clone());
            }
        }
        None
    }

    /// Enter plugin management mode (shows only plugins, triggered by "/" prefix)
    async fn enter_plugin_management(&mut self) {
        self.state.plugin_management = true;
        self.state.query.clear();
        self.send_update(CoreUpdate::PluginManagementChanged { active: true });
        self.send_update(CoreUpdate::ClearInput);

        // Perform search filtered to plugins only
        let results = self.perform_main_search("").await;
        self.send_update_cached(CoreUpdate::Results {
            results,
            placeholder: Some("Search plugins...".to_string()),
            clear_input: None,
            input_mode: None,
            context: None,
            navigate_forward: None,
            display_hint: None,
        });
    }

    /// Exit plugin management mode
    fn exit_plugin_management(&mut self) {
        if self.state.plugin_management {
            debug!("Exiting plugin list mode");
            self.state.plugin_management = false;
            self.send_update(CoreUpdate::PluginManagementChanged { active: false });
        }
    }

    async fn handle_query_submitted(&mut self, query: String) {
        // TUI only sends QuerySubmitted when in submit mode, so just check for active plugin
        if self.state.active_plugin.is_some() {
            self.send_plugin_search(&query).await;
        }
    }

    /// Get a plugin's frecency mode from its manifest.
    fn get_frecency_mode(&self, plugin_id: &str) -> Option<FrecencyMode> {
        self.plugins
            .get(plugin_id)
            .and_then(|p| p.manifest.frecency.clone())
    }

    /// Build an `ExecutionContext` from the current query state.
    fn build_execution_context(&self) -> ExecutionContext {
        ExecutionContext {
            search_term: if self.state.query.is_empty() {
                None
            } else {
                Some(self.state.query.clone())
            },
            launch_from_empty: self.state.query.is_empty(),
            ..Default::default()
        }
    }

    async fn handle_item_selected(
        &mut self,
        id: String,
        action: Option<String>,
        event_plugin_id: Option<String>,
    ) {
        // Implicitly mark launcher as open when receiving item selections
        if !self.state.is_open {
            debug!("Implicitly marking launcher as open (received item_selected)");
            self.state.is_open = true;
        }

        if let Some(ref active_plugin) = self.state.active_plugin {
            self.handle_active_plugin_item_selected(active_plugin.id.clone(), id, action)
                .await;
            return;
        }

        if self.plugins.get(&id).is_some() {
            self.record_plugin_open(&id);
            self.handle_open_plugin(id).await;
            return;
        }

        // Handle __plugin__ entries (smart suggestions/recent items that represent opening a plugin)
        if id == ID_PLUGIN_ENTRY {
            if let Some(plugin_id) = event_plugin_id
                && self.plugins.get(&plugin_id).is_some()
            {
                self.record_plugin_open(&plugin_id);
                self.handle_open_plugin(plugin_id).await;
                return;
            }
            debug!("{} entry without valid plugin_id", ID_PLUGIN_ENTRY);
            return;
        }

        // Handle __pattern_match__ entries (prefix-triggered plugin activation)
        if let Some(plugin_id) = id.strip_prefix(PREFIX_PATTERN_MATCH) {
            if self.plugins.get(plugin_id).is_some() {
                self.record_plugin_open(plugin_id);

                // Re-compute the remaining query by calling find_matching with current query
                // This ensures we get the prefix-stripped query to pass to the plugin
                let remaining_query = self
                    .plugins
                    .find_matching(&self.state.query)
                    .map(|(_, remaining)| remaining)
                    .unwrap_or_default();

                if remaining_query.is_empty() {
                    self.handle_open_plugin(plugin_id.to_string()).await;
                    self.send_update(CoreUpdate::ClearInput);
                } else {
                    // Open plugin with the remaining query (prefix stripped)
                    self.handle_open_plugin_with_query(plugin_id.to_string(), remaining_query)
                        .await;
                }
                return;
            }
            debug!(
                "__pattern_match__ entry with invalid plugin_id: {}",
                plugin_id
            );
            return;
        }

        if self
            .handle_indexed_item_selected(&id, action, event_plugin_id.as_deref())
            .await
        {
            return;
        }

        debug!("Item not found: {}", id);
    }

    /// Handle item selection when there's an active plugin.
    async fn handle_active_plugin_item_selected(
        &mut self,
        plugin_id: String,
        id: String,
        action: Option<String>,
    ) {
        let frecency_mode = self.get_frecency_mode(&plugin_id);
        let context = self.build_execution_context();

        let fallback_item = self.state.last_results.iter().find(|r| r.id == id).cloned();

        self.index.record_execution_with_item(
            &plugin_id,
            &id,
            &context,
            frecency_mode.as_ref(),
            fallback_item.as_ref(),
        );

        if let Some(item) = self.index.get_item_mut(&plugin_id, &id) {
            let entry_point = serde_json::json!({
                "step": "action",
                "selected": { "id": id },
                "action": action,
            });
            item.item.entry_point = Some(entry_point);
        }

        self.invalidate_recent_cache();
        self.send_plugin_action(&id, action).await;
    }

    /// Handle selection of an indexed item (from recent/suggestions).
    /// Returns true if an indexed item was found and handled.
    async fn handle_indexed_item_selected(
        &mut self,
        id: &str,
        action: Option<String>,
        event_plugin_id: Option<&str>,
    ) -> bool {
        let found = if let Some(pid) = event_plugin_id {
            self.index
                .get_item(pid, id)
                .map(|item| (pid.to_string(), item.item.entry_point.clone()))
        } else {
            self.find_indexed_item(id)
        };

        let Some((plugin_id, entry_point)) = found else {
            return false;
        };

        let frecency_mode = self.get_frecency_mode(&plugin_id);
        let context = self.build_execution_context();
        self.index
            .record_execution(&plugin_id, id, &context, frecency_mode.as_ref());
        self.invalidate_recent_cache();

        if let Some(entry_point) = entry_point {
            self.replay_from_entry_point(&plugin_id, &entry_point, action)
                .await;
            return true;
        }

        debug!(
            "No entry_point for indexed item {}, using item id as fallback",
            id
        );
        self.send_replay_action(&plugin_id, id, action).await;
        true
    }

    /// Find an indexed item by searching all plugins.
    fn find_indexed_item(&self, id: &str) -> Option<(String, Option<serde_json::Value>)> {
        for plugin_id in self.index.plugin_ids() {
            if let Some(item) = self.index.get_item(plugin_id, id) {
                return Some((plugin_id.to_string(), item.item.entry_point.clone()));
            }
        }
        None
    }

    /// Replay an action from a stored `entry_point`.
    async fn replay_from_entry_point(
        &mut self,
        plugin_id: &str,
        entry_point: &serde_json::Value,
        action: Option<String>,
    ) {
        let Some(obj) = entry_point.as_object() else {
            return;
        };

        let step = obj.get("step").and_then(|v| v.as_str());
        let selected = obj.get("selected");
        let stored_action = obj.get("action").and_then(|v| v.as_str());
        let effective_action = action.as_deref().or(stored_action);

        if step == Some("action")
            && let Some(sel) = selected
            && let Some(sel_id) = sel.get("id").and_then(|v| v.as_str())
        {
            self.send_replay_action(
                plugin_id,
                sel_id,
                effective_action.map(std::string::ToString::to_string),
            )
            .await;
        }
    }

    async fn handle_slider_changed(&mut self, id: String, value: f64, plugin_id: Option<String>) {
        let plugin_id =
            plugin_id.or_else(|| self.state.active_plugin.as_ref().map(|p| p.id.clone()));

        if let Some(ref plugin_id) = plugin_id {
            self.record_control_execution(plugin_id, &id);
            self.send_plugin_slider_change(plugin_id, &id, value).await;
        }
    }

    async fn handle_switch_toggled(&mut self, id: String, value: bool, plugin_id: Option<String>) {
        let plugin_id =
            plugin_id.or_else(|| self.state.active_plugin.as_ref().map(|p| p.id.clone()));

        if let Some(ref plugin_id) = plugin_id {
            self.record_control_execution(plugin_id, &id);
            self.send_plugin_switch_toggle(plugin_id, &id, value).await;
        }
    }

    /// Record a continuous control (slider/switch) execution with throttling.
    /// Only records once per control until `CONTROL_IDLE_THRESHOLD_MS` of inactivity.
    fn record_control_execution(&mut self, plugin_id: &str, item_id: &str) {
        let control_key = format!("{plugin_id}/{item_id}");
        let now = now_millis();

        let should_record = match &self.control_throttle.last_control_key {
            Some(last_key) if last_key == &control_key => {
                now.saturating_sub(self.control_throttle.last_record_time)
                    >= CONTROL_IDLE_THRESHOLD_MS
            }
            _ => true,
        };

        if should_record {
            let frecency_mode = self.get_frecency_mode(plugin_id);
            let context = ExecutionContext::default();

            let fallback_item = self
                .state
                .last_results
                .iter()
                .find(|r| r.id == item_id)
                .cloned();

            self.index.record_execution_with_item(
                plugin_id,
                item_id,
                &context,
                frecency_mode.as_ref(),
                fallback_item.as_ref(),
            );

            self.invalidate_recent_cache();

            self.control_throttle.last_control_key = Some(control_key);
            self.control_throttle.last_record_time = now;
        }
    }

    /// Record opening a plugin (for frecency tracking).
    /// Only records for plugins with frecency: "plugin" mode.
    /// Item-level plugins track frecency on individual items instead.
    fn record_plugin_open(&mut self, plugin_id: &str) {
        let frecency_mode = self
            .plugins
            .get(plugin_id)
            .and_then(|p| p.manifest.frecency.as_ref());

        // Item-level plugins track frecency on individual items
        if !matches!(frecency_mode, Some(crate::plugin::FrecencyMode::Plugin)) {
            return;
        }

        let context = self.build_execution_context();

        self.index.record_execution(
            plugin_id,
            ID_PLUGIN_ENTRY,
            &context,
            Some(&FrecencyMode::Plugin),
        );

        self.invalidate_recent_cache();
    }

    async fn handle_back(&mut self) {
        if self.state.plugin_management {
            self.exit_plugin_management();
            self.state.query.clear();
            let results = self.perform_main_search("").await;
            self.send_update_cached(CoreUpdate::results(results));
            return;
        }

        if self.state.active_plugin.is_some() {
            self.send_plugin_action(ID_BACK, None).await;
        }
    }

    async fn handle_launcher_opened(&mut self) {
        self.state.is_open = true;

        // Check if we should restore state (within time window and has state)
        let within_window = self.state.last_close_time.is_some_and(|t| {
            let elapsed = t.elapsed().as_millis();
            debug!(
                "Time since close: {}ms (window: {}ms)",
                elapsed, STATE_RESTORE_WINDOW_MS
            );
            elapsed < STATE_RESTORE_WINDOW_MS
        });
        let has_state = self.has_restorable_state();
        debug!(
            "Restore check: within_window={}, has_state={}, active_plugin={:?}, query='{}', last_results={}, last_context={:?}",
            within_window,
            has_state,
            self.state.active_plugin.as_ref().map(|p| &p.id),
            self.state.query,
            self.state.last_results.len(),
            self.state.last_context
        );

        if within_window && has_state {
            debug!("Restoring launcher state");
            self.resend_current_state();
        } else {
            debug!("Fresh launcher start");
            self.handle_close_plugin().await;
            self.state.query.clear();
            self.state.last_results.clear();
            self.state.last_placeholder = None;
            self.state.last_context = None;
        }

        self.state.last_close_time = None;

        // Always send Show to tell UI to become visible
        self.send_update(CoreUpdate::Show);
    }

    fn handle_launcher_closed(&mut self) {
        self.state.is_open = false;
        self.exit_plugin_management();
        self.state.last_close_time = Some(std::time::Instant::now());
        // Rebuild recent list in background so it's ready for next open
        self.rebuild_recent_cache();
        self.send_update(CoreUpdate::Close);
    }

    async fn handle_cancel(&mut self) {
        self.exit_plugin_management();
        self.handle_close_plugin().await;
        self.send_update(CoreUpdate::Close);
    }

    async fn handle_open_plugin(&mut self, id: String) {
        self.exit_plugin_management();

        let Some(plugin) = self.plugins.get(&id) else {
            self.send_update(CoreUpdate::Error {
                message: format!("Plugin not found: {id}"),
            });
            return;
        };

        let session = generate_session_id();

        self.state.active_plugin = Some(ActivePlugin {
            id: id.clone(),
            name: plugin.manifest.name.clone(),
            icon: plugin.manifest.icon.clone(),
            session: session.clone(),
            last_selected_item: None,
            context: None,
        });
        self.state.navigation_depth = 0;
        self.state.query.clear();

        self.state.input_mode = match plugin.manifest.input_mode {
            Some(crate::plugin::InputMode::Submit) => InputMode::Submit,
            _ => InputMode::Realtime,
        };

        self.send_update(CoreUpdate::PluginActivated {
            id: id.clone(),
            name: plugin.manifest.name.clone(),
            icon: plugin.manifest.icon.clone(),
        });

        if plugin.is_daemon()
            && !self.daemons.contains_key(&id)
            && let Err(e) = self.start_daemon(&id)
        {
            self.send_update(CoreUpdate::Error {
                message: format!("Failed to start plugin: {e}"),
            });
            return;
        }
        self.send_plugin_initial(&id, &session).await;
    }

    /// Open a plugin and immediately send a search query
    async fn handle_open_plugin_with_query(&mut self, id: String, initial_query: String) {
        self.state.pending_initial_query = Some(initial_query.clone());
        self.handle_open_plugin(id).await;

        if self.state.input_mode == InputMode::Realtime && !initial_query.is_empty() {
            self.send_plugin_search(&initial_query).await;
        }
    }

    async fn handle_close_plugin(&mut self) {
        debug!(
            "handle_close_plugin called, active_plugin: {:?}",
            self.state.active_plugin.as_ref().map(|p| &p.id)
        );
        if self.state.active_plugin.is_some() {
            if let Some((mut process, _)) = self.active_process.take()
                && let Err(e) = process.kill().await
            {
                warn!("Failed to kill plugin process: {e}");
            }

            self.state.active_plugin = None;
            self.state.navigation_depth = 0;
            self.state.input_mode = InputMode::Realtime;

            self.send_update(CoreUpdate::PluginDeactivated);
            self.send_update(CoreUpdate::Busy { busy: false });

            self.state.query.clear();
            debug!("Restoring main search after plugin close");
            let results = self.perform_main_search("").await;
            debug!("Got {} results for main search", results.len());
            self.send_update_cached(CoreUpdate::results(results));
            debug!("Main search results sent");
        }
    }

    async fn handle_form_submitted(
        &mut self,
        form_data: std::collections::HashMap<String, String>,
        context: Option<String>,
    ) {
        let Some((plugin_id, session)) = self.active_plugin_info() else {
            return;
        };

        let form_data_json = Self::serialize_form_data(&form_data);

        let input = PluginInput {
            step: Step::Form,
            query: Some(self.state.query.clone()),
            selected: None,
            action: None,
            session: Some(session),
            context,
            value: None,
            form_data: Some(form_data_json),
            source: None,
        };

        self.send_update(CoreUpdate::Busy { busy: true });
        self.send_to_plugin(&plugin_id, &input).await;
    }

    async fn handle_form_cancelled(&mut self) {
        let Some((plugin_id, session)) = self.active_plugin_info() else {
            return;
        };

        let input = PluginInput {
            step: Step::Action,
            query: Some(self.state.query.clone()),
            selected: Some(SelectedItem {
                id: ID_FORM_CANCEL.to_string(),
                extra: None,
            }),
            action: None,
            session: Some(session),
            context: None,
            value: None,
            form_data: None,
            source: None,
        };

        self.send_update(CoreUpdate::Busy { busy: true });
        self.send_to_plugin(&plugin_id, &input).await;
    }

    /// Handle live form field changes (sent when `form.live_update` is true)
    async fn handle_form_field_changed(
        &mut self,
        _field_id: String,
        _value: String,
        form_data: std::collections::HashMap<String, String>,
        context: Option<String>,
    ) {
        let Some((plugin_id, session)) = self.active_plugin_info() else {
            return;
        };

        let form_data_json = Self::serialize_form_data(&form_data);

        let input = PluginInput {
            step: Step::Form,
            query: Some(self.state.query.clone()),
            selected: None,
            action: None,
            session: Some(session),
            context,
            value: None,
            form_data: Some(form_data_json),
            source: None,
        };

        // Don't show busy indicator for live updates to avoid flicker
        self.send_to_plugin(&plugin_id, &input).await;
    }

    async fn handle_plugin_action(&mut self, action_id: String) {
        let Some((plugin_id, session)) = self.active_plugin_info() else {
            return;
        };

        let context = self
            .state
            .active_plugin
            .as_ref()
            .and_then(|a| a.context.clone());

        let input = PluginInput {
            step: Step::Action,
            query: Some(self.state.query.clone()),
            selected: Some(SelectedItem {
                id: ID_PLUGIN_ENTRY.to_string(),
                extra: None,
            }),
            action: Some(action_id),
            session: Some(session),
            context,
            value: None,
            form_data: None,
            source: None,
        };

        self.send_update(CoreUpdate::Busy { busy: true });
        self.send_to_plugin(&plugin_id, &input).await;
    }

    async fn perform_main_search(&mut self, query: &str) -> Vec<SearchResult> {
        if query.is_empty() && self.state.plugin_management {
            return self.get_plugin_list();
        }

        if query.is_empty() {
            return self.get_recent_and_suggestions();
        }

        if let Some((plugin, remaining_query)) = self.plugins.find_matching(query) {
            debug!(
                "Pattern match: plugin '{}' matches query '{}', remaining: '{}'",
                plugin.id, query, remaining_query
            );

            // Try to get inline preview from plugin (e.g., calculator result)
            if let Some(result) = self
                .try_pattern_match_preview(&plugin.id, &plugin.handler_path, &plugin.path, query)
                .await
            {
                return vec![result];
            }

            // Fall back to generic PatternMatch entry
            return vec![Self::create_pattern_match_result(
                plugin,
                query,
                &remaining_query,
            )];
        }

        let all_searchables = self.build_all_searchables(query);
        let matches = self.search.search(query, &all_searchables);

        let mut results: Vec<_> = matches
            .iter()
            .map(|m| {
                let frecency = match &m.searchable.source {
                    SearchableSource::IndexedItem { plugin_id, item } => self
                        .index
                        .get_item(plugin_id, &item.id)
                        .map_or(0.0, |i| self.index.calculate_frecency(i)),
                    SearchableSource::Plugin { id } => self
                        .index
                        .get_item(id, ID_PLUGIN_ENTRY)
                        .map_or(0.0, |i| self.index.calculate_frecency(i)),
                };

                let match_type = if m.is_history_term() {
                    MatchType::Exact
                } else {
                    MatchType::Fuzzy
                };

                let name_bonus = SearchEngine::name_match_bonus(query, &m.searchable.name);

                // Plugin entries (entry points) get a bonus over indexed items
                // This ensures "Settings" plugin ranks above "seat" emoji when typing "se"
                let plugin_entry_bonus = match &m.searchable.source {
                    SearchableSource::Plugin { .. } => 150.0,
                    SearchableSource::IndexedItem { .. } => 0.0,
                };

                // User-configurable per-plugin bonus
                let user_plugin_bonus = self
                    .config
                    .search
                    .plugin_ranking_bonus
                    .get(m.plugin_id())
                    .copied()
                    .unwrap_or(0.0);

                let composite = FrecencyScorer::composite_score(
                    match_type,
                    m.score + name_bonus + plugin_entry_bonus + user_plugin_bonus,
                    frecency,
                );

                (m, composite)
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut seen = std::collections::HashSet::new();
        results.retain(|(m, _)| seen.insert(m.searchable.id.clone()));

        FrecencyScorer::apply_diversity_decay(
            &mut results,
            |m| m.searchable.source.plugin_id(),
            self.config.search.diversity_decay,
            self.config.search.max_results_per_plugin,
        );

        let max_results = self.config.search.max_displayed_results;
        results
            .into_iter()
            .take(max_results)
            .map(|(m, score)| self.convert_search_match(m, score))
            .collect()
    }

    /// Try to get an inline preview result from a pattern-matched plugin.
    ///
    /// This invokes the plugin with `Step::Match` to get a computed result
    /// (e.g., calculator showing `444` for `123+321`). Returns `None` on
    /// timeout, error, or if the plugin doesn't support inline previews.
    async fn try_pattern_match_preview(
        &self,
        plugin_id: &str,
        handler_path: &Path,
        working_dir: &Path,
        query: &str,
    ) -> Option<SearchResult> {
        const MATCH_TIMEOUT_MS: u64 = 150;

        let response = invoke_match(
            plugin_id,
            handler_path,
            working_dir,
            query,
            MATCH_TIMEOUT_MS,
        )
        .await?;

        match response {
            PluginResponse::Match { result: Some(item) } => {
                // Convert ResultItem to SearchResult with plugin context
                let mut result = SearchResult {
                    id: format!("{PREFIX_MATCH_PREVIEW}{}:{}", plugin_id, item.id),
                    name: item.name,
                    description: item.description,
                    icon: item.icon,
                    icon_type: item.icon_type,
                    verb: item.verb,
                    result_type: ResultType::Plugin,
                    plugin_id: Some(plugin_id.to_string()),
                    actions: item.actions,
                    open_url: item.open_url,
                    copy: item.copy,
                    notify: item.notify,
                    should_close: item.should_close,
                    ..Default::default()
                };

                // Use plugin icon if item doesn't specify one
                if result.icon.is_none()
                    && let Some(plugin) = self.plugins.get(plugin_id)
                {
                    result.icon.clone_from(&plugin.manifest.icon);
                }

                Some(result)
            }
            _ => None,
        }
    }

    /// Create a `SearchResult` for a plugin pattern match
    fn create_pattern_match_result(
        plugin: &crate::plugin::Plugin,
        query: &str,
        remaining_query: &str,
    ) -> SearchResult {
        let entry_point = if remaining_query.is_empty() {
            None
        } else {
            Some(serde_json::json!({ "remaining_query": remaining_query }))
        };

        SearchResult {
            id: format!("{PREFIX_PATTERN_MATCH}{}", plugin.id),
            name: query.to_string(),
            description: Some(format!("Run with {}", plugin.manifest.name)),
            icon: Some(
                plugin
                    .manifest
                    .icon
                    .clone()
                    .unwrap_or_else(|| DEFAULT_PLUGIN_ICON.to_string()),
            ),
            icon_type: None,
            verb: Some(plugin.manifest.name.clone()),
            result_type: ResultType::PatternMatch,
            plugin_id: Some(plugin.id.clone()),
            entry_point,
            ..Default::default()
        }
    }

    fn build_all_searchables(&self, query: &str) -> Vec<Searchable> {
        let plugin_names: HashMap<_, _> = self
            .plugins
            .all()
            .map(|p| (p.id.clone(), p.manifest.name.clone()))
            .collect();

        let searchables = self.index.build_searchables(&plugin_names);
        debug!(
            "Built {} searchables from index, query: '{}'",
            searchables.len(),
            query
        );

        let mut all_searchables = searchables;
        for plugin in self.plugins.all() {
            if plugin.manifest.hidden {
                continue;
            }
            all_searchables.push(Searchable::from_plugin(
                &plugin.id,
                &plugin.manifest.name,
                plugin.manifest.description.as_deref(),
            ));

            // Add history term searchables from ID_PLUGIN_ENTRY (for frecency: "plugin" mode)
            if let Some(plugin_entry) = self.index.get_item(&plugin.id, ID_PLUGIN_ENTRY) {
                for term in &plugin_entry.frecency.recent_search_terms {
                    all_searchables.push(Searchable {
                        id: plugin.id.clone(),
                        name: term.clone(),
                        keywords: Vec::new(),
                        source: SearchableSource::Plugin {
                            id: plugin.id.clone(),
                        },
                        is_history_term: true,
                    });
                }
            }
        }

        if self.state.plugin_management {
            all_searchables.retain(|s| matches!(s.source, SearchableSource::Plugin { .. }));
            debug!(
                "Plugin list mode: filtered to {} plugin entries",
                all_searchables.len()
            );
        }

        all_searchables
    }

    fn convert_search_match(&self, m: &SearchMatch, score: f64) -> SearchResult {
        match &m.searchable.source {
            SearchableSource::Plugin { id } => {
                let plugin = self.plugins.get(id);
                SearchResult {
                    id: id.clone(),
                    name: plugin
                        .map_or_else(|| m.searchable.name.clone(), |p| p.manifest.name.clone()),
                    description: plugin.and_then(|p| p.manifest.description.clone()),
                    icon: Some(
                        plugin
                            .and_then(|p| p.manifest.icon.as_ref())
                            .cloned()
                            .unwrap_or_else(|| DEFAULT_PLUGIN_ICON.to_string()),
                    ),
                    icon_type: None,
                    verb: Some(DEFAULT_VERB_OPEN.to_string()),
                    result_type: ResultType::Plugin,
                    composite_score: score,
                    ..Default::default()
                }
            }
            SearchableSource::IndexedItem { plugin_id, item } => {
                let (icon, icon_type) = item.icon.as_ref().map_or_else(
                    || (DEFAULT_PLUGIN_ICON.to_string(), None),
                    |i| {
                        let effective_type = item.icon_type.as_deref();
                        (
                            i.clone(),
                            effective_type.map(std::string::ToString::to_string),
                        )
                    },
                );

                let actions = item.actions.clone();

                let result_type = ResultType::IndexedItem;

                SearchResult {
                    id: item.id.clone(),
                    name: item.name.clone(),
                    description: item.description.clone(),
                    icon: Some(icon),
                    icon_type,
                    verb: item.verb.clone(),
                    result_type,
                    plugin_id: Some(plugin_id.clone()),
                    app_id: item.app_id.clone(),
                    app_id_fallback: item.app_id_fallback.clone(),
                    actions,
                    badges: item.badges.clone(),
                    chips: item.chips.clone(),
                    widget: item.widget.clone(),
                    composite_score: score,
                    ..Default::default()
                }
            }
        }
    }

    fn load_static_indexes(&mut self) {
        for plugin in self.plugins.all() {
            if let Some(ref static_index) = plugin.manifest.static_index {
                let items: Vec<_> = static_index
                    .iter()
                    .map(|item| hamr_types::ResultItem {
                        id: item.id.clone(),
                        name: item.name.clone(),
                        description: item.description.clone(),
                        icon: item.icon.clone(),
                        icon_type: item.icon_type.clone(),
                        keywords: item.keywords.clone(),
                        verb: item.verb.clone(),
                        entry_point: item.entry_point.clone(),
                        ..Default::default()
                    })
                    .collect();

                self.index.update_full(&plugin.id, items);
            }
        }
    }

    #[must_use]
    pub fn state(&self) -> &LauncherState {
        &self.state
    }

    /// Set the open state (used when plugin requests close)
    pub fn set_open(&mut self, open: bool) {
        self.state.is_open = open;
    }

    #[must_use]
    pub fn dirs(&self) -> &Directories {
        &self.dirs
    }

    #[must_use]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Reload plugins from the filesystem.
    ///
    /// # Errors
    ///
    /// Returns an error if plugin discovery fails.
    pub fn reload_plugins(&mut self) -> Result<()> {
        let (_diff, _plugins) = self.plugins.rescan_with_plugins()?;
        Ok(())
    }

    /// Reload config from file (for hot-reload support).
    ///
    /// # Errors
    ///
    /// Returns an error if the config file cannot be read or contains invalid JSON.
    pub fn reload_config(&mut self) -> Result<()> {
        match Config::load(&self.dirs.config_file) {
            Ok(new_config) => {
                self.config = new_config;
                Ok(())
            }
            Err(e) => {
                // Log error but don't fail - keep existing config
                error!("Failed to reload config: {}", e);
                Err(e)
            }
        }
    }

    /// # Errors
    ///
    /// Returns an error if serialization fails or the file cannot be written.
    pub fn save_index(&mut self) -> Result<()> {
        self.index.save(&self.dirs.index_cache)
    }

    #[must_use]
    pub fn is_index_dirty(&self) -> bool {
        self.index.is_dirty()
    }

    #[must_use]
    pub fn last_index_dirty_at(&self) -> u64 {
        self.index.last_dirty_at()
    }

    #[must_use]
    pub fn index_stats(&self) -> IndexStats {
        self.index.stats()
    }

    /// Cache plugin results for frecency lookups
    /// Called by daemon when `plugin_results` are received
    pub fn cache_plugin_results(&mut self, results: Vec<SearchResult>) {
        self.state.last_results = results;
    }

    /// Update plugin index from daemon
    /// mode: None or "full" = replace all, "incremental" = add/remove
    pub fn update_plugin_index(
        &mut self,
        plugin_id: &str,
        items: Vec<crate::plugin::IndexItem>,
        mode: Option<&str>,
        remove: Option<Vec<String>>,
    ) {
        match mode {
            Some("incremental") => {
                self.index
                    .update_incremental(plugin_id, items, remove.unwrap_or_default());
            }
            _ => {
                self.index.update_full(plugin_id, items);
            }
        }
    }

    /// Check if there's state worth restoring
    fn has_restorable_state(&self) -> bool {
        self.state.active_plugin.is_some()
            || !self.state.query.is_empty()
            || !self.state.last_results.is_empty()
            || self.state.last_context.is_some()
    }

    /// Resend current state to UI for restoration
    fn resend_current_state(&self) {
        if let Some(ref active) = self.state.active_plugin {
            self.send_update(CoreUpdate::PluginActivated {
                id: active.id.clone(),
                name: active.name.clone(),
                icon: active.icon.clone(),
            });
        }

        if !self.state.last_results.is_empty() {
            self.send_update(CoreUpdate::results(self.state.last_results.clone()));
        }

        let mode = match self.state.input_mode {
            InputMode::Realtime => "realtime",
            InputMode::Submit => "submit",
        };
        self.send_update(CoreUpdate::InputModeChanged {
            mode: mode.to_string(),
        });

        if self.state.last_context.is_some() {
            self.send_update(CoreUpdate::ContextChanged {
                context: self.state.last_context.clone(),
            });
        }

        if let Some(ref placeholder) = self.state.last_placeholder {
            self.send_update(CoreUpdate::Placeholder {
                placeholder: placeholder.clone(),
            });
        }

        self.send_update(CoreUpdate::Prompt {
            prompt: self.state.query.clone(),
        });
    }
}

/// Statistics about the index
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IndexStats {
    pub plugin_count: usize,
    pub item_count: usize,
    pub items_per_plugin: Vec<(String, usize)>,
}

fn generate_session_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

    let id = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("session_{id}")
}
