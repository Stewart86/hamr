//! Error modal rendering.

use crate::{colors, state::ErrorState};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Render error modal centered on screen
// Terminal dimensions are u16, percentage calc uses f32
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn render_error(f: &mut Frame, error_state: &ErrorState) {
    let size = f.area();

    let dialog_width = (f32::from(size.width) * 0.5).min(80.0) as u16;
    let dialog_width = dialog_width.max(40);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((size.width.saturating_sub(dialog_width)) / 2),
            Constraint::Length(dialog_width),
            Constraint::Min(0),
        ])
        .split(size);

    let dialog_area = horizontal[1];

    let title_lines = 1;
    let plugin_lines = u16::from(error_state.plugin_id.is_some());
    let message_lines =
        estimate_wrapped_lines(&error_state.message, dialog_area.width.saturating_sub(4));
    let details_lines = error_state.details.as_ref().map_or(0, |d| {
        estimate_wrapped_lines(d, dialog_area.width.saturating_sub(4))
    });
    let footer_lines = 2;

    let total_height = (title_lines
        + plugin_lines
        + message_lines
        + if error_state.details.is_some() {
            details_lines + 1
        } else {
            0
        }
        + footer_lines
        + 4)
    .min(size.height.saturating_sub(2));

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((size.height.saturating_sub(total_height)) / 2),
            Constraint::Length(total_height),
            Constraint::Min(0),
        ])
        .split(size);

    let dialog_rect = Rect {
        x: dialog_area.x,
        y: vertical[1].y,
        width: dialog_area.width,
        height: vertical[1].height,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Error ")
        .title_alignment(Alignment::Center)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .style(Style::default().fg(colors::ERROR));

    let mut content_lines: Vec<Line> = Vec::new();

    content_lines.push(Line::from(Span::styled(
        &error_state.title,
        Style::default()
            .fg(colors::ERROR)
            .add_modifier(Modifier::BOLD),
    )));

    if let Some(plugin_id) = &error_state.plugin_id {
        content_lines.push(Line::from(Span::styled(
            format!("Plugin: {plugin_id}"),
            Style::default()
                .fg(colors::SUBTEXT)
                .add_modifier(Modifier::DIM),
        )));
    }

    content_lines.push(Line::from(""));

    for line in error_state.message.lines() {
        content_lines.push(Line::from(line));
    }

    if let Some(details) = &error_state.details {
        content_lines.push(Line::from(""));
        content_lines.push(Line::from(Span::styled(
            details,
            Style::default().fg(colors::SUBTEXT),
        )));
    }

    content_lines.push(Line::from(""));
    content_lines.push(Line::from(Span::styled(
        "[Esc to dismiss]",
        Style::default()
            .fg(colors::SUBTEXT)
            .add_modifier(Modifier::ITALIC),
    )));

    let paragraph = Paragraph::new(content_lines)
        .block(block)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);

    f.render_widget(paragraph, dialog_rect);
}

/// Estimate number of lines text will take when wrapped to given width
// Word length is usize, terminal width is u16
#[allow(clippy::cast_possible_truncation)]
fn estimate_wrapped_lines(text: &str, width: u16) -> u16 {
    if width == 0 {
        return 1;
    }

    let mut lines = 1;
    let mut current_line_len = 0;

    for word in text.split_whitespace() {
        let word_len = word.len() as u16;

        if current_line_len + word_len + 1 > width {
            lines += 1;
            current_line_len = word_len;
        } else {
            current_line_len += word_len + 1;
        }
    }

    lines
}
