//! Plugin notification handlers.
//!
//! Handlers for notifications received from plugins:
//! - `plugin_results` - search results from plugin
//! - `plugin_status` - status bar updates (badges, chips, ambient)
//! - `plugin_index` - index updates for searchable items
//! - `plugin_execute` - action execution requests
//! - `plugin_update` - result patches for live updates

use hamr_rpc::protocol::{Message, Notification};
use hamr_types::PluginStatus;
use serde_json::Value;
use tracing::{debug, warn};

use crate::error::{DaemonError, Result};

use super::{HandlerContext, send_status_to_ui};

/// Handle `plugin_results` notification - forward search results to active UI.
pub(super) fn handle_plugin_results(
    ctx: &mut HandlerContext<'_>,
    params: Option<&Value>,
) -> Result<()> {
    let plugin_id = ctx
        .current_session()
        .and_then(|s| s.plugin_id())
        .ok_or_else(|| DaemonError::InvalidParams("Not a plugin session".to_string()))?
        .to_string();

    let state = ctx.core.state();
    let active_plugin_id = state.active_plugin.as_ref().map(|p| p.id.clone());
    let is_open = state.is_open;
    debug!(
        "[{}] Received plugin_results (active_plugin={:?}, is_open={})",
        plugin_id, active_plugin_id, is_open
    );

    if !is_open {
        debug!(
            "[{}] Ignoring plugin_results - launcher is closed",
            plugin_id
        );
        return Ok(());
    }

    if active_plugin_id.as_deref() != Some(&plugin_id) {
        debug!(
            "[{}] Ignoring plugin_results - not the active plugin",
            plugin_id
        );
        return Ok(());
    }

    if let Some(params) = params {
        if let Some(results_value) = params.get("results").and_then(|v| v.as_array()) {
            debug!(
                "[{}] Forwarding {} results to active UI",
                plugin_id,
                results_value.len()
            );

            if let Ok(results) = serde_json::from_value::<Vec<hamr_types::SearchResult>>(
                serde_json::json!(results_value),
            ) {
                ctx.core.cache_plugin_results(results);
            }

            if let Some(ui_id) = ctx.active_ui.as_ref() {
                if let Some(tx) = ctx.client_senders.get(ui_id) {
                    let notification = Notification::new(
                        "results",
                        Some(serde_json::json!({ "results": results_value })),
                    );
                    if let Err(e) = tx.send(Message::Notification(notification)) {
                        warn!("[{}] Failed to forward results to UI: {}", plugin_id, e);
                    }
                } else {
                    debug!("[{}] Active UI {} not in client_senders", plugin_id, ui_id);
                }
            } else {
                debug!("[{}] No active UI to forward results to", plugin_id);
            }
        } else {
            debug!("[{}] Results params missing 'results' array", plugin_id);
        }
    }

    Ok(())
}

/// Handle `plugin_status` notification - update status bar and forward to UI.
pub(super) fn handle_plugin_status(
    ctx: &mut HandlerContext<'_>,
    params: Option<&Value>,
) -> Result<()> {
    let plugin_id = ctx
        .current_session()
        .and_then(|s| s.plugin_id())
        .ok_or_else(|| DaemonError::InvalidParams("Not a plugin session".to_string()))?
        .to_string();

    debug!("[{}] Received plugin_status", plugin_id);

    if let Some(params) = params {
        if let Some(status_value) = params.get("status") {
            match serde_json::from_value::<PluginStatus>(status_value.clone()) {
                Ok(status) => {
                    debug!(
                        "[{}] Status update: badges={}, chips={}, ambient={}, fab={:?}",
                        plugin_id,
                        status.badges.len(),
                        status.chips.len(),
                        status.ambient.len(),
                        status.fab.as_ref().map(|f| (f.chips.len(), f.badges.len()))
                    );

                    ctx.plugin_statuses
                        .insert(plugin_id.clone(), status.clone());

                    if let Some(ui_id) = ctx.active_ui.as_ref() {
                        if let Some(tx) = ctx.client_senders.get(ui_id) {
                            send_status_to_ui(tx, &plugin_id, &status);
                        } else {
                            debug!("[{}] Active UI {} not in client_senders", plugin_id, ui_id);
                        }
                    } else {
                        debug!("[{}] No active UI to forward status to", plugin_id);
                    }
                }
                Err(e) => {
                    warn!("[{}] Failed to parse plugin_status: {}", plugin_id, e);
                }
            }
        } else {
            debug!("[{}] Status params missing 'status' field", plugin_id);
        }
    }

    Ok(())
}

/// Handle `plugin_index` notification - update searchable item index.
pub(super) fn handle_plugin_index(
    ctx: &mut HandlerContext<'_>,
    params: Option<&Value>,
) -> Result<()> {
    let plugin_id = ctx
        .current_session()
        .and_then(|s| s.plugin_id())
        .ok_or_else(|| DaemonError::InvalidParams("Not a plugin session".to_string()))?
        .to_string();

    let Some(params) = params else {
        return Ok(());
    };

    let items: Vec<hamr_core::plugin::IndexItem> = params
        .get("items")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let mode = params.get("mode").and_then(|v| v.as_str());
    let remove: Option<Vec<String>> = params
        .get("remove")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    debug!(
        "[{}] Received plugin_index ({} items, mode: {:?}, remove: {:?})",
        plugin_id,
        items.len(),
        mode,
        remove.as_ref().map(std::vec::Vec::len)
    );

    ctx.core
        .update_plugin_index(&plugin_id, items, mode, remove);

    Ok(())
}

/// Handle `plugin_execute` notification - plugin requests action execution.
pub(super) fn handle_plugin_execute(
    ctx: &mut HandlerContext<'_>,
    params: Option<&Value>,
) -> Result<()> {
    let plugin_id = ctx
        .current_session()
        .and_then(|s| s.plugin_id())
        .ok_or_else(|| DaemonError::InvalidParams("Not a plugin session".to_string()))?
        .to_string();

    debug!("[{}] Received plugin_execute", plugin_id);

    if let Some(params) = params {
        if let Some(action_value) = params.get("action") {
            let action_str = serde_json::to_string(action_value).unwrap_or_else(|e| {
                warn!(
                    "[{}] Failed to serialize action for logging: {}",
                    plugin_id, e
                );
                format!("<invalid: {e}>")
            });
            debug!("[{}] Execute action: {:?}", plugin_id, action_str);

            if let Some(ui_id) = ctx.active_ui.as_ref() {
                if let Some(tx) = ctx.client_senders.get(ui_id) {
                    let notification = Notification::new(
                        "execute",
                        Some(serde_json::json!({ "action": action_value })),
                    );
                    if let Err(e) = tx.send(Message::Notification(notification)) {
                        warn!("[{}] Failed to forward execute to UI: {}", plugin_id, e);
                    }

                    if let Some(obj) = action_value.as_object()
                        && let Some(sound) = obj.get("sound").and_then(|v| v.as_str())
                    {
                        let action_type = obj.get("type").and_then(|v| v.as_str());
                        if action_type != Some("sound") {
                            debug!("[{}] Sending separate PlaySound for: {}", plugin_id, sound);
                            let sound_notification = Notification::new(
                                "execute",
                                Some(
                                    serde_json::json!({ "action": { "type": "sound", "sound": sound } }),
                                ),
                            );
                            if let Err(e) = tx.send(Message::Notification(sound_notification)) {
                                warn!(
                                    "[{}] Failed to forward sound execute to UI: {}",
                                    plugin_id, e
                                );
                            }
                        }
                    }
                } else {
                    debug!("[{}] Active UI {} not in client_senders", plugin_id, ui_id);
                }
            } else {
                debug!("[{}] No active UI to forward execute to", plugin_id);
            }
        } else {
            debug!("[{}] Execute params missing 'action' field", plugin_id);
        }
    }

    Ok(())
}

/// Handle `plugin_update` notification - forward result patches to UI.
pub(super) fn handle_plugin_update(
    ctx: &mut HandlerContext<'_>,
    params: Option<&Value>,
) -> Result<()> {
    let plugin_id = ctx
        .current_session()
        .and_then(|s| s.plugin_id())
        .ok_or_else(|| DaemonError::InvalidParams("Not a plugin session".to_string()))?
        .to_string();

    debug!("[{}] Received plugin_update", plugin_id);

    if let Some(params) = params {
        if let Some(patches) = params.get("patches").and_then(|v| v.as_array()) {
            debug!(
                "[{}] Forwarding {} patches to active UI",
                plugin_id,
                patches.len()
            );

            if let Some(ui_id) = ctx.active_ui.as_ref() {
                if let Some(tx) = ctx.client_senders.get(ui_id) {
                    let notification = Notification::new(
                        "results_update",
                        Some(serde_json::json!({ "patches": patches })),
                    );
                    if let Err(e) = tx.send(Message::Notification(notification)) {
                        warn!("[{}] Failed to forward update to UI: {}", plugin_id, e);
                    }
                } else {
                    debug!("[{}] Active UI {} not in client_senders", plugin_id, ui_id);
                }
            } else {
                debug!("[{}] No active UI to forward update to", plugin_id);
            }
        } else {
            debug!("[{}] Update params missing 'patches' array", plugin_id);
        }
    }

    Ok(())
}
