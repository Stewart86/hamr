//! Hamr launcher CLI
//!
//! Unified entry point for the Hamr launcher. Provides:
//! - Default: Start GTK UI (auto-starts daemon if needed)
//! - Subcommands for daemon control and utilities

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use hamr_core::config::Directories;
use hamr_core::plugin::{ChecksumsData, Plugin, PluginVerifyStatus};
use hamr_rpc::{
    client::{RpcClient, dev_socket_path, socket_path},
    protocol::ClientRole,
};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

/// Find a binary, preferring the dev build in target/debug if it exists
fn find_binary(name: &str) -> PathBuf {
    // Check if we're running from target/debug (dev mode)
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let dev_binary = dir.join(name);
        if dev_binary.exists() {
            return dev_binary;
        }
    }
    // Fall back to PATH lookup
    PathBuf::from(name)
}

/// Check if we're in dev mode (running from target/debug or target/release)
fn is_dev_mode() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|p| {
            p.parent()
                .map(|d| d.ends_with("target/debug") || d.ends_with("target/release"))
        })
        .unwrap_or(false)
}

/// Run a binary in the foreground, bailing on failure
fn run_foreground(name: &str) -> Result<()> {
    let binary = find_binary(name);
    let status = Command::new(&binary)
        .status()
        .with_context(|| format!("Failed to start {}. Is it installed?", binary.display()))?;
    if !status.success() {
        bail!("{name} exited with status: {status}");
    }
    Ok(())
}

/// Check if hamr-daemon systemd service exists and is enabled
fn has_systemd_service() -> bool {
    Command::new("systemctl")
        .args(["--user", "is-enabled", "hamr-daemon", "--quiet"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Start daemon via systemd
fn start_daemon_systemd() -> Result<()> {
    let status = Command::new("systemctl")
        .args(["--user", "start", "hamr-daemon"])
        .status()
        .context("Failed to start hamr-daemon via systemd")?;

    if !status.success() {
        bail!("systemctl start hamr-daemon failed");
    }
    Ok(())
}

/// Hamr launcher CLI
#[derive(Parser)]
#[command(name = "hamr")]
#[command(about = "Hamr launcher - extensible application launcher")]
#[command(version)]
#[command(after_help = "\
Examples:
  hamr                    Start GTK launcher (auto-starts daemon)
  hamr daemon             Run daemon in foreground (for systemd)
  hamr gtk                Run GTK UI in foreground (for systemd)
  hamr tui                Start TUI client (for terminal use)
  hamr toggle             Toggle launcher visibility
  hamr plugin clipboard   Open clipboard plugin
  hamr plugins list       List installed plugins
  hamr plugins audit      Verify plugin checksums
  hamr status             Check daemon status
  hamr restart            Restart daemon (or systemd services if enabled)
  hamr uninstall          Remove binaries and services (preserves config)
  hamr uninstall --purge  Remove everything including user config

Keybinding examples (Hyprland):
  exec-once = hamr        # Auto-start on login (spawns daemon + GTK)
  bind = SUPER, Space, exec, hamr toggle
  bind = SUPER, V, exec, hamr plugin clipboard

Keybinding examples (Niri):
  Mod+Space { spawn \"hamr\" \"toggle\"; }
  Mod+V { spawn \"hamr\" \"plugin\" \"clipboard\"; }
")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the daemon in foreground (for systemd or manual use)
    Daemon,

    /// Start the GTK UI in foreground (for systemd or manual use)
    Gtk,

    /// Start the TUI client (terminal-based launcher)
    Tui,

    /// Toggle launcher visibility
    Toggle,

    /// Show the launcher
    Show,

    /// Hide the launcher
    Hide,

    /// Open a specific plugin
    Plugin {
        /// Plugin ID to open
        id: String,
    },

    /// Plugin management commands
    Plugins {
        #[command(subcommand)]
        command: PluginsCommand,
    },

    /// Update a plugin's status (badges, chips, ambient items)
    #[command(name = "update-status")]
    UpdateStatus {
        /// Plugin ID to update
        plugin_id: String,
        /// Status JSON (e.g. '{"badges": [{"text": "5"}]}')
        status_json: String,
    },

    /// Show daemon status
    Status,

    /// Shutdown the daemon
    Shutdown,

    /// Restart the daemon, or systemd user services if enabled
    Restart,

    /// Reload plugins
    #[command(name = "reload-plugins")]
    ReloadPlugins,

    /// Install hamr (systemd service, user directories)
    Install {
        /// Check what would be done without making changes
        #[arg(long)]
        check: bool,
    },

    /// Uninstall hamr (removes binaries, systemd services, preserves config by default)
    Uninstall {
        /// Also remove user config and plugins (~/.config/hamr)
        #[arg(long)]
        purge: bool,
    },
}

#[derive(Subcommand)]
enum PluginsCommand {
    /// List installed plugins
    List,

    /// Install a plugin from the registry (not yet implemented)
    Install {
        /// Plugin name to install
        name: String,
    },

    /// Audit plugins for checksum verification status
    Audit,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => run_gtk_with_daemon().await,
        Some(Commands::Daemon) => run_daemon(),
        Some(Commands::Gtk) => run_gtk(),
        Some(Commands::Tui) => run_tui_with_daemon().await,
        Some(Commands::Toggle) => run_toggle().await,
        Some(Commands::Show) => run_show().await,
        Some(Commands::Hide) => run_hide().await,
        Some(Commands::Plugin { id }) => run_plugin(id).await,
        Some(Commands::Plugins { command }) => run_plugins_command(command).await,
        Some(Commands::UpdateStatus {
            plugin_id,
            status_json,
        }) => run_update_status(plugin_id, status_json).await,
        Some(Commands::Status) => run_status().await,
        Some(Commands::Shutdown) => run_shutdown().await,
        Some(Commands::Restart) => run_restart().await,
        Some(Commands::ReloadPlugins) => run_reload_plugins().await,
        Some(Commands::Install { check }) => run_install(check),
        Some(Commands::Uninstall { purge }) => run_uninstall(purge),
    }
}

/// Start GTK UI, auto-starting daemon as background process
async fn run_gtk_with_daemon() -> Result<()> {
    ensure_daemon_running().await?;
    run_foreground("hamr-gtk")
}

/// Run GTK UI in foreground (for systemd `ExecStart`)
fn run_gtk() -> Result<()> {
    run_foreground("hamr-gtk")
}

/// Start TUI, auto-starting daemon if needed
async fn run_tui_with_daemon() -> Result<()> {
    ensure_daemon_running().await?;
    run_foreground("hamr-tui")
}

/// Ensure daemon is running, starting it if needed
async fn ensure_daemon_running() -> Result<()> {
    let socket = socket_path();

    if socket.exists() && is_daemon_responsive().await {
        return Ok(());
    }

    if is_dev_mode() {
        eprintln!("Starting daemon (dev mode)...");
        start_daemon_background()?;
    } else if has_systemd_service() {
        eprintln!("Starting daemon via systemd...");
        start_daemon_systemd()?;
    } else {
        eprintln!("Starting daemon...");
        start_daemon_background()?;
    }

    // Wait for daemon to be ready
    if !wait_for_daemon(Duration::from_secs(5)).await {
        bail!("Daemon failed to start within 5 seconds");
    }
    eprintln!("Daemon started");

    Ok(())
}

/// Check if daemon is responsive (socket exists and accepts connection)
async fn is_daemon_responsive() -> bool {
    match RpcClient::connect().await {
        Ok(mut client) => {
            // Try to register - if it works, daemon is alive
            client.register(ClientRole::Control).await.is_ok()
        }
        Err(_) => false,
    }
}

/// Start daemon as background process
fn start_daemon_background() -> Result<()> {
    let daemon_binary = find_binary("hamr-daemon");
    Command::new(&daemon_binary)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| {
            format!(
                "Failed to spawn {}. Is it installed?",
                daemon_binary.display()
            )
        })?;

    Ok(())
}

/// Wait for daemon to become responsive
async fn wait_for_daemon(timeout: Duration) -> bool {
    let start = std::time::Instant::now();
    let poll_interval = Duration::from_millis(100);

    while start.elapsed() < timeout {
        if is_daemon_responsive().await {
            return true;
        }
        sleep(poll_interval).await;
    }

    false
}

fn run_daemon() -> Result<()> {
    run_foreground("hamr-daemon")
}

async fn connect_and_register_at(socket: PathBuf) -> Result<RpcClient> {
    if !socket.exists() {
        bail!(
            "Daemon not running (socket not found at {}).\nStart with: hamr daemon",
            socket.display()
        );
    }

    let mut client = RpcClient::connect_to(socket)
        .await
        .context("Failed to connect to daemon. Is it running?")?;

    client
        .register(ClientRole::Control)
        .await
        .context("Failed to register with daemon")?;

    Ok(client)
}

async fn connect_and_register() -> Result<RpcClient> {
    connect_and_register_at(socket_path()).await
}

async fn try_connect_dev_daemon() -> Result<Option<RpcClient>> {
    if is_dev_mode() {
        return Ok(None);
    }

    let socket = dev_socket_path();
    if !socket.exists() {
        return Ok(None);
    }

    match connect_and_register_at(socket).await {
        Ok(client) => Ok(Some(client)),
        Err(_) => Ok(None),
    }
}

async fn run_toggle() -> Result<()> {
    let client = match try_connect_dev_daemon().await? {
        Some(client) => client,
        None => connect_and_register().await?,
    };

    let result: serde_json::Value = client
        .request("toggle", None)
        .await
        .context("Toggle command failed")?;

    let status = result
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");

    match status {
        "ok" => {
            // Toggle sent successfully - actual visibility depends on UI state
        }
        "no_ui" => {
            let msg = result
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("No UI connected");
            bail!("{msg}");
        }
        "error" => {
            let msg = result
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Unknown error");
            bail!("Toggle failed: {msg}");
        }
        _ => {
            eprintln!("Unexpected toggle response: {result:?}");
        }
    }

    Ok(())
}

async fn run_show() -> Result<()> {
    let client = connect_and_register().await?;

    let _: serde_json::Value = client
        .request("show", None)
        .await
        .context("Show command failed")?;

    println!("Launcher shown");
    Ok(())
}

async fn run_hide() -> Result<()> {
    let client = connect_and_register().await?;

    let _: serde_json::Value = client
        .request("hide", None)
        .await
        .context("Hide command failed")?;

    println!("Launcher hidden");
    Ok(())
}

async fn run_plugin(id: String) -> Result<()> {
    let client = match try_connect_dev_daemon().await? {
        Some(client) => client,
        None => connect_and_register().await?,
    };

    let _: serde_json::Value = client
        .request("open_plugin", Some(serde_json::json!({ "plugin_id": id })))
        .await
        .context("Open plugin command failed")?;

    println!("Plugin '{id}' opened");
    Ok(())
}

async fn run_update_status(plugin_id: String, status_json: String) -> Result<()> {
    let status: serde_json::Value =
        serde_json::from_str(&status_json).context("Invalid JSON for status")?;

    let client = connect_and_register().await?;

    let _: serde_json::Value = client
        .request(
            "update_status",
            Some(serde_json::json!({
                "plugin_id": plugin_id,
                "status": status
            })),
        )
        .await
        .context("Update status command failed")?;

    println!("Status updated for plugin '{plugin_id}'");
    Ok(())
}

async fn run_status() -> Result<()> {
    let socket = socket_path();

    if !socket.exists() {
        println!("Status: Not running");
        println!("Socket: {} (not found)", socket.display());
        return Ok(());
    }

    match connect_and_register().await {
        Ok(client) => {
            let result: serde_json::Value = client
                .request("status", None)
                .await
                .context("Status request failed")?;

            let uptime_secs = result
                .get("uptime_secs")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let plugins_loaded = result
                .get("plugins_loaded")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let ui_connected = result
                .get("ui_connected")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let active_plugin = result.get("active_plugin").and_then(|v| v.as_str());

            println!("Status: Running");
            println!("Socket: {}", socket.display());
            println!("Uptime: {uptime_secs}s");
            println!("Plugins loaded: {plugins_loaded}");
            println!("UI connected: {}", if ui_connected { "yes" } else { "no" });
            if let Some(plugin) = active_plugin {
                println!("Active plugin: {plugin}");
            }
        }
        Err(e) => {
            println!("Status: Error");
            println!(
                "Socket: {} (exists but connection failed)",
                socket.display()
            );
            println!("Error: {e}");
        }
    }

    Ok(())
}

async fn run_shutdown() -> Result<()> {
    let client = connect_and_register().await?;

    // Send shutdown as notification - don't wait for response since daemon will exit
    client
        .notify("shutdown", None)
        .await
        .context("Shutdown command failed")?;

    println!("Daemon shutting down");
    Ok(())
}

async fn run_restart() -> Result<()> {
    // If the user opted into systemd (`hamr install`), prefer restarting services.
    if has_systemd_service() {
        let status = Command::new("systemctl")
            .args(["--user", "restart", "hamr-daemon", "hamr-gtk"])
            .status()
            .context("Failed to restart hamr services via systemd")?;

        if !status.success() {
            bail!("systemctl --user restart failed");
        }

        println!("Restarted systemd user services (hamr-daemon, hamr-gtk)");
        return Ok(());
    }

    // Fallback: restart the daemon process.
    // Best-effort shutdown if it's currently running.
    if let Ok(client) = connect_and_register().await {
        let _ = client.notify("shutdown", None).await;
        sleep(Duration::from_millis(300)).await;
    }

    start_daemon_background()?;
    if !wait_for_daemon(Duration::from_secs(5)).await {
        bail!("Daemon failed to restart within 5 seconds");
    }

    println!("Daemon restarted");
    println!("If you're not using systemd, run 'hamr' to (re)start the GTK UI.");
    Ok(())
}

async fn run_reload_plugins() -> Result<()> {
    let client = connect_and_register().await?;

    let _: serde_json::Value = client
        .request("reload_plugins", None)
        .await
        .context("Reload plugins command failed")?;

    println!("Plugins reloaded");
    Ok(())
}

async fn run_plugins_command(command: PluginsCommand) -> Result<()> {
    match command {
        PluginsCommand::List => run_plugins_list().await,
        PluginsCommand::Install { name } => run_plugins_install(&name),
        PluginsCommand::Audit => run_plugins_audit(),
    }
}

async fn run_plugins_list() -> Result<()> {
    let client = connect_and_register().await?;

    let result: serde_json::Value = client
        .request("list_plugins", None)
        .await
        .context("List plugins command failed")?;

    let Some(plugins) = result.get("plugins").and_then(|v| v.as_array()) else {
        println!("No plugins data received.");
        return Ok(());
    };

    if plugins.is_empty() {
        println!("No plugins found.");
        return Ok(());
    }

    println!("\nInstalled Plugins:\n");

    for plugin in plugins {
        let id = plugin.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let name = plugin.get("name").and_then(|v| v.as_str()).unwrap_or(id);
        let desc = plugin
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let prefix = plugin.get("prefix").and_then(|v| v.as_str());
        let is_socket = plugin
            .get("is_socket")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let connected = plugin
            .get("connected")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let status = if is_socket {
            if connected {
                "daemon (connected)"
            } else {
                "daemon"
            }
        } else {
            "stdio"
        };

        print!("  {id:<16} {name}");
        if let Some(pfx) = prefix {
            print!(" [{pfx}]");
        }
        println!(" ({status})");

        if !desc.is_empty() {
            println!("                   {desc}");
        }
    }

    println!();
    Ok(())
}

fn run_plugins_install(name: &str) -> Result<()> {
    bail!(
        "Plugin registry not yet available.\n\n\
         The `hamr plugins install {name}` command will allow installing plugins \
         from a central registry in a future release.\n\n\
         For now, install plugins manually:\n\
         1. Download the plugin to ~/.config/hamr/plugins/{name}/\n\
         2. Ensure it has a manifest.json file\n\
         3. Run `hamr reload-plugins` to load it"
    );
}

fn run_plugins_audit() -> Result<()> {
    let dirs = Directories::new().context("Failed to determine project directories")?;
    let checksums_path = dirs.builtin_plugins.join("checksums.json");

    let Some(checksums) = ChecksumsData::load(&checksums_path) else {
        println!("No checksums.json found at: {}", checksums_path.display());
        println!("\nChecksum verification is only available for official releases.");
        println!("To generate checksums, run: scripts/generate-plugin-checksums.sh");
        return Ok(());
    };

    if !checksums.is_available() {
        println!("checksums.json is empty - no plugins to verify.");
        return Ok(());
    }

    println!("Plugin Audit Report\n");
    println!("Checksums: {}", checksums_path.display());
    println!("Plugins tracked: {}\n", checksums.plugin_count());

    let mut verified = Vec::new();
    let mut modified = Vec::new();
    let mut unknown = Vec::new();

    let plugin_dirs = [&dirs.builtin_plugins, &dirs.user_plugins];

    for plugin_dir in &plugin_dirs {
        if !plugin_dir.exists() {
            continue;
        }

        let Ok(entries) = std::fs::read_dir(plugin_dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let plugin_id = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            if SKIP_PLUGIN_DIRS.contains(&plugin_id.as_str()) {
                continue;
            }

            let Ok(plugin) = Plugin::load(path.clone()) else {
                continue;
            };

            let status = checksums.verify_plugin(&plugin_id, &path);

            match status {
                PluginVerifyStatus::Verified => {
                    verified.push((plugin_id, plugin.manifest.name));
                }
                PluginVerifyStatus::Modified(files) => {
                    modified.push((plugin_id, plugin.manifest.name, files));
                }
                PluginVerifyStatus::Unknown => {
                    unknown.push((plugin_id, plugin.manifest.name));
                }
            }
        }
    }

    if !verified.is_empty() {
        println!("VERIFIED ({}):", verified.len());
        for (id, name) in &verified {
            println!("  [OK] {id:<16} {name}");
        }
        println!();
    }

    if !modified.is_empty() {
        println!("MODIFIED ({}):", modified.len());
        for (id, name, files) in &modified {
            println!("  [!!] {id:<16} {name}");
            for file in files {
                println!("       - {file}");
            }
        }
        println!();
    }

    if !unknown.is_empty() {
        println!("UNKNOWN ({}):", unknown.len());
        for (id, name) in &unknown {
            println!("  [??] {id:<16} {name}");
        }
        println!();
    }

    println!("---");
    println!(
        "Summary: {} verified, {} modified, {} unknown",
        verified.len(),
        modified.len(),
        unknown.len()
    );

    if !modified.is_empty() {
        println!("\nWARNING: Modified plugins may have been tampered with.");
        println!("Review the modified files or reinstall from a trusted source.");
    }

    Ok(())
}

fn generate_daemon_service() -> Result<String> {
    let daemon_path = which_daemon()?;

    Ok(format!(
        r#"[Unit]
Description=Hamr Launcher Daemon
Documentation=https://hamr.run
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=/bin/sh -c 'runtime="${{XDG_RUNTIME_DIR:-/run/user/$(id -u)}}"; if [ -z "${{NIRI_SOCKET:-}}" ]; then for candidate in "$runtime"/niri.*.sock; do if [ -S "$candidate" ]; then export NIRI_SOCKET="$candidate"; break; fi; done; fi; if [ -z "${{HYPRLAND_INSTANCE_SIGNATURE:-}}" ]; then for candidate in /tmp/hypr/*/.socket.sock /tmp/hypr/*/.socket2.sock; do if [ -S "$candidate" ]; then export HYPRLAND_INSTANCE_SIGNATURE="$(basename "$(dirname "$candidate")")"; break; fi; done; fi; exec "{daemon_path}"'
Restart=on-failure
RestartSec=3
KillMode=process

[Install]
WantedBy=graphical-session.target
"#,
        daemon_path = daemon_path.display()
    ))
}

fn generate_gtk_service() -> Result<String> {
    let gtk_path = which_gtk()?;

    let service_content = format!(
        r#"[Unit]
Description=Hamr Launcher GTK UI
Documentation=https://hamr.run
PartOf=graphical-session.target
After=hamr-daemon.service
Wants=hamr-daemon.service

[Service]
Type=simple
ExecStart={}
Restart=always
RestartSec=3
KillMode=process
# Wait for display to be available
ExecStartPre=/bin/sh -c 'runtime="$XDG_RUNTIME_DIR"; if [ -z "$runtime" ]; then runtime="/run/user/$(id -u)"; fi; socket=""; if [ -n "$WAYLAND_DISPLAY" ]; then socket="$runtime/$WAYLAND_DISPLAY"; else for candidate in "$runtime"/wayland-*; do if [ -e "$candidate" ]; then socket="$candidate"; break; fi; done; fi; while [ -z "$socket" ] || ! [ -e "$socket" ]; do sleep 0.1; if [ -n "$WAYLAND_DISPLAY" ]; then socket="$runtime/$WAYLAND_DISPLAY"; else socket=""; for candidate in "$runtime"/wayland-*; do if [ -e "$candidate" ]; then socket="$candidate"; break; fi; done; fi; done'

[Install]
WantedBy=graphical-session.target
"#,
        gtk_path.display()
    );
    Ok(service_content)
}

/// Find a hamr binary by name
fn which_binary(name: &str) -> Result<PathBuf> {
    // Priority 1: Same directory as current executable
    // This ensures /usr/bin/hamr finds /usr/bin/hamr-daemon, not ~/.local/bin/hamr-daemon
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let binary = dir.join(name);
        if binary.exists() {
            return Ok(binary.canonicalize()?);
        }
    }

    // Priority 2: Check PATH (respects user's PATH order)
    if let Ok(output) = Command::new("which").arg(name).output()
        && output.status.success()
    {
        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path_str.is_empty() {
            return Ok(PathBuf::from(path_str));
        }
    }

    // Priority 3: Check ~/.local/bin (fallback for user installs)
    if let Some(home) = dirs::home_dir() {
        let local_bin = home.join(format!(".local/bin/{name}"));
        if local_bin.exists() {
            return Ok(local_bin.canonicalize()?);
        }
    }

    bail!(
        "Could not find {name} binary.\n\
         Make sure it's installed in one of:\n\
         - Same directory as hamr\n\
         - In your PATH\n\
         - ~/.local/bin/{name}"
    )
}

fn which_daemon() -> Result<PathBuf> {
    which_binary("hamr-daemon")
}

fn which_gtk() -> Result<PathBuf> {
    which_binary("hamr-gtk")
}

fn get_config_dir() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|d| d.join("hamr"))
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))
}

/// Directories to skip when iterating plugin dirs
const SKIP_PLUGIN_DIRS: &[&str] = &["sdk", "__pycache__"];

/// Essential plugins that should be copied to user config on install
const ESSENTIAL_PLUGINS: &[&str] = &["apps", "shell", "calculate", "clipboard", "power", "sdk"];

/// Find the source plugins directory (same logic as hamr-core's `Directories::find_builtin_plugins`)
fn find_source_plugins() -> Option<PathBuf> {
    if let Ok(exe_path) = std::env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        // Priority 1: Next to current executable (release/GitHub download)
        let plugins_dir = exe_dir.join("plugins");
        if plugins_dir.exists() {
            return Some(plugins_dir);
        }

        // Priority 2: FHS-style share path relative to binary (e.g. Nix: ../share/hamr/plugins)
        let share_plugins = exe_dir.join("../share/hamr/plugins");
        if share_plugins.exists() {
            return share_plugins.canonicalize().ok();
        }
    }

    // Priority 3: Development paths
    let dev_paths: [PathBuf; 2] = [PathBuf::from("plugins"), PathBuf::from("../hamr/plugins")];

    for path in dev_paths {
        if path.exists() {
            return path.canonicalize().ok();
        }
    }

    // Priority 4: System-wide location
    #[cfg(target_os = "macos")]
    let system_path = PathBuf::from("/Library/Application Support/hamr/plugins");
    #[cfg(not(target_os = "macos"))]
    let system_path = PathBuf::from("/usr/share/hamr/plugins");

    if system_path.exists() {
        return Some(system_path);
    }

    None
}

/// Copy a plugin directory recursively, skipping __pycache__ directories
fn copy_plugin_dir(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    use std::fs;

    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if SKIP_PLUGIN_DIRS.contains(&name_str.as_ref()) {
            continue;
        }

        let src_path = entry.path();
        let dst_path = dst.join(&name);

        if file_type.is_dir() {
            copy_plugin_dir(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Install essential plugins from source to user config
fn install_essential_plugins(user_plugins_dir: &std::path::Path, check: bool) -> Result<()> {
    let Some(source_dir) = find_source_plugins() else {
        println!("  Warning: Could not find source plugins directory");
        return Ok(());
    };

    println!("  Source: {}", source_dir.display());

    for plugin_name in ESSENTIAL_PLUGINS {
        let src = source_dir.join(plugin_name);
        let dst = user_plugins_dir.join(plugin_name);

        if !src.exists() {
            println!("  Skip:    {plugin_name} (not found in source)");
            continue;
        }

        if dst.exists() {
            println!("  Exists:  {plugin_name}");
        } else if check {
            println!("  Copy:    {plugin_name}");
        } else {
            copy_plugin_dir(&src, &dst)?;
            println!("  Copied:  {plugin_name}");
        }
    }

    Ok(())
}

fn get_systemd_dir() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|d| d.join("systemd/user"))
        .ok_or_else(|| anyhow::anyhow!("Could not determine systemd user directory"))
}

/// Check or create a directory
fn ensure_dir(path: &std::path::Path, check: bool) -> Result<()> {
    use std::fs;
    if path.exists() {
        println!("  Exists:  {}", path.display());
    } else if check {
        println!("  Create:  {}", path.display());
    } else {
        fs::create_dir_all(path)?;
        println!("  Created: {}", path.display());
    }
    Ok(())
}

/// Check or create a file with content
fn ensure_file(path: &std::path::Path, content: &str, check: bool) -> Result<()> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    if path.exists() {
        println!("  Update:  {}", path.display());
    } else if check {
        println!("  Create:  {}", path.display());
    }
    if !check {
        fs::write(path, content)?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o644))?;
    }
    Ok(())
}

/// Check if systemctl is available
fn is_systemctl_available() -> bool {
    Command::new("systemctl")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if a systemd user service is enabled
fn is_service_enabled(name: &str) -> bool {
    Command::new("systemctl")
        .args(["--user", "is-enabled", name, "--quiet"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Enable a systemd user service
fn enable_service(name: &str) {
    let result = Command::new("systemctl")
        .args(["--user", "enable", name])
        .status();
    if result.map(|s| s.success()).unwrap_or(false) {
        println!("  Enabled {name}.service");
    } else {
        println!("  Warning: Failed to enable {name} service");
    }
}

/// Check a binary exists, print status, return true if found
fn check_binary(name: &str, label: &str, check: bool) -> bool {
    match which_binary(name) {
        Ok(path) => {
            println!("  {label}: {}", path.display());
            true
        }
        Err(e) => {
            println!("  {label}: NOT FOUND");
            if check {
                println!("    Error: {e}");
            }
            false
        }
    }
}

/// Print whether file would be created or updated
fn print_file_action(path: &std::path::Path) {
    if path.exists() {
        println!("  Update:  {}", path.display());
    } else {
        println!("  Create:  {}", path.display());
    }
}

/// Check/install systemd services
fn install_systemd_services(systemd_dir: &std::path::Path, check: bool) -> Result<()> {
    if !systemd_dir.exists() && !check {
        std::fs::create_dir_all(systemd_dir)?;
    }

    let daemon_file = systemd_dir.join("hamr-daemon.service");
    let gtk_file = systemd_dir.join("hamr-gtk.service");

    print_file_action(&daemon_file);
    print_file_action(&gtk_file);

    if !check {
        ensure_file(&daemon_file, &generate_daemon_service()?, false)?;
        ensure_file(&gtk_file, &generate_gtk_service()?, false)?;
    }
    Ok(())
}

/// Configure systemd (reload daemon, enable services)
fn configure_systemd(check: bool) {
    if check {
        let daemon_enabled = is_service_enabled("hamr-daemon");
        let gtk_enabled = is_service_enabled("hamr-gtk");
        println!(
            "  hamr-daemon.service: {}",
            if daemon_enabled {
                "enabled"
            } else {
                "will enable"
            }
        );
        println!(
            "  hamr-gtk.service:    {}",
            if gtk_enabled {
                "enabled"
            } else {
                "will enable"
            }
        );
        return;
    }

    let reload = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();
    if reload.map(|s| s.success()).unwrap_or(false) {
        println!("  Reloaded systemd user daemon");
    } else {
        println!("  Warning: Failed to reload systemd daemon");
    }
    enable_service("hamr-daemon");
    enable_service("hamr-gtk");
}

fn run_install(check: bool) -> Result<()> {
    println!(
        "{}\n",
        if check {
            "Checking installation requirements..."
        } else {
            "Installing hamr..."
        }
    );

    // Validate binaries can be found
    println!("Binaries:");
    let daemon_found = check_binary("hamr-daemon", "hamr-daemon", check);
    let gtk_found = check_binary("hamr-gtk", "hamr-gtk   ", check);

    if check && (!daemon_found || !gtk_found) {
        println!("\nInstallation would fail: Missing binaries");
        return Ok(());
    }

    // Check/create config directories
    println!("\nDirectories:");
    let config_dir = get_config_dir()?;
    ensure_dir(&config_dir, check)?;
    ensure_dir(&config_dir.join("plugins"), check)?;

    // Check/create default config
    println!("\nConfig:");
    let config_file = config_dir.join("config.json");
    if config_file.exists() {
        println!("  Exists:  {}", config_file.display());
    } else {
        ensure_file(&config_file, "{}\n", check)?;
    }

    // Check/install essential plugins
    println!("\nPlugins:");
    let plugins_dir = config_dir.join("plugins");
    install_essential_plugins(&plugins_dir, check)?;

    // Check/install systemd services
    println!("\nSystemd services:");
    install_systemd_services(&get_systemd_dir()?, check)?;

    // Check/configure systemd
    println!("\nSystemd configuration:");
    if is_systemctl_available() {
        configure_systemd(check);
    } else {
        println!("  Warning: systemctl not available (services won't be enabled)");
    }

    if check {
        println!("\nCheck complete. Run 'hamr install' to proceed.");
    } else {
        println!("\nInstallation complete!");
        println!("\nNext steps:");
        println!("  1. Start both services: systemctl --user start hamr-gtk");
        println!("     (this will also start hamr-daemon due to Requires=)");
        println!("  2. Or just run:         hamr");
        println!("\nTo configure keybindings:");
        println!("  Hyprland: bind = SUPER, Space, exec, hamr toggle");
        println!("  Niri:     Mod+Space {{ spawn \"hamr\" \"toggle\"; }}");
    }

    Ok(())
}

fn run_uninstall(purge: bool) -> Result<()> {
    use std::fs;

    println!("Uninstalling hamr...\n");

    println!("Systemd services:");
    if is_systemctl_available() {
        for service in &["hamr-gtk", "hamr-daemon"] {
            if let Err(e) = Command::new("systemctl")
                .args(["--user", "stop", service])
                .status()
            {
                eprintln!("  Warning: Failed to stop {service}: {e}");
            }
            if let Err(e) = Command::new("systemctl")
                .args(["--user", "disable", service])
                .status()
            {
                eprintln!("  Warning: Failed to disable {service}: {e}");
            }
            println!("  Stopped and disabled {service}.service");
        }
    } else {
        println!("  systemctl not available, skipping");
    }

    println!("\nService files:");
    let systemd_dir = get_systemd_dir()?;
    for name in &["hamr-gtk.service", "hamr-daemon.service"] {
        let path = systemd_dir.join(name);
        if path.exists() {
            fs::remove_file(&path)?;
            println!("  Removed: {}", path.display());
        } else {
            println!("  Not found: {}", path.display());
        }
    }

    if is_systemctl_available() {
        if let Err(e) = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status()
        {
            eprintln!("  Warning: Failed to reload systemd daemon: {e}");
        } else {
            println!("  Reloaded systemd daemon");
        }
    }

    println!("\nSocket:");
    let socket = socket_path();
    if socket.exists() {
        fs::remove_file(&socket)?;
        println!("  Removed: {}", socket.display());
    } else {
        println!("  Not found (daemon was not running)");
    }
    let dev_sock = dev_socket_path();
    if dev_sock.exists() {
        fs::remove_file(&dev_sock)?;
        println!("  Removed: {}", dev_sock.display());
    }

    println!("\nBinaries:");
    let bin_dir = find_install_bin_dir();
    if let Some(ref bin_dir) = bin_dir {
        for name in &["hamr", "hamr-daemon", "hamr-gtk", "hamr-tui"] {
            let path = bin_dir.join(name);
            if path.exists() {
                fs::remove_file(&path)?;
                println!("  Removed: {}", path.display());
            }
        }

        let system_plugins = bin_dir.join("plugins");
        if system_plugins.exists() {
            fs::remove_dir_all(&system_plugins)?;
            println!("  Removed: {}", system_plugins.display());
        }
    } else {
        println!("  Could not determine install directory, skipping binary removal");
        println!("  Remove manually from ~/.local/bin/ or wherever you installed");
    }

    println!("\nShell PATH:");
    if let Some(ref bin_dir) = bin_dir {
        remove_path_from_shell_rc(bin_dir);
    } else {
        println!("  Skipped (install directory unknown)");
    }

    let config_dir = get_config_dir()?;
    if purge {
        println!("\nUser data (--purge):");
        if config_dir.exists() {
            fs::remove_dir_all(&config_dir)?;
            println!("  Removed: {}", config_dir.display());
        } else {
            println!("  Not found: {}", config_dir.display());
        }
    } else {
        println!("\nUser data:");
        println!("  Preserved: {}", config_dir.display());
        println!("  To remove: rm -rf {}", config_dir.display());
        println!("  Or re-run: hamr uninstall --purge");
    }

    println!("\nUninstall complete!");
    Ok(())
}

/// Determine where binaries were installed (the bin directory containing the current exe)
fn find_install_bin_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;

    // Skip if running from a dev build
    if dir.ends_with("target/debug") || dir.ends_with("target/release") {
        return None;
    }

    Some(dir.to_path_buf())
}

/// Remove the hamr PATH entry from shell rc files
fn remove_path_from_shell_rc(bin_dir: &std::path::Path) {
    let bin_str = bin_dir.to_string_lossy();

    let rc_files: Vec<PathBuf> = [".bashrc", ".zshrc"]
        .iter()
        .filter_map(|name| dirs::home_dir().map(|h| h.join(name)))
        .chain(dirs::config_dir().map(|d| d.join("fish/config.fish")))
        .collect();

    for rc_file in rc_files {
        if !rc_file.exists() {
            continue;
        }

        let Ok(content) = std::fs::read_to_string(&rc_file) else {
            continue;
        };

        if !content.contains(&*bin_str) {
            continue;
        }

        let filtered: Vec<&str> = content
            .lines()
            .filter(|line| {
                !line.contains("# Added by hamr installer")
                    && !(line.contains(&*bin_str)
                        && (line.contains("export PATH") || line.contains("set -gx PATH")))
            })
            .collect();

        let new_content = filtered.join("\n");
        let new_content = if content.ends_with('\n') && !new_content.ends_with('\n') {
            format!("{new_content}\n")
        } else {
            new_content
        };

        if new_content != content {
            if std::fs::write(&rc_file, &new_content).is_ok() {
                println!("  Cleaned PATH from: {}", rc_file.display());
            } else {
                println!("  Warning: Could not update {}", rc_file.display());
            }
        }
    }
}
