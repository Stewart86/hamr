//! Form rendering.

use crate::colors;
use crate::state::FormState;
use hamr_rpc::{FormData, FormField, FormFieldType};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

/// Render form modal for user input.
// TUI layout math uses usize for sums, u16 for terminal dimensions
// 12 field type variants - each arm delegates to helper, splitting would fragment the exhaustive match
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]
pub fn render_form(f: &mut Frame, form_state: &FormState) {
    let bg_block = Block::default().style(Style::default().bg(colors::BG));
    f.render_widget(bg_block, f.area());

    let form = &form_state.form;

    // Calculate form area - centered dialog
    let area = f.area();
    let form_width = 60.min(area.width.saturating_sub(4));

    let field_heights: usize = form
        .fields
        .iter()
        .map(|f| match f.field_type {
            FormFieldType::TextArea => 5,
            _ => 3,
        })
        .sum();
    let form_height = (field_heights + 4 + 2).min(area.height.saturating_sub(4) as usize) as u16;

    let x = (area.width.saturating_sub(form_width)) / 2;
    let y = (area.height.saturating_sub(form_height)) / 2;
    let form_area = Rect::new(x, y, form_width, form_height);

    // Main form block
    let form_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", form.title))
        .style(Style::default().bg(colors::SURFACE))
        .border_style(Style::default().fg(colors::PRIMARY));

    f.render_widget(Clear, form_area);
    f.render_widget(form_block, form_area);

    let inner = Rect::new(
        form_area.x + 2,
        form_area.y + 1,
        form_area.width.saturating_sub(4),
        form_area.height.saturating_sub(2),
    );

    let mut constraints: Vec<Constraint> = form
        .fields
        .iter()
        .map(|field| match field.field_type {
            FormFieldType::TextArea => Constraint::Length(5),
            _ => Constraint::Length(3),
        })
        .collect();
    constraints.push(Constraint::Length(2));
    constraints.push(Constraint::Min(0));

    let field_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (i, field) in form.fields.iter().enumerate() {
        let is_focused = form_state.focused_field == i;
        let value = form_state
            .field_values
            .get(&field.id)
            .cloned()
            .unwrap_or_default();

        let field_style = if is_focused {
            Style::default()
                .fg(colors::ON_SURFACE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors::SUBTEXT)
        };

        let is_empty_required = field.required && value.trim().is_empty();
        let border_color = if is_focused {
            colors::PRIMARY
        } else if is_empty_required {
            colors::ERROR
        } else {
            colors::OUTLINE
        };

        match field.field_type {
            FormFieldType::Text
            | FormFieldType::Password
            | FormFieldType::Number
            | FormFieldType::TextArea => {
                render_text_input_field(
                    f,
                    field,
                    &value,
                    is_focused,
                    field_style,
                    border_color,
                    field_chunks[i],
                    form_state.cursor_position,
                );
            }
            FormFieldType::Select => {
                let options: Vec<&str> = field.options.iter().map(|o| o.label.as_str()).collect();
                let current_label = field
                    .options
                    .iter()
                    .find(|o| o.value == value)
                    .map_or(options.first().copied().unwrap_or(""), |o| o.label.as_str());

                let select_block = Block::default()
                    .borders(Borders::ALL)
                    .title(format!("{} [<-/->]", field.label))
                    .border_style(Style::default().fg(border_color));

                let text = Paragraph::new(format!("< {current_label} >"))
                    .style(field_style)
                    .block(select_block);
                f.render_widget(text, field_chunks[i]);
            }
            FormFieldType::Checkbox => {
                let is_checked = value == "true" || value == "1";
                let checkbox_char = if is_checked { "[x]" } else { "[ ]" };

                let checkbox_block = Block::default()
                    .borders(Borders::ALL)
                    .title(format!("{} [Space]", field.label))
                    .border_style(Style::default().fg(border_color));

                let text = Paragraph::new(checkbox_char)
                    .style(field_style)
                    .block(checkbox_block);
                f.render_widget(text, field_chunks[i]);
            }
            FormFieldType::Switch => {
                render_switch_field(f, field, &value, border_color, field_chunks[i]);
            }
            FormFieldType::Slider => {
                let bar_width = (inner.width as usize).saturating_sub(20);
                render_slider_field(
                    f,
                    field,
                    &value,
                    is_focused,
                    border_color,
                    field_chunks[i],
                    bar_width,
                );
            }
            FormFieldType::Hidden => {}
            FormFieldType::Date | FormFieldType::Time => {
                render_datetime_field(
                    f,
                    field,
                    &value,
                    is_focused,
                    field_style,
                    border_color,
                    field_chunks[i],
                    form_state.cursor_position,
                );
            }
            FormFieldType::Email | FormFieldType::Url | FormFieldType::Phone => {
                render_validated_text_field(
                    f,
                    field,
                    &value,
                    is_focused,
                    field_style,
                    border_color,
                    field_chunks[i],
                    form_state.cursor_position,
                );
            }
        }
    }

    let button_idx = form.fields.len();
    let button_area = field_chunks[button_idx];
    render_form_buttons(f, form, form_state, button_area, form_area);

    if form_state.show_cancel_confirm {
        render_cancel_confirm_dialog(f);
    }
}

/// Render validated text input (Email, URL, Phone) with validation status.
#[allow(clippy::too_many_arguments)]
fn render_validated_text_field(
    f: &mut Frame,
    field: &FormField,
    value: &str,
    is_focused: bool,
    field_style: Style,
    border_color: ratatui::style::Color,
    area: Rect,
    cursor_position: usize,
) {
    let (type_hint, placeholder_hint) = match field.field_type {
        FormFieldType::Email => ("email", "user@example.com"),
        FormFieldType::Url => ("url", "https://example.com"),
        FormFieldType::Phone => ("phone", "+1 (555) 123-4567"),
        _ => return,
    };

    let display = if value.is_empty() {
        field
            .placeholder
            .clone()
            .unwrap_or(placeholder_hint.to_string())
    } else {
        value.to_string()
    };

    let validation_error = if value.is_empty() {
        None
    } else {
        match field.field_type {
            FormFieldType::Email => {
                if crate::state::is_valid_email(value) {
                    None
                } else {
                    Some("invalid")
                }
            }
            FormFieldType::Url => {
                if crate::state::is_valid_url(value) {
                    None
                } else {
                    Some("invalid")
                }
            }
            FormFieldType::Phone => {
                if crate::state::is_valid_phone(value) {
                    None
                } else {
                    Some("invalid")
                }
            }
            _ => None,
        }
    };

    let text_style = if value.is_empty() {
        Style::default().fg(colors::OUTLINE)
    } else if validation_error.is_some() {
        Style::default().fg(colors::ERROR)
    } else {
        field_style
    };

    let actual_border_color = if validation_error.is_some() && !is_focused {
        colors::ERROR
    } else {
        border_color
    };

    let label_suffix = if field.required { " *" } else { "" };
    let title = if validation_error.is_some() {
        format!("{}{} ({}) - invalid", field.label, label_suffix, type_hint)
    } else {
        format!("{}{} ({})", field.label, label_suffix, type_hint)
    };

    let field_block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(actual_border_color));

    let field_widget = Paragraph::new(display).style(text_style).block(field_block);
    f.render_widget(field_widget, area);

    if is_focused {
        set_field_cursor(f, area, cursor_position);
    }
}

/// Render form buttons (submit/cancel) and help text.
fn render_form_buttons(
    f: &mut Frame,
    form: &FormData,
    form_state: &FormState,
    button_area: Rect,
    form_area: Rect,
) {
    let submit_style = if form_state.is_on_submit() {
        Style::default()
            .fg(colors::BG)
            .bg(colors::PRIMARY)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors::PRIMARY)
    };

    let cancel_style = if form_state.is_on_cancel() {
        Style::default()
            .fg(colors::BG)
            .bg(colors::ERROR)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors::SUBTEXT)
    };

    let submit_text = format!(" [{}] ", form.submit_label);
    let cancel_text = format!(" [{}] ", form.cancel_label.as_deref().unwrap_or("Cancel"));

    let buttons = Line::from(vec![
        Span::raw("  "),
        Span::styled(submit_text, submit_style),
        Span::raw("  "),
        Span::styled(cancel_text, cancel_style),
    ]);

    let button_para = Paragraph::new(buttons);
    f.render_widget(button_para, button_area);

    let help_area = Rect::new(
        form_area.x,
        form_area.y + form_area.height,
        form_area.width,
        1,
    );

    let help_text = Line::from(vec![
        Span::styled("Tab", Style::default().fg(colors::PRIMARY)),
        Span::styled(": next  ", Style::default().fg(colors::SUBTEXT)),
        Span::styled("S-Tab", Style::default().fg(colors::PRIMARY)),
        Span::styled(": prev  ", Style::default().fg(colors::SUBTEXT)),
        Span::styled("Enter", Style::default().fg(colors::PRIMARY)),
        Span::styled(": submit  ", Style::default().fg(colors::SUBTEXT)),
        Span::styled("Esc", Style::default().fg(colors::PRIMARY)),
        Span::styled(": cancel", Style::default().fg(colors::SUBTEXT)),
    ]);

    if help_area.y < f.area().height {
        f.render_widget(Paragraph::new(help_text), help_area);
    }
}

/// Render basic text input field (`Text`, `Password`, `Number`, `TextArea`).
// Field rendering requires frame, field, value, area + styling context (focus, style, color, cursor)
#[allow(clippy::too_many_arguments)]
fn render_text_input_field(
    f: &mut Frame,
    field: &FormField,
    value: &str,
    is_focused: bool,
    field_style: Style,
    border_color: ratatui::style::Color,
    area: Rect,
    cursor_position: usize,
) {
    let display_value = if matches!(field.field_type, FormFieldType::Password) {
        "*".repeat(value.len())
    } else if value.is_empty() {
        field.placeholder.clone().unwrap_or_default()
    } else {
        value.to_string()
    };

    let text_style = if value.is_empty() && !matches!(field.field_type, FormFieldType::Password) {
        Style::default().fg(colors::OUTLINE)
    } else {
        field_style
    };

    let label_suffix = if field.required { " *" } else { "" };
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("{}{}", field.label, label_suffix))
        .border_style(Style::default().fg(border_color));

    let text = Paragraph::new(display_value)
        .style(text_style)
        .block(input_block);
    f.render_widget(text, area);

    // Only set cursor for single-line text inputs, not TextArea
    if is_focused && !matches!(field.field_type, FormFieldType::TextArea) {
        set_field_cursor(f, area, cursor_position);
    }
}

/// Render date/time text input field with placeholder.
// Field rendering requires frame, field, value, area + styling context (focus, style, color, cursor)
#[allow(clippy::too_many_arguments)]
fn render_datetime_field(
    f: &mut Frame,
    field: &FormField,
    value: &str,
    is_focused: bool,
    field_style: Style,
    border_color: ratatui::style::Color,
    area: Rect,
    cursor_position: usize,
) {
    let (type_hint, default_placeholder) = match field.field_type {
        FormFieldType::Date => ("date", "YYYY-MM-DD"),
        FormFieldType::Time => ("time", "HH:MM"),
        _ => return,
    };

    let display = if value.is_empty() {
        field
            .placeholder
            .clone()
            .unwrap_or_else(|| default_placeholder.to_string())
    } else {
        value.to_string()
    };

    let text_style = if value.is_empty() {
        Style::default().fg(colors::OUTLINE)
    } else {
        field_style
    };

    let label_suffix = if field.required { " *" } else { "" };
    let field_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("{}{} ({})", field.label, label_suffix, type_hint))
        .border_style(Style::default().fg(border_color));

    let field_widget = Paragraph::new(display).style(text_style).block(field_block);
    f.render_widget(field_widget, area);

    if is_focused {
        set_field_cursor(f, area, cursor_position);
    }
}

/// Render switch field with toggle visualization.
fn render_switch_field(
    f: &mut Frame,
    field: &FormField,
    value: &str,
    border_color: ratatui::style::Color,
    area: Rect,
) {
    let is_on = value == "true" || value == "1" || value == "on";

    let (left_style, right_style, track_char) = if is_on {
        (
            Style::default().fg(colors::OUTLINE),
            Style::default()
                .fg(colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
            '=',
        )
    } else {
        (
            Style::default()
                .fg(colors::ERROR)
                .add_modifier(Modifier::BOLD),
            Style::default().fg(colors::OUTLINE),
            '-',
        )
    };

    let switch_visual = Line::from(vec![
        Span::styled(if is_on { " OFF " } else { "[OFF]" }, left_style),
        Span::styled(
            format!("{track_char}{track_char}{track_char}"),
            Style::default().fg(colors::OUTLINE),
        ),
        Span::styled(if is_on { "[ON] " } else { " ON  " }, right_style),
    ]);

    let label_suffix = if field.required { " *" } else { "" };
    let switch_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("{}{} [Space/Enter]", field.label, label_suffix))
        .border_style(Style::default().fg(border_color));

    let switch_widget = Paragraph::new(switch_visual).block(switch_block);
    f.render_widget(switch_widget, area);
}

/// Render slider field with progress bar visualization.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn render_slider_field(
    f: &mut Frame,
    field: &FormField,
    value: &str,
    is_focused: bool,
    border_color: ratatui::style::Color,
    area: Rect,
    bar_width: usize,
) {
    let current_val: f64 = value.parse().unwrap_or(0.0);
    let min = field.min.unwrap_or(0.0);
    let max = field.max.unwrap_or(100.0);

    let pct = if max > min {
        ((current_val - min) / (max - min)).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let filled = (pct * bar_width as f64).round() as usize;
    let empty = bar_width.saturating_sub(filled);

    let arrow_color = if is_focused {
        colors::PRIMARY
    } else {
        colors::OUTLINE
    };

    let slider_line = Line::from(vec![
        Span::styled("<", Style::default().fg(arrow_color)),
        Span::styled("#".repeat(filled), Style::default().fg(colors::SUCCESS)),
        Span::styled(".".repeat(empty), Style::default().fg(colors::OUTLINE)),
        Span::styled(">", Style::default().fg(arrow_color)),
        Span::styled(
            format!(" {current_val:.1}"),
            Style::default()
                .fg(colors::ON_SURFACE)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let label_suffix = if field.required { " *" } else { "" };
    let slider_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("{}{} [<-/->]", field.label, label_suffix))
        .border_style(Style::default().fg(border_color));

    let slider_widget = Paragraph::new(slider_line).block(slider_block);
    f.render_widget(slider_widget, area);
}

/// Position cursor for text input field if focused.
fn set_field_cursor(f: &mut Frame, area: Rect, cursor_position: usize) {
    // Terminal cursor positions are bounded by terminal dimensions (u16)
    #[allow(clippy::cast_possible_truncation)]
    let cursor_x = area.x + 1 + cursor_position as u16;
    let cursor_y = area.y + 1;
    if cursor_x < area.x + area.width - 1 {
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

/// Render the cancel confirmation dialog overlay.
fn render_cancel_confirm_dialog(f: &mut Frame) {
    let area = f.area();
    let dialog_width = 45.min(area.width.saturating_sub(4));
    let dialog_height = 5;
    let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
    let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;

    let dialog_area = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);

    f.render_widget(Clear, dialog_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Discard Changes? ")
        .style(Style::default().bg(colors::SURFACE_HIGH))
        .border_style(Style::default().fg(colors::WARNING));

    let inner = block.inner(dialog_area);
    f.render_widget(block, dialog_area);

    let text = vec![
        Line::from(Span::styled(
            "You have unsaved changes.",
            Style::default().fg(colors::ON_SURFACE),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(colors::ERROR)),
            Span::styled(": discard  ", Style::default().fg(colors::SUBTEXT)),
            Span::styled("Esc", Style::default().fg(colors::PRIMARY)),
            Span::styled(": keep editing", Style::default().fg(colors::SUBTEXT)),
        ]),
    ];

    let paragraph = Paragraph::new(text);
    f.render_widget(paragraph, inner);
}
