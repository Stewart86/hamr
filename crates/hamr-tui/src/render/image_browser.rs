//! Image browser rendering.

use crate::colors;
use crate::state::ImageBrowserState;
use hamr_rpc::ImageItem;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// Truncate a path string with leading ellipsis if too long.
fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() > max_len {
        format!("...{}", &path[path.len().saturating_sub(max_len - 3)..])
    } else {
        path.to_string()
    }
}

/// Get the file type icon based on file extension.
fn get_file_icon(path: &str) -> &'static str {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => "[JPG]",
        "png" => "[PNG]",
        "gif" => "[GIF]",
        "webp" => "[WEB]",
        "svg" => "[SVG]",
        "bmp" => "[BMP]",
        "ico" => "[ICO]",
        "tiff" | "tif" => "[TIF]",
        _ => "[IMG]",
    }
}

/// Build the two display lines for an image item.
fn build_image_item_lines(
    img: &ImageItem,
    is_selected: bool,
    max_path_len: usize,
) -> (Line<'_>, Line<'_>) {
    let base_style = if is_selected {
        Style::default()
            .fg(colors::ON_SURFACE)
            .bg(colors::SURFACE_HIGH)
    } else {
        Style::default().fg(colors::ON_SURFACE)
    };

    let filename = img.name.as_deref().unwrap_or_else(|| {
        std::path::Path::new(&img.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&img.path)
    });

    let icon = get_file_icon(&img.path);
    let icon_style = if is_selected {
        Style::default()
            .fg(colors::PRIMARY)
            .bg(colors::SURFACE_HIGH)
    } else {
        Style::default().fg(colors::PRIMARY)
    };

    let name_style = if is_selected {
        base_style.add_modifier(Modifier::BOLD)
    } else {
        base_style
    };
    let path_style = if is_selected {
        Style::default()
            .fg(colors::SUBTEXT)
            .bg(colors::SURFACE_HIGH)
    } else {
        Style::default().fg(colors::SUBTEXT)
    };

    let path_display = truncate_path(&img.path, max_path_len);

    let line1 = Line::from(vec![
        Span::styled(format!(" {icon} "), icon_style),
        Span::styled(filename, name_style),
    ]);
    let line2 = Line::from(vec![
        Span::styled("     ", base_style),
        Span::styled(path_display, path_style),
    ]);

    (line1, line2)
}

/// Render the help bar at the bottom of the image browser.
fn render_help_bar(f: &mut Frame, area: Rect) {
    let help = Line::from(vec![
        Span::styled("^v/jk", Style::default().fg(colors::PRIMARY)),
        Span::styled(": nav  ", Style::default().fg(colors::SUBTEXT)),
        Span::styled("Enter", Style::default().fg(colors::PRIMARY)),
        Span::styled(": select  ", Style::default().fg(colors::SUBTEXT)),
        Span::styled("Esc", Style::default().fg(colors::PRIMARY)),
        Span::styled(": back", Style::default().fg(colors::SUBTEXT)),
    ]);
    f.render_widget(
        Paragraph::new(help).style(Style::default().bg(colors::SURFACE)),
        area,
    );
}

/// Render image browser for displaying image file listings.
// Grid math: usize indices to u16 terminal coords
#[allow(clippy::cast_possible_truncation)]
pub fn render_image_browser(f: &mut Frame, state: &ImageBrowserState) {
    let area = f.area();
    let bg = Block::default().style(Style::default().bg(colors::BG));
    f.render_widget(bg, area);

    let title = state.data.title.as_deref().unwrap_or("Images");
    let dir_display = truncate_path(state.data.directory.as_deref().unwrap_or(""), 30);

    let browser_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(
            " {} - {} ({} files) ",
            title,
            dir_display,
            state.data.images.len()
        ))
        .style(Style::default().bg(colors::SURFACE))
        .border_style(Style::default().fg(colors::PRIMARY));

    let inner = browser_block.inner(area);
    f.render_widget(browser_block, area);

    if state.data.images.is_empty() {
        let empty_msg =
            Paragraph::new("No images to display").style(Style::default().fg(colors::SUBTEXT));
        f.render_widget(empty_msg, inner);
        return;
    }

    let lines_per_item = 2;
    let visible_items = (inner.height as usize) / lines_per_item;
    let scroll_offset = if state.selected >= visible_items {
        state.selected.saturating_sub(visible_items) + 1
    } else {
        0
    };

    let max_path_len = inner.width as usize - 6;
    let mut y = inner.y;

    for (display_idx, img_idx) in (scroll_offset..state.data.images.len()).enumerate() {
        if display_idx >= visible_items || y + 1 >= inner.y + inner.height {
            break;
        }
        let img = &state.data.images[img_idx];
        let is_selected = img_idx == state.selected;
        let base_style = if is_selected {
            Style::default()
                .fg(colors::ON_SURFACE)
                .bg(colors::SURFACE_HIGH)
        } else {
            Style::default().fg(colors::ON_SURFACE)
        };

        let (line1, line2) = build_image_item_lines(img, is_selected, max_path_len);

        f.render_widget(
            Paragraph::new(line1).style(base_style),
            Rect::new(inner.x, y, inner.width, 1),
        );
        f.render_widget(
            Paragraph::new(line2).style(base_style),
            Rect::new(inner.x, y + 1, inner.width, 1),
        );

        y += lines_per_item as u16;
    }

    let help_area = Rect::new(
        area.x,
        area.y + area.height.saturating_sub(1),
        area.width,
        1,
    );
    render_help_bar(f, help_area);
}
