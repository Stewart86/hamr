//! Ripple button widget - Reusable button with hover effects and tooltip support
//!
//! Used for action buttons, slider +/- buttons, and other icon buttons.
//! This widget is fully reusable - you can drop the `RippleButton` struct after
//! appending `widget()` to a container. All state is stored on the GTK widgets.

use super::design;
use gtk4::prelude::*;
use gtk4::{Align, Orientation};
use hamr_types::Action;
use std::cell::RefCell;
use std::rc::Rc;

// Keys for storing data on the button widget
const POPOVER_KEY: &str = "ripple-popover";
const FOCUSED_KEY: &str = "ripple-focused";
const ACTION_ID_KEY: &str = "ripple-action-id";

/// A ripple button with icon, hover effects, and optional tooltip.
///
/// This widget is designed to be reusable - the `RippleButton` struct can be
/// dropped after the widget is added to a container. All necessary state
/// (popover, focused state) is stored on the GTK button widget itself.
pub struct RippleButton {
    container: gtk4::Box,
    button: gtk4::Button,
}

impl RippleButton {
    /// Create a new ripple button from an Action with optional keyboard hint
    pub fn from_action(action: &Action, keyboard_hint: Option<&str>) -> Self {
        let icon = action.icon.as_deref().unwrap_or("arrow_forward");
        let button = Self::new(icon, &action.id);
        button.set_tooltip(&action.name, keyboard_hint.unwrap_or(""));
        button
    }

    /// Create a new ripple button with an icon
    pub fn new(icon: &str, action_id: &str) -> Self {
        let container = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .build();

        let button = gtk4::Button::builder()
            .css_classes(["ripple-button"])
            .halign(Align::Center)
            .valign(Align::Center)
            .build();

        let icon_label = gtk4::Label::builder()
            .label(icon)
            .css_classes(["ripple-button-icon", "material-icon"])
            .halign(Align::Center)
            .valign(Align::Center)
            .build();
        button.set_child(Some(&icon_label));

        // Create popover and store it on the button
        let tooltip_popover = gtk4::Popover::builder()
            .css_classes(["action-tooltip"])
            .autohide(false)
            .has_arrow(false)
            .position(gtk4::PositionType::Bottom)
            .build();

        let tooltip_content = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(6)
            .css_classes(["action-tooltip-content"])
            .build();
        tooltip_popover.set_child(Some(&tooltip_content));
        tooltip_popover.set_parent(&button);

        // Store state on the button widget itself
        unsafe {
            button.set_data(POPOVER_KEY, tooltip_popover);
            button.set_data(FOCUSED_KEY, Rc::new(RefCell::new(false)));
            button.set_data(ACTION_ID_KEY, action_id.to_string());
        }

        // Clean up popover when button is destroyed
        button.connect_destroy(|btn| {
            if let Some(popover) = Self::get_popover(btn) {
                popover.unparent();
            }
        });

        // Setup hover handlers
        Self::setup_hover_handlers(&button);

        container.append(&button);

        Self { container, button }
    }

    /// Set the tooltip content with action name and optional keyboard hint
    pub fn set_tooltip(&self, action_name: &str, keyboard_hint: &str) {
        Self::set_tooltip_on_button(&self.button, action_name, keyboard_hint);
    }

    /// Set tooltip on any button that was created by `RippleButton`
    fn set_tooltip_on_button(button: &gtk4::Button, action_name: &str, keyboard_hint: &str) {
        let Some(popover) = Self::get_popover(button) else {
            return;
        };

        let content = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(6)
            .css_classes(["action-tooltip-content"])
            .build();

        if !keyboard_hint.is_empty() {
            let kbd = gtk4::Label::builder()
                .label(keyboard_hint)
                .css_classes(["action-tooltip-kbd"])
                .build();
            content.append(&kbd);
        }

        let name = gtk4::Label::builder()
            .label(action_name)
            .css_classes(["action-tooltip-name"])
            .build();
        content.append(&name);

        popover.set_child(Some(&content));
    }

    fn get_popover(button: &gtk4::Button) -> Option<gtk4::Popover> {
        unsafe {
            button
                .data::<gtk4::Popover>(POPOVER_KEY)
                .map(|p| p.as_ref().clone())
        }
    }

    fn get_focused(button: &gtk4::Button) -> Option<Rc<RefCell<bool>>> {
        unsafe {
            button
                .data::<Rc<RefCell<bool>>>(FOCUSED_KEY)
                .map(|p| p.as_ref().clone())
        }
    }

    fn get_action_id(button: &gtk4::Button) -> Option<String> {
        unsafe {
            button
                .data::<String>(ACTION_ID_KEY)
                .map(|p| p.as_ref().clone())
        }
    }

    fn setup_hover_handlers(button: &gtk4::Button) {
        let motion_controller = gtk4::EventControllerMotion::new();

        let btn_enter = button.clone();
        motion_controller.connect_enter(move |_, _, _| {
            let Some(popover) = Self::get_popover(&btn_enter) else {
                return;
            };
            let Some(is_focused) = Self::get_focused(&btn_enter) else {
                return;
            };
            if !*is_focused.borrow() {
                popover.popup();
            }
        });

        let btn_leave = button.clone();
        motion_controller.connect_leave(move |_| {
            let Some(popover) = Self::get_popover(&btn_leave) else {
                return;
            };
            let Some(is_focused) = Self::get_focused(&btn_leave) else {
                return;
            };
            if !*is_focused.borrow() {
                popover.popdown();
            }
        });

        button.add_controller(motion_controller);
    }

    /// Get the action ID
    pub fn action_id(&self) -> String {
        Self::get_action_id(&self.button).unwrap_or_default()
    }

    /// Get the underlying container widget
    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    /// Get the button widget (for click handlers)
    #[allow(dead_code)]
    pub fn button(&self) -> &gtk4::Button {
        &self.button
    }

    /// Set focused state (for keyboard navigation)
    pub fn set_focused(&self, focused: bool) {
        Self::set_focused_on_button(&self.button, focused);
    }

    fn set_focused_on_button(button: &gtk4::Button, focused: bool) {
        let Some(is_focused_rc) = Self::get_focused(button) else {
            return;
        };

        *is_focused_rc.borrow_mut() = focused;
        if focused {
            button.add_css_class("focused");
        } else {
            button.remove_css_class("focused");
        }

        // Handle popover if present (not present for native tooltip variant)
        if let Some(popover) = Self::get_popover(button) {
            if focused {
                popover.popup();
            } else {
                popover.popdown();
            }
        }
    }

    /// Update the icon
    pub fn set_icon(&self, icon: &str) {
        if let Some(child) = self.button.child()
            && let Some(label) = child.downcast_ref::<gtk4::Label>()
            && label.text() != icon
        {
            label.set_text(icon);
        }
    }

    /// Update from Action data with optional keyboard hint
    pub fn update(&self, action: &Action, keyboard_hint: Option<&str>) {
        let icon = action.icon.as_deref().unwrap_or("arrow_forward");
        self.set_icon(icon);
        self.set_tooltip(&action.name, keyboard_hint.unwrap_or(""));
    }

    /// Connect a click handler
    pub fn connect_clicked<F: Fn(&str) + 'static>(&self, f: F) {
        let button = self.button.clone();
        self.button.connect_clicked(move |_| {
            if let Some(action_id) = Self::get_action_id(&button) {
                f(&action_id);
            }
        });
    }
}

/// Generate CSS for ripple button styling
pub fn ripple_button_css(theme: &crate::config::Theme) -> String {
    let colors = &theme.colors;

    // Pre-compute scaled values
    let size = theme.scaled(design::ripple_button::SIZE);
    let radius = theme.scaled(design::ripple_button::RADIUS);
    let tooltip_offset = theme.scaled(design::ripple_button::TOOLTIP_OFFSET);
    let icon_size = theme.scaled_font(design::ripple_button::ICON_SIZE);

    // Tooltip dimensions
    let border = theme.scaled(1);
    let tooltip_radius = theme.scaled(design::radius::SM - design::spacing::XXXS); // 8 - 2 = 6
    let tooltip_padding_v = theme.scaled(design::spacing::XS); // 4
    let tooltip_padding_h = theme.scaled(design::spacing::SM); // 8
    let content_padding_v = theme.scaled(design::spacing::XXXS); // 2

    // Kbd styling
    let kbd_radius = theme.scaled(3); // Close to radius::XS=4 but smaller for tight fit
    let kbd_padding_v = theme.scaled(design::spacing::XXXS); // 2
    let kbd_padding_h = theme.scaled(design::spacing::XS); // 4

    // Fonts
    let font_kbd = theme.scaled_font(design::font::XS); // 9
    let font_name = theme.scaled_font(design::font::SM); // 11

    format!(
        r#"
        .ripple-button {{
            min-width: {size}px;
            min-height: {size}px;
            border-radius: {radius}px;
            background-color: {surface_container};
            padding: 0;
            border: none;
            transition: background-color 150ms ease;
        }}

        .ripple-button:hover {{
            background-color: alpha({on_surface}, 0.03);
        }}

        .ripple-button:active {{
            background-color: alpha({on_surface}, 0.14);
        }}

        .ripple-button.focused {{
            background-color: alpha({primary}, 0.18);
        }}

        .ripple-button-icon {{
            font-family: "Material Symbols Rounded";
            font-size: {icon_size}px;
            color: {outline};
            opacity: 0.8;
        }}

        .ripple-button:hover .ripple-button-icon {{
            color: {on_surface_variant};
            opacity: 0.5;
        }}

        .ripple-button.focused .ripple-button-icon {{
            color: {on_primary};
            opacity: 1.0;
        }}

        .action-tooltip {{
            background-color: {surface_container};
            border: {border}px solid {outline_variant};
            border-radius: {tooltip_radius}px;
            padding: {tooltip_padding_v}px {tooltip_padding_h}px;
            margin-top: {tooltip_offset}px;
        }}

        .action-tooltip-content {{
            padding: {content_padding_v}px 0;
        }}

        .action-tooltip-kbd {{
            font-family: "Inter", sans-serif;
            font-size: {font_kbd}px;
            font-weight: 500;
            color: {on_surface_variant};
            background-color: {surface_container_high};
            border-radius: {kbd_radius}px;
            padding: {kbd_padding_v}px {kbd_padding_h}px;
        }}

        .action-tooltip-name {{
            font-family: "Inter", sans-serif;
            font-size: {font_name}px;
            color: {on_surface};
        }}
        "#,
        size = size,
        icon_size = icon_size,
        radius = radius,
        tooltip_offset = tooltip_offset,
        tooltip_radius = tooltip_radius,
        tooltip_padding_v = tooltip_padding_v,
        tooltip_padding_h = tooltip_padding_h,
        content_padding_v = content_padding_v,
        border = border,
        kbd_radius = kbd_radius,
        kbd_padding_v = kbd_padding_v,
        kbd_padding_h = kbd_padding_h,
        font_kbd = font_kbd,
        font_name = font_name,
        primary = colors.primary,
        on_primary = colors.on_surface,
        outline = colors.outline,
        on_surface_variant = colors.on_surface_variant,
        on_surface = colors.on_surface,
        surface_container = colors.surface_container,
        surface_container_high = colors.surface_container_high,
        outline_variant = colors.outline_variant,
    )
}
