//! Helper rendering functions shared across views.

use crate::colors;
use crate::widgets;
use hamr_rpc::SliderValue;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// Render a slider as styled spans using the slider widget.
///
/// Returns a Vec of Spans for use in Line rendering.
pub fn render_slider_spans(
    slider: &SliderValue,
    width: usize,
    selected: bool,
) -> Vec<Span<'static>> {
    widgets::Slider::from_slider_value(
        slider.value,
        slider.min,
        slider.max,
        slider.step,
        None, // Don't pass display_value to avoid lifetime issues
    )
    .selected(selected)
    .render_inline(width)
}

/// Render a progress bar as a string.
// Progress percentage is f64, width is bounded by terminal size
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
pub fn render_progress_bar(value: f64, max: f64, width: usize) -> String {
    let pct = if max > 0.0 { value / max } else { 0.0 };
    let filled = (pct * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("[{}{}]", "=".repeat(filled), "-".repeat(empty))
}

/// Render markdown content to lines for TUI display.
pub fn render_markdown_to_lines(md: &str, width: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    for line in md.lines() {
        let trimmed = line.trim();

        if let Some(header) = trimmed.strip_prefix("### ") {
            lines.push(Line::from(Span::styled(
                header.to_string(),
                Style::default()
                    .fg(colors::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if let Some(header) = trimmed.strip_prefix("## ") {
            lines.push(Line::from(Span::styled(
                header.to_string(),
                Style::default()
                    .fg(colors::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if let Some(header) = trimmed.strip_prefix("# ") {
            lines.push(Line::from(Span::styled(
                header.to_string(),
                Style::default()
                    .fg(colors::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )));
        }
        // Bullet points
        else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let content = &trimmed[2..];
            let truncated = if content.len() > width.saturating_sub(4) {
                format!(
                    "  * {}...",
                    &content[..width.saturating_sub(7).min(content.len())]
                )
            } else {
                format!("  * {content}")
            };
            lines.push(Line::from(Span::styled(
                truncated,
                Style::default().fg(colors::ON_SURFACE),
            )));
        } else if trimmed.starts_with("```") {
            lines.push(Line::from(Span::styled(
                "-".repeat(width.min(40)),
                Style::default().fg(colors::OUTLINE),
            )));
        } else if trimmed.starts_with("**") && trimmed.ends_with("**") && trimmed.len() > 4 {
            let content = &trimmed[2..trimmed.len() - 2];
            lines.push(Line::from(Span::styled(
                content.to_string(),
                Style::default()
                    .fg(colors::ON_SURFACE)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if !trimmed.is_empty() {
            let truncated = if trimmed.len() > width {
                format!(
                    "{}...",
                    &trimmed[..width.saturating_sub(3).min(trimmed.len())]
                )
            } else {
                trimmed.to_string()
            };
            lines.push(Line::from(Span::styled(
                truncated,
                Style::default().fg(colors::ON_SURFACE),
            )));
        } else {
            lines.push(Line::from(""));
        }
    }

    lines
}
