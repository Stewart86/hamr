//! Hamr TUI - Terminal UI for the Hamr launcher.
//!
//! This is the main entry point for the TUI application. It connects to the
//! hamr-daemon via RPC and provides an interactive terminal interface for
//! searching and executing actions.

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEventKind,
        KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures_util::StreamExt;
use hamr_rpc::{
    ClientRole, CoreEvent, CoreUpdate, FormFieldType, InputMode, Message, ResultType, RpcClient,
    WidgetData,
};
use ratatui::{Frame, Terminal, backend::CrosstermBackend};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use std::io;

mod app;
mod cli;
mod colors;
mod compositor;
mod render;
mod rpc;
mod state;
mod widgets;

use app::App;
use cli::{Cli, Commands};
use render::{
    render_card, render_error, render_form, render_grid_browser, render_image_browser,
    render_results_ui, render_window_picker,
};
use rpc::{notification_to_update, send_event};
use state::{FormState, ViewMode};

/// Helper to send form field change event for live update forms
async fn send_form_field_changed(
    client: &RpcClient,
    form_state: &FormState,
    field_id: String,
    value: String,
) -> Result<()> {
    send_event(
        client,
        CoreEvent::FormFieldChanged {
            field_id,
            value,
            form_data: form_state.get_form_data(),
            context: form_state.context.clone(),
        },
    )
    .await?;
    Ok(())
}

/// Adjusts slider value by step amount. Returns true if slider was adjusted.
async fn adjust_slider(client: &RpcClient, app: &App, increment: bool) -> Result<bool> {
    let Some(result) = app.results.get(app.selected) else {
        return Ok(false);
    };
    let Some(WidgetData::Slider {
        value,
        min,
        max,
        step,
        ..
    }) = &result.widget
    else {
        return Ok(false);
    };

    let new_val = if increment {
        (value + step).min(*max)
    } else {
        (value - step).max(*min)
    };

    send_event(
        client,
        CoreEvent::SliderChanged {
            id: result.id.clone(),
            value: new_val,
            plugin_id: result.plugin_id.clone(),
        },
    )
    .await?;
    Ok(true)
}

/// Toggles switch value. Returns true if switch was toggled.
async fn toggle_switch(client: &RpcClient, app: &mut App) -> Result<bool> {
    let Some(result) = app.results.get(app.selected) else {
        return Ok(false);
    };
    let Some(WidgetData::Switch { value: current }) = &result.widget else {
        return Ok(false);
    };

    let new_value = !current;
    let id = result.id.clone();
    let plugin_id = result.plugin_id.clone();

    // Optimistically update the local switch state
    if let Some(WidgetData::Switch { value }) = &mut app.results[app.selected].widget {
        *value = new_value;
    }

    send_event(
        client,
        CoreEvent::SwitchToggled {
            id,
            value: new_value,
            plugin_id,
        },
    )
    .await?;
    Ok(true)
}

/// Sends item selected event with pending app tracking.
async fn select_item_with_action(
    client: &RpcClient,
    app: &mut App,
    action: Option<String>,
) -> Result<()> {
    let Some(result) = app.results.get(app.selected) else {
        return Ok(());
    };

    app.pending_app_id.clone_from(&result.app_id);
    app.pending_app_name = Some(result.name.clone());

    send_event(
        client,
        CoreEvent::ItemSelected {
            id: result.id.clone(),
            action,
            plugin_id: result.plugin_id.clone(),
        },
    )
    .await?;
    Ok(())
}

/// Navigates back within plugin or closes plugin if at root level.
async fn navigate_back_or_close(client: &RpcClient, app: &mut App) -> Result<()> {
    if app.navigation_depth > 0 {
        app.pending_back = true;
        send_event(client, CoreEvent::Back).await?;
    } else {
        send_event(client, CoreEvent::ClosePlugin).await?;
    }
    Ok(())
}

/// Handles Enter key press in results mode. Returns true if handled.
async fn handle_enter_key(client: &RpcClient, app: &mut App) -> Result<()> {
    // Execute selected preview action with Enter when preview is visible
    if app.show_preview {
        if let Some(action_id) = app.get_selected_preview_action_id() {
            select_item_with_action(client, app, Some(action_id)).await?;
        }
        return Ok(());
    }

    tracing::debug!(
        "Enter pressed: input_mode={:?}, active_plugin={:?}, results.len={}, input={}",
        app.input_mode,
        app.active_plugin,
        app.results.len(),
        app.input
    );

    // Handle pending confirmation first
    if let Some((action_id, _)) = app.pending_confirm.take() {
        send_event(client, CoreEvent::PluginActionTriggered { action_id }).await?;
        return Ok(());
    }

    // Submit mode with either an active plugin or a plugin context (e.g., edit mode)
    if app.input_mode == InputMode::Submit
        && (app.active_plugin.is_some() || app.plugin_context.is_some())
    {
        let query = app.input.clone();
        let context = app.plugin_context.clone();
        tracing::debug!(
            "Sending QuerySubmitted: query={}, context={:?}",
            query,
            context
        );
        send_event(client, CoreEvent::QuerySubmitted { query, context }).await?;
        return Ok(());
    }

    // No results to select
    if app.results.is_empty() {
        return Ok(());
    }

    let result = &app.results[app.selected];
    let id = result.id.clone();
    let action = app.get_selected_action();

    tracing::debug!(
        "Enter on item: selected={}, id={}, result_type={:?}, plugin_id={:?}",
        app.selected,
        id,
        result.result_type,
        result.plugin_id
    );

    if id.starts_with("__pattern_match__:") {
        let plugin_id = id.strip_prefix("__pattern_match__:").unwrap();
        send_event(
            client,
            CoreEvent::OpenPlugin {
                plugin_id: plugin_id.to_string(),
            },
        )
        .await?;
        let query = app.input.clone();
        send_event(client, CoreEvent::QueryChanged { query }).await?;
    } else if result.is_switch() {
        toggle_switch(client, app).await?;
    } else if result.plugin_id.is_some() && app.active_plugin.is_none() {
        if matches!(result.result_type, ResultType::Plugin) {
            send_event(client, CoreEvent::OpenPlugin { plugin_id: id }).await?;
        } else {
            select_item_with_action(client, app, action).await?;
        }
    } else {
        select_item_with_action(client, app, action).await?;
    }
    Ok(())
}

/// Handles key events in Form mode. Returns true if event was handled.
async fn handle_form_mode_key(
    client: &RpcClient,
    app: &mut App,
    key_code: KeyCode,
    shift: bool,
) -> Result<bool> {
    let ViewMode::Form(ref mut form_state) = app.view_mode else {
        return Ok(false);
    };

    match key_code {
        KeyCode::Esc => {
            if form_state.show_cancel_confirm {
                form_state.show_cancel_confirm = false;
            } else if form_state.is_dirty() {
                form_state.show_cancel_confirm = true;
            } else {
                app.view_mode = ViewMode::Results;
                send_event(client, CoreEvent::FormCancelled).await?;
            }
        }
        KeyCode::Enter if form_state.show_cancel_confirm => {
            form_state.show_cancel_confirm = false;
            app.view_mode = ViewMode::Results;
            send_event(client, CoreEvent::FormCancelled).await?;
        }
        KeyCode::Tab if !shift => {
            form_state.focus_next();
        }
        KeyCode::BackTab | KeyCode::Tab if shift => {
            form_state.focus_prev();
        }
        KeyCode::Down => {
            form_state.focus_next();
        }
        KeyCode::Up => {
            form_state.focus_prev();
        }
        KeyCode::Enter => {
            if form_state.is_on_submit() {
                let missing = form_state.get_missing_required();
                let validation_errors = form_state.get_validation_errors();
                if !missing.is_empty() {
                    app.status_message = Some(format!("Required: {}", missing.join(", ")));
                } else if !validation_errors.is_empty() {
                    app.status_message = Some(format!("Invalid: {}", validation_errors.join(", ")));
                } else {
                    let form_data = form_state.get_form_data();
                    let context = form_state.context.clone();
                    app.view_mode = ViewMode::Results;
                    send_event(client, CoreEvent::FormSubmitted { form_data, context }).await?;
                }
            } else if form_state.is_on_cancel() {
                app.view_mode = ViewMode::Results;
                send_event(client, CoreEvent::FormCancelled).await?;
            } else {
                form_state.focus_next();
            }
        }
        KeyCode::Left => {
            handle_form_left_right(client, form_state, false).await?;
        }
        KeyCode::Right => {
            handle_form_left_right(client, form_state, true).await?;
        }
        KeyCode::Char(' ') => {
            handle_form_space_key(client, form_state).await?;
        }
        KeyCode::Backspace => {
            handle_form_backspace(client, form_state).await?;
        }
        KeyCode::Char(c) => {
            handle_form_char_input(client, form_state, c).await?;
        }
        _ => {}
    }
    Ok(true)
}

/// Handles Left/Right arrow in form for Select/Slider fields.
async fn handle_form_left_right(
    client: &RpcClient,
    form_state: &mut FormState,
    forward: bool,
) -> Result<()> {
    let Some(field) = form_state.current_field() else {
        return Ok(());
    };
    let field_id = field.id.clone();
    let field_type = field.field_type.clone();

    match field_type {
        FormFieldType::Select => {
            if forward {
                form_state.cycle_select_next();
            } else {
                form_state.cycle_select_prev();
            }
            if form_state.form.live_update {
                let value = form_state.current_value();
                send_form_field_changed(client, form_state, field_id, value).await?;
            }
        }
        FormFieldType::Slider => {
            form_state.adjust_slider(forward);
            if form_state.form.live_update {
                let value = form_state.current_value();
                send_form_field_changed(client, form_state, field_id, value).await?;
            }
        }
        _ => {
            if forward {
                let len = form_state.current_value().len();
                if form_state.cursor_position < len {
                    form_state.cursor_position += 1;
                }
            } else if form_state.cursor_position > 0 {
                form_state.cursor_position -= 1;
            }
        }
    }
    Ok(())
}

/// Handles space key in form - toggles checkboxes/switches or inserts space.
async fn handle_form_space_key(client: &RpcClient, form_state: &mut FormState) -> Result<()> {
    let Some(field) = form_state.current_field() else {
        return Ok(());
    };
    let field_id = field.id.clone();
    if matches!(
        field.field_type,
        FormFieldType::Checkbox | FormFieldType::Switch
    ) {
        form_state.toggle_bool_field();
    } else {
        form_state.insert_char(' ');
    }
    if form_state.form.live_update {
        let value = form_state.current_value();
        send_form_field_changed(client, form_state, field_id, value).await?;
    }
    Ok(())
}

/// Handles backspace in form - deletes character and sends live update.
async fn handle_form_backspace(client: &RpcClient, form_state: &mut FormState) -> Result<()> {
    let field_id = form_state.current_field().map(|f| f.id.clone());
    form_state.delete_char();
    if let Some(field_id) = field_id
        && form_state.form.live_update
    {
        let value = form_state.current_value();
        send_form_field_changed(client, form_state, field_id, value).await?;
    }
    Ok(())
}

/// Handles character input in form - inserts character and sends live update.
async fn handle_form_char_input(
    client: &RpcClient,
    form_state: &mut FormState,
    c: char,
) -> Result<()> {
    let field_id = form_state.current_field().map(|f| f.id.clone());
    form_state.insert_char(c);
    if let Some(field_id) = field_id
        && form_state.form.live_update
    {
        let value = form_state.current_value();
        send_form_field_changed(client, form_state, field_id, value).await?;
    }
    Ok(())
}

/// Handles key events in Error mode. Returns true if event was handled.
fn handle_error_mode_key(app: &mut App, key_code: KeyCode) -> bool {
    if !matches!(app.view_mode, ViewMode::Error(_)) {
        return false;
    }
    match key_code {
        KeyCode::Esc | KeyCode::Enter => {
            app.view_mode = ViewMode::Results;
        }
        _ => {}
    }
    true
}

/// Handles key events in `WindowPicker` mode. Returns true if event was handled.
fn handle_window_picker_key(app: &mut App, key_code: KeyCode) -> bool {
    let ViewMode::WindowPicker(ref mut picker_state) = app.view_mode else {
        return false;
    };

    match key_code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.view_mode = ViewMode::Results;
            app.pending_app_id = None;
            app.pending_app_name = None;
            app.status_message = Some("Window selection cancelled".to_string());
        }
        KeyCode::Up | KeyCode::Char('k') => {
            picker_state.select_previous();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            picker_state.select_next();
        }
        KeyCode::Enter => {
            if let Some(window) = picker_state.get_selected_window() {
                if app.compositor.focus_window(&window.id) {
                    app.status_message = Some(format!("Focused: {}", window.title));
                    app.should_quit = true;
                } else {
                    app.status_message = Some("Failed to focus window".to_string());
                }
            }
            app.view_mode = ViewMode::Results;
            app.pending_app_id = None;
            app.pending_app_name = None;
        }
        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
            let idx = (c as usize) - ('1' as usize);
            if idx < picker_state.windows.len() {
                let window = &picker_state.windows[idx];
                if app.compositor.focus_window(&window.id) {
                    app.status_message = Some(format!("Focused: {}", window.title));
                    app.should_quit = true;
                } else {
                    app.status_message = Some("Failed to focus window".to_string());
                }
                app.view_mode = ViewMode::Results;
                app.pending_app_id = None;
                app.pending_app_name = None;
            }
        }
        _ => {}
    }
    true
}

/// Handles key events in `GridBrowser` mode. Returns true if event was handled.
async fn handle_grid_browser_key(
    client: &RpcClient,
    app: &mut App,
    key_code: KeyCode,
    ctrl: bool,
) -> Result<bool> {
    let ViewMode::GridBrowser(ref mut state) = app.view_mode else {
        return Ok(false);
    };

    match key_code {
        KeyCode::Esc => {
            app.view_mode = ViewMode::Results;
        }
        KeyCode::Left | KeyCode::Char('h') if !ctrl => state.move_left(),
        KeyCode::Right | KeyCode::Char('l') if !ctrl => state.move_right(),
        KeyCode::Up | KeyCode::Char('k') if !ctrl => state.move_up(),
        KeyCode::Down | KeyCode::Char('j') if !ctrl => state.move_down(),
        KeyCode::Enter => {
            if let Some(item) = state.get_selected_item() {
                let id = item.id.clone();
                send_event(
                    client,
                    CoreEvent::ItemSelected {
                        id,
                        action: None,
                        plugin_id: None,
                    },
                )
                .await?;
            }
        }
        _ => {}
    }
    Ok(true)
}

/// Handles key events in `ImageBrowser` mode. Returns true if event was handled.
async fn handle_image_browser_key(
    client: &RpcClient,
    app: &mut App,
    key_code: KeyCode,
    ctrl: bool,
) -> Result<bool> {
    let ViewMode::ImageBrowser(ref mut state) = app.view_mode else {
        return Ok(false);
    };

    match key_code {
        KeyCode::Esc => {
            app.view_mode = ViewMode::Results;
        }
        KeyCode::Up | KeyCode::Char('k') if !ctrl => {
            if state.selected > 0 {
                state.selected -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') if !ctrl => {
            if state.selected < state.data.images.len().saturating_sub(1) {
                state.selected += 1;
            }
        }
        KeyCode::Enter => {
            if let Some(img) = state.get_selected_image() {
                let id = img.id.clone().unwrap_or_else(|| img.path.clone());
                send_event(
                    client,
                    CoreEvent::ItemSelected {
                        id,
                        action: None,
                        plugin_id: None,
                    },
                )
                .await?;
            }
        }
        _ => {}
    }
    Ok(true)
}

/// Handles key events in Card mode. Returns true if event was handled.
async fn handle_card_mode_key(
    client: &RpcClient,
    app: &mut App,
    key_code: KeyCode,
) -> Result<bool> {
    let ViewMode::Card(ref mut card_state) = app.view_mode else {
        return Ok(false);
    };

    match key_code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.view_mode = ViewMode::Results;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            card_state.scroll_offset = card_state.scroll_offset.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            card_state.scroll_offset = card_state.scroll_offset.saturating_add(1);
        }
        KeyCode::PageUp => {
            card_state.scroll_offset = card_state.scroll_offset.saturating_sub(10);
        }
        KeyCode::PageDown => {
            card_state.scroll_offset = card_state.scroll_offset.saturating_add(10);
        }
        KeyCode::Tab => {
            let max = card_state.card.actions.len() + 1;
            card_state.selected_action = (card_state.selected_action + 1) % max;
        }
        KeyCode::BackTab => {
            let max = card_state.card.actions.len() + 1;
            card_state.selected_action = if card_state.selected_action == 0 {
                max - 1
            } else {
                card_state.selected_action - 1
            };
        }
        KeyCode::Enter => {
            if card_state.selected_action == card_state.card.actions.len() {
                app.view_mode = ViewMode::Results;
            } else if let Some(action) = card_state.card.actions.get(card_state.selected_action) {
                let action_id = action.id.clone();
                let keep_open = action.keep_open;
                send_event(
                    client,
                    CoreEvent::ItemSelected {
                        id: "__card__".to_string(),
                        action: Some(action_id),
                        plugin_id: app.active_plugin.as_ref().map(|(id, _)| id.clone()),
                    },
                )
                .await?;
                if !keep_open {
                    app.view_mode = ViewMode::Results;
                }
            }
        }
        _ => {}
    }
    Ok(true)
}

/// Set up logging with file output. TUI must log to file since it uses the terminal for display.
fn setup_logging(debug_flag: bool) {
    let level = if debug_flag || cfg!(debug_assertions) {
        "debug"
    } else {
        "warn"
    };

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let log_filename = format!("hamr-tui-{timestamp}.log");
    let log_path = std::path::Path::new("/tmp").join(&log_filename);

    let symlink_path = std::path::Path::new("/tmp/hamr-tui.log");
    let _ = std::fs::remove_file(symlink_path);
    let _ = std::os::unix::fs::symlink(&log_path, symlink_path);

    let file_appender = tracing_appender::rolling::never("/tmp", &log_filename);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    std::mem::forget(guard);

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true)
                .with_line_number(true),
        )
        .with(filter)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    setup_logging(cli.debug);

    let mut client = match RpcClient::connect().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to connect to hamr daemon: {e}");
            eprintln!();
            eprintln!("Make sure hamr-daemon is running:");
            eprintln!("  hamr-daemon &");
            return Ok(());
        }
    };

    if let Err(e) = client
        .register(ClientRole::Ui {
            name: "hamr-tui".to_string(),
        })
        .await
    {
        eprintln!("Failed to register with daemon: {e}");
        return Ok(());
    }

    match cli.command {
        Some(Commands::Plugins {
            id: Some(plugin_id),
        }) => run_tui_with_plugin(&mut client, Some(&plugin_id)).await?,
        Some(Commands::Plugins { id: None }) => list_plugins(&mut client).await?,
        Some(Commands::Index) => show_index_stats(&mut client).await?,
        Some(Commands::Query { query }) => search_query(&mut client, &query).await?,
        Some(Commands::Test { plugin, query }) => test_plugin(&mut client, &plugin, &query).await?,
        Some(Commands::Tui) | None => run_tui_with_plugin(&mut client, None).await?,
    }

    Ok(())
}

// Event loop with setup/teardown - core logic is in view-mode handlers
#[allow(clippy::too_many_lines)]
async fn run_tui_with_plugin(client: &mut RpcClient, plugin_id: Option<&str>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    app.refresh_running_apps();

    send_event(client, CoreEvent::LauncherOpened).await?;

    if let Some(pid) = plugin_id {
        send_event(
            client,
            CoreEvent::OpenPlugin {
                plugin_id: pid.to_string(),
            },
        )
        .await?;
    } else {
        send_event(
            client,
            CoreEvent::QueryChanged {
                query: String::new(),
            },
        )
        .await?;
    }

    let mut event_stream = EventStream::new();
    let mut needs_render = true;

    loop {
        if needs_render {
            terminal.draw(|f| ui(f, &mut app))?;
            needs_render = false;
        }

        tokio::select! {
            Some(msg) = client.recv() => {
                let (method, params) = match &msg {
                    Message::Notification(notif) => (notif.method.as_str(), notif.params.clone()),
                    Message::Request(req) if req.id.is_none() => (req.method.as_str(), req.params.clone()),
                    _ => continue,
                };

                tracing::debug!("Received notification: method={}", method);

                if let Some(update) = notification_to_update(method, params.clone()) {
                    tracing::debug!("Parsed update successfully");
                    app.handle_update(update);
                    needs_render = true;
                } else {
                    tracing::warn!("Failed to parse notification: method={}", method);
                }
            }

            Some(event_result) = event_stream.next() => {
                let event = match event_result {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::error!("Event stream error: {}", e);
                        continue;
                    }
                };

                let key = match event {
                    Event::Key(k) if k.kind == KeyEventKind::Press => k,
                    _ => continue,
                };

                needs_render = true;

                let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                let shift = key.modifiers.contains(KeyModifiers::SHIFT);
                let alt = key.modifiers.contains(KeyModifiers::ALT);
            let has_ambient = !app.ambient_items_by_plugin.is_empty();

            tracing::debug!(
                "KEY EVENT: code={:?}, modifiers={:?}, ctrl={}, shift={}, alt={}",
                key.code,
                key.modifiers,
                ctrl,
                shift,
                alt
            );

            // Dispatch to view-mode-specific handlers
            if handle_form_mode_key(client, &mut app, key.code, shift).await? {
                continue;
            }
            if handle_error_mode_key(&mut app, key.code) {
                continue;
            }
            if handle_window_picker_key(&mut app, key.code) {
                continue;
            }
            if handle_grid_browser_key(client, &mut app, key.code, ctrl).await? {
                continue;
            }
            if handle_image_browser_key(client, &mut app, key.code, ctrl).await? {
                continue;
            }
            if handle_card_mode_key(client, &mut app, key.code).await? {
                continue;
            }
            handle_results_mode_key(client, &mut app, key.code, ctrl, shift, alt, has_ambient)
                .await?;
            }
        }

        if app.should_quit {
            let _ = send_event(client, CoreEvent::LauncherClosed).await;
            break;
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Some(text) = app.pending_type_text {
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        let _ = std::process::Command::new("ydotool")
            .args(["type", "--clearmodifiers", "--", &text])
            .spawn();
    }

    Ok(())
}

/// Handle key events in results mode.
// 30+ keyboard bindings - each arm is minimal, splitting would fragment the key mapping
#[allow(clippy::too_many_lines)]
// Keyboard modifier flags (ctrl, shift, alt) are idiomatic as separate bools
#[allow(clippy::fn_params_excessive_bools)]
async fn handle_results_mode_key(
    client: &RpcClient,
    app: &mut App,
    key_code: KeyCode,
    ctrl: bool,
    shift: bool,
    alt: bool,
    has_ambient: bool,
) -> Result<()> {
    match key_code {
        KeyCode::Esc => {
            // Cancel pending confirmation first
            if app.pending_confirm.is_some() {
                app.pending_confirm = None;
            } else {
                // Esc always quits TUI - state can be restored on next launch
                app.should_quit = true;
            }
        }
        KeyCode::Enter => {
            handle_enter_key(client, app).await?;
        }
        KeyCode::Down => {
            app.select_next();
        }
        KeyCode::Char('j') if ctrl && !shift => {
            app.select_next();
        }
        KeyCode::Up => {
            app.select_previous();
        }
        KeyCode::Char('k') if ctrl && !shift => {
            app.select_previous();
        }
        KeyCode::Char('h' | 'H') if ctrl && shift => {
            adjust_slider(client, app, false).await?;
        }
        KeyCode::Char('l' | 'L') if ctrl && shift => {
            adjust_slider(client, app, true).await?;
        }
        KeyCode::Char('t' | 'T') if ctrl && shift => {
            toggle_switch(client, app).await?;
        }
        KeyCode::Char('h') if ctrl && !shift => {
            if app.active_plugin.is_some() {
                navigate_back_or_close(client, app).await?;
            } else {
                app.move_cursor_left();
            }
        }
        KeyCode::Char('l') if ctrl && !shift => {
            if !app.results.is_empty() && app.active_plugin.is_some() {
                let result = &app.results[app.selected];
                let id = result.id.clone();
                let action = app.get_selected_action();
                let plugin_id = result.plugin_id.clone();
                send_event(
                    client,
                    CoreEvent::ItemSelected {
                        id,
                        action,
                        plugin_id,
                    },
                )
                .await?;
            } else {
                app.move_cursor_right();
            }
        }
        KeyCode::Backspace if ctrl => {
            if app.active_plugin.is_some() {
                send_event(client, CoreEvent::ClosePlugin).await?;
            }
        }
        KeyCode::Backspace => {
            if app.input.is_empty() && app.active_plugin.is_some() {
                navigate_back_or_close(client, app).await?;
            } else {
                app.delete_char();
                if app.input_mode == InputMode::Realtime {
                    send_event(
                        client,
                        CoreEvent::QueryChanged {
                            query: app.input.clone(),
                        },
                    )
                    .await?;
                }
            }
        }
        KeyCode::Left => {
            // For sliders, Left arrow adjusts value; otherwise move cursor
            if !adjust_slider(client, app, false).await? {
                app.move_cursor_left();
            }
        }
        KeyCode::Right => {
            // For sliders, Right arrow adjusts value; otherwise move cursor
            if !adjust_slider(client, app, true).await? {
                app.move_cursor_right();
            }
        }
        KeyCode::Tab if ctrl && has_ambient => {
            let total = app.get_all_ambient_items().len();
            if total > 0 {
                app.selected_ambient = (app.selected_ambient + 1) % total;
            }
        }
        KeyCode::BackTab if ctrl && has_ambient => {
            let total = app.get_all_ambient_items().len();
            if total > 0 {
                app.selected_ambient = if app.selected_ambient == 0 {
                    total.saturating_sub(1)
                } else {
                    app.selected_ambient - 1
                };
            }
        }
        KeyCode::Tab if !shift && !ctrl => {
            app.cycle_action();
        }
        KeyCode::BackTab | KeyCode::Tab if shift && !ctrl => {
            app.cycle_action_prev();
        }
        // Alt+U/I/O/P: Item actions (1-4)
        KeyCode::Char(c @ ('u' | 'i' | 'o' | 'p')) if alt && !shift => {
            let action_idx = match c {
                'u' => 0,
                'i' => 1,
                'o' => 2,
                'p' => 3,
                _ => unreachable!(),
            };
            if let Some(result) = app.results.get(app.selected)
                && let Some(action) = result.actions.get(action_idx)
            {
                select_item_with_action(client, app, Some(action.id.clone())).await?;
            }
        }
        // Ctrl+Q/W/E/R/T/Y: Plugin actions toolbar (1-6)
        KeyCode::Char(c @ ('q' | 'w' | 'e' | 'r' | 't' | 'y')) if ctrl && !shift => {
            let action_idx = match c {
                'q' => 0,
                'w' => 1,
                'e' => 2,
                'r' => 3,
                't' => 4,
                'y' => 5,
                _ => unreachable!(),
            };
            if let Some(action) = app.plugin_actions.get(action_idx) {
                // Check if action requires confirmation
                if let Some(ref confirm_msg) = action.confirm {
                    app.pending_confirm = Some((action.id.clone(), confirm_msg.clone()));
                } else {
                    send_event(
                        client,
                        CoreEvent::PluginActionTriggered {
                            action_id: action.id.clone(),
                        },
                    )
                    .await?;
                }
            }
        }
        KeyCode::Char(c @ '1'..='3')
            if has_ambient && app.active_plugin.is_none() && app.input.is_empty() =>
        {
            let action_idx = (c as usize) - ('1' as usize);
            if let Some((plugin_id, item)) = app.get_selected_ambient_with_plugin()
                && let Some(action) = item.actions.get(action_idx)
            {
                send_event(
                    client,
                    CoreEvent::AmbientAction {
                        plugin_id,
                        item_id: item.id.clone(),
                        action: Some(action.id.clone()),
                    },
                )
                .await?;
            }
        }
        KeyCode::Char('x')
            if has_ambient && app.active_plugin.is_none() && app.input.is_empty() =>
        {
            if let Some((plugin_id, item)) = app.get_selected_ambient_with_plugin() {
                send_event(
                    client,
                    CoreEvent::DismissAmbient {
                        plugin_id,
                        item_id: item.id.clone(),
                    },
                )
                .await?;
            }
        }
        KeyCode::Char('p') if !ctrl && app.input.is_empty() => {
            if app.get_selected_preview().is_some() {
                app.show_preview = !app.show_preview;
                app.preview_scroll = 0;
            }
        }
        KeyCode::Char('J') if shift && app.show_preview => {
            app.preview_scroll = app.preview_scroll.saturating_add(1);
        }
        KeyCode::Char('K') if shift && app.show_preview => {
            app.preview_scroll = app.preview_scroll.saturating_sub(1);
        }
        // Preview action navigation with Tab/Shift+Tab when preview is visible
        KeyCode::Tab if app.show_preview && !shift => {
            app.cycle_preview_action();
        }
        KeyCode::BackTab | KeyCode::Tab if shift && app.show_preview => {
            app.cycle_preview_action_prev();
        }
        // Execute preview action with number keys (1-9)
        KeyCode::Char(c @ '1'..='9') if app.show_preview => {
            let action_idx = (c as usize) - ('1' as usize);
            if let Some(preview) = app.get_selected_preview()
                && action_idx < preview.actions.len()
            {
                let action_id = preview.actions[action_idx].id.clone();
                select_item_with_action(client, app, Some(action_id)).await?;
            }
        }
        KeyCode::Char(c) => {
            tracing::debug!(
                "Char '{}' entered (ctrl={}, shift={}, alt={})",
                c,
                ctrl,
                shift,
                alt
            );
            app.enter_char(c);
            if app.input_mode == InputMode::Realtime {
                send_event(
                    client,
                    CoreEvent::QueryChanged {
                        query: app.input.clone(),
                    },
                )
                .await?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn ui(f: &mut Frame, app: &mut App) {
    match &app.view_mode {
        ViewMode::Form(form_state) => {
            render_form(f, form_state);
        }
        ViewMode::Card(card_state) => {
            render_card(f, card_state);
        }
        ViewMode::GridBrowser(grid_state) => {
            render_grid_browser(f, grid_state);
        }
        ViewMode::ImageBrowser(image_state) => {
            render_image_browser(f, image_state);
        }
        ViewMode::WindowPicker(picker_state) => {
            render_window_picker(f, picker_state);
        }
        ViewMode::Error(error_state) => {
            render_error(f, error_state);
        }
        ViewMode::Results => {
            render_results_ui(f, app);
        }
    }
}

async fn list_plugins(client: &mut RpcClient) -> Result<()> {
    let result: serde_json::Value = client.request("list_plugins", None).await?;

    println!("\nAvailable Plugins:\n==================\n");

    if let Some(plugins) = result.get("plugins").and_then(|v| v.as_array()) {
        if plugins.is_empty() {
            println!("No plugins found.");
        } else {
            for plugin in plugins {
                let id = plugin.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                let name = plugin.get("name").and_then(|v| v.as_str()).unwrap_or(id);
                let desc = plugin
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let is_socket = plugin
                    .get("is_socket")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let connected = plugin
                    .get("connected")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);

                let status = if is_socket {
                    if connected { "[connected]" } else { "[socket]" }
                } else {
                    "[stdio]"
                };

                println!("  {id:<16} {name} {status}");
                if !desc.is_empty() {
                    println!("                   {desc}");
                }
            }
        }
    } else {
        println!("No plugins data received.");
    }

    println!();
    Ok(())
}

async fn show_index_stats(client: &mut RpcClient) -> Result<()> {
    use hamr_core::IndexStats;

    let stats: IndexStats = client.request("index_stats", None).await?;

    println!();
    println!("Index Statistics");
    println!("================");
    println!();
    println!("Total Items: {}", stats.item_count);
    println!();

    if !stats.items_per_plugin.is_empty() {
        println!("By Plugin:");
        for (plugin_id, count) in &stats.items_per_plugin {
            let plural = if *count == 1 { "item" } else { "items" };
            println!("  {plugin_id:<16} {count} {plural}");
        }
    }
    println!();

    Ok(())
}

async fn search_query(client: &mut RpcClient, query: &str) -> Result<()> {
    println!("Searching: {query}");

    send_event(client, CoreEvent::LauncherOpened).await?;
    send_event(
        client,
        CoreEvent::QueryChanged {
            query: query.to_string(),
        },
    )
    .await?;

    let timeout = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while let Some(msg) = client.recv().await {
            let (method, params) = match &msg {
                Message::Notification(notif) => (notif.method.as_str(), notif.params.clone()),
                Message::Request(req) if req.id.is_none() => {
                    (req.method.as_str(), req.params.clone())
                }
                _ => continue,
            };

            if let Some(update) = notification_to_update(method, params)
                && let CoreUpdate::Results { results, .. } = update
            {
                println!();
                println!("Results: {} items", results.len());
                for (i, r) in results.iter().take(20).enumerate() {
                    let desc = r.description.as_deref().unwrap_or("");
                    println!(
                        "  {}. {} - {} [{}] {:?}",
                        i + 1,
                        r.name,
                        desc,
                        r.verb_or_default(),
                        r.result_type
                    );
                }
                return;
            }
        }
    });

    if timeout.await.is_err() {
        println!("No results received within timeout");
    }

    Ok(())
}

async fn test_plugin(client: &mut RpcClient, plugin_id: &str, query: &str) -> Result<()> {
    println!("Testing plugin: {plugin_id}");
    println!("Query: {query}");
    println!();

    send_event(
        client,
        CoreEvent::OpenPlugin {
            plugin_id: plugin_id.to_string(),
        },
    )
    .await?;

    let activated = tokio::time::timeout(std::time::Duration::from_millis(500), async {
        while let Some(msg) = client.recv().await {
            let (method, params) = match &msg {
                Message::Notification(notif) => (notif.method.as_str(), notif.params.clone()),
                Message::Request(req) if req.id.is_none() => {
                    (req.method.as_str(), req.params.clone())
                }
                _ => continue,
            };

            if let Some(update) = notification_to_update(method, params) {
                match update {
                    CoreUpdate::PluginActivated { id, name, .. } => {
                        println!("[OK] Plugin activated: {name} ({id})");
                        return true;
                    }
                    CoreUpdate::Error { message } => {
                        println!("[ERROR] {message}");
                        return false;
                    }
                    _ => {}
                }
            }
        }
        false
    })
    .await;

    match activated {
        Ok(true) => {}
        Ok(false) => return Ok(()),
        Err(_) => {
            println!("[TIMEOUT] Plugin activation timed out");
            return Ok(());
        }
    }

    println!("[...] Sending query: {query}");
    send_event(
        client,
        CoreEvent::QueryChanged {
            query: query.to_string(),
        },
    )
    .await?;

    let timeout = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while let Some(msg) = client.recv().await {
            let (method, params) = match &msg {
                Message::Notification(notif) => (notif.method.as_str(), notif.params.clone()),
                Message::Request(req) if req.id.is_none() => {
                    (req.method.as_str(), req.params.clone())
                }
                _ => continue,
            };

            if let Some(update) = notification_to_update(method, params) {
                match update {
                    CoreUpdate::Results { results, .. } => {
                        println!();
                        println!("Results: {} items", results.len());
                        for (i, r) in results.iter().take(10).enumerate() {
                            let desc = r.description.as_deref().unwrap_or("");
                            println!(
                                "  {}. {} - {} [{}]",
                                i + 1,
                                r.name,
                                desc,
                                r.verb_or_default()
                            );
                        }
                        return;
                    }
                    CoreUpdate::Placeholder { placeholder } => {
                        println!("[INFO] Placeholder: {placeholder}");
                    }
                    CoreUpdate::Error { message } => {
                        println!("[ERROR] {message}");
                    }
                    _ => {}
                }
            }
        }
    });

    if timeout.await.is_err() {
        println!("[TIMEOUT] No results received within 3 seconds");
    }

    Ok(())
}
