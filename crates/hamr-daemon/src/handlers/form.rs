//! Form-related request handlers.
//!
//! Handles form submission, cancellation, and field changes.

use std::collections::HashMap;

use hamr_core::CoreEvent;
use serde_json::Value;
use tracing::{debug, warn};

use crate::error::{DaemonError, Result};
use crate::plugin_rpc;

use super::HandlerContext;

/// Handle form submission from UI.
///
/// Forwards form data to the active plugin if connected.
pub(super) async fn handle_form_submitted(
    ctx: &mut HandlerContext<'_>,
    params: Option<&Value>,
) -> Result<Value> {
    #[derive(serde::Deserialize)]
    struct Params {
        form_data: HashMap<String, String>,
        #[serde(default)]
        context: Option<String>,
    }

    let params: Params = params
        .ok_or_else(|| DaemonError::InvalidParams("Missing params".to_string()))
        .and_then(|v| serde_json::from_value(v.clone()).map_err(DaemonError::Json))?;

    let form_data = params.form_data.clone();
    let context = params.context.clone();

    let plugin_id = ctx
        .core
        .state()
        .active_plugin
        .as_ref()
        .map(|p| p.id.clone());

    debug!(
        "handle_form_submitted: plugin_id={:?}, context={:?}, form_data={:?}",
        plugin_id, context, form_data
    );

    ctx.core
        .process(CoreEvent::FormSubmitted {
            form_data: form_data.clone(),
            context: context.clone(),
        })
        .await;

    if let Some(pid) = plugin_id.clone()
        && let Some(sender) = ctx.plugin_registry.get_plugin_sender(&pid)
    {
        debug!("[{}] Forwarding form_submitted to socket plugin", pid);

        if let Err(e) =
            plugin_rpc::send_form_submitted(sender, &pid, &form_data, context.as_deref())
        {
            warn!("[{}] Failed to send form_submitted to plugin: {}", pid, e);
        }
    } else {
        debug!(
            "NOT forwarding form_submitted: plugin_id={:?}, has_sender={}",
            plugin_id,
            plugin_id
                .as_ref()
                .is_some_and(|pid| ctx.plugin_registry.get_plugin_sender(pid).is_some())
        );
    }

    Ok(super::ok_response())
}

/// Handle form cancellation from UI.
pub(super) async fn handle_form_cancelled(ctx: &mut HandlerContext<'_>) -> Result<()> {
    ctx.core.process(CoreEvent::FormCancelled).await;
    Ok(())
}

/// Handle form field change from UI.
///
/// Notifies core of field value changes for validation or live updates.
pub(super) async fn handle_form_field_changed(
    ctx: &mut HandlerContext<'_>,
    params: Option<&Value>,
) -> Result<()> {
    #[derive(serde::Deserialize)]
    struct Params {
        field_id: String,
        value: String,
        form_data: HashMap<String, String>,
        #[serde(default)]
        context: Option<String>,
    }

    let params: Params = params
        .ok_or_else(|| DaemonError::InvalidParams("Missing params".to_string()))
        .and_then(|v| serde_json::from_value(v.clone()).map_err(DaemonError::Json))?;

    ctx.core
        .process(CoreEvent::FormFieldChanged {
            field_id: params.field_id,
            value: params.value,
            form_data: params.form_data,
            context: params.context,
        })
        .await;

    Ok(())
}
