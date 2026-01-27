//! Click-catcher window for detecting clicks outside the launcher.
//!
//! Uses a separate full-screen `layer-shell` surface on `Layer::Top` (below the launcher on Overlay).
//! This allows click-away-to-close without affecting the launcher's drag performance.

use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

/// Click-catcher window for detecting clicks outside the launcher.
pub(crate) struct ClickCatcher {
    pub(crate) window: gtk4::Window,
}

impl ClickCatcher {
    pub(crate) fn new(app: &gtk4::Application) -> Self {
        let window = gtk4::Window::builder()
            .application(app)
            .title("Hamr Click Catcher")
            .decorated(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_keyboard_mode(KeyboardMode::None);
        window.set_namespace(Some("hamr-click-catcher"));

        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        window.set_anchor(Edge::Bottom, true);

        // DrawingArea properly fills space and receives input events
        let content = gtk4::DrawingArea::builder()
            .css_classes(["click-catcher"])
            .hexpand(true)
            .vexpand(true)
            .build();
        content.set_draw_func(|_, _, _, _| {});
        window.set_child(Some(&content));

        Self { window }
    }
}
