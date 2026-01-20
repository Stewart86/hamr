//! Floating Action Button (FAB) window
//!
//! A separate layer-shell window that appears when the launcher is minimized.
//! Displays hamr icon, ambient items, and optional plugin chips/badges.
//! Features:
//! - Drag anywhere to reposition
//! - Expand button to open launcher
//! - Close button to hide FAB and reset hasUsedMinimize preference
//! - Ambient items with dismiss support (same as action bar)

#![allow(dead_code)]

use crate::config::Theme;
use crate::state::StateManager;
use crate::widgets::design::fab as design;
use crate::widgets::{AmbientItemWithPlugin, AmbientItemsContainer, BadgeWidget, ChipWidget};
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use hamr_types::FabOverride;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use tracing::{debug, info};

type Callback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

/// Callback for ambient actions (`plugin_id`, `item_id`, `action_id`)
type AmbientActionCallback = Rc<RefCell<Option<Box<dyn Fn(&str, &str, &str)>>>>;
/// Callback for ambient dismiss (`plugin_id`, `item_id`)
type AmbientDismissCallback = Rc<RefCell<Option<Box<dyn Fn(&str, &str)>>>>;

pub struct FabWindow {
    window: gtk4::Window,
    main_box: gtk4::Box,
    icon: gtk4::Label,
    label: gtk4::Label,
    ambient_container: AmbientItemsContainer,
    chips_container: gtk4::Box,
    badges_container: gtk4::Box,
    expand_button: gtk4::Button,
    close_button: gtk4::Button,
    css_provider: gtk4::CssProvider,
    on_expand: Callback,
    on_close: Callback,
    on_ambient_action: AmbientActionCallback,
    on_ambient_dismiss: AmbientDismissCallback,
    state_manager: Rc<StateManager>,
    monitor_name: Rc<RefCell<Option<String>>>,
    monitor_geometry: Rc<Cell<(i32, i32, i32, i32)>>,
}

impl FabWindow {
    // Window construction with gesture callbacks sharing local state via Rc<Cell/RefCell>
    #[allow(clippy::cast_possible_truncation, clippy::too_many_lines)]
    pub fn new(app: &gtk4::Application, theme: &Theme, state_manager: Rc<StateManager>) -> Self {
        let window = gtk4::Window::builder()
            .application(app)
            .title("Hamr FAB")
            .decorated(false)
            .resizable(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::None);
        window.set_namespace(Some("hamr-fab"));

        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, false);
        window.set_anchor(Edge::Bottom, false);
        window.set_exclusive_zone(-1);

        let css_provider = gtk4::CssProvider::new();
        Self::apply_css(&css_provider, theme);
        if let Some(display) = gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &css_provider,
                gtk4::STYLE_PROVIDER_PRIORITY_USER,
            );
        }

        // Main container - layout: [icon] [label] [ambient] [chips/badges] [expand] [close]
        let main_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .css_classes(["fab-container"])
            .build();

        let icon = gtk4::Label::builder()
            .label("gavel")
            .css_classes(["material-icon", "fab-icon"])
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .build();

        let label = gtk4::Label::builder()
            .label("hamr")
            .css_classes(["fab-label"])
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .build();

        // Ambient items container (reuses same widget as action bar)
        let ambient_container = AmbientItemsContainer::new();

        let chips_container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(4)
            .css_classes(["fab-chips"])
            .build();

        let badges_container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(4)
            .css_classes(["fab-badges"])
            .build();

        // Expand button - opens the launcher
        let expand_button = gtk4::Button::builder()
            .css_classes(["fab-button"])
            .tooltip_text("Expand")
            .build();
        let expand_icon = gtk4::Label::builder()
            .label("open_in_full")
            .css_classes(["material-icon", "fab-button-icon"])
            .build();
        expand_button.set_child(Some(&expand_icon));

        // Close button - hides FAB and resets hasUsedMinimize
        let close_button = gtk4::Button::builder()
            .css_classes(["fab-button"])
            .tooltip_text("Close")
            .build();
        let close_icon = gtk4::Label::builder()
            .label("close")
            .css_classes(["material-icon", "fab-button-icon"])
            .build();
        close_button.set_child(Some(&close_icon));

        main_box.append(&icon);
        main_box.append(&label);
        main_box.append(ambient_container.widget());
        main_box.append(&chips_container);
        main_box.append(&badges_container);
        main_box.append(&expand_button);
        main_box.append(&close_button);

        window.set_child(Some(&main_box));
        window.set_visible(false);

        let on_expand: Callback = Rc::new(RefCell::new(None));
        let on_close: Callback = Rc::new(RefCell::new(None));
        let on_ambient_action: AmbientActionCallback = Rc::new(RefCell::new(None));
        let on_ambient_dismiss: AmbientDismissCallback = Rc::new(RefCell::new(None));
        let monitor_name: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let monitor_geometry = Rc::new(Cell::new((0, 0, 1920, 1080)));

        let is_dragging = Rc::new(Cell::new(false));
        let drag_start_margin = Rc::new(Cell::new((0, 0)));

        // Wire up ambient container callbacks
        {
            let on_action = on_ambient_action.clone();
            ambient_container.connect_action(move |plugin_id, item_id, action_id| {
                if let Some(ref cb) = *on_action.borrow() {
                    cb(plugin_id, item_id, action_id);
                }
            });
        }
        {
            let on_dismiss = on_ambient_dismiss.clone();
            ambient_container.connect_dismiss(move |plugin_id, item_id| {
                if let Some(ref cb) = *on_dismiss.borrow() {
                    cb(plugin_id, item_id);
                }
            });
        }

        // Drag gesture on the whole FAB
        let drag = gtk4::GestureDrag::new();
        {
            let is_dragging = is_dragging.clone();
            let drag_start = drag_start_margin.clone();
            let window_ref = window.clone();
            drag.connect_drag_begin(move |_, _, _| {
                is_dragging.set(true);
                let left = window_ref.margin(Edge::Left);
                let top = window_ref.margin(Edge::Top);
                drag_start.set((left, top));
            });
        }
        {
            let window_ref = window.clone();
            let drag_start = drag_start_margin.clone();
            let monitor_geom = monitor_geometry.clone();
            drag.connect_drag_update(move |_, offset_x, offset_y| {
                let (start_left, start_top) = drag_start.get();
                // Layer-shell margins are relative to the assigned monitor
                let (_mon_x, _mon_y, mon_w, mon_h) = monitor_geom.get();
                let margin = design::SCREEN_MARGIN;

                let window_width = window_ref.width();
                let window_height = window_ref.height();

                // Calculate bounds within the monitor (margins are monitor-relative)
                let min_x = margin;
                let max_x = mon_w - window_width - margin;
                let min_y = margin;
                let max_y = mon_h - window_height - margin;

                let new_left = (start_left + offset_x as i32).clamp(min_x, max_x.max(min_x));
                let new_top = (start_top + offset_y as i32).clamp(min_y, max_y.max(min_y));

                window_ref.set_margin(Edge::Left, new_left);
                window_ref.set_margin(Edge::Top, new_top);
            });
        }
        {
            let is_dragging_clone = is_dragging.clone();
            let window_ref = window.clone();
            let state_manager_clone = state_manager.clone();
            let monitor_name_clone = monitor_name.clone();
            let monitor_geom = monitor_geometry.clone();
            drag.connect_drag_end(move |_, _, _| {
                is_dragging_clone.set(false);

                // Layer-shell margins are relative to the assigned monitor
                let left = window_ref.margin(Edge::Left);
                let top = window_ref.margin(Edge::Top);
                let fab_width = window_ref.width().max(120);
                let (_mon_x, _mon_y, mon_w, mon_h) = monitor_geom.get();

                // Calculate ratio within monitor bounds (margins are already monitor-relative)
                let x_ratio = if mon_w > 0 {
                    (f64::from(left) + f64::from(fab_width) / 2.0) / f64::from(mon_w)
                } else {
                    0.5
                };
                let y_ratio = if mon_h > 0 {
                    f64::from(top) / f64::from(mon_h)
                } else {
                    0.9
                };

                // Save position per-monitor
                if let Some(ref name) = *monitor_name_clone.borrow() {
                    state_manager_clone.set_fab_position_for_monitor(
                        name,
                        x_ratio.clamp(0.0, 1.0),
                        y_ratio.clamp(0.0, 1.0),
                    );
                }
            });
        }
        main_box.add_controller(drag);

        // Expand button click
        let expand_cb = on_expand.clone();
        expand_button.connect_clicked(move |_| {
            if let Some(ref cb) = *expand_cb.borrow() {
                cb();
            }
        });

        // Close button click
        let close_cb = on_close.clone();
        close_button.connect_clicked(move |_| {
            if let Some(ref cb) = *close_cb.borrow() {
                cb();
            }
        });

        Self {
            window,
            main_box,
            icon,
            label,
            ambient_container,
            chips_container,
            badges_container,
            expand_button,
            close_button,
            css_provider,
            on_expand,
            on_close,
            on_ambient_action,
            on_ambient_dismiss,
            state_manager,
            monitor_name,
            monitor_geometry,
        }
    }

    fn apply_css(provider: &gtk4::CssProvider, theme: &Theme) {
        use crate::widgets::design::spacing;

        let colors = &theme.colors;
        let fonts = &theme.config.fonts;

        let css = format!(
            r#"
            window {{
                background-color: transparent;
            }}

            .fab-container {{
                background-color: {surface_container};
                border-radius: {radius}px;
                padding: {padding_v}px {padding_h}px;
                border: {border}px solid {primary};
                box-shadow: 0 {shadow_y}px {shadow_blur}px rgba(0, 0, 0, 0.3);
            }}

            .fab-icon {{
                font-family: "{icon_font}";
                font-size: {icon_size}px;
                color: {primary};
            }}

            .fab-label {{
                font-family: "{main_font}";
                font-size: {fab_label_size}px;
                font-weight: 500;
                color: {on_surface};
            }}

            .fab-chips, .fab-badges {{
                margin-left: {margin_start}px;
            }}

            .fab-button {{
                background-color: transparent;
                border: none;
                padding: {btn_padding}px;
                min-width: {button_size}px;
                min-height: {button_size}px;
                border-radius: 50%;
            }}

            .fab-button:hover {{
                background-color: alpha({on_surface}, 0.1);
            }}

            .fab-button-icon {{
                font-family: "{icon_font}";
                font-size: {fab_btn_icon_size}px;
                color: {on_surface_variant};
            }}
            "#,
            surface_container = colors.surface_container,
            primary = colors.primary,
            on_surface = colors.on_surface,
            on_surface_variant = colors.on_surface_variant,
            icon_font = fonts.icon,
            main_font = fonts.main,
            icon_size = theme.scaled_font(design::ICON_SIZE),
            button_size = theme.scaled(design::BUTTON_SIZE),
            radius = theme.scaled(design::BORDER_RADIUS),
            padding_v = theme.scaled(spacing::SM),
            padding_h = theme.scaled(spacing::MD),
            border = theme.scaled(1),
            shadow_y = theme.scaled(2),
            shadow_blur = theme.scaled(spacing::SM),
            margin_start = theme.scaled(spacing::XS),
            btn_padding = theme.scaled(spacing::XXXS),
            fab_label_size = theme.scaled_font(14),
            fab_btn_icon_size = theme.scaled_font(18),
        );

        provider.load_from_string(&css);
    }

    pub fn window(&self) -> &gtk4::Window {
        &self.window
    }

    pub fn set_monitor(&self, monitor: &gdk::Monitor) {
        self.window.set_monitor(Some(monitor));

        // Store monitor name for per-monitor position storage
        *self.monitor_name.borrow_mut() = monitor.connector().map(|s| s.to_string());

        let geom = monitor.geometry();
        self.monitor_geometry
            .set((geom.x(), geom.y(), geom.width(), geom.height()));

        self.apply_position_from_ratios();
    }

    // Position ratios are f64, GTK margins are i32, bounded by screen size
    #[allow(clippy::cast_possible_truncation)]
    fn apply_position_from_ratios(&self) {
        // Get position for current monitor
        let pos = self
            .monitor_name
            .borrow()
            .as_ref()
            .map_or_else(crate::state::PositionRatio::with_fab_defaults, |name| {
                self.state_manager.fab_position_for_monitor(name)
            });

        let (_mon_x, _mon_y, mon_w, mon_h) = self.monitor_geometry.get();

        let estimated_fab_width = 180;
        let estimated_fab_height = 44;

        // Calculate position within monitor bounds (margins are monitor-relative)
        let x = (pos.x_ratio * f64::from(mon_w)) as i32 - estimated_fab_width / 2;
        let y = (pos.y_ratio * f64::from(mon_h)) as i32;

        // Clamp to keep FAB within monitor bounds (with margin)
        let x = x
            .max(design::SCREEN_MARGIN)
            .min(mon_w - estimated_fab_width - design::SCREEN_MARGIN);
        let y = y
            .max(design::SCREEN_MARGIN)
            .min(mon_h - estimated_fab_height - design::SCREEN_MARGIN);

        info!(
            "FAB position: ratio=({:.2}, {:.2}), screen={}x{}, pos=({}, {})",
            pos.x_ratio, pos.y_ratio, mon_w, mon_h, x, y
        );

        self.window.set_margin(Edge::Left, x);
        self.window.set_margin(Edge::Top, y);
    }

    pub fn set_position(&self, x: i32, y: i32) {
        self.window.set_margin(Edge::Left, x);
        self.window.set_margin(Edge::Top, y);
    }

    pub fn show(&self) {
        info!("FAB show() called");
        self.apply_position_from_ratios();
        self.window.set_visible(true);
        self.window.present();
        info!("FAB window visible: {}", self.window.is_visible());
    }

    pub fn hide(&self) {
        self.window.set_visible(false);
    }

    pub fn is_visible(&self) -> bool {
        self.window.is_visible()
    }

    /// Connect callback for clicking the expand button (opens launcher)
    pub fn connect_clicked<F: Fn() + 'static>(&self, callback: F) {
        *self.on_expand.borrow_mut() = Some(Box::new(callback));
    }

    /// Connect callback for clicking the close button (resets hasUsedMinimize)
    pub fn connect_close<F: Fn() + 'static>(&self, callback: F) {
        *self.on_close.borrow_mut() = Some(Box::new(callback));
    }

    /// Connect callback for ambient item action (`plugin_id`, `item_id`, `action_id`)
    pub fn connect_ambient_action<F: Fn(&str, &str, &str) + 'static>(&self, f: F) {
        *self.on_ambient_action.borrow_mut() = Some(Box::new(f));
    }

    /// Connect callback for ambient item dismiss (`plugin_id`, `item_id`)
    pub fn connect_ambient_dismiss<F: Fn(&str, &str) + 'static>(&self, f: F) {
        *self.on_ambient_dismiss.borrow_mut() = Some(Box::new(f));
    }

    /// Set ambient items (with diffing support)
    pub fn set_ambient_items(&self, items: &[AmbientItemWithPlugin]) {
        self.ambient_container.set_items(items);

        // Hide label when we have ambient items
        let has_ambient = !items.is_empty();
        if has_ambient {
            self.label.set_visible(false);
        }

        debug!("FAB updated with {} ambient items", items.len());
    }

    pub fn update_theme(&self, theme: &Theme) {
        Self::apply_css(&self.css_provider, theme);
    }

    pub fn chips_container(&self) -> &gtk4::Box {
        &self.chips_container
    }

    pub fn badges_container(&self) -> &gtk4::Box {
        &self.badges_container
    }

    /// Update FAB display with chips and badges from an override
    pub fn update_override(&self, override_data: Option<&FabOverride>) {
        Self::clear_container(&self.chips_container);
        Self::clear_container(&self.badges_container);

        let has_ambient = !self.ambient_container.is_empty();

        let Some(data) = override_data else {
            // Show label only if no ambient items
            self.label.set_visible(!has_ambient);
            return;
        };

        // Hide label when we have override data or ambient items
        let has_content = !data.chips.is_empty() || !data.badges.is_empty();
        self.label.set_visible(!has_content && !has_ambient);

        // Use ChipWidget for consistent styling with main launcher
        for chip in &data.chips {
            let chip_widget = ChipWidget::new(chip);
            self.chips_container.append(chip_widget.widget());
        }

        // Use BadgeWidget for consistent styling with main launcher
        for badge in &data.badges {
            let badge_widget = BadgeWidget::new(badge);
            self.badges_container.append(badge_widget.widget());
        }

        debug!(
            "Updated FAB with {} chips, {} badges",
            data.chips.len(),
            data.badges.len()
        );
    }

    fn clear_container(container: &gtk4::Box) {
        while let Some(child) = container.first_child() {
            container.remove(&child);
        }
    }
}
