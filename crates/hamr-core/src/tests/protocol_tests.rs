//! Tests for plugin protocol: JSON parsing, index updates, responses

use crate::plugin::{IndexItem, PluginResponse};
use hamr_types::WidgetData;

#[test]
fn test_parse_index_response() {
    let json = r#"{
        "type": "index",
        "items": [
            {"id": "app1", "name": "App One", "verb": "Open"},
            {"id": "app2", "name": "App Two", "description": "Second app"}
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Index {
            items,
            mode,
            remove,
            status: _,
        } => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].id, "app1");
            assert_eq!(items[0].name, "App One");
            assert_eq!(items[1].description, Some("Second app".to_string()));
            assert!(mode.is_none());
            assert!(remove.is_none());
        }
        _ => panic!("Expected Index response"),
    }
}

#[test]
fn test_parse_index_with_keywords() {
    let json = r#"{
        "type": "index",
        "items": [
            {
                "id": "firefox",
                "name": "Firefox",
                "keywords": ["browser", "web", "mozilla"]
            }
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Index { items, .. } => {
            let keywords = items[0].keywords.as_ref().unwrap();
            assert_eq!(keywords.len(), 3);
            assert!(keywords.contains(&"browser".to_string()));
        }
        _ => panic!("Expected Index response"),
    }
}

#[test]
fn test_parse_index_with_entry_point() {
    let json = r#"{
        "type": "index",
        "items": [
            {
                "id": "shutdown",
                "name": "Shutdown",
                "entryPoint": {"selected": {"id": "shutdown"}, "step": "action"}
            }
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Index { items, .. } => {
            assert!(items[0].entry_point.is_some());
            let ep = items[0].entry_point.as_ref().unwrap();
            assert!(ep.get("step").is_some());
        }
        _ => panic!("Expected Index response"),
    }
}

#[test]
fn test_parse_index_incremental_mode() {
    let json = r#"{
        "type": "index",
        "mode": "incremental",
        "items": [{"id": "new", "name": "New Item"}],
        "remove": ["old1", "old2"]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Index { mode, remove, .. } => {
            assert_eq!(mode, Some("incremental".to_string()));
            let remove = remove.unwrap();
            assert_eq!(remove.len(), 2);
            assert!(remove.contains(&"old1".to_string()));
        }
        _ => panic!("Expected Index response"),
    }
}

#[test]
fn test_parse_results_response() {
    let json = r#"{
        "type": "results",
        "items": [
            {"id": "r1", "name": "Result One", "verb": "Select"},
            {"id": "r2", "name": "Result Two", "description": "With desc"}
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Results {
            items,
            prepend,
            input_mode,
            ..
        } => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].id, "r1");
            assert!(!prepend);
            assert!(input_mode.is_none());
        }
        _ => panic!("Expected Results response"),
    }
}

#[test]
fn test_parse_results_with_slider_type() {
    // This tests that plugin results with "type": "slider" are parsed correctly
    let json = r#"{
        "type": "results",
        "results": [
            {
                "id": "setting:sizes.searchWidth",
                "name": "searchWidth",
                "description": "Launcher search bar width (px)",
                "icon": "tune",
                "type": "slider",
                "value": 580.0,
                "min": 400,
                "max": 1000,
                "step": 10
            }
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Results { items, .. } => {
            assert_eq!(items.len(), 1);
            let item = &items[0];
            assert_eq!(item.id, "setting:sizes.searchWidth");
            assert_eq!(
                item.result_type,
                hamr_types::ResultType::Slider,
                "type field should be parsed as Slider"
            );
            assert!(
                matches!(
                    item.widget,
                    Some(WidgetData::Slider { value, min, max, step, .. })
                    if value == 580.0 && min == 400.0 && max == 1000.0 && step == 10.0
                ),
                "widget field should contain Slider data"
            );
        }
        _ => panic!("Expected Results response"),
    }
}

#[test]
fn test_parse_results_with_badges() {
    let json = r#"{
        "type": "results",
        "items": [
            {
                "id": "item1",
                "name": "Item with Badge",
                "badges": [{"text": "5"}, {"text": "NEW", "color": "green"}]
            }
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Results { items, .. } => {
            let badges = &items[0].badges;
            assert_eq!(badges.len(), 2);
            assert_eq!(badges[0].text, Some("5".to_string()));
            assert_eq!(badges[1].text, Some("NEW".to_string()));
        }
        _ => panic!("Expected Results response"),
    }
}

#[test]
fn test_parse_results_with_actions() {
    let json = r#"{
        "type": "results",
        "items": [
            {
                "id": "item1",
                "name": "Item",
                "actions": [
                    {"id": "copy", "name": "Copy"},
                    {"id": "delete", "name": "Delete", "icon": "trash"}
                ]
            }
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Results { items, .. } => {
            let actions = &items[0].actions;
            assert_eq!(actions.len(), 2);
            assert_eq!(actions[0].id, "copy");
            assert_eq!(actions[1].icon, Some("trash".to_string()));
        }
        _ => panic!("Expected Results response"),
    }
}

#[test]
fn test_parse_switch_value_as_bool() {
    // Switches send value as true/false, should be converted to 1.0/0.0
    let json = r#"{
        "type": "index",
        "items": [
            {"id": "switch1", "name": "Enabled Switch", "type": "switch", "value": true},
            {"id": "switch2", "name": "Disabled Switch", "type": "switch", "value": false}
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Index { items, .. } => {
            assert!(
                matches!(items[0].widget, Some(WidgetData::Switch { value: true })),
                "enabled switch should have widget.value = true"
            );
            assert!(
                matches!(items[1].widget, Some(WidgetData::Switch { value: false })),
                "disabled switch should have widget.value = false"
            );
        }
        _ => panic!("Expected Index response"),
    }
}

#[test]
fn test_parse_slider_value_as_number() {
    let json = r#"{
        "type": "results",
        "items": [
            {
                "id": "volume",
                "name": "Volume",
                "type": "slider",
                "value": 75.5,
                "min": 0,
                "max": 100,
                "step": 5
            }
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Results { items, .. } => {
            assert!(
                matches!(
                    items[0].widget,
                    Some(WidgetData::Slider { value, min, max, step, .. })
                    if value == 75.5 && min == 0.0 && max == 100.0 && step == 5.0
                ),
                "widget field should contain Slider data"
            );
        }
        _ => panic!("Expected Results response"),
    }
}

#[test]
fn test_parse_execute_response() {
    let json = r#"{
        "type": "execute",
        "launch": "/usr/share/applications/firefox.desktop"
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Execute(data) => {
            assert_eq!(
                data.launch,
                Some("/usr/share/applications/firefox.desktop".to_string())
            );
        }
        _ => panic!("Expected Execute response"),
    }
}

#[test]
fn test_parse_execute_open_url() {
    let json = r#"{
        "type": "execute",
        "openUrl": "https://example.com"
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Execute(data) => {
            assert_eq!(data.open_url, Some("https://example.com".to_string()));
        }
        _ => panic!("Expected Execute response"),
    }
}

#[test]
fn test_parse_execute_open_path() {
    let json = r#"{
        "type": "execute",
        "open": "/home/user/documents"
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Execute(data) => {
            assert_eq!(data.open, Some("/home/user/documents".to_string()));
        }
        _ => panic!("Expected Execute response"),
    }
}

#[test]
fn test_parse_execute_notify() {
    let json = r#"{
        "type": "execute",
        "notify": "Action completed!",
        "close": true
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Execute(data) => {
            assert_eq!(data.notify, Some("Action completed!".to_string()));
            assert_eq!(data.close, Some(true));
        }
        _ => panic!("Expected Execute response"),
    }
}

#[test]
fn test_parse_execute_with_copy() {
    let json = r#"{
        "type": "execute",
        "copy": "Hello, World!",
        "close": true
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Execute(data) => {
            assert_eq!(data.copy, Some("Hello, World!".to_string()));
            assert_eq!(data.close, Some(true));
        }
        _ => panic!("Expected Execute response"),
    }
}

#[test]
fn test_parse_card_response() {
    let json = r#"{
        "type": "card",
        "card": {
            "title": "Result Card",
            "content": "This is the content",
            "actions": [{"id": "ok", "name": "OK"}]
        }
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Card { card, .. } => {
            assert_eq!(card.title, "Result Card");
            assert_eq!(card.content, Some("This is the content".to_string()));
            assert_eq!(card.actions.len(), 1);
        }
        _ => panic!("Expected Card response"),
    }
}

#[test]
fn test_parse_card_response_with_context() {
    // This is the format notes plugin sends
    let json = r#"{
        "type": "card",
        "card": {
            "content": "Test Note Content",
            "markdown": true,
            "actions": [
                {"id": "edit", "name": "Edit", "icon": "edit"},
                {"id": "copy", "name": "Copy", "icon": "content_copy"},
                {"id": "delete", "name": "Delete", "icon": "delete"},
                {"id": "back", "name": "Back", "icon": "arrow_back"}
            ]
        },
        "context": "note_123456"
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Card { card, context, .. } => {
            assert_eq!(card.content, Some("Test Note Content".to_string()));
            assert_eq!(card.actions.len(), 4);
            assert_eq!(
                context,
                Some("note_123456".to_string()),
                "Context should be parsed from JSON"
            );
        }
        _ => panic!("Expected Card response"),
    }
}

#[test]
fn test_parse_form_response() {
    let json = r#"{
        "type": "form",
        "form": {
            "title": "Create Item",
            "fields": [
                {"id": "name", "label": "Name", "required": true},
                {"id": "desc", "label": "Description", "type": "textarea"}
            ],
            "submitLabel": "Create"
        },
        "context": "create_new"
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Form { form, context, .. } => {
            assert_eq!(form.title, "Create Item");
            assert_eq!(form.fields.len(), 2);
            assert!(form.fields[0].required);
            assert_eq!(form.submit_label, "Create".to_string());
            assert_eq!(context, Some("create_new".to_string()));
        }
        _ => panic!("Expected Form response"),
    }
}

#[test]
fn test_parse_form_response_notes_plugin_format() {
    // Exact format from notes plugin
    let json = r#"{"type": "form", "form": {"title": "Add New Note", "submitLabel": "Save", "cancelLabel": "Cancel", "fields": [{"id": "title", "type": "text", "label": "Title", "placeholder": "Enter note title...", "required": true, "default": "test_value"}, {"id": "content", "type": "textarea", "label": "Content", "placeholder": "Enter note content...", "rows": 6}]}, "context": "__add__"}"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Form { form, context, .. } => {
            assert_eq!(form.title, "Add New Note");
            assert_eq!(form.submit_label, "Save");
            assert_eq!(form.cancel_label, Some("Cancel".to_string()));
            assert_eq!(form.fields.len(), 2);

            // Title field
            assert_eq!(form.fields[0].id, "title");
            assert_eq!(form.fields[0].field_type, Some("text".to_string()));
            assert!(form.fields[0].required);
            assert_eq!(form.fields[0].default_value, Some("test_value".to_string()));

            // Content field
            assert_eq!(form.fields[1].id, "content");
            assert_eq!(form.fields[1].field_type, Some("textarea".to_string()));
            assert_eq!(form.fields[1].rows, Some(6));

            // Context
            assert_eq!(context, Some("__add__".to_string()));
        }
        _ => panic!("Expected Form response"),
    }
}

#[test]
fn test_parse_form_response_with_navigate_forward() {
    let json = r#"{
        "type": "form",
        "form": {"title": "Edit Setting", "submitLabel": "Save", "fields": []},
        "context": "edit:test",
        "navigateForward": true
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Form {
            form,
            context,
            navigate_forward,
        } => {
            assert_eq!(form.title, "Edit Setting");
            assert_eq!(context, Some("edit:test".to_string()));
            assert_eq!(
                navigate_forward,
                Some(true),
                "navigateForward should be parsed"
            );
        }
        _ => panic!("Expected Form response"),
    }
}

#[test]
fn test_parse_error_response() {
    let json = r#"{
        "type": "error",
        "message": "Something went wrong",
        "details": "Stack trace here"
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Error { message, details } => {
            assert_eq!(message, "Something went wrong");
            assert_eq!(details, Some("Stack trace here".to_string()));
        }
        _ => panic!("Expected Error response"),
    }
}

#[test]
fn test_parse_update_response() {
    let json = r#"{
        "type": "update",
        "items": [
            {"id": "item1", "badges": [{"text": "10"}]}
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Update { items, .. } => {
            let items = items.unwrap();
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].id, "item1");
        }
        _ => panic!("Expected Update response"),
    }
}

#[test]
fn test_parse_status_response() {
    let json = r#"{
        "type": "status",
        "status": {
            "badges": [{"text": "5", "color": "red"}],
            "description": "5 items pending"
        }
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Status { status } => {
            assert_eq!(status.badges.len(), 1);
            assert_eq!(status.description, Some("5 items pending".to_string()));
        }
        _ => panic!("Expected Status response"),
    }
}

#[test]
fn test_index_item_serialization() {
    let item = IndexItem {
        id: "test".to_string(),
        name: "Test Item".to_string(),
        description: Some("A test".to_string()),
        icon: Some("test_icon".to_string()),
        keywords: Some(vec!["test".to_string(), "item".to_string()]),
        verb: Some("Run".to_string()),
        ..Default::default()
    }
    .with_slider(42.0, 0.0, 100.0, 1.0, None);

    let json = serde_json::to_string(&item).unwrap();
    assert!(json.contains("\"id\":\"test\""));
    assert!(json.contains("\"name\":\"Test Item\""));
    // Verify the widget field is populated
    assert!(
        matches!(
            item.widget,
            Some(WidgetData::Slider { value, .. }) if value == 42.0
        ),
        "widget should contain Slider with value 42.0"
    );
}

#[test]
fn test_plugin_input_camelcase_serialization() {
    use crate::plugin::{PluginInput, Step};
    use std::collections::HashMap;

    let mut form_data = HashMap::new();
    form_data.insert("title".to_string(), "Test Note".to_string());
    form_data.insert("content".to_string(), "Test content".to_string());

    let input = PluginInput {
        step: Step::Form,
        query: Some("test".to_string()),
        selected: None,
        action: None,
        session: Some("session123".to_string()),
        context: Some("__add__".to_string()),
        value: None,
        form_data: Some(serde_json::to_value(&form_data).unwrap()),
        source: None,
    };

    let json = serde_json::to_string(&input).unwrap();

    // Verify camelCase serialization
    assert!(
        json.contains("\"formData\""),
        "formData should be camelCase"
    );
    assert!(
        !json.contains("\"form_data\""),
        "form_data should not be snake_case"
    );
    assert!(json.contains("\"step\":\"form\""));
    assert!(json.contains("\"context\":\"__add__\""));
    assert!(json.contains("\"session\":\"session123\""));
}

#[test]
fn test_parse_image_browser_response() {
    let json = r#"{
        "type": "imageBrowser",
        "title": "Select Image",
        "directory": "/home/user/pictures",
        "images": [
            {"path": "/home/user/pictures/photo1.jpg", "name": "Photo 1"},
            {"path": "/home/user/pictures/photo2.png"}
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::ImageBrowser {
            images,
            title,
            directory,
            ..
        } => {
            assert_eq!(title, Some("Select Image".to_string()));
            assert_eq!(directory, Some("/home/user/pictures".to_string()));
            assert_eq!(images.len(), 2);
            assert_eq!(images[0].path, "/home/user/pictures/photo1.jpg");
            assert_eq!(images[0].name, Some("Photo 1".to_string()));
            assert_eq!(images[1].path, "/home/user/pictures/photo2.png");
            assert!(images[1].name.is_none());
        }
        _ => panic!("Expected ImageBrowser response"),
    }
}

#[test]
fn test_parse_image_browser_nested_format() {
    // QML hamr format with nested imageBrowser object
    let json = r#"{
        "type": "imageBrowser",
        "imageBrowser": {
            "directory": "/tmp/screenshots",
            "images": [
                {"path": "/tmp/screenshots/screen1.png", "id": "s1"}
            ]
        }
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::ImageBrowser { image_browser, .. } => {
            let inner = image_browser.unwrap();
            assert_eq!(inner.directory, Some("/tmp/screenshots".to_string()));
            assert_eq!(inner.images.len(), 1);
            assert_eq!(inner.images[0].id, Some("s1".to_string()));
        }
        _ => panic!("Expected ImageBrowser response"),
    }
}

#[test]
fn test_parse_grid_browser_response() {
    let json = r#"{
        "type": "gridBrowser",
        "title": "Select Emoji",
        "columns": 8,
        "items": [
            {"id": "smile", "name": "Smile", "icon": "sentiment_satisfied"},
            {"id": "heart", "name": "Heart", "icon": "favorite"}
        ],
        "actions": [
            {"id": "copy", "name": "Copy"}
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::GridBrowser {
            items,
            title,
            columns,
            actions,
        } => {
            assert_eq!(title, Some("Select Emoji".to_string()));
            assert_eq!(columns, Some(8));
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].id, "smile");
            assert_eq!(items[0].icon, Some("sentiment_satisfied".to_string()));
            assert_eq!(actions.len(), 1);
        }
        _ => panic!("Expected GridBrowser response"),
    }
}

#[test]
fn test_parse_grid_browser_with_thumbnails() {
    let json = r#"{
        "type": "gridBrowser",
        "items": [
            {"id": "img1", "name": "Image 1", "thumbnail": "/path/to/thumb1.jpg"},
            {"id": "img2", "name": "Image 2", "thumbnail": "/path/to/thumb2.jpg", "description": "A nice photo"}
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::GridBrowser { items, .. } => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].thumbnail, Some("/path/to/thumb1.jpg".to_string()));
            assert!(items[0].description.is_none());
            assert_eq!(items[1].description, Some("A nice photo".to_string()));
        }
        _ => panic!("Expected GridBrowser response"),
    }
}

#[test]
fn test_parse_prompt_response() {
    let json = r#"{
        "type": "prompt",
        "prompt": {
            "text": "Enter your name:",
            "placeholder": "John Doe"
        }
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Prompt { prompt } => {
            assert_eq!(prompt.text, "Enter your name:");
            assert_eq!(prompt.placeholder, Some("John Doe".to_string()));
        }
        _ => panic!("Expected Prompt response"),
    }
}

#[test]
fn test_parse_prompt_response_minimal() {
    let json = r#"{
        "type": "prompt",
        "prompt": {
            "text": "Confirm action?"
        }
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Prompt { prompt } => {
            assert_eq!(prompt.text, "Confirm action?");
            assert!(prompt.placeholder.is_none());
        }
        _ => panic!("Expected Prompt response"),
    }
}

#[test]
fn test_parse_match_response_with_result() {
    let json = r#"{
        "type": "match",
        "result": {
            "id": "calc-result",
            "name": "= 42",
            "description": "2 + 40",
            "icon": "calculate",
            "verb": "Copy"
        }
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Match { result } => {
            let result = result.unwrap();
            assert_eq!(result.id, "calc-result");
            assert_eq!(result.name, "= 42");
            assert_eq!(result.description, Some("2 + 40".to_string()));
            assert_eq!(result.verb, Some("Copy".to_string()));
        }
        _ => panic!("Expected Match response"),
    }
}

#[test]
fn test_parse_match_response_no_result() {
    let json = r#"{
        "type": "match",
        "result": null
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Match { result } => {
            assert!(result.is_none(), "No match should return None");
        }
        _ => panic!("Expected Match response"),
    }
}

#[test]
fn test_parse_noop_response() {
    let json = r#"{"type": "noop"}"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Noop => {
            // Success - noop parsed correctly
        }
        _ => panic!("Expected Noop response"),
    }
}

#[test]
fn test_parse_card_with_blocks() {
    let json = r#"{
        "type": "card",
        "card": {
            "title": "Chat History",
            "kind": "blocks",
            "blocks": [
                {"type": "pill", "text": "Today"},
                {"type": "message", "role": "user", "content": "Hello!"},
                {"type": "message", "role": "assistant", "content": "Hi there!"},
                {"type": "separator"},
                {"type": "note", "content": "End of conversation"}
            ]
        }
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Card { card, .. } => {
            assert_eq!(card.title, "Chat History");
            assert_eq!(card.kind, Some("blocks".to_string()));
            assert_eq!(card.blocks.len(), 5);

            // Pill block
            match &card.blocks[0] {
                crate::plugin::CardBlockData::Pill { text } => {
                    assert_eq!(text, "Today");
                }
                _ => panic!("Expected Pill block"),
            }

            // User message
            match &card.blocks[1] {
                crate::plugin::CardBlockData::Message { role, content } => {
                    assert_eq!(role, "user");
                    assert_eq!(content, "Hello!");
                }
                _ => panic!("Expected Message block"),
            }

            // Assistant message
            match &card.blocks[2] {
                crate::plugin::CardBlockData::Message { role, content } => {
                    assert_eq!(role, "assistant");
                    assert_eq!(content, "Hi there!");
                }
                _ => panic!("Expected Message block"),
            }

            // Separator
            match &card.blocks[3] {
                crate::plugin::CardBlockData::Separator => {}
                _ => panic!("Expected Separator block"),
            }

            // Note
            match &card.blocks[4] {
                crate::plugin::CardBlockData::Note { content } => {
                    assert_eq!(content, "End of conversation");
                }
                _ => panic!("Expected Note block"),
            }
        }
        _ => panic!("Expected Card response"),
    }
}

#[test]
fn test_parse_progress_as_number() {
    let json = r#"{
        "type": "results",
        "items": [
            {"id": "task1", "name": "Download", "progress": 75}
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Results { items, .. } => {
            assert!(
                matches!(
                    items[0].widget,
                    Some(WidgetData::Progress { value, max, .. })
                    if value == 75.0 && max == 100.0
                ),
                "widget should contain Progress data"
            );
        }
        _ => panic!("Expected Results response"),
    }
}

#[test]
fn test_parse_progress_as_object() {
    let json = r#"{
        "type": "results",
        "items": [
            {
                "id": "task1",
                "name": "Upload",
                "progress": {
                    "value": 50,
                    "max": 200,
                    "label": "50/200 MB",
                    "color": "blue"
                }
            }
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Results { items, .. } => match &items[0].widget {
            Some(WidgetData::Progress {
                value,
                max,
                label,
                color,
            }) => {
                assert_eq!(*value, 50.0);
                assert_eq!(*max, 200.0);
                assert_eq!(*label, Some("50/200 MB".to_string()));
                assert_eq!(*color, Some("blue".to_string()));
            }
            _ => panic!("Expected Progress widget"),
        },
        _ => panic!("Expected Results response"),
    }
}

#[test]
fn test_parse_status_with_ambient() {
    let json = r#"{
        "type": "status",
        "status": {
            "ambient": [
                {
                    "id": "timer1",
                    "name": "Timer",
                    "description": "5:00 remaining",
                    "icon": "timer",
                    "duration": 300000,
                    "actions": [{"id": "stop", "name": "Stop"}]
                }
            ]
        }
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Status { status } => {
            let ambient = status.ambient.unwrap();
            assert_eq!(ambient.len(), 1);
            assert_eq!(ambient[0].id, "timer1");
            assert_eq!(ambient[0].duration, 300_000);
            assert_eq!(ambient[0].actions.len(), 1);
        }
        _ => panic!("Expected Status response"),
    }
}

#[test]
fn test_parse_status_ambient_null_clears() {
    let json = r#"{
        "type": "status",
        "status": {
            "ambient": null
        }
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Status { status } => {
            // null should result in Some(vec![]) to signal "clear all ambient items"
            let ambient = status.ambient.unwrap();
            assert!(ambient.is_empty(), "null should clear ambient items");
        }
        _ => panic!("Expected Status response"),
    }
}

#[test]
fn test_parse_status_with_fab() {
    let json = r#"{
        "type": "status",
        "status": {
            "fab": {
                "badges": [{"text": "3"}],
                "priority": 10,
                "showFab": true
            }
        }
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Status { status } => {
            let fab = status.fab.unwrap();
            assert_eq!(fab.badges.len(), 1);
            assert_eq!(fab.priority, 10);
            assert!(fab.show_fab);
        }
        _ => panic!("Expected Status response"),
    }
}

#[test]
fn test_parse_results_navigation_control() {
    let json = r#"{
        "type": "results",
        "items": [],
        "navigateForward": true,
        "clearInput": true,
        "placeholder": "Search notes..."
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Results {
            navigate_forward,
            clear_input,
            placeholder,
            ..
        } => {
            assert_eq!(navigate_forward, Some(true));
            assert!(clear_input);
            assert_eq!(placeholder, Some("Search notes...".to_string()));
        }
        _ => panic!("Expected Results response"),
    }
}

#[test]
fn test_parse_results_navigate_back() {
    let json = r#"{
        "type": "results",
        "items": [],
        "navigateForward": false
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Results {
            navigate_forward, ..
        } => {
            assert_eq!(navigate_forward, Some(false), "Should signal navigate back");
        }
        _ => panic!("Expected Results response"),
    }
}

#[test]
fn test_parse_execute_type_text() {
    let json = r#"{
        "type": "execute",
        "typeText": "Hello, World!",
        "close": true
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Execute(data) => {
            assert_eq!(data.type_text, Some("Hello, World!".to_string()));
            assert_eq!(data.close, Some(true));
        }
        _ => panic!("Expected Execute response"),
    }
}

#[test]
fn test_parse_execute_sound() {
    let json = r#"{
        "type": "execute",
        "sound": "notification"
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Execute(data) => {
            assert_eq!(data.sound, Some("notification".to_string()));
        }
        _ => panic!("Expected Execute response"),
    }
}

#[test]
fn test_parse_execute_keep_open() {
    let json = r#"{
        "type": "execute",
        "copy": "test",
        "keepOpen": true
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Execute(data) => {
            assert!(data.keep_open);
            assert!(data.close.is_none());
        }
        _ => panic!("Expected Execute response"),
    }
}

#[test]
fn test_parse_form_all_field_types() {
    let json = r#"{
        "type": "form",
        "form": {
            "title": "Settings",
            "fields": [
                {"id": "name", "label": "Name", "type": "text"},
                {"id": "password", "label": "Password", "type": "password"},
                {"id": "age", "label": "Age", "type": "number"},
                {"id": "bio", "label": "Bio", "type": "textarea", "rows": 4},
                {"id": "country", "label": "Country", "type": "select", "options": [
                    {"value": "us", "label": "United States"},
                    {"value": "uk", "label": "United Kingdom"}
                ]},
                {"id": "agree", "label": "I agree", "type": "checkbox"},
                {"id": "notifications", "label": "Notifications", "type": "switch"},
                {"id": "volume", "label": "Volume", "type": "slider", "min": 0, "max": 100, "step": 5},
                {"id": "token", "label": "Token", "type": "hidden", "default": "secret123"},
                {"id": "dob", "label": "Date of Birth", "type": "date"},
                {"id": "reminder", "label": "Reminder", "type": "time"}
            ],
            "submitLabel": "Save",
            "cancelLabel": "Cancel",
            "liveUpdate": true
        }
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Form { form, .. } => {
            assert_eq!(form.fields.len(), 11);
            assert!(form.live_update);

            // Text field
            assert_eq!(form.fields[0].field_type, Some("text".to_string()));

            // Password field
            assert_eq!(form.fields[1].field_type, Some("password".to_string()));

            // Number field
            assert_eq!(form.fields[2].field_type, Some("number".to_string()));

            // Textarea with rows
            assert_eq!(form.fields[3].field_type, Some("textarea".to_string()));
            assert_eq!(form.fields[3].rows, Some(4));

            // Select with options
            assert_eq!(form.fields[4].field_type, Some("select".to_string()));
            assert_eq!(form.fields[4].options.len(), 2);
            assert_eq!(form.fields[4].options[0].value, "us");

            // Checkbox
            assert_eq!(form.fields[5].field_type, Some("checkbox".to_string()));

            // Switch
            assert_eq!(form.fields[6].field_type, Some("switch".to_string()));

            // Slider with min/max/step
            assert_eq!(form.fields[7].field_type, Some("slider".to_string()));
            assert_eq!(form.fields[7].min, Some(0.0));
            assert_eq!(form.fields[7].max, Some(100.0));
            assert_eq!(form.fields[7].step, Some(5.0));

            // Hidden with default
            assert_eq!(form.fields[8].field_type, Some("hidden".to_string()));
            assert_eq!(form.fields[8].default_value, Some("secret123".to_string()));

            // Date
            assert_eq!(form.fields[9].field_type, Some("date".to_string()));

            // Time
            assert_eq!(form.fields[10].field_type, Some("time".to_string()));
        }
        _ => panic!("Expected Form response"),
    }
}

#[test]
fn test_parse_results_with_preview() {
    let json = r#"{
        "type": "results",
        "items": [
            {
                "id": "note1",
                "name": "My Note",
                "preview": {
                    "title": "My Note",
                    "markdown": "Hello **bold** text.",
                    "metadata": [
                        {"label": "Created", "value": "2024-01-01"},
                        {"label": "Tags", "value": "work"}
                    ],
                    "actions": [
                        {"id": "edit", "name": "Edit"},
                        {"id": "delete", "name": "Delete"}
                    ]
                }
            }
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Results { items, .. } => {
            let preview = items[0].preview.as_ref().unwrap();
            assert_eq!(preview.title, Some("My Note".to_string()));
            assert!(preview.markdown.as_ref().unwrap().contains("**bold**"));
            assert_eq!(preview.metadata.len(), 2);
            assert_eq!(preview.actions.len(), 2);
        }
        _ => panic!("Expected Results response"),
    }
}

#[test]
fn test_parse_index_with_app_id() {
    let json = r#"{
        "type": "index",
        "items": [
            {
                "id": "firefox",
                "name": "Firefox",
                "appId": "firefox.desktop",
                "entryPoint": {
                    "step": "action",
                    "selected": {"id": "firefox"}
                }
            }
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Index { items, .. } => {
            assert_eq!(items[0].app_id, Some("firefox.desktop".to_string()));
            assert!(items[0].entry_point.is_some());
        }
        _ => panic!("Expected Index response"),
    }
}

#[test]
fn test_parse_index_with_keep_open() {
    let json = r#"{
        "type": "index",
        "items": [
            {"id": "settings", "name": "Settings", "keepOpen": true}
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Index { items, .. } => {
            assert!(items[0].keep_open);
        }
        _ => panic!("Expected Index response"),
    }
}

/// Test that Results response items use `hamr_types::ResultItem` directly
/// This ensures we can access `ResultItem` fields without conversion
#[test]
fn test_results_use_hamr_types_result_item() {
    let json = r#"{
        "type": "results",
        "items": [
            {
                "id": "volume",
                "name": "Volume",
                "type": "slider",
                "value": 75.0,
                "min": 0.0,
                "max": 100.0,
                "step": 1.0
            }
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Results { items, .. } => {
            // These fields should be directly accessible from hamr_types::ResultItem
            assert_eq!(items[0].id, "volume");
            assert_eq!(items[0].name, "Volume");
            assert_eq!(items[0].result_type, hamr_types::ResultType::Slider);
            // Widget field should contain Slider data
            assert!(
                matches!(
                    items[0].widget,
                    Some(WidgetData::Slider { value, min, max, step, .. })
                    if value == 75.0 && min == 0.0 && max == 100.0 && step == 1.0
                ),
                "widget should contain Slider data"
            );
        }
        _ => panic!("Expected Results response"),
    }
}

/// Test that Match response uses `hamr_types::ResultItem`
#[test]
fn test_match_uses_hamr_types_result_item() {
    let json = r#"{
        "type": "match",
        "result": {
            "id": "calc",
            "name": "= 42",
            "type": "normal",
            "badges": [{"text": "math"}]
        }
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Match { result } => {
            let item = result.unwrap();
            assert_eq!(item.id, "calc");
            assert_eq!(item.result_type, hamr_types::ResultType::Normal);
            assert_eq!(item.badges.len(), 1);
        }
        _ => panic!("Expected Match response"),
    }
}

/// Test backward compat: "resultType" alias works for `hamr_types::ResultItem`
#[test]
fn test_result_type_alias_backward_compat() {
    let json = r#"{
        "type": "results",
        "items": [
            {"id": "switch1", "name": "WiFi", "resultType": "switch", "value": true}
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Results { items, .. } => {
            assert_eq!(items[0].result_type, hamr_types::ResultType::Switch);
            // Widget field should contain Switch with value = true
            assert!(
                matches!(items[0].widget, Some(WidgetData::Switch { value: true })),
                "widget should contain Switch with value = true"
            );
        }
        _ => panic!("Expected Results response"),
    }
}

/// Test that actions are `hamr_types::Action` directly (no conversion needed)
#[test]
fn test_actions_use_hamr_types_action() {
    let json = r#"{
        "type": "results",
        "items": [
            {
                "id": "item1",
                "name": "Item",
                "actions": [
                    {"id": "copy", "name": "Copy", "icon": "content_copy", "keepOpen": true}
                ]
            }
        ]
    }"#;

    let response: PluginResponse = serde_json::from_str(json).unwrap();

    match response {
        PluginResponse::Results { items, .. } => {
            let action = &items[0].actions[0];
            assert_eq!(action.id, "copy");
            assert_eq!(action.name, "Copy");
            assert_eq!(action.icon, Some("content_copy".to_string()));
            assert!(action.keep_open);
        }
        _ => panic!("Expected Results response"),
    }
}
