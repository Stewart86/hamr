//! Kbd widget - keyboard shortcut hint component
//!
//! Displays a keyboard key or shortcut in a styled box, similar to <kbd> in HTML.
//! Used for showing keyboard shortcuts on buttons and in help text.

use super::design;
use gtk4::prelude::*;

pub struct KbdWidget {
    container: gtk4::Box,
}

impl KbdWidget {
    /// Create a new Kbd widget with the given key text
    ///
    /// # Arguments
    /// * `keys` - The key text to display (e.g., "Ctrl+Enter", "Esc", "Tab")
    pub fn new(keys: &str) -> Self {
        let container = gtk4::Box::builder()
            .css_classes(["kbd"])
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .build();

        let label = gtk4::Label::builder()
            .label(keys)
            .css_classes(["kbd-text"])
            .build();

        container.append(&label);

        Self { container }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }
}

pub fn kbd_css(theme: &crate::config::Theme) -> String {
    let colors = &theme.colors;

    let border = theme.scaled(1);
    let radius = theme.scaled(design::kbd::BORDER_RADIUS);
    let pad_h = theme.scaled(design::kbd::PADDING_H);
    let pad_v = theme.scaled(design::kbd::PADDING_V);
    let font_size = theme.scaled_font(design::kbd::FONT_SIZE);

    format!(
        r#"
        .kbd {{
            background-color: {surface_highest};
            border-radius: {radius}px;
            border: {border}px solid {outline};
            padding: {pad_v}px {pad_h}px;
        }}

        .kbd-text {{
            font-family: "JetBrains Mono", "Fira Code", monospace;
            font-size: {font_size}px;
            font-weight: 500;
            color: {on_surface_variant};
        }}
        "#,
        surface_highest = colors.surface_container_highest,
        outline = colors.outline,
        on_surface_variant = colors.on_surface_variant,
    )
}
