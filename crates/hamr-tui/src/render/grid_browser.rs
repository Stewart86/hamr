//! Grid browser rendering.

use crate::colors;
use crate::state::GridBrowserState;
use crate::widgets;
use hamr_rpc::GridItem;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// Truncate text to max length with ellipsis.
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() > max_len {
        format!("{}...", &text[..max_len.saturating_sub(3)])
    } else {
        text.to_string()
    }
}

/// Build spans for a single grid cell (name line and description line).
fn build_cell_spans(
    item: &GridItem,
    is_selected: bool,
    cell_width: usize,
) -> (Vec<Span<'static>>, Vec<Span<'static>>) {
    let icon = item
        .icon
        .as_ref()
        .map_or("[.]", |i| widgets::icon_to_str(i));
    let max_name_len = cell_width.saturating_sub(5);
    let name = truncate_text(&item.name, max_name_len);

    let style = if is_selected {
        Style::default()
            .fg(colors::ON_SURFACE)
            .bg(colors::SURFACE_HIGH)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors::ON_SURFACE)
    };
    let icon_style = if is_selected {
        Style::default()
            .fg(colors::PRIMARY)
            .bg(colors::SURFACE_HIGH)
    } else {
        Style::default().fg(colors::OUTLINE)
    };

    let line1 = vec![
        Span::styled(format!("{icon} "), icon_style),
        Span::styled(
            format!("{:width$}", name, width = cell_width.saturating_sub(5)),
            style,
        ),
    ];

    let desc = item.description.as_deref().unwrap_or("");
    let desc_truncated = truncate_text(desc, cell_width.saturating_sub(2));
    let desc_style = if is_selected {
        Style::default()
            .fg(colors::SUBTEXT)
            .bg(colors::SURFACE_HIGH)
    } else {
        Style::default().fg(colors::SUBTEXT)
    };
    let line2 = vec![Span::styled(
        format!(
            "  {:width$}",
            desc_truncated,
            width = cell_width.saturating_sub(2)
        ),
        desc_style,
    )];

    (line1, line2)
}

/// Render the help bar at the bottom.
fn render_help_bar(f: &mut Frame, area: Rect) {
    let help_area = Rect::new(
        area.x,
        area.y + area.height.saturating_sub(1),
        area.width,
        1,
    );
    let help = Line::from(vec![
        Span::styled("<>^v/hjkl", Style::default().fg(colors::PRIMARY)),
        Span::styled(": nav  ", Style::default().fg(colors::SUBTEXT)),
        Span::styled("Enter", Style::default().fg(colors::PRIMARY)),
        Span::styled(": select  ", Style::default().fg(colors::SUBTEXT)),
        Span::styled("Esc", Style::default().fg(colors::PRIMARY)),
        Span::styled(": back", Style::default().fg(colors::SUBTEXT)),
    ]);
    f.render_widget(
        Paragraph::new(help).style(Style::default().bg(colors::SURFACE)),
        help_area,
    );
}

/// Render grid browser for displaying items in a grid layout.
// Grid math: usize indices to u16 terminal coords
#[allow(clippy::cast_possible_truncation)]
pub fn render_grid_browser(f: &mut Frame, state: &GridBrowserState) {
    let area = f.area();
    let bg = Block::default().style(Style::default().bg(colors::BG));
    f.render_widget(bg, area);

    let title = state.data.title.as_deref().unwrap_or("Grid Browser");
    let browser_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ({} items) ", title, state.data.items.len()))
        .style(Style::default().bg(colors::SURFACE))
        .border_style(Style::default().fg(colors::PRIMARY));

    let inner = browser_block.inner(area);
    f.render_widget(browser_block, area);

    if state.data.items.is_empty() {
        let empty_msg =
            Paragraph::new("No items to display").style(Style::default().fg(colors::SUBTEXT));
        f.render_widget(empty_msg, inner);
        return;
    }

    let columns = state.columns.max(1);
    let cell_width = (inner.width as usize / columns).max(10);
    let lines_per_item = 2;
    let visible_rows = (inner.height as usize).saturating_sub(1) / lines_per_item;
    let selected_row = state.selected / columns;
    let scroll_offset = if selected_row >= visible_rows {
        selected_row.saturating_sub(visible_rows) + 1
    } else {
        0
    };

    let mut y = inner.y;
    for row in scroll_offset..(scroll_offset + visible_rows) {
        if y >= inner.y + inner.height.saturating_sub(1) {
            break;
        }
        let mut line1_spans: Vec<Span> = Vec::new();
        let mut line2_spans: Vec<Span> = Vec::new();

        for col in 0..columns {
            let idx = row * columns + col;
            if let Some(item) = state.data.items.get(idx) {
                let is_selected = idx == state.selected;
                let (cell_line1, cell_line2) = build_cell_spans(item, is_selected, cell_width);
                line1_spans.extend(cell_line1);
                line2_spans.extend(cell_line2);
            } else {
                line1_spans.push(Span::raw(" ".repeat(cell_width)));
                line2_spans.push(Span::raw(" ".repeat(cell_width)));
            }
        }
        f.render_widget(
            Paragraph::new(Line::from(line1_spans)),
            Rect::new(inner.x, y, inner.width, 1),
        );
        if y + 1 < inner.y + inner.height {
            f.render_widget(
                Paragraph::new(Line::from(line2_spans)),
                Rect::new(inner.x, y + 1, inner.width, 1),
            );
        }
        y += lines_per_item as u16;
    }

    render_help_bar(f, area);
}
