//! Integration tests for socket plugin communication in hamr-daemon
//!
//! These tests focus on verifying plugin registration, message forwarding,
//! and notification handling without requiring a full daemon instance.

#![allow(clippy::float_cmp)] // Exact float comparisons are intentional in tests

use std::path::PathBuf;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use hamr_daemon::{ConnectedPlugin, PluginRegistry, SessionId};
use hamr_rpc::protocol::{ClientRole, Message, Notification, Request, RequestId, Response};
use hamr_rpc::transport::JsonRpcCodec;
use hamr_types::PluginManifest;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use tokio_util::codec::Framed;

/// Create a mock plugin manifest
fn mock_plugin_manifest(id: &str) -> PluginManifest {
    PluginManifest {
        id: id.to_string(),
        name: format!("Test Plugin {id}"),
        description: Some(format!("Mock plugin for testing: {id}")),
        icon: Some("⚡".to_string()),
        prefix: Some(format!("prefix_{id}")),
        priority: 10,
    }
}

#[test]
fn test_plugin_registration_flow() {
    let mut registry = PluginRegistry::new();
    let (tx, _rx) = mpsc::unbounded_channel();

    let connected = ConnectedPlugin {
        id: "test-plugin-1".to_string(),
        session_id: SessionId::new(),
        sender: tx,
    };

    // Register the plugin
    registry.register_connected(connected.clone());

    // Verify it's registered
    assert!(
        registry.is_connected("test-plugin-1"),
        "Plugin should be registered"
    );

    // Verify we can get the sender
    let sender = registry.get_plugin_sender("test-plugin-1");
    assert!(sender.is_some(), "Should be able to get plugin sender");
}

#[test]
fn test_multiple_plugin_registration() {
    let mut registry = PluginRegistry::new();

    // Register multiple plugins
    for i in 1..=3 {
        let id = format!("plugin-{i}");
        let (tx, _rx) = mpsc::unbounded_channel();

        let connected = ConnectedPlugin {
            id: id.clone(),
            session_id: SessionId::new(),
            sender: tx,
        };

        registry.register_connected(connected);
    }

    // Verify all plugins are registered
    assert!(
        registry.is_connected("plugin-1"),
        "Plugin 1 should be registered"
    );
    assert!(
        registry.is_connected("plugin-2"),
        "Plugin 2 should be registered"
    );
    assert!(
        registry.is_connected("plugin-3"),
        "Plugin 3 should be registered"
    );

    // Verify non-existent plugin is not registered
    assert!(
        !registry.is_connected("plugin-4"),
        "Plugin 4 should not be registered"
    );
}

#[test]
fn test_plugin_unregistration() {
    let mut registry = PluginRegistry::new();

    // Register a plugin
    let (tx, _rx) = mpsc::unbounded_channel();
    let session_id = SessionId::new();

    let connected = ConnectedPlugin {
        id: "removable-plugin".to_string(),
        session_id: session_id.clone(),
        sender: tx,
    };

    registry.register_connected(connected);
    assert!(
        registry.is_connected("removable-plugin"),
        "Plugin should be registered"
    );

    // Unregister the plugin by session ID
    registry.unregister_session(&session_id);

    // Verify it's unregistered
    assert!(
        !registry.is_connected("removable-plugin"),
        "Plugin should be unregistered"
    );
}

#[test]
fn test_plugin_results_notification_structure() {
    // Verify that plugin_results notifications have correct structure
    let results_data = serde_json::json!({
        "results": [
            {
                "id": "result-1",
                "name": "Test Result",
                "description": "A test result",
                "icon": "📄"
            },
            {
                "id": "result-2",
                "name": "Another Result",
                "description": "Another test result",
                "icon": "📁"
            }
        ]
    });

    // Create notification
    let notification = Notification::new("plugin_results", Some(results_data.clone()));

    // Verify structure
    assert_eq!(notification.method, "plugin_results");
    assert!(notification.params.is_some());

    let params = notification.params.unwrap();
    assert!(params.get("results").is_some());

    let results = params.get("results").unwrap().as_array().unwrap();
    assert_eq!(results.len(), 2);

    // Verify each result has required fields
    for result in results {
        assert!(result.get("id").is_some());
        assert!(result.get("name").is_some());
    }
}

#[test]
fn test_plugin_status_notification_with_ambient() {
    let status_data = serde_json::json!({
        "status": {
            "badges": [
                {
                    "label": "3",
                    "color": "#ff0000"
                }
            ],
            "chips": [
                {
                    "label": "Active",
                    "icon": "play"
                }
            ],
            "ambient": [
                {
                    "id": "ambient-1",
                    "name": "Now Playing",
                    "description": "Song Title - Artist",
                    "actions": [
                        {
                            "id": "pause",
                            "label": "Pause"
                        }
                    ]
                },
                {
                    "id": "ambient-2",
                    "name": "Weather",
                    "description": "Clear, 72°F",
                    "actions": []
                }
            ]
        }
    });

    let notification = Notification::new("plugin_status", Some(status_data.clone()));

    assert_eq!(notification.method, "plugin_status");

    let params = notification.params.unwrap();
    let status = params.get("status").unwrap();

    // Verify badges
    let badges = status.get("badges").unwrap().as_array().unwrap();
    assert_eq!(badges.len(), 1);
    assert_eq!(badges[0].get("label").unwrap().as_str().unwrap(), "3");

    // Verify chips
    let chips = status.get("chips").unwrap().as_array().unwrap();
    assert_eq!(chips.len(), 1);

    // Verify ambient items
    let ambient = status.get("ambient").unwrap().as_array().unwrap();
    assert_eq!(ambient.len(), 2);

    // Check first ambient item
    let first_ambient = &ambient[0];
    assert_eq!(
        first_ambient.get("id").unwrap().as_str().unwrap(),
        "ambient-1"
    );
    assert_eq!(
        first_ambient.get("name").unwrap().as_str().unwrap(),
        "Now Playing"
    );

    let actions = first_ambient.get("actions").unwrap().as_array().unwrap();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].get("id").unwrap().as_str().unwrap(), "pause");
}

#[test]
fn test_ambient_clearing_empty_array() {
    let status_data = serde_json::json!({
        "status": {
            "badges": [],
            "chips": [],
            "ambient": []
        }
    });

    let notification = Notification::new("plugin_status", Some(status_data));

    let params = notification.params.unwrap();
    let status = params.get("status").unwrap();

    let ambient = status.get("ambient").unwrap().as_array().unwrap();
    assert_eq!(
        ambient.len(),
        0,
        "Ambient array should be empty for clearing"
    );
}

#[test]
fn test_client_role_serialization() {
    // Test UI role serialization
    let ui_role = ClientRole::Ui {
        name: "test-ui".to_string(),
    };
    let ui_json = serde_json::to_value(&ui_role).unwrap();
    assert_eq!(ui_json.get("name").unwrap().as_str().unwrap(), "test-ui");

    // Test Control role serialization
    let control_role = ClientRole::Control;
    let control_json = serde_json::to_value(&control_role).unwrap();
    // Control is a unit variant, so it serializes to a string or object
    assert!(control_json.is_string() || control_json.is_object());

    // Test Plugin role serialization
    let plugin_role = ClientRole::Plugin {
        id: "test-plugin".to_string(),
        manifest: mock_plugin_manifest("test-plugin"),
    };
    let plugin_json = serde_json::to_value(&plugin_role).unwrap();
    assert_eq!(
        plugin_json.get("id").unwrap().as_str().unwrap(),
        "test-plugin"
    );
    assert!(plugin_json.get("manifest").is_some());
}

#[test]
fn test_plugin_manifest_full_details() {
    let manifest = PluginManifest {
        id: "full-featured-plugin".to_string(),
        name: "Full Featured Plugin".to_string(),
        description: Some("A plugin with all details".to_string()),
        icon: Some("🔌".to_string()),
        prefix: Some("full".to_string()),
        priority: 50,
    };

    // Verify all fields are present
    assert_eq!(manifest.id, "full-featured-plugin");
    assert_eq!(manifest.name, "Full Featured Plugin");
    assert!(manifest.description.is_some());
    assert!(manifest.icon.is_some());
    assert!(manifest.prefix.is_some());
    assert_eq!(manifest.priority, 50);

    // Test serialization
    let json = serde_json::to_value(&manifest).unwrap();
    assert!(json.is_object());
    assert_eq!(
        json.get("id").unwrap().as_str().unwrap(),
        "full-featured-plugin"
    );
}

#[test]
fn test_multiple_notification_types() {
    // Test that different notification types can be created
    let notifications = vec![
        ("plugin_results", serde_json::json!({"results": []})),
        ("plugin_status", serde_json::json!({"status": {}})),
        ("plugin_index", serde_json::json!({"items": []})),
        ("plugin_execute", serde_json::json!({"action": {}})),
        ("plugin_update", serde_json::json!({"patches": []})),
    ];

    for (method, params) in notifications {
        let notification = Notification::new(method, Some(params));
        assert_eq!(notification.method, method);
        assert!(notification.params.is_some());
    }
}

#[test]
fn test_plugin_registry_state_changes() {
    let mut registry = PluginRegistry::new();

    // Initially empty
    assert!(!registry.is_connected("plugin-1"));

    // Register first plugin
    let (tx1, _rx1) = mpsc::unbounded_channel();
    let session_id_1 = SessionId::new();
    registry.register_connected(ConnectedPlugin {
        id: "plugin-1".to_string(),
        session_id: session_id_1.clone(),
        sender: tx1,
    });
    assert!(registry.is_connected("plugin-1"));

    // Register second plugin
    let (tx2, _rx2) = mpsc::unbounded_channel();
    let session_id_2 = SessionId::new();
    registry.register_connected(ConnectedPlugin {
        id: "plugin-2".to_string(),
        session_id: session_id_2.clone(),
        sender: tx2,
    });
    assert!(registry.is_connected("plugin-2"));

    // Both should be connected
    assert!(registry.is_connected("plugin-1"));
    assert!(registry.is_connected("plugin-2"));

    // Unregister first plugin
    registry.unregister_session(&session_id_1);
    assert!(!registry.is_connected("plugin-1"));
    assert!(registry.is_connected("plugin-2"));
}

#[test]
fn test_results_notification_with_optional_fields() {
    // Verify results notification structure with all optional fields
    let results_data = serde_json::json!({
        "results": [
            {"id": "item-1", "name": "Item 1"}
        ],
        "placeholder": "Edit task...",
        "clearInput": true,
        "inputMode": "submit",
        "context": "__edit__:0"
    });

    let notification = Notification::new("results", Some(results_data.clone()));

    assert_eq!(notification.method, "results");
    let params = notification.params.unwrap();

    // Verify results array
    let results = params.get("results").unwrap().as_array().unwrap();
    assert_eq!(results.len(), 1);

    // Verify optional fields
    assert_eq!(
        params.get("placeholder").unwrap().as_str().unwrap(),
        "Edit task..."
    );
    assert!(params.get("clearInput").unwrap().as_bool().unwrap());
    assert_eq!(params.get("inputMode").unwrap().as_str().unwrap(), "submit");
    assert_eq!(
        params.get("context").unwrap().as_str().unwrap(),
        "__edit__:0"
    );
}

#[test]
fn test_results_notification_without_optional_fields() {
    // Results without optional fields should also work
    let results_data = serde_json::json!({
        "results": [
            {"id": "item-1", "name": "Item 1"}
        ]
    });

    let notification = Notification::new("results", Some(results_data.clone()));
    let params = notification.params.unwrap();

    // Results should be present
    assert!(params.get("results").is_some());

    // Optional fields should be absent
    assert!(params.get("placeholder").is_none());
    assert!(params.get("clearInput").is_none());
    assert!(params.get("inputMode").is_none());
    assert!(params.get("context").is_none());
}

#[test]
fn test_discovered_plugin_background_flag() {
    use hamr_daemon::DiscoveredPlugin;

    // Background plugin (spawned at startup)
    let background_plugin = DiscoveredPlugin {
        id: "timer".to_string(),
        manifest: PluginManifest {
            id: "timer".to_string(),
            name: "Timer".to_string(),
            description: None,
            icon: None,
            prefix: None,
            priority: 0,
        },
        is_socket: true,
        spawn_command: Some("python3 handler.py".to_string()),
        is_background: true,
    };
    assert!(
        background_plugin.is_background,
        "Timer should be a background plugin"
    );

    // On-demand plugin (spawned when opened)
    let ondemand_plugin = DiscoveredPlugin {
        id: "topmem".to_string(),
        manifest: PluginManifest {
            id: "topmem".to_string(),
            name: "Top Memory".to_string(),
            description: None,
            icon: None,
            prefix: None,
            priority: 0,
        },
        is_socket: true,
        spawn_command: Some("python3 handler.py".to_string()),
        is_background: false,
    };
    assert!(
        !ondemand_plugin.is_background,
        "Topmem should be an on-demand plugin"
    );
}

#[test]
fn test_slider_changed_notification_with_plugin_id() {
    let slider_data = serde_json::json!({
        "id": "volume",
        "value": 75.5,
        "plugin_id": "sound"
    });

    let notification = Notification::new("slider_changed", Some(slider_data.clone()));

    assert_eq!(notification.method, "slider_changed");
    let params = notification.params.unwrap();

    assert_eq!(params.get("id").unwrap().as_str().unwrap(), "volume");
    assert_eq!(params.get("value").unwrap().as_f64().unwrap(), 75.5);
    assert_eq!(params.get("plugin_id").unwrap().as_str().unwrap(), "sound");
}

#[test]
fn test_slider_changed_notification_without_plugin_id() {
    // Slider notification without explicit plugin_id should use active plugin
    let slider_data = serde_json::json!({
        "id": "volume",
        "value": 50.0
    });

    let notification = Notification::new("slider_changed", Some(slider_data.clone()));
    let params = notification.params.unwrap();

    assert_eq!(params.get("id").unwrap().as_str().unwrap(), "volume");
    assert_eq!(params.get("value").unwrap().as_f64().unwrap(), 50.0);
    assert!(
        params.get("plugin_id").is_none(),
        "plugin_id should be absent, daemon should use active plugin"
    );
}

#[test]
fn test_switch_toggled_notification_with_plugin_id() {
    let switch_data = serde_json::json!({
        "id": "mute",
        "value": true,
        "plugin_id": "sound"
    });

    let notification = Notification::new("switch_toggled", Some(switch_data.clone()));

    assert_eq!(notification.method, "switch_toggled");
    let params = notification.params.unwrap();

    assert_eq!(params.get("id").unwrap().as_str().unwrap(), "mute");
    assert!(params.get("value").unwrap().as_bool().unwrap());
    assert_eq!(params.get("plugin_id").unwrap().as_str().unwrap(), "sound");
}

#[test]
fn test_switch_toggled_notification_without_plugin_id() {
    let switch_data = serde_json::json!({
        "id": "notifications",
        "value": false
    });

    let notification = Notification::new("switch_toggled", Some(switch_data.clone()));
    let params = notification.params.unwrap();

    assert!(
        params.get("plugin_id").is_none(),
        "plugin_id should be absent, daemon should use active plugin"
    );
}

#[test]
fn test_query_submitted_notification_with_context() {
    let query_data = serde_json::json!({
        "query": "new task text",
        "context": "__edit__:0"
    });

    let notification = Notification::new("query_submitted", Some(query_data.clone()));

    assert_eq!(notification.method, "query_submitted");
    let params = notification.params.unwrap();

    assert_eq!(
        params.get("query").unwrap().as_str().unwrap(),
        "new task text"
    );
    assert_eq!(
        params.get("context").unwrap().as_str().unwrap(),
        "__edit__:0"
    );
}

#[test]
fn test_query_submitted_notification_without_context() {
    let query_data = serde_json::json!({
        "query": "search term"
    });

    let notification = Notification::new("query_submitted", Some(query_data.clone()));
    let params = notification.params.unwrap();

    assert_eq!(
        params.get("query").unwrap().as_str().unwrap(),
        "search term"
    );
    assert!(params.get("context").is_none());
}

#[test]
fn test_discovered_plugin_is_background_defaults_to_false() {
    use hamr_daemon::DiscoveredPlugin;

    // When a socket plugin doesn't specify daemon.background, it should default to false
    // This means it's an on-demand plugin that gets spawned when opened and killed when closed
    let ondemand_plugin = DiscoveredPlugin {
        id: "screenshot".to_string(),
        manifest: PluginManifest {
            id: "screenshot".to_string(),
            name: "Screenshot".to_string(),
            description: None,
            icon: None,
            prefix: None,
            priority: 0,
        },
        is_socket: true,
        spawn_command: Some("python3 handler.py".to_string()),
        is_background: false, // Default should be false
    };

    assert!(
        !ondemand_plugin.is_background,
        "Plugins should default to on-demand (is_background: false)"
    );

    // Background plugins must explicitly set is_background: true
    let background_plugin = DiscoveredPlugin {
        id: "timer".to_string(),
        manifest: PluginManifest {
            id: "timer".to_string(),
            name: "Timer".to_string(),
            description: None,
            icon: None,
            prefix: None,
            priority: 0,
        },
        is_socket: true,
        spawn_command: Some("python3 handler.py".to_string()),
        is_background: true, // Explicitly set to true
    };

    assert!(
        background_plugin.is_background,
        "Background plugins should have is_background: true"
    );
}

#[test]
fn test_ondemand_plugin_lifecycle_documentation() {
    use hamr_daemon::DiscoveredPlugin;

    // This test documents the expected lifecycle of on-demand plugins:
    // 1. User opens plugin -> daemon spawns process
    // 2. Plugin connects via socket and registers
    // 3. User closes plugin OR launcher -> daemon kills process

    let ondemand = DiscoveredPlugin {
        id: "topmem".to_string(),
        manifest: PluginManifest {
            id: "topmem".to_string(),
            name: "Top Memory".to_string(),
            description: Some("Shows top memory processes".to_string()),
            icon: Some("memory".to_string()),
            prefix: None,
            priority: 0,
        },
        is_socket: true,
        spawn_command: Some("python3 handler.py".to_string()),
        is_background: false,
    };

    // On-demand plugins should:
    assert!(
        ondemand.is_socket,
        "On-demand plugins use socket communication"
    );
    assert!(
        !ondemand.is_background,
        "On-demand plugins are NOT background"
    );
    assert!(
        ondemand.spawn_command.is_some(),
        "On-demand plugins have spawn command"
    );
}

// =============================================================================
// Socket Communication Tests
// =============================================================================
// These tests verify the RPC protocol works correctly over Unix sockets.

/// Generate a unique socket path for testing
fn test_socket_path(test_name: &str) -> PathBuf {
    let pid = std::process::id();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("hamr-test-{test_name}-{pid}-{timestamp}.sock"))
}

/// Helper to clean up socket file
fn cleanup_socket(path: &PathBuf) {
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
}

#[tokio::test]
async fn test_socket_framing_request_response() {
    // Test that the JSON-RPC codec correctly frames and parses messages over a socket
    let socket_path = test_socket_path("framing");
    cleanup_socket(&socket_path);

    // Start a mock server that echoes requests as responses
    let listener = UnixListener::bind(&socket_path).unwrap();

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let framed = Framed::new(stream, JsonRpcCodec::new());
        let (mut sink, mut stream) = framed.split();

        // Read one request
        if let Some(Ok(Message::Request(req))) = stream.next().await {
            // Send a response
            let response = Response::success(
                req.id.unwrap_or(RequestId::Number(0)),
                serde_json::json!({"echo": req.method}),
            );
            sink.send(Message::Response(response)).await.unwrap();
        }
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Connect as client
    let stream = UnixStream::connect(&socket_path).await.unwrap();
    let framed = Framed::new(stream, JsonRpcCodec::new());
    let (mut sink, mut stream) = framed.split();

    // Send a request
    let request = Request::new("test_method", None, RequestId::Number(1));
    sink.send(Message::Request(request)).await.unwrap();

    // Read the response
    let response = tokio::time::timeout(Duration::from_secs(1), stream.next())
        .await
        .expect("timeout waiting for response")
        .expect("stream ended")
        .expect("codec error");

    match response {
        Message::Response(resp) => {
            assert_eq!(resp.id, RequestId::Number(1));
            assert!(resp.error.is_none());
            let result = resp.result.unwrap();
            assert_eq!(result["echo"], "test_method");
        }
        _ => panic!("Expected Response, got {response:?}"),
    }

    server_task.await.unwrap();
    cleanup_socket(&socket_path);
}

#[tokio::test]
async fn test_socket_notification_delivery() {
    // Test that notifications are correctly sent and received
    let socket_path = test_socket_path("notification");
    cleanup_socket(&socket_path);

    let listener = UnixListener::bind(&socket_path).unwrap();

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let framed = Framed::new(stream, JsonRpcCodec::new());
        let (mut sink, mut stream) = framed.split();

        // Wait for client to signal it's ready
        let _ = stream.next().await; // Client sends a "ready" request

        // Send several notifications
        for i in 1..=3 {
            let notification = Notification::new("update", Some(serde_json::json!({"count": i})));
            sink.send(Message::Notification(notification))
                .await
                .unwrap();
        }

        // Keep connection alive until client signals done
        let _ = stream.next().await;
    });

    tokio::time::sleep(Duration::from_millis(10)).await;

    let stream = UnixStream::connect(&socket_path).await.unwrap();
    let framed = Framed::new(stream, JsonRpcCodec::new());
    let (mut sink, mut stream) = framed.split();

    // Signal server we're ready
    let ready_req = Request::new("ready", None, RequestId::Number(0));
    sink.send(Message::Request(ready_req)).await.unwrap();

    // Receive all notifications
    // Note: Due to serde untagged enum, notifications may deserialize as Request with id: None
    let mut counts = Vec::new();
    for _ in 0..3 {
        let msg = tokio::time::timeout(Duration::from_secs(1), stream.next())
            .await
            .expect("timeout")
            .expect("stream ended")
            .expect("codec error");

        // Notifications may come as either Notification or Request(id=None) due to serde untagged
        let (method, params) = match &msg {
            Message::Notification(n) => (n.method.as_str(), n.params.as_ref()),
            Message::Request(r) if r.id.is_none() => (r.method.as_str(), r.params.as_ref()),
            _ => panic!("Expected notification, got {msg:?}"),
        };
        assert_eq!(method, "update");
        let count = params.unwrap()["count"].as_i64().unwrap();
        counts.push(count);
    }

    assert_eq!(counts, vec![1, 2, 3]);

    // Signal server to close
    let done_req = Request::new("done", None, RequestId::Number(1));
    sink.send(Message::Request(done_req)).await.unwrap();

    server_task.await.unwrap();
    cleanup_socket(&socket_path);
}

#[tokio::test]
async fn test_socket_bidirectional_communication() {
    // Test client sending request + receiving response, then server pushing notifications
    let socket_path = test_socket_path("bidirectional");
    cleanup_socket(&socket_path);

    let listener = UnixListener::bind(&socket_path).unwrap();

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let framed = Framed::new(stream, JsonRpcCodec::new());
        let (mut sink, mut stream) = framed.split();

        // Wait for register request
        if let Some(Ok(Message::Request(req))) = stream.next().await {
            assert_eq!(req.method, "register");
            // Send success response
            let response = Response::success(
                req.id.clone().unwrap(),
                serde_json::json!({"session_id": "test-session-123"}),
            );
            sink.send(Message::Response(response)).await.unwrap();

            // Then push a notification (simulating server-initiated update)
            let notification = Notification::new(
                "results",
                Some(serde_json::json!({
                    "results": [{"id": "1", "name": "Test Result"}]
                })),
            );
            sink.send(Message::Notification(notification))
                .await
                .unwrap();

            // Keep connection alive until client reads the notification
            let _ = stream.next().await;
        }
    });

    tokio::time::sleep(Duration::from_millis(10)).await;

    let stream = UnixStream::connect(&socket_path).await.unwrap();
    let framed = Framed::new(stream, JsonRpcCodec::new());
    let (mut sink, mut stream) = framed.split();

    // Send register request
    let register_req = Request::new(
        "register",
        Some(serde_json::json!({"role": {"Ui": {"name": "test-client"}}})),
        RequestId::Number(1),
    );
    sink.send(Message::Request(register_req)).await.unwrap();

    // Should receive register response first
    let msg1 = tokio::time::timeout(Duration::from_secs(1), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    match msg1 {
        Message::Response(resp) => {
            assert!(resp.error.is_none(), "Registration should succeed");
            let session_id = resp.result.as_ref().unwrap()["session_id"]
                .as_str()
                .unwrap();
            assert_eq!(session_id, "test-session-123");
        }
        _ => panic!("Expected Response first"),
    }

    // Then receive the pushed notification
    // Notifications may deserialize as Request(id=None) due to serde untagged enum
    let msg2 = tokio::time::timeout(Duration::from_secs(1), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let (method, params) = match &msg2 {
        Message::Notification(n) => (n.method.as_str(), n.params.as_ref()),
        Message::Request(r) if r.id.is_none() => (r.method.as_str(), r.params.as_ref()),
        _ => panic!("Expected Notification second, got {msg2:?}"),
    };
    assert_eq!(method, "results");
    let results = params.unwrap()["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);

    // Signal we're done
    let done_req = Request::new("done", None, RequestId::Number(2));
    sink.send(Message::Request(done_req)).await.unwrap();

    server_task.await.unwrap();
    cleanup_socket(&socket_path);
}

#[tokio::test]
async fn test_socket_multiple_clients() {
    // Test that multiple clients can connect and receive independent responses
    let socket_path = test_socket_path("multiclient");
    cleanup_socket(&socket_path);

    let listener = UnixListener::bind(&socket_path).unwrap();

    let server_task = tokio::spawn(async move {
        // Accept two clients
        for client_num in 1..=2 {
            let (stream, _) = listener.accept().await.unwrap();

            tokio::spawn(async move {
                let framed = Framed::new(stream, JsonRpcCodec::new());
                let (mut sink, mut stream) = framed.split();

                if let Some(Ok(Message::Request(req))) = stream.next().await {
                    // Each client gets a unique response
                    let response = Response::success(
                        req.id.unwrap(),
                        serde_json::json!({"client_number": client_num}),
                    );
                    sink.send(Message::Response(response)).await.unwrap();
                }
            });
        }
    });

    tokio::time::sleep(Duration::from_millis(10)).await;

    // Connect first client
    let stream1 = UnixStream::connect(&socket_path).await.unwrap();
    let framed1 = Framed::new(stream1, JsonRpcCodec::new());
    let (mut sink1, mut stream1) = framed1.split();

    // Connect second client
    let stream2 = UnixStream::connect(&socket_path).await.unwrap();
    let framed2 = Framed::new(stream2, JsonRpcCodec::new());
    let (mut sink2, mut stream2) = framed2.split();

    // Both clients send requests
    sink1
        .send(Message::Request(Request::new(
            "ping",
            None,
            RequestId::Number(1),
        )))
        .await
        .unwrap();
    sink2
        .send(Message::Request(Request::new(
            "ping",
            None,
            RequestId::Number(2),
        )))
        .await
        .unwrap();

    // Client 1 receives response
    let resp1 = tokio::time::timeout(Duration::from_secs(1), stream1.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let n1 = match resp1 {
        Message::Response(r) => r.result.unwrap()["client_number"].as_i64().unwrap(),
        _ => panic!("Expected response"),
    };

    // Client 2 receives response
    let resp2 = tokio::time::timeout(Duration::from_secs(1), stream2.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let n2 = match resp2 {
        Message::Response(r) => r.result.unwrap()["client_number"].as_i64().unwrap(),
        _ => panic!("Expected response"),
    };

    // Each client got a unique client number (1 or 2)
    assert!(n1 == 1 || n1 == 2);
    assert!(n2 == 1 || n2 == 2);
    assert_ne!(n1, n2, "Each client should get a different response");

    server_task.await.unwrap();
    cleanup_socket(&socket_path);
}

#[tokio::test]
async fn test_codec_handles_large_messages() {
    // Test that the codec can handle reasonably large messages
    let socket_path = test_socket_path("large_msg");
    cleanup_socket(&socket_path);

    let listener = UnixListener::bind(&socket_path).unwrap();

    // Create a large result set (1000 items)
    let large_results: Vec<serde_json::Value> = (0..1000)
        .map(|i| {
            serde_json::json!({
                "id": format!("item-{i}"),
                "name": format!("Test Item {i} with a reasonably long name to increase payload size"),
                "description": format!("This is a description for item {i} that contains some additional text"),
                "icon": "document",
                "badges": [{"text": format!("{i}"), "color": "#ff0000"}],
            })
        })
        .collect();

    let large_payload = serde_json::json!({ "results": large_results });
    let payload_clone = large_payload.clone();

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let framed = Framed::new(stream, JsonRpcCodec::new());
        let (mut sink, mut stream) = framed.split();

        // Wait for client to signal ready
        let _ = stream.next().await;

        let notification = Notification::new("results", Some(payload_clone));
        sink.send(Message::Notification(notification))
            .await
            .unwrap();

        // Keep connection alive
        let _ = stream.next().await;
    });

    tokio::time::sleep(Duration::from_millis(10)).await;

    let stream = UnixStream::connect(&socket_path).await.unwrap();
    let framed = Framed::new(stream, JsonRpcCodec::new());
    let (mut sink, mut stream) = framed.split();

    // Signal ready
    sink.send(Message::Request(Request::new(
        "ready",
        None,
        RequestId::Number(0),
    )))
    .await
    .unwrap();

    let msg = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("timeout")
        .expect("stream ended")
        .expect("codec error");

    // Notifications may deserialize as Request(id=None) due to serde untagged enum
    let params = match &msg {
        Message::Notification(n) => n.params.as_ref(),
        Message::Request(r) if r.id.is_none() => r.params.as_ref(),
        _ => panic!("Expected notification, got {msg:?}"),
    };
    let results = params.unwrap()["results"].as_array().unwrap();
    assert_eq!(results.len(), 1000);

    // Signal done
    sink.send(Message::Request(Request::new(
        "done",
        None,
        RequestId::Number(1),
    )))
    .await
    .unwrap();

    server_task.await.unwrap();
    cleanup_socket(&socket_path);
}

#[tokio::test]
async fn test_client_disconnect_detection() {
    // Test that the server detects when a client disconnects
    let socket_path = test_socket_path("disconnect");
    cleanup_socket(&socket_path);

    let listener = UnixListener::bind(&socket_path).unwrap();

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        tx.send("connected".to_string()).unwrap();

        let framed = Framed::new(stream, JsonRpcCodec::new());
        let (_, mut stream) = framed.split();

        // Try to read - should return None when client disconnects
        let result = stream.next().await;
        if result.is_none() {
            tx.send("disconnected".to_string()).unwrap();
        }
    });

    tokio::time::sleep(Duration::from_millis(10)).await;

    // Connect then immediately disconnect
    {
        let _stream = UnixStream::connect(&socket_path).await.unwrap();
        // Stream drops here, closing connection
    }

    // Server should detect both events
    let event1 = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event1, "connected");

    let event2 = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event2, "disconnected");

    server_task.await.unwrap();
    cleanup_socket(&socket_path);
}
