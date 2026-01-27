//! Compositor integration for Wayland compositors.
//!
//! Provides a unified interface for interacting with different Wayland compositors
//! (Hyprland, Niri, Sway) for:
//! - Compositor detection and layer-shell support
//! - Window listing and focusing
//! - Running app detection

use crate::niri_ipc::NiriIpc;
use serde::Deserialize;
use std::process::{Command, Stdio};
use tracing::{debug, warn};

/// A window from the compositor
#[derive(Debug, Clone)]
pub struct Window {
    /// Compositor-specific window ID
    pub id: String,
    /// Window title
    pub title: String,
    /// Application ID (e.g., "firefox", "org.gnome.Nautilus")
    pub app_id: String,
    /// Workspace identifier (name or number)
    pub workspace: String,
}

/// Type of compositor/desktop environment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositorType {
    /// Hyprland compositor
    Hyprland,
    /// Niri compositor
    Niri,
    /// Sway compositor
    Sway,
    /// GNOME (no layer-shell support)
    Gnome,
    /// X11 (no layer-shell support)
    X11,
    /// Unknown compositor (assumed layer-shell support on Wayland)
    Unknown,
}

/// Compositor client for window management
pub struct Compositor {
    compositor_type: CompositorType,
    /// Direct IPC client for Niri (avoids spawning processes)
    niri_ipc: Option<NiriIpc>,
}

impl Compositor {
    /// Detect and create a compositor client based on environment variables
    pub fn detect() -> Self {
        let compositor_type = Self::detect_type();
        debug!("Detected compositor: {:?}", compositor_type);

        // Initialize Niri IPC if running under Niri
        let niri_ipc = if compositor_type == CompositorType::Niri {
            NiriIpc::from_env()
        } else {
            None
        };

        Self {
            compositor_type,
            niri_ipc,
        }
    }

    fn detect_type() -> CompositorType {
        if std::env::var("DISPLAY").is_ok() && std::env::var("WAYLAND_DISPLAY").is_err() {
            debug!("Detected X11 display");
            return CompositorType::X11;
        }

        if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
            debug!("Detected Hyprland via HYPRLAND_INSTANCE_SIGNATURE");
            return CompositorType::Hyprland;
        }

        if std::env::var("NIRI_SOCKET").is_ok() {
            debug!("Detected Niri via NIRI_SOCKET");
            return CompositorType::Niri;
        }

        if std::env::var("SWAYSOCK").is_ok() {
            debug!("Detected Sway via SWAYSOCK");
            return CompositorType::Sway;
        }

        if std::env::var("GNOME_DESKTOP_SESSION_ID").is_ok()
            || std::env::var("XDG_CURRENT_DESKTOP")
                .map(|d| d.to_lowercase().contains("gnome"))
                .unwrap_or(false)
        {
            debug!("Detected GNOME desktop");
            return CompositorType::Gnome;
        }

        if std::env::var("KDE_FULL_SESSION").is_ok() && std::env::var("WAYLAND_DISPLAY").is_ok() {
            debug!("Detected KDE Plasma on Wayland");
            return CompositorType::Unknown;
        }

        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            debug!("Detected generic Wayland display");
            return CompositorType::Unknown;
        }

        debug!("Could not detect compositor type");
        CompositorType::Unknown
    }

    /// Check if layer-shell is supported
    pub fn supports_layer_shell(&self) -> bool {
        matches!(
            self.compositor_type,
            CompositorType::Hyprland
                | CompositorType::Niri
                | CompositorType::Sway
                | CompositorType::Unknown
        )
    }

    /// List all windows from the compositor
    pub fn list_windows(&self) -> Vec<Window> {
        match self.compositor_type {
            CompositorType::Hyprland => self.hyprland_list_windows(),
            CompositorType::Niri => self.niri_list_windows(),
            CompositorType::Sway => self.sway_list_windows(),
            _ => Vec::new(),
        }
    }

    /// Get the set of all running app IDs (lowercase)
    ///
    /// Returns a `HashSet` of all `app_ids` from currently open windows.
    /// Used for determining which apps in the launcher have running instances.
    pub fn get_running_app_ids(&self) -> std::collections::HashSet<String> {
        self.list_windows()
            .into_iter()
            .map(|w| w.app_id.to_lowercase())
            .collect()
    }

    /// Find windows matching an `app_id` (case-insensitive)
    ///
    /// The `app_id` should be the `StartupWMClass` from the `.desktop` file,
    /// which matches what compositors report as the window's `app_id`.
    /// Also handles apps without `StartupWMClass` by trying name variations.
    pub fn find_windows_by_app_id(&self, app_id: &str) -> Vec<Window> {
        let app_id_lower = app_id.to_lowercase();

        // For apps without StartupWMClass, the window app_id might be:
        // - The app name with spaces: "MongoDB Compass"
        // - Hyphenated: "mongodb-compass"
        // Create normalized versions for comparison
        let normalized = app_id_lower.replace('-', " ");
        let hyphenated = app_id_lower.replace(' ', "-");

        self.list_windows()
            .into_iter()
            .filter(|w| {
                let window_lower = w.app_id.to_lowercase();
                let window_normalized = window_lower.replace('-', " ");

                window_lower == app_id_lower
                    || window_normalized == normalized
                    || window_lower == hyphenated
            })
            .collect()
    }

    /// Focus a specific window by ID
    pub fn focus_window(&self, window_id: &str) -> bool {
        match self.compositor_type {
            CompositorType::Hyprland => self.hyprland_focus_window(window_id),
            CompositorType::Niri => self.niri_focus_window(window_id),
            CompositorType::Sway => self.sway_focus_window(window_id),
            _ => false,
        }
    }

    /// Get the currently focused output/monitor name
    pub fn get_focused_output(&self) -> Option<String> {
        match self.compositor_type {
            CompositorType::Hyprland => self.hyprland_get_focused_output(),
            CompositorType::Niri => self.niri_get_focused_output(),
            CompositorType::Sway => self.sway_get_focused_output(),
            _ => None,
        }
    }

    #[allow(clippy::unused_self)] // Method for API consistency
    fn hyprland_list_windows(&self) -> Vec<Window> {
        #[derive(Deserialize)]
        struct HyprlandClient {
            address: String,
            title: String,
            class: String,
            workspace: HyprlandWorkspace,
        }

        #[derive(Deserialize)]
        struct HyprlandWorkspace {
            name: String,
        }

        let output = Command::new("hyprctl")
            .args(["clients", "-j"])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output();

        let Ok(output) = output else {
            warn!("Failed to run hyprctl clients");
            return Vec::new();
        };

        if !output.status.success() {
            warn!("hyprctl clients failed with status: {}", output.status);
            return Vec::new();
        }

        let Ok(clients) = serde_json::from_slice::<Vec<HyprlandClient>>(&output.stdout) else {
            warn!("Failed to parse hyprctl clients output");
            return Vec::new();
        };

        clients
            .into_iter()
            .map(|c| Window {
                id: c.address,
                title: c.title,
                app_id: c.class,
                workspace: c.workspace.name,
            })
            .collect()
    }

    #[allow(clippy::unused_self)] // Method for API consistency
    fn hyprland_focus_window(&self, window_id: &str) -> bool {
        let result = Command::new("hyprctl")
            .args(["dispatch", "focuswindow", &format!("address:{window_id}")])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        matches!(result, Ok(status) if status.success())
    }

    fn niri_list_windows(&self) -> Vec<Window> {
        let Some(ref ipc) = self.niri_ipc else {
            warn!("Niri IPC not available, cannot list windows");
            return Vec::new();
        };

        ipc.get_windows()
            .into_iter()
            .map(|w| Window {
                id: w.id.to_string(),
                title: w.title,
                app_id: w.app_id,
                workspace: w.workspace_id.to_string(),
            })
            .collect()
    }

    fn niri_focus_window(&self, window_id: &str) -> bool {
        let Some(ref ipc) = self.niri_ipc else {
            warn!("Niri IPC not available, cannot focus window");
            return false;
        };

        let Ok(id) = window_id.parse::<u64>() else {
            warn!("Invalid window ID for Niri: {}", window_id);
            return false;
        };

        ipc.focus_window(id)
    }

    #[allow(clippy::unused_self)] // Method for API consistency
    fn sway_list_windows(&self) -> Vec<Window> {
        #[derive(Deserialize)]
        struct SwayNode {
            id: i64,
            name: Option<String>,
            app_id: Option<String>,
            #[serde(default)]
            nodes: Vec<SwayNode>,
            #[serde(default)]
            floating_nodes: Vec<SwayNode>,
            #[serde(rename = "type")]
            node_type: Option<String>,
        }

        fn collect_windows(node: SwayNode, workspace: &str) -> Vec<Window> {
            let mut windows = Vec::new();

            let ws = if node.node_type.as_deref() == Some("workspace") {
                node.name.as_deref().unwrap_or(workspace)
            } else {
                workspace
            };

            if let Some(ref app_id) = node.app_id
                && !app_id.is_empty()
            {
                windows.push(Window {
                    id: node.id.to_string(),
                    title: node.name.clone().unwrap_or_default(),
                    app_id: app_id.clone(),
                    workspace: workspace.to_string(),
                });
            }

            for child in node.nodes {
                windows.extend(collect_windows(child, ws));
            }
            for child in node.floating_nodes {
                windows.extend(collect_windows(child, ws));
            }

            windows
        }

        let output = Command::new("swaymsg")
            .args(["-t", "get_tree"])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output();

        let Ok(output) = output else {
            warn!("Failed to run swaymsg");
            return Vec::new();
        };

        if !output.status.success() {
            warn!("swaymsg failed with status: {}", output.status);
            return Vec::new();
        }

        let Ok(root) = serde_json::from_slice::<SwayNode>(&output.stdout) else {
            warn!("Failed to parse swaymsg output");
            return Vec::new();
        };

        collect_windows(root, "")
    }

    #[allow(clippy::unused_self)] // Method for API consistency
    fn sway_focus_window(&self, window_id: &str) -> bool {
        let result = Command::new("swaymsg")
            .args([&format!("[con_id={window_id}] focus")])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        matches!(result, Ok(status) if status.success())
    }

    #[allow(clippy::unused_self)] // Method for API consistency
    fn hyprland_get_focused_output(&self) -> Option<String> {
        #[derive(Deserialize)]
        struct HyprlandMonitor {
            name: String,
            focused: bool,
        }

        let output = Command::new("hyprctl")
            .args(["monitors", "-j"])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let monitors: Vec<HyprlandMonitor> = serde_json::from_slice(&output.stdout).ok()?;
        monitors.into_iter().find(|m| m.focused).map(|m| m.name)
    }

    fn niri_get_focused_output(&self) -> Option<String> {
        let Some(ref ipc) = self.niri_ipc else {
            warn!("Niri IPC not available, cannot get focused output");
            return None;
        };

        ipc.get_focused_output_name()
    }

    #[allow(clippy::unused_self)] // Method for API consistency
    fn sway_get_focused_output(&self) -> Option<String> {
        #[derive(Deserialize)]
        struct SwayOutput {
            name: String,
            focused: bool,
        }

        let output = Command::new("swaymsg")
            .args(["-t", "get_outputs"])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let outputs: Vec<SwayOutput> = serde_json::from_slice(&output.stdout).ok()?;
        outputs.into_iter().find(|o| o.focused).map(|o| o.name)
    }
}

impl Default for Compositor {
    fn default() -> Self {
        Self::detect()
    }
}

impl std::fmt::Debug for Compositor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Compositor")
            .field("compositor_type", &self.compositor_type)
            .field("niri_ipc", &self.niri_ipc.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compositor_detection() {
        let _compositor = Compositor::detect();
    }
}
