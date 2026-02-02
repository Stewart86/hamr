//! `KeybindingMap` widget - Popup showing all keyboard shortcuts
//!
//! Displays a comprehensive list of keyboard shortcuts organized by category:
//! - Navigation (Ctrl+HJKL or arrow keys)
//! - Actions (Enter, Tab, Backspace, etc.)
//! - Quick Prefixes (from config)
//! - Window controls (Esc, Ctrl+M)

use crate::widgets::design;
use crate::widgets::kbd::KbdWidget;
use gtk4::prelude::*;
use gtk4::Orientation;
use hamr_core::config::ActionBarHint;

/// `KeybindingMap` popup widget
pub struct KeybindingMap {
    container: gtk4::Box,
    prefixes_section: gtk4::Box,
}

impl KeybindingMap {
    pub fn new() -> Self {
        let container = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(design::keybinding_map::SECTION_SPACING)
            .css_classes(["keybinding-map"])
            .build();

        // Left column: Navigation
        let nav_section = Self::create_navigation_section(false);
        container.append(&nav_section);

        container.append(&Self::create_separator_vertical());

        // Middle column: Actions + Window
        let middle_col = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(design::keybinding_map::SECTION_SPACING)
            .build();
        middle_col.append(&Self::create_actions_section());
        middle_col.append(&Self::create_separator());
        middle_col.append(&Self::create_window_section());
        container.append(&middle_col);

        // Right column: Prefixes (initially empty, updated via set_prefixes)
        let prefixes_section = Self::create_prefixes_section(&[]);
        container.append(&Self::create_separator_vertical());
        container.append(&prefixes_section);

        Self {
            container,
            prefixes_section,
        }
    }

    /// Update the prefix hints from config
    // Grid row index is usize from enumerate, GTK attach requires i32
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub fn set_prefixes(&self, hints: &[ActionBarHint]) {
        // Remove all children from prefixes section
        while let Some(child) = self.prefixes_section.first_child() {
            self.prefixes_section.remove(&child);
        }

        // Rebuild the section content
        self.prefixes_section
            .append(&Self::create_section_header("Prefixes"));

        if hints.is_empty() {
            let empty_label = gtk4::Label::builder()
                .label("None configured")
                .css_classes(["keybinding-label"])
                .halign(gtk4::Align::Start)
                .build();
            self.prefixes_section.append(&empty_label);
        } else {
            let grid = gtk4::Grid::builder()
                .row_spacing(design::keybinding_map::ROW_SPACING)
                .column_spacing(design::keybinding_map::COLUMN_SPACING)
                .build();

            for (i, hint) in hints.iter().enumerate() {
                let row = i as i32;
                let label = hint
                    .label
                    .as_deref()
                    .or(hint.description.as_deref())
                    .unwrap_or(&hint.plugin);
                Self::add_keybinding_row(&grid, row, 0, &hint.prefix, label);
            }

            self.prefixes_section.append(&grid);
        }
    }

    /// Get the GTK widget
    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    fn create_section_header(title: &str) -> gtk4::Label {
        gtk4::Label::builder()
            .label(title)
            .css_classes(["keybinding-section-header"])
            .halign(gtk4::Align::Start)
            .build()
    }

    fn create_separator() -> gtk4::Separator {
        gtk4::Separator::builder()
            .orientation(Orientation::Horizontal)
            .css_classes(["keybinding-separator"])
            .build()
    }

    fn create_separator_vertical() -> gtk4::Separator {
        gtk4::Separator::builder()
            .orientation(Orientation::Vertical)
            .css_classes(["keybinding-separator-vertical"])
            .build()
    }

    fn create_navigation_section(browser_keys: bool) -> gtk4::Box {
        let section = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .build();

        let title = if browser_keys {
            "Grid Nav"
        } else {
            "Navigation"
        };
        section.append(&Self::create_section_header(title));

        let nav_container = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .halign(gtk4::Align::Center)
            .build();

        let left_label = if browser_keys { "left" } else { "back" };
        let left_text = gtk4::Label::builder()
            .label(left_label)
            .css_classes(["keybinding-nav-label"])
            .valign(gtk4::Align::Center)
            .build();

        let nav_grid = gtk4::Grid::builder()
            .row_spacing(2)
            .column_spacing(2)
            .halign(gtk4::Align::Center)
            .build();

        let (up, down, left, right) = if browser_keys {
            ("K", "J", "H", "L")
        } else {
            ("^K", "^J", "^H", "^L")
        };

        let up_kbd = KbdWidget::new(up);
        nav_grid.attach(up_kbd.widget(), 1, 0, 1, 1);

        let left_kbd = KbdWidget::new(left);
        let down_kbd = KbdWidget::new(down);
        let right_kbd = KbdWidget::new(right);
        nav_grid.attach(left_kbd.widget(), 0, 1, 1, 1);
        nav_grid.attach(down_kbd.widget(), 1, 1, 1, 1);
        nav_grid.attach(right_kbd.widget(), 2, 1, 1, 1);

        let right_label = if browser_keys { "right" } else { "select" };
        let right_text = gtk4::Label::builder()
            .label(right_label)
            .css_classes(["keybinding-nav-label"])
            .valign(gtk4::Align::Center)
            .build();

        nav_container.append(&left_text);
        nav_container.append(&nav_grid);
        nav_container.append(&right_text);

        section.append(&nav_container);

        section
    }

    fn create_actions_section() -> gtk4::Box {
        let section = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .build();

        section.append(&Self::create_section_header("Actions"));

        let grid = gtk4::Grid::builder()
            .row_spacing(design::keybinding_map::ROW_SPACING)
            .column_spacing(design::keybinding_map::COLUMN_SPACING)
            .build();

        Self::add_keybinding_row(&grid, 0, 0, "Enter", "confirm");
        Self::add_keybinding_row(&grid, 1, 0, "Bksp", "go back");
        Self::add_keybinding_row(&grid, 2, 0, "^UIOP", "actions 1-4");
        Self::add_keybinding_row(&grid, 3, 0, "^+HL", "slider -/+");

        Self::add_keybinding_row(&grid, 0, 2, "Tab", "cycle");
        Self::add_keybinding_row(&grid, 1, 2, "^Bksp", "exit plugin");
        Self::add_keybinding_row(&grid, 2, 2, "^1-6", "FAB actions");
        Self::add_keybinding_row(&grid, 3, 2, "^+T", "toggle");

        Self::add_keybinding_row(&grid, 4, 0, "+Enter", "slider -");
        Self::add_keybinding_row(&grid, 4, 2, "Enter", "slider +");

        section.append(&grid);
        section
    }

    // Grid row index is usize from enumerate, GTK attach requires i32
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    fn create_prefixes_section(hints: &[ActionBarHint]) -> gtk4::Box {
        let section = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .valign(gtk4::Align::Center)
            .build();

        section.append(&Self::create_section_header("Prefixes"));

        if hints.is_empty() {
            let empty_label = gtk4::Label::builder()
                .label("None configured")
                .css_classes(["keybinding-label"])
                .halign(gtk4::Align::Start)
                .build();
            section.append(&empty_label);
        } else {
            let grid = gtk4::Grid::builder()
                .row_spacing(design::keybinding_map::ROW_SPACING)
                .column_spacing(design::keybinding_map::COLUMN_SPACING)
                .build();

            for (i, hint) in hints.iter().enumerate() {
                let row = i as i32;
                let label = hint
                    .label
                    .as_deref()
                    .or(hint.description.as_deref())
                    .unwrap_or(&hint.plugin);
                Self::add_keybinding_row(&grid, row, 0, &hint.prefix, label);
            }

            section.append(&grid);
        }

        section
    }

    fn create_window_section() -> gtk4::Box {
        let section = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .build();

        section.append(&Self::create_section_header("Window"));

        let grid = gtk4::Grid::builder()
            .row_spacing(design::keybinding_map::ROW_SPACING)
            .column_spacing(design::keybinding_map::COLUMN_SPACING)
            .build();

        Self::add_keybinding_row(&grid, 0, 0, "Esc", "close");
        Self::add_keybinding_row(&grid, 1, 0, "?", "shortcuts");

        section.append(&grid);
        section
    }

    fn add_keybinding_row(grid: &gtk4::Grid, row: i32, col: i32, keys: &str, label: &str) {
        let kbd = KbdWidget::new(keys);
        kbd.widget().set_width_request(48);
        kbd.widget().set_halign(gtk4::Align::Start);

        let label_widget = gtk4::Label::builder()
            .label(label)
            .css_classes(["keybinding-label"])
            .halign(gtk4::Align::Start)
            .build();

        grid.attach(kbd.widget(), col, row, 1, 1);
        grid.attach(&label_widget, col + 1, row, 1, 1);
    }
}

impl Default for KeybindingMap {
    fn default() -> Self {
        Self::new()
    }
}

pub fn keybinding_map_css(theme: &crate::config::Theme) -> String {
    let colors = &theme.colors;

    let border = theme.scaled(1);
    let radius = theme.scaled(design::keybinding_map::BORDER_RADIUS);
    let padding = theme.scaled(design::keybinding_map::PADDING);
    let font_small = theme.scaled_font(design::font::SM);
    let font_tiny = theme.scaled_font(design::font::XS + 1); // 10

    format!(
        r"
        .keybinding-map {{
            background-color: {surface_container};
            border-radius: {radius}px;
            border: {border}px solid {outline_variant};
            padding: {padding}px;
        }}

        .keybinding-section-header {{
            font-size: {font_small}px;
            font-weight: 500;
            color: {primary};
        }}

        .keybinding-separator {{
            background-color: {outline_variant};
            min-height: {border}px;
        }}

        .keybinding-separator-vertical {{
            background-color: {outline_variant};
            min-width: {border}px;
        }}

        .keybinding-nav-label {{
            font-size: {font_tiny}px;
            color: {outline};
        }}

        .keybinding-label {{
            font-size: {font_small}px;
            color: {on_surface_variant};
        }}
        ",
        surface_container = colors.surface_container,
        outline_variant = colors.outline_variant,
        primary = colors.primary,
        outline = colors.outline,
        on_surface_variant = colors.on_surface_variant,
    )
}
