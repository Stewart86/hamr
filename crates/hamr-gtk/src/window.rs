//! Launcher window with layer-shell support
#![allow(clippy::cast_possible_truncation)]

use crate::click_catcher::ClickCatcher;
use crate::compositor::{Compositor, Window as CompositorWindow};
use crate::config::{ClickOutsideAction, ConfigWatcher, Theme};
use crate::fab_window::FabWindow;
use crate::preview_window::PreviewWindow;
use crate::rpc::RpcHandle;
use crate::state::{LauncherVisibility, SessionState, StateManager};
use crate::widgets::AmbientItemWithPlugin;
use crate::widgets::design::preview_panel as preview_design;
use crate::widgets::form_view::FormView;
use crate::widgets::{
    self, ActionBar, ActionBarAction, ActionBarMode, KeybindingMap, PreviewPanel, ResultCard,
    ResultView,
};
use gtk4::prelude::*;
use gtk4::{cairo, gdk, gio, glib, graphene};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use hamr_rpc::{
    CardData, CoreEvent, CoreUpdate, FormData, PluginAction, PluginStatus, ResultType, SearchResult,
};
use hamr_types::{AmbientItem, DisplayHint, WidgetData};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::widgets::design::search_bar as design;

const DEFAULT_PLACEHOLDER: &str = "It's hamr time!";

fn is_daemon_disconnect_error(message: &str) -> bool {
    message.starts_with("Failed to connect to daemon")
        || message.starts_with("Failed to register")
        || message.starts_with("Daemon disconnected")
}

/// Get the desktop file ID of the default browser
fn get_default_browser_desktop_id() -> Option<String> {
    let default_app = gio::AppInfo::default_for_type("x-scheme-handler/https", false)
        .or_else(|| gio::AppInfo::default_for_type("x-scheme-handler/http", false))
        .or_else(|| gio::AppInfo::default_for_type("text/html", false))?;

    let id = default_app.id()?;
    let id_str = id.to_string();

    // Normalize: some IDs come as "firefox.desktop", others as "firefox"
    Some(id_str)
}

fn get_startup_wm_class(desktop_id: &str) -> Option<String> {
    let desktop_file = desktop_id.strip_suffix(".desktop").unwrap_or(desktop_id);

    let search_paths = [
        format!("/var/lib/flatpak/exports/share/applications/{desktop_file}.desktop"),
        format!("/usr/share/applications/{desktop_file}.desktop"),
        format!("/home/siwei/.local/share/applications/{desktop_file}.desktop"),
    ];

    for path in search_paths {
        if let Ok(content) = std::fs::read_to_string(&path) {
            for line in content.lines() {
                if let Some(wm_class) = line.strip_prefix("StartupWMClass=") {
                    return Some(wm_class.to_lowercase());
                }
            }
        }
    }
    None
}

fn window_is_default_browser(window: &CompositorWindow, browser_desktop_id: &str) -> bool {
    let window_class = window.app_id.to_lowercase();
    let browser_name = browser_desktop_id
        .strip_suffix(".desktop")
        .unwrap_or(browser_desktop_id)
        .to_lowercase();

    if window_class == browser_name
        || window_class.contains(&browser_name)
        || browser_name.contains(&window_class)
    {
        return true;
    }

    if let Some(wm_class) = get_startup_wm_class(browser_desktop_id)
        && (window_class == wm_class
            || window_class.contains(&wm_class)
            || wm_class.contains(&window_class))
    {
        return true;
    }

    false
}

/// View mode for result display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResultViewMode {
    #[default]
    List,
    Grid,
}

/// Application state shared across callbacks
#[allow(clippy::struct_excessive_bools)] // UI state flags are independent boolean conditions
struct AppState {
    results: Vec<SearchResult>,
    placeholder: String,
    active_plugin: Option<(String, String)>,
    input_mode: String,
    navigation_depth: usize,
    pending_back: bool,
    plugin_actions: Vec<PluginAction>,
    selected_action_index: i32,
    show_keybinding_map: bool,
    /// Track last query to detect changes and reset selection
    last_query: String,
    /// Plugin status updates (badges, chips) keyed by `plugin_id`
    plugin_statuses: HashMap<String, PluginStatus>,
    /// Ambient items keyed by `plugin_id`
    ambient_items: HashMap<String, Vec<AmbientItem>>,
    /// Primary app ID for focus-or-launch (`StartupWMClass` from `.desktop`)
    pending_app_id: Option<String>,
    /// Fallback app ID for focus-or-launch (desktop filename)
    pending_app_id_fallback: Option<String>,
    /// Pending app name for window picker display
    pending_app_name: Option<String>,
    /// Pending app icon for window picker display
    pending_app_icon: Option<String>,
    /// Window picker state: windows to choose from
    window_picker_windows: Vec<CompositorWindow>,
    /// Whether window picker is showing
    show_window_picker: bool,
    /// Desktop file to launch if user requests new instance from window picker
    pending_desktop_file: Option<String>,
    /// Current card being displayed (if any)
    current_card: Option<CardData>,
    /// Current result view mode (list or grid)
    view_mode: ResultViewMode,
    /// Plugin-requested display hint
    plugin_display_hint: Option<DisplayHint>,
    /// Current form context (for multi-step flows)
    form_context: Option<String>,
    /// Latest form data snapshot
    form_data: HashMap<String, String>,
    /// Current form configuration (when active)
    form_config: Option<FormData>,
    /// Current form widget (when active)
    form_view: Option<Rc<FormView>>,
    /// Whether compact mode is enabled
    compact_mode: bool,
    /// Pending text to type after window closes
    pending_type_text: Option<String>,
    /// Whether plugin management mode is active (showing only plugins via "/" prefix)
    plugin_management: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            results: Vec::new(),
            placeholder: DEFAULT_PLACEHOLDER.to_string(),
            active_plugin: None,
            input_mode: "realtime".to_string(),
            navigation_depth: 0,
            pending_back: false,
            plugin_actions: Vec::new(),
            selected_action_index: -1,
            show_keybinding_map: false,
            plugin_statuses: HashMap::new(),
            ambient_items: HashMap::new(),
            pending_app_id: None,
            pending_app_id_fallback: None,
            pending_app_name: None,
            pending_app_icon: None,
            window_picker_windows: Vec::new(),
            show_window_picker: false,
            pending_desktop_file: None,
            current_card: None,
            view_mode: ResultViewMode::default(),
            plugin_display_hint: None,
            form_context: None,
            form_data: HashMap::new(),
            form_config: None,
            form_view: None,
            compact_mode: false,
            pending_type_text: None,
            plugin_management: false,
            last_query: String::new(),
        }
    }
}

/// Context for handling `CoreUpdate` events
struct UpdateContext {
    state: Rc<RefCell<AppState>>,
    result_view: Rc<RefCell<ResultView>>,
    result_card: Rc<ResultCard>,
    preview_window: gtk4::Window,
    preview_revealer: gtk4::Revealer,
    preview_panel: Rc<PreviewPanel>,
    action_bar: Rc<ActionBar>,
    form_container: gtk4::Box,
    form_title: gtk4::Label,
    form_fields: gtk4::Box,
    form_submit: Rc<gtk4::Button>,
    form_cancel: Rc<gtk4::Button>,
    search_entry: gtk4::Entry,
    icon_container: gtk4::Box,
    icon_label: gtk4::Label,
    spinner: gtk4::Label,
    depth_indicator: gtk4::Box,
    caret_icon: gtk4::Label,
    action_bar_visible: Rc<RefCell<bool>>,
    content_container: gtk4::Box,
    layout_fixed: gtk4::Fixed,
    launcher_root: gtk4::Overlay,
    background: gtk4::DrawingArea,
    window: gtk4::Window,
    click_catcher_window: gtk4::Window,
    event_tx: async_channel::Sender<CoreEvent>,
    compositor: Rc<Compositor>,
    config_watcher: Rc<ConfigWatcher>,
    width_animation_source: Rc<RefCell<Option<glib::SourceId>>>,
    state_manager: Rc<StateManager>,
    drag_state: Rc<RefCell<DragState>>,
    fab_window: Rc<FabWindow>,
    error_dialog: Rc<widgets::ErrorDialog>,
}

pub struct LauncherWindow {
    window: gtk4::Window,
    search_entry: gtk4::Entry,
    icon_container: gtk4::Box,
    icon_label: gtk4::Label,
    spinner: gtk4::Label,
    depth_indicator: gtk4::Box,
    caret_button: gtk4::Box,
    caret_icon: gtk4::Label,
    action_bar_visible: Rc<RefCell<bool>>,
    result_view: Rc<RefCell<ResultView>>,
    result_card: Rc<ResultCard>,
    form_container: gtk4::Box,
    form_title: gtk4::Label,
    form_fields: gtk4::Box,
    form_submit: Rc<gtk4::Button>,
    form_cancel: Rc<gtk4::Button>,
    preview_window: PreviewWindow,
    action_bar: Rc<ActionBar>,
    confirm_dialog: Rc<widgets::ConfirmDialog>,
    error_dialog: Rc<widgets::ErrorDialog>,
    keybinding_map: Rc<KeybindingMap>,
    css_provider: gtk4::CssProvider,
    state: Rc<RefCell<AppState>>,
    rpc: Option<RpcHandle>,
    config_watcher: Rc<ConfigWatcher>,
    compositor: Rc<Compositor>,
    content_container: gtk4::Box,
    /// Fullscreen layout surface; launcher UI is positioned inside via Fixed.
    layout_fixed: gtk4::Fixed,
    /// Root widget inserted into `layout_fixed` (contains the entire launcher UI).
    launcher_root: gtk4::Overlay,
    /// Fullscreen background widget to force `layout_fixed` to cover the entire monitor.
    background: gtk4::DrawingArea,
    width_animation_source: Rc<RefCell<Option<glib::SourceId>>>,
    state_manager: Rc<StateManager>,
    /// Current drag state for window repositioning
    drag_state: Rc<RefCell<DragState>>,
    /// Click-catcher for detecting clicks outside the launcher
    click_catcher: ClickCatcher,
    /// Floating action button window (visible when minimized)
    fab_window: Rc<FabWindow>,
    /// Manager for pinned preview panels (sticky notes)
    pinned_panel_manager: Rc<widgets::pinned_panel::PinnedPanelManager>,
}

/// Tracks drag operation state
#[derive(Debug, Default)]
struct DragState {
    /// Whether a drag operation is in progress
    is_dragging: bool,
    /// Whether drag mode is active (after hold + move threshold)
    drag_mode_active: bool,
    /// Starting content position when drag began
    start_top: i32,
    start_left: i32,
    /// Current content position
    current_top: i32,
    current_left: i32,
    /// Launcher dimensions cached at drag start
    launcher_width: i32,
    launcher_height: i32,
    /// Screen dimensions for ratio calculations
    screen_width: i32,
    screen_height: i32,
    /// Current monitor connector name (for per-monitor position storage)
    monitor_name: Option<String>,
}

impl LauncherWindow {
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        clippy::too_many_lines
    )]
    pub fn new(app: &gtk4::Application) -> Self {
        // Force GL renderer to avoid Vulkan swapchain suboptimal warnings
        unsafe {
            std::env::set_var("GSK_RENDERER", "gl");
        }
        let config_watcher = Rc::new(ConfigWatcher::new());
        let theme = config_watcher.theme();

        let css_provider = gtk4::CssProvider::new();
        crate::styles::apply_css(&css_provider, &theme);

        gtk4::style_context_add_provider_for_display(
            &gdk::Display::default().unwrap(),
            &css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );

        let window = gtk4::Window::builder()
            .application(app)
            .title("Hamr")
            .decorated(false)
            .resizable(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::Exclusive);
        window.set_namespace(Some("hamr"));

        // Fullscreen surface: we position the launcher UI *inside* the window.
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        window.set_anchor(Edge::Bottom, true);
        window.set_exclusive_zone(-1);

        // Handle surface realization to prevent Vulkan swapchain suboptimal warnings
        window.connect_realize(|window| {
            // Queue a redraw to ensure proper surface initialization
            window.queue_draw();
        });

        // Load persisted state for positioning
        let state_manager = Rc::new(StateManager::new());
        let launcher_state = state_manager.launcher();

        // Get screen dimensions for position calculation
        // Try to use the last used monitor, fall back to first monitor
        let display = gdk::Display::default().expect("No display found");
        let monitor = state_manager
            .last_monitor()
            .and_then(|name| Self::get_monitor_by_name(&name))
            .or_else(|| display.monitors().item(0).and_downcast::<gdk::Monitor>())
            .expect("No monitor found");
        let geometry = monitor.geometry();
        let screen_width = geometry.width();
        let screen_height = geometry.height();

        // Set scale factor for image caches (HiDPI support)
        let scale_factor = f64::from(monitor.scale_factor());
        crate::thumbnail_cache::set_scale_factor(scale_factor);

        // Use config values if state has default position (user hasn't dragged yet)
        // Config values take precedence for initial position
        let x_ratio = if (launcher_state.x_ratio - 0.5).abs() < 0.001 {
            theme.config.appearance.launcher_x_ratio
        } else {
            launcher_state.x_ratio
        };
        let y_ratio = if (launcher_state.y_ratio - 0.1).abs() < 0.001 {
            theme.config.appearance.launcher_y_ratio
        } else {
            launcher_state.y_ratio
        };

        // Calculate initial UI position from ratios.
        // x_ratio is center-based, so offset by half the expected launcher width.
        let launcher_width = theme.config.sizes.search_width;
        let left_margin = ((x_ratio * f64::from(screen_width)) - (f64::from(launcher_width) / 2.0))
            .floor() as i32;
        let top_margin = (y_ratio * f64::from(screen_height)).floor() as i32;

        // Clamp to keep drag handle accessible (same logic as before).
        let min_left = -(launcher_width - 60);
        let max_left = screen_width - launcher_width;
        let min_top = 0;
        let max_top = screen_height - 60;

        let initial_left = left_margin.max(min_left).min(max_left);
        let initial_top = top_margin.max(min_top).min(max_top);

        window.set_margin(Edge::Left, 0);
        window.set_margin(Edge::Top, 0);

        let drag_state = Rc::new(RefCell::new(DragState {
            screen_width,
            screen_height,
            current_left: initial_left,
            current_top: initial_top,
            ..Default::default()
        }));

        // Initialize view mode from config
        let initial_view_mode = if theme.config.appearance.default_result_view == "grid" {
            ResultViewMode::Grid
        } else {
            ResultViewMode::List
        };

        let state = Rc::new(RefCell::new(AppState {
            view_mode: initial_view_mode,
            compact_mode: state_manager.compact_mode(),
            ..Default::default()
        }));

        let (
            content,
            search_entry,
            icon_container,
            icon_label,
            spinner,
            depth_indicator,
            caret_button,
            caret_icon,
            content_container,
            result_view,
            result_card,
            action_bar,
            form_container,
            form_title,
            form_fields,
            form_submit,
            form_cancel,
            keybinding_map,
        ) = Self::build_content(&theme, &state);

        let layout_fixed = gtk4::Fixed::new();
        layout_fixed.set_hexpand(true);
        layout_fixed.set_vexpand(true);
        window.set_child(Some(&layout_fixed));

        // Force the fixed container to span the entire monitor. Without this, GtkFixed will size
        // itself to the union of its children, and "click-away" only works over that region.
        let background = gtk4::DrawingArea::new();
        background.set_hexpand(true);
        background.set_vexpand(true);
        background.set_size_request(screen_width, screen_height);
        background.set_can_target(true);
        layout_fixed.put(&background, 0.0, 0.0);

        let (initial_left, initial_top) = {
            let ds = drag_state.borrow();
            (ds.current_left, ds.current_top)
        };
        layout_fixed.put(&content, f64::from(initial_left), f64::from(initial_top));

        let rpc = Some(RpcHandle::connect());

        let result_view = Rc::new(RefCell::new(result_view));
        let result_card = Rc::new(result_card);
        let action_bar = Rc::new(action_bar);

        let form_submit = Rc::new(form_submit);
        let form_cancel = Rc::new(form_cancel);

        // Create separate preview window
        let preview_window = PreviewWindow::new(app, &theme);
        preview_window.set_monitor(&monitor);

        // Configure result view
        {
            let mut view = result_view.borrow_mut();
            view.set_max_height(theme.config.sizes.max_results_height);
            view.set_grid_columns(theme.config.appearance.grid.columns);
            view.set_grid_spacing(theme.config.appearance.grid.spacing as u32);
        }
        result_card.set_max_height(theme.config.sizes.max_results_height);

        let compositor = Rc::new(Compositor::detect());
        let width_animation_source = Rc::new(RefCell::new(None));
        let action_bar_visible = Rc::new(RefCell::new(false));

        // Create click-catcher for detecting clicks outside the launcher
        let click_catcher = ClickCatcher::new(app);

        // Create FAB window (visible when launcher is minimized)
        let fab_window = Rc::new(FabWindow::new(app, &theme, state_manager.clone()));
        fab_window.set_monitor(&monitor);

        // Create pinned panel manager
        let pinned_panel_manager = Rc::new(widgets::pinned_panel::PinnedPanelManager::new(
            state_manager.clone(),
        ));

        // Create confirmation dialog (separate layer-shell window)
        let confirm_dialog = Rc::new(widgets::ConfirmDialog::new(app, &theme));
        confirm_dialog.set_monitor(&monitor);

        // Create error dialog (separate layer-shell window)
        let error_dialog = Rc::new(widgets::ErrorDialog::new(app, &theme));
        error_dialog.set_monitor(&monitor);

        let mut launcher = Self {
            window,
            search_entry,
            icon_container,
            icon_label,
            spinner,
            depth_indicator,
            caret_button,
            caret_icon,
            action_bar_visible,
            result_view,
            result_card,
            form_container,
            form_title,
            form_fields,
            form_submit,
            form_cancel,
            preview_window,
            action_bar,
            confirm_dialog,
            error_dialog,
            keybinding_map,
            css_provider,
            state,
            rpc,
            config_watcher: config_watcher.clone(),
            compositor,
            content_container,
            layout_fixed,
            launcher_root: content,
            background,
            width_animation_source,
            state_manager: state_manager.clone(),
            drag_state,
            click_catcher,
            fab_window,
            pinned_panel_manager,
        };

        launcher.setup_key_handlers();
        launcher.setup_focus();
        launcher.setup_search_handlers();
        launcher.setup_result_view_handlers();
        launcher.setup_result_card_handlers();
        launcher.setup_form_handlers();
        launcher.setup_preview_panel_handlers();
        launcher.setup_action_bar_handlers();
        launcher.setup_caret_drag();
        launcher.setup_click_catcher();
        launcher.setup_fab_window();
        launcher.setup_update_polling();

        launcher
            .action_bar
            .set_compact_mode(state_manager.compact_mode());
        launcher.action_bar.set_actions_visible(false);

        // Load initial prefix hints from hamr-core config
        launcher.update_keybinding_prefixes();

        let css_provider_clone = launcher.css_provider.clone();
        let result_view_clone = launcher.result_view.clone();
        let result_card_clone = launcher.result_card.clone();
        let action_bar = launcher.action_bar.clone();
        let state_for_config = launcher.state.clone();
        let content_container_clone = launcher.content_container.clone();
        let width_animation_source_clone = launcher.width_animation_source.clone();
        let keybinding_map_clone = launcher.keybinding_map.clone();
        let fab_window_clone = launcher.fab_window.clone();
        let confirm_dialog_clone = launcher.confirm_dialog.clone();
        let error_dialog_clone = launcher.error_dialog.clone();
        let preview_window_config = launcher.preview_window.window.clone();
        let preview_revealer_config = launcher.preview_window.revealer.clone();
        let launcher_root_config = launcher.launcher_root.clone();
        let drag_state_config = launcher.drag_state.clone();
        config_watcher.set_on_change(move |theme| {
            info!("Config changed, updating styles");
            crate::styles::apply_css(&css_provider_clone, theme);
            let mut view = result_view_clone.borrow_mut();
            view.set_max_height(theme.config.sizes.max_results_height);
            view.set_grid_columns(theme.config.appearance.grid.columns);
            view.set_grid_spacing(theme.config.appearance.grid.spacing as u32);
            result_card_clone.set_max_height(theme.config.sizes.max_results_height);

            let compact_mode = state_for_config.borrow().compact_mode;
            action_bar.set_compact_mode(compact_mode);

            // Animate width to new search_width value, repositioning preview during animation
            let preview_window_anim = preview_window_config.clone();
            let preview_revealer_anim = preview_revealer_config.clone();
            let launcher_root_anim = launcher_root_config.clone();
            let drag_state_anim = drag_state_config.clone();
            Self::animate_width_with_callback(
                &content_container_clone,
                &width_animation_source_clone,
                theme.config.sizes.search_width,
                Some(move |new_width| {
                    let ds = drag_state_anim.borrow();
                    Self::reposition_preview(
                        &preview_window_anim,
                        &preview_revealer_anim,
                        ds.current_left,
                        ds.current_top,
                        ds.screen_width,
                        new_width,
                        launcher_root_anim.width(),
                    );
                }),
            );

            // Update keybinding map prefixes from hamr-core config
            if let Some(hints) = Self::load_action_bar_hints() {
                keybinding_map_clone.set_prefixes(&hints);
            }

            // Update FAB window theme
            fab_window_clone.update_theme(theme);

            // Update confirm dialog theme
            confirm_dialog_clone.update_theme(theme);

            // Update error dialog theme
            error_dialog_clone.update_theme(theme);
        });
        config_watcher.start_watching();

        launcher
    }

    // Builder pattern returns all widgets needed by caller; struct would add boilerplate
    // Layout construction - sequential GTK widget creation with nested containers
    #[allow(clippy::type_complexity, clippy::too_many_lines)]
    fn build_content(
        theme: &Theme,
        state: &Rc<RefCell<AppState>>,
    ) -> (
        gtk4::Overlay,
        gtk4::Entry,
        gtk4::Box,
        gtk4::Label,
        gtk4::Label, // spinner (material icon with animation)
        gtk4::Box,
        gtk4::Box,
        gtk4::Label,
        gtk4::Box,
        ResultView,
        ResultCard,
        ActionBar,
        gtk4::Box,
        gtk4::Label,
        gtk4::Box,
        gtk4::Button,
        gtk4::Button,
        Rc<KeybindingMap>,
    ) {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .css_classes(["launcher-container"])
            .build();

        let search_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(design::ROW_SPACING)
            .margin_start(design::ROW_MARGIN)
            .margin_end(design::ROW_MARGIN)
            .margin_top(6)
            .margin_bottom(6)
            .valign(gtk4::Align::Center)
            .build();

        let icon_container = gtk4::Box::builder()
            .valign(gtk4::Align::Center)
            .halign(gtk4::Align::Center)
            .css_classes(["icon-container"])
            .build();

        let icon_label = gtk4::Label::builder()
            .label("gavel")
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .css_classes(["material-icon"])
            .build();
        icon_container.append(&icon_label);

        let spinner = gtk4::Label::builder()
            .label("progress_activity")
            .visible(false)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .css_classes(["material-icon", "search-spinner"])
            .build();
        icon_container.append(&spinner);

        let search_input_container = gtk4::Box::builder()
            .hexpand(true)
            .valign(gtk4::Align::Center)
            .css_classes(["search-input-container"])
            .build();

        let depth_indicator = gtk4::Box::builder()
            .spacing(4)
            .valign(gtk4::Align::Center)
            .halign(gtk4::Align::Start)
            .margin_start(4)
            .margin_end(8)
            .visible(false)
            .build();

        let placeholder = state.borrow().placeholder.clone();
        let search_entry = gtk4::Entry::builder()
            .placeholder_text(&placeholder)
            .hexpand(true)
            .css_classes(["launcher-search"])
            .build();

        search_input_container.append(&depth_indicator);
        search_input_container.append(&search_entry);

        let caret_button = gtk4::Box::builder()
            .css_classes(["caret-toggle"])
            .valign(gtk4::Align::Center)
            .halign(gtk4::Align::Center)
            .build();
        let caret_icon = gtk4::Label::builder()
            .label("chevron_left")
            .valign(gtk4::Align::Center)
            .halign(gtk4::Align::Center)
            .css_classes(["material-icon", "caret-icon"])
            .build();
        caret_button.append(&caret_icon);

        let action_bar = ActionBar::new();
        action_bar.actions_widget().set_valign(gtk4::Align::Center);
        action_bar.actions_widget().set_halign(gtk4::Align::End);

        // Use a horizontal Box instead of Overlay so margin_end works correctly
        let search_area = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .valign(gtk4::Align::Center)
            .hexpand(true)
            .build();
        search_area.append(&search_input_container);
        search_area.append(action_bar.actions_widget());

        search_row.append(&icon_container);
        search_row.append(&search_area);
        search_row.append(&caret_button);
        container.append(&search_row);

        let ambient_row = action_bar.ambient_widget();
        ambient_row.set_margin_start(design::ROW_MARGIN);
        ambient_row.set_margin_end(design::ROW_MARGIN);
        ambient_row.set_margin_bottom(6);
        container.append(ambient_row);

        // Create unified result view (starts in List mode by default)
        let view_mode = state.borrow().view_mode;
        let result_view = ResultView::new(view_mode, theme);

        container.append(result_view.widget());

        let result_card = ResultCard::new();
        result_card.widget().set_visible(false);
        container.append(result_card.widget());

        let form_container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .css_classes(["form-container"])
            .build();
        form_container.set_visible(false);

        let form_title = gtk4::Label::builder()
            .label("")
            .halign(gtk4::Align::Start)
            .css_classes(["form-title"])
            .build();
        form_container.append(&form_title);

        let form_fields = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .build();
        form_container.append(&form_fields);

        let form_actions = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk4::Align::End)
            .css_classes(["form-actions"])
            .build();

        let form_cancel = gtk4::Button::builder()
            .label("Cancel")
            .css_classes(["cancel-button"])
            .build();
        let form_submit = gtk4::Button::builder()
            .label("Submit")
            .css_classes(["submit-button"])
            .build();
        form_actions.append(&form_cancel);
        form_actions.append(&form_submit);
        form_container.append(&form_actions);

        container.append(&form_container);

        let keybinding_map = Rc::new(KeybindingMap::new());

        keybinding_map.widget().set_halign(gtk4::Align::Center);
        keybinding_map.widget().set_valign(gtk4::Align::Center);
        keybinding_map.widget().set_visible(false);

        let keybinding_overlay = gtk4::Overlay::new();
        keybinding_overlay.set_child(Some(&container));
        keybinding_overlay.add_overlay(keybinding_map.widget());

        (
            keybinding_overlay,
            search_entry,
            icon_container,
            icon_label,
            spinner,
            depth_indicator,
            caret_button,
            caret_icon,
            container,
            result_view,
            result_card,
            action_bar,
            form_container,
            form_title,
            form_fields,
            form_submit,
            form_cancel,
            keybinding_map,
        )
    }

    /// Setup action bar event handlers
    // Event handler setup - multiple callback connections sharing state via Rc<RefCell>
    #[allow(clippy::too_many_lines)]
    fn setup_action_bar_handlers(&self) {
        let Some(rpc) = &self.rpc else { return };

        let event_tx = rpc.event_sender();
        self.action_bar.connect_home(move || {
            info!("Home clicked - closing plugin");
            if let Err(e) = event_tx.send_blocking(CoreEvent::ClosePlugin) {
                error!("Failed to send ClosePlugin: {}", e);
            }
        });

        let state = self.state.clone();
        let action_bar = self.action_bar.clone();
        let state_manager = self.state_manager.clone();
        let result_view = self.result_view.clone();
        let search_entry = self.search_entry.clone();
        self.action_bar.connect_compact_toggle(move |enabled| {
            info!("Compact mode toggled: {}", enabled);
            state.borrow_mut().compact_mode = enabled;
            action_bar.set_compact_mode(enabled);
            state_manager.set_compact_mode(enabled);

            let empty_query = search_entry.text().trim().is_empty();
            let should_hide_results = enabled && empty_query;
            result_view
                .borrow()
                .widget()
                .set_visible(!should_hide_results);
        });

        // Grid view toggle - same as Ctrl+G
        let state = self.state.clone();
        let action_bar = self.action_bar.clone();
        let result_view = self.result_view.clone();
        let content_container = self.content_container.clone();
        let width_animation_source = self.width_animation_source.clone();
        let preview_window = self.preview_window.window.clone();
        let preview_revealer = self.preview_window.revealer.clone();
        let launcher_root = self.launcher_root.clone();
        let drag_state = self.drag_state.clone();
        let config_watcher = self.config_watcher.clone();
        self.action_bar.connect_grid_view(move |grid_view| {
            info!("Grid view toggled: {}", grid_view);
            action_bar.set_grid_view(grid_view);

            // Rebuild the menu to update the button label/icon
            action_bar.rebuild_launcher_actions();

            let new_mode = {
                let mut s = state.borrow_mut();
                s.view_mode = if grid_view {
                    ResultViewMode::Grid
                } else {
                    ResultViewMode::List
                };
                s.view_mode
            };

            // Switch view mode
            result_view.borrow_mut().set_mode(new_mode);

            // Animate width, repositioning preview during animation
            let theme = config_watcher.theme();
            let target_width = match new_mode {
                ResultViewMode::List => theme.config.sizes.search_width,
                ResultViewMode::Grid => theme.config.appearance.grid.calculate_width(),
            };
            let preview_window_anim = preview_window.clone();
            let preview_revealer_anim = preview_revealer.clone();
            let launcher_root_anim = launcher_root.clone();
            let drag_state_anim = drag_state.clone();
            Self::animate_width_with_callback(
                &content_container,
                &width_animation_source,
                target_width,
                Some(move |new_width| {
                    let ds = drag_state_anim.borrow();
                    Self::reposition_preview(
                        &preview_window_anim,
                        &preview_revealer_anim,
                        ds.current_left,
                        ds.current_top,
                        ds.screen_width,
                        new_width,
                        launcher_root_anim.width(),
                    );
                }),
            );
        });

        let event_tx = rpc.event_sender();
        let state = self.state.clone();
        self.action_bar.connect_back(move || {
            let mut s = state.borrow_mut();
            if s.navigation_depth > 0 {
                info!("Back clicked - navigating back");
                s.pending_back = true;
                drop(s);
                if let Err(e) = event_tx.send_blocking(CoreEvent::Back) {
                    error!("Failed to send Back: {}", e);
                }
            } else if s.active_plugin.is_some() {
                info!("Back clicked - closing plugin");
                drop(s);
                if let Err(e) = event_tx.send_blocking(CoreEvent::ClosePlugin) {
                    error!("Failed to send ClosePlugin: {}", e);
                }
            }
        });

        let event_tx = rpc.event_sender();
        self.action_bar
            .connect_action(move |action_id, _confirmed| {
                info!("Plugin action clicked: {}", action_id);
                if let Err(e) = event_tx.send_blocking(CoreEvent::PluginActionTriggered {
                    action_id: action_id.to_string(),
                }) {
                    error!("Failed to send PluginActionTriggered: {}", e);
                }
            });

        // Handle confirmation requests - show modal dialog
        let confirm_dialog = self.confirm_dialog.clone();
        let drag_state = self.drag_state.clone();
        let launcher_root = self.launcher_root.clone();
        self.action_bar
            .connect_confirm_request(move |action_id, message| {
                info!("Confirmation requested for action: {}", action_id);
                let ds = drag_state.borrow();
                let launcher_x = ds.current_left;
                let launcher_y = ds.current_top;
                let launcher_width = launcher_root.width();
                confirm_dialog.show(action_id, message, launcher_x, launcher_y, launcher_width);
            });

        // Handle confirmation dialog result
        let event_tx = rpc.event_sender();
        self.confirm_dialog
            .connect_result(move |confirmed, action_id| {
                if confirmed {
                    info!("Confirmed action: {}", action_id);
                    if let Err(e) = event_tx.send_blocking(CoreEvent::PluginActionTriggered {
                        action_id: action_id.to_string(),
                    }) {
                        error!("Failed to send PluginActionTriggered: {}", e);
                    }
                } else {
                    info!("Action cancelled: {}", action_id);
                }
            });

        let keybinding_widget = self.keybinding_map.widget().clone();
        let state = self.state.clone();
        self.action_bar.connect_help(move || {
            let mut s = state.borrow_mut();
            s.show_keybinding_map = !s.show_keybinding_map;
            keybinding_widget.set_visible(s.show_keybinding_map);
        });

        let event_tx = rpc.event_sender();
        self.action_bar
            .connect_ambient_action(move |plugin_id, item_id, action_id| {
                info!(
                    "Ambient action: {} on item {} (plugin {})",
                    action_id, item_id, plugin_id
                );
                if let Err(e) = event_tx.send_blocking(CoreEvent::AmbientAction {
                    plugin_id: plugin_id.to_string(),
                    item_id: item_id.to_string(),
                    action: Some(action_id.to_string()),
                }) {
                    error!("Failed to send AmbientAction: {}", e);
                }
            });

        // Use AmbientAction with "__dismiss__" to match QML behavior
        let event_tx = rpc.event_sender();
        self.action_bar
            .connect_ambient_dismiss(move |plugin_id, item_id| {
                info!("Ambient dismiss: item {} (plugin {})", item_id, plugin_id);
                if let Err(e) = event_tx.send_blocking(CoreEvent::AmbientAction {
                    plugin_id: plugin_id.to_string(),
                    item_id: item_id.to_string(),
                    action: Some("__dismiss__".to_string()),
                }) {
                    error!("Failed to send AmbientAction for dismiss: {}", e);
                }
            });

        // FAB ambient action callback (same logic as action bar)
        let event_tx = rpc.event_sender();
        self.fab_window
            .connect_ambient_action(move |plugin_id, item_id, action_id| {
                info!(
                    "FAB ambient action: {} on item {} (plugin {})",
                    action_id, item_id, plugin_id
                );
                if let Err(e) = event_tx.send_blocking(CoreEvent::AmbientAction {
                    plugin_id: plugin_id.to_string(),
                    item_id: item_id.to_string(),
                    action: Some(action_id.to_string()),
                }) {
                    error!("Failed to send AmbientAction from FAB: {}", e);
                }
            });

        // FAB ambient dismiss callback (same logic as action bar)
        let event_tx = rpc.event_sender();
        self.fab_window
            .connect_ambient_dismiss(move |plugin_id, item_id| {
                info!(
                    "FAB ambient dismiss: item {} (plugin {})",
                    item_id, plugin_id
                );
                if let Err(e) = event_tx.send_blocking(CoreEvent::AmbientAction {
                    plugin_id: plugin_id.to_string(),
                    item_id: item_id.to_string(),
                    action: Some("__dismiss__".to_string()),
                }) {
                    error!("Failed to send AmbientAction for FAB dismiss: {}", e);
                }
            });

        // Minimize button handler
        let window = self.window.clone();
        let click_catcher_window = self.click_catcher.window.clone();
        let preview_window = self.preview_window.window.clone();
        let preview_revealer = self.preview_window.revealer.clone();
        let fab_window = self.fab_window.clone();
        let state = self.state.clone();
        let search_entry = self.search_entry.clone();
        let state_manager = self.state_manager.clone();
        let event_tx = rpc.event_sender();
        self.action_bar.connect_minimize(move || {
            info!("Minimize button clicked");
            state_manager.set_has_used_minimize();
            let visibility_state = state_manager.visibility_state();
            let session = {
                let state_ref = state.borrow();
                SessionState {
                    query: search_entry.text().to_string(),
                    results: state_ref.results.clone(),
                    active_plugin: state_ref.active_plugin.clone(),
                }
            };
            visibility_state.save_session(session);
            visibility_state.minimize();
            window.set_visible(false);
            click_catcher_window.set_visible(false);
            preview_revealer.set_reveal_child(false);
            preview_window.set_visible(false);
            fab_window.show();

            if let Err(e) = event_tx.send_blocking(CoreEvent::LauncherClosed) {
                error!("Failed to send LauncherClosed: {}", e);
            }
        });
    }

    /// Setup caret button with dual functionality:
    /// - Quick click: toggles action bar visibility
    /// - Hold + drag: repositions the launcher window
    // GTK gesture setup - callbacks share local state via Rc<RefCell<>>
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::too_many_lines
    )]
    fn setup_caret_drag(&self) {
        let caret_button = self.caret_button.clone();
        let caret_icon = self.caret_icon.clone();
        let window = self.window.clone();
        let layout_fixed = self.layout_fixed.clone();
        let launcher_root = self.launcher_root.clone();
        let drag_state = self.drag_state.clone();
        let state_manager = self.state_manager.clone();
        let action_bar_visible = self.action_bar_visible.clone();
        let compositor = self.compositor.clone();

        // Single drag gesture handles both click and drag
        let drag_gesture = gtk4::GestureDrag::new();
        drag_gesture.set_button(gdk::BUTTON_PRIMARY);

        // Track press time to distinguish click vs hold
        let press_time: Rc<RefCell<Option<std::time::Instant>>> = Rc::new(RefCell::new(None));
        // Track if we've moved enough to be considered a drag
        let has_dragged: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));

        // Prefer reading pointer position from GDK in the window surface coordinate space.
        // This stays stable while we move the launcher widget inside the fullscreen surface.
        let pointer_device = gdk::Display::default()
            .and_then(|d| d.default_seat())
            .and_then(|seat| seat.pointer());
        let start_pointer_in_surface: Rc<RefCell<Option<(f64, f64)>>> = Rc::new(RefCell::new(None));

        // Track pointer position in the stable fullscreen coordinate space.
        // This mirrors the QML implementation and avoids jitter from using gesture offsets
        // while moving the widget under the cursor.
        let pointer_in_fixed: Rc<RefCell<Option<(f64, f64)>>> = Rc::new(RefCell::new(None));
        let pointer_in_fixed_motion = pointer_in_fixed.clone();
        let motion = gtk4::EventControllerMotion::new();
        motion.connect_motion(move |_, x, y| {
            *pointer_in_fixed_motion.borrow_mut() = Some((x, y));
        });
        layout_fixed.add_controller(motion);

        let drag_start_pointer: Rc<RefCell<Option<(f64, f64)>>> = Rc::new(RefCell::new(None));
        let drag_offset: Rc<RefCell<Option<(f64, f64)>>> = Rc::new(RefCell::new(None));

        let drag_state_begin = drag_state.clone();
        let window_begin = window.clone();
        let launcher_root_begin = launcher_root.clone();
        let compositor_begin = compositor.clone();
        let press_time_begin = press_time.clone();
        let has_dragged_begin = has_dragged.clone();
        let pointer_device_begin = pointer_device.clone();
        let start_pointer_in_surface_begin = start_pointer_in_surface.clone();
        let pointer_in_fixed_begin = pointer_in_fixed.clone();
        let drag_start_pointer_begin = drag_start_pointer.clone();
        let drag_offset_begin = drag_offset.clone();

        drag_gesture.connect_drag_begin(move |_, _, _| {
            *press_time_begin.borrow_mut() = Some(std::time::Instant::now());
            *has_dragged_begin.borrow_mut() = false;

            let mut state = drag_state_begin.borrow_mut();
            state.is_dragging = false;
            state.drag_mode_active = false;
            state.start_left = state.current_left;
            state.start_top = state.current_top;
            state.launcher_width = launcher_root_begin.width().max(200);
            state.launcher_height = launcher_root_begin.height().max(50);
            let monitor = window_begin
                .surface()
                .and_then(|surface| surface.display().monitor_at_surface(&surface))
                .or_else(|| window_begin.monitor())
                .or_else(|| Self::get_focused_monitor(&compositor_begin))
                .or_else(|| {
                    gdk::Display::default()
                        .and_then(|d| d.monitors().item(0).and_downcast::<gdk::Monitor>())
                });
            if let Some(monitor) = monitor {
                let geometry = monitor.geometry();
                state.screen_width = geometry.width();
                state.screen_height = geometry.height();
                state.monitor_name = monitor
                    .connector()
                    .map(|name| name.to_string())
                    .or_else(|| compositor_begin.get_focused_output());
            }

            let current_pointer = *pointer_in_fixed_begin.borrow();
            *drag_start_pointer_begin.borrow_mut() = current_pointer;
            *drag_offset_begin.borrow_mut() = current_pointer.map(|(x, y)| {
                (
                    x - f64::from(state.start_left),
                    y - f64::from(state.start_top),
                )
            });

            if let Some(device) = &pointer_device_begin
                && let Some(surface) = window_begin.surface()
                && let Some((x, y, _mask)) = surface.device_position(device)
            {
                *start_pointer_in_surface_begin.borrow_mut() = Some((x, y));
                *drag_offset_begin.borrow_mut() = Some((
                    x - f64::from(state.start_left),
                    y - f64::from(state.start_top),
                ));
            }
        });

        let drag_state_update = drag_state.clone();
        let window_update = window.clone();
        let layout_fixed_update = layout_fixed.clone();
        let launcher_root_update = launcher_root.clone();
        let caret_icon_update = caret_icon.clone();
        let caret_button_update = caret_button.clone();
        let press_time_update = press_time.clone();
        let has_dragged_update = has_dragged.clone();
        let pointer_device_update = pointer_device.clone();
        let start_pointer_in_surface_update = start_pointer_in_surface.clone();
        let pointer_in_fixed_update = pointer_in_fixed.clone();
        let drag_start_pointer_update = drag_start_pointer.clone();
        let drag_offset_update = drag_offset.clone();
        let preview_window_drag = self.preview_window.window.clone();
        let preview_revealer_drag = self.preview_window.revealer.clone();

        drag_gesture.connect_drag_update(move |_, offset_x, offset_y| {
            let mut state = drag_state_update.borrow_mut();

            let current_pointer = *pointer_in_fixed_update.borrow();
            let start_pointer = *drag_start_pointer_update.borrow();

            let surface_pointer = if let Some(device) = &pointer_device_update
                && let Some(surface) = window_update.surface()
            {
                surface.device_position(device).map(|(x, y, _)| (x, y))
            } else {
                None
            };
            let start_surface_pointer = *start_pointer_in_surface_update.borrow();

            let (movement_x, movement_y) = if let (Some((cx, cy)), Some((sx, sy))) =
                (surface_pointer, start_surface_pointer)
            {
                (cx - sx, cy - sy)
            } else if let (Some((cx, cy)), Some((sx, sy))) = (current_pointer, start_pointer) {
                (cx - sx, cy - sy)
            } else {
                (offset_x, offset_y)
            };

            // Check if held long enough (300ms) and moved enough (5px) to start dragging
            let dominated_movement = movement_x.abs().max(movement_y.abs());
            let held_long_enough = press_time_update
                .borrow()
                .map(|t| t.elapsed() >= Duration::from_millis(300))
                .unwrap_or(false);

            if !state.is_dragging && held_long_enough && dominated_movement > 5.0 {
                state.is_dragging = true;
                state.drag_mode_active = true;
                *has_dragged_update.borrow_mut() = true;
                caret_icon_update.set_label("drag_indicator");
                caret_button_update.add_css_class("drag-mode");
            }

            if !state.is_dragging {
                return;
            }

            let (new_left, new_top) = if let (Some((cx, cy)), Some((ox, oy))) =
                (surface_pointer, *drag_offset_update.borrow())
            {
                ((cx - ox) as i32, (cy - oy) as i32)
            } else if let (Some((cx, cy)), Some((ox, oy))) =
                (current_pointer, *drag_offset_update.borrow())
            {
                ((cx - ox) as i32, (cy - oy) as i32)
            } else {
                (
                    state.start_left + offset_x as i32,
                    state.start_top + offset_y as i32,
                )
            };

            let launcher_width = state.launcher_width;
            let launcher_height = state.launcher_height;

            // Clamp to ensure drag handle (on right side) remains accessible
            // min_left: allow left edge off-screen, but keep at least 60px of left side visible
            // max_left: keep right edge (where drag handle is) on screen
            let min_left = -(launcher_width - 60);
            let max_left = state.screen_width - launcher_width;
            let min_top = 0;
            let max_top = state.screen_height - launcher_height.min(60);

            let clamped_left = new_left.max(min_left).min(max_left);
            let clamped_top = new_top.max(min_top).min(max_top);

            state.current_left = clamped_left;
            state.current_top = clamped_top;

            layout_fixed_update.move_(
                &launcher_root_update,
                f64::from(clamped_left),
                f64::from(clamped_top),
            );

            // Update preview window position if revealed (on side with more space)
            if preview_revealer_drag.reveals_child() {
                // Use actual window width (changes with grid mode) instead of config value
                let launcher_width = launcher_root_update.width();
                let preview_width = preview_design::WIDTH;
                let gap = 8;

                let right_space = state.screen_width - (clamped_left + launcher_width);
                let left_space = clamped_left;

                let show_on_right = right_space >= left_space;
                let preview_x = if show_on_right {
                    clamped_left + launcher_width + gap
                } else {
                    (clamped_left - preview_width - gap).max(0)
                };

                preview_window_drag.set_margin(Edge::Left, preview_x);
                preview_window_drag.set_margin(Edge::Top, clamped_top);
            }
        });

        let drag_state_end = drag_state.clone();
        let window_end = window.clone();
        let layout_fixed_end = layout_fixed;
        let launcher_root_end = launcher_root;
        let state_manager_end = state_manager.clone();
        let caret_icon_end = caret_icon.clone();
        let caret_button_end = caret_button.clone();
        let action_bar_visible_end = action_bar_visible.clone();
        let action_bar = self.action_bar.clone();
        let has_dragged_end = has_dragged.clone();
        let compositor_end = compositor.clone();
        let pointer_in_fixed_end = pointer_in_fixed;
        let drag_offset_end = drag_offset;
        let pointer_device_end = pointer_device;

        drag_gesture.connect_drag_end(move |_, offset_x, offset_y| {
            let mut state = drag_state_end.borrow_mut();
            let did_drag = *has_dragged_end.borrow();

            if state.is_dragging {
                let monitor = window_end
                    .surface()
                    .and_then(|surface| surface.display().monitor_at_surface(&surface))
                    .or_else(|| window_end.monitor());
                if let Some(monitor) = monitor {
                    let geometry = monitor.geometry();
                    state.screen_width = geometry.width();
                    state.screen_height = geometry.height();
                    state.monitor_name = monitor
                        .connector()
                        .map(|name| name.to_string())
                        .or_else(|| compositor_end.get_focused_output());
                }

                // Finish drag operation
                let launcher_width = state.launcher_width;
                let launcher_height = state.launcher_height;

                // Clamp to ensure drag handle (on right side) remains accessible
                let min_left = -(launcher_width - 60);
                let max_left = state.screen_width - launcher_width;
                let min_top = 0;
                let max_top = state.screen_height - launcher_height.min(60);

                let final_left = (state.start_left + offset_x as i32)
                    .max(min_left)
                    .min(max_left);
                let final_top = (state.start_top + offset_y as i32)
                    .max(min_top)
                    .min(max_top);

                let surface_pointer = if let Some(device) = &pointer_device_end
                    && let Some(surface) = window_end.surface()
                {
                    surface.device_position(device).map(|(x, y, _)| (x, y))
                } else {
                    None
                };

                let (final_left, final_top) = if let (Some((cx, cy)), Some((ox, oy))) =
                    (surface_pointer, *drag_offset_end.borrow())
                {
                    (
                        ((cx - ox) as i32).max(min_left).min(max_left),
                        ((cy - oy) as i32).max(min_top).min(max_top),
                    )
                } else if let (Some((cx, cy)), Some((ox, oy))) =
                    (*pointer_in_fixed_end.borrow(), *drag_offset_end.borrow())
                {
                    (
                        ((cx - ox) as i32).max(min_left).min(max_left),
                        ((cy - oy) as i32).max(min_top).min(max_top),
                    )
                } else {
                    (final_left, final_top)
                };

                state.current_left = final_left;
                state.current_top = final_top;
                layout_fixed_end.move_(
                    &launcher_root_end,
                    f64::from(final_left),
                    f64::from(final_top),
                );

                let x_ratio = (f64::from(final_left) + f64::from(launcher_width) / 2.0)
                    / f64::from(state.screen_width);
                let y_ratio = f64::from(final_top) / f64::from(state.screen_height);

                // Save position per-monitor if we have a monitor name
                if let Some(ref monitor_name) = state.monitor_name {
                    state_manager_end.set_launcher_position_for_monitor(
                        monitor_name,
                        x_ratio,
                        y_ratio,
                    );
                } else {
                    state_manager_end.set_launcher_position(x_ratio, y_ratio);
                }

                state.is_dragging = false;
                state.drag_mode_active = false;
                drop(state);

                caret_button_end.remove_css_class("drag-mode");
                let icon = if *action_bar_visible_end.borrow() {
                    "chevron_right"
                } else {
                    "chevron_left"
                };
                caret_icon_end.set_label(icon);
            } else if !did_drag {
                // Quick click - toggle action bar
                drop(state);

                let next_visible = !*action_bar_visible_end.borrow();
                *action_bar_visible_end.borrow_mut() = next_visible;

                action_bar.set_actions_visible(next_visible);
                caret_icon_end.set_label(if next_visible {
                    "chevron_right"
                } else {
                    "chevron_left"
                });
            }
        });

        caret_button.add_controller(drag_gesture);
    }

    fn setup_click_catcher(&self) {
        let window = self.window.clone();
        let search_entry = self.search_entry.clone();
        let state = self.state.clone();
        let rpc_sender = self.rpc.as_ref().map(super::rpc::RpcHandle::event_sender);
        let click_catcher = self.click_catcher.window.clone();
        let preview_window = self.preview_window.window.clone();
        let preview_revealer = self.preview_window.revealer.clone();
        let state_manager = self.state_manager.clone();
        let config_watcher = self.config_watcher.clone();
        let fab_window = self.fab_window.clone();
        let layout_fixed = self.layout_fixed.clone();
        let launcher_root = self.launcher_root.clone();

        let handle_click_outside = Rc::new(move || {
            if state.borrow().show_window_picker {
                return;
            }

            let theme = config_watcher.theme();
            let click_action = &theme.config.behavior.click_outside_action;
            let visibility_state = state_manager.visibility_state();

            let should_minimize = match click_action {
                ClickOutsideAction::Close => false,
                ClickOutsideAction::Minimize => true,
                ClickOutsideAction::Intuitive => visibility_state.has_used_minimize(),
            };

            if should_minimize {
                info!("Click-away detected, minimizing to FAB");
                let session = {
                    let state_ref = state.borrow();
                    SessionState {
                        query: search_entry.text().to_string(),
                        results: state_ref.results.clone(),
                        active_plugin: state_ref.active_plugin.clone(),
                    }
                };
                visibility_state.save_session(session);
                visibility_state.minimize();
                window.set_visible(false);
                click_catcher.set_visible(false);
                preview_revealer.set_reveal_child(false);
                preview_window.set_visible(false);
                fab_window.show();
            } else {
                info!("Click-away detected, closing launcher");
                visibility_state.close();
                window.set_visible(false);
                click_catcher.set_visible(false);
                preview_revealer.set_reveal_child(false);
                preview_window.set_visible(false);
                fab_window.hide();
                // Don't clear UI state - will be kept or reset on next open based on time
            }

            if let Some(tx) = &rpc_sender
                && let Err(e) = tx.send_blocking(CoreEvent::LauncherClosed)
            {
                error!("Failed to send LauncherClosed: {}", e);
            }
        });

        // Fullscreen click-away: capture clicks anywhere in the launcher surface and only close
        // if the click is outside the launcher UI widget (QML-style boundary check).
        let handle_click_outside_gesture = handle_click_outside;
        let layout_fixed_for_bounds = layout_fixed.clone();
        let launcher_root_for_bounds = launcher_root;
        let gesture = gtk4::GestureClick::new();
        gesture.set_propagation_phase(gtk4::PropagationPhase::Capture);
        gesture.connect_released(move |_, _, x, y| {
            let Some(bounds) = launcher_root_for_bounds.compute_bounds(&layout_fixed_for_bounds)
            else {
                return;
            };

            let point = graphene::Point::new(x as f32, y as f32);
            if bounds.contains_point(&point) {
                return;
            }

            handle_click_outside_gesture();
        });
        layout_fixed.add_controller(gesture);
    }

    /// Setup FAB window click handler to restore launcher
    fn setup_fab_window(&self) {
        let window = self.window.clone();
        let click_catcher = self.click_catcher.window.clone();
        let search_entry = self.search_entry.clone();
        let state_manager = self.state_manager.clone();
        let rpc_sender = self.rpc.as_ref().map(super::rpc::RpcHandle::event_sender);
        let fab_window = self.fab_window.clone();
        let state = self.state.clone();
        let result_view = self.result_view.clone();
        let config_watcher = self.config_watcher.clone();
        let result_card = self.result_card.clone();
        let form_container = self.form_container.clone();
        let preview_window = self.preview_window.window.clone();
        let preview_revealer = self.preview_window.revealer.clone();

        self.fab_window.connect_clicked(move || {
            info!("FAB clicked, restoring launcher");
            let visibility_state = state_manager.visibility_state();
            let theme = config_watcher.theme();
            let restore_window_ms = theme.config.behavior.state_restore_window_ms;

            let restored_session = visibility_state.take_session_if_restorable(restore_window_ms);
            visibility_state.open();

            fab_window.hide();
            click_catcher.set_visible(false);
            window.set_visible(true);
            window.present();
            search_entry.grab_focus();

            form_container.set_visible(false);
            result_card.widget().set_visible(false);
            preview_revealer.set_reveal_child(false);
            preview_window.set_visible(false);

            {
                let mut state_mut = state.borrow_mut();
                state_mut.form_config = None;
                state_mut.form_view = None;
            }

            if let Some(session) = restored_session {
                info!(
                    "Restoring session: query='{}', {} results",
                    session.query,
                    session.results.len()
                );
                search_entry.set_text(&session.query);
                search_entry.set_position(-1);
                {
                    let mut state_mut = state.borrow_mut();
                    state_mut.results.clone_from(&session.results);
                    state_mut.active_plugin = session.active_plugin;
                }
                result_view.borrow().set_results(&session.results, &theme);

                if let Some(tx) = &rpc_sender
                    && let Err(e) = tx.send_blocking(CoreEvent::QueryChanged {
                        query: session.query,
                    })
                {
                    error!("Failed to send restored query: {}", e);
                }
            } else if let Some(tx) = &rpc_sender
                && let Err(e) = tx.send_blocking(CoreEvent::QueryChanged {
                    query: String::new(),
                })
            {
                error!("Failed to send initial query: {}", e);
            }
        });

        // FAB close button: hide FAB and reset hasUsedMinimize preference
        let state_manager = self.state_manager.clone();
        let fab_window = self.fab_window.clone();
        self.fab_window.connect_close(move || {
            info!("FAB close button clicked, resetting hasUsedMinimize");
            let visibility_state = state_manager.visibility_state();
            visibility_state.hard_close();
            state_manager.reset_has_used_minimize();
            fab_window.hide();
        });
    }

    /// Setup result list event handlers
    // Event handler setup - multiple callback connections sharing state via Rc<RefCell>
    #[allow(clippy::cast_sign_loss, clippy::too_many_lines)]
    fn setup_result_view_handlers(&self) {
        let Some(rpc) = &self.rpc else { return };

        let event_tx = rpc.event_sender();
        let state = self.state.clone();
        let compositor = self.compositor.clone();
        let window = self.window.clone();
        let search_entry = self.search_entry.clone();
        let result_view = self.result_view.clone();
        self.result_view.borrow().connect_select(move |item_id| {
            info!("Item selected: {}", item_id);

            let is_window_picker = {
                let state_ref = state.borrow();
                state_ref.show_window_picker
            };

            if is_window_picker && item_id.starts_with("__window__:") {
                let should_close =
                    Self::handle_window_picker_action(&state, &compositor, item_id, None);
                if should_close {
                    window.set_visible(false);
                    search_entry.set_text("");
                    result_view.borrow().clear();
                    if let Err(e) = event_tx.send_blocking(CoreEvent::LauncherClosed) {
                        error!("Failed to send LauncherClosed: {}", e);
                    }
                }
                return;
            }

            let plugin_id = {
                let mut state_mut = state.borrow_mut();
                let result = state_mut.results.iter().find(|r| r.id == item_id);
                let pending = result.map(|r| {
                    (
                        r.app_id.clone(),
                        r.app_id_fallback.clone(),
                        r.name.clone(),
                        r.icon.clone(),
                    )
                });
                let plugin_id = result.and_then(|r| r.plugin_id.clone());
                debug!(
                    "Storing pending from selection: item_id={}, pending={:?}",
                    item_id, pending
                );
                if let Some((app_id, app_id_fallback, name, icon)) = pending {
                    state_mut.pending_app_id = app_id;
                    state_mut.pending_app_id_fallback = app_id_fallback;
                    state_mut.pending_app_name = Some(name);
                    state_mut.pending_app_icon = icon;
                }
                plugin_id
            };
            if let Err(e) = event_tx.send_blocking(CoreEvent::ItemSelected {
                id: item_id.to_string(),
                action: None,
                plugin_id,
            }) {
                error!("Failed to send selection: {}", e);
            }
        });

        let event_tx = rpc.event_sender();
        let state = self.state.clone();
        let compositor = self.compositor.clone();
        let window = self.window.clone();
        let search_entry = self.search_entry.clone();
        let result_view = self.result_view.clone();
        let config_watcher = self.config_watcher.clone();
        self.result_view
            .borrow()
            .connect_action(move |item_id, action_id| {
                info!("Action clicked: {} on item {}", action_id, item_id);

                let is_window_picker = {
                    let state_ref = state.borrow();
                    state_ref.show_window_picker
                };

                if is_window_picker && item_id.starts_with("__window__:") {
                    let should_close = Self::handle_window_picker_action(
                        &state,
                        &compositor,
                        item_id,
                        Some(action_id),
                    );
                    if should_close {
                        window.set_visible(false);
                        search_entry.set_text("");
                        result_view.borrow().clear();
                        if let Err(e) = event_tx.send_blocking(CoreEvent::LauncherClosed) {
                            error!("Failed to send LauncherClosed: {}", e);
                        }
                    } else {
                        let (app_name, windows) = {
                            let state_ref = state.borrow();
                            (
                                state_ref
                                    .pending_app_name
                                    .clone()
                                    .unwrap_or_else(|| "App".to_string()),
                                state_ref.window_picker_windows.clone(),
                            )
                        };
                        if !windows.is_empty() {
                            let theme = config_watcher.theme();
                            Self::show_window_picker_view(
                                &state,
                                &compositor,
                                &window,
                                &result_view,
                                &app_name,
                                &theme,
                            );
                        }
                    }
                    return;
                }

                let plugin_id = {
                    let mut state_mut = state.borrow_mut();
                    let result = state_mut.results.iter().find(|r| r.id == item_id);
                    let pending = result.map(|r| {
                        (
                            r.app_id.clone(),
                            r.app_id_fallback.clone(),
                            r.name.clone(),
                            r.icon.clone(),
                        )
                    });
                    let plugin_id = result.and_then(|r| r.plugin_id.clone());
                    if let Some((app_id, app_id_fallback, name, icon)) = pending {
                        state_mut.pending_app_id = app_id;
                        state_mut.pending_app_id_fallback = app_id_fallback;
                        state_mut.pending_app_name = Some(name);
                        state_mut.pending_app_icon = icon;
                    }
                    plugin_id
                };
                if let Err(e) = event_tx.send_blocking(CoreEvent::ItemSelected {
                    id: item_id.to_string(),
                    action: Some(action_id.to_string()),
                    plugin_id,
                }) {
                    error!("Failed to send action: {}", e);
                }
            });

        let event_tx = rpc.event_sender();
        let state = self.state.clone();
        self.result_view
            .borrow()
            .connect_slider(move |item_id, value| {
                debug!("Slider changed: {} = {}", item_id, value);
                let state_ref = state.borrow();
                let plugin_id = state_ref
                    .results
                    .iter()
                    .find(|r| r.id == item_id)
                    .and_then(|r| r.plugin_id.clone())
                    .or_else(|| state_ref.active_plugin.as_ref().map(|(id, _)| id.clone()));
                drop(state_ref);
                if let Err(e) = event_tx.send_blocking(CoreEvent::SliderChanged {
                    id: item_id.to_string(),
                    value,
                    plugin_id,
                }) {
                    error!("Failed to send slider change: {}", e);
                }
            });

        let event_tx = rpc.event_sender();
        let state = self.state.clone();
        self.result_view
            .borrow()
            .connect_switch(move |item_id, value| {
                let state_ref = state.borrow();
                let result = state_ref.results.iter().find(|r| r.id == item_id);
                debug!(
                    "Switch toggled: {} = {}, found_result={}, results_count={}",
                    item_id,
                    value,
                    result.is_some(),
                    state_ref.results.len()
                );
                if let Some(r) = result {
                    debug!("  result.plugin_id = {:?}", r.plugin_id);
                }
                let plugin_id = result
                    .and_then(|r| r.plugin_id.clone())
                    .or_else(|| state_ref.active_plugin.as_ref().map(|(id, _)| id.clone()));
                drop(state_ref);
                if let Err(e) = event_tx.send_blocking(CoreEvent::SwitchToggled {
                    id: item_id.to_string(),
                    value,
                    plugin_id,
                }) {
                    error!("Failed to send switch toggle: {}", e);
                }
            });
    }

    fn setup_result_card_handlers(&self) {
        let Some(rpc) = &self.rpc else { return };

        let event_tx = rpc.event_sender();
        let state = self.state.clone();
        self.result_card.connect_action(move |context, action_id| {
            info!("Card action clicked: {} on context {}", action_id, context);

            let plugin_id = state
                .borrow()
                .active_plugin
                .as_ref()
                .map(|(id, _)| id.clone());

            if let Err(e) = event_tx.send_blocking(CoreEvent::ItemSelected {
                id: context.to_string(),
                action: Some(action_id.to_string()),
                plugin_id,
            }) {
                error!("Failed to send card action: {}", e);
            }
        });
    }

    fn setup_form_handlers(&self) {
        let Some(rpc) = &self.rpc else { return };

        let event_tx = rpc.event_sender();
        let state = self.state.clone();
        let form_container = self.form_container.clone();
        let form_cancel = self.form_cancel.clone();
        let result_view = self.result_view.clone();
        let search_entry = self.search_entry.clone();
        form_cancel.connect_clicked(move |_| {
            info!("Form cancel clicked");
            form_container.set_visible(false);
            let mut state_mut = state.borrow_mut();
            state_mut.form_view = None;
            state_mut.form_config = None;
            state_mut.form_context = None;
            state_mut.form_data.clear();
            let compact_mode = state_mut.compact_mode;
            drop(state_mut);

            let empty_query = search_entry.text().trim().is_empty();
            result_view
                .borrow()
                .widget()
                .set_visible(!(compact_mode && empty_query));

            if let Err(e) = event_tx.send_blocking(CoreEvent::FormCancelled) {
                error!("Failed to send FormCancelled: {}", e);
            }
        });

        let event_tx = rpc.event_sender();
        let state = self.state.clone();
        let form_submit = self.form_submit.clone();
        let form_container = self.form_container.clone();
        let result_view = self.result_view.clone();
        let search_entry = self.search_entry.clone();
        form_submit.connect_clicked(move |_| {
            let state_mut = state.borrow_mut();
            let Some(form) = state_mut.form_config.clone() else {
                return;
            };
            let form_data = match state_mut.form_view.as_ref() {
                Some(view) => view.collect_values(),
                None => state_mut.form_data.clone(),
            };
            let context = state_mut.form_context.clone();
            let live_update = form.live_update;
            drop(state_mut);

            if live_update {
                return;
            }

            info!("Form submit clicked ({} fields)", form_data.len());
            form_container.set_visible(false);
            let mut state_mut = state.borrow_mut();
            state_mut.form_view = None;
            state_mut.form_config = None;
            state_mut.form_context = None;
            state_mut.form_data.clear();
            let compact_mode = state_mut.compact_mode;
            drop(state_mut);

            let empty_query = search_entry.text().trim().is_empty();
            result_view
                .borrow()
                .widget()
                .set_visible(!(compact_mode && empty_query));

            if let Err(e) = event_tx.send_blocking(CoreEvent::FormSubmitted { form_data, context })
            {
                error!("Failed to send FormSubmitted: {}", e);
            }
        });
    }

    // Event handler setup - multiple callback connections sharing state via Rc<RefCell>
    #[allow(clippy::too_many_lines)]
    fn setup_preview_panel_handlers(&self) {
        let Some(rpc) = &self.rpc else { return };

        // Handle preview panel action clicks
        let event_tx = rpc.event_sender();
        let state = self.state.clone();
        self.preview_window
            .panel
            .connect_action(move |item_id: &str, action_id: &str| {
                info!("Preview action clicked: {} on item {}", action_id, item_id);

                let plugin_id = state
                    .borrow()
                    .results
                    .iter()
                    .find(|r| r.id == item_id)
                    .and_then(|r| r.plugin_id.clone());

                if let Err(e) = event_tx.send_blocking(CoreEvent::ItemSelected {
                    id: item_id.to_string(),
                    action: Some(action_id.to_string()),
                    plugin_id,
                }) {
                    error!("Failed to send preview action: {}", e);
                }
            });

        // Handle preview panel pin clicks (create sticky note)
        let pinned_manager = self.pinned_panel_manager.clone();
        let app = self.window.application().expect("Application");
        let config_watcher = self.config_watcher.clone();
        let drag_state = self.drag_state.clone();
        let preview_window_for_pin = self.preview_window.window.clone();
        let preview_revealer_for_pin = self.preview_window.revealer.clone();
        self.preview_window
            .panel
            .connect_pin(move |item_id, title, preview| {
                info!("Pin requested for item: {}", item_id);
                let theme = config_watcher.theme();
                let ds = drag_state.borrow();

                // Get current preview window position (absolute, monitor-relative)
                let left = preview_window_for_pin.margin(Edge::Left);
                let top = preview_window_for_pin.margin(Edge::Top);

                // Get the monitor the preview is on
                let monitor = preview_window_for_pin.monitor();

                pinned_manager.pin(
                    &app,
                    &theme,
                    item_id,
                    title,
                    preview,
                    left,
                    top,
                    ds.screen_width,
                    ds.screen_height,
                    monitor.as_ref(),
                );

                // Hide the preview panel after pinning
                preview_revealer_for_pin.set_reveal_child(false);
                preview_window_for_pin.set_visible(false);
            });

        // Connect selection change to update preview panel
        let preview_panel = self.preview_window.panel.clone();
        let preview_window_widget = self.preview_window.window.clone();
        let preview_revealer = self.preview_window.revealer.clone();
        let preview_current_id = self.preview_window.current_item_id.clone();
        let launcher_window = self.window.clone();
        let launcher_root_for_preview = self.launcher_root.clone();
        let drag_state = self.drag_state.clone();
        self.result_view
            .borrow()
            .connect_selection_change(move |result| {
                if let Some(result) = result {
                    if let Some(preview) = &result.preview {
                        if !launcher_window.is_visible() {
                            return;
                        }
                        debug!("Updating preview for: {}", result.id);

                        // Position preview window on the side with more space
                        let ds = drag_state.borrow();
                        // Use actual launcher widget width (changes with grid mode) instead of config value
                        let launcher_width = launcher_root_for_preview.width();
                        let preview_width = preview_design::WIDTH;
                        let gap = 8;

                        let launcher_left = ds.current_left;
                        let launcher_top = ds.current_top;

                        let right_space = ds.screen_width - (launcher_left + launcher_width);
                        let left_space = launcher_left;

                        // Choose the side with more available space
                        let show_on_right = right_space >= left_space;
                        let preview_x = if show_on_right {
                            launcher_left + launcher_width + gap
                        } else {
                            (launcher_left - preview_width - gap).max(0)
                        };

                        let transition = if show_on_right {
                            gtk4::RevealerTransitionType::SlideLeft
                        } else {
                            gtk4::RevealerTransitionType::SlideRight
                        };
                        preview_revealer.set_transition_type(transition);

                        preview_window_widget.set_margin(Edge::Left, preview_x);
                        preview_window_widget.set_margin(Edge::Top, launcher_top);
                        preview_window_widget.set_visible(true);

                        // Animated content change: slide out, update, slide in
                        let current_id = preview_current_id.borrow().clone();
                        let is_visible = preview_revealer.reveals_child();
                        let is_different = current_id != result.id && !current_id.is_empty();

                        if is_visible && is_different {
                            // Slide out first, then update content
                            preview_revealer.set_reveal_child(false);

                            let panel = preview_panel.clone();
                            let revealer = preview_revealer.clone();
                            let current_id_ref = preview_current_id.clone();
                            let new_id = result.id.clone();
                            let new_preview = preview.clone();

                            let preview_window_guard = preview_window_widget.clone();
                            gtk4::glib::timeout_add_local_once(
                                std::time::Duration::from_millis(u64::from(
                                    PreviewWindow::ANIMATION_DURATION_MS,
                                )),
                                move || {
                                    // Guard: only reveal if preview window is still visible
                                    // (launcher may have closed during the animation)
                                    if !preview_window_guard.is_visible() {
                                        return;
                                    }
                                    current_id_ref.borrow_mut().clone_from(&new_id);
                                    panel.set_preview(&new_id, &new_preview);
                                    revealer.set_reveal_child(true);
                                },
                            );
                        } else {
                            // First show or same item - update and reveal immediately
                            preview_current_id.borrow_mut().clone_from(&result.id);
                            preview_panel.set_preview(&result.id, preview);
                            preview_revealer.set_reveal_child(true);
                        }
                    } else {
                        *preview_current_id.borrow_mut() = String::new();
                        preview_revealer.set_reveal_child(false);
                        preview_window_widget.set_visible(false);
                    }
                } else {
                    *preview_current_id.borrow_mut() = String::new();
                    preview_revealer.set_reveal_child(false);
                    preview_window_widget.set_visible(false);
                }
            });
    }

    // Event handler setup - multiple callback connections sharing state via Rc<RefCell>
    #[allow(clippy::cast_sign_loss, clippy::too_many_lines)]
    fn setup_search_handlers(&mut self) {
        let Some(rpc) = &self.rpc else { return };

        let event_tx = rpc.event_sender();
        let state = self.state.clone();
        self.search_entry.connect_changed(move |entry| {
            let query = entry.text().to_string();
            let state = state.borrow();
            if state.input_mode == "realtime"
                && let Err(e) = event_tx.send_blocking(CoreEvent::QueryChanged { query })
            {
                error!("Failed to send query: {}", e);
            }
        });

        let event_tx = rpc.event_sender();
        let state = self.state.clone();
        let result_view = self.result_view.clone();
        let compositor = self.compositor.clone();
        let window = self.window.clone();
        let search_entry_clone = self.search_entry.clone();
        self.search_entry.connect_activate(move |entry| {
            let (input_mode, selected_action_index, is_window_picker) = {
                let state_ref = state.borrow();
                (
                    state_ref.input_mode.clone(),
                    state_ref.selected_action_index,
                    state_ref.show_window_picker,
                )
            };

            if input_mode == "submit" {
                let query = entry.text().to_string();
                info!("Submit query: {}", query);
                if let Err(e) = event_tx.send_blocking(CoreEvent::QuerySubmitted {
                    query,
                    context: None,
                }) {
                    error!("Failed to send query: {}", e);
                }
            } else if let Some(id) = result_view.borrow().selected_id() {
                if is_window_picker && id.starts_with("__window__:") {
                    info!("Window picker selection (Enter): {}", id);
                    let should_close =
                        Self::handle_window_picker_action(&state, &compositor, &id, None);
                    if should_close {
                        window.set_visible(false);
                        search_entry_clone.set_text("");
                        result_view.borrow().clear();
                        if let Err(e) = event_tx.send_blocking(CoreEvent::LauncherClosed) {
                            error!("Failed to send LauncherClosed: {}", e);
                        }
                    }
                    return;
                }

                let action = if selected_action_index >= 0 {
                    result_view.borrow().selected_result().and_then(|r| {
                        r.actions
                            .get(selected_action_index as usize)
                            .map(|a| a.id.clone())
                    })
                } else {
                    None
                };

                if result_view.borrow().selected_is_switch() {
                    let result = result_view.borrow().selected_result();
                    if let Some(ref r) = result {
                        let current = r.slider_value().is_some_and(|v| v.value > 0.0);
                        info!(
                            "Toggle switch via Enter: {} from {} to {}",
                            r.id, current, !current
                        );
                        result_view.borrow().toggle_selected_switch();
                        if let Err(e) = event_tx.send_blocking(CoreEvent::SwitchToggled {
                            id: r.id.clone(),
                            value: !current,
                            plugin_id: r.plugin_id.clone(),
                        }) {
                            error!("Failed to send switch toggle: {}", e);
                        }
                    }
                    return;
                }

                let plugin_id = {
                    let mut state_mut = state.borrow_mut();
                    let result = state_mut.results.iter().find(|r| r.id == id);
                    let pending = result.map(|r| {
                        (
                            r.app_id.clone(),
                            r.app_id_fallback.clone(),
                            r.name.clone(),
                            r.icon.clone(),
                        )
                    });
                    let plugin_id = result.and_then(|r| r.plugin_id.clone());
                    if let Some((app_id, app_id_fallback, name, icon)) = pending {
                        state_mut.pending_app_id = app_id;
                        state_mut.pending_app_id_fallback = app_id_fallback;
                        state_mut.pending_app_name = Some(name);
                        state_mut.pending_app_icon = icon;
                    }
                    plugin_id
                };

                info!("Select item: {} with action: {:?}", id, action);
                if let Err(e) = event_tx.send_blocking(CoreEvent::ItemSelected {
                    id,
                    action,
                    plugin_id,
                }) {
                    error!("Failed to send selection: {}", e);
                }
            }
        });

        // Key controller on entry for backspace handling and grid navigation
        // (must be on entry to intercept before text deletion / cursor movement)
        let event_tx = rpc.event_sender();
        let state = self.state.clone();
        let search_entry = self.search_entry.clone();
        let result_view = self.result_view.clone();
        let state_manager = self.state_manager.clone();
        let entry_key_controller = gtk4::EventControllerKey::new();
        entry_key_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
        entry_key_controller.connect_key_pressed(move |_, keyval, _keycode, modifier| {
            let ctrl = modifier.contains(gdk::ModifierType::CONTROL_MASK);
            let shift = modifier.contains(gdk::ModifierType::SHIFT_MASK);

            match keyval {
                // Ctrl+C: Toggle compact mode (intercept before entry handles it as copy)
                gdk::Key::c if ctrl && !shift => {
                    let enabled = {
                        let mut s = state.borrow_mut();
                        s.compact_mode = !s.compact_mode;
                        s.compact_mode
                    };

                    info!("Compact mode toggled via keyboard: {}", enabled);
                    state_manager.set_compact_mode(enabled);

                    let empty_query = search_entry.text().trim().is_empty();
                    let should_hide_results = enabled && empty_query;
                    result_view
                        .borrow()
                        .widget()
                        .set_visible(!should_hide_results);

                    glib::Propagation::Stop
                }
                // Shift+Backspace - exit plugin immediately
                gdk::Key::BackSpace if shift => {
                    let s = state.borrow();
                    if s.active_plugin.is_some() {
                        let _ = event_tx.send_blocking(CoreEvent::ClosePlugin);
                        return glib::Propagation::Stop;
                    }
                    glib::Propagation::Proceed
                }
                gdk::Key::BackSpace => {
                    let input_empty = search_entry.text().is_empty();
                    if input_empty {
                        let s = state.borrow();
                        if s.plugin_management {
                            drop(s);
                            let _ = event_tx.send_blocking(CoreEvent::Back);
                            return glib::Propagation::Stop;
                        } else if s.active_plugin.is_some() {
                            if s.navigation_depth > 0 {
                                drop(s);
                                state.borrow_mut().pending_back = true;
                                let _ = event_tx.send_blocking(CoreEvent::Back);
                            } else {
                                drop(s);
                                let _ = event_tx.send_blocking(CoreEvent::ClosePlugin);
                            }
                            return glib::Propagation::Stop;
                        }
                    }
                    glib::Propagation::Proceed
                }
                // Enter/Shift+Enter: Adjust slider value (only when slider is selected)
                gdk::Key::Return | gdk::Key::KP_Enter | gdk::Key::ISO_Enter => {
                    if result_view.borrow().selected_is_slider() {
                        let direction = if shift { -1 } else { 1 };
                        let result = result_view.borrow().selected_result();
                        if let Some(ref r) = result
                            && let Some(slider) = r.slider_value()
                        {
                            let new_val = if shift {
                                (slider.value - slider.step).max(slider.min)
                            } else {
                                (slider.value + slider.step).min(slider.max)
                            };
                            info!(
                                "Adjust slider via {}Enter: {} from {} to {}",
                                if shift { "Shift+" } else { "" },
                                r.id,
                                slider.value,
                                new_val
                            );
                            result_view.borrow().adjust_selected_slider(direction);
                            if let Err(e) = event_tx.send_blocking(CoreEvent::SliderChanged {
                                id: r.id.clone(),
                                value: new_val,
                                plugin_id: r.plugin_id.clone(),
                            }) {
                                error!("Failed to send slider change: {}", e);
                            }
                        }
                        return glib::Propagation::Stop;
                    }
                    // Not a slider, let connect_activate handle it
                    glib::Propagation::Proceed
                }

                // In Grid mode, intercept Left/Right arrow keys for grid navigation
                gdk::Key::Left | gdk::Key::Right => {
                    let view_mode = state.borrow().view_mode;
                    if view_mode == ResultViewMode::Grid {
                        if keyval == gdk::Key::Left {
                            result_view.borrow().select_left();
                        } else {
                            result_view.borrow().select_right();
                        }
                        state.borrow_mut().selected_action_index = -1;
                        result_view.borrow().set_selected_action(-1);
                        return glib::Propagation::Stop;
                    }
                    glib::Propagation::Proceed
                }
                _ => glib::Propagation::Proceed,
            }
        });
        self.search_entry.add_controller(entry_key_controller);
    }

    // GTK margins require i32, screen positions bounded by display dimensions
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    fn setup_update_polling(&mut self) {
        let Some(rpc) = &self.rpc else { return };

        let update_rx = rpc.update_receiver();
        let event_tx = rpc.event_sender();
        let state = self.state.clone();
        let result_view = self.result_view.clone();
        let result_card = self.result_card.clone();
        let action_bar = self.action_bar.clone();
        let search_entry = self.search_entry.clone();
        let icon_container = self.icon_container.clone();
        let icon_label = self.icon_label.clone();
        let spinner = self.spinner.clone();
        let caret_icon = self.caret_icon.clone();
        let action_bar_visible = self.action_bar_visible.clone();
        let content_container = self.content_container.clone();
        let layout_fixed = self.layout_fixed.clone();
        let launcher_root = self.launcher_root.clone();
        let background = self.background.clone();
        let window = self.window.clone();
        let click_catcher_window = self.click_catcher.window.clone();
        let compositor = self.compositor.clone();
        let config_watcher = self.config_watcher.clone();
        let width_animation_source = self.width_animation_source.clone();
        let state_manager = self.state_manager.clone();
        let drag_state = self.drag_state.clone();
        let fab_window = self.fab_window.clone();
        let form_container = self.form_container.clone();
        let form_title = self.form_title.clone();
        let form_fields = self.form_fields.clone();
        let form_submit = self.form_submit.clone();
        let form_cancel = self.form_cancel.clone();
        let error_dialog = self.error_dialog.clone();

        let preview_window = self.preview_window.window.clone();
        let preview_revealer = self.preview_window.revealer.clone();
        let preview_panel = self.preview_window.panel.clone();

        let depth_indicator = self.depth_indicator.clone();
        let ctx = UpdateContext {
            state,
            result_view,
            result_card,
            preview_window,
            preview_revealer,
            preview_panel,
            action_bar,
            form_container,
            form_title,
            form_fields,
            form_submit,
            form_cancel,
            search_entry,
            icon_container,
            icon_label,
            spinner,
            depth_indicator,
            caret_icon,
            action_bar_visible,
            content_container,
            layout_fixed,
            launcher_root,
            background,
            window,
            click_catcher_window,
            event_tx,
            compositor,
            config_watcher,
            width_animation_source,
            state_manager,
            drag_state,
            fab_window,
            error_dialog,
        };

        glib::spawn_future_local(async move {
            while let Ok(update) = update_rx.recv().await {
                Self::handle_update(update, &ctx);
            }
            debug!("Update receiver closed");
        });
    }

    fn reset_state_after_daemon_disconnect(ctx: &UpdateContext) {
        let UpdateContext {
            state,
            result_view,
            result_card,
            preview_window,
            preview_revealer,
            preview_panel,
            action_bar,
            form_container,
            search_entry,
            icon_container,
            icon_label,
            depth_indicator,
            action_bar_visible,
            config_watcher,
            state_manager,
            ..
        } = ctx;

        let default_mode = if config_watcher.theme().config.appearance.default_result_view == "grid"
        {
            ResultViewMode::Grid
        } else {
            ResultViewMode::List
        };

        let visibility_state = state_manager.visibility_state();
        visibility_state.hard_close();
        let _ = visibility_state.take_session_if_restorable(0);

        search_entry.set_text("");
        {
            let mut state_mut = state.borrow_mut();
            state_mut.results.clear();
            state_mut.active_plugin = None;
            state_mut.input_mode = "realtime".to_string();
            state_mut.navigation_depth = 0;
            state_mut.pending_back = false;
            state_mut.plugin_actions.clear();
            state_mut.selected_action_index = -1;
            state_mut.placeholder = DEFAULT_PLACEHOLDER.to_string();
            state_mut.current_card = None;
            state_mut.plugin_display_hint = None;
            state_mut.form_context = None;
            state_mut.form_data.clear();
            state_mut.form_config = None;
            state_mut.form_view = None;
            state_mut.view_mode = default_mode;
            state_mut.plugin_management = false;
        }

        action_bar.set_mode(ActionBarMode::Hints);
        action_bar.set_navigation_depth(0);
        action_bar.set_actions(Vec::new());
        action_bar.set_actions_visible(*action_bar_visible.borrow());
        Self::update_depth_indicator(depth_indicator, 0, config_watcher);

        icon_label.set_label("gavel");
        icon_container.remove_css_class("plugin-active");

        form_container.set_visible(false);
        result_card.clear();
        result_card.widget().set_visible(false);
        preview_revealer.set_reveal_child(false);
        preview_window.set_visible(false);
        preview_panel.clear();

        result_view.borrow().clear();
        result_view.borrow_mut().set_mode(default_mode);
        result_view.borrow().widget().set_visible(true);
    }

    // GTK margins require i32, screen positions bounded by display dimensions
    // 1:1 CoreUpdate variant mapping - each arm updates specific UI components
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        clippy::too_many_lines
    )]
    fn handle_update(update: CoreUpdate, ctx: &UpdateContext) {
        let UpdateContext {
            state,
            result_view,
            result_card,
            preview_window,
            preview_revealer,
            preview_panel,
            action_bar,
            form_container,
            form_title: _,
            form_fields: _,
            form_submit: _,
            form_cancel: _,
            search_entry,
            icon_container,
            icon_label,
            spinner,
            depth_indicator,
            caret_icon,
            action_bar_visible,
            content_container,
            layout_fixed,
            launcher_root,
            background,
            window,
            click_catcher_window,
            event_tx,
            compositor,
            config_watcher,
            width_animation_source,
            state_manager,
            drag_state,
            fab_window,
            error_dialog,
        } = ctx;
        match update {
            CoreUpdate::Results {
                results,
                placeholder,
                clear_input,
                input_mode,
                display_hint,
                navigate_forward,
                ..
            } => {
                debug!(
                    "Received {} results, display_hint={:?}, navigate_forward={:?}",
                    results.len(),
                    display_hint,
                    navigate_forward
                );

                let (view_mode, compact_mode, empty_query, has_form) = {
                    let mut state_mut = state.borrow_mut();
                    state_mut.results.clone_from(&results);
                    state_mut.selected_action_index = -1;
                    state_mut.current_card = None;

                    // Handle navigation depth changes based on navigate_forward and pending_back
                    let depth_changed;
                    if let Some(true) = navigate_forward {
                        state_mut.navigation_depth += 1;
                        state_mut.pending_back = false;
                        action_bar.set_navigation_depth(state_mut.navigation_depth);
                        depth_changed = true;
                    } else if state_mut.pending_back {
                        state_mut.navigation_depth = state_mut.navigation_depth.saturating_sub(1);
                        state_mut.pending_back = false;
                        action_bar.set_navigation_depth(state_mut.navigation_depth);
                        depth_changed = true;
                    } else {
                        depth_changed = false;
                    }

                    if depth_changed {
                        Self::update_depth_indicator(
                            depth_indicator,
                            state_mut.navigation_depth,
                            config_watcher,
                        );
                    }

                    if let Some(p) = placeholder {
                        state_mut.placeholder.clone_from(&p);
                        search_entry.set_placeholder_text(Some(&p));
                    }
                    if let Some(true) = clear_input {
                        search_entry.set_text("");
                    }
                    if let Some(mode) = input_mode {
                        state_mut.input_mode = mode;
                    }

                    // Apply display hint from plugin if provided
                    if let Some(ref hint) = display_hint {
                        state_mut.plugin_display_hint = Some(hint.clone());
                        match hint {
                            DisplayHint::Grid | DisplayHint::LargeGrid => {
                                state_mut.view_mode = ResultViewMode::Grid;
                            }
                            DisplayHint::List => {
                                state_mut.view_mode = ResultViewMode::List;
                            }
                            DisplayHint::Auto => {
                                // Keep current view mode
                            }
                        }
                    }

                    // Clear form if we're navigating back
                    if depth_changed && state_mut.form_view.is_some() {
                        state_mut.form_view = None;
                        state_mut.form_config = None;
                        state_mut.form_context = None;
                    }

                    let compact_mode = state_mut.compact_mode;
                    let empty_query = search_entry.text().trim().is_empty();
                    let has_form = state_mut.form_view.is_some();
                    (state_mut.view_mode, compact_mode, empty_query, has_form)
                };

                if has_form {
                    return;
                }

                // Hide card when receiving results
                result_card.widget().set_visible(false);
                form_container.set_visible(false);

                // Dismiss any existing action popover
                result_view.borrow().set_selected_action(-1);

                // Set the view mode (this handles switching internally)
                result_view.borrow_mut().set_mode(view_mode);

                let should_hide_results = compact_mode && empty_query;
                result_view
                    .borrow()
                    .widget()
                    .set_visible(!should_hide_results);

                // Merge status at display time only
                let merged_results: Vec<SearchResult> = {
                    let state_ref = state.borrow();
                    results
                        .iter()
                        .map(|r| {
                            if matches!(r.result_type, ResultType::Plugin | ResultType::Recent)
                                && let Some(status) = state_ref.plugin_statuses.get(&r.id)
                            {
                                let mut merged = r.clone();
                                merged.badges.clone_from(&status.badges);
                                merged.chips.clone_from(&status.chips);
                                merged
                            } else {
                                r.clone()
                            }
                        })
                        .collect()
                };

                // Refresh running apps from compositor
                result_view.borrow().refresh_running_apps(compositor);

                // Get theme for passing to result view
                let theme = config_watcher.theme();

                // Check if query changed to determine if we should reset selection
                let current_query = search_entry.text().to_string();
                let query_changed = {
                    let state_borrow = state.borrow();
                    state_borrow.last_query != current_query
                };

                // Update stored query first
                {
                    state.borrow_mut().last_query.clone_from(&current_query);
                }

                // Update results
                let in_plugin = state.borrow().active_plugin.is_some();
                if in_plugin {
                    result_view.borrow().update_results_diff_with_selection(
                        &merged_results,
                        &theme,
                        query_changed,
                    );
                } else {
                    result_view.borrow().set_results_with_selection(
                        &merged_results,
                        &theme,
                        query_changed,
                    );
                }

                // GtkFixed doesn't always relayout immediately when a positioned child changes its
                // preferred size. Force a resize pass so the results list can expand.
                result_view.borrow().widget().queue_resize();
                launcher_root.queue_resize();
                layout_fixed.queue_resize();

                let result_view_widget = result_view.borrow().widget().clone();
                let launcher_root_widget = launcher_root.clone();
                glib::idle_add_local_once(move || {
                    debug!(
                        "layout after results: launcher_root(alloc={}x{}), result_view(visible={}, alloc={}x{})",
                        launcher_root_widget.width(),
                        launcher_root_widget.height(),
                        result_view_widget.is_visible(),
                        result_view_widget.width(),
                        result_view_widget.height(),
                    );
                });

                // Animate width based on view mode, repositioning preview during animation
                let theme = config_watcher.theme();
                let target_width = match view_mode {
                    ResultViewMode::List => theme.config.sizes.search_width,
                    ResultViewMode::Grid => theme.config.appearance.grid.calculate_width(),
                };
                let preview_window_anim = preview_window.clone();
                let preview_revealer_anim = preview_revealer.clone();
                let launcher_root_anim = launcher_root.clone();
                let drag_state_anim = drag_state.clone();
                Self::animate_width_with_callback(
                    content_container,
                    width_animation_source,
                    target_width,
                    Some(move |new_width| {
                        let ds = drag_state_anim.borrow();
                        Self::reposition_preview(
                            &preview_window_anim,
                            &preview_revealer_anim,
                            ds.current_left,
                            ds.current_top,
                            ds.screen_width,
                            new_width,
                            launcher_root_anim.width(),
                        );
                    }),
                );
            }
            CoreUpdate::ResultsUpdate { patches } => {
                // Apply partial updates to existing results
                let mut state_mut = state.borrow_mut();
                for patch in patches {
                    if let Some(result) = state_mut.results.iter_mut().find(|r| r.id == patch.id) {
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
                                if let Some(WidgetData::Slider { min, max, step, .. }) =
                                    &result.widget
                                {
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

                let compact_mode = state_mut.compact_mode;
                let empty_query = search_entry.text().trim().is_empty();
                let has_form = state_mut.form_view.is_some();

                // Re-merge with plugin status and update display
                let merged_results: Vec<SearchResult> = state_mut
                    .results
                    .iter()
                    .map(|r| {
                        if matches!(r.result_type, ResultType::Plugin | ResultType::Recent)
                            && let Some(status) = state_mut.plugin_statuses.get(&r.id)
                        {
                            let mut merged = r.clone();
                            merged.badges.clone_from(&status.badges);
                            merged.chips.clone_from(&status.chips);
                            merged
                        } else {
                            r.clone()
                        }
                    })
                    .collect();
                drop(state_mut);

                if !has_form {
                    let should_hide_results = compact_mode && empty_query;
                    result_view
                        .borrow()
                        .widget()
                        .set_visible(!should_hide_results);
                    let theme = config_watcher.theme();
                    result_view
                        .borrow()
                        .update_results_diff(&merged_results, &theme);
                }
            }
            CoreUpdate::Card { card, context } => {
                info!(
                    "Received card: title='{}', content={:?}, markdown={:?}, blocks={}, context={:?}",
                    card.title,
                    card.content.as_ref().map(std::string::String::len),
                    card.markdown.as_ref().map(std::string::String::len),
                    card.blocks.len(),
                    context
                );
                // Store current card in state (context now stored in ResultCard)
                {
                    let mut state_mut = state.borrow_mut();
                    state_mut.current_card = Some(card.clone());
                    state_mut.form_config = None;
                    state_mut.form_view = None;
                }

                // Hide result view and preview window, show card with context
                result_view.borrow().widget().set_visible(false);
                form_container.set_visible(false);
                preview_revealer.set_reveal_child(false);
                preview_window.set_visible(false);
                result_card.set_card_with_context(&card, context.as_deref());
                // Apply card-specific max height if specified
                if let Some(max_height) = card.max_height {
                    result_card.set_max_height(max_height.cast_signed());
                }
                result_card.widget().set_visible(true);
            }
            CoreUpdate::Form { form } => {
                info!(
                    "Received form: '{}' ({} fields)",
                    form.title,
                    form.fields.len()
                );

                // Increment navigation depth for forms (like QML does)
                // Forms always represent drilling down into detail
                {
                    let mut state_mut = state.borrow_mut();
                    state_mut.navigation_depth += 1;
                    state_mut.pending_back = false;
                    action_bar.set_navigation_depth(state_mut.navigation_depth);
                    Self::update_depth_indicator(
                        depth_indicator,
                        state_mut.navigation_depth,
                        config_watcher,
                    );
                }

                Self::show_form(&form, ctx);
            }
            CoreUpdate::InputModeChanged { mode } => {
                debug!("Input mode changed: {}", mode);
                state.borrow_mut().input_mode = mode;
            }
            CoreUpdate::ContextChanged { context } => {
                debug!("Context changed: {:?}", context);
                let mut state_mut = state.borrow_mut();
                state_mut.form_context = context;
                if let Some(form) = state_mut.form_config.clone() {
                    let view = state_mut.form_view.clone();
                    drop(state_mut);
                    if let Some(view) = view {
                        Self::render_form_view(&form, &view, ctx);
                    }
                }
            }

            CoreUpdate::PluginActivated { id, name, icon } => {
                info!("Plugin activated: {} ({})", name, id);
                {
                    let mut state = state.borrow_mut();
                    state.active_plugin = Some((id, name));
                    state.navigation_depth = 0;
                    state.form_context = None;
                    state.form_data.clear();
                    state.form_config = None;
                    state.form_view = None;
                }

                // Reset selection when entering a plugin
                result_view.borrow().reset_selection();

                action_bar.set_mode(ActionBarMode::Plugin);
                action_bar.set_navigation_depth(0);
                action_bar.set_actions_visible(*action_bar_visible.borrow());

                // Update icon to plugin icon (or fallback to "extension")
                let plugin_icon = icon.unwrap_or_else(|| "extension".to_string());
                icon_label.set_label(&plugin_icon);
                icon_container.add_css_class("plugin-active");

                // Clear input when entering plugin
                search_entry.set_text("");
            }

            CoreUpdate::PluginDeactivated => {
                info!("Plugin deactivated");
                let (view_mode, compact_mode, empty_query) = {
                    let mut state_mut = state.borrow_mut();
                    state_mut.active_plugin = None;
                    state_mut.input_mode = "realtime".to_string();
                    state_mut.navigation_depth = 0;
                    state_mut.plugin_actions.clear();
                    state_mut.placeholder = DEFAULT_PLACEHOLDER.to_string();
                    state_mut.current_card = None;
                    state_mut.plugin_display_hint = None;
                    state_mut.form_context = None;
                    state_mut.form_data.clear();
                    state_mut.form_config = None;
                    state_mut.form_view = None;
                    // Reset to default view mode from config

                    let default_mode =
                        if config_watcher.theme().config.appearance.default_result_view == "grid" {
                            ResultViewMode::Grid
                        } else {
                            ResultViewMode::List
                        };
                    state_mut.view_mode = default_mode;
                    (
                        default_mode,
                        state_mut.compact_mode,
                        search_entry.text().trim().is_empty(),
                    )
                };

                action_bar.set_mode(ActionBarMode::Hints);
                action_bar.set_navigation_depth(0);
                action_bar.set_actions(Vec::new());
                Self::update_depth_indicator(depth_indicator, 0, config_watcher);

                // Reset icon to default
                icon_label.set_label("gavel");
                icon_container.remove_css_class("plugin-active");

                // Hide card/preview/form, clear and show appropriate view
                form_container.set_visible(false);
                result_card.clear();
                result_card.widget().set_visible(false);
                preview_revealer.set_reveal_child(false);
                preview_window.set_visible(false);
                result_view.borrow().clear();
                result_view.borrow_mut().set_mode(view_mode);
                result_view
                    .borrow()
                    .widget()
                    .set_visible(!(compact_mode && empty_query));

                // Animate width based on view mode
                let theme = config_watcher.theme();
                let target_width = match view_mode {
                    ResultViewMode::List => theme.config.sizes.search_width,
                    ResultViewMode::Grid => theme.config.appearance.grid.calculate_width(),
                };
                Self::animate_width(content_container, width_animation_source, target_width);

                // Clear input and reset placeholder when returning to main view
                search_entry.set_text("");
                search_entry.set_placeholder_text(Some(DEFAULT_PLACEHOLDER));
            }
            CoreUpdate::NavigationDepthChanged { depth } => {
                let mut state = state.borrow_mut();
                state.navigation_depth = depth as usize;
                state.pending_back = false;
                action_bar.set_navigation_depth(depth as usize);
                Self::update_depth_indicator(depth_indicator, depth as usize, config_watcher);
            }
            CoreUpdate::NavigateForward => {
                let mut state = state.borrow_mut();
                if !state.pending_back {
                    state.navigation_depth += 1;
                    action_bar.set_navigation_depth(state.navigation_depth);
                    Self::update_depth_indicator(
                        depth_indicator,
                        state.navigation_depth,
                        config_watcher,
                    );
                }
                state.pending_back = false;
            }
            CoreUpdate::NavigateBack => {
                let mut state = state.borrow_mut();
                state.navigation_depth = state.navigation_depth.saturating_sub(1);
                state.pending_back = false;
                action_bar.set_navigation_depth(state.navigation_depth);
                Self::update_depth_indicator(
                    depth_indicator,
                    state.navigation_depth,
                    config_watcher,
                );
            }
            CoreUpdate::PluginActionsUpdate { actions } => {
                let mut state = state.borrow_mut();
                state.plugin_actions.clone_from(&actions);
                action_bar.set_actions(actions.iter().map(ActionBarAction::from).collect());
            }
            CoreUpdate::Placeholder { placeholder } => {
                search_entry.set_placeholder_text(Some(&placeholder));
                state.borrow_mut().placeholder = placeholder;
            }
            CoreUpdate::Close => {
                // Check if we're showing window picker - if so, ignore the close
                let showing_picker = state.borrow().show_window_picker;
                if showing_picker {
                    debug!("Ignoring Close - window picker is active");
                    return;
                }

                // Check if we're minimized - if so, keep FAB visible
                let visibility = state_manager.visibility_state().visibility();
                let is_minimized = matches!(visibility, LauncherVisibility::Minimized);

                if is_minimized {
                    info!("Hide requested by daemon, but keeping FAB visible (minimized)");
                } else {
                    info!("Hide requested by daemon");
                    state_manager.visibility_state().close();
                    fab_window.hide();
                }

                window.set_visible(false);
                click_catcher_window.set_visible(false);
                preview_revealer.set_reveal_child(false);
                preview_window.set_visible(false);
                preview_panel.clear();
                form_container.set_visible(false);

                // Handle pending type text after window is hidden
                let pending_text = state.borrow_mut().pending_type_text.take();
                if let Some(text) = pending_text {
                    info!("Typing pending text after close");
                    // Spawn with delay to ensure window is fully hidden and focus returned
                    glib::timeout_add_local_once(
                        std::time::Duration::from_millis(150),
                        move || {
                            let _ = std::process::Command::new("ydotool")
                                .args(["type", "--clearmodifiers", "--", &text])
                                .stdin(std::process::Stdio::null())
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .spawn();
                        },
                    );
                }

                // Don't clear state here - preserve for potential restoration within 30s
                // State will be cleared when reopening if restore window has passed

                // Don't send LauncherClosed back - daemon already knows (it sent us Close)
            }
            CoreUpdate::Show => {
                info!("Show requested by daemon");

                // Check for state restoration
                let visibility_state = state_manager.visibility_state();
                let restore_window_ms = config_watcher
                    .theme()
                    .config
                    .behavior
                    .state_restore_window_ms;
                let should_restore = visibility_state.is_within_restore_window(restore_window_ms);
                visibility_state.open();

                // Find focused monitor and set windows to it
                let monitor = Self::get_focused_monitor(compositor);
                let monitor_name = monitor
                    .as_ref()
                    .and_then(gtk4::prelude::MonitorExt::connector)
                    .map(|s| s.to_string());
                debug!(
                    "Show: setting monitor to {:?} (geometry: {:?})",
                    monitor_name,
                    monitor.as_ref().map(gtk4::prelude::MonitorExt::geometry)
                );
                if let Some(ref mon) = monitor {
                    // Hide window before changing monitor (layer-shell requires this for monitor switch)
                    let was_visible = window.is_visible();
                    if was_visible {
                        debug!("Show: hiding window before monitor switch");
                        window.set_visible(false);
                    }
                    window.set_monitor(Some(mon));
                    preview_window.set_monitor(Some(mon));
                    // Note: click_catcher spans all monitors (no set_monitor call)
                    fab_window.set_monitor(mon);
                }

                // Calculate position for the current monitor
                let geometry = monitor.as_ref().map_or_else(
                    || {
                        gdk::Display::default()
                            .and_then(|d| d.monitors().item(0))
                            .and_downcast::<gdk::Monitor>()
                            .map_or(gdk::Rectangle::new(0, 0, 1920, 1080), |m| m.geometry())
                    },
                    gtk4::prelude::MonitorExt::geometry,
                );

                let screen_width = geometry.width();
                let screen_height = geometry.height();

                background.set_size_request(screen_width, screen_height);

                // Update drag state with current screen dimensions and monitor name
                {
                    let mut ds = drag_state.borrow_mut();
                    ds.screen_width = screen_width;
                    ds.screen_height = screen_height;
                    ds.monitor_name.clone_from(&monitor_name);
                }

                // Save last used monitor for FAB restoration on restart
                if let Some(ref name) = monitor_name {
                    state_manager.set_last_monitor(name);
                }

                // Calculate position from stored ratios (per-monitor)
                let position = monitor_name.as_deref().map_or_else(
                    || {
                        let launcher_state = state_manager.launcher();
                        crate::state::PositionRatio::new(
                            launcher_state.x_ratio,
                            launcher_state.y_ratio,
                        )
                    },
                    |name| state_manager.launcher_position_for_monitor(name),
                );
                let theme = config_watcher.theme();
                let launcher_width = theme.config.sizes.search_width;

                let left_margin = ((position.x_ratio * f64::from(screen_width))
                    - (f64::from(launcher_width) / 2.0)) as i32;
                let top_margin = (position.y_ratio * f64::from(screen_height)) as i32;

                // Clamp to ensure drag handle (on right side) remains accessible
                // min_left: allow left edge off-screen, but keep at least 60px of left side visible
                // max_left: keep right edge (where drag handle is) on screen
                let min_left = -(launcher_width - 60);
                let max_left = screen_width - launcher_width;
                let min_top = 0;
                let max_top = screen_height - 60;

                let clamped_left = left_margin.max(min_left).min(max_left);
                let clamped_top = top_margin.max(min_top).min(max_top);

                {
                    let mut ds = drag_state.borrow_mut();
                    ds.current_left = clamped_left;
                    ds.current_top = clamped_top;
                }

                layout_fixed.move_(
                    launcher_root,
                    f64::from(clamped_left),
                    f64::from(clamped_top),
                );

                // Hide FAB, show launcher
                fab_window.hide();
                // With the fullscreen launcher surface, click-away is handled internally.
                click_catcher_window.set_visible(false);
                window.set_visible(true);
                window.present();
                search_entry.grab_focus();

                *action_bar_visible.borrow_mut() = false;
                action_bar.set_actions_visible(false);
                caret_icon.set_label("chevron_left");

                // Reset view visibility based on current view mode
                let view_mode = state.borrow().view_mode;
                result_card.widget().set_visible(false);
                form_container.set_visible(false);
                result_view.borrow_mut().set_mode(view_mode);
                result_view.borrow().widget().set_visible(true);

                if should_restore {
                    // Within restore window - keep current UI state, just re-send query
                    info!("Restoring state (within {}ms window)", restore_window_ms);
                    let query = search_entry.text().to_string();
                    if let Err(e) = event_tx.send_blocking(CoreEvent::QueryChanged { query }) {
                        error!("Failed to send restored query: {}", e);
                    }
                } else {
                    // Restore window passed - clear state and start fresh
                    info!("Starting fresh (restore window passed)");

                    // Close any active plugin on daemon side
                    let had_plugin = state.borrow().active_plugin.is_some();
                    if had_plugin {
                        let _ = event_tx.send_blocking(CoreEvent::ClosePlugin);
                    }

                    search_entry.set_text("");
                    {
                        let mut state_mut = state.borrow_mut();
                        state_mut.results.clear();
                        state_mut.active_plugin = None;
                        state_mut.navigation_depth = 0;
                    }
                    result_view.borrow().clear();

                    // Reset action bar to hints mode
                    action_bar.set_mode(ActionBarMode::Hints);

                    // Reset icon
                    icon_label.set_label("gavel");
                    icon_container.remove_css_class("plugin-active");

                    if let Err(e) = event_tx.send_blocking(CoreEvent::QueryChanged {
                        query: String::new(),
                    }) {
                        error!("Failed to send initial query: {}", e);
                    }
                }
            }
            CoreUpdate::Toggle => {
                debug!("Toggle received from daemon");
                // Toggle with intuitive mode support
                let visibility_state = state_manager.visibility_state();
                let current_visibility = visibility_state.visibility();
                debug!("Toggle: current visibility = {:?}", current_visibility);

                match current_visibility {
                    LauncherVisibility::Open => {
                        // Currently open - check intuitive mode to decide close vs minimize
                        let theme = config_watcher.theme();
                        let click_action = &theme.config.behavior.click_outside_action;

                        let should_minimize = match click_action {
                            ClickOutsideAction::Close => false,
                            ClickOutsideAction::Minimize => true,
                            ClickOutsideAction::Intuitive => visibility_state.has_used_minimize(),
                        };

                        if should_minimize {
                            info!("Toggle: Minimizing to FAB (intuitive mode)");
                            let session = {
                                let state_ref = state.borrow();
                                SessionState {
                                    query: search_entry.text().to_string(),
                                    results: state_ref.results.clone(),
                                    active_plugin: state_ref.active_plugin.clone(),
                                }
                            };
                            visibility_state.save_session(session);
                            visibility_state.minimize();
                            window.set_visible(false);
                            click_catcher_window.set_visible(false);
                            preview_revealer.set_reveal_child(false);
                            preview_window.set_visible(false);
                            fab_window.show();
                        } else {
                            info!("Toggle: Closing launcher");
                            visibility_state.close();
                            window.set_visible(false);
                            click_catcher_window.set_visible(false);
                            preview_revealer.set_reveal_child(false);
                            preview_window.set_visible(false);
                            fab_window.hide();
                            // Don't clear UI state - will be kept or reset on next open based on time
                        }

                        if let Err(e) = event_tx.send_blocking(CoreEvent::LauncherClosed) {
                            error!("Failed to send LauncherClosed: {}", e);
                        }
                    }
                    LauncherVisibility::Minimized | LauncherVisibility::Closed => {
                        // Currently closed/minimized - open the launcher
                        info!("Toggle: Opening launcher");
                        let restore_window_ms = config_watcher
                            .theme()
                            .config
                            .behavior
                            .state_restore_window_ms;
                        let should_restore =
                            visibility_state.is_within_restore_window(restore_window_ms);
                        visibility_state.open();

                        // Find focused monitor
                        let monitor = Self::get_focused_monitor(compositor);
                        let monitor_name = monitor
                            .as_ref()
                            .and_then(gtk4::prelude::MonitorExt::connector)
                            .map(|s| s.to_string());
                        debug!(
                            "Toggle: setting monitor to {:?} (geometry: {:?})",
                            monitor_name,
                            monitor.as_ref().map(gtk4::prelude::MonitorExt::geometry)
                        );
                        if let Some(ref mon) = monitor {
                            // Hide window before changing monitor (layer-shell requires this for monitor switch)
                            let was_visible = window.is_visible();
                            if was_visible {
                                debug!("Toggle: hiding window before monitor switch");
                                window.set_visible(false);
                            }
                            window.set_monitor(Some(mon));
                            preview_window.set_monitor(Some(mon));
                            // Note: click_catcher spans all monitors (no set_monitor call)
                            fab_window.set_monitor(mon);
                        }

                        let geometry = monitor.as_ref().map_or_else(
                            || {
                                gdk::Display::default()
                                    .and_then(|d| d.monitors().item(0))
                                    .and_downcast::<gdk::Monitor>()
                                    .map_or(gdk::Rectangle::new(0, 0, 1920, 1080), |m| m.geometry())
                            },
                            gtk4::prelude::MonitorExt::geometry,
                        );

                        let screen_width = geometry.width();
                        let screen_height = geometry.height();

                        background.set_size_request(screen_width, screen_height);

                        {
                            let mut ds = drag_state.borrow_mut();
                            ds.screen_width = screen_width;
                            ds.screen_height = screen_height;
                            ds.monitor_name.clone_from(&monitor_name);
                        }

                        // Save last used monitor for FAB restoration on restart
                        if let Some(ref name) = monitor_name {
                            state_manager.set_last_monitor(name);
                        }

                        // Calculate position from stored ratios (per-monitor)
                        let position = monitor_name.as_deref().map_or_else(
                            || {
                                let launcher_state = state_manager.launcher();
                                crate::state::PositionRatio::new(
                                    launcher_state.x_ratio,
                                    launcher_state.y_ratio,
                                )
                            },
                            |name| state_manager.launcher_position_for_monitor(name),
                        );
                        let theme = config_watcher.theme();
                        let launcher_width = theme.config.sizes.search_width;

                        let left_margin = ((position.x_ratio * f64::from(screen_width))
                            - (f64::from(launcher_width) / 2.0))
                            as i32;
                        let top_margin = (position.y_ratio * f64::from(screen_height)) as i32;

                        // Clamp to ensure drag handle (on right side) remains accessible
                        // min_left: allow left edge off-screen, but keep at least 60px of left side visible
                        // max_left: keep right edge (where drag handle is) on screen
                        let min_left = -(launcher_width - 60);
                        let max_left = screen_width - launcher_width;
                        let min_top = 0;
                        let max_top = screen_height - 60;

                        let clamped_left = left_margin.max(min_left).min(max_left);
                        let clamped_top = top_margin.max(min_top).min(max_top);

                        {
                            let mut ds = drag_state.borrow_mut();
                            ds.current_left = clamped_left;
                            ds.current_top = clamped_top;
                        }

                        layout_fixed.move_(
                            launcher_root,
                            f64::from(clamped_left),
                            f64::from(clamped_top),
                        );

                        fab_window.hide();
                        click_catcher_window.set_visible(false);
                        window.set_visible(true);
                        window.present();
                        search_entry.grab_focus();

                        *action_bar_visible.borrow_mut() = false;
                        action_bar.set_actions_visible(false);
                        caret_icon.set_label("chevron_left");

                        let view_mode = state.borrow().view_mode;
                        result_card.widget().set_visible(false);
                        form_container.set_visible(false);
                        result_view.borrow_mut().set_mode(view_mode);
                        result_view.borrow().widget().set_visible(true);

                        if should_restore {
                            // Within restore window - keep current UI state, just re-send query
                            info!("Restoring state (within {}ms window)", restore_window_ms);
                            let query = search_entry.text().to_string();
                            if let Err(e) =
                                event_tx.send_blocking(CoreEvent::QueryChanged { query })
                            {
                                error!("Failed to send restored query: {}", e);
                            }
                        } else {
                            // Restore window passed - clear state and start fresh
                            info!("Starting fresh (restore window passed)");

                            // Close any active plugin on daemon side
                            if state.borrow().active_plugin.is_some() {
                                let _ = event_tx.send_blocking(CoreEvent::ClosePlugin);
                            }

                            search_entry.set_text("");
                            {
                                let mut state_mut = state.borrow_mut();
                                state_mut.results.clear();
                                state_mut.active_plugin = None;
                                state_mut.navigation_depth = 0;
                            }
                            result_view.borrow().clear();

                            // Reset action bar to hints mode
                            action_bar.set_mode(ActionBarMode::Hints);

                            // Reset icon
                            icon_label.set_label("gavel");
                            icon_container.remove_css_class("plugin-active");

                            if let Err(e) = event_tx.send_blocking(CoreEvent::QueryChanged {
                                query: String::new(),
                            }) {
                                error!("Failed to send initial query: {}", e);
                            }
                        }
                    }
                }
            }
            CoreUpdate::Error { message } => {
                error!("Error from daemon: {}", message);
                if is_daemon_disconnect_error(&message) {
                    Self::reset_state_after_daemon_disconnect(ctx);
                    error_dialog.show_non_blocking("Daemon disconnected", &message, window);
                } else {
                    error_dialog.show_error(&message, window);
                }
            }
            CoreUpdate::PluginStatusUpdate { plugin_id, status } => {
                debug!("Plugin status update for {}: {:?}", plugin_id, status);

                // Handle FAB override from this plugin
                let visibility_state = state_manager.visibility_state();
                visibility_state.update_fab_override(&plugin_id, status.fab.as_ref());
                let active = visibility_state.active_fab_override();
                fab_window.update_override(active.as_ref());

                // Show/hide FAB based on visibility state (plugin may have forced FAB visible via show_fab)
                match visibility_state.visibility() {
                    LauncherVisibility::Minimized => fab_window.show(),
                    LauncherVisibility::Closed | LauncherVisibility::Open => {}
                }

                let mut state_mut = state.borrow_mut();
                state_mut.plugin_statuses.insert(plugin_id.clone(), status);

                // Don't update results if launcher is not visible
                if !window.is_visible() {
                    return;
                }

                // Re-render results if any match this plugin_id
                let needs_update = state_mut
                    .results
                    .iter()
                    .any(|r| r.id == plugin_id || r.plugin_id.as_ref() == Some(&plugin_id));

                if needs_update {
                    // Re-merge status into results and update display
                    // Note: state.results contains original results without status merged
                    let merged_results: Vec<SearchResult> = state_mut
                        .results
                        .iter()
                        .map(|r| {
                            if matches!(r.result_type, ResultType::Plugin | ResultType::Recent)
                                && let Some(status) = state_mut.plugin_statuses.get(&r.id)
                            {
                                let mut merged = r.clone();
                                merged.badges.clone_from(&status.badges);
                                merged.chips.clone_from(&status.chips);
                                merged
                            } else {
                                r.clone()
                            }
                        })
                        .collect();
                    drop(state_mut);
                    // Use reactive update
                    let theme = config_watcher.theme();
                    result_view
                        .borrow()
                        .update_results_diff(&merged_results, &theme);
                }
            }
            CoreUpdate::Execute { action } => {
                info!("Execute action: {:?}", action);
                let should_close = Self::execute_action_view(
                    &action,
                    state,
                    compositor,
                    window,
                    result_view,
                    config_watcher,
                );

                if should_close {
                    window.set_visible(false);
                    preview_revealer.set_reveal_child(false);
                    preview_window.set_visible(false);
                    search_entry.set_text("");
                    result_view.borrow().clear();

                    // Clear pending app info
                    {
                        let mut state_mut = state.borrow_mut();
                        state_mut.pending_app_id = None;
                        state_mut.pending_app_id_fallback = None;
                        state_mut.pending_app_name = None;
                        state_mut.pending_app_icon = None;
                    }

                    // Notify daemon that launcher is now hidden
                    if let Err(e) = event_tx.send_blocking(CoreEvent::LauncherClosed) {
                        error!("Failed to send LauncherClosed: {}", e);
                    }
                }
            }
            CoreUpdate::AmbientUpdate { plugin_id, items } => {
                debug!("Ambient update from {}: {} items", plugin_id, items.len());
                let mut state_mut = state.borrow_mut();

                // Update ambient items for this plugin
                if items.is_empty() {
                    state_mut.ambient_items.remove(&plugin_id);
                } else {
                    state_mut.ambient_items.insert(plugin_id, items);
                }

                // Check if there are any ambient items across all plugins
                let has_ambient = !state_mut.ambient_items.is_empty();
                drop(state_mut);

                // Auto-show FAB if there are active ambient items
                let visibility_state = state_manager.visibility_state();
                visibility_state.set_has_ambient_items(has_ambient);

                // Convert to flat list with plugin IDs for both action bar and FAB
                let ambient_list: Vec<AmbientItemWithPlugin> = state
                    .borrow()
                    .ambient_items
                    .iter()
                    .flat_map(|(pid, items)| {
                        items.iter().map(move |item| AmbientItemWithPlugin {
                            item: item.clone(),
                            plugin_id: pid.clone(),
                        })
                    })
                    .collect();

                action_bar.set_ambient_items(&ambient_list);
                fab_window.set_ambient_items(&ambient_list);
            }
            CoreUpdate::ClearInput => {
                search_entry.set_text("");
            }
            CoreUpdate::FabUpdate { .. } => {
                // FAB updates are handled via PluginStatusUpdate with proper plugin_id.
                // This separate FabUpdate is legacy and would create orphaned "_system" overrides.
                debug!("Ignoring FabUpdate (handled via PluginStatusUpdate)");
            }
            CoreUpdate::Busy { busy } => {
                if busy {
                    icon_label.set_visible(false);
                    spinner.set_visible(true);
                } else {
                    spinner.set_visible(false);
                    icon_label.set_visible(true);
                }
            }
            CoreUpdate::PluginManagementChanged { active } => {
                state.borrow_mut().plugin_management = active;
                if active {
                    debug!("Plugin management mode activated");
                    icon_label.set_label("apps");
                    icon_container.add_css_class("plugin-management");
                } else {
                    debug!("Plugin management mode deactivated");
                    icon_label.set_label("gavel");
                    icon_container.remove_css_class("plugin-management");
                }
            }
            _ => {
                debug!("Unhandled update: {:?}", std::mem::discriminant(&update));
            }
        }
    }

    /// Display a form and set up live update handlers if needed
    fn show_form(form: &FormData, ctx: &UpdateContext) {
        let UpdateContext {
            state,
            result_view,
            result_card,
            preview_revealer,
            preview_window,
            form_container,
            event_tx,
            search_entry,
            ..
        } = ctx;

        let view = Rc::new(FormView::new(form));

        {
            let mut state_mut = state.borrow_mut();
            state_mut.form_context.clone_from(&form.context);
            state_mut.form_config = Some(form.clone());
            state_mut.form_view = Some(view.clone());
            state_mut.current_card = None;
        }

        result_view.borrow().widget().set_visible(false);
        result_card.widget().set_visible(false);
        preview_revealer.set_reveal_child(false);
        preview_window.set_visible(false);

        Self::render_form_view(form, &view, ctx);

        if form.live_update {
            let event_tx = event_tx.clone();
            let state = state.clone();
            view.set_on_change(move |field_id, value, all_values| {
                debug!("Form field changed: {} = {}", field_id, value);
                let context = state.borrow().form_context.clone();
                if let Err(e) = event_tx.send_blocking(CoreEvent::FormSubmitted {
                    form_data: all_values,
                    context,
                }) {
                    error!("Failed to send live form update: {}", e);
                }
            });
        }

        form_container.set_visible(true);
        search_entry.grab_focus();
    }

    /// Render form fields into the form container
    fn render_form_view(form: &FormData, view: &Rc<FormView>, ctx: &UpdateContext) {
        let UpdateContext {
            form_title,
            form_fields,
            form_submit,
            form_cancel,
            ..
        } = ctx;

        form_title.set_label(&form.title);

        while let Some(child) = form_fields.first_child() {
            form_fields.remove(&child);
        }

        form_fields.append(view.widget());

        form_submit.set_label(&form.submit_label);
        form_submit.set_visible(!form.live_update);

        if let Some(cancel_label) = &form.cancel_label {
            form_cancel.set_label(cancel_label);
            form_cancel.set_visible(true);
        } else {
            form_cancel.set_visible(!form.live_update);
        }
    }

    /// Launch a desktop file using native GIO API with proper startup notification
    /// and process detachment to survive hamr restarts
    fn launch_desktop_file(desktop_file: &str, display: Option<&gdk::Display>) {
        // Try to get DesktopAppInfo from the desktop file path/name
        let app_info = if desktop_file.contains('/') {
            // Full path: use new_from_filename
            gio::DesktopAppInfo::from_filename(desktop_file)
        } else {
            // Desktop ID: use new (expects "app.desktop" format)
            gio::DesktopAppInfo::new(desktop_file)
        };

        match app_info {
            Some(app_info) => {
                // Get launch context from display for proper startup notification
                let display = display.cloned().or_else(gdk::Display::default);
                let context = display.map(|d| d.app_launch_context());

                // Enhanced detachment to ensure apps survive hamr restarts
                // The issue with some apps (like Opencode) is that they may re-attach
                // to the parent session or have their own process management
                let user_setup: Option<Box<dyn FnOnce() + 'static>> = Some(Box::new(move || {
                    unsafe {
                        // Create new session and process group
                        libc::setsid();

                        // Also detach from process group to prevent SIGHUP propagation
                        // This is especially important for GUI apps that might spawn
                        // background processes
                        libc::setpgid(0, 0);

                        // Close standard file descriptors to prevent hanging
                        if libc::close(0) == -1 || libc::close(1) == -1 || libc::close(2) == -1 {
                            // Ignore errors - file descriptors might already be closed
                        }

                        // Redirect to /dev/null to prevent issues with broken pipes
                        let devnull =
                            libc::open(c"/dev/null".as_ptr().cast::<libc::c_char>(), libc::O_RDWR);
                        if devnull != -1 {
                            libc::dup2(devnull, 0);
                            libc::dup2(devnull, 1);
                            libc::dup2(devnull, 2);
                            if devnull > 2 {
                                libc::close(devnull);
                            }
                        }
                    }
                }));

                let mut pid_callback = |_info: &gio::DesktopAppInfo, _pid: glib::Pid| {};

                if let Err(e) = app_info.launch_uris_as_manager_with_fds(
                    &[],
                    context.as_ref(),
                    glib::SpawnFlags::empty(),
                    user_setup,
                    Some(&mut pid_callback),
                    None::<std::os::fd::OwnedFd>,
                    None::<std::os::fd::OwnedFd>,
                    None::<std::os::fd::OwnedFd>,
                ) {
                    warn!("Failed to launch {} via GIO: {}", desktop_file, e);
                }
            }
            None => {
                warn!("Could not find desktop file: {}", desktop_file);
            }
        }
    }

    /// Execute an action. Returns true if the launcher should close.
    // 1:1 ExecuteAction variant mapping - each arm performs specific action
    #[allow(clippy::too_many_lines)]
    fn execute_action_view(
        action: &hamr_rpc::ExecuteAction,
        state: &Rc<RefCell<AppState>>,
        compositor: &Rc<Compositor>,
        window: &gtk4::Window,
        result_view: &Rc<RefCell<ResultView>>,
        config_watcher: &ConfigWatcher,
    ) -> bool {
        use hamr_rpc::ExecuteAction;
        match action {
            ExecuteAction::Launch { desktop_file } => {
                // Focus-or-launch with fallback chain:
                // 1. Try StartupWMClass (app_id)
                // 2. Try desktop filename (app_id_fallback)
                // 3. Launch new instance
                let (pending_app_id, pending_app_id_fallback, pending_app_name) = {
                    let state_ref = state.borrow();
                    (
                        state_ref.pending_app_id.clone(),
                        state_ref.pending_app_id_fallback.clone(),
                        state_ref.pending_app_name.clone(),
                    )
                };

                debug!(
                    "Focus-or-launch: app_id={:?}, fallback={:?}, name={:?}",
                    pending_app_id, pending_app_id_fallback, pending_app_name
                );

                // Try to find windows using app_id or fallback
                let matching = if let Some(ref app_id) = pending_app_id {
                    if app_id.is_empty() {
                        None
                    } else {
                        let windows = compositor.find_windows_by_app_id(app_id);
                        debug!(
                            "Primary app_id '{}': found {} windows",
                            app_id,
                            windows.len()
                        );
                        if windows.is_empty() {
                            None
                        } else {
                            Some(windows)
                        }
                    }
                } else {
                    None
                };

                // If no windows found with primary, try fallback
                let matching = matching.or_else(|| {
                    if let Some(ref fallback) = pending_app_id_fallback {
                        if fallback.is_empty() {
                            None
                        } else {
                            let windows = compositor.find_windows_by_app_id(fallback);
                            debug!(
                                "Fallback app_id '{}': found {} windows",
                                fallback,
                                windows.len()
                            );
                            if windows.is_empty() {
                                None
                            } else {
                                Some(windows)
                            }
                        }
                    } else {
                        None
                    }
                });

                if let Some(windows) = matching {
                    if windows.len() == 1 {
                        // Single window, focus it directly
                        let win = &windows[0];
                        info!("Focusing existing window: {} ({})", win.title, win.id);
                        if !compositor.focus_window(&win.id) {
                            warn!("Failed to focus window, launching new instance");
                            Self::launch_desktop_file(
                                desktop_file,
                                Some(&WidgetExt::display(window)),
                            );
                        }
                    } else {
                        // Multiple windows, show window picker
                        info!("Multiple windows found ({}), showing picker", windows.len());
                        let mut state_mut = state.borrow_mut();
                        state_mut.window_picker_windows = windows;
                        state_mut.show_window_picker = true;
                        state_mut.pending_desktop_file = Some(desktop_file.clone());
                        drop(state_mut);

                        // Show the window picker UI
                        let app_name = pending_app_name.unwrap_or_else(|| {
                            pending_app_id
                                .or(pending_app_id_fallback)
                                .unwrap_or_else(|| "App".to_string())
                        });
                        let theme = config_watcher.theme();
                        Self::show_window_picker_view(
                            state,
                            compositor,
                            window,
                            result_view,
                            &app_name,
                            &theme,
                        );
                        return false; // Don't close - show window picker instead
                    }
                } else {
                    // No windows found with either app_id, launch new instance
                    info!("No existing windows found, launching: {}", desktop_file);
                    Self::launch_desktop_file(desktop_file, Some(&WidgetExt::display(window)));
                }
                true
            }
            ExecuteAction::OpenUrl { url } => {
                info!("Opening URL: {}", url);

                // Get the default browser's desktop file ID
                let Some(browser_desktop_id) = get_default_browser_desktop_id() else {
                    // No default browser, use xdg-open
                    let launcher = gtk4::UriLauncher::new(url);
                    launcher.launch(Some(window), gio::Cancellable::NONE, |result| {
                        if let Err(e) = result {
                            error!("Failed to open URL: {}", e);
                        }
                    });
                    return true;
                };

                // Check if any open window belongs to the default browser
                let browser_windows: Vec<CompositorWindow> = compositor
                    .list_windows()
                    .into_iter()
                    .filter(|w| window_is_default_browser(w, &browser_desktop_id))
                    .collect();

                if !browser_windows.is_empty() {
                    // Found default browser window - focus it, then use UriLauncher
                    // UriLauncher will open URL in the already-focused window/tab
                    info!(
                        "Focusing existing {} window and opening URL",
                        browser_desktop_id
                    );
                    compositor.focus_window(&browser_windows[0].id);
                }

                // Always use UriLauncher to open the URL
                let launcher = gtk4::UriLauncher::new(url);
                launcher.launch(Some(window), gio::Cancellable::NONE, |result| {
                    if let Err(e) = result {
                        error!("Failed to open URL: {}", e);
                    }
                });
                true
            }
            ExecuteAction::Open { path } => {
                info!("Opening path: {}", path);
                let file = gio::File::for_path(path);
                let launcher = gtk4::FileLauncher::new(Some(&file));
                launcher.launch(Some(window), gio::Cancellable::NONE, |result| {
                    if let Err(e) = result {
                        error!("Failed to open path: {}", e);
                    }
                });
                true
            }
            ExecuteAction::Copy { text } => {
                info!("Copying to clipboard");
                window.clipboard().set_text(text);
                true
            }
            ExecuteAction::Notify { message } => {
                info!("Notification: {}", message);
                if let Some(app) = window.application() {
                    let notification = gio::Notification::new("Hamr");
                    notification.set_body(Some(message));
                    app.send_notification(None, &notification);
                } else {
                    // Fallback to notify-send if no application context
                    let _ = std::process::Command::new("notify-send")
                        .args(["Hamr", message])
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn();
                }
                false // Don't close launcher on notification
            }
            ExecuteAction::TypeText { text } => {
                info!("Type text (pending): {}", text);
                state.borrow_mut().pending_type_text = Some(text.clone());
                true
            }
            ExecuteAction::PlaySound { sound } => {
                info!("Play sound: {}", sound);
                if let Some(path) = resolve_sound_path(sound) {
                    if let Err(e) = std::process::Command::new("paplay")
                        .arg(&path)
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn()
                    {
                        warn!(
                            "Failed to play sound '{}' ({}): {}",
                            sound,
                            path.display(),
                            e
                        );
                    }
                } else {
                    warn!("Sound not found: {}", sound);
                }
                true
            }
        }
    }

    /// Update depth indicator dots in the search input
    fn update_depth_indicator(
        depth_indicator: &gtk4::Box,
        depth: usize,
        config_watcher: &ConfigWatcher,
    ) {
        // Remove all existing dots
        while let Some(child) = depth_indicator.first_child() {
            depth_indicator.remove(&child);
        }

        if depth == 0 {
            depth_indicator.set_visible(false);
            return;
        }

        let colors = config_watcher.theme().colors.clone();
        let primary_color = colors.primary.clone();

        for _ in 0..depth {
            let dot = Self::create_depth_dot(&primary_color);
            depth_indicator.append(&dot);
        }

        depth_indicator.set_visible(true);
    }

    /// Create a single depth indicator dot (mini LED glow)
    #[allow(clippy::many_single_char_names)] // Graphics code uses conventional w,h,r,g,b names
    fn create_depth_dot(color: &str) -> gtk4::DrawingArea {
        let dot_size = 6;
        let drawing_area = gtk4::DrawingArea::builder()
            .width_request(dot_size)
            .height_request(dot_size)
            .valign(gtk4::Align::Center)
            .build();

        let color = color.to_string();
        drawing_area.set_draw_func(move |_, cr, width, height| {
            let w = f64::from(width);
            let h = f64::from(height);
            let center_x = w / 2.0;
            let center_y = h / 2.0;
            let radius = w.min(h) / 2.0;

            let (r, g, b) = Self::parse_hex_color(&color);

            // Soft glow
            let glow =
                cairo::RadialGradient::new(center_x, center_y, 0.0, center_x, center_y, radius);
            glow.add_color_stop_rgba(0.0, r, g, b, 1.0);
            glow.add_color_stop_rgba(0.6, r, g, b, 0.6);
            glow.add_color_stop_rgba(1.0, r, g, b, 0.0);
            let _ = cr.set_source(&glow);
            cr.arc(center_x, center_y, radius, 0.0, 2.0 * std::f64::consts::PI);
            let _ = cr.fill();
        });

        drawing_area
    }

    /// Parse hex color string to RGB components (0.0-1.0)
    fn parse_hex_color(hex: &str) -> (f64, f64, f64) {
        let hex = hex.trim_start_matches('#');
        if hex.len() >= 6 {
            let r = f64::from(u8::from_str_radix(&hex[0..2], 16).unwrap_or(255)) / 255.0;
            let g = f64::from(u8::from_str_radix(&hex[2..4], 16).unwrap_or(255)) / 255.0;
            let b = f64::from(u8::from_str_radix(&hex[4..6], 16).unwrap_or(255)) / 255.0;
            (r, g, b)
        } else {
            (1.0, 1.0, 1.0)
        }
    }

    /// Get the GDK monitor matching the compositor's focused output
    fn get_focused_monitor(compositor: &Compositor) -> Option<gdk::Monitor> {
        let output_name = compositor.get_focused_output()?;
        Self::get_monitor_by_name(&output_name)
    }

    /// Get a GDK monitor by its connector name
    fn get_monitor_by_name(name: &str) -> Option<gdk::Monitor> {
        let display = gdk::Display::default()?;
        let monitors = display.monitors();
        let name_lower = name.to_lowercase();

        for i in 0..monitors.n_items() {
            if let Some(monitor) = monitors.item(i).and_downcast::<gdk::Monitor>()
                && (monitor.connector().as_deref() == Some(name)
                    || monitor
                        .model()
                        .as_deref()
                        .is_some_and(|model| name_lower.contains(&model.to_lowercase()))
                    || monitor
                        .manufacturer()
                        .as_deref()
                        .is_some_and(|maker| name_lower.contains(&maker.to_lowercase())))
            {
                return Some(monitor);
            }
        }

        // Fallback to first monitor if no match found
        warn!("No monitor matched '{}', falling back to first", name);
        monitors.item(0).and_downcast::<gdk::Monitor>()
    }

    /// Smoothly animate container width to target value.
    /// If `on_tick` is provided, it will be called on each animation tick with the new width.
    // Animation math uses f64, GTK width requires i32
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    fn animate_width_with_callback<F>(
        container: &gtk4::Box,
        animation_source: &Rc<RefCell<Option<glib::SourceId>>>,
        target: i32,
        on_tick: Option<F>,
    ) where
        F: Fn(i32) + 'static,
    {
        const ANIMATION_DURATION_MS: u32 = 250;
        const ANIMATION_FPS: u32 = 60;
        const TICK_MS: u32 = 1000 / ANIMATION_FPS;

        // Cancel any existing animation
        if let Some(source_id) = animation_source.borrow_mut().take() {
            source_id.remove();
        }

        let current_width = container.width();
        if current_width == target || current_width <= 0 {
            // Either already at target or not yet realized
            container.set_width_request(target);
            if let Some(ref cb) = on_tick {
                cb(target);
            }
            return;
        }

        let start_width = f64::from(current_width);
        let end_width = f64::from(target);
        let total_ticks = f64::from(ANIMATION_DURATION_MS / TICK_MS);
        let tick_count = Rc::new(RefCell::new(0u32));

        let container = container.clone();
        let animation_source_inner = animation_source.clone();

        let source_id =
            glib::timeout_add_local(Duration::from_millis(u64::from(TICK_MS)), move || {
                let mut count = tick_count.borrow_mut();
                *count += 1;

                // Ease-out cubic: 1 - (1 - t)^3
                let t = (f64::from(*count) / total_ticks).min(1.0);
                let eased_t = 1.0 - (1.0 - t).powi(3);

                let new_width = start_width + (end_width - start_width) * eased_t;
                let new_width_i32 = new_width as i32;
                container.set_width_request(new_width_i32);

                if let Some(ref cb) = on_tick {
                    cb(new_width_i32);
                }

                if t >= 1.0 {
                    *animation_source_inner.borrow_mut() = None;
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });

        *animation_source.borrow_mut() = Some(source_id);
    }

    /// Smoothly animate container width to target value
    fn animate_width(
        container: &gtk4::Box,
        animation_source: &Rc<RefCell<Option<glib::SourceId>>>,
        target: i32,
    ) {
        Self::animate_width_with_callback(container, animation_source, target, None::<fn(i32)>);
    }

    /// Reposition preview window based on current launcher position and width.
    /// Keeps preview on the same side (right if fits, otherwise left).
    fn reposition_preview(
        preview_window: &gtk4::Window,
        preview_revealer: &gtk4::Revealer,
        launcher_left: i32,
        launcher_top: i32,
        screen_width: i32,
        launcher_width: i32,
        launcher_actual_width: i32,
    ) {
        if !preview_revealer.reveals_child() {
            return;
        }

        let preview_width = preview_design::WIDTH;
        let gap = 8;

        // Use the larger of animated width and actual width to prevent overlap
        // (GTK may expand container beyond width_request if content is wider)
        let effective_width = launcher_width.max(launcher_actual_width);

        let right_space = screen_width - (launcher_left + effective_width);
        let left_space = launcher_left;

        let show_on_right = right_space >= left_space;
        let preview_x = if show_on_right {
            launcher_left + effective_width + gap
        } else {
            (launcher_left - preview_width - gap).max(0)
        };

        preview_window.set_margin(Edge::Left, preview_x);
        preview_window.set_margin(Edge::Top, launcher_top);
    }

    /// Show the window picker for multiple windows of the same app
    fn show_window_picker_view(
        state: &Rc<RefCell<AppState>>,
        _compositor: &Rc<Compositor>,
        _window: &gtk4::Window,
        result_view: &Rc<RefCell<ResultView>>,
        app_name: &str,
        theme: &Theme,
    ) {
        // Convert windows to search results for display in the result list
        let (windows, app_icon) = {
            let state_ref = state.borrow();
            (
                state_ref.window_picker_windows.clone(),
                state_ref.pending_app_icon.clone(),
            )
        };

        // Use app icon if available, otherwise fall back to material icon
        let (icon, icon_type) = app_icon.map_or_else(
            || {
                (
                    Some("select_window".to_string()),
                    Some("material".to_string()),
                )
            },
            |i| (Some(i), Some("system".to_string())),
        );

        let results: Vec<SearchResult> = windows
            .iter()
            .map(|w| SearchResult {
                id: format!("__window__:{}", w.id),
                name: w.title.clone(),
                description: Some(format!("Workspace {}", w.workspace)),
                icon: icon.clone(),
                icon_type: icon_type.clone(),
                verb: Some("Focus".to_string()),
                result_type: ResultType::Normal,
                actions: vec![hamr_rpc::Action {
                    id: "close".to_string(),
                    name: "Close".to_string(),
                    icon: Some("close".to_string()),
                    icon_type: Some("material".to_string()),
                    keep_open: true,
                }],
                ..Default::default()
            })
            .chain(std::iter::once(SearchResult {
                id: "__window__:new".to_string(),
                name: format!("Open new {app_name}"),
                description: Some("Launch a new instance".to_string()),
                icon: Some("open_in_new".to_string()),
                icon_type: Some("material".to_string()),
                verb: Some("Open".to_string()),
                result_type: ResultType::Normal,
                ..Default::default()
            }))
            .collect();

        // Store results for action handling
        {
            let mut state_mut = state.borrow_mut();
            state_mut.results.clone_from(&results);
        }

        // Display in result view
        result_view.borrow().set_results(&results, theme);
    }

    /// Handle window picker selection
    fn handle_window_picker_action(
        state: &Rc<RefCell<AppState>>,
        compositor: &Rc<Compositor>,
        item_id: &str,
        action: Option<&str>,
    ) -> bool {
        let (windows, desktop_file) = {
            let state_ref = state.borrow();
            (
                state_ref.window_picker_windows.clone(),
                state_ref.pending_desktop_file.clone(),
            )
        };

        if item_id == "__window__:new" {
            // Launch new instance
            if let Some(desktop_file) = desktop_file {
                info!("Launching new instance: {}", desktop_file);
                Self::launch_desktop_file(&desktop_file, None);
            }
            Self::clear_window_picker_state(state);
            return true; // Close launcher
        }

        if let Some(window_id) = item_id.strip_prefix("__window__:") {
            if action == Some("close") {
                info!("Close window requested: {}", window_id);
                let remaining: Vec<_> = windows.into_iter().filter(|w| w.id != window_id).collect();

                if remaining.len() == 1 {
                    // Only one window left, focus it
                    if compositor.focus_window(&remaining[0].id) {
                        info!("Focused last remaining window");
                    }
                    Self::clear_window_picker_state(state);
                    return true;
                } else if remaining.is_empty() {
                    // No windows left, launch new instance
                    if let Some(desktop_file) = desktop_file {
                        Self::launch_desktop_file(&desktop_file, None);
                    }
                    Self::clear_window_picker_state(state);
                    return true;
                }
                // Update the picker with remaining windows
                let mut state_mut = state.borrow_mut();
                state_mut.window_picker_windows = remaining;
                return false; // Keep picker open
            }
            // Focus the selected window
            info!("Focusing window: {}", window_id);
            compositor.focus_window(window_id);
            Self::clear_window_picker_state(state);
            return true; // Close launcher
        }

        true // Default: close launcher
    }

    fn clear_window_picker_state(state: &Rc<RefCell<AppState>>) {
        let mut state_mut = state.borrow_mut();
        state_mut.show_window_picker = false;
        state_mut.window_picker_windows.clear();
        state_mut.pending_desktop_file = None;
        state_mut.pending_app_id = None;
        state_mut.pending_app_id_fallback = None;
        state_mut.pending_app_name = None;
        state_mut.pending_app_icon = None;
    }

    // Action counts are usize, Tab navigation uses i32 modulo, bounded by list size
    // Keyboard event handler - many key bindings for navigation, actions, and shortcuts
    #[allow(
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::too_many_lines
    )]
    fn setup_key_handlers(&self) {
        let window = self.window.clone();
        let click_catcher_window = self.click_catcher.window.clone();
        let preview_window = self.preview_window.window.clone();
        let preview_revealer = self.preview_window.revealer.clone();
        let result_view = self.result_view.clone();
        let search_entry = self.search_entry.clone();
        let state = self.state.clone();
        let keybinding_widget = self.keybinding_map.widget().clone();
        let rpc_sender = self.rpc.as_ref().map(super::rpc::RpcHandle::event_sender);
        let content_container = self.content_container.clone();
        let launcher_root = self.launcher_root.clone();
        let config_watcher = self.config_watcher.clone();
        let width_animation_source = self.width_animation_source.clone();
        let state_manager = self.state_manager.clone();
        let fab_window = self.fab_window.clone();
        let drag_state = self.drag_state.clone();

        let key_controller = gtk4::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, keyval, _keycode, modifier| {
            let ctrl = modifier.contains(gdk::ModifierType::CONTROL_MASK);
            let shift = modifier.contains(gdk::ModifierType::SHIFT_MASK);
            let alt = modifier.contains(gdk::ModifierType::ALT_MASK);

            {
                let mut s = state.borrow_mut();
                if s.show_keybinding_map {
                    s.show_keybinding_map = false;
                    keybinding_widget.set_visible(false);
                    if keyval != gdk::Key::Escape {
                        return glib::Propagation::Stop;
                    }
                }
            }

            // Helper to get current view mode
            let view_mode = state.borrow().view_mode;

            match keyval {
                // Ctrl+M: Minimize to FAB
                gdk::Key::m if ctrl && !shift => {
                    info!("Ctrl+M: Minimizing to FAB");
                    state_manager.set_has_used_minimize();
                    let visibility_state = state_manager.visibility_state();
                    let session = {
                        let state_ref = state.borrow();
                        SessionState {
                            query: search_entry.text().to_string(),
                            results: state_ref.results.clone(),
                            active_plugin: state_ref.active_plugin.clone(),
                        }
                    };
                    visibility_state.save_session(session);
                    visibility_state.minimize();
                    window.set_visible(false);
                    click_catcher_window.set_visible(false);
                    preview_revealer.set_reveal_child(false);
                    preview_window.set_visible(false);
                    fab_window.show();

                    if let Some(tx) = &rpc_sender {
                        let _ = tx.send_blocking(CoreEvent::LauncherClosed);
                    }
                    glib::Propagation::Stop
                }

                gdk::Key::Escape => {
                    let visibility_state = state_manager.visibility_state();

                    let theme = config_watcher.theme();
                    let click_action = &theme.config.behavior.click_outside_action;

                    let should_minimize = match click_action {
                        ClickOutsideAction::Close => false,
                        ClickOutsideAction::Minimize => true,
                        ClickOutsideAction::Intuitive => visibility_state.has_used_minimize(),
                    };

                    if should_minimize {
                        info!("Escape: Minimizing to FAB");
                        let session = {
                            let state_ref = state.borrow();
                            SessionState {
                                query: search_entry.text().to_string(),
                                results: state_ref.results.clone(),
                                active_plugin: state_ref.active_plugin.clone(),
                            }
                        };
                        visibility_state.save_session(session);
                        visibility_state.minimize();
                        window.set_visible(false);
                        click_catcher_window.set_visible(false);
                        preview_revealer.set_reveal_child(false);
                        preview_window.set_visible(false);
                        fab_window.show();
                    } else {
                        info!("Escape: Closing launcher");
                        visibility_state.close();
                        window.set_visible(false);
                        click_catcher_window.set_visible(false);
                        preview_revealer.set_reveal_child(false);
                        preview_window.set_visible(false);
                        fab_window.hide();
                        // Don't clear UI state - will be kept or reset on next open based on time
                    }

                    if let Some(tx) = &rpc_sender {
                        let _ = tx.send_blocking(CoreEvent::LauncherClosed);
                    }
                    glib::Propagation::Stop
                }

                // Ctrl+G: Toggle view mode
                gdk::Key::g if ctrl && !shift => {
                    let new_mode = {
                        let mut s = state.borrow_mut();
                        s.view_mode = match s.view_mode {
                            ResultViewMode::List => ResultViewMode::Grid,
                            ResultViewMode::Grid => ResultViewMode::List,
                        };
                        s.view_mode
                    };

                    // Switch view mode
                    result_view.borrow_mut().set_mode(new_mode);

                    // Animate width, repositioning preview during animation
                    let theme = config_watcher.theme();
                    let target_width = match new_mode {
                        ResultViewMode::List => theme.config.sizes.search_width,
                        ResultViewMode::Grid => theme.config.appearance.grid.calculate_width(),
                    };
                    let preview_window_anim = preview_window.clone();
                    let preview_revealer_anim = preview_revealer.clone();
                    let launcher_root_anim = launcher_root.clone();
                    let drag_state_anim = drag_state.clone();
                    Self::animate_width_with_callback(
                        &content_container,
                        &width_animation_source,
                        target_width,
                        Some(move |new_width| {
                            let ds = drag_state_anim.borrow();
                            Self::reposition_preview(
                                &preview_window_anim,
                                &preview_revealer_anim,
                                ds.current_left,
                                ds.current_top,
                                ds.screen_width,
                                new_width,
                                launcher_root_anim.width(),
                            );
                        }),
                    );

                    glib::Propagation::Stop
                }

                // Ctrl+C: Toggle compact mode
                gdk::Key::c if ctrl && !shift => {
                    let enabled = {
                        let mut s = state.borrow_mut();
                        s.compact_mode = !s.compact_mode;
                        s.compact_mode
                    };

                    info!("Compact mode toggled via keyboard: {}", enabled);
                    state_manager.set_compact_mode(enabled);

                    let empty_query = search_entry.text().trim().is_empty();
                    let should_hide_results = enabled && empty_query;
                    result_view
                        .borrow()
                        .widget()
                        .set_visible(!should_hide_results);

                    glib::Propagation::Stop
                }

                gdk::Key::Down => {
                    result_view.borrow().set_selected_action(-1);
                    result_view.borrow().select_down();
                    state.borrow_mut().selected_action_index = -1;
                    glib::Propagation::Stop
                }
                gdk::Key::j if ctrl && !shift => {
                    result_view.borrow().set_selected_action(-1);
                    result_view.borrow().select_down();
                    state.borrow_mut().selected_action_index = -1;
                    glib::Propagation::Stop
                }

                gdk::Key::Up => {
                    result_view.borrow().set_selected_action(-1);
                    result_view.borrow().select_up();
                    state.borrow_mut().selected_action_index = -1;
                    glib::Propagation::Stop
                }
                gdk::Key::k if ctrl && !shift => {
                    result_view.borrow().set_selected_action(-1);
                    result_view.borrow().select_up();
                    state.borrow_mut().selected_action_index = -1;
                    glib::Propagation::Stop
                }

                gdk::Key::h if ctrl && !shift => {
                    let mut s = state.borrow_mut();
                    if view_mode == ResultViewMode::Grid {
                        // Grid mode: navigate left (priority over plugin back)
                        drop(s);
                        result_view.borrow().set_selected_action(-1);
                        result_view.borrow().select_left();
                        state.borrow_mut().selected_action_index = -1;
                        glib::Propagation::Stop
                    } else if s.active_plugin.is_some() {
                        // In plugin: navigate back
                        if s.navigation_depth > 0 {
                            s.pending_back = true;
                            drop(s);
                            if let Some(tx) = &rpc_sender {
                                let _ = tx.send_blocking(CoreEvent::Back);
                            }
                        } else {
                            drop(s);
                            if let Some(tx) = &rpc_sender {
                                let _ = tx.send_blocking(CoreEvent::ClosePlugin);
                            }
                        }
                        glib::Propagation::Stop
                    } else {
                        glib::Propagation::Proceed
                    }
                }

                gdk::Key::l if ctrl && !shift => {
                    let s = state.borrow();
                    if view_mode == ResultViewMode::Grid {
                        // Grid mode: navigate right (priority over plugin forward)
                        drop(s);
                        result_view.borrow().set_selected_action(-1);
                        result_view.borrow().select_right();
                        state.borrow_mut().selected_action_index = -1;
                        glib::Propagation::Stop
                    } else if s.active_plugin.is_some() && !s.results.is_empty() {
                        // In plugin: forward/select action
                        let view = result_view.borrow();
                        let id = view.selected_id();
                        let action = if s.selected_action_index >= 0 {
                            view.selected_result().and_then(|r| {
                                r.actions
                                    .get(s.selected_action_index as usize)
                                    .map(|a| a.id.clone())
                            })
                        } else {
                            None
                        };
                        let plugin_id = s.active_plugin.as_ref().map(|(pid, _)| pid.clone());
                        drop(view);
                        if let Some(id) = id
                            && let Some(tx) = &rpc_sender
                        {
                            let _ = tx.send_blocking(CoreEvent::ItemSelected {
                                id,
                                action,
                                plugin_id,
                            });
                        }
                        glib::Propagation::Stop
                    } else {
                        glib::Propagation::Proceed
                    }
                }

                gdk::Key::Tab if !shift && !ctrl => {
                    let mut s = state.borrow_mut();
                    if let Some(result) = result_view.borrow().selected_result() {
                        let action_count = result.actions.len() as i32;
                        if action_count > 0 {
                            s.selected_action_index = (s.selected_action_index + 1) % action_count;
                            result_view
                                .borrow()
                                .set_selected_action(s.selected_action_index);
                        }
                    }
                    glib::Propagation::Stop
                }

                gdk::Key::ISO_Left_Tab | gdk::Key::Tab if shift && !ctrl => {
                    let mut s = state.borrow_mut();
                    if let Some(result) = result_view.borrow().selected_result() {
                        let action_count = result.actions.len() as i32;
                        if action_count > 0 {
                            s.selected_action_index = if s.selected_action_index <= 0 {
                                action_count - 1
                            } else {
                                s.selected_action_index - 1
                            };
                            result_view
                                .borrow()
                                .set_selected_action(s.selected_action_index);
                        }
                    }
                    glib::Propagation::Stop
                }

                gdk::Key::h | gdk::Key::H if ctrl && shift => {
                    if let Some(result) = result_view.borrow().selected_result()
                        && result.is_slider()
                        && let Some(slider) = result.slider_value()
                    {
                        let new_val = (slider.value - slider.step).max(slider.min);
                        if let Some(tx) = &rpc_sender {
                            let _ = tx.send_blocking(CoreEvent::SliderChanged {
                                id: result.id.clone(),
                                value: new_val,
                                plugin_id: result.plugin_id.clone(),
                            });
                        }
                    }
                    glib::Propagation::Stop
                }

                gdk::Key::l | gdk::Key::L if ctrl && shift => {
                    if let Some(result) = result_view.borrow().selected_result()
                        && result.is_slider()
                        && let Some(slider) = result.slider_value()
                    {
                        let new_val = (slider.value + slider.step).min(slider.max);
                        if let Some(tx) = &rpc_sender {
                            let _ = tx.send_blocking(CoreEvent::SliderChanged {
                                id: result.id.clone(),
                                value: new_val,
                                plugin_id: result.plugin_id.clone(),
                            });
                        }
                    }
                    glib::Propagation::Stop
                }

                gdk::Key::t | gdk::Key::T if ctrl && shift => {
                    if let Some(result) = result_view.borrow().selected_result()
                        && result.is_switch()
                    {
                        let current = result.slider_value().is_some_and(|v| v.value > 0.0);
                        if let Some(tx) = &rpc_sender {
                            let _ = tx.send_blocking(CoreEvent::SwitchToggled {
                                id: result.id.clone(),
                                value: !current,
                                plugin_id: result.plugin_id.clone(),
                            });
                        }
                    }
                    glib::Propagation::Stop
                }

                gdk::Key::Left => {
                    // Grid mode: navigate left
                    if view_mode == ResultViewMode::Grid {
                        result_view.borrow().set_selected_action(-1);
                        result_view.borrow().select_left();
                        state.borrow_mut().selected_action_index = -1;
                        return glib::Propagation::Stop;
                    }
                    // List mode: handle slider
                    if let Some(result) = result_view.borrow().selected_result()
                        && result.is_slider()
                    {
                        if let Some(slider) = result.slider_value() {
                            let new_val = (slider.value - slider.step).max(slider.min);
                            if let Some(tx) = &rpc_sender {
                                let _ = tx.send_blocking(CoreEvent::SliderChanged {
                                    id: result.id.clone(),
                                    value: new_val,
                                    plugin_id: result.plugin_id.clone(),
                                });
                            }
                        }
                        return glib::Propagation::Stop;
                    }
                    glib::Propagation::Proceed
                }

                gdk::Key::Right => {
                    // Grid mode: navigate right
                    if view_mode == ResultViewMode::Grid {
                        result_view.borrow().set_selected_action(-1);
                        result_view.borrow().select_right();
                        state.borrow_mut().selected_action_index = -1;
                        return glib::Propagation::Stop;
                    }
                    // List mode: handle slider
                    if let Some(result) = result_view.borrow().selected_result()
                        && result.is_slider()
                    {
                        if let Some(slider) = result.slider_value() {
                            let new_val = (slider.value + slider.step).min(slider.max);
                            if let Some(tx) = &rpc_sender {
                                let _ = tx.send_blocking(CoreEvent::SliderChanged {
                                    id: result.id.clone(),
                                    value: new_val,
                                    plugin_id: result.plugin_id.clone(),
                                });
                            }
                        }
                        return glib::Propagation::Stop;
                    }
                    glib::Propagation::Proceed
                }

                gdk::Key::u if alt && !ctrl => {
                    Self::trigger_item_action_view(&result_view, 0, rpc_sender.as_ref());
                    glib::Propagation::Stop
                }
                gdk::Key::i if alt && !ctrl => {
                    Self::trigger_item_action_view(&result_view, 1, rpc_sender.as_ref());
                    glib::Propagation::Stop
                }
                gdk::Key::o if alt && !ctrl => {
                    Self::trigger_item_action_view(&result_view, 2, rpc_sender.as_ref());
                    glib::Propagation::Stop
                }
                gdk::Key::p if alt && !ctrl => {
                    Self::trigger_item_action_view(&result_view, 3, rpc_sender.as_ref());
                    glib::Propagation::Stop
                }

                gdk::Key::question if !ctrl => {
                    let mut s = state.borrow_mut();
                    s.show_keybinding_map = !s.show_keybinding_map;
                    keybinding_widget.set_visible(s.show_keybinding_map);
                    glib::Propagation::Stop
                }

                _ => {
                    // Check plugin-defined action shortcuts
                    if Self::try_trigger_plugin_action_by_shortcut(
                        &state,
                        keyval,
                        ctrl,
                        shift,
                        alt,
                        rpc_sender.as_ref(),
                    ) {
                        glib::Propagation::Stop
                    } else {
                        glib::Propagation::Proceed
                    }
                }
            }
        });
        self.window.add_controller(key_controller);
    }

    fn trigger_item_action_view(
        result_view: &Rc<RefCell<ResultView>>,
        index: usize,
        rpc_sender: Option<&async_channel::Sender<CoreEvent>>,
    ) {
        if let Some(result) = result_view.borrow().selected_result()
            && let Some(action) = result.actions.get(index)
            && let Some(tx) = &rpc_sender
        {
            let _ = tx.send_blocking(CoreEvent::ItemSelected {
                id: result.id.clone(),
                action: Some(action.id.clone()),
                plugin_id: result.plugin_id.clone(),
            });
        }
    }

    /// Check if a keypress matches a plugin action's shortcut and trigger it.
    /// Returns true if a matching action was found and triggered.
    fn try_trigger_plugin_action_by_shortcut(
        state: &Rc<RefCell<AppState>>,
        keyval: gdk::Key,
        ctrl: bool,
        shift: bool,
        alt: bool,
        rpc_sender: Option<&async_channel::Sender<CoreEvent>>,
    ) -> bool {
        let s = state.borrow();
        for action in &s.plugin_actions {
            if let Some(shortcut) = &action.shortcut
                && crate::keybindings::shortcut_matches(shortcut, keyval, ctrl, shift, alt)
            {
                if let Some(tx) = rpc_sender {
                    let _ = tx.send_blocking(CoreEvent::PluginActionTriggered {
                        action_id: action.id.clone(),
                    });
                }
                return true;
            }
        }
        false
    }

    fn setup_focus(&self) {
        let search_entry = self.search_entry.clone();
        self.window.connect_show(move |_| {
            search_entry.grab_focus();
        });
    }

    pub fn run(&self) {
        // Check if user previously used minimize - if so, show FAB on startup
        let visibility_state = self.state_manager.visibility_state();
        if visibility_state.has_used_minimize() {
            info!("User has used minimize before - showing FAB on startup");
            visibility_state.minimize();
            self.fab_window.show();
        } else {
            info!("Launcher running in background, waiting for toggle");
            // Set initial state to Closed so first toggle opens the launcher
            visibility_state.hard_close();
        }

        // Restore pinned panels from persisted state
        let theme = self.config_watcher.theme();
        let app = self.window.application().expect("Application");
        self.pinned_panel_manager.restore(&app, &theme);
    }

    /// Load action bar hints from hamr-core config file
    fn load_action_bar_hints() -> Option<Vec<hamr_core::config::ActionBarHint>> {
        let config_path = Self::hamr_config_path();
        let config = hamr_core::config::Config::load(&config_path).ok()?;
        Some(config.action_bar_hints().to_vec())
    }

    /// Get the path to the hamr config file
    fn hamr_config_path() -> PathBuf {
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

    /// Update keybinding map with current prefix hints from config
    fn update_keybinding_prefixes(&self) {
        if let Some(hints) = Self::load_action_bar_hints() {
            self.keybinding_map.set_prefixes(&hints);
        }
    }
}

/// Resolve a sound name to a file path.
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
