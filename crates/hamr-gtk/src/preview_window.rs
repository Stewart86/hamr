//! Preview window for displaying item previews
//!
//! A separate floating layer-shell window positioned adjacent to the launcher.
//! Uses a Revealer for smooth slide-in/out animation.

use crate::config::Theme;
use crate::widgets::design::preview_panel as preview_design;
use crate::widgets::design::search_bar as design;
use crate::widgets::{self, PreviewPanel};
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;

/// Preview window - a separate floating layer-shell window for displaying item previews.
/// Positioned adjacent to the launcher (like QML's detached preview panel).
/// Uses a Revealer for smooth slide-in/out animation.
pub(crate) struct PreviewWindow {
    pub(crate) window: gtk4::Window,
    pub(crate) panel: Rc<PreviewPanel>,
    pub(crate) revealer: gtk4::Revealer,
    /// Current item ID being displayed (for detecting content changes)
    pub(crate) current_item_id: Rc<RefCell<String>>,
}

impl PreviewWindow {
    /// Animation duration in milliseconds
    pub(crate) const ANIMATION_DURATION_MS: u32 = 180;

    pub(crate) fn new(app: &gtk4::Application, theme: &Theme) -> Self {
        let window = gtk4::Window::builder()
            .application(app)
            .title("Hamr Preview")
            .decorated(false)
            .resizable(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::None);
        window.set_namespace(Some("hamr-preview"));

        // Anchor to top-left for margin-based positioning
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, false);
        window.set_anchor(Edge::Bottom, false);
        window.set_exclusive_zone(-1);

        let panel = Rc::new(PreviewPanel::new());

        // Set max height to match launcher's max results height
        let max_height = theme.config.sizes.max_results_height;
        panel.set_max_height(max_height);

        // Wrap panel in a ScrolledWindow to handle overflow gracefully
        let scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .propagate_natural_height(true)
            .propagate_natural_width(false)
            .max_content_height(max_height)
            .css_classes(["preview-clip-box"])
            .build();
        scroll.set_size_request(preview_design::WIDTH, -1);
        scroll.set_child(Some(panel.widget()));

        // Wrap in Revealer for slide animation
        let revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideLeft)
            .transition_duration(Self::ANIMATION_DURATION_MS)
            .reveal_child(false)
            .child(&scroll)
            .build();

        window.set_child(Some(&revealer));

        // Apply CSS via display provider
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

        Self {
            window,
            panel,
            revealer,
            current_item_id: Rc::new(RefCell::new(String::new())),
        }
    }

    fn apply_css(provider: &gtk4::CssProvider, theme: &Theme) {
        let colors = &theme.colors;

        let preview_css = widgets::preview_panel::preview_panel_css(theme);

        // Window-specific styling
        let window_css = format!(
            r"
            window {{
                background-color: transparent;
            }}

            .preview-clip-box {{
                background-color: {surface_container_low};
                background: {surface_container_low};
                border-radius: {radius}px;
                border: 1px solid alpha({outline}, 0.18);
                box-shadow: inset 0 1px rgba(255, 255, 255, 0.08), inset 0 -1px rgba(0, 0, 0, 0.28);
            }}

            box.preview-panel {{
                background-color: transparent;
                background: transparent;
            }}
            ",
            surface_container_low = colors.surface_container_low,
            radius = design::RADIUS,
            outline = colors.outline,
        );

        let css = format!("{window_css}\n{preview_css}");
        provider.load_from_string(&css);
    }

    pub(crate) fn set_monitor(&self, monitor: &gdk::Monitor) {
        self.window.set_monitor(Some(monitor));
    }
}
