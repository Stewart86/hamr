//! Request handlers for the daemon.
//!
//! This module contains handlers for all RPC methods organized by category:
//! - Registration
//! - UI methods (`query_changed`, `item_selected`, etc.)
//! - Control methods (toggle, show, hide, etc.)
//! - Plugin methods (`plugin_results`, `plugin_status`, etc.)

mod form;
mod item;
mod plugin;
mod query;

use std::collections::HashMap;

use hamr_core::{CoreEvent, HamrCore};
use hamr_rpc::protocol::{
    ClientRole, Message, Notification, RegisterParams, RegisterResult, Request, RequestId,
    Response, RpcError,
};
use hamr_types::PluginStatus;
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::{debug, trace, warn};

use crate::error::{DaemonError, Result};
use crate::plugin_rpc;
use crate::plugin_spawner::PluginSpawner;
use crate::registry::PluginRegistry;
use crate::session::{ClientInfo, ControlSession, PluginSession, Session, SessionId, UiSession};

pub struct HandlerContext<'a> {
    pub core: &'a mut HamrCore,
    pub clients: &'a mut HashMap<SessionId, Session>,
    pub active_ui: &'a mut Option<SessionId>,
    pub client_id: &'a SessionId,
    pub plugin_registry: &'a mut PluginRegistry,
    pub plugin_spawner: &'a mut PluginSpawner,
    pub client_sender: Option<mpsc::UnboundedSender<Message>>,
    pub client_senders: &'a HashMap<SessionId, mpsc::UnboundedSender<Message>>,
    pub plugin_statuses: &'a mut HashMap<String, PluginStatus>,
}

impl HandlerContext<'_> {
    pub fn current_session(&self) -> Option<&Session> {
        self.clients.get(self.client_id)
    }

    pub fn is_registered(&self) -> bool {
        self.current_session()
            .is_some_and(super::session::Session::is_registered)
    }

    pub fn is_active_ui(&self) -> bool {
        self.active_ui
            .as_ref()
            .is_some_and(|id| id == self.client_id)
    }
}

pub async fn handle_request(
    ctx: &mut HandlerContext<'_>,
    request: &Request,
) -> std::result::Result<Response, RpcError> {
    let id = request.id.clone().unwrap_or(RequestId::Number(0));

    let result = match request.method.as_str() {
        "register" => handle_register(ctx, request.params.as_ref()),

        "query_submitted" => {
            require_ui(ctx)?;
            query::handle_query_submitted(ctx, request.params.as_ref()).await
        }
        "item_selected" => {
            require_ui(ctx)?;
            item::handle_item_selected(ctx, request.params.as_ref()).await
        }
        "form_submitted" => {
            require_ui(ctx)?;
            form::handle_form_submitted(ctx, request.params.as_ref()).await
        }
        "ambient_action" => {
            require_ui(ctx)?;
            handle_ambient_action(ctx, request.params.as_ref()).await
        }

        "toggle" => {
            require_control_or_ui(ctx)?;
            handle_toggle(ctx)
        }
        "show" => {
            require_control_or_ui(ctx)?;
            handle_show(ctx).await
        }
        "hide" => {
            require_control_or_ui(ctx)?;
            handle_hide(ctx).await
        }
        "open_plugin" => {
            require_control_or_ui(ctx)?;
            handle_open_plugin(ctx, request.params.as_ref()).await
        }
        "update_status" => {
            require_control_or_ui(ctx)?;
            handle_update_status(ctx, request.params.as_ref())
        }
        "status" => {
            require_registered(ctx)?;
            handle_status(ctx)
        }
        "shutdown" => {
            require_control_or_ui(ctx)?;
            handle_shutdown(ctx)
        }
        "reload_plugins" => {
            require_control_or_ui(ctx)?;
            handle_reload_plugins(ctx)
        }
        "index_stats" => {
            require_registered(ctx)?;
            handle_index_stats(ctx)
        }
        "list_plugins" => {
            require_registered(ctx)?;
            handle_list_plugins(ctx)
        }
        method => Err(DaemonError::MethodNotFound(method.to_string())),
    };

    match result {
        Ok(value) => Ok(Response::success(id, value)),
        Err(e) => Ok(Response::error(id, e.into())),
    }
}

pub async fn handle_notification(
    ctx: &mut HandlerContext<'_>,
    notification: &Notification,
) -> Result<()> {
    trace!(
        "Handling notification: method={}, is_active_ui={}",
        notification.method,
        ctx.is_active_ui()
    );

    // Route to UI or plugin notification handlers
    if let Some(result) = handle_ui_notification(ctx, notification).await {
        return result;
    }
    if let Some(result) = handle_plugin_notification(ctx, notification) {
        return result;
    }

    warn!("Unknown notification method: {}", notification.method);
    Ok(())
}

async fn handle_ui_notification(
    ctx: &mut HandlerContext<'_>,
    notification: &Notification,
) -> Option<Result<()>> {
    // Check if this is a known UI notification method
    let is_ui_method = matches!(
        notification.method.as_str(),
        "query_changed"
            | "slider_changed"
            | "switch_toggled"
            | "form_submitted"
            | "back"
            | "cancel"
            | "launcher_opened"
            | "launcher_closed"
            | "dismiss_ambient"
            | "ambient_action"
            | "form_cancelled"
            | "form_field_changed"
            | "plugin_action_triggered"
            | "close_plugin"
            | "open_plugin"
            | "query_submitted"
            | "item_selected"
    );

    if !is_ui_method {
        return None;
    }

    // UI notifications require active UI - silently ignore if not active
    if !ctx.is_active_ui() {
        trace!("Ignoring {} - not active UI", notification.method);
        return Some(Ok(()));
    }

    let params = notification.params.as_ref();
    let result = match notification.method.as_str() {
        "query_changed" => {
            trace!("Processing query_changed");
            query::handle_query_changed(ctx, params).await
        }
        "slider_changed" => item::handle_slider_changed(ctx, params).await,
        "switch_toggled" => item::handle_switch_toggled(ctx, params).await,
        "form_submitted" => form::handle_form_submitted(ctx, params).await.map(|_| ()),
        "back" => handle_back(ctx).await,
        "cancel" => handle_cancel(ctx).await,
        "launcher_opened" => {
            trace!("Processing launcher_opened");
            handle_launcher_opened(ctx).await
        }
        "launcher_closed" => handle_launcher_closed(ctx).await,
        "dismiss_ambient" => handle_dismiss_ambient(ctx, params).await,
        "ambient_action" => handle_ambient_action(ctx, params).await.map(|_| ()),
        "form_cancelled" => form::handle_form_cancelled(ctx).await,
        "form_field_changed" => form::handle_form_field_changed(ctx, params).await,
        "plugin_action_triggered" => handle_plugin_action_triggered(ctx, params).await,
        "close_plugin" => handle_close_plugin(ctx).await,
        "open_plugin" => handle_open_plugin(ctx, params).await.map(|_| ()),
        "query_submitted" => query::handle_query_submitted(ctx, params).await.map(|_| ()),
        "item_selected" => item::handle_item_selected(ctx, params).await.map(|_| ()),
        _ => unreachable!(),
    };

    Some(result)
}

fn handle_plugin_notification(
    ctx: &mut HandlerContext<'_>,
    notification: &Notification,
) -> Option<Result<()>> {
    // Check if this is a known plugin notification method
    let is_plugin_method = matches!(
        notification.method.as_str(),
        "plugin_results" | "plugin_status" | "plugin_index" | "plugin_execute" | "plugin_update"
    );

    if !is_plugin_method {
        return None;
    }

    // Plugin notifications must come from plugin sessions
    if !is_plugin_session(ctx) {
        trace!("Ignoring {} - not from plugin", notification.method);
        return Some(Ok(()));
    }

    let params = notification.params.as_ref();
    let result = match notification.method.as_str() {
        "plugin_results" => plugin::handle_plugin_results(ctx, params),
        "plugin_status" => plugin::handle_plugin_status(ctx, params),
        "plugin_index" => plugin::handle_plugin_index(ctx, params),
        "plugin_execute" => plugin::handle_plugin_execute(ctx, params),
        "plugin_update" => plugin::handle_plugin_update(ctx, params),
        _ => unreachable!(),
    };

    Some(result)
}

fn require_registered(ctx: &HandlerContext<'_>) -> Result<()> {
    if !ctx.is_registered() {
        return Err(DaemonError::NotRegistered);
    }
    Ok(())
}

fn require_ui(ctx: &HandlerContext<'_>) -> Result<()> {
    require_registered(ctx)?;
    if !ctx.is_active_ui() {
        return Err(DaemonError::NotActiveUi);
    }
    Ok(())
}

fn require_control_or_ui(ctx: &HandlerContext<'_>) -> Result<()> {
    require_registered(ctx)?;
    let session = ctx.current_session().ok_or(DaemonError::NotRegistered)?;
    if session.is_control() || session.is_ui() {
        Ok(())
    } else {
        Err(DaemonError::ControlRequired)
    }
}

fn is_plugin_session(ctx: &HandlerContext<'_>) -> bool {
    ctx.current_session()
        .is_some_and(|s| matches!(s, Session::Plugin(_)))
}

/// Forward cached plugin statuses to a newly connected UI
fn forward_cached_statuses(
    tx: &mpsc::UnboundedSender<Message>,
    plugin_statuses: &HashMap<String, PluginStatus>,
    session_id: &SessionId,
) {
    for (plugin_id, status) in plugin_statuses {
        debug!(
            "Forwarding cached status for {} to new UI {}",
            plugin_id, session_id
        );
        let notification = Notification::new(
            "plugin_status_update",
            Some(serde_json::json!({
                "plugin_id": plugin_id,
                "status": status
            })),
        );
        if let Err(e) = tx.send(Message::Notification(notification)) {
            warn!(
                "Failed to forward cached status for {} to UI: {}",
                plugin_id, e
            );
        }

        if !status.ambient.is_empty() {
            let ambient_notification = Notification::new(
                "ambient_update",
                Some(serde_json::json!({
                    "plugin_id": plugin_id,
                    "items": status.ambient
                })),
            );
            if let Err(e) = tx.send(Message::Notification(ambient_notification)) {
                warn!(
                    "Failed to forward cached ambient for {} to UI: {}",
                    plugin_id, e
                );
            }
        }
    }
}

/// Register a plugin and send initial request if it's the active plugin
fn register_plugin_in_registry(
    ctx: &mut HandlerContext<'_>,
    plugin_id: &str,
    session_id: &SessionId,
    sender: &mpsc::UnboundedSender<Message>,
) {
    use crate::registry::ConnectedPlugin;
    let connected = ConnectedPlugin {
        id: plugin_id.to_string(),
        session_id: session_id.clone(),
        sender: sender.clone(),
    };
    ctx.plugin_registry.register_connected(connected);

    let active_plugin_id = ctx
        .core
        .state()
        .active_plugin
        .as_ref()
        .map(|p| p.id.clone());
    let is_active_plugin = active_plugin_id.as_deref() == Some(plugin_id);

    debug!(
        "[{}] Plugin registered, active_plugin={:?}, is_active={}",
        plugin_id, active_plugin_id, is_active_plugin
    );

    if is_active_plugin {
        debug!(
            "[{}] Sending initial to newly connected active plugin",
            plugin_id
        );
        let sender_clone = sender.clone();
        let pid = plugin_id.to_string();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            if let Err(e) = plugin_rpc::send_initial(&sender_clone, &pid, None) {
                tracing::warn!(
                    "[{}] Failed to send initial to newly connected plugin: {}",
                    pid,
                    e
                );
            }
        });
    }
}

fn handle_register(ctx: &mut HandlerContext<'_>, params: Option<&Value>) -> Result<Value> {
    let params: RegisterParams = params
        .ok_or_else(|| DaemonError::InvalidParams("Missing params".to_string()))
        .and_then(|v| serde_json::from_value(v.clone()).map_err(DaemonError::Json))?;

    if ctx.is_registered() {
        return Err(DaemonError::AlreadyRegistered);
    }

    let session_id = ctx.client_id.clone();
    let mut info = ClientInfo::with_id(session_id.clone());
    info.register(params.role.clone());

    let session = match params.role {
        ClientRole::Ui { ref name } => {
            if let Some(old_ui) = ctx.active_ui.take() {
                debug!("UI {} is being replaced by {}", old_ui, session_id);
            }
            *ctx.active_ui = Some(session_id.clone());

            if let Some(tx) = ctx.client_senders.get(&session_id) {
                forward_cached_statuses(tx, ctx.plugin_statuses, &session_id);
            }

            Session::Ui(UiSession::new(info, name.clone()))
        }
        ClientRole::Control => Session::Control(ControlSession::new(info)),
        ClientRole::Plugin { ref id, .. } => {
            if let Some(sender) = ctx.client_sender.clone() {
                register_plugin_in_registry(ctx, id, &session_id, &sender);
            }
            Session::Plugin(PluginSession::new(info, id.clone()))
        }
    };

    ctx.clients.insert(session_id.clone(), session.clone());

    match &session {
        Session::Ui(_) => debug!(
            "UI client registered: {} (name: {})",
            session.id(),
            session.ui_name().unwrap_or("unknown")
        ),
        Session::Control(_) => debug!("Control client registered: {}", session.id()),
        Session::Plugin(_) => debug!(
            "Plugin client registered: {} (plugin: {})",
            session.id(),
            session.plugin_id().unwrap_or("unknown")
        ),
        Session::Pending(_) => unreachable!("Session should not be pending after registration"),
    }

    let result = RegisterResult {
        session_id: session_id.to_string(),
    };
    Ok(serde_json::to_value(result)?)
}

async fn handle_back(ctx: &mut HandlerContext<'_>) -> Result<()> {
    let active_plugin = ctx
        .core
        .state()
        .active_plugin
        .as_ref()
        .map(|p| (p.id.clone(), p.context.clone()));

    ctx.core.process(CoreEvent::Back).await;

    if let Some((plugin_id, context)) = active_plugin
        && ctx.plugin_registry.is_connected(&plugin_id)
        && let Some(sender) = ctx.plugin_registry.get_plugin_sender(&plugin_id)
    {
        let sender_clone = sender.clone();
        let pid_clone = plugin_id.clone();

        debug!(
            "[{}] Forwarding __back__ action to socket plugin with context={:?}",
            plugin_id, context
        );

        tokio::spawn(async move {
            if let Err(e) = plugin_rpc::send_action(
                &sender_clone,
                &pid_clone,
                "__back__".to_string(),
                None,
                context,
                None,
            ) {
                trace!("[{}] Failed to send __back__ to plugin: {}", pid_clone, e);
            }
        });
    }

    Ok(())
}

async fn handle_cancel(ctx: &mut HandlerContext<'_>) -> Result<()> {
    ctx.core.process(CoreEvent::Cancel).await;
    Ok(())
}

async fn handle_launcher_opened(ctx: &mut HandlerContext<'_>) -> Result<()> {
    ctx.core.process(CoreEvent::LauncherOpened).await;
    Ok(())
}

async fn handle_launcher_closed(ctx: &mut HandlerContext<'_>) -> Result<()> {
    // Stop any on-demand (non-background) plugins when launcher closes
    let active_plugin_id = ctx
        .core
        .state()
        .active_plugin
        .as_ref()
        .map(|p| p.id.clone());

    if let Some(ref plugin_id) = active_plugin_id
        && let Some(plugin) = ctx.plugin_registry.get_socket_plugin(plugin_id)
        && !plugin.is_background
    {
        debug!(
            "[{}] Stopping on-demand plugin (launcher closed)",
            plugin_id
        );
        ctx.plugin_spawner.stop_plugin(plugin_id).await;
    }

    ctx.core.process(CoreEvent::LauncherClosed).await;
    Ok(())
}

// Handler dispatch requires consistent Result<Value> signature
#[allow(clippy::unnecessary_wraps)]
fn handle_toggle(ctx: &mut HandlerContext<'_>) -> Result<Value> {
    // Send Toggle notification to the active UI
    // The GTK client will handle this with intuitive mode logic
    // (deciding whether to close, minimize to FAB, or open based on hasUsedMinimize preference)
    let Some(ui_id) = ctx.active_ui.as_ref() else {
        warn!("Toggle requested but no active UI connected");
        return Ok(serde_json::json!({
            "status": "no_ui",
            "message": "No UI connected. Start hamr-gtk first."
        }));
    };

    let Some(tx) = ctx.client_senders.get(ui_id) else {
        warn!("Toggle: UI {} registered but sender not found", ui_id);
        return Ok(serde_json::json!({
            "status": "error",
            "message": "UI registered but connection lost"
        }));
    };

    let notification = Notification::new("toggle", None);
    if let Err(e) = tx.send(Message::Notification(notification)) {
        warn!("Failed to send toggle to UI {}: {}", ui_id, e);
        return Ok(serde_json::json!({
            "status": "error",
            "message": format!("Failed to send toggle: {}", e)
        }));
    }

    debug!("Toggle notification sent to UI {}", ui_id);
    Ok(serde_json::json!({"status": "ok"}))
}

async fn handle_show(ctx: &mut HandlerContext<'_>) -> Result<Value> {
    ctx.core.process(CoreEvent::LauncherOpened).await;
    Ok(serde_json::json!({"status": "ok"}))
}

async fn handle_hide(ctx: &mut HandlerContext<'_>) -> Result<Value> {
    ctx.core.process(CoreEvent::LauncherClosed).await;
    Ok(serde_json::json!({"status": "ok"}))
}

async fn handle_open_plugin(ctx: &mut HandlerContext<'_>, params: Option<&Value>) -> Result<Value> {
    #[derive(serde::Deserialize)]
    struct Params {
        plugin_id: String,
    }

    let params: Params = params
        .ok_or_else(|| DaemonError::InvalidParams("Missing params".to_string()))
        .and_then(|v| serde_json::from_value(v.clone()).map_err(DaemonError::Json))?;

    let plugin_id = params.plugin_id.clone();

    // Toggle logic: if launcher is open with same plugin, close instead
    let state = ctx.core.state();
    let was_already_open = state.is_open;
    if was_already_open
        && let Some(ref active) = state.active_plugin
        && active.id == plugin_id
    {
        debug!(
            "[{}] Toggle: plugin already open, closing launcher",
            plugin_id
        );
        // Close plugin first to clear state (prevents restore on next open)
        ctx.core.process(CoreEvent::ClosePlugin).await;
        ctx.core.process(CoreEvent::LauncherClosed).await;
        return Ok(serde_json::json!({"status": "toggled", "action": "closed"}));
    }

    let mut spawned_on_demand = false;

    if let Some(plugin) = ctx.plugin_registry.get_socket_plugin(&plugin_id)
        && !plugin.is_background
        && !ctx.plugin_spawner.is_spawned(&plugin_id)
        && !ctx.plugin_registry.is_connected(&plugin_id)
    {
        debug!("[{}] Spawning on demand", plugin_id);

        let dirs = ctx.core.dirs();
        let working_dir = if dirs.builtin_plugins.join(&plugin_id).exists() {
            dirs.builtin_plugins.join(&plugin_id)
        } else if dirs.user_plugins.join(&plugin_id).exists() {
            dirs.user_plugins.join(&plugin_id)
        } else {
            warn!("[{}] Plugin directory not found, skipping spawn", plugin_id);
            ctx.core
                .process(CoreEvent::OpenPlugin {
                    plugin_id: plugin_id.clone(),
                })
                .await;
            return Ok(serde_json::json!({"status": "ok"}));
        };

        let plugin_clone = plugin.clone();
        if let Err(e) = ctx.plugin_spawner.spawn_in_dir(&plugin_clone, &working_dir) {
            warn!("[{}] Failed to spawn on-demand plugin: {}", plugin_id, e);
        } else {
            spawned_on_demand = true;
        }
    }

    // Show the launcher first (only if not already open), then open the plugin
    if !was_already_open {
        ctx.core.process(CoreEvent::LauncherOpened).await;
    }
    ctx.core
        .process(CoreEvent::OpenPlugin {
            plugin_id: plugin_id.clone(),
        })
        .await;

    if !spawned_on_demand
        && ctx.plugin_registry.is_connected(&plugin_id)
        && let Some(sender) = ctx.plugin_registry.get_plugin_sender(&plugin_id)
    {
        let sender_clone = sender.clone();
        let pid = plugin_id.clone();
        tokio::spawn(async move {
            if let Err(e) = plugin_rpc::send_initial(&sender_clone, &pid, None) {
                trace!("[{}] Failed to send initial to socket plugin: {}", pid, e);
            }
        });
    }

    Ok(serde_json::json!({"status": "ok"}))
}

fn handle_update_status(ctx: &mut HandlerContext<'_>, params: Option<&Value>) -> Result<Value> {
    #[derive(serde::Deserialize)]
    struct Params {
        plugin_id: String,
        status: PluginStatus,
    }

    let params: Params = params
        .ok_or_else(|| DaemonError::InvalidParams("Missing params".to_string()))
        .and_then(|v| serde_json::from_value(v.clone()).map_err(DaemonError::Json))?;

    let plugin_id = params.plugin_id;
    let status = params.status;

    debug!(
        "[{}] CLI status update: badges={}, chips={}, ambient={}, fab={:?}",
        plugin_id,
        status.badges.len(),
        status.chips.len(),
        status.ambient.len(),
        status.fab.as_ref().map(|f| (f.chips.len(), f.badges.len()))
    );

    ctx.plugin_statuses
        .insert(plugin_id.clone(), status.clone());

    if let Some(ui_id) = ctx.active_ui.as_ref()
        && let Some(tx) = ctx.client_senders.get(ui_id)
    {
        let notification = Notification::new(
            "plugin_status_update",
            Some(serde_json::json!({
                "plugin_id": plugin_id,
                "status": status
            })),
        );
        if let Err(e) = tx.send(Message::Notification(notification)) {
            warn!("[{}] Failed to forward status to UI: {}", plugin_id, e);
        }

        let ambient_notification = Notification::new(
            "ambient_update",
            Some(serde_json::json!({
                "plugin_id": plugin_id,
                "items": status.ambient
            })),
        );
        if let Err(e) = tx.send(Message::Notification(ambient_notification)) {
            warn!("[{}] Failed to forward ambient to UI: {}", plugin_id, e);
        }
    }

    Ok(serde_json::json!({"status": "ok"}))
}

// Handler dispatch requires consistent Result<Value> signature
#[allow(clippy::unnecessary_wraps)]
fn handle_status(ctx: &mut HandlerContext<'_>) -> Result<Value> {
    let state = ctx.core.state();
    let index_stats = ctx.core.index_stats();

    Ok(serde_json::json!({
        "is_open": state.is_open,
        "query": state.query,
        "active_plugin": state.active_plugin.as_ref().map(|p| &p.id),
        "input_mode": format!("{:?}", state.input_mode).to_lowercase(),
        "busy": state.busy,
        "index": {
            "plugin_count": index_stats.plugin_count,
            "item_count": index_stats.item_count,
        }
    }))
}

// Handler dispatch requires consistent Result<Value> signature
#[allow(clippy::unnecessary_wraps)]
fn handle_shutdown(_ctx: &mut HandlerContext<'_>) -> Result<Value> {
    debug!("Shutdown requested");
    Ok(serde_json::json!({"status": "shutting_down"}))
}

fn handle_reload_plugins(ctx: &mut HandlerContext<'_>) -> Result<Value> {
    debug!("Reload plugins requested");
    ctx.core.reload_plugins()?;
    Ok(serde_json::json!({"status": "ok"}))
}

async fn handle_ambient_action(
    ctx: &mut HandlerContext<'_>,
    params: Option<&Value>,
) -> Result<Value> {
    #[derive(serde::Deserialize)]
    struct Params {
        plugin_id: String,
        item_id: String,
        #[serde(default)]
        action: Option<String>,
    }

    let params: Params = params
        .ok_or_else(|| DaemonError::InvalidParams("Missing params".to_string()))
        .and_then(|v| serde_json::from_value(v.clone()).map_err(DaemonError::Json))?;

    let plugin_id = params.plugin_id.clone();
    let item_id = params.item_id.clone();
    let action = params.action.clone();

    debug!(
        "[{}] handle_ambient_action: item_id={}, action={:?}",
        plugin_id, item_id, action
    );

    ctx.core
        .process(CoreEvent::AmbientAction {
            plugin_id: plugin_id.clone(),
            item_id: item_id.clone(),
            action: action.clone(),
        })
        .await;

    let is_connected = ctx.plugin_registry.is_connected(&plugin_id);
    let has_sender = ctx.plugin_registry.get_plugin_sender(&plugin_id).is_some();
    debug!(
        "[{}] ambient_action: is_connected={}, has_sender={}",
        plugin_id, is_connected, has_sender
    );

    if is_connected && let Some(sender) = ctx.plugin_registry.get_plugin_sender(&plugin_id) {
        debug!(
            "[{}] Forwarding ambient action to socket plugin: {:?}",
            plugin_id, action
        );
        let sender_clone = sender.clone();
        let pid = plugin_id.clone();
        let iid = item_id.clone();
        let act = action.clone();
        tokio::spawn(async move {
            if let Err(e) = plugin_rpc::send_action(
                &sender_clone,
                &pid,
                iid,
                act,
                None,
                Some("ambient".to_string()), // source: ambient
            ) {
                warn!("[{}] Failed to send action to socket plugin: {}", pid, e);
            }
        });
    } else {
        debug!(
            "[{}] NOT forwarding ambient action: is_connected={}, has_sender={}",
            plugin_id, is_connected, has_sender
        );
    }

    Ok(serde_json::json!({"status": "ok"}))
}

async fn handle_dismiss_ambient(
    ctx: &mut HandlerContext<'_>,
    params: Option<&Value>,
) -> Result<()> {
    #[derive(serde::Deserialize)]
    struct Params {
        plugin_id: String,
        item_id: String,
    }

    let params: Params = params
        .ok_or_else(|| DaemonError::InvalidParams("Missing params".to_string()))
        .and_then(|v| serde_json::from_value(v.clone()).map_err(DaemonError::Json))?;

    let plugin_id = params.plugin_id.clone();
    let item_id = params.item_id.clone();

    debug!(
        "[{}] handle_dismiss_ambient: item_id={}",
        plugin_id, item_id
    );

    ctx.core
        .process(CoreEvent::DismissAmbient {
            plugin_id: plugin_id.clone(),
            item_id: item_id.clone(),
        })
        .await;

    let is_connected = ctx.plugin_registry.is_connected(&plugin_id);
    let has_sender = ctx.plugin_registry.get_plugin_sender(&plugin_id).is_some();
    debug!(
        "[{}] dismiss: is_connected={}, has_sender={}",
        plugin_id, is_connected, has_sender
    );

    if is_connected && let Some(sender) = ctx.plugin_registry.get_plugin_sender(&plugin_id) {
        debug!("[{}] Forwarding dismiss to socket plugin", plugin_id);
        let sender_clone = sender.clone();
        let pid = plugin_id.clone();
        let iid = item_id.clone();
        tokio::spawn(async move {
            if let Err(e) = plugin_rpc::send_action(
                &sender_clone,
                &pid,
                iid,
                Some("__dismiss__".to_string()),
                None,
                Some("ambient".to_string()), // source: ambient
            ) {
                warn!("[{}] Failed to send dismiss to socket plugin: {}", pid, e);
            }
        });
    } else {
        debug!(
            "[{}] NOT forwarding dismiss: is_connected={}, has_sender={}",
            plugin_id, is_connected, has_sender
        );
    }

    Ok(())
}

async fn handle_plugin_action_triggered(
    ctx: &mut HandlerContext<'_>,
    params: Option<&Value>,
) -> Result<()> {
    let action_id = params
        .and_then(|p| p.get("action_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| DaemonError::InvalidParams("Missing action_id".to_string()))?
        .to_string();

    // For socket plugins, forward the action directly
    if let Some(active_plugin) = ctx.core.state().active_plugin.as_ref()
        && ctx.plugin_registry.is_connected(&active_plugin.id)
    {
        let plugin_id = active_plugin.id.clone();
        if let Some(sender) = ctx.plugin_registry.get_plugin_sender(&plugin_id) {
            let sender_clone = sender.clone();
            let action_clone = action_id.clone();
            let pid_clone = plugin_id.clone();

            debug!(
                "[{}] Forwarding plugin action '{}' to socket plugin",
                plugin_id, action_id
            );

            tokio::spawn(async move {
                if let Err(e) = plugin_rpc::send_action(
                    &sender_clone,
                    &pid_clone,
                    "__plugin__".to_string(),
                    Some(action_clone),
                    None,
                    None,
                ) {
                    trace!("[{}] Failed to send plugin action: {}", pid_clone, e);
                }
            });
            return Ok(());
        }
    }

    // Fallback to core handling for non-socket plugins
    ctx.core
        .process(CoreEvent::PluginActionTriggered { action_id })
        .await;
    Ok(())
}

async fn handle_close_plugin(ctx: &mut HandlerContext<'_>) -> Result<()> {
    let active_plugin_id = ctx
        .core
        .state()
        .active_plugin
        .as_ref()
        .map(|p| p.id.clone());

    ctx.core.process(CoreEvent::ClosePlugin).await;

    if let Some(ref plugin_id) = active_plugin_id
        && let Some(plugin) = ctx.plugin_registry.get_socket_plugin(plugin_id)
        && !plugin.is_background
    {
        debug!("[{}] Stopping on-demand socket plugin", plugin_id);
        ctx.plugin_spawner.stop_plugin(plugin_id).await;
    }

    Ok(())
}

fn handle_index_stats(ctx: &mut HandlerContext<'_>) -> std::result::Result<Value, DaemonError> {
    let stats = ctx.core.index_stats();
    serde_json::to_value(stats).map_err(DaemonError::Json)
}

// Handler dispatch requires consistent Result<Value> signature
#[allow(clippy::unnecessary_wraps)]
fn handle_list_plugins(ctx: &mut HandlerContext<'_>) -> Result<Value> {
    let plugins: Vec<_> = ctx
        .plugin_registry
        .all_plugins()
        .map(|p| {
            serde_json::json!({
                "id": p.manifest.id,
                "name": p.manifest.name,
                "description": p.manifest.description,
                "icon": p.manifest.icon,
                "prefix": p.manifest.prefix,
                "is_socket": p.is_socket,
                "connected": ctx.plugin_registry.is_connected(&p.id),
            })
        })
        .collect();

    Ok(serde_json::json!({ "plugins": plugins }))
}
