//! Chip widget - 18px height pill-shaped tag
//!
//! Chips display text with an optional icon, similar to badges
//! but with a pill shape and slightly different styling.
//! Supports custom text/icon color via `color` field.

use gtk4::Align;
use gtk4::glib;
use gtk4::prelude::*;
use hamr_types::Chip;

/// A pill-shaped chip widget (18px height)
pub struct ChipWidget {
    container: gtk4::Box,
}

impl ChipWidget {
    /// Create a new chip from Chip data
    pub fn new(chip: &Chip) -> Self {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(3)
            .valign(Align::Center)
            .css_classes(["chip"])
            .build();

        // Get custom color for text/icon (if provided)
        let custom_color = chip.color.as_deref();

        // Add icon if present - use Pango markup for custom color
        if let Some(icon) = &chip.icon {
            let icon_label = gtk4::Label::builder()
                .css_classes(["chip-icon", "material-icon"])
                .valign(Align::Center)
                .use_markup(true)
                .build();

            if let Some(color) = custom_color {
                icon_label.set_markup(&format!("<span foreground=\"{color}\">{icon}</span>"));
            } else {
                icon_label.set_label(icon);
            }
            container.append(&icon_label);
        }

        // Add text - use Pango markup for custom color
        let text_label = gtk4::Label::builder()
            .css_classes(["chip-text"])
            .valign(Align::Center)
            .use_markup(true)
            .build();

        if let Some(color) = custom_color {
            text_label.set_markup(&format!(
                "<span foreground=\"{}\">{}</span>",
                color,
                glib::markup_escape_text(&chip.text)
            ));
        } else {
            text_label.set_label(&chip.text);
        }
        container.append(&text_label);

        Self { container }
    }

    /// Get the underlying GTK widget
    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }
}

impl AsRef<gtk4::Widget> for ChipWidget {
    fn as_ref(&self) -> &gtk4::Widget {
        self.container.upcast_ref()
    }
}

/// Generate CSS for chip styling
pub fn chip_css(theme: &crate::config::Theme) -> String {
    use super::design::{font, radius, spacing};

    let colors = &theme.colors;

    // Pre-compute scaled values
    let min_height = theme.scaled(spacing::LG + spacing::XXXS); // 18 = 16 + 2
    let border = theme.scaled(1);
    let h_margin = theme.scaled(spacing::XS + 1); // 5 = 4 + 1

    format!(
        r#"
        box.chip {{
            min-height: {min_height}px;
            border-radius: {pill}px;
            background-color: {bg};
            border: {border}px solid {border_color};
            padding: 0;
            margin: 0;
        }}

        box.chip > label:first-child {{
            margin-left: {h_margin}px;
        }}

        box.chip > label:last-child {{
            margin-right: {h_margin}px;
        }}

        box.chip > label.chip-icon {{
            font-family: "Material Symbols Rounded";
            font-size: {font_icon}px;
            color: {text_color};
            margin-top: 0;
            margin-bottom: 0;
            padding: 0;
            min-width: 0;
            min-height: 0;
        }}

        box.chip > label.chip-text {{
            font-family: "Inter", "Google Sans Flex", sans-serif;
            font-size: {font_text}px;
            font-weight: 500;
            color: {text_color};
            margin-top: 0;
            margin-bottom: 0;
            padding: 0;
            min-width: 0;
            min-height: 0;
        }}
        "#,
        min_height = min_height,
        pill = radius::PILL,
        bg = colors.surface_container_highest,
        border = border,
        border_color = colors.outline,
        h_margin = h_margin,
        text_color = colors.on_surface,
        font_icon = theme.scaled_font(font::SM), // 11
        font_text = theme.scaled_font(font::XS), // 9
    )
}
