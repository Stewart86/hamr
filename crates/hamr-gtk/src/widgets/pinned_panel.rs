//! Pinned panel widget - A draggable, persistent preview window (sticky note style)
//!
//! Creates independent layer-shell windows for pinned previews that:
//! - Survive launcher close
//! - Persist across restarts
//! - Can be dragged to reposition
//! - Can be closed individually

use crate::config::Theme;
use crate::state::{PinnedPanelState, StateManager};

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk4::Orientation;
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use hamr_types::PreviewData;
use tracing::{debug, info};

use super::PreviewPanel;
use super::design::{font, preview_panel as preview_design, radius, spacing};

/// Callback type for close button clicks
type CloseCallback = Rc<RefCell<Option<Box<dyn Fn(&str)>>>>;

/// Callback type for position changes: (`id`, `x_ratio`, `y_ratio`)
type PositionCallback = Rc<RefCell<Option<Box<dyn Fn(&str, f64, f64)>>>>;

/// A pinned preview panel that exists as an independent window
pub struct PinnedPanel {
    /// Unique identifier
    id: String,
    /// The layer-shell window
    window: gtk4::Window,
    /// The preview panel widget (kept for future theme updates)
    #[allow(dead_code)]
    panel: Rc<PreviewPanel>,
    /// CSS provider for theming (kept for future theme updates)
    #[allow(dead_code)]
    css_provider: gtk4::CssProvider,
    /// Close callback
    on_close: CloseCallback,
    /// Position change callback
    on_position_changed: PositionCallback,
    /// Current screen dimensions (for drag calculations)
    screen_width: Cell<i32>,
    screen_height: Cell<i32>,
}

impl PinnedPanel {
    /// Create a new pinned panel from saved state
    // Position ratios are f64, GTK margins are i32, bounded by screen size
    #[allow(clippy::cast_possible_truncation)]
    pub fn new(
        app: &gtk4::Application,
        theme: &Theme,
        state: &PinnedPanelState,
        screen_width: i32,
        screen_height: i32,
        monitor: Option<&gdk::Monitor>,
    ) -> Self {
        let window = gtk4::Window::builder()
            .application(app)
            .title(format!("Hamr Pinned: {}", state.id))
            .decorated(false)
            .resizable(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_keyboard_mode(KeyboardMode::None);
        window.set_namespace(Some("hamr-pinned"));

        // Set monitor if provided
        if let Some(mon) = monitor {
            window.set_monitor(Some(mon));
        }

        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, false);
        window.set_anchor(Edge::Bottom, false);
        window.set_exclusive_zone(-1);

        // Main container with header and content
        let outer_container = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .css_classes(["pinned-panel-container"])
            .build();

        // Custom header with drag handle and close button
        let header = Self::build_header();
        outer_container.append(&header);

        // Preview panel content
        let panel = Rc::new(PreviewPanel::new());
        let max_height = theme.config.sizes.max_results_height;
        panel.set_max_height(max_height);

        // Wrap panel in ScrolledWindow
        let scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .propagate_natural_height(true)
            .propagate_natural_width(false)
            .max_content_height(max_height)
            .build();
        scroll.set_size_request(preview_design::WIDTH, -1);
        scroll.set_child(Some(panel.widget()));

        outer_container.append(&scroll);
        window.set_child(Some(&outer_container));

        // Calculate initial position from ratios
        let left = (state.position.x_ratio * f64::from(screen_width)) as i32;
        let top = (state.position.y_ratio * f64::from(screen_height)) as i32;
        window.set_margin(Edge::Left, left);
        window.set_margin(Edge::Top, top);

        // Apply CSS
        let css_provider = gtk4::CssProvider::new();
        Self::apply_css(&css_provider, theme);
        if let Some(display) = gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &css_provider,
                gtk4::STYLE_PROVIDER_PRIORITY_USER,
            );
        }

        // Set preview content
        panel.set_preview(&state.item_id, &state.preview);

        let pinned = Self {
            id: state.id.clone(),
            window,
            panel,
            css_provider,
            on_close: Rc::new(RefCell::new(None)),
            on_position_changed: Rc::new(RefCell::new(None)),
            screen_width: Cell::new(screen_width),
            screen_height: Cell::new(screen_height),
        };

        pinned.setup_drag(&header);
        pinned.setup_close_button(&header);

        pinned
    }

    fn build_header() -> gtk4::Box {
        let header = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .css_classes(["pinned-header"])
            .build();

        // Drag handle (left side, expands)
        let drag_handle = gtk4::Box::builder()
            .hexpand(true)
            .css_classes(["pinned-drag-handle"])
            .build();

        let drag_icon = gtk4::Label::builder()
            .label("drag_indicator")
            .css_classes(["material-icon", "pinned-drag-icon"])
            .halign(gtk4::Align::Start)
            .build();
        drag_handle.append(&drag_icon);

        // Close button (right side)
        let close_button = gtk4::Button::builder()
            .css_classes(["pinned-close-button"])
            .build();

        let close_icon = gtk4::Label::builder()
            .label("close")
            .css_classes(["material-icon", "pinned-close-icon"])
            .build();
        close_button.set_child(Some(&close_icon));

        header.append(&drag_handle);
        header.append(&close_button);

        header
    }

    // Drag offsets are f64, GTK margins are i32
    #[allow(clippy::cast_possible_truncation)]
    fn setup_drag(&self, header: &gtk4::Box) {
        let drag_handle = header
            .first_child()
            .and_downcast::<gtk4::Box>()
            .expect("drag handle");

        let gesture = gtk4::GestureDrag::new();
        let on_position_changed = self.on_position_changed.clone();
        let id = self.id.clone();

        // Wrap screen dimensions in Rc for sharing across closures
        let screen_w: Rc<Cell<i32>> = Rc::new(Cell::new(self.screen_width.get()));
        let screen_h: Rc<Cell<i32>> = Rc::new(Cell::new(self.screen_height.get()));

        let initial_pos_x: Rc<Cell<i32>> = Rc::new(Cell::new(0));
        let initial_pos_y: Rc<Cell<i32>> = Rc::new(Cell::new(0));

        let drag_start_x = initial_pos_x.clone();
        let drag_start_y = initial_pos_y.clone();
        let window_start = self.window.clone();

        gesture.connect_drag_begin(move |_, _, _| {
            drag_start_x.set(window_start.margin(Edge::Left));
            drag_start_y.set(window_start.margin(Edge::Top));
        });

        let update_start_x = initial_pos_x.clone();
        let update_start_y = initial_pos_y.clone();
        let width_for_update = screen_w.clone();
        let height_for_update = screen_h.clone();
        let window_update = self.window.clone();

        gesture.connect_drag_update(move |_, offset_x, offset_y| {
            let new_x = update_start_x.get() + offset_x as i32;
            let new_y = update_start_y.get() + offset_y as i32;

            let sw = width_for_update.get();
            let sh = height_for_update.get();

            let clamped_x = new_x.max(0).min(sw - 100);
            let clamped_y = new_y.max(0).min(sh - 100);

            window_update.set_margin(Edge::Left, clamped_x);
            window_update.set_margin(Edge::Top, clamped_y);
        });

        let window_end = self.window.clone();
        let width_for_end = screen_w.clone();
        let height_for_end = screen_h.clone();

        gesture.connect_drag_end(move |_, _, _| {
            let final_x = window_end.margin(Edge::Left);
            let final_y = window_end.margin(Edge::Top);

            let sw = width_for_end.get();
            let sh = height_for_end.get();

            if sw > 0 && sh > 0 {
                let x_ratio = f64::from(final_x) / f64::from(sw);
                let y_ratio = f64::from(final_y) / f64::from(sh);

                if let Some(ref cb) = *on_position_changed.borrow() {
                    cb(&id, x_ratio, y_ratio);
                }
            }
        });

        drag_handle.add_controller(gesture);
    }

    fn setup_close_button(&self, header: &gtk4::Box) {
        let close_button = header
            .last_child()
            .and_downcast::<gtk4::Button>()
            .expect("close button");

        let on_close = self.on_close.clone();
        let id = self.id.clone();
        let window = self.window.clone();

        close_button.connect_clicked(move |_| {
            info!("Pinned panel close clicked: {}", id);
            window.set_visible(false);
            if let Some(ref cb) = *on_close.borrow() {
                cb(&id);
            }
        });
    }

    fn apply_css(provider: &gtk4::CssProvider, theme: &Theme) {
        let colors = &theme.colors;

        let pad = theme.scaled(spacing::SM);
        let pad_xs = theme.scaled(spacing::XS);
        let border = theme.scaled(2); // Thicker border for pinned panels
        let panel_radius = theme.scaled(radius::MD);
        let btn_radius = theme.scaled(radius::SM);
        let font_icon = theme.scaled_font(font::MD);

        let css = format!(
            r"
            .pinned-panel-container {{
                background-color: {surface_container};
                border-radius: {panel_radius}px;
                border: {border}px solid {primary};
                box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
            }}

            .pinned-header {{
                padding: {pad_xs}px {pad}px;
                border-bottom: {border}px solid alpha({outline_variant}, 0.5);
            }}

            .pinned-drag-handle {{
                cursor: grab;
                padding: {pad_xs}px;
            }}

            .pinned-drag-icon {{
                font-size: {font_icon}px;
                color: {outline};
            }}

            .pinned-drag-handle:hover .pinned-drag-icon {{
                color: {on_surface};
            }}

            .pinned-close-button {{
                background: transparent;
                border: none;
                padding: {pad_xs}px;
                border-radius: {btn_radius}px;
                min-width: 0;
                min-height: 0;
            }}

            .pinned-close-button:hover {{
                background: alpha(#f44336, 0.15);
            }}

            .pinned-close-icon {{
                font-size: {font_icon}px;
                color: {outline};
            }}

            .pinned-close-button:hover .pinned-close-icon {{
                color: #f44336;
            }}
            ",
            surface_container = colors.surface_container,
            outline_variant = colors.outline_variant,
            primary = colors.primary,
            outline = colors.outline,
            on_surface = colors.on_surface,
        );

        provider.load_from_string(&css);
    }

    /// Get the panel ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Show the pinned panel
    pub fn show(&self) {
        self.window.set_visible(true);
        debug!("Showing pinned panel: {}", self.id);
    }

    /// Hide the pinned panel
    #[allow(dead_code)]
    pub fn hide(&self) {
        self.window.set_visible(false);
    }

    /// Check if visible
    #[allow(dead_code)]
    pub fn is_visible(&self) -> bool {
        self.window.is_visible()
    }

    /// Update screen dimensions (for drag bounds)
    #[allow(dead_code)]
    pub fn set_screen_size(&self, width: i32, height: i32) {
        self.screen_width.set(width);
        self.screen_height.set(height);
    }

    /// Set monitor for the window
    #[allow(dead_code)]
    pub fn set_monitor(&self, monitor: &gdk::Monitor) {
        self.window.set_monitor(Some(monitor));
        let geom = monitor.geometry();
        self.set_screen_size(geom.width(), geom.height());
    }

    /// Connect close callback
    pub fn connect_close<F: Fn(&str) + 'static>(&self, f: F) {
        *self.on_close.borrow_mut() = Some(Box::new(f));
    }

    /// Connect position changed callback
    pub fn connect_position_changed<F: Fn(&str, f64, f64) + 'static>(&self, f: F) {
        *self.on_position_changed.borrow_mut() = Some(Box::new(f));
    }

    /// Update theme
    #[allow(dead_code)]
    pub fn update_theme(&self, theme: &Theme) {
        Self::apply_css(&self.css_provider, theme);
    }

    /// Destroy the window
    #[allow(dead_code)]
    pub fn destroy(&self) {
        self.window.close();
    }
}

/// Manager for multiple pinned panels
pub struct PinnedPanelManager {
    panels: RefCell<Vec<Rc<PinnedPanel>>>,
    state_manager: Rc<StateManager>,
}

impl PinnedPanelManager {
    pub fn new(state_manager: Rc<StateManager>) -> Self {
        Self {
            panels: RefCell::new(Vec::new()),
            state_manager,
        }
    }

    /// Restore pinned panels from state on startup
    pub fn restore(&self, app: &gtk4::Application, theme: &Theme) {
        let saved_panels = self.state_manager.pinned_panels();
        info!("Restoring {} pinned panels", saved_panels.len());

        let display = gdk::Display::default();

        for state in saved_panels {
            // Find the monitor by connector name, or use primary
            let monitor = display.as_ref().and_then(|d| {
                if let Some(ref monitor_name) = state.monitor {
                    let monitors = d.monitors();
                    for i in 0..monitors.n_items() {
                        if let Some(mon) = monitors.item(i).and_downcast::<gdk::Monitor>()
                            && mon.connector().as_deref() == Some(monitor_name.as_str())
                        {
                            return Some(mon);
                        }
                    }
                }
                // Fallback to primary/first monitor
                d.monitors().item(0).and_downcast::<gdk::Monitor>()
            });

            let (screen_width, screen_height) = monitor.as_ref().map_or((1920, 1080), |m| {
                let geom = m.geometry();
                (geom.width(), geom.height())
            });

            let panel = Rc::new(PinnedPanel::new(
                app,
                theme,
                &state,
                screen_width,
                screen_height,
                monitor.as_ref(),
            ));

            self.setup_panel_callbacks(&panel);
            panel.show();
            self.panels.borrow_mut().push(panel);
        }
    }

    /// Create and show a new pinned panel at the specified absolute position
    // Widget positioning requires app, theme, item metadata, coordinates, and monitor info
    #[allow(clippy::too_many_arguments)]
    pub fn pin(
        &self,
        app: &gtk4::Application,
        theme: &Theme,
        item_id: &str,
        title: Option<String>,
        preview: &PreviewData,
        left: i32,
        top: i32,
        screen_width: i32,
        screen_height: i32,
        monitor: Option<&gdk::Monitor>,
    ) -> String {
        // Convert absolute position to ratios for persistence
        let x_ratio = if screen_width > 0 {
            f64::from(left) / f64::from(screen_width)
        } else {
            0.5
        };
        let y_ratio = if screen_height > 0 {
            f64::from(top) / f64::from(screen_height)
        } else {
            0.3
        };
        let monitor_name = monitor
            .and_then(gtk4::prelude::MonitorExt::connector)
            .map(|s| s.to_string());

        let state = PinnedPanelState::new(
            item_id.to_string(),
            title,
            preview.clone(),
            x_ratio,
            y_ratio,
            monitor_name,
        );
        let id = self.state_manager.add_pinned_panel(state.clone());

        let panel = Rc::new(PinnedPanel::new(
            app,
            theme,
            &state,
            screen_width,
            screen_height,
            monitor,
        ));

        self.setup_panel_callbacks(&panel);
        panel.show();
        self.panels.borrow_mut().push(panel);

        info!("Created new pinned panel: {}", id);
        id
    }

    fn setup_panel_callbacks(&self, panel: &Rc<PinnedPanel>) {
        // Close callback
        let state_manager = self.state_manager.clone();
        let panels = self.panels.clone();

        panel.connect_close(move |id| {
            state_manager.remove_pinned_panel(id);
            panels.borrow_mut().retain(|p| p.id() != id);
            info!("Pinned panel removed: {}", id);
        });

        // Position changed callback
        let state_manager = self.state_manager.clone();
        panel.connect_position_changed(move |id, x, y| {
            state_manager.update_pinned_panel_position(id, x, y);
        });
    }

    /// Remove a pinned panel by ID
    #[allow(dead_code)]
    pub fn remove(&self, id: &str) {
        if let Some(panel) = self.panels.borrow().iter().find(|p| p.id() == id) {
            panel.destroy();
        }
        self.panels.borrow_mut().retain(|p| p.id() != id);
        self.state_manager.remove_pinned_panel(id);
    }

    /// Update theme for all panels
    #[allow(dead_code)]
    pub fn update_theme(&self, theme: &Theme) {
        for panel in self.panels.borrow().iter() {
            panel.update_theme(theme);
        }
    }

    /// Get number of active pinned panels
    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.panels.borrow().len()
    }
}
