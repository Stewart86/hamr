//! Window picker dialog rendering.

use crate::colors;
use crate::state::WindowPickerState;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

/// Render window picker dialog for selecting between multiple windows.
// Window count is usize, terminal dimensions are u16
#[allow(clippy::cast_possible_truncation)]
pub fn render_window_picker(f: &mut Frame, picker_state: &WindowPickerState) {
    let area = f.area();

    let bg = Block::default().style(Style::default().bg(colors::BG));
    f.render_widget(bg, area);

    let picker_width = 60.min(area.width.saturating_sub(4));
    let picker_height = (picker_state.windows.len() as u16 + 6).min(area.height.saturating_sub(4));

    let x = (area.width.saturating_sub(picker_width)) / 2;
    let y = (area.height.saturating_sub(picker_height)) / 2;
    let picker_area = Rect::new(x, y, picker_width, picker_height);

    f.render_widget(Clear, picker_area);

    let title = format!(" Select Window: {} ", picker_state.app_name);
    let picker_block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(
            Style::default()
                .fg(colors::ON_SURFACE)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(colors::SURFACE))
        .border_style(Style::default().fg(colors::PRIMARY));

    f.render_widget(picker_block.clone(), picker_area);

    let inner = picker_block.inner(picker_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(inner);

    let items: Vec<ListItem> = picker_state
        .windows
        .iter()
        .enumerate()
        .map(|(i, window)| {
            let is_selected = i == picker_state.selected;

            let prefix = format!("[{}] ", i + 1);
            let workspace = format!(" (Workspace {})", window.workspace);

            let max_title_len = inner.width as usize - prefix.len() - workspace.len() - 2;
            let title = if window.title.len() > max_title_len {
                format!("{}...", &window.title[..max_title_len.saturating_sub(3)])
            } else {
                window.title.clone()
            };

            let style = if is_selected {
                Style::default()
                    .fg(colors::ON_SURFACE)
                    .bg(colors::PRIMARY_CONTAINER)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors::SUBTEXT)
            };

            let line = Line::from(vec![
                Span::styled(prefix, Style::default().fg(colors::PRIMARY)),
                Span::styled(title, style),
                Span::styled(workspace, Style::default().fg(colors::OUTLINE)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, chunks[0]);

    let help_text = Line::from(vec![
        Span::styled("j/k", Style::default().fg(colors::PRIMARY)),
        Span::styled(": navigate  ", Style::default().fg(colors::SUBTEXT)),
        Span::styled("Enter/1-9", Style::default().fg(colors::PRIMARY)),
        Span::styled(": select  ", Style::default().fg(colors::SUBTEXT)),
        Span::styled("Esc", Style::default().fg(colors::PRIMARY)),
        Span::styled(": cancel", Style::default().fg(colors::SUBTEXT)),
    ]);
    f.render_widget(Paragraph::new(help_text), chunks[1]);
}
