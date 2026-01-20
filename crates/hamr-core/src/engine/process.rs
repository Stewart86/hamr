//! Plugin response processing for `HamrCore`.
//!
//! This module handles converting plugin responses into `CoreUpdate` events
//! and spawning background listeners for plugin communication.

use crate::plugin::{
    PluginReceiver, PluginResponse, plugin_response_to_updates, process_status_data,
};
use hamr_types::CoreUpdate;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;

/// Process a plugin response and return updates.
///
/// Delegates to the shared `plugin_response_to_updates` for most responses,
/// with special handling for `Index` responses that need `CoreUpdate::IndexUpdate`.
pub fn process_plugin_response(plugin_id: &str, response: PluginResponse) -> Vec<CoreUpdate> {
    // Handle Index response specially - engine needs to emit IndexUpdate
    if let PluginResponse::Index {
        ref items,
        ref mode,
        ref remove,
        ref status,
    } = response
    {
        let mut updates = vec![CoreUpdate::Busy { busy: false }];

        if !items.is_empty() || remove.is_some() {
            let items_json = serde_json::to_value(items).unwrap_or_default();
            updates.push(CoreUpdate::IndexUpdate {
                plugin_id: plugin_id.to_string(),
                items: items_json,
                mode: mode.clone(),
                remove: remove.clone(),
            });
        }

        if let Some(status_data) = status {
            updates.extend(process_status_data(plugin_id, status_data.clone()));
        }

        return updates;
    }

    // For all other responses, use the shared conversion
    plugin_response_to_updates(plugin_id, response)
}

/// Start listening for responses from a plugin receiver.
///
/// Spawns a background task that forwards responses to the update channel.
/// The listener runs until the receiver is closed or the update channel is dropped.
pub fn spawn_response_listener(
    plugin_id: String,
    mut receiver: PluginReceiver,
    update_tx: UnboundedSender<CoreUpdate>,
) {
    tokio::spawn(async move {
        while let Some(response) = receiver.recv().await {
            let updates = process_plugin_response(&plugin_id, response);
            for update in updates {
                if update_tx.send(update).is_err() {
                    return; // Channel closed
                }
            }
        }
        debug!("Plugin {} receiver closed", plugin_id);
    });
}
