//! Query-related handlers.
//!
//! Handlers for search query notifications:
//! - `query_changed` - user typing in search box (real-time)
//! - `query_submitted` - user pressed Enter to submit query

use hamr_core::CoreEvent;
use serde_json::Value;
use tracing::{debug, trace, warn};

use crate::error::{DaemonError, Result};
use crate::plugin_rpc;

use super::HandlerContext;

/// Handle `query_submitted` - user pressed Enter to submit query.
///
/// Returns the search to the plugin if one is active and connected.
pub(super) async fn handle_query_submitted(
    ctx: &mut HandlerContext<'_>,
    params: Option<&Value>,
) -> Result<Value> {
    #[derive(serde::Deserialize)]
    struct Params {
        query: String,
        #[serde(default)]
        context: Option<String>,
    }

    let params: Params = params
        .ok_or_else(|| DaemonError::InvalidParams("Missing params".to_string()))
        .and_then(|v| serde_json::from_value(v.clone()).map_err(DaemonError::Json))?;

    debug!(
        "QuerySubmitted: query='{}', context={:?}",
        params.query, params.context
    );

    ctx.core
        .process(CoreEvent::QuerySubmitted {
            query: params.query.clone(),
            context: params.context.clone(),
        })
        .await;

    if let Some(active) = ctx.core.state().active_plugin.as_ref()
        && ctx.plugin_registry.is_connected(&active.id)
        && let Some(sender) = ctx.plugin_registry.get_plugin_sender(&active.id)
    {
        let plugin_id = active.id.clone();
        let query = params.query;
        let context = params.context.or_else(|| active.context.clone());
        let sender_clone = sender.clone();

        debug!(
            "[{}] Forwarding QuerySubmitted as search: query='{}', context={:?}",
            plugin_id, query, context
        );

        tokio::spawn(async move {
            if let Err(e) = plugin_rpc::send_search(&sender_clone, &plugin_id, query, context) {
                tracing::warn!("[{}] Failed to send search to plugin: {}", plugin_id, e);
            }
        });
    }

    Ok(super::ok_response())
}

/// Handle `query_changed` - user typing in search box.
///
/// Spawns on-demand plugins if needed and forwards search to active plugin.
pub(super) async fn handle_query_changed(
    ctx: &mut HandlerContext<'_>,
    params: Option<&Value>,
) -> Result<()> {
    #[derive(serde::Deserialize)]
    struct Params {
        query: String,
    }

    let params: Params = params
        .ok_or_else(|| DaemonError::InvalidParams("Missing params".to_string()))
        .and_then(|v| serde_json::from_value(v.clone()).map_err(DaemonError::Json))?;

    let query = params.query.clone();

    ctx.core
        .process(CoreEvent::QueryChanged {
            query: query.clone(),
        })
        .await;

    // State restoration: plugin was stopped when launcher closed
    if let Some(active) = ctx.core.state().active_plugin.as_ref() {
        let plugin_id = active.id.clone();

        if let Some(plugin) = ctx.needs_on_demand_spawn(&plugin_id) {
            let plugin = plugin.clone();
            debug!("[{plugin_id}] Spawning on demand (state restored, plugin not running)");

            match ctx.resolve_and_spawn(&plugin_id, &plugin) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    warn!("[{plugin_id}] Failed to spawn on-demand plugin: {e}");
                    return Ok(());
                }
            }
        }

        if ctx.plugin_registry.is_connected(&plugin_id)
            && let Some(sender) = ctx.plugin_registry.get_plugin_sender(&plugin_id)
        {
            let query_clone = query.clone();
            let context = active.context.clone();
            let sender_clone = sender.clone();

            tokio::spawn(async move {
                if let Err(e) =
                    plugin_rpc::send_search(&sender_clone, &plugin_id, query_clone, context)
                {
                    trace!("[{}] Failed to send search to plugin: {}", plugin_id, e);
                }
            });
        }
    }

    Ok(())
}
