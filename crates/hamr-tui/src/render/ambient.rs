//! Ambient bar rendering.

use crate::app::App;
use crate::colors;
use crate::widgets;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// Render the ambient bar showing background notifications/items.
pub fn render_ambient_bar(f: &mut Frame, app: &App, area: Rect) {
    let ambient_items: Vec<_> = app.get_all_ambient_items();
    let mut spans: Vec<Span> = vec![];

    for (i, item) in ambient_items.iter().enumerate() {
        let is_selected = i == app.selected_ambient;
        let style = if is_selected {
            Style::default()
                .fg(colors::PRIMARY)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors::ON_SURFACE)
        };

        if i > 0 {
            spans.push(Span::styled(" | ", Style::default().fg(colors::OUTLINE)));
        }

        if is_selected {
            spans.push(Span::styled("> ", Style::default().fg(colors::PRIMARY)));
        } else {
            spans.push(Span::raw("  "));
        }

        if let Some(icon) = &item.icon {
            spans.push(Span::styled(
                format!("{} ", widgets::icon_to_str(icon)),
                Style::default().fg(colors::OUTLINE),
            ));
        }

        spans.push(Span::styled(&item.name, style));

        if let Some(desc) = &item.description {
            spans.push(Span::styled(
                format!(" - {desc}"),
                Style::default().fg(colors::SUBTEXT),
            ));
        }

        if item.duration > 0 {
            spans.push(Span::styled(" [T]", Style::default().fg(colors::OUTLINE)));
        }

        for badge in item.badges.iter().take(2) {
            if let Some(text) = badge.text.as_deref() {
                spans.push(Span::styled(
                    format!(" [{text}]"),
                    Style::default().fg(colors::SECONDARY),
                ));
            }
        }

        if is_selected && !item.actions.is_empty() {
            spans.push(Span::raw(" "));
            for (idx, action) in item.actions.iter().take(3).enumerate() {
                spans.push(Span::styled(
                    format!("{}:", idx + 1),
                    Style::default().fg(colors::PRIMARY),
                ));
                spans.push(Span::styled(
                    &action.name,
                    Style::default().fg(colors::ON_SURFACE),
                ));
                spans.push(Span::raw(" "));
            }
        }
    }

    if !ambient_items.is_empty() {
        spans.push(Span::styled(
            " [x:dismiss]",
            Style::default().fg(colors::OUTLINE),
        ));
    }

    let ambient_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Ambient ({}) ", ambient_items.len()))
        .style(Style::default().bg(colors::SURFACE_HIGH))
        .border_style(Style::default().fg(colors::SUCCESS));

    let ambient_text = Paragraph::new(Line::from(spans)).block(ambient_block);
    f.render_widget(ambient_text, area);
}
