//! Compositor integration for Wayland (Hyprland, Niri)

use serde::Deserialize;

use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct Window {
    pub id: String,
    pub title: String,
    pub app_id: String,
    pub workspace: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositorType {
    Hyprland,
    Niri,
    Unknown,
}

pub struct Compositor {
    compositor_type: CompositorType,
}

impl Compositor {
    pub fn detect() -> Self {
        let compositor_type = if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
            CompositorType::Hyprland
        } else if std::env::var("NIRI_SOCKET").is_ok() {
            CompositorType::Niri
        } else {
            CompositorType::Unknown
        };

        tracing::info!("Detected compositor: {:?}", compositor_type);
        Self { compositor_type }
    }

    pub fn list_windows(&self) -> Vec<Window> {
        match self.compositor_type {
            CompositorType::Hyprland => self.hyprland_list_windows(),
            CompositorType::Niri => self.niri_list_windows(),
            CompositorType::Unknown => Vec::new(),
        }
    }

    pub fn find_windows_by_app_id(&self, app_id: &str) -> Vec<Window> {
        self.list_windows()
            .into_iter()
            .filter(|w| w.app_id.eq_ignore_ascii_case(app_id))
            .collect()
    }

    pub fn focus_window(&self, window_id: &str) -> bool {
        match self.compositor_type {
            CompositorType::Hyprland => self.hyprland_focus_window(window_id),
            CompositorType::Niri => self.niri_focus_window(window_id),
            CompositorType::Unknown => false,
        }
    }

    pub fn get_running_app_ids(&self) -> std::collections::HashSet<String> {
        self.list_windows()
            .into_iter()
            .map(|w| w.app_id.to_lowercase())
            .collect()
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
            tracing::warn!("Failed to run hyprctl clients");
            return Vec::new();
        };

        if !output.status.success() {
            tracing::warn!("hyprctl clients failed with status: {}", output.status);
            return Vec::new();
        }

        let Ok(clients) = serde_json::from_slice::<Vec<HyprlandClient>>(&output.stdout) else {
            tracing::warn!("Failed to parse hyprctl clients output");
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

    #[allow(clippy::unused_self)] // Method for API consistency
    fn niri_list_windows(&self) -> Vec<Window> {
        #[derive(Deserialize)]
        struct NiriWindow {
            id: u64,
            title: String,
            app_id: String,
            workspace_id: u64,
        }

        let output = Command::new("niri")
            .args(["msg", "--json", "windows"])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output();

        let Ok(output) = output else {
            tracing::warn!("Failed to run niri msg windows");
            return Vec::new();
        };

        if !output.status.success() {
            tracing::warn!("niri msg windows failed with status: {}", output.status);
            return Vec::new();
        }

        let Ok(windows) = serde_json::from_slice::<Vec<NiriWindow>>(&output.stdout) else {
            tracing::warn!("Failed to parse niri msg windows output");
            return Vec::new();
        };

        windows
            .into_iter()
            .map(|w| Window {
                id: w.id.to_string(),
                title: w.title,
                app_id: w.app_id,
                workspace: w.workspace_id.to_string(),
            })
            .collect()
    }

    #[allow(clippy::unused_self)] // Method for API consistency
    fn niri_focus_window(&self, window_id: &str) -> bool {
        let result = Command::new("niri")
            .args(["msg", "action", "focus-window", "--id", window_id])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        matches!(result, Ok(status) if status.success())
    }
}

impl Default for Compositor {
    fn default() -> Self {
        Self::detect()
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
