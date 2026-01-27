//! Badge widget - 20x20px circular badge
//!
//! Supports:
//! - Material icon badges
//! - Text badges (1-3 characters)
//!
//! Custom colors via `badge.color` apply to text/icon foreground,
//! matching QML's `textColor` property.

use super::design;
use gtk4::Align;
use gtk4::glib;
use gtk4::prelude::*;
use hamr_types::Badge;

/// A circular badge widget (20x20px)
pub struct BadgeWidget {
    container: gtk4::Box,
}

impl BadgeWidget {
    /// Create a new badge from Badge data
    pub fn new(badge: &Badge) -> Self {
        // Get custom color for text/icon (QML's textColor property)
        let custom_color = badge.color.as_deref();

        // Create box container with fixed 20x20 size
        let container = gtk4::Box::builder()
            .css_classes(["badge"])
            .halign(Align::Center)
            .valign(Align::Center)
            .hexpand(false)
            .vexpand(false)
            .build();

        // Set exact size using set_size_request
        container.set_size_request(design::badge::SIZE, design::badge::SIZE);

        // Inner size accounting for border
        let inner_size = design::badge::SIZE - (design::badge::BORDER_WIDTH * 2);

        // Determine content type and create label
        if let Some(icon) = &badge.icon {
            // Material icon badge
            let label = gtk4::Label::builder()
                .css_classes(["badge-content", "badge-icon", "material-icon"])
                .halign(Align::Center)
                .valign(Align::Center)
                .hexpand(false)
                .vexpand(false)
                .width_request(inner_size)
                .height_request(inner_size)
                .use_markup(true)
                .build();

            if let Some(color) = custom_color {
                label.set_markup(&format!("<span foreground=\"{color}\">{icon}</span>"));
            } else {
                label.set_label(icon);
            }
            container.append(&label);
        } else if let Some(text) = &badge.text {
            // Text badge - font size class depends on character count
            let size_class = match text.chars().count() {
                1 => "badge-text-1",
                2 => "badge-text-2",
                _ => "badge-text-3",
            };
            let label = gtk4::Label::builder()
                .css_classes(["badge-content", "badge-text", size_class])
                .halign(Align::Center)
                .valign(Align::Center)
                .hexpand(false)
                .vexpand(false)
                .width_request(inner_size)
                .height_request(inner_size)
                .use_markup(true)
                .build();

            let display_text = text.chars().take(3).collect::<String>().to_uppercase();
            if let Some(color) = custom_color {
                label.set_markup(&format!(
                    "<span foreground=\"{}\">{}</span>",
                    color,
                    glib::markup_escape_text(&display_text)
                ));
            } else {
                label.set_label(&display_text);
            }
            container.append(&label);
        }

        Self { container }
    }

    /// Get the underlying GTK widget
    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }
}

impl AsRef<gtk4::Widget> for BadgeWidget {
    fn as_ref(&self) -> &gtk4::Widget {
        self.container.upcast_ref()
    }
}

/// Generate CSS for badge styling
pub fn badge_css(theme: &crate::config::Theme) -> String {
    let colors = &theme.colors;

    // Pre-compute scaled values
    let border = theme.scaled(design::badge::BORDER_WIDTH);
    let inner_size = theme.scaled(design::badge::SIZE) - (border * 2);
    let radius = theme.scaled(design::badge::RADIUS);

    format!(
        r#"
        /* Badge container - exact 20x20 circle */
        box.badge {{
            min-width: {inner_size}px;
            min-height: {inner_size}px;
            border-radius: {radius}px;
            background-color: {bg};
            border: {border}px solid {border_color};
            padding: 0;
            margin: 0;
        }}

        /* Content label inside badge */
        box.badge > label.badge-content {{
            padding: 0;
            margin: 0;
            min-width: {inner_size}px;
            min-height: {inner_size}px;
        }}

        /* Icon badge */
        box.badge > label.badge-icon {{
            font-family: "Material Symbols Rounded";
            font-size: {icon_size}px;
            color: {text_color};
        }}

        /* Text badge */
        box.badge > label.badge-text {{
            font-family: "JetBrains Mono NF", monospace;
            font-weight: bold;
            color: {text_color};
        }}

        /* Text badge sizes based on character count */
        box.badge > label.badge-text-1 {{
            font-size: {text_1}px;
        }}

        box.badge > label.badge-text-2 {{
            font-size: {text_2}px;
        }}

        box.badge > label.badge-text-3 {{
            font-size: {text_3}px;
        }}
        "#,
        icon_size = theme.scaled_font(design::badge::ICON_SIZE),
        text_1 = theme.scaled_font(design::badge::TEXT_SIZE_1),
        text_2 = theme.scaled_font(design::badge::TEXT_SIZE_2),
        text_3 = theme.scaled_font(design::badge::TEXT_SIZE_3),
        bg = colors.surface_container_highest,
        border_color = colors.outline,
        text_color = colors.on_surface,
    )
}
