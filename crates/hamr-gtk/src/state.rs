//! Persistent state management for launcher position and other settings
//!
//! Stores state in ~/.local/share/hamr/states.json following QML implementation pattern.
//! Position is stored as ratios (0.0-1.0) for resolution independence.

use hamr_rpc::SearchResult;
use hamr_types::{FabOverride, PreviewData};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Launcher visibility state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LauncherVisibility {
    /// Launcher window is visible
    #[default]
    Open,
    /// FAB visible, launcher hidden
    Minimized,
    /// Both hidden
    Closed,
}

/// Position as ratios for resolution independence
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionRatio {
    /// X position as ratio of screen width (0.0-1.0), centered on widget
    #[serde(default = "default_x_ratio")]
    pub x_ratio: f64,
    /// Y position as ratio of screen height (0.0-1.0), from top edge
    #[serde(default = "default_y_ratio")]
    pub y_ratio: f64,
}

impl Default for PositionRatio {
    fn default() -> Self {
        Self {
            x_ratio: default_x_ratio(),
            y_ratio: default_y_ratio(),
        }
    }
}

impl PositionRatio {
    pub fn new(x_ratio: f64, y_ratio: f64) -> Self {
        Self { x_ratio, y_ratio }
    }

    pub fn with_fab_defaults() -> Self {
        Self {
            x_ratio: 0.5,
            y_ratio: 0.9,
        }
    }
}

/// Launcher position and state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherState {
    /// X position as ratio of screen width (0.0-1.0), centered on launcher
    #[serde(default = "default_x_ratio")]
    pub x_ratio: f64,
    /// Y position as ratio of screen height (0.0-1.0), from top edge
    #[serde(default = "default_y_ratio")]
    pub y_ratio: f64,
    /// Whether user has ever used minimize (Ctrl+M) - enables intuitive mode
    #[serde(default)]
    pub has_used_minimize: bool,
    /// Whether compact mode is enabled
    #[serde(default)]
    pub compact_mode: bool,
    /// Per-monitor launcher positions (keyed by monitor connector name)
    #[serde(default)]
    pub monitor_positions: HashMap<String, PositionRatio>,
    /// Last used monitor connector name (for restoring FAB position on restart)
    #[serde(default)]
    pub last_monitor: Option<String>,
}

fn default_x_ratio() -> f64 {
    0.5
}

fn default_y_ratio() -> f64 {
    0.1
}

impl Default for LauncherState {
    fn default() -> Self {
        Self {
            x_ratio: default_x_ratio(),
            y_ratio: default_y_ratio(),
            has_used_minimize: false,
            compact_mode: false,
            monitor_positions: HashMap::new(),
            last_monitor: None,
        }
    }
}

/// Pinned panel state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinnedPanelState {
    /// Unique identifier for the pinned panel
    pub id: String,
    /// Item ID that was pinned
    pub item_id: String,
    /// Title of the pinned content
    pub title: Option<String>,
    /// Preview data serialized for persistence
    pub preview: PreviewData,
    /// Position as screen-relative ratios
    pub position: PositionRatio,
    /// Monitor connector name (for multi-monitor support)
    #[serde(default)]
    pub monitor: Option<String>,
}

impl PinnedPanelState {
    pub fn new(
        item_id: String,
        title: Option<String>,
        preview: PreviewData,
        x_ratio: f64,
        y_ratio: f64,
        monitor: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            item_id,
            title,
            preview,
            position: PositionRatio::new(x_ratio, y_ratio),
            monitor,
        }
    }
}

/// Root state structure matching QML states.json format
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct States {
    #[serde(default)]
    pub launcher: LauncherState,
    #[serde(default)]
    pub form: FormState,
    /// Per-monitor FAB positions (keyed by monitor connector name)
    #[serde(default)]
    pub fab_positions: HashMap<String, PositionRatio>,
    /// Pinned preview panels (sticky notes)
    #[serde(default)]
    pub pinned_panels: Vec<PinnedPanelState>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormState {
    #[serde(default)]
    pub values: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub valid: bool,
}

/// Session state for restoration after soft close
#[derive(Debug, Clone, Default)]
pub struct SessionState {
    /// The query text
    pub query: String,
    /// The search results
    pub results: Vec<SearchResult>,
    /// The active plugin (if any)
    pub active_plugin: Option<(String, String)>,
}

/// Runtime launcher visibility state (not persisted)
pub struct VisibilityState {
    visibility: Cell<LauncherVisibility>,
    /// Whether user has ever used minimize (Ctrl+M)
    has_used_minimize: Cell<bool>,
    /// When soft close occurred (for state preservation window)
    soft_close_time: RefCell<Option<Instant>>,
    /// Session state saved on soft close for potential restoration
    saved_session: RefCell<Option<SessionState>>,
    /// FAB overrides from plugins, keyed by `plugin_id`
    fab_overrides: RefCell<HashMap<String, FabOverride>>,
    /// Resolved active FAB override (highest priority wins)
    active_fab_override: RefCell<Option<FabOverride>>,
}

impl Default for VisibilityState {
    fn default() -> Self {
        Self::new()
    }
}

impl VisibilityState {
    pub fn new() -> Self {
        Self {
            visibility: Cell::new(LauncherVisibility::Open),
            has_used_minimize: Cell::new(false),
            soft_close_time: RefCell::new(None),
            saved_session: RefCell::new(None),
            fab_overrides: RefCell::new(HashMap::new()),
            active_fab_override: RefCell::new(None),
        }
    }

    pub fn visibility(&self) -> LauncherVisibility {
        self.visibility.get()
    }

    pub fn has_used_minimize(&self) -> bool {
        self.has_used_minimize.get()
    }

    pub fn set_has_used_minimize(&self, value: bool) {
        self.has_used_minimize.set(value);
    }

    pub fn active_fab_override(&self) -> Option<FabOverride> {
        self.active_fab_override.borrow().clone()
    }

    /// Transition to Open state
    pub fn open(&self) {
        if self.soft_close_time.borrow().is_some() {
            *self.soft_close_time.borrow_mut() = None;
            debug!("Reopened within state preservation window");
        }
        self.visibility.set(LauncherVisibility::Open);
        debug!("Launcher visibility: Open");
    }

    /// Transition to Minimized state (shows FAB)
    pub fn minimize(&self) {
        self.has_used_minimize.set(true);
        self.visibility.set(LauncherVisibility::Minimized);
        *self.soft_close_time.borrow_mut() = Some(Instant::now());
        debug!("Launcher visibility: Minimized");
    }

    /// Transition to Closed state
    pub fn close(&self) {
        self.visibility.set(LauncherVisibility::Closed);
        *self.soft_close_time.borrow_mut() = Some(Instant::now());
        debug!("Launcher visibility: Closed (soft close)");
    }

    /// Hard close - clear state immediately
    pub fn hard_close(&self) {
        self.visibility.set(LauncherVisibility::Closed);
        *self.soft_close_time.borrow_mut() = None;
        debug!("Launcher visibility: Closed (hard close)");
    }

    /// Check if within state preservation window
    pub fn is_within_restore_window(&self, window_ms: u64) -> bool {
        if let Some(close_time) = *self.soft_close_time.borrow() {
            close_time.elapsed().as_millis() < u128::from(window_ms)
        } else {
            false
        }
    }

    /// Save session state for potential restoration
    pub fn save_session(&self, session: SessionState) {
        *self.saved_session.borrow_mut() = Some(session);
        debug!("Session state saved for potential restoration");
    }

    /// Take saved session if within restore window, clearing it
    pub fn take_session_if_restorable(&self, window_ms: u64) -> Option<SessionState> {
        if self.is_within_restore_window(window_ms) {
            let session = self.saved_session.borrow_mut().take();
            if session.is_some() {
                debug!("Restoring saved session state");
            }
            session
        } else {
            *self.saved_session.borrow_mut() = None;
            None
        }
    }

    /// Update FAB override from a plugin
    pub fn update_fab_override(&self, plugin_id: &str, fab: Option<&FabOverride>) {
        {
            let mut overrides = self.fab_overrides.borrow_mut();
            if let Some(data) = fab {
                debug!(
                    "FAB override from {}: chips={}, badges={}, priority={}",
                    plugin_id,
                    data.chips.len(),
                    data.badges.len(),
                    data.priority
                );
                if data.show_fab && self.visibility.get() == LauncherVisibility::Closed {
                    self.visibility.set(LauncherVisibility::Minimized);
                    debug!("Plugin {} forced FAB visible", plugin_id);
                }
                overrides.insert(plugin_id.to_string(), data.clone());
            } else {
                debug!("FAB override cleared for {}", plugin_id);
                overrides.remove(plugin_id);
            }
        }
        self.resolve_fab_override();
    }

    fn resolve_fab_override(&self) {
        let overrides = self.fab_overrides.borrow();
        let best = overrides.values().max_by_key(|o| o.priority).cloned();
        debug!(
            "Resolved FAB override: {} overrides ({:?}), active={:?}",
            overrides.len(),
            overrides.keys().collect::<Vec<_>>(),
            best.as_ref().map(|b| (b.chips.len(), b.badges.len()))
        );
        *self.active_fab_override.borrow_mut() = best;
    }
}

/// Manages persistent state storage
pub struct StateManager {
    states: Rc<RefCell<States>>,
    file_path: PathBuf,
    save_pending: Rc<RefCell<bool>>,
    visibility_state: Rc<VisibilityState>,
}

impl StateManager {
    pub fn new() -> Self {
        let file_path = Self::state_file_path();
        let states = Self::load_from_file(&file_path);

        let visibility_state = VisibilityState::new();
        visibility_state.set_has_used_minimize(states.launcher.has_used_minimize);

        Self {
            states: Rc::new(RefCell::new(states)),
            file_path,
            save_pending: Rc::new(RefCell::new(false)),
            visibility_state: Rc::new(visibility_state),
        }
    }

    /// Get visibility state for runtime state transitions
    pub fn visibility_state(&self) -> Rc<VisibilityState> {
        self.visibility_state.clone()
    }

    fn state_file_path() -> PathBuf {
        let state_dir = std::env::var("XDG_STATE_HOME")
            .map_or_else(
                |_| {
                    dirs::home_dir()
                        .map(|h| h.join(".local/state"))
                        .unwrap_or_default()
                },
                PathBuf::from,
            )
            .join("hamr");

        state_dir.join("states.json")
    }

    fn load_from_file(path: &PathBuf) -> States {
        if path.exists() {
            match std::fs::read_to_string(path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(states) => {
                        info!("Loaded state from {:?}", path);
                        return states;
                    }
                    Err(e) => {
                        warn!("Failed to parse states.json: {}", e);
                    }
                },
                Err(e) => {
                    warn!("Failed to read states.json: {}", e);
                }
            }
        }

        info!("Using default state");
        States::default()
    }

    /// Get current launcher state
    pub fn launcher(&self) -> LauncherState {
        self.states.borrow().launcher.clone()
    }

    /// Update launcher position (as ratios) - legacy global position
    pub fn set_launcher_position(&self, x_ratio: f64, y_ratio: f64) {
        {
            let mut states = self.states.borrow_mut();
            states.launcher.x_ratio = x_ratio.clamp(0.0, 1.0);
            states.launcher.y_ratio = y_ratio.clamp(0.0, 1.0);
        }
        self.schedule_save();
    }

    /// Get launcher position for a specific monitor, falling back to global position
    pub fn launcher_position_for_monitor(&self, monitor_name: &str) -> PositionRatio {
        let states = self.states.borrow();
        states
            .launcher
            .monitor_positions
            .get(monitor_name)
            .cloned()
            .unwrap_or_else(|| PositionRatio::new(states.launcher.x_ratio, states.launcher.y_ratio))
    }

    /// Update launcher position for a specific monitor
    pub fn set_launcher_position_for_monitor(
        &self,
        monitor_name: &str,
        x_ratio: f64,
        y_ratio: f64,
    ) {
        {
            let mut states = self.states.borrow_mut();
            states.launcher.monitor_positions.insert(
                monitor_name.to_string(),
                PositionRatio::new(x_ratio.clamp(0.0, 1.0), y_ratio.clamp(0.0, 1.0)),
            );
        }
        self.schedule_save();
    }

    /// Get FAB position for a specific monitor, falling back to default
    pub fn fab_position_for_monitor(&self, monitor_name: &str) -> PositionRatio {
        let states = self.states.borrow();
        states
            .fab_positions
            .get(monitor_name)
            .cloned()
            .unwrap_or_else(PositionRatio::with_fab_defaults)
    }

    /// Update FAB position for a specific monitor
    pub fn set_fab_position_for_monitor(&self, monitor_name: &str, x_ratio: f64, y_ratio: f64) {
        {
            let mut states = self.states.borrow_mut();
            states.fab_positions.insert(
                monitor_name.to_string(),
                PositionRatio::new(x_ratio.clamp(0.0, 1.0), y_ratio.clamp(0.0, 1.0)),
            );
        }
        self.schedule_save();
    }

    /// Mark that user has used minimize (persisted for intuitive mode)
    pub fn set_has_used_minimize(&self) {
        if self.visibility_state.has_used_minimize() {
            return;
        }
        self.visibility_state.set_has_used_minimize(true);
        self.states.borrow_mut().launcher.has_used_minimize = true;
        self.schedule_save();
        debug!("Persisted has_used_minimize = true (intuitive mode enabled)");
    }

    /// Reset hasUsedMinimize (called when FAB close button is clicked)
    pub fn reset_has_used_minimize(&self) {
        if !self.visibility_state.has_used_minimize() {
            return;
        }
        self.visibility_state.set_has_used_minimize(false);
        self.states.borrow_mut().launcher.has_used_minimize = false;
        self.schedule_save();
        debug!("Persisted has_used_minimize = false (intuitive mode disabled)");
    }

    /// Get compact mode preference
    pub fn compact_mode(&self) -> bool {
        self.states.borrow().launcher.compact_mode
    }

    /// Set compact mode preference
    pub fn set_compact_mode(&self, value: bool) {
        if self.states.borrow().launcher.compact_mode == value {
            return;
        }
        self.states.borrow_mut().launcher.compact_mode = value;
        self.schedule_save();
        debug!("Persisted compact_mode = {}", value);
    }

    /// Get the last used monitor name
    pub fn last_monitor(&self) -> Option<String> {
        self.states.borrow().launcher.last_monitor.clone()
    }

    /// Set the last used monitor name
    pub fn set_last_monitor(&self, monitor_name: &str) {
        let current = self.states.borrow().launcher.last_monitor.clone();
        if current.as_deref() == Some(monitor_name) {
            return;
        }
        self.states.borrow_mut().launcher.last_monitor = Some(monitor_name.to_string());
        self.schedule_save();
        debug!("Persisted last_monitor = {}", monitor_name);
    }

    /// Get all pinned panels
    pub fn pinned_panels(&self) -> Vec<PinnedPanelState> {
        self.states.borrow().pinned_panels.clone()
    }

    /// Add a new pinned panel
    pub fn add_pinned_panel(&self, panel: PinnedPanelState) -> String {
        let id = panel.id.clone();
        self.states.borrow_mut().pinned_panels.push(panel);
        self.schedule_save();
        debug!("Added pinned panel: {}", id);
        id
    }

    /// Remove a pinned panel by ID
    pub fn remove_pinned_panel(&self, id: &str) {
        self.states
            .borrow_mut()
            .pinned_panels
            .retain(|p| p.id != id);
        self.schedule_save();
        debug!("Removed pinned panel: {}", id);
    }

    /// Update pinned panel position
    pub fn update_pinned_panel_position(&self, id: &str, x_ratio: f64, y_ratio: f64) {
        let mut states = self.states.borrow_mut();
        if let Some(panel) = states.pinned_panels.iter_mut().find(|p| p.id == id) {
            panel.position = PositionRatio::new(x_ratio.clamp(0.0, 1.0), y_ratio.clamp(0.0, 1.0));
        }
        drop(states);
        self.schedule_save();
    }

    /// Schedule a debounced save
    fn schedule_save(&self) {
        if *self.save_pending.borrow() {
            return;
        }

        *self.save_pending.borrow_mut() = true;

        let states = self.states.clone();
        let file_path = self.file_path.clone();
        let save_pending = self.save_pending.clone();

        gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
            *save_pending.borrow_mut() = false;
            Self::save_to_file(&states.borrow(), &file_path);
        });
    }

    fn save_to_file(states: &States, path: &PathBuf) {
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            error!("Failed to create state directory: {}", e);
            return;
        }

        match serde_json::to_string_pretty(states) {
            Ok(content) => match std::fs::write(path, content) {
                Ok(()) => {
                    debug!("Saved state to {:?}", path);
                }
                Err(e) => {
                    error!("Failed to write states.json: {}", e);
                }
            },
            Err(e) => {
                error!("Failed to serialize state: {}", e);
            }
        }
    }
}
