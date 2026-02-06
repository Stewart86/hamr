//! Item-related request handlers.
//!
//! Handles item selection, slider changes, and switch toggles.

use hamr_core::CoreEvent;
use serde_json::Value;
use tracing::{debug, trace, warn};

use crate::error::{DaemonError, Result};
use crate::plugin_rpc;
use crate::registry::PluginRegistry;

use super::HandlerContext;

/// Try to spawn an on-demand plugin if it exists but isn't running.
/// Returns true if spawning was attempted (regardless of success).
pub(super) fn try_spawn_on_demand_plugin(ctx: &mut HandlerContext<'_>, pid: &str) -> bool {
    let Some(plugin) = ctx.needs_on_demand_spawn(pid) else {
        return false;
    };
    let plugin = plugin.clone();

    debug!(
        "[{}] Spawning on demand (non-background socket plugin)",
        pid
    );

    match ctx.resolve_and_spawn(pid, &plugin) {
        Ok(_) => true,
        Err(e) => {
            warn!("[{pid}] Failed to spawn on-demand plugin: {e}");
            true
        }
    }
}

/// Send initial message to a connected plugin.
/// Returns true if the message was sent (or attempted).
pub(super) fn try_send_initial_to_plugin(ctx: &HandlerContext<'_>, pid: &str) -> bool {
    if !ctx.plugin_registry.is_connected(pid) {
        return false;
    }

    debug!("[{}] Sending initial to connected socket plugin", pid);
    if let Some(sender) = ctx.plugin_registry.get_plugin_sender(pid) {
        let sender_clone = sender.clone();
        let pid_clone = pid.to_string();
        tokio::spawn(async move {
            if let Err(e) = plugin_rpc::send_initial(&sender_clone, &pid_clone, None) {
                warn!("[{}] Failed to send initial: {}", pid_clone, e);
            }
        });
        true
    } else {
        warn!("[{}] Plugin connected but no sender found", pid);
        true
    }
}

/// Forward an action to a connected plugin.
pub(super) fn forward_action_to_plugin(
    registry: &PluginRegistry,
    pid: &str,
    item_id: &str,
    action: Option<String>,
) {
    if let Some(sender) = registry.get_plugin_sender(pid) {
        let sender_clone = sender.clone();
        let item_id_clone = item_id.to_string();
        let pid_clone = pid.to_string();

        debug!("Forwarding action to plugin: {}", item_id);

        tokio::spawn(async move {
            if let Err(e) = plugin_rpc::send_action(
                &sender_clone,
                &pid_clone,
                item_id_clone,
                action,
                None,
                None,
            ) {
                trace!("[{}] Failed to send action to plugin: {}", pid_clone, e);
            }
        });
    } else {
        debug!("NOT forwarding action (no sender for plugin {})", pid);
    }
}

pub(super) async fn handle_item_selected(
    ctx: &mut HandlerContext<'_>,
    params: Option<&Value>,
) -> Result<Value> {
    #[derive(serde::Deserialize)]
    struct Params {
        id: String,
        #[serde(default)]
        action: Option<String>,
        #[serde(default)]
        plugin_id: Option<String>,
    }

    let params: Params = params
        .ok_or_else(|| DaemonError::InvalidParams("Missing params".to_string()))
        .and_then(|v| serde_json::from_value(v.clone()).map_err(DaemonError::Json))?;

    let item_id = params.id.clone();
    let action = params.action.clone();
    let mut plugin_id = params.plugin_id.clone();

    let active_plugin_before = ctx
        .core
        .state()
        .active_plugin
        .as_ref()
        .map(|p| p.id.clone());

    ctx.core
        .process(CoreEvent::ItemSelected {
            id: item_id.clone(),
            action: action.clone(),
            plugin_id: plugin_id.clone(),
        })
        .await;

    let active_plugin_after = ctx
        .core
        .state()
        .active_plugin
        .as_ref()
        .map(|p| p.id.clone());

    let plugin_just_opened = match (&active_plugin_before, &active_plugin_after) {
        (None, Some(pid)) => Some(pid.clone()),
        (Some(old), Some(new)) if old != new => Some(new.clone()),
        _ => None,
    };

    if let Some(ref pid) = plugin_just_opened {
        debug!("[{}] Plugin opened", pid);

        if try_spawn_on_demand_plugin(ctx, pid) {
            return Ok(super::ok_response());
        }

        if try_send_initial_to_plugin(ctx, pid) {
            return Ok(super::ok_response());
        }

        debug!(
            "[{}] Plugin not connected (is_socket: {})",
            pid,
            ctx.plugin_registry.get_socket_plugin(pid).is_some()
        );
    }

    if plugin_id.is_none()
        && let Some(active_plugin) = ctx.core.state().active_plugin.as_ref()
        && ctx.plugin_registry.is_connected(&active_plugin.id)
    {
        plugin_id = Some(active_plugin.id.clone());
    }

    debug!(
        "handle_item_selected: plugin_id={:?}, item_id={}",
        plugin_id, item_id
    );

    if let Some(pid) = plugin_id {
        forward_action_to_plugin(ctx.plugin_registry, &pid, &item_id, action);
    } else {
        debug!("NOT forwarding action (no plugin_id)");
    }

    Ok(super::ok_response())
}

pub(super) async fn handle_slider_changed(
    ctx: &mut HandlerContext<'_>,
    params: Option<&Value>,
) -> Result<()> {
    #[derive(serde::Deserialize)]
    struct Params {
        id: String,
        value: f64,
        #[serde(default)]
        plugin_id: Option<String>,
    }

    let params: Params = params
        .ok_or_else(|| DaemonError::InvalidParams("Missing params".to_string()))
        .and_then(|v| serde_json::from_value(v.clone()).map_err(DaemonError::Json))?;

    let slider_id = params.id.clone();
    let value = params.value;

    let plugin_id = params.plugin_id.clone().or_else(|| {
        ctx.core
            .state()
            .active_plugin
            .as_ref()
            .map(|p| p.id.clone())
    });

    ctx.core
        .process(CoreEvent::SliderChanged {
            id: slider_id.clone(),
            value,
            plugin_id: plugin_id.clone(),
        })
        .await;

    if let Some(pid) = plugin_id
        && let Some(sender) = ctx.plugin_registry.get_plugin_sender(&pid)
    {
        let sender_clone = sender.clone();
        let slider_id_clone = slider_id.clone();
        let pid_clone = pid.clone();

        tokio::spawn(async move {
            if let Err(e) =
                plugin_rpc::send_slider_changed(&sender_clone, &pid_clone, &slider_id_clone, value)
            {
                trace!(
                    "[{}] Failed to send slider_changed to plugin: {}",
                    pid_clone, e
                );
            }
        });
    }

    Ok(())
}

pub(super) async fn handle_switch_toggled(
    ctx: &mut HandlerContext<'_>,
    params: Option<&Value>,
) -> Result<()> {
    #[derive(serde::Deserialize)]
    struct Params {
        id: String,
        value: bool,
        #[serde(default)]
        plugin_id: Option<String>,
    }

    let params: Params = params
        .ok_or_else(|| DaemonError::InvalidParams("Missing params".to_string()))
        .and_then(|v| serde_json::from_value(v.clone()).map_err(DaemonError::Json))?;

    let switch_id = params.id.clone();
    let value = params.value;

    let plugin_id = params.plugin_id.clone().or_else(|| {
        ctx.core
            .state()
            .active_plugin
            .as_ref()
            .map(|p| p.id.clone())
    });

    debug!(
        "handle_switch_toggled: switch_id={}, value={}, plugin_id={:?}",
        switch_id, value, plugin_id
    );

    ctx.core
        .process(CoreEvent::SwitchToggled {
            id: switch_id.clone(),
            value,
            plugin_id: plugin_id.clone(),
        })
        .await;

    if let Some(pid) = plugin_id.clone()
        && let Some(sender) = ctx.plugin_registry.get_plugin_sender(&pid)
    {
        let sender_clone = sender.clone();
        let switch_id_clone = switch_id.clone();
        let pid_clone = pid.clone();

        debug!("[{}] Forwarding switch_toggled to socket plugin", pid);

        tokio::spawn(async move {
            if let Err(e) =
                plugin_rpc::send_switch_toggled(&sender_clone, &pid_clone, &switch_id_clone, value)
            {
                trace!(
                    "[{}] Failed to send switch_toggled to plugin: {}",
                    pid_clone, e
                );
            }
        });
    } else {
        debug!(
            "NOT forwarding switch_toggled: plugin_id={:?}, has_sender={}",
            plugin_id,
            plugin_id
                .as_ref()
                .is_some_and(|pid| ctx.plugin_registry.get_plugin_sender(pid).is_some())
        );
    }

    Ok(())
}
