//! Socket server implementation for the hamr daemon.
//!
//! This module provides the main socket server that accepts connections from
//! UI clients, control commands, and plugin daemons.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use hamr_core::plugin::{PluginResponse, plugin_response_to_updates};
use hamr_core::{CoreUpdate, HamrCore};
use hamr_rpc::PluginManifest;
use hamr_rpc::client::socket_path;
use hamr_rpc::protocol::{Message, Notification, Response};
use hamr_rpc::transport::JsonRpcCodec;
use hamr_types::{CoreEvent, DisplayHint, SearchResult};
use serde_json::Value;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio_util::codec::Framed;
use tracing::{debug, error, info, trace, warn};

const HEALTH_CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);
const INDEX_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);
const CONFIG_RELOAD_LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

use crate::config_watcher::spawn_config_watcher;
use crate::error::Result;
use crate::handlers::{HandlerContext, handle_notification, handle_request};
use crate::plugin_spawner::PluginSpawner;
use crate::plugin_watcher::PluginWatcher;
use crate::registry::{DiscoveredPlugin, PluginRegistry};
use crate::session::{Session, SessionId};

use hamr_core::plugin::{ChecksumsData, PluginVerifyStatus};

pub struct DaemonState {
    pub core: HamrCore,
    pub clients: HashMap<SessionId, Session>,
    pub active_ui: Option<SessionId>,
    pub client_senders: HashMap<SessionId, mpsc::UnboundedSender<Message>>,
    pub shutdown: bool,
    pub plugin_registry: PluginRegistry,
    pub plugin_spawner: PluginSpawner,
    pub plugin_statuses: HashMap<String, hamr_types::PluginStatus>,
}

impl DaemonState {
    #[must_use]
    pub fn new(core: HamrCore) -> Self {
        Self {
            core,
            clients: HashMap::new(),
            active_ui: None,
            client_senders: HashMap::new(),
            shutdown: false,
            plugin_registry: PluginRegistry::new(),
            plugin_spawner: PluginSpawner::new(),
            plugin_statuses: HashMap::new(),
        }
    }

    pub fn remove_client(&mut self, session_id: &SessionId) -> bool {
        self.clients.remove(session_id);
        self.client_senders.remove(session_id);

        self.plugin_registry.unregister_session(session_id);

        if self.active_ui.as_ref() == Some(session_id) {
            self.active_ui = None;
            debug!("Active UI disconnected: {}", session_id);
            return true;
        }
        false
    }

    pub fn create_handler_context<'a>(
        &'a mut self,
        session_id: &'a SessionId,
    ) -> HandlerContext<'a> {
        let client_sender = self.client_senders.get(session_id).cloned();
        HandlerContext {
            core: &mut self.core,
            clients: &mut self.clients,
            active_ui: &mut self.active_ui,
            client_id: session_id,
            plugin_registry: &mut self.plugin_registry,
            plugin_spawner: &mut self.plugin_spawner,
            client_sender,
            client_senders: &self.client_senders,
            plugin_statuses: &mut self.plugin_statuses,
        }
    }
}

// Sequential plugin discovery with checksum verification - splitting would fragment the logic
#[allow(clippy::too_many_lines)]
fn discover_plugins(builtin_dir: &Path, user_dir: &Path, registry: &mut PluginRegistry) {
    let checksums_path = builtin_dir.join("checksums.json");
    let checksums = ChecksumsData::load(&checksums_path);

    if let Some(ref cs) = checksums {
        info!(
            "Loaded plugin checksums ({} plugins) from {:?}",
            cs.plugin_count(),
            checksums_path
        );
    } else {
        debug!(
            "No checksums.json found at {:?}, skipping verification",
            checksums_path
        );
    }

    let plugin_dirs = [builtin_dir, user_dir];

    for plugin_base in plugin_dirs {
        if !plugin_base.exists() {
            debug!("Plugin directory does not exist: {:?}", plugin_base);
            continue;
        }

        info!("Discovering plugins from {:?}", plugin_base);

        let entries = match std::fs::read_dir(plugin_base) {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to read plugin directory {:?}: {}", plugin_base, e);
                continue;
            }
        };

        for entry in entries.flatten() {
            let plugin_path = entry.path();
            if !plugin_path.is_dir() {
                continue;
            }

            let manifest_path = plugin_path.join("manifest.json");
            if !manifest_path.exists() {
                continue;
            }

            let manifest_content = match std::fs::read_to_string(&manifest_path) {
                Ok(c) => c,
                Err(e) => {
                    warn!("Failed to read manifest {:?}: {}", manifest_path, e);
                    continue;
                }
            };

            let manifest: serde_json::Value = match serde_json::from_str(&manifest_content) {
                Ok(m) => m,
                Err(e) => {
                    warn!("Failed to parse manifest {:?}: {}", manifest_path, e);
                    continue;
                }
            };

            let plugin_id = plugin_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            if let Some(ref cs) = checksums {
                match cs.verify_plugin(&plugin_id, &plugin_path) {
                    PluginVerifyStatus::Verified => {
                        debug!("[{}] Plugin checksum verified", plugin_id);
                    }
                    PluginVerifyStatus::Modified(files) => {
                        warn!(
                            "[{}] Plugin files modified (security warning): {:?}",
                            plugin_id, files
                        );
                    }
                    PluginVerifyStatus::Unknown => {
                        debug!(
                            "[{}] Plugin not in checksums (user-installed or new)",
                            plugin_id
                        );
                    }
                }
            }

            let name = manifest["name"].as_str().unwrap_or(&plugin_id).to_string();
            let description = manifest["description"].as_str().map(String::from);
            let icon = manifest["icon"].as_str().map(String::from);
            let prefix = manifest["prefix"].as_str().map(String::from);

            let handler = manifest.get("handler");
            let is_socket = handler
                .and_then(|h| h.get("type"))
                .and_then(|t| t.as_str())
                .is_some_and(|t| t == "socket");

            let spawn_command = if is_socket {
                handler
                    .and_then(|h| h.get("command"))
                    .and_then(|c| c.as_str())
                    .map(String::from)
            } else {
                None
            };

            let daemon = manifest.get("daemon");
            let is_background = daemon
                .and_then(|d| d.get("background"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);

            let rpc_manifest = PluginManifest {
                id: plugin_id.clone(),
                name,
                description,
                icon,
                prefix,
                priority: 0,
            };

            let discovered = DiscoveredPlugin {
                id: plugin_id.clone(),
                manifest: rpc_manifest,
                is_socket,
                spawn_command: spawn_command.clone(),
                is_background,
            };

            info!(
                "Discovered plugin: {} (socket: {}, background: {}, command: {:?})",
                plugin_id, is_socket, is_background, spawn_command
            );

            registry.register_discovered(discovered);
        }
    }
}

fn collect_background_plugins_to_spawn(
    registry: &PluginRegistry,
    builtin_dir: &Path,
    user_dir: &Path,
) -> Vec<(DiscoveredPlugin, PathBuf)> {
    registry
        .pending_background_plugins()
        .filter_map(|plugin| {
            plugin.spawn_command.as_ref()?;

            let working_dir = if builtin_dir.join(&plugin.id).exists() {
                builtin_dir.join(&plugin.id)
            } else if user_dir.join(&plugin.id).exists() {
                user_dir.join(&plugin.id)
            } else {
                warn!("[{}] Plugin directory not found", plugin.id);
                return None;
            };

            Some((plugin.clone(), working_dir))
        })
        .collect()
}

fn spawn_socket_plugins(
    spawner: &mut PluginSpawner,
    plugins_to_spawn: Vec<(DiscoveredPlugin, PathBuf)>,
) {
    for (plugin, working_dir) in plugins_to_spawn {
        info!(
            "[{}] Spawning socket plugin from {:?}",
            plugin.id, working_dir
        );

        if let Err(e) = spawner.spawn_in_dir(&plugin, &working_dir) {
            error!("[{}] Failed to spawn: {}", plugin.id, e);
        }
    }
}

async fn spawn_config_watcher_task(config_path: PathBuf, state: Arc<RwLock<DaemonState>>) {
    let (reload_tx, mut reload_rx) = mpsc::unbounded_channel::<()>();

    spawn_config_watcher(config_path, reload_tx);

    while reload_rx.recv().await.is_some() {
        debug!("Config reload event received");

        {
            let state_guard = tokio::time::timeout(CONFIG_RELOAD_LOCK_TIMEOUT, state.write()).await;

            let Ok(mut state_guard) = state_guard else {
                error!("Config reload timed out waiting for write lock");
                continue;
            };
            if let Err(e) = state_guard.core.reload_config() {
                error!("Failed to reload config: {}", e);
                continue;
            }
            info!("Config reloaded successfully");

            if let Some(ui_id) = &state_guard.active_ui {
                let notification = Notification::new("config_reloaded", None);
                if let Some(tx) = state_guard.client_senders.get(ui_id)
                    && let Err(e) = tx.send(Message::Notification(notification))
                {
                    warn!("Failed to send config_reloaded notification: {}", e);
                }
            } else {
                debug!("No active UI to notify of config reload");
            }
        }
    }

    debug!("Config watcher task ended");
}

async fn plugin_health_monitor(state: Arc<RwLock<DaemonState>>) {
    let mut interval = tokio::time::interval(HEALTH_CHECK_INTERVAL);

    loop {
        interval.tick().await;

        let shutdown = {
            let state_guard = state.read().await;
            state_guard.shutdown
        };

        if shutdown {
            break;
        }

        {
            let mut state_guard = state.write().await;
            state_guard.plugin_spawner.check_and_restart().await;
        }
    }

    debug!("Plugin health monitor stopped");
}

/// Background task for debounced index persistence.
/// Only saves when:
/// 1. Index is dirty (has unsaved changes)
/// 2. At least 1 second has passed since the last modification (debounce)
///
/// This matches QML hamr behavior - saves 1 second after the last change,
/// avoiding excessive writes during continuous interactions like slider adjustments.
// u128 millis fits in u64 for realistic timestamps
#[allow(clippy::cast_possible_truncation)]
async fn index_saver(state: Arc<RwLock<DaemonState>>) {
    const DEBOUNCE_MS: u64 = 1000;
    let mut interval = tokio::time::interval(INDEX_POLL_INTERVAL);

    loop {
        interval.tick().await;

        let shutdown = {
            let state_guard = state.read().await;
            state_guard.shutdown
        };

        if shutdown {
            let mut state_guard = state.write().await;
            if state_guard.core.is_index_dirty() {
                if let Err(e) = state_guard.core.save_index() {
                    error!("Failed to save index on shutdown: {}", e);
                } else {
                    info!("Index saved on shutdown");
                }
            }
            break;
        }

        {
            let mut state_guard = state.write().await;
            if state_guard.core.is_index_dirty() {
                let last_dirty = state_guard.core.last_index_dirty_at();
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);

                if now.saturating_sub(last_dirty) >= DEBOUNCE_MS {
                    if let Err(e) = state_guard.core.save_index() {
                        error!("Failed to save index: {}", e);
                    } else {
                        trace!(
                            "Index saved (debounced, {}ms since last change)",
                            now - last_dirty
                        );
                    }
                }
            }
        }
    }

    debug!("Index saver stopped");
}

/// Run the daemon server.
///
/// # Errors
///
/// Returns an error if socket setup, core initialization, or server execution fails.
pub async fn run(custom_socket_path: Option<PathBuf>) -> Result<()> {
    let path = custom_socket_path.unwrap_or_else(socket_path);

    // Clean up stale socket if it exists
    cleanup_stale_socket(&path).await?;

    // Create the socket listener
    let listener = UnixListener::bind(&path)?;
    info!("Daemon listening on {:?}", path);

    // Initialize core and get update receiver
    let (core, update_rx) = HamrCore::new()?;

    // Initialize daemon state (separate from update_rx)
    let state = Arc::new(RwLock::new(DaemonState::new(core)));
    let update_rx = Arc::new(Mutex::new(update_rx));

    // Start the core
    {
        let mut state_guard = state.write().await;
        state_guard.core.start()?;
    }

    let plugins_to_spawn = {
        let mut state_guard = state.write().await;
        let dirs = state_guard.core.dirs().clone();
        discover_plugins(
            &dirs.builtin_plugins,
            &dirs.user_plugins,
            &mut state_guard.plugin_registry,
        );

        collect_background_plugins_to_spawn(
            &state_guard.plugin_registry,
            &dirs.builtin_plugins,
            &dirs.user_plugins,
        )
    };

    {
        let mut state_guard = state.write().await;
        spawn_socket_plugins(&mut state_guard.plugin_spawner, plugins_to_spawn);
    }

    let state_clone = state.clone();
    let update_rx_clone = update_rx.clone();
    tokio::spawn(async move {
        forward_updates(state_clone, update_rx_clone).await;
    });

    let config_path = {
        let state_guard = state.read().await;
        state_guard.core.dirs().config_file.clone()
    };
    let state_clone = state.clone();
    tokio::spawn(async move {
        spawn_config_watcher_task(config_path, state_clone).await;
    });

    let state_clone = state.clone();
    tokio::spawn(async move {
        plugin_health_monitor(state_clone).await;
    });

    let state_clone = state.clone();
    tokio::spawn(async move {
        index_saver(state_clone).await;
    });

    // Plugin directory watcher for hot-reload
    let (plugin_reload_tx, mut plugin_reload_rx) = mpsc::unbounded_channel::<()>();
    let plugin_dirs = {
        let state_guard = state.read().await;
        let dirs = state_guard.core.dirs();
        vec![dirs.builtin_plugins.clone(), dirs.user_plugins.clone()]
    };

    let _plugin_watcher = PluginWatcher::spawn(plugin_dirs, plugin_reload_tx);

    let state_clone = state.clone();
    tokio::spawn(async move {
        while plugin_reload_rx.recv().await.is_some() {
            info!("Plugin directory change detected, reloading plugins");
            let mut state_guard = state_clone.write().await;
            if let Err(e) = state_guard.core.reload_plugins() {
                error!("Plugin reload failed: {}", e);
            }
        }
    });

    info!("Ready to accept connections");
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                debug!("Accepted connection");
                let state = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, state).await {
                        error!("Connection error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Accept error: {}", e);
            }
        }

        {
            let state_guard = state.read().await;
            if state_guard.shutdown {
                info!("Shutdown requested, stopping server");
                break;
            }
        }
    }

    if path.exists()
        && let Err(e) = std::fs::remove_file(&path)
    {
        warn!("Failed to remove socket file {:?}: {}", path, e);
    }

    Ok(())
}

async fn cleanup_stale_socket(path: &Path) -> Result<()> {
    if path.exists() {
        if UnixStream::connect(path).await.is_ok() {
            return Err(crate::error::DaemonError::Io(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                "Another daemon is already running",
            )));
        }
        info!("Removing stale socket at {}", path.display());
        std::fs::remove_file(path)?;
    }
    Ok(())
}

async fn handle_connection(stream: UnixStream, state: Arc<RwLock<DaemonState>>) -> Result<()> {
    let framed = Framed::new(stream, JsonRpcCodec::new());
    let (mut sink, mut stream) = framed.split();

    let session_id = SessionId::new();
    debug!("New connection: {}", session_id);

    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    {
        let mut state_guard = state.write().await;
        state_guard.clients.insert(
            session_id.clone(),
            Session::Pending(crate::session::ClientInfo::with_id(session_id.clone())),
        );
        state_guard.client_senders.insert(session_id.clone(), tx);
    }

    let session_id_clone = session_id.clone();
    let send_task = tokio::spawn(async move {
        trace!("[{}] send_task started", session_id_clone);
        while let Some(msg) = rx.recv().await {
            match &msg {
                Message::Request(r) => trace!(
                    "[{}] send_task: sending request method={}",
                    session_id_clone, r.method
                ),
                Message::Response(r) => trace!(
                    "[{}] send_task: sending response id={:?}",
                    session_id_clone, r.id
                ),
                Message::Notification(n) => trace!(
                    "[{}] send_task: sending notification method={}",
                    session_id_clone, n.method
                ),
            }
            if let Err(e) = sink.send(msg).await {
                warn!("Failed to send to {}: {}", session_id_clone, e);
                break;
            }
        }
        trace!("[{}] send_task ended", session_id_clone);
    });

    while let Some(result) = stream.next().await {
        match result {
            Ok(msg) => {
                let response = process_message(&session_id, msg, &state).await;

                if let Some(resp) = response {
                    let state_guard = state.read().await;
                    if let Some(tx) = state_guard.client_senders.get(&session_id)
                        && tx.send(resp).is_err()
                    {
                        break;
                    }
                }

                let state_guard = state.read().await;
                if state_guard.shutdown {
                    break;
                }
            }
            Err(e) => {
                warn!("Read error from {}: {}", session_id, e);
                break;
            }
        }
    }

    debug!("Connection closed: {}", session_id);
    let was_active_ui = {
        let mut state_guard = state.write().await;
        state_guard.remove_client(&session_id)
    };

    if was_active_ui {
        debug!("Auto-closing launcher due to UI disconnect");
        let mut state_guard = state.write().await;
        state_guard
            .core
            .process(hamr_rpc::CoreEvent::LauncherClosed)
            .await;
    }

    send_task.abort();

    Ok(())
}

async fn process_message(
    session_id: &SessionId,
    msg: Message,
    state: &Arc<RwLock<DaemonState>>,
) -> Option<Message> {
    match msg {
        Message::Request(ref request) if request.id.is_none() => {
            debug!(
                "Processing notification (from Request): method={}",
                request.method
            );
            let notification = Notification::new(&request.method, request.params.clone());

            let mut state_guard = state.write().await;
            let mut ctx = state_guard.create_handler_context(session_id);

            if let Err(e) = handle_notification(&mut ctx, &notification).await {
                warn!("Notification handler error: {}", e);
            }

            None
        }

        Message::Request(request) => {
            trace!(
                "Processing request: method={}, id={:?}",
                request.method, request.id
            );
            let response = {
                let mut state_guard = state.write().await;
                let mut ctx = state_guard.create_handler_context(session_id);

                match handle_request(&mut ctx, &request).await {
                    Ok(resp) => resp,
                    Err(err) => Response::error(
                        request.id.clone().unwrap_or(hamr_rpc::RequestId::Number(0)),
                        err,
                    ),
                }
            };

            if request.method == "shutdown" {
                let mut state_guard = state.write().await;
                state_guard.shutdown = true;
            }

            Some(Message::Response(response))
        }

        Message::Notification(notification) => {
            debug!("Processing notification: method={}", notification.method);

            if notification.method == "shutdown" {
                let mut state_guard = state.write().await;
                state_guard.shutdown = true;
                debug!("Shutdown notification received, setting shutdown flag");
                return None;
            }

            let mut state_guard = state.write().await;
            let mut ctx = state_guard.create_handler_context(session_id);

            if let Err(e) = handle_notification(&mut ctx, &notification).await {
                warn!("Notification handler error: {}", e);
            }

            None
        }

        Message::Response(resp) => {
            handle_plugin_response(session_id, resp, state).await;
            None
        }
    }
}

/// Handle a response message from a plugin, forwarding updates to the active UI.
async fn handle_plugin_response(
    session_id: &SessionId,
    resp: Response,
    state: &Arc<RwLock<DaemonState>>,
) {
    let (is_plugin, plugin_id, ui_sender, has_active_ui) = {
        let state_guard = state.read().await;
        let session = state_guard.clients.get(session_id);
        let is_plugin = session.is_some_and(|s| matches!(s, Session::Plugin(_)));
        let plugin_id = session.and_then(|s| s.plugin_id().map(String::from));
        let active_ui = state_guard.active_ui.clone();
        let ui_sender = active_ui
            .as_ref()
            .and_then(|ui_id| state_guard.client_senders.get(ui_id).cloned());
        (is_plugin, plugin_id, ui_sender, active_ui.is_some())
    };

    if !is_plugin {
        warn!("Unexpected response from non-plugin client: {:?}", resp.id);
        return;
    }

    let Some(plugin_id) = plugin_id else {
        return;
    };

    debug!(
        "[{}] Received response from plugin: request_id={:?}, has_active_ui={}, has_results={}",
        plugin_id,
        resp.id,
        has_active_ui,
        resp.result
            .as_ref()
            .is_some_and(|r| r.get("results").is_some())
    );

    if let Some(result) = &resp.result
        && let Some(context_value) = result.get("context")
    {
        let context = context_value.as_str().map(String::from);
        let mut state_guard = state.write().await;
        state_guard
            .core
            .process(CoreEvent::SetContext { context })
            .await;
    }

    let Some(tx) = ui_sender else {
        debug!("[{}] No active UI to forward plugin response", plugin_id);
        return;
    };

    if let Some(result) = &resp.result {
        forward_plugin_result(&plugin_id, result, &tx, state).await;
    } else if let Some(error) = &resp.error {
        debug!(
            "[{}] Plugin returned error: {} ({})",
            plugin_id, error.message, error.code
        );
        let notification = Notification::new(
            "plugin_error",
            Some(serde_json::json!({
                "plugin_id": plugin_id,
                "error": {
                    "code": error.code,
                    "message": error.message,
                    "data": error.data
                }
            })),
        );
        let _ = tx.send(Message::Notification(notification));
    }
}

/// Forward a successful plugin result to the UI as notifications.
async fn forward_plugin_result(
    plugin_id: &str,
    result: &Value,
    tx: &mpsc::UnboundedSender<Message>,
    state: &Arc<RwLock<DaemonState>>,
) {
    if result.get("type").and_then(|v| v.as_str()) == Some("form") {
        debug!(
            "[{}] Raw form JSON: {}",
            plugin_id,
            serde_json::to_string(result).unwrap_or_default()
        );
    }

    match serde_json::from_value::<PluginResponse>(result.clone()) {
        Ok(plugin_response) => {
            debug!(
                "[{}] Deserialized plugin response, forwarding to UI",
                plugin_id
            );
            let updates = plugin_response_to_updates(plugin_id, plugin_response);
            for update in &updates {
                if matches!(update, CoreUpdate::Close) {
                    let mut state_guard = state.write().await;
                    state_guard.core.set_open(false);
                }
            }
            for update in updates {
                let notification = core_update_to_notification(&update);
                let _ = tx.send(Message::Notification(notification));
            }
        }
        Err(e) => {
            let error_msg = format!("Plugin '{plugin_id}' returned invalid response: {e}");
            warn!(
                "[{}] {}. Raw: {}",
                plugin_id,
                error_msg,
                serde_json::to_string(result).unwrap_or_default()
            );
            let error_response = PluginResponse::Error {
                message: error_msg,
                details: None,
            };
            let updates = plugin_response_to_updates(plugin_id, error_response);
            for update in &updates {
                if matches!(update, CoreUpdate::Close) {
                    let mut state_guard = state.write().await;
                    state_guard.core.set_open(false);
                }
            }
            for update in updates {
                let notification = core_update_to_notification(&update);
                let _ = tx.send(Message::Notification(notification));
            }
        }
    }
}

async fn forward_updates(
    state: Arc<RwLock<DaemonState>>,
    update_rx: Arc<Mutex<mpsc::UnboundedReceiver<CoreUpdate>>>,
) {
    debug!("forward_updates task started");
    loop {
        let update = {
            let mut rx = update_rx.lock().await;
            rx.recv().await
        };

        let Some(update) = update else {
            debug!("forward_updates: channel closed");
            break;
        };

        // Cache results in core for frecency auto-indexing (stdio plugins)
        if let CoreUpdate::Results { ref results, .. } = update {
            let mut state_guard = state.write().await;
            state_guard.core.cache_plugin_results(results.clone());
            drop(state_guard);
        }

        if let CoreUpdate::IndexUpdate {
            plugin_id,
            items,
            mode,
            remove,
        } = &update
        {
            let items: Vec<hamr_core::plugin::IndexItem> =
                serde_json::from_value(items.clone()).unwrap_or_default();
            debug!(
                "[{}] Processing IndexUpdate ({} items, mode: {:?})",
                plugin_id,
                items.len(),
                mode
            );
            let mut state_guard = state.write().await;
            state_guard
                .core
                .update_plugin_index(plugin_id, items, mode.as_deref(), remove.clone());
            continue;
        }

        // When Close is requested (e.g., from plugin close: true), update is_open state
        if matches!(update, CoreUpdate::Close) {
            let mut state_guard = state.write().await;
            state_guard.core.set_open(false);
            drop(state_guard);
        }

        if let CoreUpdate::ActivatePlugin { ref plugin_id } = update {
            let mut state_guard = state.write().await;
            state_guard.core.activate_plugin_for_multistep(plugin_id);
            drop(state_guard);
            continue;
        }

        if let CoreUpdate::ContextChanged { ref context } = update {
            let mut state_guard = state.write().await;
            state_guard.core.set_plugin_context(context.clone());
            drop(state_guard);
        }

        let notification = core_update_to_notification(&update);

        let state_guard = state.read().await;
        if let Some(ref ui_id) = state_guard.active_ui {
            debug!(
                "forward_updates: sending {} to UI {}",
                notification.method, ui_id
            );
            if let Some(tx) = state_guard.client_senders.get(ui_id)
                && tx.send(Message::Notification(notification)).is_err()
            {
                warn!("Failed to forward update to UI");
            }
        } else {
            debug!(
                "forward_updates: no active UI, dropping update {:?}",
                std::mem::discriminant(&update)
            );
        }
    }
}

/// Build JSON for a Results notification, including only non-None optional fields
fn build_results_json(
    results: &[SearchResult],
    placeholder: Option<&String>,
    clear_input: Option<bool>,
    input_mode: Option<&String>,
    context: Option<&String>,
    navigate_forward: Option<bool>,
    display_hint: Option<&DisplayHint>,
) -> Value {
    let mut json = serde_json::json!({ "results": results });
    if let Some(p) = placeholder {
        json["placeholder"] = serde_json::json!(p);
    }
    if let Some(c) = clear_input {
        json["clearInput"] = serde_json::json!(c);
    }
    if let Some(m) = input_mode {
        json["inputMode"] = serde_json::json!(m);
    }
    if let Some(c) = context {
        json["context"] = serde_json::json!(c);
    }
    if let Some(nf) = navigate_forward {
        json["navigateForward"] = serde_json::json!(nf);
    }
    if let Some(dh) = display_hint {
        json["displayHint"] = serde_json::json!(dh);
    }
    json
}

// 1:1 variant mapping - each arm is minimal, splitting would fragment the exhaustive match
#[allow(clippy::too_many_lines)]
fn core_update_to_notification(update: &CoreUpdate) -> Notification {
    match update {
        CoreUpdate::Results {
            results,
            placeholder,
            clear_input,
            input_mode,
            context,
            navigate_forward,
            display_hint,
        } => Notification::new(
            "results",
            Some(build_results_json(
                results,
                placeholder.as_ref(),
                *clear_input,
                input_mode.as_ref(),
                context.as_ref(),
                *navigate_forward,
                display_hint.as_ref(),
            )),
        ),
        CoreUpdate::ResultsUpdate { patches } => Notification::new(
            "results_update",
            Some(serde_json::json!({ "patches": patches })),
        ),
        CoreUpdate::Card { card, context } => Notification::new(
            "card",
            Some(serde_json::json!({ "card": card, "context": context })),
        ),
        CoreUpdate::Form { form } => {
            Notification::new("form", Some(serde_json::json!({ "form": form })))
        }
        CoreUpdate::PluginActivated { id, name, icon } => {
            let json = serde_json::json!({ "id": id, "name": name, "icon": icon });
            Notification::new("plugin_activated", Some(json))
        }
        CoreUpdate::PluginDeactivated => Notification::new("plugin_deactivated", None),
        CoreUpdate::Busy { busy } => {
            Notification::new("busy", Some(serde_json::json!({ "busy": busy })))
        }
        CoreUpdate::Error { message } => {
            Notification::new("error", Some(serde_json::json!({ "message": message })))
        }
        CoreUpdate::Prompt { prompt } => {
            Notification::new("prompt", Some(serde_json::json!({ "prompt": prompt })))
        }
        CoreUpdate::Placeholder { placeholder } => Notification::new(
            "placeholder",
            Some(serde_json::json!({ "placeholder": placeholder })),
        ),
        CoreUpdate::Execute { action } => {
            Notification::new("execute", Some(serde_json::json!({ "action": action })))
        }
        CoreUpdate::Close => Notification::new("close", None),
        CoreUpdate::Show => Notification::new("show", None),
        CoreUpdate::Toggle => Notification::new("toggle", None),
        CoreUpdate::ClearInput => Notification::new("clear_input", None),
        CoreUpdate::InputModeChanged { mode } => Notification::new(
            "input_mode_changed",
            Some(serde_json::json!({ "mode": mode })),
        ),
        CoreUpdate::ContextChanged { context } => Notification::new(
            "context_changed",
            Some(serde_json::json!({ "context": context })),
        ),
        CoreUpdate::PluginStatusUpdate { plugin_id, status } => {
            let json = serde_json::json!({ "plugin_id": plugin_id, "status": status });
            Notification::new("plugin_status_update", Some(json))
        }
        CoreUpdate::AmbientUpdate { plugin_id, items } => {
            let json = serde_json::json!({ "plugin_id": plugin_id, "items": items });
            Notification::new("ambient_update", Some(json))
        }
        CoreUpdate::FabUpdate { fab } => {
            Notification::new("fab_update", Some(serde_json::json!({ "fab": fab })))
        }
        CoreUpdate::ImageBrowser { browser } => Notification::new(
            "image_browser",
            Some(serde_json::json!({ "browser": browser })),
        ),
        CoreUpdate::GridBrowser { browser } => Notification::new(
            "grid_browser",
            Some(serde_json::json!({ "browser": browser })),
        ),
        CoreUpdate::PluginActionsUpdate { actions } => Notification::new(
            "plugin_actions_update",
            Some(serde_json::json!({ "actions": actions })),
        ),
        CoreUpdate::NavigationDepthChanged { depth } => Notification::new(
            "navigation_depth_changed",
            Some(serde_json::json!({ "depth": depth })),
        ),
        CoreUpdate::NavigateForward => Notification::new("navigate_forward", None),
        CoreUpdate::NavigateBack => Notification::new("navigate_back", None),
        CoreUpdate::ConfigReloaded => Notification::new("config_reloaded", None),
        CoreUpdate::PluginManagementChanged { active } => Notification::new(
            "plugin_management_changed",
            Some(serde_json::json!({ "active": active })),
        ),
        CoreUpdate::IndexUpdate { .. } => Notification::new("_internal_index", None),
        CoreUpdate::ActivatePlugin { .. } => Notification::new("_internal_activate", None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hamr_types::{
        AmbientItem, CardData, DisplayHint, ExecuteAction, FabOverride, FormData, FormField,
        FormFieldType, GridBrowserData, ImageBrowserData, ImageItem, PluginAction, PluginStatus,
        ResultPatch, SearchResult,
    };

    #[test]
    fn test_core_update_to_notification_results_minimal() {
        let update = CoreUpdate::results(vec![]);
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "results");
        let params = notification.params.unwrap();
        assert!(params.get("results").is_some());
        assert!(params.get("placeholder").is_none());
        assert!(params.get("clearInput").is_none());
    }

    #[test]
    fn test_core_update_to_notification_results_all_fields() {
        let update = CoreUpdate::Results {
            results: vec![SearchResult {
                id: "test".to_string(),
                name: "Test".to_string(),
                ..Default::default()
            }],
            placeholder: Some("Search...".to_string()),
            clear_input: Some(true),
            input_mode: Some("search".to_string()),
            context: Some("test-context".to_string()),
            navigate_forward: Some(true),
            display_hint: Some(DisplayHint::List),
        };
        let notification = core_update_to_notification(&update);
        let params = notification.params.unwrap();
        assert_eq!(params["placeholder"], "Search...");
        assert_eq!(params["clearInput"], true);
        assert_eq!(params["inputMode"], "search");
        assert_eq!(params["context"], "test-context");
        assert_eq!(params["navigateForward"], true);
        assert_eq!(params["displayHint"], "list");
    }

    #[test]
    fn test_core_update_to_notification_results_update() {
        let update = CoreUpdate::ResultsUpdate {
            patches: vec![ResultPatch {
                id: "test".to_string(),
                ..Default::default()
            }],
        };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "results_update");
        let params = notification.params.unwrap();
        assert!(params.get("patches").is_some());
    }

    #[test]
    fn test_core_update_to_notification_card() {
        let update = CoreUpdate::Card {
            card: CardData {
                title: "Test Card".to_string(),
                content: None,
                markdown: None,
                actions: vec![],
                kind: None,
                blocks: vec![],
                max_height: None,
                show_details: None,
                allow_toggle_details: None,
            },
            context: Some("card-context".to_string()),
        };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "card");
        let params = notification.params.unwrap();
        assert!(params.get("card").is_some());
        assert_eq!(params["context"], "card-context");
    }

    #[test]
    fn test_core_update_to_notification_form() {
        let update = CoreUpdate::Form {
            form: FormData {
                title: "Test Form".to_string(),
                fields: vec![FormField {
                    id: "field1".to_string(),
                    label: "Field 1".to_string(),
                    field_type: FormFieldType::default(),
                    placeholder: None,
                    default_value: None,
                    required: false,
                    options: vec![],
                    hint: None,
                    rows: None,
                    min: None,
                    max: None,
                    step: None,
                }],
                submit_label: "Submit".to_string(),
                cancel_label: None,
                context: None,
                live_update: false,
            },
        };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "form");
        let params = notification.params.unwrap();
        assert!(params.get("form").is_some());
    }

    #[test]
    fn test_core_update_to_notification_plugin_activated() {
        let update = CoreUpdate::PluginActivated {
            id: "test-plugin".to_string(),
            name: "Test Plugin".to_string(),
            icon: Some("icon.png".to_string()),
        };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "plugin_activated");
        let params = notification.params.unwrap();
        assert_eq!(params["id"], "test-plugin");
        assert_eq!(params["name"], "Test Plugin");
        assert_eq!(params["icon"], "icon.png");
    }

    #[test]
    fn test_core_update_to_notification_plugin_deactivated() {
        let update = CoreUpdate::PluginDeactivated;
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "plugin_deactivated");
        assert!(notification.params.is_none());
    }

    #[test]
    fn test_core_update_to_notification_busy() {
        let update = CoreUpdate::Busy { busy: true };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "busy");
        assert_eq!(notification.params.unwrap()["busy"], true);
    }

    #[test]
    fn test_core_update_to_notification_error() {
        let update = CoreUpdate::Error {
            message: "Something went wrong".to_string(),
        };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "error");
        assert_eq!(
            notification.params.unwrap()["message"],
            "Something went wrong"
        );
    }

    #[test]
    fn test_core_update_to_notification_prompt() {
        let update = CoreUpdate::Prompt {
            prompt: "Enter password:".to_string(),
        };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "prompt");
        assert_eq!(notification.params.unwrap()["prompt"], "Enter password:");
    }

    #[test]
    fn test_core_update_to_notification_placeholder() {
        let update = CoreUpdate::Placeholder {
            placeholder: "Type to search...".to_string(),
        };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "placeholder");
        assert_eq!(
            notification.params.unwrap()["placeholder"],
            "Type to search..."
        );
    }

    #[test]
    fn test_core_update_to_notification_execute() {
        let update = CoreUpdate::Execute {
            action: ExecuteAction::Open {
                path: "/path/to/file".to_string(),
            },
        };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "execute");
        assert!(notification.params.unwrap().get("action").is_some());
    }

    #[test]
    fn test_core_update_to_notification_close() {
        let update = CoreUpdate::Close;
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "close");
        assert!(notification.params.is_none());
    }

    #[test]
    fn test_core_update_to_notification_show() {
        let update = CoreUpdate::Show;
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "show");
        assert!(notification.params.is_none());
    }

    #[test]
    fn test_core_update_to_notification_toggle() {
        let update = CoreUpdate::Toggle;
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "toggle");
        assert!(notification.params.is_none());
    }

    #[test]
    fn test_core_update_to_notification_clear_input() {
        let update = CoreUpdate::ClearInput;
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "clear_input");
        assert!(notification.params.is_none());
    }

    #[test]
    fn test_core_update_to_notification_input_mode_changed() {
        let update = CoreUpdate::InputModeChanged {
            mode: "password".to_string(),
        };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "input_mode_changed");
        assert_eq!(notification.params.unwrap()["mode"], "password");
    }

    #[test]
    fn test_core_update_to_notification_context_changed() {
        let update = CoreUpdate::ContextChanged {
            context: Some("new-context".to_string()),
        };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "context_changed");
        assert_eq!(notification.params.unwrap()["context"], "new-context");
    }

    #[test]
    fn test_core_update_to_notification_context_changed_none() {
        let update = CoreUpdate::ContextChanged { context: None };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "context_changed");
        assert!(notification.params.unwrap()["context"].is_null());
    }

    #[test]
    fn test_core_update_to_notification_plugin_status_update() {
        let update = CoreUpdate::PluginStatusUpdate {
            plugin_id: "wifi".to_string(),
            status: PluginStatus::default(),
        };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "plugin_status_update");
        let params = notification.params.unwrap();
        assert_eq!(params["plugin_id"], "wifi");
        assert!(params.get("status").is_some());
    }

    #[test]
    fn test_core_update_to_notification_ambient_update() {
        let update = CoreUpdate::AmbientUpdate {
            plugin_id: "timer".to_string(),
            items: vec![AmbientItem {
                id: "timer-1".to_string(),
                name: "Timer 1".to_string(),
                description: None,
                icon: None,
                badges: vec![],
                chips: vec![],
                actions: vec![],
                duration: 0,
                plugin_id: None,
            }],
        };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "ambient_update");
        let params = notification.params.unwrap();
        assert_eq!(params["plugin_id"], "timer");
        assert!(params.get("items").is_some());
    }

    #[test]
    fn test_core_update_to_notification_fab_update() {
        let update = CoreUpdate::FabUpdate {
            fab: Some(FabOverride {
                badges: vec![],
                chips: vec![],
                priority: 0,
                show_fab: false,
            }),
        };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "fab_update");
        assert!(notification.params.unwrap().get("fab").is_some());
    }

    #[test]
    fn test_core_update_to_notification_fab_update_none() {
        let update = CoreUpdate::FabUpdate { fab: None };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "fab_update");
        assert!(notification.params.unwrap()["fab"].is_null());
    }

    #[test]
    fn test_core_update_to_notification_image_browser() {
        let update = CoreUpdate::ImageBrowser {
            browser: ImageBrowserData {
                directory: Some("/path/to/images".to_string()),
                images: vec![ImageItem {
                    path: "/path/to/image.png".to_string(),
                    id: Some("img-1".to_string()),
                    name: Some("Image 1".to_string()),
                }],
                title: Some("My Images".to_string()),
            },
        };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "image_browser");
        assert!(notification.params.unwrap().get("browser").is_some());
    }

    #[test]
    fn test_core_update_to_notification_grid_browser() {
        let update = CoreUpdate::GridBrowser {
            browser: GridBrowserData {
                items: vec![],
                title: None,
                columns: None,
                actions: vec![],
            },
        };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "grid_browser");
        assert!(notification.params.unwrap().get("browser").is_some());
    }

    #[test]
    fn test_core_update_to_notification_plugin_actions_update() {
        let update = CoreUpdate::PluginActionsUpdate {
            actions: vec![PluginAction {
                id: "action-1".to_string(),
                name: "Do Something".to_string(),
                icon: None,
                shortcut: None,
                confirm: None,
                active: false,
            }],
        };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "plugin_actions_update");
        assert!(notification.params.unwrap().get("actions").is_some());
    }

    #[test]
    fn test_core_update_to_notification_navigation_depth_changed() {
        let update = CoreUpdate::NavigationDepthChanged { depth: 2 };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "navigation_depth_changed");
        assert_eq!(notification.params.unwrap()["depth"], 2);
    }

    #[test]
    fn test_core_update_to_notification_navigate_forward() {
        let update = CoreUpdate::NavigateForward;
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "navigate_forward");
        assert!(notification.params.is_none());
    }

    #[test]
    fn test_core_update_to_notification_navigate_back() {
        let update = CoreUpdate::NavigateBack;
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "navigate_back");
        assert!(notification.params.is_none());
    }

    #[test]
    fn test_core_update_to_notification_config_reloaded() {
        let update = CoreUpdate::ConfigReloaded;
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "config_reloaded");
        assert!(notification.params.is_none());
    }

    #[test]
    fn test_core_update_to_notification_plugin_management_changed() {
        let update = CoreUpdate::PluginManagementChanged { active: true };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "plugin_management_changed");
        assert_eq!(notification.params.unwrap()["active"], true);
    }

    #[test]
    fn test_core_update_to_notification_index_update() {
        let update = CoreUpdate::IndexUpdate {
            plugin_id: "apps".to_string(),
            items: serde_json::json!([]),
            mode: Some("full".to_string()),
            remove: None,
        };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "_internal_index");
        assert!(notification.params.is_none());
    }

    #[test]
    fn test_core_update_to_notification_activate_plugin() {
        let update = CoreUpdate::ActivatePlugin {
            plugin_id: "notes".to_string(),
        };
        let notification = core_update_to_notification(&update);
        assert_eq!(notification.method, "_internal_activate");
        assert!(notification.params.is_none());
    }
}
