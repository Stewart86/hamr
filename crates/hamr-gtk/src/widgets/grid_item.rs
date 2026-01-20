//! Grid item widget for displaying results in a grid layout.
//!
//! Layout structure:
//! ```text
//! +-- GridItem Container --------+
//! |  +-- Visual Area -----------+ |
//! |  |  (image/emoji/icon)      | |
//! |  |  (square, configurable)  | |
//! |  +-------------------------+ |
//! |  +-- Name Label -----------+ |
//! |  |  (truncated, centered)   | |
//! |  +-------------------------+ |
//! |  +-- Actions Row (hover) --+ |
//! |  |  [A1] [A2] [A3]          | |
//! |  +-------------------------+ |
//! +-----------------------------+
//! ```

use super::design;
use super::result_visual::{ResultVisual, VisualSize};
use super::ripple_button::RippleButton;
use gtk4::prelude::*;
use gtk4::{Align, Orientation, Overlay};
use hamr_rpc::SearchResult;
use std::cell::RefCell;
use std::rc::Rc;

type ActionCallback = Rc<dyn Fn(&str, &str)>;

/// Keyboard hints for action buttons (Alt+U, Alt+I, Alt+O, Alt+P)
const ACTION_HINTS: [&str; 4] = ["Alt+U", "Alt+I", "Alt+O", "Alt+P"];

/// Grid items always use Thumbnail size for consistency
fn visual_size_for_result(_result: &SearchResult) -> VisualSize {
    // All grid items use the same size so icons, gauges, and thumbnails align
    VisualSize::Thumbnail
}

/// Result of building action buttons overlay
struct ActionButtonsResult {
    buttons: Vec<RippleButton>,
    overlay: gtk4::Box,
}

/// Build the action buttons overlay for a grid item
fn build_action_buttons(result: &SearchResult) -> ActionButtonsResult {
    let action_count = result.actions.len().min(design::grid::MAX_ACTION_BUTTONS);
    let use_grid = action_count > 2;

    let actions_grid = gtk4::Box::builder()
        .orientation(Orientation::Vertical)
        .css_classes(["grid-item-actions-overlay"])
        .halign(Align::Fill)
        .valign(Align::Fill)
        .spacing(design::grid::ACTION_SPACING)
        .build();

    let actions_row_top = gtk4::Box::builder()
        .orientation(Orientation::Horizontal)
        .halign(if use_grid {
            Align::Start
        } else {
            Align::Center
        })
        .valign(if use_grid { Align::End } else { Align::Center })
        .spacing(design::grid::ACTION_SPACING)
        .hexpand(true)
        .vexpand(true)
        .build();

    let actions_row_bottom = gtk4::Box::builder()
        .orientation(Orientation::Horizontal)
        .halign(Align::Start)
        .valign(Align::Start)
        .spacing(design::grid::ACTION_SPACING)
        .hexpand(true)
        .vexpand(true)
        .build();

    if use_grid {
        actions_grid.set_halign(Align::Center);
        actions_grid.set_valign(Align::Center);
        actions_grid.set_hexpand(false);
        actions_grid.set_vexpand(false);
        actions_row_top.set_hexpand(false);
        actions_row_top.set_vexpand(false);
        actions_row_bottom.set_hexpand(false);
        actions_row_bottom.set_vexpand(false);
    }

    let mut buttons = Vec::new();
    for (index, action) in result
        .actions
        .iter()
        .take(design::grid::MAX_ACTION_BUTTONS)
        .enumerate()
    {
        let hint = ACTION_HINTS.get(index).copied();
        let button = RippleButton::from_action(action, hint);
        button.widget().add_css_class("grid-action-button");
        if index < 2 {
            actions_row_top.append(button.widget());
        } else {
            actions_row_bottom.append(button.widget());
        }
        buttons.push(button);
    }

    actions_grid.append(&actions_row_top);
    if action_count > 2 {
        actions_grid.append(&actions_row_bottom);
    }

    ActionButtonsResult {
        buttons,
        overlay: actions_grid,
    }
}

pub struct GridItem {
    container: gtk4::Box,
    highlight_area: Overlay, // Overlay that receives hover/selection styles
    id: String,
    button_actions: RefCell<Vec<RippleButton>>,
    actions_row: gtk4::Box,
}

impl GridItem {
    pub fn new(result: &SearchResult, selected: bool) -> Self {
        let container = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .css_classes(["grid-item-container"])
            .halign(Align::Center)
            .valign(Align::Start)
            .width_request(design::grid::ITEM_WIDTH)
            .build();

        let highlight_content = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .halign(Align::Fill)
            .valign(Align::Fill)
            .build();

        let visual = ResultVisual::new(result, visual_size_for_result(result));
        highlight_content.append(visual.widget());

        let highlight_area = Overlay::new();
        highlight_area.add_css_class("grid-item");
        if selected {
            highlight_area.add_css_class("selected");
        }
        highlight_area.set_size_request(design::grid::HIGHLIGHT_SIZE, design::grid::HIGHLIGHT_SIZE);
        highlight_area.set_halign(Align::Center);
        highlight_area.set_child(Some(&highlight_content));

        let ActionButtonsResult { buttons, overlay } = build_action_buttons(result);

        let name_label = gtk4::Label::builder()
            .label(&result.name)
            .css_classes(["grid-item-name"])
            .halign(Align::Center)
            .valign(Align::End)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .max_width_chars(24)
            .width_request(design::grid::HIGHLIGHT_SIZE + 32)
            .build();
        highlight_content.append(&name_label);

        if !buttons.is_empty() {
            highlight_area.add_overlay(&overlay);
        }

        container.append(&highlight_area);

        Self {
            container,
            highlight_area,
            id: result.id.clone(),
            button_actions: RefCell::new(buttons),
            actions_row: overlay,
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    pub fn set_selected(&self, selected: bool) {
        if selected {
            self.highlight_area.add_css_class("selected");
        } else {
            self.highlight_area.remove_css_class("selected");
        }
    }

    pub fn connect_action_clicked(&self, f: &ActionCallback) {
        let id = self.id.clone();
        for button in self.button_actions.borrow().iter() {
            let f = Rc::clone(f);
            let id = id.clone();
            button.connect_clicked(move |action_id| {
                f(&id, action_id);
            });
        }
    }

    // Enumerate index is usize, Tab navigation uses i32 (can be negative)
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub fn set_focused_action(&self, index: i32) {
        if index < 0 {
            for button in self.button_actions.borrow().iter() {
                button.set_focused(false);
            }
            return;
        }

        // Ensure actions overlay is visible (forces GTK to map it)
        self.actions_row.set_visible(true);

        for (i, button) in self.button_actions.borrow().iter().enumerate() {
            button.set_focused(i as i32 == index);
        }
    }
}

impl AsRef<gtk4::Widget> for GridItem {
    fn as_ref(&self) -> &gtk4::Widget {
        self.container.upcast_ref()
    }
}

/// Generate CSS for grid item styling
// CSS template - splitting would scatter related style rules
#[allow(clippy::too_many_lines)]
pub fn grid_item_css(theme: &crate::config::Theme) -> String {
    use design::{grid, icon, spacing};

    let colors = &theme.colors;
    let fonts = &theme.config.fonts;

    // Scaled dimensions
    let border_radius = theme.scaled(grid::BORDER_RADIUS);
    let padding = theme.scaled(grid::PADDING);
    let visual_size = theme.scaled(grid::VISUAL_SIZE);
    let visual_size_small = theme.scaled(grid::VISUAL_SIZE - spacing::LG); // 120 - 16 = 104
    let image_border_radius = theme.scaled(grid::IMAGE_BORDER_RADIUS);
    let name_margin_top = theme.scaled(grid::NAME_MARGIN_TOP);
    let action_button_size = theme.scaled(grid::ACTION_BUTTON_SIZE);
    let action_button_padding = theme.scaled(grid::ACTION_BUTTON_PADDING);
    let action_button_radius = theme.scaled(grid::ACTION_BUTTON_RADIUS);
    let border_width = theme.scaled(1);

    // Scaled font sizes
    let grid_icon_size = theme.scaled_font(icon::XXL); // 48
    let grid_name_size = theme.scaled_font(design::font::SM); // 11
    let grid_action_icon_size = theme.scaled_font(icon::MD); // 20

    format!(
        r#"
        /* Outer container - transparent, handles hover detection */
        .grid-item-container {{
            background: transparent;
        }}

        /* Inner highlight area - receives hover/selection styles */
        .grid-item {{
            border-radius: {border_radius}px;
            padding: {padding}px;
            background: transparent;
            background-color: transparent;
            border: {border_width}px solid transparent; /* Always have border to prevent layout shift */
            transition: background-color 180ms cubic-bezier(0.25, 0.1, 0.25, 1),
                        background 180ms cubic-bezier(0.25, 0.1, 0.25, 1),
                        border-color 180ms cubic-bezier(0.25, 0.1, 0.25, 1);
        }}

        /* Hover highlight matches list */
        .grid-item:hover,
        .grid-item-container:hover .grid-item {{
            background-color: {primary_container};
            background: {primary_container};
        }}

        .grid-item.selected {{
            background: linear-gradient(to bottom, rgba(149, 144, 136, 0.08), 10%, {surface_dark});
            background-color: {surface_dark};
            border-color: alpha({outline}, 0.28);
        }}

        .grid-item.selected:hover,
        .grid-item-container:hover .grid-item.selected {{
            background: linear-gradient(to bottom, rgba(149, 144, 136, 0.08), 10%, {surface_dark});
            background-color: {surface_high};
        }}

        /* Disable hover highlight during scroll */
        .result-grid.scrolling .grid-item:hover,
        .result-grid.scrolling .grid-item-container:hover .grid-item {{
            background-color: transparent;
            background: transparent;
        }}

        /* Grid Item Visual Container */
        .grid-item-visual {{
            min-width: {visual_size}px;
            min-height: {visual_size}px;
        }}

        /* Grid Item Image (thumbnails, file icons) */
        .grid-item-image {{
            border-radius: {image_border_radius}px;
        }}

        /* Grid Item Emoji - large centered text */
        .grid-item-emoji {{
            font-size: {grid_icon_size}px;
            min-width: {visual_size}px;
            min-height: {visual_size}px;
        }}

        /* Grid Item Icon - Material symbol */
        .grid-item-icon {{
            font-family: "{icon_font}";
            font-size: {grid_icon_size}px;
            color: {on_surface_variant};
            min-width: {visual_size}px;
            min-height: {visual_size}px;
        }}

        /* Grid Item System Icon */
        .grid-item-system-icon {{
            min-width: {visual_size_small}px;
            min-height: {visual_size_small}px;
        }}

        /* Grid Item Name - centered below visual */
        .grid-item-name {{
            font-family: "{main_font}";
            font-size: {grid_name_size}px;
            color: {on_surface};
            margin-top: {name_margin_top}px;
        }}

        /* Actions overlay visibility */
        .grid-item-actions-overlay {{
            opacity: 0;
            transition: opacity 150ms ease-out;
        }}

        .grid-item-container:hover .grid-item-actions-overlay,
        .grid-item.selected .grid-item-actions-overlay {{
            opacity: 1;
        }}

        /* Clean minimal action buttons - 2x2 grid */

        .grid-action-button {{
            min-width: {action_button_size}px;
            min-height: {action_button_size}px;
            padding: {action_button_padding}px;
            background: transparent;
            border: none;
            border-radius: {action_button_radius}px;
            transition: background-color 150ms ease-out;
        }}

        .grid-action-button:hover {{
            background-color: alpha(white, 0.2);
        }}

        .grid-action-button .material-icon {{
            font-size: {grid_action_icon_size}px;
            color: white;
        }}
        "#,
        primary_container = colors.primary_container,
        surface_high = colors.surface_container_high,
        surface_dark = colors.surface,
        outline = colors.outline,
        on_surface_variant = colors.on_surface_variant,
        on_surface = colors.on_surface,
        icon_font = fonts.icon,
        main_font = fonts.main,
        border_radius = border_radius,
        padding = padding,
        visual_size = visual_size,
        visual_size_small = visual_size_small,
        image_border_radius = image_border_radius,
        name_margin_top = name_margin_top,
        action_button_size = action_button_size,
        action_button_padding = action_button_padding,
        action_button_radius = action_button_radius,
        border_width = border_width,
        grid_icon_size = grid_icon_size,
        grid_name_size = grid_name_size,
        grid_action_icon_size = grid_action_icon_size,
    )
}
