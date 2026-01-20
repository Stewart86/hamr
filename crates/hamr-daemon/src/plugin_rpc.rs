//! RPC methods for communicating with socket plugins.
//!
//! This module provides helper functions for sending JSON-RPC 2.0 requests
//! to connected socket plugins for search, action, and initial operations.

use hamr_rpc::protocol::{Message, Notification, Request, RequestId};
use serde::Serialize;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use tracing::{trace, warn};

static REQUEST_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_request_id() -> RequestId {
    RequestId::Number(REQUEST_ID_COUNTER.fetch_add(1, Ordering::SeqCst))
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchParams {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionParams {
    pub item_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct InitialParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

/// Send a search request to a plugin.
///
/// # Errors
///
/// Returns an error if the channel send fails.
pub fn send_search(
    sender: &mpsc::UnboundedSender<Message>,
    plugin_id: &str,
    query: String,
    context: Option<String>,
) -> Result<(), String> {
    let params = SearchParams { query, context };
    send_request(sender, plugin_id, "search", &params)
}

/// Send an action request to a plugin.
///
/// # Errors
///
/// Returns an error if the channel send fails.
pub fn send_action(
    sender: &mpsc::UnboundedSender<Message>,
    plugin_id: &str,
    item_id: String,
    action: Option<String>,
    context: Option<String>,
    source: Option<String>,
) -> Result<(), String> {
    let params = ActionParams {
        item_id,
        action,
        context,
        source,
    };
    send_request(sender, plugin_id, "action", &params)
}

/// Send an initial request to a plugin.
///
/// # Errors
///
/// Returns an error if the channel send fails.
pub fn send_initial(
    sender: &mpsc::UnboundedSender<Message>,
    plugin_id: &str,
    context: Option<String>,
) -> Result<(), String> {
    let params = InitialParams { context };
    send_request(sender, plugin_id, "initial", &params)
}

/// Send a slider changed notification to a plugin.
///
/// # Errors
///
/// Returns an error if the channel send fails.
pub fn send_slider_changed(
    sender: &mpsc::UnboundedSender<Message>,
    plugin_id: &str,
    slider_id: &str,
    value: f64,
) -> Result<(), String> {
    let params = serde_json::json!({
        "id": slider_id,
        "value": value,
    });
    send_notification(sender, plugin_id, "slider_changed", params)
}

/// Send a switch toggled notification to a plugin.
///
/// # Errors
///
/// Returns an error if the channel send fails.
pub fn send_switch_toggled(
    sender: &mpsc::UnboundedSender<Message>,
    plugin_id: &str,
    switch_id: &str,
    value: bool,
) -> Result<(), String> {
    let params = serde_json::json!({
        "id": switch_id,
        "value": value,
    });
    send_notification(sender, plugin_id, "switch_toggled", params)
}

/// Send a form submitted request to a plugin.
///
/// # Errors
///
/// Returns an error if the channel send fails.
// Internal API always uses default hasher - generic hasher adds complexity without benefit
#[allow(clippy::implicit_hasher)]
pub fn send_form_submitted(
    sender: &mpsc::UnboundedSender<Message>,
    plugin_id: &str,
    form_data: &std::collections::HashMap<String, String>,
    context: Option<&str>,
) -> Result<(), String> {
    let params = serde_json::json!({
        "form_data": form_data,
        "context": context,
    });
    send_request_raw(sender, plugin_id, "form_submitted", params)
}

fn send_request<P: Serialize>(
    sender: &mpsc::UnboundedSender<Message>,
    plugin_id: &str,
    method: &str,
    params: &P,
) -> Result<(), String> {
    let params_value =
        serde_json::to_value(params).map_err(|e| format!("Failed to serialize params: {e}"))?;

    send_request_raw(sender, plugin_id, method, params_value)
}

fn send_request_raw(
    sender: &mpsc::UnboundedSender<Message>,
    plugin_id: &str,
    method: &str,
    params: Value,
) -> Result<(), String> {
    let id = next_request_id();
    let request = Request::new(method, Some(params), id.clone());
    let msg = Message::Request(request);

    trace!(
        "[{}] Sending request: method={}, id={:?}",
        plugin_id, method, id
    );

    sender.send(msg).map_err(|e| {
        warn!("[{}] Failed to send {} request: {}", plugin_id, method, e);
        format!("Failed to send {method} request: {e}")
    })
}

fn send_notification(
    sender: &mpsc::UnboundedSender<Message>,
    plugin_id: &str,
    method: &str,
    params: Value,
) -> Result<(), String> {
    let notification = Notification::new(method, Some(params));
    let msg = Message::Notification(notification);

    trace!("[{}] Sending notification: method={}", plugin_id, method);

    sender.send(msg).map_err(|e| {
        warn!(
            "[{}] Failed to send {} notification: {}",
            plugin_id, method, e
        );
        format!("Failed to send {method} notification: {e}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_increments() {
        let id1 = next_request_id();
        let id2 = next_request_id();

        match (id1, id2) {
            (RequestId::Number(n1), RequestId::Number(n2)) => {
                assert!(n2 > n1);
            }
            _ => panic!("Expected numeric IDs"),
        }
    }

    #[test]
    fn test_search_params_serialization() {
        let params = SearchParams {
            query: "test query".to_string(),
            context: Some("ctx".to_string()),
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["query"], "test query");
        assert_eq!(json["context"], "ctx");
    }

    #[test]
    fn test_search_params_without_context() {
        let params = SearchParams {
            query: "test".to_string(),
            context: None,
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["query"], "test");
        assert!(json.get("context").is_none());
    }

    #[test]
    fn test_action_params_serialization() {
        let params = ActionParams {
            item_id: "item-1".to_string(),
            action: Some("copy".to_string()),
            context: None,
            source: None,
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["item_id"], "item-1");
        assert_eq!(json["action"], "copy");
    }

    #[tokio::test]
    async fn test_send_search() {
        let (tx, mut rx) = mpsc::unbounded_channel();

        send_search(&tx, "test-plugin", "hello".to_string(), None).unwrap();

        let msg = rx.recv().await.unwrap();
        if let Message::Request(req) = msg {
            assert_eq!(req.method, "search");
            assert!(req.id.is_some());
            let params = req.params.unwrap();
            assert_eq!(params["query"], "hello");
        } else {
            panic!("Expected request message");
        }
    }

    #[tokio::test]
    async fn test_send_action() {
        let (tx, mut rx) = mpsc::unbounded_channel();

        send_action(
            &tx,
            "test-plugin",
            "item-123".to_string(),
            Some("launch".to_string()),
            None,
            None,
        )
        .unwrap();

        let msg = rx.recv().await.unwrap();
        if let Message::Request(req) = msg {
            assert_eq!(req.method, "action");
            let params = req.params.unwrap();
            assert_eq!(params["item_id"], "item-123");
            assert_eq!(params["action"], "launch");
        } else {
            panic!("Expected request message");
        }
    }

    #[tokio::test]
    async fn test_send_initial() {
        let (tx, mut rx) = mpsc::unbounded_channel();

        send_initial(&tx, "test-plugin", None).unwrap();

        let msg = rx.recv().await.unwrap();
        if let Message::Request(req) = msg {
            assert_eq!(req.method, "initial");
        } else {
            panic!("Expected request message");
        }
    }

    #[tokio::test]
    async fn test_send_slider_changed() {
        let (tx, mut rx) = mpsc::unbounded_channel();

        send_slider_changed(&tx, "test-plugin", "vol", 75.0).unwrap();

        let msg = rx.recv().await.unwrap();
        if let Message::Notification(notif) = msg {
            assert_eq!(notif.method, "slider_changed");
            let params = notif.params.unwrap();
            assert_eq!(params["id"], "vol");
            assert_eq!(params["value"], 75.0);
        } else {
            panic!("Expected notification message");
        }
    }

    #[tokio::test]
    async fn test_send_switch_toggled() {
        let (tx, mut rx) = mpsc::unbounded_channel();

        send_switch_toggled(&tx, "test-plugin", "dark_mode", true).unwrap();

        let msg = rx.recv().await.unwrap();
        if let Message::Notification(notif) = msg {
            assert_eq!(notif.method, "switch_toggled");
            let params = notif.params.unwrap();
            assert_eq!(params["id"], "dark_mode");
            assert_eq!(params["value"], true);
        } else {
            panic!("Expected notification message");
        }
    }

    #[tokio::test]
    async fn test_send_form_submitted() {
        let (tx, mut rx) = mpsc::unbounded_channel();

        let mut form_data = std::collections::HashMap::new();
        form_data.insert("username".to_string(), "test_user".to_string());
        form_data.insert("password".to_string(), "secret".to_string());

        send_form_submitted(&tx, "test-plugin", &form_data, Some("ctx")).unwrap();

        let msg = rx.recv().await.unwrap();
        if let Message::Request(req) = msg {
            assert_eq!(req.method, "form_submitted");
            let params = req.params.unwrap();
            assert_eq!(params["form_data"]["username"], "test_user");
            assert_eq!(params["form_data"]["password"], "secret");
            assert_eq!(params["context"], "ctx");
        } else {
            panic!("Expected request message");
        }
    }

    #[tokio::test]
    async fn test_send_request_channel_closed() {
        let (tx, rx) = mpsc::unbounded_channel();
        drop(rx);

        let result = send_search(&tx, "test-plugin", "hello".to_string(), None);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to send"));
    }
}
