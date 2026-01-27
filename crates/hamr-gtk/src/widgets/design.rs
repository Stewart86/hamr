//! Design constants matching QML spec exactly.
//!
//! These values are taken directly from the original QML implementation
//! to ensure pixel-perfect matching.
//!
//! Note: Some constants are intentionally kept even if unused, as they serve
//! as a reference for the QML spec and may be used in future features.

#![allow(dead_code)]

/// T-shirt sized spacing tokens for consistent padding/margins
pub mod spacing {
    pub const XXXS: i32 = 2;
    pub const XS: i32 = 4;
    pub const SM: i32 = 8;
    pub const MD: i32 = 12;
    pub const LG: i32 = 16;
    pub const XL: i32 = 20;
    pub const XXL: i32 = 24;
    pub const XXXL: i32 = 32;
}

pub mod item {
    pub const HORIZONTAL_MARGIN: i32 = 4;
    pub const BUTTON_HORIZONTAL_PADDING: i32 = 10;
    pub const BUTTON_VERTICAL_PADDING: i32 = 10;
    pub const BUTTON_RADIUS: i32 = 8;
    pub const ICON_CONTENT_SPACING: i32 = 10;
    pub const CONTENT_SPACING: i32 = 0;
    pub const ACTION_ROW_SPACING: i32 = 4;
    pub const NAME_ROW_SPACING: i32 = 4;
    pub const TYPE_ROW_SPACING: i32 = 6;
    pub const MAX_ACTION_BUTTONS: usize = 4;
}

/// T-shirt sized icon tokens for consistent iconography
pub mod icon {
    // T-shirt sizes for general use
    pub const XS: i32 = 12;
    pub const SM: i32 = 16;
    pub const MD: i32 = 20;
    pub const LG: i32 = 24;
    pub const XL: i32 = 32;
    pub const XXL: i32 = 48;

    // Legacy named sizes (for specific widget contexts)
    pub const CONTAINER_SIZE: i32 = 40;
    pub const SYSTEM_SIZE: i32 = 32;
    pub const MATERIAL_SIZE: i32 = 26;
    pub const TEXT_SIZE: i32 = 17;
    pub const THUMBNAIL_RADIUS: i32 = 4;
}

pub mod running_indicator {
    pub const WIDTH: i32 = 3;
    pub const HEIGHT: i32 = 16;
    pub const RADIUS: f64 = 1.5;

    // LED glow effect constants - LED is at x=0, half-clipped by container border
    pub const GLOW_WIDTH: i32 = 24; // Width for the glow spread area
}

pub mod action_button {
    pub const SIZE: i32 = 28;
    pub const ICON_SIZE: i32 = 20;
    pub const RADIUS: i32 = 8;
}

pub mod badge {
    pub const SIZE: i32 = 20;
    pub const RADIUS: i32 = 10; // SIZE / 2
    pub const BORDER_WIDTH: i32 = 1;
    pub const ICON_SIZE: i32 = 12;
    pub const TEXT_SIZE_1: i32 = 9;
    pub const TEXT_SIZE_2: i32 = 8;
    pub const TEXT_SIZE_3: i32 = 7;
}

pub mod chip {
    pub const HEIGHT: i32 = 16;
    pub const RADIUS: i32 = 8; // HEIGHT / 2
    pub const HORIZONTAL_PADDING: i32 = 3;
    pub const BORDER_WIDTH: i32 = 1;
    pub const ICON_SIZE: i32 = 9;
    pub const TEXT_SIZE: i32 = 8;
    pub const CONTENT_SPACING: i32 = 2;
}

pub mod slider {
    pub const PREFERRED_WIDTH: i32 = 200;
    pub const TOTAL_WIDTH: i32 = 300;
    pub const TOTAL_HEIGHT: i32 = 52;
    pub const TRACK_HEIGHT: i32 = 4;
    pub const TRACK_RADIUS: i32 = 2;
    pub const THUMB_SIZE: i32 = 16;
    pub const THUMB_RADIUS: i32 = 8;
    pub const BUTTON_SIZE: i32 = 24;
    pub const BUTTON_RADIUS: i32 = 4;
}

pub mod switch {
    pub const WIDTH: i32 = 36;
    pub const HEIGHT: i32 = 18;
    pub const TRACK_RADIUS: i32 = 9;
    pub const THUMB_SIZE: i32 = 16;
    pub const THUMB_RADIUS: i32 = 8;
    pub const BORDER_WIDTH: i32 = 1;
}

pub mod progress {
    pub const HEIGHT: i32 = 4;
    pub const RADIUS: i32 = 2;
}

/// T-shirt sized font tokens for consistent typography
pub mod font {
    pub const TINY: i32 = 8;
    pub const XS: i32 = 9;
    pub const SM: i32 = 11;
    pub const MD: i32 = 13;
    pub const LG: i32 = 15;
    pub const XL: i32 = 17;
    pub const XXL: i32 = 20;
}

/// T-shirt sized border radius tokens for consistent rounding
pub mod radius {
    pub const XS: i32 = 4;
    pub const SM: i32 = 8;
    pub const MD: i32 = 12;
    pub const LG: i32 = 16;
    pub const PILL: i32 = 9999;
}

/// Legacy rounding constants (prefer `radius::*` for new code)
pub mod rounding {
    pub const VERY_SMALL: i32 = 8;
    pub const SMALL: i32 = 12;
    pub const NORMAL: i32 = 17;
    pub const LARGE: i32 = 23;
    pub const VERY_LARGE: i32 = 30;
    pub const FULL: i32 = 9999;
}

/// Keyboard shortcut hint constants (used by `kbd.rs`, `result_item.rs`)
pub mod kbd {
    use super::{font, radius, spacing};

    pub const FONT_SIZE: i32 = font::XS; // 9
    pub const PADDING_HORIZONTAL: i32 = spacing::XS; // 4
    pub const PADDING_VERTICAL: i32 = spacing::XXXS; // 2
    pub const RADIUS: i32 = radius::XS; // 4
    // Aliases for kbd.rs compatibility
    pub const PADDING_H: i32 = PADDING_HORIZONTAL;
    pub const PADDING_V: i32 = PADDING_VERTICAL;
    pub const BORDER_RADIUS: i32 = RADIUS;
}

pub mod animation {
    pub const BACKGROUND_COLOR: u32 = 200;
    pub const ACTION_HINT_OPACITY: u32 = 150;
    pub const ACTION_BUTTON_BG: u32 = 100;
    pub const RIPPLE_DURATION: u32 = 1200;
    pub const PROGRESS_BAR: u32 = 200;
}

pub mod opacity {
    pub const RUNNING_INDICATOR: f64 = 0.7;
    pub const ACTION_ICON_NORMAL: f64 = 0.8;
    pub const ACTION_ICON_FOCUSED: f64 = 1.0;
}

pub mod result_list {
    pub const ITEM_MARGIN_HORIZONTAL: i32 = 4;
    pub const CONTAINER_PADDING: i32 = 8;
}

/// Search bar specific constants (used by window.rs)
pub mod search_bar {
    use super::{font, icon, rounding, spacing};

    pub const ICON_SIZE: i32 = icon::MD; // 20
    pub const ICON_CONTAINER_SIZE: i32 = icon::XL; // 32 (mapped from 30)
    pub const CARET_TOGGLE_SIZE: i32 = icon::LG; // 24
    pub const ROW_SPACING: i32 = spacing::SM; // 8 (mapped from 10)
    pub const ROW_MARGIN: i32 = spacing::MD; // 12
    pub const RADIUS: i32 = rounding::NORMAL; // 17
    pub const FONT_SIZE_SMALL: i32 = font::SM; // 11
    pub const FONT_SIZE_SEARCH: i32 = font::MD; // 13 (mapped from 12)
    pub const FONT_SIZE_NORMAL: i32 = font::LG; // 15 (mapped from 14)
    pub const FONT_SIZE_LARGE: i32 = font::XL; // 17 (mapped from 16)
}

/// FAB (Floating Action Button) window constants
pub mod fab {
    use super::{icon, radius, spacing};

    pub const ICON_SIZE: i32 = icon::LG; // 24
    pub const BUTTON_SIZE: i32 = icon::LG; // 24
    pub const BORDER_RADIUS: i32 = radius::LG; // 16 (nearest to original 28)
    pub const SCREEN_MARGIN: i32 = spacing::SM; // 8 (nearest to original 10)
}

/// Action bar constants (used by `action_bar.rs`)
pub mod action_bar {
    use super::{icon, spacing};

    pub const HEIGHT_NORMAL: i32 = 28;
    pub const HEIGHT_AMBIENT: i32 = 26;
    pub const BUTTON_SPACING: i32 = spacing::SM; // 8 (mapped from 6)
    pub const BUTTON_SIZE: i32 = icon::MATERIAL_SIZE; // 26
}

/// Ambient item constants (used by `ambient_item.rs`)
pub mod ambient_item {
    use super::{icon, radius, spacing};

    pub const HEIGHT: i32 = 26;
    pub const RADIUS: i32 = radius::SM; // 8
    pub const ICON_SIZE: i32 = icon::SM; // 16 (mapped from 14)
    pub const ACTION_SIZE: i32 = icon::MD; // 20
    pub const ACTION_ICON_SIZE: i32 = icon::XS; // 12
    pub const SPACING: i32 = spacing::XS; // 4
    pub const PADDING_H: i32 = spacing::XS; // 4
}

/// Ripple button constants (used by `ripple_button.rs`)
pub mod ripple_button {
    use super::{icon, radius, spacing};

    pub const SIZE: i32 = 28;
    pub const ICON_SIZE: i32 = icon::MD; // 20
    pub const RADIUS: i32 = radius::SM; // 8 (mapped from 6)
    pub const TOOLTIP_OFFSET: i32 = spacing::XS; // 4
}

/// Keybinding map popup constants (used by `keybinding_map.rs`)
pub mod keybinding_map {
    use super::{radius, spacing};

    pub const PADDING: i32 = spacing::MD; // 12
    pub const SECTION_SPACING: i32 = spacing::SM; // 8
    pub const ROW_SPACING: i32 = spacing::XS; // 4
    pub const COLUMN_SPACING: i32 = spacing::LG; // 16
    pub const BORDER_RADIUS: i32 = radius::SM; // 8
}

/// Preview panel constants (used by `preview_panel.rs`)
pub mod preview_panel {
    use super::spacing;

    // Width is 75% of launcher width (640 * 0.75 = 480)
    pub const WIDTH: i32 = 480;
    pub const PADDING_LEFT: i32 = spacing::XL; // 20
    pub const PADDING_RIGHT: i32 = spacing::MD; // 12
    pub const PADDING_TOP: i32 = spacing::MD; // 12
    pub const PADDING_BOTTOM: i32 = spacing::MD; // 12
    pub const IMAGE_MAX_HEIGHT: i32 = 200;
    pub const METADATA_ROW_SPACING: i32 = spacing::XS; // 4
    pub const METADATA_COLUMN_SPACING: i32 = spacing::SM; // 8
    pub const ACTIONS_SPACING: i32 = spacing::SM; // 8
}

/// Gauge widget constants (used by gauge.rs)
pub mod gauge {
    use super::{font, icon, spacing};

    pub const SIZE: i32 = icon::CONTAINER_SIZE; // 40
    pub const STROKE_WIDTH: f64 = spacing::XS as f64; // 4.0
    pub const START_ANGLE: f64 = 135.0; // degrees (bottom-left)
    pub const SWEEP_ANGLE: f64 = 270.0; // degrees (270° arc)
    pub const LABEL_FONT_SIZE: f64 = font::XS as f64; // 9.0
}

/// Graph/sparkline widget constants (used by graph.rs)
pub mod graph {
    use super::{icon, spacing};

    pub const SIZE: i32 = icon::CONTAINER_SIZE; // 40
    pub const PADDING: f64 = spacing::XXXS as f64; // 2.0
    pub const LINE_WIDTH: f64 = spacing::XXXS as f64; // 2.0
}

/// Grid item constants (used by `grid_item.rs`, `result_grid.rs`)
pub mod grid {
    use super::{radius, spacing};

    pub const ITEM_WIDTH: i32 = 140;
    pub const HIGHLIGHT_SIZE: i32 = 128;
    pub const VISUAL_SIZE: i32 = 120;
    pub const MAX_ACTION_BUTTONS: usize = 3;
    // CSS constants mapped to tokens
    pub const BORDER_RADIUS: i32 = radius::SM; // 8
    pub const PADDING: i32 = spacing::SM; // 8
    pub const ACTION_SPACING: i32 = spacing::XS; // 4
    pub const NAME_MARGIN_TOP: i32 = spacing::XS; // 4
    pub const ACTION_BUTTON_SIZE: i32 = 32;
    pub const ACTION_BUTTON_PADDING: i32 = spacing::XS; // 4
    pub const ACTION_BUTTON_RADIUS: i32 = 6;
    pub const IMAGE_BORDER_RADIUS: i32 = 6;
}
