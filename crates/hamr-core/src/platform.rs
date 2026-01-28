//! Platform detection for hamr
//!
//! Detects the current platform/compositor for filtering plugins.

use std::env;
use std::path::Path;

/// Supported platforms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Niri compositor (Wayland)
    Niri,
    /// Hyprland compositor (Wayland)
    Hyprland,
    /// macOS
    MacOS,
    /// Windows
    Windows,
    /// Unknown/unsupported platform
    Unknown,
}

impl Platform {
    /// Get the platform identifier string used in manifest.json
    pub fn as_str(self) -> &'static str {
        match self {
            Platform::Niri => "niri",
            Platform::Hyprland => "hyprland",
            Platform::MacOS => "macos",
            Platform::Windows => "windows",
            Platform::Unknown => "unknown",
        }
    }
}

/// Fallback for processes started before compositor set env vars
#[cfg(target_os = "linux")]
fn niri_socket_exists() -> bool {
    let Some(runtime_dir) = env::var("XDG_RUNTIME_DIR").ok() else {
        return false;
    };

    if let Ok(entries) = std::fs::read_dir(&runtime_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str()
                && name.starts_with("niri.")
                && Path::new(name)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("sock"))
            {
                return true;
            }
        }
    }
    false
}

#[cfg(target_os = "linux")]
fn hyprland_socket_exists() -> bool {
    let Some(runtime_dir) = env::var("XDG_RUNTIME_DIR").ok() else {
        return false;
    };

    let hypr_dir = Path::new(&runtime_dir).join("hypr");
    if let Ok(entries) = std::fs::read_dir(&hypr_dir) {
        for entry in entries.flatten() {
            if entry.path().join(".socket.sock").exists() {
                return true;
            }
        }
    }
    false
}

pub fn detect() -> Platform {
    #[cfg(target_os = "macos")]
    return Platform::MacOS;

    #[cfg(target_os = "windows")]
    return Platform::Windows;

    #[cfg(target_os = "linux")]
    {
        if env::var("NIRI_SOCKET").is_ok() || niri_socket_exists() {
            return Platform::Niri;
        }

        if env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() || hyprland_socket_exists() {
            return Platform::Hyprland;
        }

        Platform::Unknown
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    Platform::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_as_str() {
        assert_eq!(Platform::Niri.as_str(), "niri");
        assert_eq!(Platform::Hyprland.as_str(), "hyprland");
        assert_eq!(Platform::MacOS.as_str(), "macos");
        assert_eq!(Platform::Windows.as_str(), "windows");
        assert_eq!(Platform::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_detect_returns_valid_platform() {
        let platform = detect();
        // Just verify it doesn't panic and returns a valid variant
        let _ = platform.as_str();
    }
}
