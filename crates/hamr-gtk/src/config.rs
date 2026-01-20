//! Configuration loading and file watching

use crate::colors::Colors;
use gtk4::glib;
use gtk4::prelude::*;
use serde::Deserialize;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use tracing::{debug, info, warn};

/// Callback type for theme change events
type ThemeChangeCallback = Rc<RefCell<Option<Box<dyn Fn(&Theme)>>>>;

/// Grid layout configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GridConfig {
    /// Number of columns in grid view
    #[serde(default = "default_grid_columns")]
    pub columns: u32,
    /// Width of each grid item in pixels (visual + padding)
    #[serde(default = "default_grid_item_width")]
    pub item_width: i32,
    /// Spacing between grid items in pixels
    #[serde(default = "default_grid_spacing")]
    pub spacing: i32,
}

fn default_grid_columns() -> u32 {
    5
}
fn default_grid_item_width() -> i32 {
    110 // adjusted to tighten grid further
}
fn default_grid_spacing() -> i32 {
    2
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            columns: default_grid_columns(),
            item_width: default_grid_item_width(),
            spacing: default_grid_spacing(),
        }
    }
}

impl GridConfig {
    /// Calculate the total width needed for the grid container
    /// Formula: `columns * item_width + (columns - 1) * spacing + container_padding`
    // Columns is usize (grid count), GTK width is i32
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub fn calculate_width(&self) -> i32 {
        let columns = self.columns as i32;
        columns * self.item_width + (columns - 1) * self.spacing
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceConfig {
    #[serde(default = "default_bg_transparency")]
    pub background_transparency: f64,
    #[serde(default = "default_content_transparency")]
    pub content_transparency: f64,
    #[serde(default = "default_x_ratio")]
    pub launcher_x_ratio: f64,
    #[serde(default = "default_y_ratio")]
    pub launcher_y_ratio: f64,
    #[serde(default)]
    pub font_scale: f64,
    /// UI scale factor for spacing, padding, margins, icons (default 1.0)
    #[serde(default = "default_ui_scale")]
    pub ui_scale: f64,
    /// Default result view mode: "list" or "grid"
    #[serde(default = "default_result_view")]
    pub default_result_view: String,
    /// Grid layout settings
    #[serde(default)]
    pub grid: GridConfig,
}

fn default_bg_transparency() -> f64 {
    0.2
}
fn default_content_transparency() -> f64 {
    0.2
}
fn default_x_ratio() -> f64 {
    0.5
}
fn default_y_ratio() -> f64 {
    0.1
}
fn default_ui_scale() -> f64 {
    1.0
}
fn default_result_view() -> String {
    "list".to_string()
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            background_transparency: default_bg_transparency(),
            content_transparency: default_content_transparency(),
            launcher_x_ratio: default_x_ratio(),
            launcher_y_ratio: default_y_ratio(),
            font_scale: 1.0,
            ui_scale: default_ui_scale(),
            default_result_view: default_result_view(),
            grid: GridConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SizesConfig {
    #[serde(default = "default_search_width")]
    pub search_width: i32,
    #[serde(default = "default_max_results")]
    pub max_results_height: i32,
}

fn default_search_width() -> i32 {
    640
}
fn default_max_results() -> i32 {
    600
}

impl Default for SizesConfig {
    fn default() -> Self {
        Self {
            search_width: default_search_width(),
            max_results_height: default_max_results(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FontsConfig {
    #[serde(default = "default_main_font")]
    pub main: String,
    #[serde(default = "default_mono_font")]
    pub monospace: String,
    #[serde(default = "default_icon_font")]
    pub icon: String,
}

fn default_main_font() -> String {
    "Google Sans Flex".to_string()
}
fn default_mono_font() -> String {
    "JetBrains Mono NF".to_string()
}
fn default_icon_font() -> String {
    "Material Symbols Rounded".to_string()
}

impl Default for FontsConfig {
    fn default() -> Self {
        Self {
            main: default_main_font(),
            monospace: default_mono_font(),
            icon: default_icon_font(),
        }
    }
}

/// Click-outside behavior mode
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClickOutsideAction {
    /// Always close (hide both launcher and FAB)
    Close,
    /// Always minimize to FAB
    Minimize,
    /// Smart: minimize if user has used Ctrl+M before, otherwise close
    #[default]
    Intuitive,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BehaviorConfig {
    /// What happens when clicking outside the launcher
    #[serde(default)]
    pub click_outside_action: ClickOutsideAction,
    /// How long to preserve state after soft close (ms)
    #[serde(default = "default_state_restore_window_ms")]
    pub state_restore_window_ms: u64,
}

fn default_state_restore_window_ms() -> u64 {
    30000
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            click_outside_action: ClickOutsideAction::default(),
            state_restore_window_ms: default_state_restore_window_ms(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub appearance: AppearanceConfig,
    #[serde(default)]
    pub sizes: SizesConfig,
    #[serde(default)]
    pub fonts: FontsConfig,
    #[serde(default)]
    pub behavior: BehaviorConfig,
}

impl Config {
    pub fn load() -> Self {
        let path = Self::config_path();

        if path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            hamr_core::config::warn_unknown_gtk_fields(&content, "config.json");
            match serde_json::from_str(&content) {
                Ok(config) => {
                    info!("Loaded config from {:?}", path);
                    return config;
                }
                Err(e) => {
                    warn!("Failed to parse config.json: {e}");
                }
            }
        }

        info!("Using default config");
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

        config_dir.join("hamr").join("config.json")
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub colors: Colors,
    pub config: Config,
}

impl Theme {
    pub fn load() -> Self {
        Self {
            colors: Colors::load(),
            config: Config::load(),
        }
    }

    /// Get background opacity (1.0 - transparency)
    pub fn bg_opacity(&self) -> f64 {
        1.0 - self.config.appearance.background_transparency
    }

    /// Get content opacity (1.0 - transparency)
    pub fn content_opacity(&self) -> f64 {
        1.0 - self.config.appearance.content_transparency
    }

    /// Get font scale factor (defaults to 1.0)
    pub fn font_scale(&self) -> f64 {
        let scale = self.config.appearance.font_scale;
        if scale <= 0.0 { 1.0 } else { scale }
    }

    /// Get UI scale factor for spacing, padding, margins, icons (defaults to 1.0)
    pub fn ui_scale(&self) -> f64 {
        let scale = self.config.appearance.ui_scale;
        if scale <= 0.0 { 1.0 } else { scale }
    }

    /// Scale an integer dimension (padding, margin, icon size) by `ui_scale`
    // Scaled dimensions: f64 math back to i32 for GTK sizing
    #[allow(clippy::cast_possible_truncation)]
    pub fn scaled(&self, base: i32) -> i32 {
        (f64::from(base) * self.ui_scale()).round() as i32
    }

    /// Scale a font size by both `ui_scale` and `font_scale` factors
    // Scaled dimensions: f64 math back to i32 for GTK sizing
    #[allow(clippy::cast_possible_truncation)]
    pub fn scaled_font(&self, base_size: i32) -> i32 {
        (f64::from(base_size) * self.ui_scale() * self.font_scale()).round() as i32
    }
}

pub struct ConfigWatcher {
    theme: Rc<RefCell<Theme>>,
    on_change: ThemeChangeCallback,
}

impl ConfigWatcher {
    pub fn new() -> Self {
        Self {
            theme: Rc::new(RefCell::new(Theme::load())),
            on_change: Rc::new(RefCell::new(None)),
        }
    }

    pub fn theme(&self) -> Theme {
        self.theme.borrow().clone()
    }

    pub fn set_on_change<F: Fn(&Theme) + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    pub fn start_watching(&self) {
        let config_dir = std::env::var("XDG_CONFIG_HOME")
            .map_or_else(
                |_| {
                    dirs::home_dir()
                        .map(|h| h.join(".config"))
                        .unwrap_or_default()
                },
                PathBuf::from,
            )
            .join("hamr");

        self.watch_file(&config_dir.join("colors.json"));
        self.watch_file(&config_dir.join("config.json"));
    }

    fn watch_file(&self, path: &PathBuf) {
        let file = gtk4::gio::File::for_path(path);
        let monitor = match file.monitor_file(
            gtk4::gio::FileMonitorFlags::NONE,
            gtk4::gio::Cancellable::NONE,
        ) {
            Ok(m) => m,
            Err(e) => {
                warn!("Failed to watch {:?}: {}", path, e);
                return;
            }
        };

        let theme = self.theme.clone();
        let on_change = self.on_change.clone();
        let path_clone = path.clone();

        monitor.connect_changed(move |_, _, _, event| {
            if matches!(
                event,
                gtk4::gio::FileMonitorEvent::Changed | gtk4::gio::FileMonitorEvent::Created
            ) {
                debug!("Config file changed: {:?}", path_clone);

                // Debounce: wait a bit for file to be fully written
                let theme = theme.clone();
                let on_change = on_change.clone();

                glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
                    let new_theme = Theme::load();
                    *theme.borrow_mut() = new_theme.clone();

                    if let Some(ref callback) = *on_change.borrow() {
                        callback(&new_theme);
                    }
                });
            }
        });

        std::mem::forget(monitor);

        info!("Watching {:?}", path);
    }
}
