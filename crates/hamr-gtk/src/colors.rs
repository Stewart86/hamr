//! Colors loading from colors.json

use serde::Deserialize;
use std::path::PathBuf;

/// Material Design 3 colors from colors.json
#[derive(Debug, Clone, Deserialize)]
pub struct Colors {
    pub background: String,
    pub surface: String,
    pub surface_container: String,
    pub surface_container_low: String,
    pub surface_container_high: String,
    #[serde(default = "default_surface_container_highest")]
    pub surface_container_highest: String,
    pub on_surface: String,
    pub on_surface_variant: String,
    pub outline: String,
    pub outline_variant: String,
    pub primary: String,
    pub primary_container: String,
    pub on_primary_container: String,
    #[serde(default = "default_on_primary")]
    pub on_primary: String,
    pub secondary: String,
    pub secondary_container: String,
    #[serde(default = "default_on_secondary_container")]
    pub on_secondary_container: String,
    pub shadow: String,
}

fn default_on_secondary_container() -> String {
    "#cbc5c8".to_string()
}

fn default_on_primary() -> String {
    "#1c1b1c".to_string()
}

fn default_surface_container_highest() -> String {
    "#363435".to_string()
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            background: "#141313".to_string(),
            surface: "#141313".to_string(),
            surface_container: "#201f20".to_string(),
            surface_container_low: "#1c1b1c".to_string(),
            surface_container_high: "#2b2a2a".to_string(),
            surface_container_highest: "#363435".to_string(),
            on_surface: "#e6e1e1".to_string(),
            on_surface_variant: "#cbc5ca".to_string(),
            outline: "#948f94".to_string(),
            outline_variant: "#49464a".to_string(),
            primary: "#cbc4cb".to_string(),
            primary_container: "#2d2a2f".to_string(),
            on_primary_container: "#bcb6bc".to_string(),
            on_primary: "#1c1b1c".to_string(),
            secondary: "#cac5c8".to_string(),
            secondary_container: "#4d4b4d".to_string(),
            on_secondary_container: "#cbc5c8".to_string(),
            shadow: "#000000".to_string(),
        }
    }
}

impl Colors {
    /// Load colors from `XDG_CONFIG_HOME/hamr/colors.json`
    pub fn load() -> Self {
        let path = Self::config_path();

        if path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            if let Ok(colors) = serde_json::from_str(&content) {
                tracing::info!("Loaded colors from {:?}", path);
                return colors;
            }
            tracing::warn!("Failed to parse colors.json, using defaults");
        }

        tracing::info!("Using default colors");
        Self::default()
    }

    fn config_path() -> PathBuf {
        let config_dir = std::env::var("XDG_CONFIG_HOME").map_or_else(
            |_| {
                dirs::home_dir()
                    .map(|h| h.join(".config"))
                    .unwrap_or_default()
            },
            PathBuf::from,
        );

        config_dir.join("hamr").join("colors.json")
    }
}
