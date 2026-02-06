//! JSON-RPC 2.0 protocol types.
//!
//! This module provides the core JSON-RPC 2.0 message types for communication
//! between hamr components.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use hamr_types::PluginManifest;

pub const JSONRPC_VERSION: &str = "2.0";
pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;
pub const NOT_REGISTERED: i32 = -32000;
pub const ALREADY_REGISTERED: i32 = -32001;
pub const PLUGIN_NOT_FOUND: i32 = -32002;
pub const UI_OCCUPIED: i32 = -32003;
pub const NOT_ACTIVE_UI: i32 = -32004;
pub const CONTROL_REQUIRED: i32 = -32005;

/// JSON-RPC 2.0 Request ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    Number(u64),
    String(String),
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestId::Number(n) => write!(f, "{n}"),
            RequestId::String(s) => write!(f, "{s}"),
        }
    }
}

impl From<u64> for RequestId {
    fn from(n: u64) -> Self {
        RequestId::Number(n)
    }
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        RequestId::String(s)
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        RequestId::String(s.to_string())
    }
}

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RequestId>,
}

impl Request {
    #[must_use]
    pub fn new(method: impl Into<String>, params: Option<Value>, id: RequestId) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params,
            id: Some(id),
        }
    }

    #[must_use]
    pub fn notification(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params,
            id: None,
        }
    }
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
    pub id: RequestId,
}

impl Response {
    #[must_use]
    pub fn success(id: RequestId, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    #[must_use]
    pub fn error(id: RequestId, error: RpcError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }
}

/// JSON-RPC 2.0 Notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl Notification {
    #[must_use]
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params,
        }
    }
}

/// JSON-RPC 2.0 Error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl RpcError {
    #[must_use]
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    #[must_use]
    pub fn with_data(code: i32, message: impl Into<String>, data: Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }

    #[must_use]
    pub fn parse_error() -> Self {
        Self::new(PARSE_ERROR, "Parse error")
    }

    #[must_use]
    pub fn invalid_request() -> Self {
        Self::new(INVALID_REQUEST, "Invalid Request")
    }

    #[must_use]
    pub fn method_not_found() -> Self {
        Self::new(METHOD_NOT_FOUND, "Method not found")
    }

    #[must_use]
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(INVALID_PARAMS, message)
    }

    #[must_use]
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(INTERNAL_ERROR, message)
    }

    #[must_use]
    pub fn not_registered() -> Self {
        Self::new(NOT_REGISTERED, "Not registered")
    }

    #[must_use]
    pub fn already_registered() -> Self {
        Self::new(ALREADY_REGISTERED, "Already registered")
    }

    #[must_use]
    pub fn plugin_not_found(plugin_id: impl Into<String>) -> Self {
        Self::new(
            PLUGIN_NOT_FOUND,
            format!("Plugin not found: {}", plugin_id.into()),
        )
    }

    #[must_use]
    pub fn ui_occupied() -> Self {
        Self::new(UI_OCCUPIED, "Another UI is active")
    }

    #[must_use]
    pub fn not_active_ui() -> Self {
        Self::new(NOT_ACTIVE_UI, "Not the active UI")
    }

    #[must_use]
    pub fn control_required() -> Self {
        Self::new(CONTROL_REQUIRED, "Control or UI client required")
    }
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RPC error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for RpcError {}

/// Incoming message that could be a request, response, or notification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Message {
    Request(Request),
    Response(Response),
    Notification(Notification),
}

impl Message {
    /// Parse a JSON string into a `Message`.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON is malformed or doesn't match any message type.
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize this message to JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    #[must_use]
    pub fn is_request(&self) -> bool {
        matches!(self, Message::Request(r) if r.id.is_some())
    }

    #[must_use]
    pub fn is_notification(&self) -> bool {
        matches!(self, Message::Request(r) if r.id.is_none())
            || matches!(self, Message::Notification(_))
    }

    #[must_use]
    pub fn is_response(&self) -> bool {
        matches!(self, Message::Response(_))
    }
}

/// Client role for registration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientRole {
    /// UI client - receives updates, one active at a time
    Ui { name: String },

    /// Control client - send commands only
    Control,

    /// Plugin daemon - persistent plugin connection
    Plugin {
        id: String,
        manifest: PluginManifest,
    },
}

/// Registration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterParams {
    pub role: ClientRole,
}

/// Registration result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResult {
    pub session_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = Request::new("test", Some(serde_json::json!({"key": "value"})), 1.into());
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"test\""));
        assert!(json.contains("\"id\":1"));
    }

    #[test]
    fn test_request_without_params() {
        let req = Request::new("ping", None, 1.into());
        let json = serde_json::to_string(&req).unwrap();
        assert!(
            !json.contains("\"params\""),
            "params should be omitted when None"
        );
    }

    #[test]
    fn test_notification_no_id() {
        let notif = Request::notification("test", None);
        let json = serde_json::to_string(&notif).unwrap();
        assert!(!json.contains("\"id\""));
    }

    #[test]
    fn test_notification_struct() {
        let notif = Notification::new("update", Some(serde_json::json!({"count": 5})));
        let json = serde_json::to_string(&notif).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"update\""));
        assert!(json.contains("\"count\":5"));
        assert!(!json.contains("\"id\""));
    }

    #[test]
    fn test_response_success() {
        let resp = Response::success(1.into(), serde_json::json!({"status": "ok"}));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"result\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_response_error() {
        let resp = Response::error(1.into(), RpcError::method_not_found());
        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("\"result\""));
        assert!(json.contains("\"error\""));
        assert!(json.contains("-32601"));
    }

    #[test]
    fn test_response_roundtrip() {
        let resp = Response::success(42.into(), serde_json::json!({"data": [1, 2, 3]}));
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: Response = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, RequestId::Number(42));
        assert!(parsed.result.is_some());
        assert!(parsed.error.is_none());
    }

    #[test]
    fn test_request_id_types() {
        let id_num: RequestId = 42.into();
        let id_str: RequestId = "abc-123".into();

        assert_eq!(id_num, RequestId::Number(42));
        assert_eq!(id_str, RequestId::String("abc-123".to_string()));
    }

    #[test]
    fn test_request_id_from_str() {
        let id: RequestId = "request-1".into();
        assert_eq!(id, RequestId::String("request-1".to_string()));
    }

    #[test]
    fn test_request_id_serialization() {
        let id_num = RequestId::Number(123);
        let json = serde_json::to_string(&id_num).unwrap();
        assert_eq!(json, "123");

        let id_str = RequestId::String("abc".to_string());
        let json = serde_json::to_string(&id_str).unwrap();
        assert_eq!(json, "\"abc\"");
    }

    #[test]
    fn test_request_id_deserialization() {
        let id: RequestId = serde_json::from_str("456").unwrap();
        assert_eq!(id, RequestId::Number(456));

        let id: RequestId = serde_json::from_str("\"xyz\"").unwrap();
        assert_eq!(id, RequestId::String("xyz".to_string()));
    }

    #[test]
    fn test_rpc_error_parse_error() {
        let err = RpcError::parse_error();
        assert_eq!(err.code, PARSE_ERROR);
        assert!(err.message.to_lowercase().contains("parse"));
    }

    #[test]
    fn test_rpc_error_invalid_request() {
        let err = RpcError::invalid_request();
        assert_eq!(err.code, INVALID_REQUEST);
    }

    #[test]
    fn test_rpc_error_method_not_found() {
        let err = RpcError::method_not_found();
        assert_eq!(err.code, METHOD_NOT_FOUND);
    }

    #[test]
    fn test_rpc_error_invalid_params() {
        let err = RpcError::invalid_params("missing 'id' field");
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("missing"));
    }

    #[test]
    fn test_rpc_error_internal_error() {
        let err = RpcError::internal_error("database connection failed");
        assert_eq!(err.code, INTERNAL_ERROR);
        assert!(err.message.contains("database"));
    }

    #[test]
    fn test_rpc_error_not_registered() {
        let err = RpcError::not_registered();
        assert_eq!(err.code, NOT_REGISTERED);
    }

    #[test]
    fn test_rpc_error_already_registered() {
        let err = RpcError::already_registered();
        assert_eq!(err.code, ALREADY_REGISTERED);
    }

    #[test]
    fn test_rpc_error_plugin_not_found() {
        let err = RpcError::plugin_not_found("calculator");
        assert_eq!(err.code, PLUGIN_NOT_FOUND);
        assert!(err.message.contains("calculator"));
    }

    #[test]
    fn test_rpc_error_ui_occupied() {
        let err = RpcError::ui_occupied();
        assert_eq!(err.code, UI_OCCUPIED);
    }

    #[test]
    fn test_rpc_error_not_active_ui() {
        let err = RpcError::not_active_ui();
        assert_eq!(err.code, NOT_ACTIVE_UI);
        assert!(err.message.contains("active"));
    }

    #[test]
    fn test_rpc_error_control_required() {
        let err = RpcError::control_required();
        assert_eq!(err.code, CONTROL_REQUIRED);
        assert!(err.message.contains("Control"));
    }

    #[test]
    fn test_rpc_error_with_data() {
        let err = RpcError::with_data(
            INVALID_PARAMS,
            "Validation failed",
            serde_json::json!({"field": "name", "reason": "too short"}),
        );
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.data.is_some());
        let data = err.data.unwrap();
        assert_eq!(data["field"], "name");
    }

    #[test]
    fn test_message_parse_request() {
        let json = r#"{"jsonrpc":"2.0","method":"test","params":{"x":1},"id":1}"#;
        let msg = Message::parse(json).unwrap();
        assert!(msg.is_request());
        assert!(!msg.is_notification());
        assert!(!msg.is_response());
    }

    #[test]
    fn test_message_parse_notification() {
        let json = r#"{"jsonrpc":"2.0","method":"update","params":{}}"#;
        let msg = Message::parse(json).unwrap();
        // Note: Without 'id', this parses as Notification
        assert!(msg.is_notification());
        assert!(!msg.is_response());
    }

    #[test]
    fn test_message_parse_response() {
        let json = r#"{"jsonrpc":"2.0","result":{"ok":true},"id":1}"#;
        let msg = Message::parse(json).unwrap();
        assert!(msg.is_response());
        assert!(!msg.is_request());
        assert!(!msg.is_notification());
    }

    #[test]
    fn test_message_to_json() {
        let req = Request::new("ping", None, 1.into());
        let msg = Message::Request(req);
        let json = msg.to_json().unwrap();
        assert!(json.contains("\"method\":\"ping\""));
    }

    #[test]
    fn test_message_parse_error_response() {
        let json =
            r#"{"jsonrpc":"2.0","error":{"code":-32601,"message":"Method not found"},"id":1}"#;
        let msg = Message::parse(json).unwrap();
        assert!(msg.is_response());

        if let Message::Response(resp) = msg {
            assert!(resp.error.is_some());
            assert_eq!(resp.error.unwrap().code, -32601);
        }
    }

    #[test]
    fn test_client_role_serialization() {
        let ui = ClientRole::Ui {
            name: "tui".to_string(),
        };
        let json = serde_json::to_string(&ui).unwrap();
        assert!(json.contains("\"type\":\"ui\""));
        assert!(json.contains("\"name\":\"tui\""));

        let control = ClientRole::Control;
        let json = serde_json::to_string(&control).unwrap();
        assert!(json.contains("\"type\":\"control\""));
    }

    #[test]
    fn test_client_role_plugin() {
        let manifest = PluginManifest {
            id: "calculator".to_string(),
            name: "Calculator".to_string(),
            description: Some("Math plugin".to_string()),
            icon: Some("calculate".to_string()),
            prefix: Some("=".to_string()),
            priority: 100,
        };

        let plugin = ClientRole::Plugin {
            id: "calculator".to_string(),
            manifest,
        };

        let json = serde_json::to_string(&plugin).unwrap();
        assert!(json.contains("\"type\":\"plugin\""));
        assert!(json.contains("\"id\":\"calculator\""));
        assert!(json.contains("\"name\":\"Calculator\""));
    }

    #[test]
    fn test_client_role_deserialization() {
        let json = r#"{"type":"ui","name":"gtk"}"#;
        let role: ClientRole = serde_json::from_str(json).unwrap();

        match role {
            ClientRole::Ui { name } => assert_eq!(name, "gtk"),
            _ => panic!("Expected Ui role"),
        }

        let json = r#"{"type":"control"}"#;
        let role: ClientRole = serde_json::from_str(json).unwrap();
        assert!(matches!(role, ClientRole::Control));
    }

    #[test]
    fn test_register_params_serialization() {
        let params = RegisterParams {
            role: ClientRole::Ui {
                name: "test-ui".to_string(),
            },
        };

        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("\"role\""));
        assert!(json.contains("\"type\":\"ui\""));
    }

    #[test]
    fn test_register_result_deserialization() {
        let json = r#"{"session_id":"abc123"}"#;
        let result: RegisterResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.session_id, "abc123");
    }
}
