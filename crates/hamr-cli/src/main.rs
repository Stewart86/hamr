//! Hamr launcher CLI
//!
//! Unified entry point for the Hamr launcher. Provides:
//! - Default: Start GTK UI (auto-starts daemon if needed)
//! - Subcommands for daemon control and utilities

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use hamr_rpc::{
    client::{RpcClient, socket_path},
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

/// Check if we're in dev mode (running from target/debug)
fn is_dev_mode() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.ends_with("target/debug")))
        .unwrap_or(false)
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
  hamr status             Check daemon status

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

    /// Reload plugins
    #[command(name = "reload-plugins")]
    ReloadPlugins,

    /// Install hamr (systemd service, user directories)
    Install {
        /// Check what would be done without making changes
        #[arg(long)]
        check: bool,
    },

    /// Uninstall hamr (removes systemd service, preserves config)
    Uninstall,
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
        Some(Commands::ReloadPlugins) => run_reload_plugins().await,
        Some(Commands::Install { check }) => run_install(check),
        Some(Commands::Uninstall) => run_uninstall(),
    }
}

/// Start GTK UI, auto-starting daemon as background process
async fn run_gtk_with_daemon() -> Result<()> {
    ensure_daemon_running().await?;

    let gtk_binary = find_binary("hamr-gtk");
    let status = Command::new(&gtk_binary)
        .status()
        .with_context(|| format!("Failed to start {}. Is it installed?", gtk_binary.display()))?;

    if !status.success() {
        bail!("hamr-gtk exited with status: {status}");
    }

    Ok(())
}

/// Run GTK UI in foreground (for systemd `ExecStart`)
fn run_gtk() -> Result<()> {
    let gtk_binary = find_binary("hamr-gtk");
    let status = Command::new(&gtk_binary)
        .status()
        .with_context(|| format!("Failed to start {}. Is it installed?", gtk_binary.display()))?;

    if !status.success() {
        bail!("hamr-gtk exited with status: {status}");
    }

    Ok(())
}

/// Start TUI, auto-starting daemon if needed
async fn run_tui_with_daemon() -> Result<()> {
    ensure_daemon_running().await?;

    let tui_binary = find_binary("hamr-tui");
    let status = Command::new(&tui_binary)
        .status()
        .with_context(|| format!("Failed to start {}. Is it installed?", tui_binary.display()))?;

    if !status.success() {
        bail!("hamr-tui exited with status: {status}");
    }

    Ok(())
}

/// Ensure daemon is running, starting it if needed
async fn ensure_daemon_running() -> Result<()> {
    let socket = socket_path();

    // Check if daemon is already running
    if socket.exists() && is_daemon_responsive().await {
        return Ok(());
    }

    // Start daemon - use systemd in production, process in dev mode
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
    let daemon_binary = find_binary("hamr-daemon");
    let status = Command::new(&daemon_binary).status().with_context(|| {
        format!(
            "Failed to start {}. Is it installed?",
            daemon_binary.display()
        )
    })?;

    if !status.success() {
        bail!("hamr-daemon exited with status: {status}");
    }

    Ok(())
}

async fn connect_and_register() -> Result<RpcClient> {
    let socket = socket_path();

    if !socket.exists() {
        bail!(
            "Daemon not running (socket not found at {}).\nStart with: hamr daemon",
            socket.display()
        );
    }

    let mut client = RpcClient::connect()
        .await
        .context("Failed to connect to daemon. Is it running?")?;

    client
        .register(ClientRole::Control)
        .await
        .context("Failed to register with daemon")?;

    Ok(client)
}

async fn run_toggle() -> Result<()> {
    let client = connect_and_register().await?;

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
    let client = connect_and_register().await?;

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

fn generate_daemon_service() -> Result<String> {
    let daemon_path = which_daemon()?;

    Ok(format!(
        r"[Unit]
Description=Hamr Launcher Daemon
Documentation=https://hamr.run
PartOf=graphical-session.target
After=graphical-session.target

[Service]
Type=simple
ExecStart={daemon_path}
Restart=on-failure
RestartSec=3

[Install]
WantedBy=graphical-session.target
",
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
Requires=hamr-daemon.service

[Service]
Type=simple
ExecStart={}
Restart=always
RestartSec=3
# Wait for display to be available
ExecStartPre=/bin/sh -c 'while ! [ -e "/run/user/$(id -u)/${{WAYLAND_DISPLAY:-wayland-0}}" ]; do sleep 0.1; done'

[Install]
WantedBy=graphical-session.target
"#,
        gtk_path.display()
    );
    Ok(service_content)
}

/// Find a hamr binary by name
fn which_binary(name: &str) -> Result<PathBuf> {
    // First check if we're in dev mode (same directory as hamr)
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let binary = dir.join(name);
        if binary.exists() {
            return Ok(binary.canonicalize()?);
        }
    }

    // Check ~/.local/bin (common user install location)
    if let Some(home) = dirs::home_dir() {
        let local_bin = home.join(format!(".local/bin/{name}"));
        if local_bin.exists() {
            return Ok(local_bin.canonicalize()?);
        }
    }

    // Check if it's in PATH
    if let Ok(output) = Command::new("which").arg(name).output()
        && output.status.success()
    {
        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path_str.is_empty() {
            return Ok(PathBuf::from(path_str));
        }
    }

    bail!(
        "Could not find {name} binary.\n\
         Make sure it's installed in one of:\n\
         - ~/.local/bin/{name}\n\
         - /usr/local/bin/{name}\n\
         - Or in your PATH"
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

/// Essential plugins that should be copied to user config on install
const ESSENTIAL_PLUGINS: &[&str] = &["apps", "shell", "calculate", "clipboard", "power", "sdk"];

/// Find the source plugins directory (same logic as hamr-core's `Directories::find_builtin_plugins`)
fn find_source_plugins() -> Option<PathBuf> {
    // Priority 1: Next to current executable (release/GitHub download)
    if let Ok(exe_path) = std::env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        let plugins_dir = exe_dir.join("plugins");
        if plugins_dir.exists() {
            return Some(plugins_dir);
        }
    }

    // Priority 2: Development paths
    let dev_paths: [PathBuf; 2] = [PathBuf::from("plugins"), PathBuf::from("../hamr/plugins")];

    for path in dev_paths {
        if path.exists() {
            return path.canonicalize().ok();
        }
    }

    // Priority 3: System-wide location
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

        // Skip __pycache__ directories
        if name_str == "__pycache__" {
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

fn run_uninstall() -> Result<()> {
    use std::fs;

    println!("Uninstalling hamr...\n");

    // 1. Stop and disable systemd services (GTK first, then daemon)
    println!("Stopping systemd services...");

    let _ = Command::new("systemctl")
        .args(["--user", "stop", "hamr-gtk"])
        .status();

    let _ = Command::new("systemctl")
        .args(["--user", "disable", "hamr-gtk"])
        .status();

    println!("  Stopped and disabled hamr-gtk.service");

    let _ = Command::new("systemctl")
        .args(["--user", "stop", "hamr-daemon"])
        .status();

    let _ = Command::new("systemctl")
        .args(["--user", "disable", "hamr-daemon"])
        .status();

    println!("  Stopped and disabled hamr-daemon.service");

    // 2. Remove systemd service files
    let systemd_dir = get_systemd_dir()?;

    let gtk_service_file = systemd_dir.join("hamr-gtk.service");
    if gtk_service_file.exists() {
        fs::remove_file(&gtk_service_file)?;
        println!("  Removed: {}", gtk_service_file.display());
    }

    let daemon_service_file = systemd_dir.join("hamr-daemon.service");
    if daemon_service_file.exists() {
        fs::remove_file(&daemon_service_file)?;
        println!("  Removed: {}", daemon_service_file.display());
    }

    // Reload systemd
    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();

    println!("\nUninstall complete!");
    println!("\nNote: Config and plugins were preserved at:");
    let config_dir = get_config_dir()?;
    println!("  {}", config_dir.display());
    println!("\nTo remove config: rm -rf {}", config_dir.display());

    Ok(())
}
