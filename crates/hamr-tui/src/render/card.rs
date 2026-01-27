//! Card modal rendering.

use crate::colors;
use crate::render::helpers::render_markdown_to_lines;
use crate::state::CardState;
use hamr_rpc::{Action, CardBlock};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

fn render_card_blocks(blocks: &[CardBlock], width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for block in blocks {
        match block {
            CardBlock::Pill { text } => {
                lines.push(Line::from(vec![Span::styled(
                    format!(" {text} "),
                    Style::default().fg(colors::BG).bg(colors::PRIMARY),
                )]));
            }
            CardBlock::Separator => {
                lines.push(Line::from(Span::styled(
                    "-".repeat(width),
                    Style::default().fg(colors::OUTLINE),
                )));
            }
            CardBlock::Message { role, content } => {
                let prefix = match role.as_str() {
                    "user" => ("You: ", colors::PRIMARY),
                    "assistant" => ("AI: ", colors::SUCCESS),
                    "system" => ("Sys: ", colors::OUTLINE),
                    _ => ("", colors::ON_SURFACE),
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        prefix.0.to_string(),
                        Style::default().fg(prefix.1).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(content.clone(), Style::default().fg(colors::ON_SURFACE)),
                ]));
            }
            CardBlock::Note { content } => {
                lines.push(Line::from(vec![
                    Span::styled("i ".to_string(), Style::default().fg(colors::PRIMARY)),
                    Span::styled(content.clone(), Style::default().fg(colors::SUBTEXT)),
                ]));
            }
        }
        lines.push(Line::from(""));
    }
    lines
}

fn render_card_actions(f: &mut Frame, actions: &[Action], selected: usize, area: Rect) {
    let mut spans: Vec<Span<'static>> = vec![Span::raw(" ")];
    for (i, action) in actions.iter().enumerate() {
        let style = if i == selected {
            Style::default()
                .fg(colors::BG)
                .bg(colors::PRIMARY)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors::PRIMARY)
        };
        spans.push(Span::styled(format!(" [{}] ", action.name), style));
        spans.push(Span::raw(" "));
    }
    let close_style = if selected == actions.len() {
        Style::default()
            .fg(colors::BG)
            .bg(colors::ERROR)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors::ERROR)
    };
    spans.push(Span::styled(" [Close] ".to_string(), close_style));
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Render card modal with markdown support.
// Terminal dimensions are u16, percentage calc uses f32
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn render_card(f: &mut Frame, card_state: &CardState) {
    let area = f.area();
    let bg = Block::default().style(Style::default().bg(colors::BG));
    f.render_widget(bg, area);

    let card_width = 80.min(area.width.saturating_sub(4));
    let card_height = (f32::from(area.height) * 0.8) as u16;
    let x = (area.width.saturating_sub(card_width)) / 2;
    let y = (area.height.saturating_sub(card_height)) / 2;
    let card_area = Rect::new(x, y, card_width, card_height);

    f.render_widget(Clear, card_area);

    let card_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", card_state.card.title))
        .title_style(
            Style::default()
                .fg(colors::ON_SURFACE)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(colors::SURFACE))
        .border_style(Style::default().fg(colors::PRIMARY));
    f.render_widget(card_block.clone(), card_area);

    let inner = card_block.inner(card_area);
    let content_height = inner.height.saturating_sub(3);

    let content: Vec<Line<'static>> = if let Some(md) = &card_state.card.markdown {
        render_markdown_to_lines(md, inner.width as usize)
    } else if let Some(content) = &card_state.card.content {
        content.lines().map(|l| Line::from(l.to_string())).collect()
    } else if card_state.card.kind.as_deref() == Some("blocks") {
        render_card_blocks(&card_state.card.blocks, inner.width as usize)
    } else {
        vec![]
    };

    let visible_lines: Vec<Line<'static>> = content
        .into_iter()
        .skip(card_state.scroll_offset)
        .take(content_height as usize)
        .collect();

    let content_para = Paragraph::new(visible_lines).wrap(ratatui::widgets::Wrap { trim: false });
    let content_area = Rect::new(inner.x, inner.y, inner.width, content_height);
    f.render_widget(content_para, content_area);

    let action_area = Rect::new(inner.x, inner.y + content_height, inner.width, 2);
    render_card_actions(
        f,
        &card_state.card.actions,
        card_state.selected_action,
        action_area,
    );

    let help_area = Rect::new(
        card_area.x,
        card_area.y + card_area.height,
        card_area.width,
        1,
    );
    if help_area.y < area.height {
        let help_text = Line::from(vec![
            Span::styled("j/k", Style::default().fg(colors::PRIMARY)),
            Span::styled(": scroll  ", Style::default().fg(colors::SUBTEXT)),
            Span::styled("Tab", Style::default().fg(colors::PRIMARY)),
            Span::styled(": action  ", Style::default().fg(colors::SUBTEXT)),
            Span::styled("Enter", Style::default().fg(colors::PRIMARY)),
            Span::styled(": select  ", Style::default().fg(colors::SUBTEXT)),
            Span::styled("Esc/q", Style::default().fg(colors::PRIMARY)),
            Span::styled(": close", Style::default().fg(colors::SUBTEXT)),
        ]);
        f.render_widget(Paragraph::new(help_text), help_area);
    }
}
