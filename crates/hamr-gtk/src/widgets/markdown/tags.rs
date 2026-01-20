//! `TextTag` definitions for markdown rendering.
//!
//! Spacing philosophy:
//! - Headings: Large top margin (16-24px) creates clear section breaks,
//!   smaller bottom margin (8-12px) keeps content connected to its heading
//! - Paragraphs: Moderate spacing (8px) for comfortable reading rhythm
//! - Lists: Consistent left indent (24px base), slight vertical padding (2-4px)
//! - Code blocks: Generous padding (12-16px) for visual separation
//! - Blockquotes/Alerts: Left indent suggests quotation, vertical padding (8px)

use gtk4::prelude::*;
use gtk4::{TextTag, TextTagTable};

/// Tag names for markdown elements
pub mod names {
    pub const HEADING1: &str = "h1";
    pub const HEADING2: &str = "h2";
    pub const HEADING3: &str = "h3";
    pub const HEADING4: &str = "h4";
    pub const HEADING5: &str = "h5";
    pub const HEADING6: &str = "h6";
    pub const BOLD: &str = "bold";
    pub const ITALIC: &str = "italic";
    pub const BOLD_ITALIC: &str = "bold-italic";
    pub const STRIKETHROUGH: &str = "strikethrough";
    pub const CODE_INLINE: &str = "code-inline";
    pub const CODE_BLOCK: &str = "code-block";
    pub const LINK: &str = "link";
    pub const BLOCKQUOTE: &str = "blockquote";
    pub const LIST_ITEM: &str = "list-item";
    pub const TASK_CHECKED: &str = "task-checked";
    pub const TASK_UNCHECKED: &str = "task-unchecked";
    pub const SUBSCRIPT: &str = "subscript";
    pub const SUPERSCRIPT: &str = "superscript";
    pub const INSERTED: &str = "inserted";
    pub const TABLE_HEADER: &str = "table-header";
    pub const TABLE_CELL: &str = "table-cell";
    pub const ALERT_NOTE: &str = "alert-note";
    pub const ALERT_TIP: &str = "alert-tip";
    pub const ALERT_IMPORTANT: &str = "alert-important";
    pub const ALERT_WARNING: &str = "alert-warning";
    pub const ALERT_CAUTION: &str = "alert-caution";
}

/// Spacing constants for consistent vertical rhythm (in pixels)
mod spacing {
    /// Large spacing for major section breaks (H1, H2)
    pub const LARGE: i32 = 20;
    /// Medium spacing for subsection breaks (H3, H4)
    pub const MEDIUM: i32 = 14;
    /// Small spacing for minor breaks (H5, H6, blocks)
    pub const SMALL: i32 = 10;
    /// Tight spacing for inline elements
    pub const TIGHT: i32 = 6;
    /// Minimal spacing between related content
    pub const MINIMAL: i32 = 4;
}

/// Create a `TextTagTable` with all markdown-related tags
pub fn create_tag_table() -> TextTagTable {
    let table = TextTagTable::new();

    // Headings - larger top margin creates clear section breaks
    // H1: Major section (24px above, 10px below)
    table.add(&create_heading_tag(
        names::HEADING1,
        1.75,
        800,
        spacing::LARGE + 4,
        spacing::SMALL,
    ));
    // H2: Subsection (20px above, 8px below)
    table.add(&create_heading_tag(
        names::HEADING2,
        1.45,
        700,
        spacing::LARGE,
        spacing::TIGHT + 2,
    ));
    // H3: Minor section (16px above, 6px below)
    table.add(&create_heading_tag(
        names::HEADING3,
        1.25,
        700,
        spacing::MEDIUM + 2,
        spacing::TIGHT,
    ));
    // H4-H6: Smaller headings with proportionally less spacing
    table.add(&create_heading_tag(
        names::HEADING4,
        1.1,
        600,
        spacing::MEDIUM,
        spacing::MINIMAL,
    ));
    table.add(&create_heading_tag(
        names::HEADING5,
        1.0,
        600,
        spacing::SMALL,
        spacing::MINIMAL,
    ));
    table.add(&create_heading_tag(
        names::HEADING6,
        0.95,
        600,
        spacing::SMALL,
        spacing::MINIMAL,
    ));

    // Inline styles
    table.add(&create_bold_tag());
    table.add(&create_italic_tag());
    table.add(&create_bold_italic_tag());
    table.add(&create_strikethrough_tag());
    table.add(&create_code_inline_tag());
    table.add(&create_code_block_tag());
    table.add(&create_link_tag());
    table.add(&create_blockquote_tag());
    table.add(&create_list_item_tag());

    // Task list
    table.add(&create_task_checked_tag());
    table.add(&create_task_unchecked_tag());

    // HTML tags
    table.add(&create_subscript_tag());
    table.add(&create_superscript_tag());
    table.add(&create_inserted_tag());

    // Tables
    table.add(&create_table_header_tag());
    table.add(&create_table_cell_tag());

    // GFM Alerts
    table.add(&create_alert_note_tag());
    table.add(&create_alert_tip_tag());
    table.add(&create_alert_important_tag());
    table.add(&create_alert_warning_tag());
    table.add(&create_alert_caution_tag());

    table
}

fn create_heading_tag(
    name: &str,
    scale: f64,
    weight: i32,
    pixels_above: i32,
    pixels_below: i32,
) -> TextTag {
    TextTag::builder()
        .name(name)
        .scale(scale)
        .weight(weight)
        .pixels_above_lines(pixels_above)
        .pixels_below_lines(pixels_below)
        .build()
}

fn create_bold_tag() -> TextTag {
    TextTag::builder().name(names::BOLD).weight(700).build()
}

fn create_italic_tag() -> TextTag {
    TextTag::builder()
        .name(names::ITALIC)
        .style(gtk4::pango::Style::Italic)
        .build()
}

fn create_bold_italic_tag() -> TextTag {
    TextTag::builder()
        .name(names::BOLD_ITALIC)
        .weight(700)
        .style(gtk4::pango::Style::Italic)
        .build()
}

fn create_strikethrough_tag() -> TextTag {
    TextTag::builder()
        .name(names::STRIKETHROUGH)
        .strikethrough(true)
        .build()
}

fn create_code_inline_tag() -> TextTag {
    TextTag::builder()
        .name(names::CODE_INLINE)
        .family("monospace")
        .scale(0.92)
        .foreground("#e8b4b8")
        .background("rgba(255, 255, 255, 0.08)")
        .build()
}

fn create_code_block_tag() -> TextTag {
    // Legacy tag - code blocks now use embedded widgets
    // Kept for backwards compatibility
    TextTag::builder()
        .name(names::CODE_BLOCK)
        .family("monospace")
        .scale(0.9)
        .background("rgba(128, 128, 128, 0.12)")
        .paragraph_background("rgba(128, 128, 128, 0.12)")
        .left_margin(16)
        .right_margin(16)
        .pixels_above_lines(spacing::TIGHT)
        .pixels_below_lines(spacing::TIGHT)
        .pixels_inside_wrap(2)
        .build()
}

fn create_link_tag() -> TextTag {
    TextTag::builder()
        .name(names::LINK)
        .foreground("#7cacf8")
        .underline(gtk4::pango::Underline::Single)
        .build()
}

fn create_blockquote_tag() -> TextTag {
    TextTag::builder()
        .name(names::BLOCKQUOTE)
        .foreground("rgba(180, 180, 180, 1.0)")
        .paragraph_background("rgba(100, 100, 100, 0.08)")
        .left_margin(20)
        .right_margin(8)
        .pixels_above_lines(spacing::TIGHT)
        .pixels_below_lines(spacing::TIGHT)
        .pixels_inside_wrap(2)
        .style(gtk4::pango::Style::Italic)
        .build()
}

fn create_list_item_tag() -> TextTag {
    TextTag::builder()
        .name(names::LIST_ITEM)
        .left_margin(24)
        .pixels_above_lines(spacing::MINIMAL / 2)
        .pixels_below_lines(spacing::MINIMAL / 2)
        .build()
}

fn create_task_checked_tag() -> TextTag {
    TextTag::builder()
        .name(names::TASK_CHECKED)
        .foreground("#4caf50")
        .family("Material Symbols Rounded")
        .rise(-2048) // Lower the icon to align with text baseline (Pango units)
        .build()
}

fn create_task_unchecked_tag() -> TextTag {
    TextTag::builder()
        .name(names::TASK_UNCHECKED)
        .foreground("rgba(150, 150, 150, 1.0)")
        .family("Material Symbols Rounded")
        .rise(-2048) // Lower the icon to align with text baseline (Pango units)
        .build()
}

fn create_subscript_tag() -> TextTag {
    TextTag::builder()
        .name(names::SUBSCRIPT)
        .rise(-4096) // Pango units (negative = below baseline)
        .scale(0.75)
        .build()
}

fn create_superscript_tag() -> TextTag {
    TextTag::builder()
        .name(names::SUPERSCRIPT)
        .rise(6144) // Pango units (positive = above baseline)
        .scale(0.75)
        .build()
}

fn create_inserted_tag() -> TextTag {
    TextTag::builder()
        .name(names::INSERTED)
        .underline(gtk4::pango::Underline::Single)
        .build()
}

fn create_table_header_tag() -> TextTag {
    TextTag::builder()
        .name(names::TABLE_HEADER)
        .weight(700)
        .family("monospace")
        .background("rgba(100, 100, 100, 0.15)")
        .build()
}

fn create_table_cell_tag() -> TextTag {
    TextTag::builder()
        .name(names::TABLE_CELL)
        .family("monospace")
        .build()
}

fn create_alert_tag(name: &str, color: &str, bg_color: &str) -> TextTag {
    TextTag::builder()
        .name(name)
        .foreground(color)
        .paragraph_background(bg_color)
        .left_margin(20)
        .right_margin(8)
        .pixels_above_lines(spacing::TIGHT)
        .pixels_below_lines(spacing::TIGHT)
        .pixels_inside_wrap(2)
        .build()
}

fn create_alert_note_tag() -> TextTag {
    create_alert_tag(names::ALERT_NOTE, "#58a6ff", "rgba(88, 166, 255, 0.08)")
}

fn create_alert_tip_tag() -> TextTag {
    create_alert_tag(names::ALERT_TIP, "#3fb950", "rgba(63, 185, 80, 0.08)")
}

fn create_alert_important_tag() -> TextTag {
    create_alert_tag(
        names::ALERT_IMPORTANT,
        "#a371f7",
        "rgba(163, 113, 247, 0.08)",
    )
}

fn create_alert_warning_tag() -> TextTag {
    create_alert_tag(names::ALERT_WARNING, "#d29922", "rgba(210, 153, 34, 0.08)")
}

fn create_alert_caution_tag() -> TextTag {
    create_alert_tag(names::ALERT_CAUTION, "#f85149", "rgba(248, 81, 73, 0.08)")
}

/// Get heading tag name for a given level (1-6)
pub fn heading_tag_name(level: u8) -> &'static str {
    match level {
        1 => names::HEADING1,
        2 => names::HEADING2,
        3 => names::HEADING3,
        4 => names::HEADING4,
        5 => names::HEADING5,
        _ => names::HEADING6,
    }
}

const LINK_URL_KEY: &str = "link-url";

/// Create a link tag with a specific URL stored on it
pub fn create_link_tag_with_url(tag_table: &TextTagTable, url: &str, link_id: usize) -> String {
    let tag_name = format!("link-{link_id}");

    let tag = TextTag::builder()
        .name(&tag_name)
        .foreground("#7cacf8")
        .underline(gtk4::pango::Underline::Single)
        .build();

    unsafe {
        tag.set_data(LINK_URL_KEY, url.to_string());
    }

    tag_table.add(&tag);
    tag_name
}

/// Get the URL stored on a link tag
pub fn get_link_url(tag: &TextTag) -> Option<String> {
    unsafe { tag.data::<String>(LINK_URL_KEY).map(|p| p.as_ref().clone()) }
}
