//! Application state and update handling for the TUI.

use crate::compositor::Compositor;
use crate::state::{
    CardState, ErrorState, FormState, GridBrowserState, ImageBrowserState, ViewMode,
    WindowPickerState,
};
use hamr_rpc::{
    AmbientItem, CoreUpdate, ExecuteAction, InputMode, PluginAction, PluginStatus, PreviewData,
    ResultPatch, ResultType, SearchResult, WidgetData,
};
use ratatui::widgets::ListState;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tracing::warn;

/// Spawn a command with all I/O redirected to null (fire and forget)
fn spawn_silent(program: &str, args: &[&str]) {
    let _ = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

/// Copy text to clipboard using wl-copy or xclip fallback
fn copy_to_clipboard(text: &str) {
    let result = Command::new("wl-copy")
        .arg(text)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    if result.is_err() {
        let _ = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(stdin) = child.stdin.as_mut() {
                    stdin.write_all(text.as_bytes())?;
                }
                Ok(())
            });
    }
}

/// Main application state
#[allow(clippy::struct_excessive_bools)] // UI state flags are independent boolean conditions
pub struct App {
    pub input: String,
    pub cursor_position: usize,
    pub results: Vec<SearchResult>,
    pub selected: usize,
    pub selected_action: usize,
    pub list_state: ListState,
    pub active_plugin: Option<(String, String)>,
    pub placeholder: String,
    pub busy: bool,
    pub status_message: Option<String>,
    pub should_quit: bool,
    pub pending_type_text: Option<String>,
    pub ambient_items_by_plugin: HashMap<String, Vec<AmbientItem>>,
    pub selected_ambient: usize,
    pub view_mode: ViewMode,
    pub show_preview: bool,
    pub preview_scroll: usize,
    /// Input mode: Realtime or Submit
    pub input_mode: InputMode,
    /// Plugin context for multi-step flows (edit mode, etc.)
    pub plugin_context: Option<String>,
    /// App ID of the item being executed
    pub pending_app_id: Option<String>,
    /// App name for window picker display
    pub pending_app_name: Option<String>,
    pub compositor: Compositor,
    /// Plugin actions toolbar (Ctrl+1-6)
    pub plugin_actions: Vec<PluginAction>,
    pub navigation_depth: u32,
    pub pending_back: bool,
    pub pending_confirm: Option<(String, String)>,
    pub running_app_ids: std::collections::HashSet<String>,
    pub plugin_statuses: HashMap<String, PluginStatus>,
    pub selected_preview_action: usize,
}

impl App {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            input: String::new(),
            cursor_position: 0,
            results: Vec::new(),
            selected: 0,
            selected_action: 0,
            list_state,
            active_plugin: None,
            placeholder: "Search...".to_string(),
            busy: false,
            status_message: None,
            should_quit: false,
            pending_type_text: None,
            ambient_items_by_plugin: HashMap::new(),
            selected_ambient: 0,
            view_mode: ViewMode::default(),
            show_preview: false,
            preview_scroll: 0,
            input_mode: InputMode::Realtime,
            plugin_context: None,
            pending_app_id: None,
            pending_app_name: None,
            compositor: Compositor::detect(),
            plugin_actions: Vec::new(),
            navigation_depth: 0,
            pending_back: false,
            pending_confirm: None,
            running_app_ids: std::collections::HashSet::new(),
            plugin_statuses: HashMap::new(),
            selected_preview_action: 0,
        }
    }

    /// Refresh the list of running app IDs from the compositor
    pub fn refresh_running_apps(&mut self) {
        self.running_app_ids = self.compositor.get_running_app_ids();
    }

    pub fn get_selected_preview(&self) -> Option<&PreviewData> {
        self.results.get(self.selected)?.preview.as_ref()
    }

    pub fn get_all_ambient_items(&self) -> Vec<&AmbientItem> {
        self.ambient_items_by_plugin
            .values()
            .flat_map(|items| items.iter())
            .collect()
    }

    pub fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.cursor_position.saturating_sub(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_left);
    }

    pub fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.cursor_position.saturating_add(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_right);
    }

    pub fn enter_char(&mut self, c: char) {
        self.input.insert(self.cursor_position, c);
        self.move_cursor_right();
    }

    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            let current_index = self.cursor_position;
            let from_left_to_current_index = current_index - 1;
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input.chars().skip(current_index);
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    pub fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.len())
    }

    pub fn select_next(&mut self) {
        if !self.results.is_empty() {
            self.selected = (self.selected + 1) % self.results.len();
            self.selected_action = 0;
            self.selected_preview_action = 0;
            self.list_state.select(Some(self.selected));
            self.preview_scroll = 0;
        }
    }

    pub fn select_previous(&mut self) {
        if !self.results.is_empty() {
            self.selected = if self.selected == 0 {
                self.results.len() - 1
            } else {
                self.selected - 1
            };
            self.selected_action = 0;
            self.selected_preview_action = 0;
            self.list_state.select(Some(self.selected));
            self.preview_scroll = 0;
        }
    }

    pub fn cycle_action(&mut self) {
        if !self.results.is_empty() {
            let result = &self.results[self.selected];
            let action_count = result.actions.len() + 1; // +1 for default action
            self.selected_action = (self.selected_action + 1) % action_count;
        }
    }

    pub fn cycle_action_prev(&mut self) {
        if !self.results.is_empty() {
            let result = &self.results[self.selected];
            let action_count = result.actions.len() + 1; // +1 for default action
            self.selected_action = if self.selected_action == 0 {
                action_count - 1
            } else {
                self.selected_action - 1
            };
        }
    }

    pub fn get_selected_action(&self) -> Option<String> {
        if self.selected_action == 0 {
            None // Default action
        } else if let Some(result) = self.results.get(self.selected) {
            result
                .actions
                .get(self.selected_action - 1)
                .map(|a| a.id.clone())
        } else {
            None
        }
    }

    /// Cycle to the next preview action
    pub fn cycle_preview_action(&mut self) {
        if let Some(preview) = self.get_selected_preview() {
            let action_count = preview.actions.len();
            if action_count > 0 {
                self.selected_preview_action = (self.selected_preview_action + 1) % action_count;
            }
        }
    }

    /// Cycle to the previous preview action
    pub fn cycle_preview_action_prev(&mut self) {
        if let Some(preview) = self.get_selected_preview() {
            let action_count = preview.actions.len();
            if action_count > 0 {
                self.selected_preview_action = if self.selected_preview_action == 0 {
                    action_count - 1
                } else {
                    self.selected_preview_action - 1
                };
            }
        }
    }

    /// Get the ID of the selected preview action
    pub fn get_selected_preview_action_id(&self) -> Option<String> {
        self.get_selected_preview()
            .and_then(|preview| preview.actions.get(self.selected_preview_action))
            .map(|action| action.id.clone())
    }

    /// Handle a core update from the daemon
    // 1:1 CoreUpdate variant mapping - each arm is minimal, splitting would fragment the exhaustive match
    #[allow(clippy::too_many_lines)]
    pub fn handle_update(&mut self, update: CoreUpdate) {
        match update {
            CoreUpdate::Results {
                results,
                placeholder,
                clear_input,
                input_mode,
                context,
                navigate_forward,
                ..
            } => self.handle_core_results(
                results,
                placeholder,
                clear_input,
                input_mode,
                context,
                navigate_forward,
            ),
            CoreUpdate::ResultsUpdate { patches } => self.apply_result_patches(patches),
            CoreUpdate::PluginActivated { id, name, .. } => {
                self.active_plugin = Some((id, name));
                self.input.clear();
                self.cursor_position = 0;
            }
            CoreUpdate::PluginDeactivated => {
                self.active_plugin = None;
                self.input.clear();
                self.cursor_position = 0;
                self.input_mode = InputMode::Realtime;
                self.plugin_context = None;
                self.plugin_actions.clear();
                self.navigation_depth = 0;
            }
            CoreUpdate::Placeholder { placeholder } => {
                self.placeholder = placeholder;
            }
            CoreUpdate::Busy { busy } => {
                self.busy = busy;
            }
            CoreUpdate::Error { message } => {
                // Show error in modal
                let error_state = ErrorState::new(
                    "Plugin Error".to_string(),
                    message,
                    None,
                    self.active_plugin.as_ref().map(|(id, _)| id.clone()),
                );
                self.view_mode = ViewMode::Error(error_state);
            }
            CoreUpdate::Close => {
                if matches!(self.view_mode, ViewMode::WindowPicker(_)) {
                    return;
                }
                if self.active_plugin.is_some() {
                    self.active_plugin = None;
                } else {
                    self.should_quit = true;
                }
            }
            CoreUpdate::Toggle => {
                // TUI doesn't support FAB/minimize, treat as close
                if self.active_plugin.is_some() {
                    self.active_plugin = None;
                } else {
                    self.should_quit = true;
                }
            }
            CoreUpdate::Execute { action } => {
                self.handle_execute_action(&action);
            }
            CoreUpdate::AmbientUpdate { plugin_id, items } => {
                if items.is_empty() {
                    self.ambient_items_by_plugin.remove(&plugin_id);
                } else {
                    self.ambient_items_by_plugin.insert(plugin_id, items);
                }
                let total = self.get_all_ambient_items().len();
                if self.selected_ambient >= total {
                    self.selected_ambient = 0;
                }
            }
            CoreUpdate::PluginStatusUpdate { plugin_id, status } => {
                self.plugin_statuses.insert(plugin_id, status);
            }
            CoreUpdate::InputModeChanged { mode } => {
                self.input_mode = mode;
            }
            CoreUpdate::ContextChanged { context } => {
                self.plugin_context = context;
            }
            CoreUpdate::ClearInput => {
                self.input.clear();
                self.cursor_position = 0;
            }
            CoreUpdate::Card { card, .. } => {
                self.busy = false;
                self.view_mode = ViewMode::Card(CardState::new(card));
            }
            CoreUpdate::Form { form } => {
                self.busy = false;
                let context = form.context.clone();
                self.view_mode = ViewMode::Form(FormState::new(form, context));
            }
            CoreUpdate::GridBrowser { browser } => {
                self.busy = false;
                self.view_mode = ViewMode::GridBrowser(GridBrowserState::new(browser));
            }
            CoreUpdate::ImageBrowser { browser } => {
                self.busy = false;
                self.view_mode = ViewMode::ImageBrowser(ImageBrowserState::new(browser));
            }
            CoreUpdate::Prompt { prompt } => {
                self.input = prompt;
                self.cursor_position = self.input.len();
            }
            CoreUpdate::PluginActionsUpdate { actions } => {
                self.plugin_actions = actions;
            }
            CoreUpdate::NavigationDepthChanged { depth } => {
                self.navigation_depth = depth;
                self.pending_back = false; // Clear pending state on explicit depth set
            }
            CoreUpdate::NavigateForward => {
                if !self.pending_back {
                    self.navigation_depth += 1;
                }
                self.pending_back = false;
            }
            CoreUpdate::NavigateBack => {
                self.navigation_depth = self.navigation_depth.saturating_sub(1);
                self.pending_back = false;
            }
            // TUI doesn't handle these updates
            CoreUpdate::Show
            | CoreUpdate::FabUpdate { .. }
            | CoreUpdate::ConfigReloaded
            | CoreUpdate::PluginManagementChanged { .. }
            | CoreUpdate::IndexUpdate { .. }
            | CoreUpdate::ActivatePlugin { .. } => {}
        }
    }

    /// Handle `CoreUpdate::Results` - updates the results list and resets selection
    fn handle_core_results(
        &mut self,
        mut results: Vec<SearchResult>,
        placeholder: Option<String>,
        clear_input: Option<bool>,
        input_mode: Option<InputMode>,
        context: Option<String>,
        navigate_forward: Option<bool>,
    ) {
        if matches!(self.view_mode, ViewMode::Form(_)) {
            self.view_mode = ViewMode::Results;
        }

        if let Some(true) = navigate_forward {
            self.navigation_depth += 1;
            self.pending_back = false;
        } else if self.pending_back {
            self.navigation_depth = self.navigation_depth.saturating_sub(1);
            self.pending_back = false;
        }

        if let Some(p) = placeholder {
            self.placeholder = p;
        }
        if let Some(true) = clear_input {
            self.input.clear();
            self.cursor_position = 0;
        }
        if let Some(mode) = input_mode {
            self.input_mode = mode;
        }
        if context.is_some() {
            self.plugin_context = context;
        }

        results.sort_by_key(|r| i32::from(r.result_type != ResultType::PatternMatch));

        self.results = results;
        self.selected = 0;
        self.selected_action = 0;
        self.selected_preview_action = 0;
        self.list_state.select(Some(0));
        self.busy = false;
    }

    /// Apply patches to existing results (for live updates from plugins)
    fn apply_result_patches(&mut self, patches: Vec<ResultPatch>) {
        for patch in patches {
            if let Some(result) = self.results.iter_mut().find(|r| r.id == patch.id) {
                if let Some(name) = patch.name {
                    result.name = name;
                }
                if let Some(description) = patch.description {
                    result.description = Some(description);
                }
                if let Some(icon) = patch.icon {
                    result.icon = Some(icon);
                }
                if let Some(icon_type) = patch.icon_type {
                    result.icon_type = Some(icon_type);
                }
                if let Some(verb) = patch.verb {
                    result.verb = Some(verb);
                }
                // Update widget field from patch
                if let Some(widget) = patch.widget {
                    // For sliders, preserve existing min/max/step if not in patch
                    if let WidgetData::Slider {
                        value,
                        display_value,
                        ..
                    } = &widget
                    {
                        if let Some(WidgetData::Slider { min, max, step, .. }) = &result.widget {
                            result.widget = Some(WidgetData::Slider {
                                value: *value,
                                min: *min,
                                max: *max,
                                step: *step,
                                display_value: display_value.clone(),
                            });
                        } else {
                            result.widget = Some(widget);
                        }
                    } else {
                        result.widget = Some(widget);
                    }
                }
                if let Some(badges) = patch.badges {
                    result.badges = badges;
                }
                if let Some(chips) = patch.chips {
                    result.chips = chips;
                }
                if let Some(has_ocr) = patch.has_ocr {
                    result.has_ocr = has_ocr;
                }
            }
        }
        self.busy = false;
    }

    fn handle_execute_action(&mut self, action: &ExecuteAction) {
        match action {
            ExecuteAction::TypeText { text } => {
                self.pending_type_text = Some(text.clone());
                self.status_message = Some("TypeText pending...".to_string());
            }
            ExecuteAction::Launch { desktop_file } => {
                if let Some(true) = self.try_focus_existing_window() {
                    return;
                }
                self.status_message = Some(format!("Launching: {desktop_file}"));
                spawn_silent("gio", &["launch", desktop_file]);
                self.pending_app_id = None;
                self.pending_app_name = None;
            }
            ExecuteAction::OpenUrl { url } => spawn_silent("xdg-open", &[url]),
            ExecuteAction::Open { path } => spawn_silent("xdg-open", &[path]),
            ExecuteAction::Copy { text } => copy_to_clipboard(text),
            ExecuteAction::Notify { message } => spawn_silent("notify-send", &["Hamr", message]),
            ExecuteAction::PlaySound { sound } => {
                if let Some(path) = resolve_sound_path(sound) {
                    let path_str = path.to_string_lossy();
                    if let Err(e) = Command::new("paplay")
                        .arg(&path)
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                    {
                        warn!("Failed to play sound '{sound}' ({path_str}): {e}");
                    }
                } else {
                    warn!("Sound not found: {sound}");
                }
            }
        }
    }

    /// Try to focus an existing window for `pending_app_id`.
    /// Returns `Some(true)` if window was focused, `Some(false)` if launch should proceed,
    /// `None` if no `app_id` is pending. May switch to `WindowPicker` mode for multiple windows.
    fn try_focus_existing_window(&mut self) -> Option<bool> {
        let app_id = self.pending_app_id.as_ref()?;
        let matching = self.compositor.find_windows_by_app_id(app_id);

        match matching.len() {
            0 => Some(false),
            1 => {
                if self.compositor.focus_window(&matching[0].id) {
                    self.status_message = Some("Focused existing window".to_string());
                    self.pending_app_id = None;
                    self.pending_app_name = None;
                    Some(true)
                } else {
                    Some(false)
                }
            }
            _ => {
                let app_name = self
                    .pending_app_name
                    .clone()
                    .unwrap_or_else(|| app_id.clone());
                self.view_mode = ViewMode::WindowPicker(WindowPickerState::new(matching, app_name));
                self.status_message = Some("Select a window to focus".to_string());
                Some(true)
            }
        }
    }

    pub fn get_selected_ambient_with_plugin(&self) -> Option<(String, AmbientItem)> {
        let mut idx = 0;
        for (plugin_id, items) in &self.ambient_items_by_plugin {
            for item in items {
                if idx == self.selected_ambient {
                    return Some((plugin_id.clone(), item.clone()));
                }
                idx += 1;
            }
        }
        None
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve a sound name or path to an actual file path.
///
/// Supports:
/// - Direct paths: `/path/to/sound.oga` or `~/sounds/alert.wav`
/// - Freedesktop sound names: `alarm`, `bell`, `complete`, etc.
fn resolve_sound_path(sound: &str) -> Option<PathBuf> {
    // If it looks like a path, use it directly
    if sound.starts_with('/') {
        let path = PathBuf::from(sound);
        return path.exists().then_some(path);
    }

    if sound.starts_with('~')
        && let Some(home) = std::env::var_os("HOME")
    {
        let path = PathBuf::from(home).join(sound.strip_prefix("~/").unwrap_or(sound));
        return path.exists().then_some(path);
    }

    // Map sound events to candidate file names (in priority order)
    // Matches QML AudioService.qml soundEventMap
    let candidates: Vec<&str> = match sound {
        "alarm" => vec!["alarm", "alarm-clock-elapsed", "bell"],
        "timer" => vec![
            "timer",
            "alarm-clock-elapsed",
            "completion-success",
            "complete",
        ],
        "complete" => vec![
            "complete",
            "completion-success",
            "outcome-success",
            "message",
            "dialog-information",
        ],
        "notification" => vec![
            "notification",
            "message-new-instant",
            "message-highlight",
            "message",
        ],
        "error" => vec![
            "error",
            "dialog-error",
            "completion-fail",
            "outcome-failure",
            "bell",
        ],
        "warning" => vec!["warning", "dialog-warning", "dialog-warning-auth", "bell"],
        "info" => vec!["dialog-information", "message"],
        "question" => vec!["dialog-question", "dialog-information"],
        "message" => vec!["message", "message-new-instant"],
        "bell" => vec!["bell", "bell-window-system"],
        "trash" => vec!["trash-empty"],
        "login" => vec!["service-login"],
        "logout" => vec!["service-logout"],
        other => vec![other],
    };

    // Search in standard sound directories
    let search_dirs = get_sound_search_dirs();
    let extensions = ["oga", "ogg", "wav", "mp3", "flac"];

    // Try each candidate name in each directory
    for dir in &search_dirs {
        for name in &candidates {
            for ext in &extensions {
                let path = dir.join(format!("{name}.{ext}"));
                if path.exists() {
                    return Some(path);
                }
            }
        }
    }

    None
}

/// Get directories to search for sound theme files
/// Prioritizes: user sounds > ocean theme > freedesktop theme
fn get_sound_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // User local sounds first (hamr config dir and XDG sounds)
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        dirs.push(home.join(".config/hamr/sounds"));
        dirs.push(home.join(".local/share/sounds"));
    }

    // System sound themes: ocean first (modern), then freedesktop (fallback)
    dirs.push(PathBuf::from("/usr/share/sounds/ocean/stereo"));
    dirs.push(PathBuf::from("/usr/share/sounds/freedesktop/stereo"));

    // XDG_DATA_DIRS for additional locations
    if let Ok(xdg_dirs) = std::env::var("XDG_DATA_DIRS") {
        for dir in xdg_dirs.split(':') {
            let base = PathBuf::from(dir).join("sounds");
            dirs.push(base.join("ocean/stereo"));
            dirs.push(base.join("freedesktop/stereo"));
        }
    }

    dirs
}

#[cfg(test)]
mod sound_tests {
    use super::*;

    #[test]
    fn test_resolve_sound_path_direct_absolute_path() {
        // Non-existent absolute path returns None
        let result = resolve_sound_path("/nonexistent/path/to/sound.oga");
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_sound_path_home_expansion() {
        // Non-existent home path returns None
        let result = resolve_sound_path("~/nonexistent/sound.oga");
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_sound_path_alarm_finds_file() {
        // "alarm" should find alarm-clock-elapsed in ocean or freedesktop theme
        let result = resolve_sound_path("alarm");
        // This test will pass if ocean or freedesktop sounds are installed
        if std::path::Path::new("/usr/share/sounds/ocean/stereo").exists()
            || std::path::Path::new("/usr/share/sounds/freedesktop/stereo").exists()
        {
            assert!(result.is_some(), "Expected to find alarm sound");
            let path = result.unwrap();
            assert!(path.exists(), "Sound file should exist");
            // Should be alarm or alarm-clock-elapsed
            let filename = path.file_stem().unwrap().to_str().unwrap();
            assert!(
                filename == "alarm" || filename == "alarm-clock-elapsed",
                "Expected alarm or alarm-clock-elapsed, got {filename}"
            );
        }
    }

    #[test]
    fn test_resolve_sound_path_unknown_sound() {
        // Unknown sound name that doesn't exist anywhere
        let result = resolve_sound_path("nonexistent_sound_xyz_123");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_sound_search_dirs_includes_system_paths() {
        let dirs = get_sound_search_dirs();

        // Should include ocean and freedesktop system paths
        assert!(
            dirs.iter()
                .any(|d| d.to_str().unwrap().contains("ocean/stereo")),
            "Should include ocean theme path"
        );
        assert!(
            dirs.iter()
                .any(|d| d.to_str().unwrap().contains("freedesktop/stereo")),
            "Should include freedesktop theme path"
        );
    }

    #[test]
    fn test_get_sound_search_dirs_ocean_before_freedesktop() {
        let dirs = get_sound_search_dirs();

        let ocean_pos = dirs
            .iter()
            .position(|d| d.to_str().unwrap().contains("/usr/share/sounds/ocean"));
        let freedesktop_pos = dirs.iter().position(|d| {
            d.to_str()
                .unwrap()
                .contains("/usr/share/sounds/freedesktop")
        });

        if let (Some(ocean), Some(freedesktop)) = (ocean_pos, freedesktop_pos) {
            assert!(
                ocean < freedesktop,
                "Ocean theme should be searched before freedesktop"
            );
        }
    }

    #[test]
    fn test_sound_event_candidates() {
        // Test that sound events map to expected candidates
        // This is a compile-time check that the match arms are correct

        // alarm should try alarm first, then alarm-clock-elapsed
        // Just verify it doesn't panic - actual file existence depends on system
        let _ = resolve_sound_path("alarm");

        // timer should try timer, alarm-clock-elapsed, etc.
        let _ = resolve_sound_path("timer");
        let _ = resolve_sound_path("complete");
        let _ = resolve_sound_path("notification");
        let _ = resolve_sound_path("error");
        let _ = resolve_sound_path("warning");
    }
}
