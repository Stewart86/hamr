//! Confirm dialog - A modal layer-shell window for dangerous action confirmation
//!
//! This creates a separate window (like `PreviewWindow`) that appears centered
//! over the launcher when a dangerous action needs confirmation.

use crate::config::Theme;

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::Orientation;
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use super::design::{font, radius, spacing};

/// Callback receives (`confirmed`, `action_id`) - `action_id` is the ID of the action that was confirmed/cancelled
type ConfirmCallback = Rc<RefCell<Option<Box<dyn Fn(bool, &str)>>>>;

/// Pending confirmation state
#[derive(Debug, Clone)]
pub struct PendingConfirm {
    pub action_id: String,
    #[allow(dead_code)]
    pub message: String,
}

pub struct ConfirmDialog {
    window: gtk4::Window,
    message_label: gtk4::Label,
    cancel_btn: gtk4::Button,
    confirm_btn: gtk4::Button,
    css_provider: gtk4::CssProvider,
    pending: Rc<RefCell<Option<PendingConfirm>>>,
    on_result: ConfirmCallback,
}

impl ConfirmDialog {
    pub fn new(app: &gtk4::Application, theme: &Theme) -> Self {
        let window = gtk4::Window::builder()
            .application(app)
            .title("Hamr Confirm")
            .decorated(false)
            .resizable(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::Exclusive);
        window.set_namespace(Some("hamr-confirm"));

        // Anchor to top-left for margin-based positioning
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, false);
        window.set_anchor(Edge::Bottom, false);
        window.set_exclusive_zone(-1);

        // Build the dialog content
        let container = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(16)
            .css_classes(["confirm-dialog"])
            .build();

        // Header with warning icon
        let header = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .build();

        let warning_icon = gtk4::Label::builder()
            .label("warning")
            .css_classes(["material-icon", "confirm-warning-icon"])
            .build();

        let title_label = gtk4::Label::builder()
            .label("Confirm Action")
            .css_classes(["confirm-title"])
            .hexpand(true)
            .halign(gtk4::Align::Start)
            .build();

        header.append(&warning_icon);
        header.append(&title_label);

        // Message
        let message_label = gtk4::Label::builder()
            .label("Are you sure you want to proceed?")
            .css_classes(["confirm-message"])
            .wrap(true)
            .max_width_chars(40)
            .halign(gtk4::Align::Start)
            .build();

        // Button row
        let button_row = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .halign(gtk4::Align::End)
            .build();

        let cancel_btn = gtk4::Button::builder()
            .label("Cancel")
            .css_classes(["confirm-button", "confirm-cancel"])
            .build();

        let confirm_btn = gtk4::Button::builder()
            .label("Confirm")
            .css_classes(["confirm-button", "confirm-action"])
            .build();

        button_row.append(&cancel_btn);
        button_row.append(&confirm_btn);

        container.append(&header);
        container.append(&message_label);
        container.append(&button_row);

        window.set_child(Some(&container));

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

        window.set_visible(false);

        let pending: Rc<RefCell<Option<PendingConfirm>>> = Rc::new(RefCell::new(None));
        let on_result: ConfirmCallback = Rc::new(RefCell::new(None));

        let dialog = Self {
            window,
            message_label,
            cancel_btn,
            confirm_btn,
            css_provider,
            pending,
            on_result,
        };

        dialog.setup_handlers();
        dialog
    }

    fn setup_handlers(&self) {
        let window = self.window.clone();
        let pending = self.pending.clone();
        let on_result = self.on_result.clone();

        self.cancel_btn.connect_clicked(move |_| {
            let action_id = pending
                .borrow_mut()
                .take()
                .map(|p| p.action_id)
                .unwrap_or_default();
            window.set_visible(false);
            if let Some(ref cb) = *on_result.borrow() {
                cb(false, &action_id);
            }
        });

        let window = self.window.clone();
        let pending = self.pending.clone();
        let on_result = self.on_result.clone();

        self.confirm_btn.connect_clicked(move |_| {
            let action_id = pending
                .borrow_mut()
                .take()
                .map(|p| p.action_id)
                .unwrap_or_default();
            window.set_visible(false);
            if let Some(ref cb) = *on_result.borrow() {
                cb(true, &action_id);
            }
        });

        // Handle Escape key to cancel
        let key_controller = gtk4::EventControllerKey::new();
        let window = self.window.clone();
        let pending = self.pending.clone();
        let on_result = self.on_result.clone();

        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                let action_id = pending
                    .borrow_mut()
                    .take()
                    .map(|p| p.action_id)
                    .unwrap_or_default();
                window.set_visible(false);
                if let Some(ref cb) = *on_result.borrow() {
                    cb(false, &action_id);
                }
                return gdk::glib::Propagation::Stop;
            }
            gdk::glib::Propagation::Proceed
        });

        self.window.add_controller(key_controller);
    }

    fn apply_css(provider: &gtk4::CssProvider, theme: &Theme) {
        let colors = &theme.colors;

        let pad = theme.scaled(spacing::MD);
        let pad_sm = theme.scaled(spacing::SM);
        let border = theme.scaled(1);
        let dialog_radius = theme.scaled(radius::MD);
        let btn_radius = theme.scaled(radius::SM);
        let font_title = theme.scaled_font(font::LG);
        let font_message = theme.scaled_font(font::MD);
        let font_button = theme.scaled_font(font::SM);
        let font_icon = theme.scaled_font(font::XL + 4);

        let css = format!(
            r#"
            window {{
                background-color: transparent;
            }}

            .confirm-dialog {{
                background-color: {surface_container};
                border-radius: {dialog_radius}px;
                border: {border}px solid {outline_variant};
                padding: {pad}px;
                min-width: 280px;
                box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
            }}

            .confirm-warning-icon {{
                font-family: "Material Symbols Rounded";
                font-size: {font_icon}px;
                color: #f44336;
            }}

            .confirm-title {{
                font-size: {font_title}px;
                font-weight: 600;
                color: {on_surface};
            }}

            .confirm-message {{
                font-size: {font_message}px;
                color: {on_surface_variant};
            }}

            .confirm-button {{
                font-size: {font_button}px;
                padding: {pad_sm}px {pad}px;
                border-radius: {btn_radius}px;
                border: none;
                min-width: 80px;
            }}

            .confirm-cancel {{
                background-color: {surface_container_high};
                color: {on_surface};
            }}

            .confirm-cancel:hover {{
                background-color: {surface_container_highest};
            }}

            .confirm-action {{
                background-color: alpha(#f44336, 0.2);
                color: #f44336;
            }}

            .confirm-action:hover {{
                background-color: alpha(#f44336, 0.3);
            }}
            "#,
            surface_container = colors.surface_container,
            surface_container_high = colors.surface_container_high,
            surface_container_highest = colors.surface_container_highest,
            outline_variant = colors.outline_variant,
            on_surface = colors.on_surface,
            on_surface_variant = colors.on_surface_variant,
        );

        provider.load_from_string(&css);
    }

    pub fn update_theme(&self, theme: &Theme) {
        Self::apply_css(&self.css_provider, theme);
    }

    /// Show the confirmation dialog positioned relative to the launcher
    pub fn show(
        &self,
        action_id: &str,
        message: &str,
        launcher_x: i32,
        launcher_y: i32,
        launcher_width: i32,
    ) {
        self.message_label.set_label(message);
        *self.pending.borrow_mut() = Some(PendingConfirm {
            action_id: action_id.to_string(),
            message: message.to_string(),
        });

        // Position centered below launcher (or centered on launcher)
        let dialog_width = 320; // approximate
        let x = launcher_x + (launcher_width - dialog_width) / 2;
        let y = launcher_y + 60; // offset below search bar

        self.window.set_margin(Edge::Left, x.max(0));
        self.window.set_margin(Edge::Top, y.max(0));

        self.window.set_visible(true);
        self.confirm_btn.grab_focus();
    }

    /// Hide the dialog
    #[allow(dead_code)]
    pub fn hide(&self) {
        self.pending.borrow_mut().take();
        self.window.set_visible(false);
    }

    /// Check if dialog is visible
    #[allow(dead_code)]
    pub fn is_visible(&self) -> bool {
        self.window.is_visible()
    }

    /// Get the pending action ID (if any)
    #[allow(dead_code)]
    pub fn pending_action_id(&self) -> Option<String> {
        self.pending.borrow().as_ref().map(|p| p.action_id.clone())
    }

    /// Connect callback for when user confirms or cancels
    /// Callback receives (`confirmed`, `action_id`)
    pub fn connect_result<F: Fn(bool, &str) + 'static>(&self, f: F) {
        *self.on_result.borrow_mut() = Some(Box::new(f));
    }

    /// Set the monitor for the dialog window
    pub fn set_monitor(&self, monitor: &gdk::Monitor) {
        self.window.set_monitor(Some(monitor));
    }

    /// Present the window (bring to front)
    #[allow(dead_code)]
    pub fn present(&self) {
        self.window.present();
    }
}
