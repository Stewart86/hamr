//! Main results view rendering.

use crate::app::App;
use crate::colors;
use crate::render::{
    render_ambient_bar, render_preview_panel, render_progress_bar, render_slider_spans,
};
use crate::widgets;
use hamr_rpc::{InputMode, ResultType, SearchResult, WidgetData};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

/// Render the main results UI (search results view).
// UI layout with 10 distinct sections - further splitting would fragment the rendering flow
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
pub fn render_results_ui(f: &mut Frame, app: &mut App) {
    let bg_block = Block::default().style(Style::default().bg(colors::BG));
    f.render_widget(bg_block, f.area());

    let has_ambient = !app.ambient_items_by_plugin.is_empty();
    let has_preview = app.show_preview && app.get_selected_preview().is_some();

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(if has_preview {
            vec![Constraint::Percentage(60), Constraint::Percentage(40)]
        } else {
            vec![Constraint::Percentage(100)]
        })
        .split(f.area());

    let results_area = main_chunks[0];

    let has_plugin_actions = !app.plugin_actions.is_empty();

    let constraints = if has_ambient && has_plugin_actions {
        vec![
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ]
    } else if has_ambient {
        vec![
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ]
    } else if has_plugin_actions {
        vec![
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(3),
        ]
    } else {
        vec![
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(constraints)
        .split(results_area);

    let title = if let Some((_, name)) = &app.active_plugin {
        if app.navigation_depth > 0 {
            let dots: String = (0..app.navigation_depth).map(|_| ".").collect();
            format!(" {name} {dots} ")
        } else {
            format!(" {name} ")
        }
    } else {
        " Hamr ".to_string()
    };

    let mode_indicator = if app.active_plugin.is_some() && app.input_mode == InputMode::Submit {
        " [Submit] "
    } else {
        ""
    };

    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("{title}{mode_indicator}"))
        .style(Style::default().bg(colors::SURFACE))
        .border_style(Style::default().fg(if app.active_plugin.is_some() {
            colors::PRIMARY
        } else {
            colors::OUTLINE
        }));

    let input_text = if app.input.is_empty() {
        Span::styled(&app.placeholder, Style::default().fg(colors::OUTLINE))
    } else {
        Span::styled(&app.input, Style::default().fg(colors::ON_SURFACE))
    };

    let input = Paragraph::new(input_text).block(input_block);
    f.render_widget(input, chunks[0]);

    if !app.input.is_empty() {
        f.set_cursor_position((
            chunks[0].x + app.cursor_position as u16 + 1,
            chunks[0].y + 1,
        ));
    }

    let mut chunk_idx = 1;
    if has_plugin_actions {
        render_plugin_actions_bar(f, app, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    let (results_chunk, help_chunk) = if has_ambient {
        render_ambient_bar(f, app, chunks[chunk_idx]);
        chunk_idx += 1;
        (chunks[chunk_idx], chunks[chunk_idx + 1])
    } else {
        (chunks[chunk_idx], chunks[chunk_idx + 1])
    };

    let width = f.area().width.saturating_sub(4) as usize;
    let items: Vec<ListItem> = app
        .results
        .iter()
        .enumerate()
        .map(|(i, result)| {
            build_result_item(
                result,
                i == app.selected,
                app.selected_action,
                width,
                &app.plugin_statuses,
                &app.running_app_ids,
            )
        })
        .collect();

    let results_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Results ({}) ", app.results.len()))
        .style(Style::default().bg(colors::SURFACE))
        .border_style(Style::default().fg(colors::OUTLINE));

    let results_list = List::new(items).block(results_block);

    f.render_stateful_widget(results_list, results_chunk, &mut app.list_state);

    let help_text = Line::from(build_help_spans(app));

    let help = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().bg(colors::SURFACE))
            .border_style(Style::default().fg(colors::OUTLINE)),
    );
    f.render_widget(help, help_chunk);

    if has_preview {
        let preview_area = Rect::new(
            main_chunks[1].x,
            main_chunks[1].y + 1,
            main_chunks[1].width.saturating_sub(1),
            main_chunks[1].height.saturating_sub(2),
        );
        render_preview_panel(f, app, preview_area);
    }

    if let Some((_, ref message)) = app.pending_confirm {
        render_confirm_dialog(f, message);
    }
}

/// Build a single result list item with name, badges, description, and action hints.
fn build_result_item<'a>(
    result: &'a SearchResult,
    is_selected: bool,
    selected_action: usize,
    width: usize,
    plugin_statuses: &'a std::collections::HashMap<String, hamr_rpc::PluginStatus>,
    running_apps: &'a std::collections::HashSet<String>,
) -> ListItem<'a> {
    let is_pattern_match = result.result_type == ResultType::PatternMatch;

    let base_style = if is_pattern_match {
        if is_selected {
            Style::default()
                .bg(colors::SURFACE_HIGH)
                .fg(colors::ON_SURFACE)
        } else {
            Style::default().bg(colors::SURFACE).fg(colors::ON_SURFACE)
        }
    } else if is_selected {
        Style::default().bg(colors::SURFACE_HIGH)
    } else {
        Style::default().bg(colors::SURFACE)
    };

    let line1_spans = build_result_line1(
        result,
        is_selected,
        is_pattern_match,
        selected_action,
        width,
        plugin_statuses,
        running_apps,
    );
    let line2_spans = build_result_line2(result, is_selected, is_pattern_match, width);
    let line3_spans = vec![Span::raw("")];

    ListItem::new(vec![
        Line::from(line1_spans),
        Line::from(line2_spans),
        Line::from(line3_spans),
    ])
    .style(base_style)
}

/// Build the first line of a result item (name, badges, chips, action hints).
#[allow(clippy::too_many_arguments)]
fn build_result_line1<'a>(
    result: &'a SearchResult,
    is_selected: bool,
    is_pattern_match: bool,
    selected_action: usize,
    width: usize,
    plugin_statuses: &'a std::collections::HashMap<String, hamr_rpc::PluginStatus>,
    running_apps: &'a std::collections::HashSet<String>,
) -> Vec<Span<'a>> {
    let mut line1_left: Vec<Span> = Vec::new();
    let mut line1_right: Vec<Span> = Vec::new();

    if is_pattern_match {
        let indicator_style = if is_selected {
            Style::default()
                .fg(colors::ON_SURFACE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(colors::PRIMARY)
                .add_modifier(Modifier::BOLD)
        };
        line1_left.push(Span::styled("= ", indicator_style));
    }

    if let Some(ref app_id) = result.app_id
        && running_apps.contains(app_id)
    {
        line1_left.push(Span::styled("|", Style::default().fg(colors::SUCCESS)));
        line1_left.push(Span::raw(" "));
    }

    if result.is_suggestion {
        line1_left.push(Span::styled(
            "✦ ",
            Style::default()
                .fg(colors::PRIMARY)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let name_style = if is_pattern_match || is_selected {
        Style::default()
            .fg(colors::ON_SURFACE)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors::ON_SURFACE)
    };
    line1_left.push(Span::styled(&result.name, name_style));

    let mut all_badges = result.badges.clone();
    let mut all_chips = result.chips.clone();

    if matches!(result.result_type, ResultType::Plugin | ResultType::Recent)
        && let Some(status) = plugin_statuses.get(&result.id)
    {
        all_badges.extend(status.badges.clone());
        all_chips.extend(status.chips.clone());
    }

    for badge in all_badges.iter().take(3) {
        let badge_span = widgets::render_badge(badge);
        if !badge_span.content.is_empty() {
            line1_left.push(Span::raw(" "));
            line1_left.push(badge_span);
        }
    }

    for chip in all_chips.iter().take(2) {
        line1_left.push(Span::raw(" "));
        line1_left.push(widgets::render_chip(chip));
    }

    if result.has_ocr {
        line1_left.push(Span::raw(" "));
        let ocr_chip = hamr_rpc::Chip {
            text: "OCR".to_string(),
            icon: Some("text_fields".to_string()),
            ..Default::default()
        };
        line1_left.push(widgets::render_chip(&ocr_chip));
    }

    if let Some(WidgetData::Graph { data, min, max }) = &result.widget {
        let sparkline = widgets::Sparkline::from_widget(data, *min, *max).to_span(12);
        if !sparkline.content.is_empty() {
            line1_left.push(Span::raw(" "));
            line1_left.push(sparkline);
        }
    }

    build_result_line1_right(result, is_selected, selected_action, &mut line1_right);

    let left_len: usize = line1_left.iter().map(|s| s.content.len()).sum();
    let right_len: usize = line1_right.iter().map(|s| s.content.len()).sum();
    let padding = width.saturating_sub(left_len + right_len + 1);

    let mut line1_spans = line1_left;
    line1_spans.push(Span::raw(" ".repeat(padding)));
    line1_spans.extend(line1_right);
    line1_spans
}

/// Build the right side of line 1 (slider, switch, or action hints).
fn build_result_line1_right<'a>(
    result: &'a SearchResult,
    is_selected: bool,
    selected_action: usize,
    line1_right: &mut Vec<Span<'a>>,
) {
    if let Some(WidgetData::Slider {
        value,
        min,
        max,
        step,
        ..
    }) = &result.widget
    {
        let slider = hamr_rpc::SliderValue {
            value: *value,
            min: *min,
            max: *max,
            step: *step,
            display_value: None,
        };
        line1_right.extend(render_slider_spans(&slider, 25, is_selected));
    } else if let Some(WidgetData::Switch { value }) = &result.widget {
        if *value {
            line1_right.push(Span::styled(
                "[ON] ",
                Style::default()
                    .fg(colors::SUCCESS)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            line1_right.push(Span::styled("[OFF]", Style::default().fg(colors::OUTLINE)));
        }
    } else {
        let verb = if is_selected && selected_action > 0 {
            result
                .actions
                .get(selected_action - 1)
                .map_or_else(|| result.verb_or_default(), |a| a.name.as_str())
        } else {
            result.verb_or_default()
        };

        if is_selected {
            line1_right.push(Span::styled(
                format!("[{verb}]"),
                Style::default().fg(colors::PRIMARY),
            ));
            for (idx, action) in result.actions.iter().take(4).enumerate() {
                let shortcut = match idx {
                    0 => "A-u",
                    1 => "A-i",
                    2 => "A-o",
                    3 => "A-p",
                    _ => unreachable!(),
                };
                let action_style = if selected_action == idx + 1 {
                    Style::default()
                        .fg(colors::PRIMARY)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(colors::OUTLINE)
                };
                line1_right.push(Span::styled(
                    format!(" {}:{}", shortcut, action.name),
                    action_style,
                ));
            }
        } else {
            line1_right.push(Span::styled(
                verb.to_string(),
                Style::default().fg(colors::OUTLINE),
            ));
        }
    }
}

/// Build the second line of a result item (description, gauge, progress, or suggestion).
fn build_result_line2(
    result: &SearchResult,
    is_selected: bool,
    is_pattern_match: bool,
    width: usize,
) -> Vec<Span<'_>> {
    if let (true, Some(desc)) = (is_pattern_match, result.description.as_ref()) {
        let truncated = if desc.len() > width.saturating_sub(2) {
            format!("{}...", &desc[..width.saturating_sub(5).min(desc.len())])
        } else {
            desc.clone()
        };
        return vec![Span::styled(
            format!("Query: {truncated}"),
            if is_selected {
                Style::default()
                    .fg(colors::ON_SURFACE)
                    .add_modifier(Modifier::DIM)
            } else {
                Style::default()
                    .fg(colors::SUBTEXT)
                    .add_modifier(Modifier::DIM)
            },
        )];
    }

    if let Some(WidgetData::Gauge {
        value,
        min,
        max,
        label,
        color,
    }) = &result.widget
    {
        return widgets::Gauge::from_widget(*value, *min, *max, label.as_deref(), color.as_deref())
            .to_spans(15);
    }

    if let Some(WidgetData::Progress {
        value, max, label, ..
    }) = &result.widget
    {
        let bar = render_progress_bar(*value, *max, 20);
        let label_str = label.as_ref().map_or_else(
            || format!(" {:.0}%", (value / max * 100.0)),
            |l| format!(" {l}"),
        );
        return vec![
            Span::styled(bar, Style::default().fg(colors::PRIMARY)),
            Span::styled(label_str, Style::default().fg(colors::SUBTEXT)),
        ];
    }

    if let Some(desc) = &result.description {
        if !desc.is_empty() {
            let truncated = if desc.len() > width.saturating_sub(2) {
                format!("{}...", &desc[..width.saturating_sub(5).min(desc.len())])
            } else {
                desc.clone()
            };
            let mut spans = vec![Span::styled(
                truncated,
                Style::default().fg(colors::SUBTEXT),
            )];
            if let Some(reason) = &result.suggestion_reason
                && result.is_suggestion
            {
                spans.push(Span::raw(" • "));
                spans.push(Span::styled(
                    format!("Suggested: {reason}"),
                    Style::default()
                        .fg(colors::SUBTEXT)
                        .add_modifier(Modifier::DIM),
                ));
            }
            return spans;
        } else if let Some(reason) = &result.suggestion_reason {
            return vec![Span::styled(
                format!("Suggested: {reason}"),
                Style::default()
                    .fg(colors::SUBTEXT)
                    .add_modifier(Modifier::DIM),
            )];
        }
    }

    if let Some(reason) = &result.suggestion_reason {
        return vec![Span::styled(
            format!("Suggested: {reason}"),
            Style::default()
                .fg(colors::SUBTEXT)
                .add_modifier(Modifier::DIM),
        )];
    }

    vec![Span::styled(" ", Style::default())]
}

/// Build the help bar spans based on current selection and app state.
fn build_help_spans(app: &App) -> Vec<Span<'_>> {
    let mut help_spans = vec![
        Span::styled("Esc", Style::default().fg(colors::PRIMARY)),
        Span::styled(": quit  ", Style::default().fg(colors::SUBTEXT)),
        Span::styled("Enter", Style::default().fg(colors::PRIMARY)),
        Span::styled(": select  ", Style::default().fg(colors::SUBTEXT)),
        Span::styled("Tab", Style::default().fg(colors::PRIMARY)),
        Span::styled(": action  ", Style::default().fg(colors::SUBTEXT)),
    ];

    if let Some(result) = app.results.get(app.selected) {
        if result.is_slider() {
            help_spans.extend(vec![
                Span::styled("<-/->", Style::default().fg(colors::PRIMARY)),
                Span::styled(": adjust  ", Style::default().fg(colors::SUBTEXT)),
            ]);
        }

        if !result.actions.is_empty() {
            help_spans.extend(vec![
                Span::styled("A-uiop", Style::default().fg(colors::PRIMARY)),
                Span::styled(": actions  ", Style::default().fg(colors::SUBTEXT)),
            ]);
        }

        if result.preview.is_some() {
            help_spans.extend(vec![
                Span::styled("p", Style::default().fg(colors::PRIMARY)),
                Span::styled(": preview  ", Style::default().fg(colors::SUBTEXT)),
            ]);
        }
    }

    if !app.ambient_items_by_plugin.is_empty() {
        help_spans.extend(vec![
            Span::styled("C-Tab", Style::default().fg(colors::PRIMARY)),
            Span::styled(": ambient  ", Style::default().fg(colors::SUBTEXT)),
            Span::styled("x", Style::default().fg(colors::PRIMARY)),
            Span::styled(": dismiss  ", Style::default().fg(colors::SUBTEXT)),
        ]);
    }

    if app.busy {
        help_spans.push(Span::styled(
            " [Loading...] ",
            Style::default().fg(colors::SUCCESS),
        ));
    }

    if let Some(msg) = &app.status_message {
        help_spans.push(Span::styled(msg, Style::default().fg(colors::ERROR)));
    }

    help_spans
}

/// Render the plugin actions toolbar bar.
fn render_plugin_actions_bar(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = Vec::new();

    for (idx, action) in app.plugin_actions.iter().take(6).enumerate() {
        let shortcut = match idx {
            0 => "C-q",
            1 => "C-w",
            2 => "C-e",
            3 => "C-r",
            4 => "C-t",
            5 => "C-y",
            _ => unreachable!(),
        };
        let style = if action.active {
            Style::default()
                .fg(colors::PRIMARY)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors::SUBTEXT)
        };

        spans.push(Span::styled(
            format!(" [{shortcut}:"),
            Style::default().fg(colors::OUTLINE),
        ));
        spans.push(Span::styled(&action.name, style));
        spans.push(Span::styled("]", Style::default().fg(colors::OUTLINE)));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(colors::SURFACE));
    f.render_widget(paragraph, area);
}

/// Render a confirmation dialog overlay.
fn render_confirm_dialog(f: &mut Frame, message: &str) {
    use ratatui::widgets::Clear;

    let area = f.area();
    let dialog_width = 50.min(area.width.saturating_sub(4));
    let dialog_height = 5;
    let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
    let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;

    let dialog_area = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);

    // Clear the area behind the dialog
    f.render_widget(Clear, dialog_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Confirm ")
        .style(Style::default().bg(colors::SURFACE_HIGH))
        .border_style(Style::default().fg(colors::WARNING));

    let inner = block.inner(dialog_area);
    f.render_widget(block, dialog_area);

    let text = vec![
        Line::from(Span::styled(
            message,
            Style::default().fg(colors::ON_SURFACE),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(colors::PRIMARY)),
            Span::styled(": confirm  ", Style::default().fg(colors::SUBTEXT)),
            Span::styled("Esc", Style::default().fg(colors::ERROR)),
            Span::styled(": cancel", Style::default().fg(colors::SUBTEXT)),
        ]),
    ];

    let paragraph = Paragraph::new(text);
    f.render_widget(paragraph, inner);
}
