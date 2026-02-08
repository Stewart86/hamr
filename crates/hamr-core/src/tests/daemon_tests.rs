//! Tests for daemon plugin response processing
//!
//! These tests verify the `process_plugin_response` function correctly converts
//! plugin responses to `CoreUpdate`s, which is the foundation of the daemon plugin
//! core integration feature.

use crate::engine::process::process_plugin_response;
use crate::plugin::{IndexItem, PluginResponse};
use hamr_types::CoreUpdate;

#[test]
fn test_process_plugin_response_results() {
    let response = PluginResponse::Results {
        items: vec![IndexItem {
            id: "test-item".to_string(),
            name: "Test Item".to_string(),
            verb: Some("Open".to_string()),
            ..Default::default()
        }],
        prepend: false,
        input_mode: None,
        status: None,
        context: None,
        placeholder: None,
        clear_input: false,
        navigate_forward: None,
        plugin_actions: vec![],
        navigation_depth: None,
        display_hint: None,
        activate: false,
    };

    let updates = process_plugin_response("test-plugin", response);

    assert_eq!(updates.len(), 2); // Busy + Results
    assert!(matches!(updates[0], CoreUpdate::Busy { busy: false }));
    assert!(matches!(&updates[1], CoreUpdate::Results { results, .. } if results.len() == 1));

    if let CoreUpdate::Results { results, .. } = &updates[1] {
        assert_eq!(results[0].id, "test-item");
    }
}

#[test]
fn test_process_plugin_response_error() {
    let response = PluginResponse::Error {
        message: "Something went wrong".to_string(),
        details: Some("Additional details".to_string()),
    };

    let updates = process_plugin_response("test-plugin", response);

    assert_eq!(updates.len(), 2); // Busy + Error
    assert!(matches!(updates[0], CoreUpdate::Busy { busy: false }));
    assert!(matches!(&updates[1], CoreUpdate::Error { .. }));
}

#[test]
fn test_process_plugin_response_index() {
    let response = PluginResponse::Index {
        items: vec![IndexItem {
            id: "item1".to_string(),
            name: "Item One".to_string(),
            verb: Some("Open".to_string()),
            ..Default::default()
        }],
        mode: None,
        remove: None,
        status: None,
    };

    let updates = process_plugin_response("test-plugin", response);

    assert_eq!(updates.len(), 2); // Busy + IndexUpdate
    assert!(matches!(updates[0], CoreUpdate::Busy { busy: false }));
    assert!(
        matches!(&updates[1], CoreUpdate::IndexUpdate { plugin_id, .. } if plugin_id == "test-plugin")
    );
}

#[test]
fn test_process_plugin_response_update() {
    use crate::plugin::UpdateItem;

    let response = PluginResponse::Update {
        items: Some(vec![UpdateItem {
            id: "item1".to_string(),
            fields: serde_json::json!({"name": "Updated Name"}),
        }]),
        status: None,
    };

    let updates = process_plugin_response("test-plugin", response);

    assert_eq!(updates.len(), 2); // Busy + ResultsUpdate
    assert!(matches!(updates[0], CoreUpdate::Busy { busy: false }));
    assert!(matches!(&updates[1], CoreUpdate::ResultsUpdate { .. }));
}

#[test]
fn test_process_plugin_response_results_conversion() {
    // This test verifies the Results response type is correctly processed,
    // which is the key path for daemon plugin frecency tracking.
    let response = PluginResponse::Results {
        items: vec![IndexItem {
            id: "volume".to_string(),
            name: "Volume".to_string(),
            description: Some("System volume".to_string()),
            verb: Some("Adjust".to_string()),
            icon: Some("audio-volume-high".to_string()),
            ..Default::default()
        }],
        prepend: false,
        input_mode: None,
        status: None,
        context: None,
        placeholder: None,
        clear_input: false,
        navigate_forward: None,
        plugin_actions: vec![],
        navigation_depth: None,
        display_hint: None,
        activate: false,
    };

    let updates = process_plugin_response("sound", response);

    assert_eq!(updates.len(), 2);

    if let CoreUpdate::Results { results, .. } = &updates[1] {
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result.id, "volume");
        assert_eq!(result.name, "Volume");
        assert_eq!(result.description, Some("System volume".to_string()));
        assert_eq!(result.verb, Some("Adjust".to_string()));
        assert_eq!(result.icon, Some("audio-volume-high".to_string()));
    } else {
        panic!("Expected Results update");
    }
}
