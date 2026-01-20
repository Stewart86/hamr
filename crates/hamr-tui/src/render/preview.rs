//! Preview panel rendering.

use crate::app::App;
use crate::colors;
use crate::render::helpers::render_markdown_to_lines;
use crate::widgets;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// Render the preview panel on the right side of the split view.
pub fn render_preview_panel(f: &mut Frame, app: &App, area: Rect) {
    let Some(preview) = app.get_selected_preview() else {
        return;
    };

    let title = preview.title.as_deref().unwrap_or("Preview");
    let preview_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .title_style(
            Style::default()
                .fg(colors::ON_SURFACE)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(colors::SURFACE))
        .border_style(Style::default().fg(colors::PRIMARY));

    f.render_widget(preview_block.clone(), area);
    let inner = preview_block.inner(area);

    let mut lines: Vec<Line> = Vec::new();

    if let Some(image) = &preview.image {
        lines.push(Line::from(vec![
            Span::styled("Image: ", Style::default().fg(colors::OUTLINE)),
            Span::styled(image.clone(), Style::default().fg(colors::SUBTEXT)),
        ]));
        lines.push(Line::from(""));
    }

    if let Some(md) = &preview.markdown {
        lines.extend(render_markdown_to_lines(md, inner.width as usize));
        lines.push(Line::from(""));
    } else if let Some(content) = &preview.content {
        for line in content.lines() {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(colors::ON_SURFACE),
            )));
        }
        lines.push(Line::from(""));
    }

    if !preview.metadata.is_empty() {
        lines.push(Line::from(Span::styled(
            "-".repeat(inner.width as usize),
            Style::default().fg(colors::OUTLINE),
        )));

        for meta in &preview.metadata {
            let icon = meta
                .icon
                .as_ref()
                .map(|i| format!("{} ", widgets::icon_to_str(i)))
                .unwrap_or_default();

            lines.push(Line::from(vec![
                Span::styled(icon, Style::default().fg(colors::OUTLINE)),
                Span::styled(meta.label.clone(), Style::default().fg(colors::SUBTEXT)),
                Span::styled(": ", Style::default().fg(colors::OUTLINE)),
                Span::styled(meta.value.clone(), Style::default().fg(colors::ON_SURFACE)),
            ]));
        }
    }

    if !preview.actions.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "-".repeat(inner.width as usize),
            Style::default().fg(colors::OUTLINE),
        )));

        let action_spans: Vec<Span> = preview
            .actions
            .iter()
            .enumerate()
            .flat_map(|(i, a)| {
                let is_selected = i == app.selected_preview_action;
                let style = if is_selected {
                    Style::default().fg(colors::BG).bg(colors::PRIMARY)
                } else {
                    Style::default().fg(colors::ON_SURFACE)
                };
                vec![
                    Span::styled(format!("{}:", i + 1), Style::default().fg(colors::PRIMARY)),
                    Span::styled(a.name.clone(), style),
                    Span::raw(" "),
                ]
            })
            .collect();
        lines.push(Line::from(action_spans));
    }

    let visible: Vec<Line> = lines
        .into_iter()
        .skip(app.preview_scroll)
        .take(inner.height as usize)
        .collect();

    let para = Paragraph::new(visible).wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(para, inner);
}
