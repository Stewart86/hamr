//! Error dialog - A layer-shell window for displaying errors to the user
//!
//! Shows plugin errors, permission issues, validation errors, etc.
//! Auto-dismisses after a timeout or can be dismissed with Escape/click.

use crate::config::Theme;

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::Orientation;
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use super::design::{font, radius, spacing};

const AUTO_DISMISS_MS: u32 = 8000;
const DIALOG_WIDTH: i32 = 380;

pub struct ErrorDialog {
    window: gtk4::Window,
    title_label: gtk4::Label,
    message_label: gtk4::Label,
    css_provider: gtk4::CssProvider,
    auto_dismiss_source: Rc<RefCell<Option<glib::SourceId>>>,
}

impl ErrorDialog {
    pub fn new(app: &gtk4::Application, theme: &Theme) -> Self {
        let window = gtk4::Window::builder()
            .application(app)
            .title("Hamr Error")
            .decorated(false)
            .resizable(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::Exclusive);
        window.set_namespace(Some("hamr-error"));

        // No anchors - window centers on screen
        window.set_anchor(Edge::Top, false);
        window.set_anchor(Edge::Left, false);
        window.set_anchor(Edge::Right, false);
        window.set_anchor(Edge::Bottom, false);
        window.set_exclusive_zone(-1);

        let container = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .width_request(DIALOG_WIDTH)
            .css_classes(["error-dialog"])
            .build();

        let header = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .build();

        let error_icon = gtk4::Label::builder()
            .label("error")
            .css_classes(["material-icon", "error-icon"])
            .build();

        let title_label = gtk4::Label::builder()
            .label("Error")
            .css_classes(["error-title"])
            .hexpand(true)
            .halign(gtk4::Align::Start)
            .build();

        let dismiss_btn = gtk4::Button::builder()
            .css_classes(["error-dismiss-btn"])
            .build();

        let dismiss_icon = gtk4::Label::builder()
            .label("close")
            .css_classes(["material-icon", "error-dismiss-icon"])
            .build();
        dismiss_btn.set_child(Some(&dismiss_icon));

        header.append(&error_icon);
        header.append(&title_label);
        header.append(&dismiss_btn);

        let message_label = gtk4::Label::builder()
            .label("")
            .css_classes(["error-message"])
            .wrap(true)
            .max_width_chars(50)
            .halign(gtk4::Align::Start)
            .xalign(0.0)
            .build();

        container.append(&header);
        container.append(&message_label);

        window.set_child(Some(&container));

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

        let auto_dismiss_source: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));

        let dialog = Self {
            window,
            title_label,
            message_label,
            css_provider,
            auto_dismiss_source,
        };

        dialog.setup_handlers(&dismiss_btn);
        dialog
    }

    fn setup_handlers(&self, dismiss_btn: &gtk4::Button) {
        let window = self.window.clone();
        let auto_dismiss_source = self.auto_dismiss_source.clone();

        dismiss_btn.connect_clicked(move |_| {
            if let Some(source_id) = auto_dismiss_source.borrow_mut().take() {
                source_id.remove();
            }
            window.set_visible(false);
        });

        let key_controller = gtk4::EventControllerKey::new();
        let window = self.window.clone();
        let auto_dismiss_source = self.auto_dismiss_source.clone();

        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape || key == gdk::Key::Return {
                if let Some(source_id) = auto_dismiss_source.borrow_mut().take() {
                    source_id.remove();
                }
                window.set_visible(false);
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
        let border = theme.scaled(2);
        let dialog_radius = theme.scaled(radius::MD);
        let btn_size = theme.scaled(28);
        let font_title = theme.scaled_font(font::LG);
        let font_message = theme.scaled_font(font::MD);
        let font_icon = theme.scaled_font(font::XL + 4);
        let dismiss_icon_size = theme.scaled_font(font::MD);

        let error_color = "#f44336";
        let error_bg = "alpha(#f44336, 0.15)";
        let error_border = "alpha(#f44336, 0.4)";

        let css = format!(
            r#"
            .error-dialog {{
                background-color: {surface_container};
                border-radius: {dialog_radius}px;
                border: {border}px solid {error_border};
                padding: {pad}px;
                box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
            }}

            .error-icon {{
                font-family: "Material Symbols Rounded";
                font-size: {font_icon}px;
                color: {error_color};
            }}

            .error-title {{
                font-size: {font_title}px;
                font-weight: 600;
                color: {error_color};
            }}

            .error-message {{
                font-size: {font_message}px;
                color: {on_surface_variant};
                padding-left: {icon_offset}px;
            }}

            .error-dismiss-btn {{
                background-color: transparent;
                border: none;
                border-radius: {btn_radius}px;
                min-width: {btn_size}px;
                min-height: {btn_size}px;
                padding: {pad_sm}px;
            }}

            .error-dismiss-btn:hover {{
                background-color: {error_bg};
            }}

            .error-dismiss-icon {{
                font-family: "Material Symbols Rounded";
                font-size: {dismiss_icon_size}px;
                color: {on_surface_variant};
            }}
            "#,
            surface_container = colors.surface_container,
            on_surface_variant = colors.on_surface_variant,
            icon_offset = font_icon + 12,
            btn_radius = dialog_radius / 2,
        );

        provider.load_from_string(&css);
    }

    pub fn update_theme(&self, theme: &Theme) {
        Self::apply_css(&self.css_provider, theme);
    }

    /// Show the error dialog with a message, centered on the same monitor as launcher
    pub fn show(&self, title: &str, message: &str, launcher_window: &gtk4::Window) {
        // Ensure we're on the same monitor as the launcher
        if let Some(surface) = launcher_window.surface() {
            let display = surface.display();
            if let Some(monitor) = display.monitor_at_surface(&surface) {
                self.window.set_monitor(Some(&monitor));
            }
        }

        self.title_label.set_label(title);
        self.message_label.set_label(message);

        if let Some(source_id) = self.auto_dismiss_source.borrow_mut().take() {
            source_id.remove();
        }

        self.window.set_visible(true);
        self.window.grab_focus();

        let window = self.window.clone();
        let auto_dismiss_source = self.auto_dismiss_source.clone();

        let source_id = glib::timeout_add_local_once(
            std::time::Duration::from_millis(u64::from(AUTO_DISMISS_MS)),
            move || {
                auto_dismiss_source.borrow_mut().take();
                window.set_visible(false);
            },
        );

        *self.auto_dismiss_source.borrow_mut() = Some(source_id);
    }

    /// Show error with default "Error" title
    pub fn show_error(&self, message: &str, launcher_window: &gtk4::Window) {
        self.show("Error", message, launcher_window);
    }

    pub fn set_monitor(&self, monitor: &gdk::Monitor) {
        self.window.set_monitor(Some(monitor));
    }
}
