//! Conversion from `PluginResponse` to `CoreUpdate`.
//!
//! This module provides shared conversion logic used by both:
//! - The engine (for stdio plugins)
//! - The daemon (for socket plugins)

use hamr_types::{
    Action, AmbientItem, CardBlock, CardData, CoreUpdate, DisplayHint, ExecuteAction, FabOverride,
    FormData, FormField, FormFieldType, GridBrowserData, ImageBrowserData, PluginAction,
    PluginStatus, ResultItem, ResultPatch, SearchResult, WidgetData,
};

use super::protocol::{AmbientItemData, CardBlockData, PluginResponse, StatusData, UpdateItem};

/// Convert a `PluginResponse` to a list of `CoreUpdates`.
///
/// A single `PluginResponse` can produce multiple `CoreUpdates` because:
/// - Execute responses can contain multiple actions (launch, copy, etc.)
/// - Responses can include inline status updates
#[must_use]
pub fn plugin_response_to_updates(plugin_id: &str, response: PluginResponse) -> Vec<CoreUpdate> {
    let mut updates = vec![CoreUpdate::Busy { busy: false }];

    match response {
        PluginResponse::Results {
            items,
            prepend: _,
            input_mode,
            status,
            context,
            placeholder,
            clear_input,
            navigate_forward,
            plugin_actions,
            navigation_depth,
            display_hint,
            activate,
        } => {
            handle_results_response(
                plugin_id,
                &mut updates,
                items,
                input_mode,
                status,
                context,
                placeholder,
                clear_input,
                navigate_forward,
                plugin_actions,
                navigation_depth,
                display_hint,
                activate,
            );
        }

        PluginResponse::Execute(data) => {
            handle_execute_response(&mut updates, data);
        }

        PluginResponse::Card {
            card,
            status,
            context,
        } => {
            handle_card_response(plugin_id, &mut updates, card, status, context);
        }

        PluginResponse::Form {
            form,
            context,
            navigate_forward,
        } => {
            handle_form_response(&mut updates, form, context, navigate_forward);
        }

        PluginResponse::Error {
            message,
            details: _,
        } => {
            updates.push(CoreUpdate::Error { message });
        }

        PluginResponse::Prompt { prompt } => {
            updates.push(CoreUpdate::Placeholder {
                placeholder: prompt.text,
            });
        }

        PluginResponse::Match { result } => {
            handle_match_response(plugin_id, &mut updates, result);
        }

        PluginResponse::Index { status, .. } => {
            if let Some(status_data) = status {
                updates.extend(process_status_data(plugin_id, status_data));
            }
        }

        PluginResponse::Status { status } => {
            updates.extend(process_status_data(plugin_id, status));
        }

        PluginResponse::Update { items, status } => {
            handle_update_response(plugin_id, &mut updates, items, status);
        }

        PluginResponse::Noop => {}

        PluginResponse::ImageBrowser {
            images,
            title,
            directory,
            image_browser,
        } => {
            handle_image_browser_response(&mut updates, images, title, directory, image_browser);
        }

        PluginResponse::GridBrowser {
            items,
            title,
            columns,
            actions,
        } => {
            updates.push(CoreUpdate::GridBrowser {
                browser: GridBrowserData {
                    items,
                    title,
                    columns,
                    actions,
                },
            });
        }
    }

    updates
}

/// Handle `PluginResponse::Results` variant
#[allow(clippy::too_many_arguments)]
fn handle_results_response(
    plugin_id: &str,
    updates: &mut Vec<CoreUpdate>,
    items: Vec<ResultItem>,
    input_mode: Option<String>,
    status: Option<StatusData>,
    context: Option<String>,
    placeholder: Option<String>,
    clear_input: bool,
    navigate_forward: Option<bool>,
    plugin_actions: Vec<PluginAction>,
    navigation_depth: Option<u32>,
    display_hint: Option<String>,
    activate: bool,
) {
    if activate {
        updates.insert(
            0,
            CoreUpdate::ActivatePlugin {
                plugin_id: plugin_id.to_string(),
            },
        );
    }

    // Check if first item has immediate execute action (openUrl/copy)
    if let Some(first_item) = items.first() {
        if let Some(ref url) = first_item.open_url {
            updates.push(CoreUpdate::Execute {
                action: ExecuteAction::OpenUrl { url: url.clone() },
            });
            return;
        }

        if let Some(ref text) = first_item.copy {
            updates.push(CoreUpdate::Execute {
                action: ExecuteAction::Copy { text: text.clone() },
            });
            return;
        }
    }

    let results = convert_plugin_results(plugin_id, items);
    updates.push(CoreUpdate::Results {
        results,
        placeholder: None,
        clear_input: None,
        input_mode: None,
        context: None,
        navigate_forward,
        display_hint: display_hint.and_then(|s| parse_display_hint(&s)),
    });

    if let Some(mode) = input_mode {
        updates.push(CoreUpdate::InputModeChanged {
            mode: if mode == "submit" {
                "submit"
            } else {
                "realtime"
            }
            .to_string(),
        });
    }

    if context.is_some() {
        updates.push(CoreUpdate::ContextChanged { context });
    }

    if let Some(placeholder_text) = placeholder {
        updates.push(CoreUpdate::Placeholder {
            placeholder: placeholder_text,
        });
    }

    if clear_input {
        updates.push(CoreUpdate::ClearInput);
    }

    if !plugin_actions.is_empty() {
        updates.push(CoreUpdate::PluginActionsUpdate {
            actions: plugin_actions
                .into_iter()
                .map(|a| PluginAction {
                    id: a.id,
                    name: a.name,
                    icon: a.icon,
                    shortcut: a.shortcut,
                    confirm: a.confirm,
                    active: a.active,
                })
                .collect(),
        });
    }

    if let Some(status_data) = status {
        updates.extend(process_status_data(plugin_id, status_data));
    }

    if let Some(depth) = navigation_depth {
        updates.push(CoreUpdate::NavigationDepthChanged { depth });
    }
}

/// Handle `PluginResponse::Execute` variant
fn handle_execute_response(updates: &mut Vec<CoreUpdate>, data: super::protocol::ExecuteData) {
    if let Some(desktop_file) = data.launch {
        updates.push(CoreUpdate::Execute {
            action: ExecuteAction::Launch { desktop_file },
        });
    }

    if let Some(url) = data.open_url {
        updates.push(CoreUpdate::Execute {
            action: ExecuteAction::OpenUrl { url },
        });
    }

    if let Some(path) = data.open {
        updates.push(CoreUpdate::Execute {
            action: ExecuteAction::Open { path },
        });
    }

    if let Some(text) = data.copy {
        updates.push(CoreUpdate::Execute {
            action: ExecuteAction::Copy { text },
        });
    }

    if let Some(text) = data.type_text {
        updates.push(CoreUpdate::Execute {
            action: ExecuteAction::TypeText { text },
        });
    }

    if let Some(message) = data.notify {
        updates.push(CoreUpdate::Execute {
            action: ExecuteAction::Notify { message },
        });
    }

    if let Some(sound) = data.sound {
        updates.push(CoreUpdate::Execute {
            action: ExecuteAction::PlaySound { sound },
        });
    }

    if data.close == Some(true) {
        updates.push(CoreUpdate::Close);
    }
}

/// Handle `PluginResponse::Card` variant
fn handle_card_response(
    plugin_id: &str,
    updates: &mut Vec<CoreUpdate>,
    card: super::protocol::CardResponseData,
    status: Option<StatusData>,
    context: Option<String>,
) {
    let markdown_content = if card
        .markdown
        .as_ref()
        .is_some_and(std::string::String::is_empty)
    {
        card.content.clone()
    } else {
        card.markdown
    };

    updates.push(CoreUpdate::Card {
        card: CardData {
            title: card.title,
            content: card.content,
            markdown: markdown_content,
            actions: card
                .actions
                .into_iter()
                .map(|a| Action {
                    id: a.id,
                    name: a.name,
                    icon: a.icon,
                    icon_type: a.icon_type,
                    keep_open: a.keep_open,
                })
                .collect(),
            kind: card.kind,
            blocks: card
                .blocks
                .into_iter()
                .map(|b| match b {
                    CardBlockData::Pill { text } => CardBlock::Pill { text },
                    CardBlockData::Separator => CardBlock::Separator,
                    CardBlockData::Message { role, content } => {
                        CardBlock::Message { role, content }
                    }
                    CardBlockData::Note { content } => CardBlock::Note { content },
                })
                .collect(),
            max_height: card.max_height,
            show_details: card.show_details,
            allow_toggle_details: card.allow_toggle_details,
        },
        context,
    });

    if let Some(status_data) = status {
        updates.extend(process_status_data(plugin_id, status_data));
    }
}

/// Handle `PluginResponse::Form` variant
fn handle_form_response(
    updates: &mut Vec<CoreUpdate>,
    form: super::protocol::FormResponseData,
    context: Option<String>,
    navigate_forward: Option<bool>,
) {
    let form_data = FormData {
        title: form.title,
        fields: form
            .fields
            .into_iter()
            .map(|f| FormField {
                id: f.id,
                label: f.label,
                field_type: parse_field_type(f.field_type.as_deref()),
                placeholder: f.placeholder,
                default_value: f.default_value,
                required: f.required,
                options: f.options,
                hint: f.hint,
                rows: f.rows,
                min: f.min,
                max: f.max,
                step: f.step,
            })
            .collect(),
        submit_label: form.submit_label,
        cancel_label: form.cancel_label,
        context,
        live_update: form.live_update,
    };

    if form_data.context.is_some() {
        updates.push(CoreUpdate::ContextChanged {
            context: form_data.context.clone(),
        });
    }

    updates.push(CoreUpdate::Form { form: form_data });

    if let Some(true) = navigate_forward {
        updates.push(CoreUpdate::NavigateForward);
    }
}

/// Parse a field type string into a `FormFieldType` enum
fn parse_field_type(field_type: Option<&str>) -> FormFieldType {
    match field_type {
        Some("password") => FormFieldType::Password,
        Some("number") => FormFieldType::Number,
        Some("textarea") => FormFieldType::TextArea,
        Some("select") => FormFieldType::Select,
        Some("checkbox") => FormFieldType::Checkbox,
        Some("switch") => FormFieldType::Switch,
        Some("slider") => FormFieldType::Slider,
        Some("hidden") => FormFieldType::Hidden,
        Some("date") => FormFieldType::Date,
        Some("time") => FormFieldType::Time,
        _ => FormFieldType::Text,
    }
}

/// Handle `PluginResponse::Match` variant
fn handle_match_response(
    plugin_id: &str,
    updates: &mut Vec<CoreUpdate>,
    result: Option<ResultItem>,
) {
    let results = result.map_or_else(Vec::new, |item| {
        convert_plugin_results(plugin_id, vec![item])
    });
    updates.push(CoreUpdate::Results {
        results,
        placeholder: None,
        clear_input: None,
        input_mode: None,
        context: None,
        navigate_forward: None,
        display_hint: None,
    });
}

/// Handle `PluginResponse::Update` variant
fn handle_update_response(
    plugin_id: &str,
    updates: &mut Vec<CoreUpdate>,
    items: Option<Vec<UpdateItem>>,
    status: Option<StatusData>,
) {
    if let Some(update_items) = items {
        let patches = convert_update_items(update_items);
        if !patches.is_empty() {
            updates.push(CoreUpdate::ResultsUpdate { patches });
        }
    }

    if let Some(status_data) = status {
        updates.extend(process_status_data(plugin_id, status_data));
    }
}

/// Handle `PluginResponse::ImageBrowser` variant
fn handle_image_browser_response(
    updates: &mut Vec<CoreUpdate>,
    images: Vec<hamr_types::ImageItem>,
    title: Option<String>,
    directory: Option<String>,
    image_browser: Option<super::protocol::ImageBrowserInner>,
) {
    let mut all_images = images;
    let mut dir = directory;

    if let Some(inner) = image_browser {
        if dir.is_none() {
            dir = inner.directory;
        }
        all_images.extend(inner.images);
    }

    updates.push(CoreUpdate::ImageBrowser {
        browser: ImageBrowserData {
            directory: dir,
            images: all_images,
            title,
        },
    });
}

/// Process status data from plugin and return `CoreUpdates`
#[must_use]
pub fn process_status_data(plugin_id: &str, status: StatusData) -> Vec<CoreUpdate> {
    let mut updates = Vec::new();

    let plugin_status = PluginStatus {
        badges: status.badges,
        chips: status.chips,
        description: status.description,
        fab: status.fab.map(|f| FabOverride {
            badges: f.badges,
            chips: f.chips,
            priority: f.priority,
            show_fab: f.show_fab,
        }),
        ambient: status
            .ambient
            .as_ref()
            .map(|items| {
                items
                    .iter()
                    .map(|item| convert_ambient_item(plugin_id, item.clone()))
                    .collect()
            })
            .unwrap_or_default(),
    };

    if !plugin_status.badges.is_empty()
        || !plugin_status.chips.is_empty()
        || plugin_status.description.is_some()
    {
        updates.push(CoreUpdate::PluginStatusUpdate {
            plugin_id: plugin_id.to_string(),
            status: plugin_status.clone(),
        });
    }

    if status.ambient.is_some() {
        updates.push(CoreUpdate::AmbientUpdate {
            plugin_id: plugin_id.to_string(),
            items: plugin_status.ambient.clone(),
        });
    }

    if let Some(fab) = plugin_status.fab {
        updates.push(CoreUpdate::FabUpdate { fab: Some(fab) });
    }

    updates
}

fn convert_ambient_item(plugin_id: &str, item: AmbientItemData) -> AmbientItem {
    AmbientItem {
        id: item.id,
        name: item.name,
        description: item.description,
        icon: item.icon,
        badges: item.badges,
        chips: item.chips,
        actions: item
            .actions
            .into_iter()
            .map(|a| Action {
                id: a.id,
                name: a.name,
                icon: a.icon,
                icon_type: a.icon_type,
                keep_open: a.keep_open,
            })
            .collect(),
        duration: item.duration,
        plugin_id: Some(plugin_id.to_string()),
    }
}

/// Convert plugin `ResultItems` to `SearchResults`.
///
/// Since `hamr_types::ResultItem` and `SearchResult` are the same type,
/// this primarily sets the `plugin_id` and ensures defaults are applied.
fn convert_plugin_results(plugin_id: &str, items: Vec<ResultItem>) -> Vec<SearchResult> {
    items
        .into_iter()
        .map(|mut item| {
            item.plugin_id = Some(plugin_id.to_string());

            if item.icon.is_none() {
                item.icon = Some("extension".to_string());
            }

            if item.verb.is_none() {
                item.verb = Some("Select".to_string());
            }

            item
        })
        .collect()
}

fn convert_update_items(items: Vec<UpdateItem>) -> Vec<ResultPatch> {
    items
        .into_iter()
        .map(|item| {
            let mut patch = ResultPatch {
                id: item.id,
                ..Default::default()
            };

            if let serde_json::Value::Object(fields) = item.fields {
                patch.name = fields
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                patch.description = fields
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                patch.icon = fields
                    .get("icon")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                patch.badges = fields
                    .get("badges")
                    .and_then(|v| serde_json::from_value(v.clone()).ok());
                patch.chips = fields
                    .get("chips")
                    .and_then(|v| serde_json::from_value(v.clone()).ok());
                patch.value = fields
                    .get("value")
                    .and_then(|v| serde_json::from_value(v.clone()).ok());
                patch.gauge = fields
                    .get("gauge")
                    .and_then(|v| serde_json::from_value(v.clone()).ok());
                patch.progress = fields
                    .get("progress")
                    .and_then(|v| serde_json::from_value(v.clone()).ok());

                // Build widget from flat fields (populate both for backward compatibility)
                if let Some(ref slider_val) = patch.value {
                    patch.widget = Some(WidgetData::Slider {
                        value: slider_val.value,
                        min: slider_val.min,
                        max: slider_val.max,
                        step: slider_val.step,
                        display_value: slider_val.display_value.clone(),
                    });
                } else if let Some(ref gauge) = patch.gauge {
                    patch.widget = Some(WidgetData::Gauge {
                        value: gauge.value,
                        min: gauge.min,
                        max: gauge.max,
                        label: gauge.label.clone(),
                        color: gauge.color.clone(),
                    });
                } else if let Some(ref progress) = patch.progress {
                    patch.widget = Some(WidgetData::Progress {
                        value: progress.value,
                        max: progress.max,
                        label: progress.label.clone(),
                        color: progress.color.clone(),
                    });
                } else if let Some(graph) = fields.get("graph")
                    && let Ok(graph_data) =
                        serde_json::from_value::<hamr_types::GraphData>(graph.clone())
                {
                    patch.graph = Some(graph_data.clone());
                    patch.widget = Some(WidgetData::Graph {
                        data: graph_data.data,
                        min: graph_data.min,
                        max: graph_data.max,
                    });
                }
            }

            patch
        })
        .collect()
}

/// Parse a display hint string into a `DisplayHint` enum
fn parse_display_hint(hint: &str) -> Option<DisplayHint> {
    match hint.to_lowercase().as_str() {
        "auto" => Some(DisplayHint::Auto),
        "list" => Some(DisplayHint::List),
        "grid" => Some(DisplayHint::Grid),
        "large_grid" | "largegrid" => Some(DisplayHint::LargeGrid),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convert(plugin_id: &str, json: &str) -> Vec<CoreUpdate> {
        let response: PluginResponse = serde_json::from_str(json).unwrap();
        plugin_response_to_updates(plugin_id, response)
    }

    #[test]
    fn test_convert_results_basic() {
        let json = r#"{
            "type": "results",
            "items": [
                {"id": "r1", "name": "Result One"},
                {"id": "r2", "name": "Result Two", "description": "A description"}
            ]
        }"#;

        let updates = convert("test-plugin", json);

        assert!(updates.len() >= 2);
        match &updates[1] {
            CoreUpdate::Results { results, .. } => {
                assert_eq!(results.len(), 2);
                assert_eq!(results[0].id, "r1");
                assert_eq!(results[0].name, "Result One");
                assert_eq!(results[1].description, Some("A description".to_string()));
            }
            _ => panic!("Expected Results update"),
        }
    }

    #[test]
    fn test_convert_results_with_context() {
        let json = r#"{
            "type": "results",
            "items": [],
            "context": "edit:category.key"
        }"#;

        let updates = convert("test-plugin", json);

        let has_context = updates
            .iter()
            .any(|u| matches!(u, CoreUpdate::ContextChanged { context } if context == &Some("edit:category.key".to_string())));
        assert!(has_context, "Should have ContextChanged update");
    }

    #[test]
    fn test_convert_results_with_placeholder() {
        let json = r#"{
            "type": "results",
            "items": [],
            "placeholder": "Search notes..."
        }"#;

        let updates = convert("test-plugin", json);

        let has_placeholder = updates.iter().any(
            |u| matches!(u, CoreUpdate::Placeholder { placeholder } if placeholder == "Search notes..."),
        );
        assert!(has_placeholder, "Should have Placeholder update");
    }

    #[test]
    fn test_convert_results_with_navigation() {
        let json = r#"{
            "type": "results",
            "items": [],
            "navigateForward": true,
            "clearInput": true
        }"#;

        let updates = convert("test-plugin", json);

        let results_update = updates
            .iter()
            .find(|u| matches!(u, CoreUpdate::Results { .. }));
        match results_update {
            Some(CoreUpdate::Results {
                navigate_forward, ..
            }) => {
                assert_eq!(navigate_forward, &Some(true));
            }
            _ => panic!("Expected Results update"),
        }

        let has_clear = updates.iter().any(|u| matches!(u, CoreUpdate::ClearInput));
        assert!(has_clear, "Should have ClearInput update");
    }

    #[test]
    fn test_convert_form_basic() {
        let json = r#"{
            "type": "form",
            "form": {
                "title": "Edit Setting",
                "fields": [
                    {"id": "value", "label": "Value"}
                ],
                "submitLabel": "Save"
            },
            "context": "edit:search.maxEntries"
        }"#;

        let updates = convert("test-plugin", json);

        let form_update = updates
            .iter()
            .find(|u| matches!(u, CoreUpdate::Form { .. }));
        match form_update {
            Some(CoreUpdate::Form { form }) => {
                assert_eq!(form.title, "Edit Setting");
                assert_eq!(form.fields.len(), 1);
                assert_eq!(form.submit_label, "Save");
                assert_eq!(form.context, Some("edit:search.maxEntries".to_string()));
            }
            _ => panic!("Expected Form update"),
        }
    }

    #[test]
    fn test_convert_form_with_live_update() {
        let json = r#"{
            "type": "form",
            "form": {
                "title": "Appearance",
                "fields": [],
                "liveUpdate": true
            }
        }"#;

        let updates = convert("test-plugin", json);

        let form_update = updates
            .iter()
            .find(|u| matches!(u, CoreUpdate::Form { .. }));
        match form_update {
            Some(CoreUpdate::Form { form }) => {
                assert!(form.live_update);
            }
            _ => panic!("Expected Form update"),
        }
    }

    #[test]
    fn test_convert_card_basic() {
        let json = r#"{
            "type": "card",
            "card": {
                "title": "Info Card",
                "content": "Some content here"
            }
        }"#;

        let updates = convert("test-plugin", json);

        let card_update = updates
            .iter()
            .find(|u| matches!(u, CoreUpdate::Card { .. }));
        match card_update {
            Some(CoreUpdate::Card { card, .. }) => {
                assert_eq!(card.title, "Info Card");
                assert_eq!(card.content, Some("Some content here".to_string()));
            }
            _ => panic!("Expected Card update"),
        }
    }

    #[test]
    fn test_convert_card_with_markdown() {
        let json = r#"{
            "type": "card",
            "card": {
                "title": "Help",
                "markdown": "Some **bold** text."
            }
        }"#;

        let updates = convert("test-plugin", json);

        let card_update = updates
            .iter()
            .find(|u| matches!(u, CoreUpdate::Card { .. }));
        match card_update {
            Some(CoreUpdate::Card { card, .. }) => {
                assert!(card.markdown.as_ref().unwrap().contains("**bold**"));
            }
            _ => panic!("Expected Card update"),
        }
    }

    #[test]
    fn test_convert_execute_launch() {
        let json = r#"{
            "type": "execute",
            "launch": "/usr/share/applications/firefox.desktop"
        }"#;

        let updates = convert("test-plugin", json);

        let execute_updates: Vec<_> = updates
            .iter()
            .filter(|u| matches!(u, CoreUpdate::Execute { .. }))
            .collect();
        assert!(!execute_updates.is_empty());

        match &execute_updates[0] {
            CoreUpdate::Execute { action } => match action {
                ExecuteAction::Launch { desktop_file } => {
                    assert_eq!(desktop_file, "/usr/share/applications/firefox.desktop");
                }
                _ => panic!("Expected Launch action"),
            },
            _ => panic!("Expected Execute update"),
        }
    }

    #[test]
    fn test_convert_execute_copy() {
        let json = r#"{
            "type": "execute",
            "copy": "Hello, World!"
        }"#;

        let updates = convert("test-plugin", json);

        let execute_updates: Vec<_> = updates
            .iter()
            .filter(|u| matches!(u, CoreUpdate::Execute { .. }))
            .collect();

        match &execute_updates[0] {
            CoreUpdate::Execute { action } => match action {
                ExecuteAction::Copy { text } => {
                    assert_eq!(text, "Hello, World!");
                }
                _ => panic!("Expected Copy action"),
            },
            _ => panic!("Expected Execute update"),
        }
    }

    #[test]
    fn test_convert_execute_multiple_actions() {
        let json = r#"{
            "type": "execute",
            "copy": "Copied text",
            "notify": "Text copied!"
        }"#;

        let updates = convert("test-plugin", json);

        let execute_updates: Vec<_> = updates
            .iter()
            .filter(|u| matches!(u, CoreUpdate::Execute { .. }))
            .collect();
        assert_eq!(execute_updates.len(), 2);
    }

    #[test]
    fn test_convert_execute_with_close() {
        let json = r#"{
            "type": "execute",
            "copy": "test",
            "close": true
        }"#;

        let updates = convert("test-plugin", json);

        let has_close = updates.iter().any(|u| matches!(u, CoreUpdate::Close));
        assert!(has_close);
    }

    #[test]
    fn test_convert_execute_open_url() {
        let json = r#"{
            "type": "execute",
            "openUrl": "https://example.com"
        }"#;

        let updates = convert("test-plugin", json);

        let execute_update = updates
            .iter()
            .find(|u| matches!(u, CoreUpdate::Execute { .. }));
        match execute_update {
            Some(CoreUpdate::Execute { action }) => match action {
                ExecuteAction::OpenUrl { url } => {
                    assert_eq!(url, "https://example.com");
                }
                _ => panic!("Expected OpenUrl action"),
            },
            _ => panic!("Expected Execute update"),
        }
    }

    #[test]
    fn test_convert_image_browser() {
        let json = r#"{
            "type": "imageBrowser",
            "title": "Select Image",
            "directory": "/home/user/pictures",
            "images": [
                {"path": "/home/user/pictures/photo1.jpg", "name": "Photo 1"}
            ]
        }"#;

        let updates = convert("test-plugin", json);

        let browser_update = updates
            .iter()
            .find(|u| matches!(u, CoreUpdate::ImageBrowser { .. }));
        match browser_update {
            Some(CoreUpdate::ImageBrowser { browser }) => {
                assert_eq!(browser.title, Some("Select Image".to_string()));
                assert_eq!(browser.directory, Some("/home/user/pictures".to_string()));
                assert_eq!(browser.images.len(), 1);
            }
            _ => panic!("Expected ImageBrowser update"),
        }
    }

    #[test]
    fn test_convert_grid_browser() {
        let json = r#"{
            "type": "gridBrowser",
            "title": "Select Emoji",
            "columns": 8,
            "items": [
                {"id": "smile", "name": "Smile", "icon": "sentiment_satisfied"}
            ]
        }"#;

        let updates = convert("test-plugin", json);

        let browser_update = updates
            .iter()
            .find(|u| matches!(u, CoreUpdate::GridBrowser { .. }));
        match browser_update {
            Some(CoreUpdate::GridBrowser { browser }) => {
                assert_eq!(browser.title, Some("Select Emoji".to_string()));
                assert_eq!(browser.columns, Some(8));
                assert_eq!(browser.items.len(), 1);
            }
            _ => panic!("Expected GridBrowser update"),
        }
    }

    #[test]
    fn test_convert_error() {
        let json = r#"{
            "type": "error",
            "message": "Something went wrong"
        }"#;

        let updates = convert("test-plugin", json);

        let error_update = updates
            .iter()
            .find(|u| matches!(u, CoreUpdate::Error { .. }));
        match error_update {
            Some(CoreUpdate::Error { message }) => {
                assert_eq!(message, "Something went wrong");
            }
            _ => panic!("Expected Error update"),
        }
    }

    #[test]
    fn test_convert_prompt() {
        let json = r#"{
            "type": "prompt",
            "prompt": {
                "text": "Enter your name:"
            }
        }"#;

        let updates = convert("test-plugin", json);

        let placeholder_update = updates
            .iter()
            .find(|u| matches!(u, CoreUpdate::Placeholder { .. }));
        match placeholder_update {
            Some(CoreUpdate::Placeholder { placeholder }) => {
                assert_eq!(placeholder, "Enter your name:");
            }
            _ => panic!("Expected Placeholder update"),
        }
    }

    #[test]
    fn test_convert_noop() {
        let json = r#"{"type": "noop"}"#;

        let updates = convert("test-plugin", json);

        assert_eq!(updates.len(), 1);
        assert!(matches!(updates[0], CoreUpdate::Busy { busy: false }));
    }

    #[test]
    fn test_convert_results_with_status() {
        let json = r#"{
            "type": "results",
            "items": [],
            "status": {
                "badges": [{"text": "5"}],
                "description": "5 items"
            }
        }"#;

        let updates = convert("test-plugin", json);

        let has_status = updates
            .iter()
            .any(|u| matches!(u, CoreUpdate::PluginStatusUpdate { .. }));
        assert!(has_status);
    }

    #[test]
    fn test_convert_status_response() {
        let json = r#"{
            "type": "status",
            "status": {
                "badges": [{"text": "NEW"}],
                "description": "New items available"
            }
        }"#;

        let updates = convert("test-plugin", json);

        assert!(!updates.is_empty());
        let has_status = updates
            .iter()
            .any(|u| matches!(u, CoreUpdate::PluginStatusUpdate { .. }));
        assert!(has_status);
    }
}
