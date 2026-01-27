//! `AmbientItem` widget - Persistent status item shown in action bar
//!
//! Ambient items are pill-shaped widgets that display persistent status
//! from plugins (timers, downloads, notifications, etc). They appear in
//! the action bar when in hints mode.
//!
//! Features:
//! - Icon + name + description
//! - Badges and chips
//! - Action buttons (up to 3)
//! - Dismiss button (x)

use crate::widgets::design::ambient_item as design;
use crate::widgets::{BadgeWidget, ChipWidget};
use gtk4::prelude::*;
use hamr_types::AmbientItem;
use std::cell::RefCell;
use std::rc::Rc;

/// Callback types for ambient item interactions
type ActionCallback = Box<dyn Fn(&str, &str)>; // (item_id, action_id)
type DismissCallback = Box<dyn Fn(&str)>; // (item_id)

/// Create an action button for an ambient item
fn create_action_button(
    action: &hamr_types::Action,
    item_id: &str,
    on_action: &Rc<RefCell<Option<ActionCallback>>>,
) -> gtk4::Button {
    let action_btn = gtk4::Button::builder()
        .css_classes(["ambient-action-btn", "has-tooltip"])
        .tooltip_text(&action.name)
        .build();

    let action_icon = action.icon.as_deref().unwrap_or("arrow_forward");
    let icon_label = gtk4::Label::builder()
        .label(action_icon)
        .css_classes(["ambient-action-icon", "material-icon"])
        .build();
    action_btn.set_child(Some(&icon_label));

    let action_id = action.id.clone();
    let item_id_clone = item_id.to_owned();
    let on_action_clone = on_action.clone();
    action_btn.connect_clicked(move |_| {
        tracing::debug!(
            "Ambient action button clicked: item={}, action={}",
            item_id_clone,
            action_id
        );
        if let Some(ref cb) = *on_action_clone.borrow() {
            cb(&item_id_clone, &action_id);
        } else {
            tracing::warn!("No action callback registered for ambient item");
        }
    });

    action_btn
}

/// Create a dismiss button for an ambient item
fn create_dismiss_button(
    item_id: &str,
    on_dismiss: &Rc<RefCell<Option<DismissCallback>>>,
) -> gtk4::Button {
    let dismiss_btn = gtk4::Button::builder()
        .css_classes(["ambient-dismiss-btn", "has-tooltip"])
        .tooltip_text("Dismiss")
        .build();

    let dismiss_icon = gtk4::Label::builder()
        .label("close")
        .css_classes(["ambient-action-icon", "material-icon"])
        .build();
    dismiss_btn.set_child(Some(&dismiss_icon));

    let item_id_clone = item_id.to_owned();
    let on_dismiss_clone = on_dismiss.clone();
    dismiss_btn.connect_clicked(move |_| {
        tracing::debug!("Ambient dismiss button clicked: item={}", item_id_clone);
        if let Some(ref cb) = *on_dismiss_clone.borrow() {
            cb(&item_id_clone);
        } else {
            tracing::warn!("No dismiss callback registered for ambient item");
        }
    });

    dismiss_btn
}

/// `AmbientItem` widget
pub struct AmbientItemWidget {
    container: gtk4::Box,
    name_label: gtk4::Label,
    desc_label: Option<gtk4::Label>,
    badges_container: gtk4::Box,
    chips_container: gtk4::Box,
    on_action: Rc<RefCell<Option<ActionCallback>>>,
    on_dismiss: Rc<RefCell<Option<DismissCallback>>>,
}

impl AmbientItemWidget {
    /// Create a new ambient item widget
    pub fn new(item: &AmbientItem, _plugin_id: &str) -> Self {
        let item_id = item.id.clone();

        let on_action: Rc<RefCell<Option<ActionCallback>>> = Rc::new(RefCell::new(None));
        let on_dismiss: Rc<RefCell<Option<DismissCallback>>> = Rc::new(RefCell::new(None));

        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(design::SPACING)
            .css_classes(["ambient-item"])
            .build();

        if let Some(icon) = &item.icon {
            let icon_label = gtk4::Label::builder()
                .label(icon)
                .css_classes(["ambient-item-icon", "material-icon"])
                .build();
            container.append(&icon_label);
        }

        let name_label = gtk4::Label::builder()
            .label(&item.name)
            .css_classes(["ambient-item-name"])
            .build();
        container.append(&name_label);

        let desc_label = if let Some(desc) = &item.description {
            let label = gtk4::Label::builder()
                .label(desc)
                .css_classes(["ambient-item-desc"])
                .build();
            container.append(&label);
            Some(label)
        } else {
            None
        };

        let badges_container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(2)
            .build();
        for badge in item.badges.iter().take(3) {
            let badge_widget = BadgeWidget::new(badge);
            badges_container.append(badge_widget.widget());
        }
        container.append(&badges_container);

        let chips_container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(2)
            .build();
        for chip in item.chips.iter().take(2) {
            let chip_widget = ChipWidget::new(chip);
            chips_container.append(chip_widget.widget());
        }
        container.append(&chips_container);

        for action in item.actions.iter().take(3) {
            let action_btn = create_action_button(action, &item_id, &on_action);
            container.append(&action_btn);
        }

        let dismiss_btn = create_dismiss_button(&item_id, &on_dismiss);
        container.append(&dismiss_btn);

        Self {
            container,
            name_label,
            desc_label,
            badges_container,
            chips_container,
            on_action,
            on_dismiss,
        }
    }

    /// Update the widget with new data (reactive update - no rebuild)
    pub fn update(&self, item: &AmbientItem) {
        if self.name_label.text() != item.name {
            tracing::trace!(
                "Ambient item name changed: {} -> {}",
                self.name_label.text(),
                item.name
            );
            self.name_label.set_text(&item.name);
        }

        if let Some(desc_label) = &self.desc_label
            && let Some(desc) = &item.description
            && desc_label.text() != *desc
        {
            tracing::trace!(
                "Ambient item desc changed: {} -> {}",
                desc_label.text(),
                desc
            );
            desc_label.set_text(desc);
        }

        let current_badge_count = {
            let mut count = 0;
            let mut child = self.badges_container.first_child();
            while child.is_some() {
                count += 1;
                child = child.and_then(|c| c.next_sibling());
            }
            count
        };

        if current_badge_count != item.badges.len() {
            while let Some(child) = self.badges_container.first_child() {
                self.badges_container.remove(&child);
            }
            for badge in item.badges.iter().take(3) {
                let badge_widget = BadgeWidget::new(badge);
                self.badges_container.append(badge_widget.widget());
            }
        }

        let current_chip_count = {
            let mut count = 0;
            let mut child = self.chips_container.first_child();
            while child.is_some() {
                count += 1;
                child = child.and_then(|c| c.next_sibling());
            }
            count
        };

        if current_chip_count != item.chips.len() {
            while let Some(child) = self.chips_container.first_child() {
                self.chips_container.remove(&child);
            }
            for chip in item.chips.iter().take(2) {
                let chip_widget = ChipWidget::new(chip);
                self.chips_container.append(chip_widget.widget());
            }
        }
    }

    /// Get the underlying GTK widget
    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    /// Connect action callback (`item_id`, `action_id`)
    pub fn connect_action<F: Fn(&str, &str) + 'static>(&self, f: F) {
        *self.on_action.borrow_mut() = Some(Box::new(f));
    }

    /// Connect dismiss callback (`item_id`)
    pub fn connect_dismiss<F: Fn(&str) + 'static>(&self, f: F) {
        *self.on_dismiss.borrow_mut() = Some(Box::new(f));
    }
}

impl AsRef<gtk4::Widget> for AmbientItemWidget {
    fn as_ref(&self) -> &gtk4::Widget {
        self.container.upcast_ref()
    }
}

/// Generate CSS for ambient item styling
pub fn ambient_item_css(theme: &crate::config::Theme) -> String {
    use crate::widgets::design::{font, radius, spacing};

    let colors = &theme.colors;

    // Pre-compute scaled values
    let height = theme.scaled(design::HEIGHT);
    let border_radius = theme.scaled(design::RADIUS);
    let padding_h = theme.scaled(design::PADDING_H);
    let border = theme.scaled(1);
    let action_size = theme.scaled(design::ACTION_SIZE);
    let tooltip_radius = theme.scaled(radius::SM);
    let tooltip_pad_v = theme.scaled(spacing::XS);
    let tooltip_pad_h = theme.scaled(spacing::SM);

    // Fonts
    let icon_size = theme.scaled_font(design::ICON_SIZE);
    let action_icon_size = theme.scaled_font(design::ACTION_ICON_SIZE);
    let font_small = theme.scaled_font(font::SM);

    format!(
        r#"
        /* Ambient item container - pill shaped */
        .ambient-item {{
            min-height: {height}px;
            border-radius: {border_radius}px;
            background-color: {bg};
            border: {border}px solid {border_color};
            padding: 0 {padding_h}px;
            transition: background-color 100ms ease-out;
        }}

        .ambient-item:hover {{
            background-color: {bg_hover};
        }}

        /* Icon */
        .ambient-item-icon {{
            font-family: "Material Symbols Rounded";
            font-size: {icon_size}px;
            color: {primary};
        }}

        /* Name */
        .ambient-item-name {{
            font-family: "Inter", "Google Sans Flex", sans-serif;
            font-size: {font_small}px;
            font-weight: 500;
            color: {on_surface};
        }}

        /* Description */
        .ambient-item-desc {{
            font-family: "Inter", "Google Sans Flex", sans-serif;
            font-size: {font_small}px;
            color: {outline};
        }}

        /* Action button */
        .ambient-action-btn {{
            min-width: {action_size}px;
            min-height: {action_size}px;
            border-radius: {border_radius}px;
            background-color: transparent;
            border: none;
            padding: 0;
        }}

        .ambient-action-btn:hover {{
            background-color: {bg_hover};
        }}

        /* Action icon */
        .ambient-action-icon {{
            font-family: "Material Symbols Rounded";
            font-size: {action_icon_size}px;
            color: {outline};
        }}

        .ambient-action-btn:hover .ambient-action-icon {{
            color: {on_surface};
        }}

        /* Dismiss button */
        .ambient-dismiss-btn {{
            min-width: {action_size}px;
            min-height: {action_size}px;
            border-radius: {border_radius}px;
            background-color: transparent;
            border: none;
            padding: 0;
        }}

        .ambient-dismiss-btn:hover {{
            background-color: {bg_hover};
        }}

        /* Native tooltip styling */
        tooltip {{
            background-color: {surface_container_highest};
            border-radius: {tooltip_radius}px;
            padding: {tooltip_pad_v}px {tooltip_pad_h}px;
        }}

        tooltip label {{
            font-family: "Inter", "Google Sans Flex", sans-serif;
            font-size: {font_small}px;
            font-weight: 500;
            color: {on_surface};
        }}
        "#,
        bg = colors.surface_container,
        bg_hover = colors.surface_container_high,
        border_color = colors.outline_variant,
        primary = colors.primary,
        on_surface = colors.on_surface,
        outline = colors.outline,
        surface_container_highest = colors.surface_container_highest,
    )
}
