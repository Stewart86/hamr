//! Direct Niri IPC socket communication.
//!
//! Communicates with Niri compositor via Unix socket instead of spawning `niri msg` processes.
//! This provides better performance and reliability for frequent operations like monitor detection.
//!
//! Protocol:
//! - Socket path from `$NIRI_SOCKET` environment variable
//! - Send JSON request on single line
//! - Receive JSON response on single line
//! - Response format: `{"Ok": {...}}` or `{"Err": "message"}`

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;
use tracing::{debug, trace, warn};

/// Niri IPC client for direct socket communication
pub struct NiriIpc {
    socket_path: String,
}

/// Niri window information
#[derive(Debug, Clone, Deserialize)]
pub struct NiriWindow {
    pub id: u64,
    pub title: String,
    pub app_id: String,
    pub workspace_id: u64,
}

/// Niri output (monitor) information
#[derive(Debug, Clone, Deserialize)]
pub struct NiriOutput {
    pub name: String,
}

/// Niri workspace information
#[derive(Debug, Clone, Deserialize)]
pub struct NiriWorkspace {
    #[allow(dead_code)]
    pub id: u64,
    pub output: String,
    pub is_focused: bool,
    #[serde(default)]
    #[allow(dead_code)]
    pub is_active: bool,
}

/// Possible responses from Niri IPC
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NiriReply<T> {
    Ok {
        #[serde(rename = "Ok")]
        ok: T,
    },
    Err {
        #[serde(rename = "Err")]
        err: String,
    },
}

/// Request types for Niri IPC
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum NiriRequest {
    Simple(String),
    Action {
        #[serde(rename = "Action")]
        action: NiriAction,
    },
}

/// Action types for Niri IPC
#[derive(Debug, Serialize)]
enum NiriAction {
    FocusWindow { id: u64 },
}

impl NiriIpc {
    /// Create a new Niri IPC client from environment
    pub fn from_env() -> Option<Self> {
        let socket_path = std::env::var("NIRI_SOCKET").ok()?;
        debug!("Niri IPC socket path: {}", socket_path);
        Some(Self { socket_path })
    }

    /// Send a request and receive a response
    fn send_request<T: for<'de> Deserialize<'de>>(&self, request: &NiriRequest) -> Option<T> {
        let mut stream = match UnixStream::connect(&self.socket_path) {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to connect to Niri socket: {}", e);
                return None;
            }
        };

        // Set timeouts to avoid hanging
        let timeout = Duration::from_millis(500);
        let _ = stream.set_read_timeout(Some(timeout));
        let _ = stream.set_write_timeout(Some(timeout));

        // Serialize and send request
        let request_json = match serde_json::to_string(request) {
            Ok(j) => j,
            Err(e) => {
                warn!("Failed to serialize Niri request: {}", e);
                return None;
            }
        };

        trace!("Niri IPC request: {}", request_json);

        if let Err(e) = writeln!(stream, "{request_json}") {
            warn!("Failed to write to Niri socket: {}", e);
            return None;
        }

        if let Err(e) = stream.flush() {
            warn!("Failed to flush Niri socket: {}", e);
            return None;
        }

        // Read response
        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        if let Err(e) = reader.read_line(&mut response) {
            warn!("Failed to read from Niri socket: {}", e);
            return None;
        }

        trace!("Niri IPC response: {}", response.trim());

        // Parse response
        match serde_json::from_str::<NiriReply<T>>(&response) {
            Ok(NiriReply::Ok { ok }) => Some(ok),
            Ok(NiriReply::Err { err }) => {
                warn!("Niri IPC error: {}", err);
                None
            }
            Err(e) => {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&response) {
                    if let Some(err) = value.get("Err").and_then(|err| err.as_str()) {
                        warn!("Niri IPC error: {}", err);
                        return None;
                    }

                    let ok_payload = value.get("Ok").and_then(|ok| match ok {
                        serde_json::Value::Object(map) => match request {
                            NiriRequest::Simple(name) => {
                                map.get(name).or_else(|| map.values().next())
                            }
                            NiriRequest::Action { .. } => map.values().next(),
                        },
                        _ => Some(ok),
                    });

                    if let Some(payload) = ok_payload {
                        return serde_json::from_value::<T>(payload.clone()).ok();
                    }

                    warn!(
                        "Failed to parse Niri response: {} (response: {})",
                        e,
                        response.trim()
                    );
                    None
                } else {
                    warn!(
                        "Failed to parse Niri response: {} (response: {})",
                        e,
                        response.trim()
                    );
                    None
                }
            }
        }
    }

    /// Get all windows
    pub fn get_windows(&self) -> Vec<NiriWindow> {
        self.send_request::<Vec<NiriWindow>>(&NiriRequest::Simple("Windows".to_string()))
            .unwrap_or_default()
    }

    /// Get focused output (monitor with focused window)
    pub fn get_focused_output(&self) -> Option<NiriOutput> {
        self.send_request::<NiriOutput>(&NiriRequest::Simple("FocusedOutput".to_string()))
    }

    /// Get all workspaces
    pub fn get_workspaces(&self) -> Vec<NiriWorkspace> {
        self.send_request::<Vec<NiriWorkspace>>(&NiriRequest::Simple("Workspaces".to_string()))
            .unwrap_or_default()
    }

    /// Get focused output name, with fallback to workspace-based detection
    ///
    /// Niri's `FocusedOutput` returns the output with the focused window, which can be `null`
    /// if no window is focused. This method falls back to finding the output via the focused
    /// workspace, which is more reliable for multi-monitor setups.
    pub fn get_focused_output_name(&self) -> Option<String> {
        // First try direct focused-output query
        if let Some(output) = self.get_focused_output() {
            debug!("Niri: focused output via direct query: {}", output.name);
            return Some(output.name);
        }

        // Fallback: find output via focused workspace
        let workspaces = self.get_workspaces();
        let focused = workspaces.iter().find(|w| w.is_focused)?;
        debug!(
            "Niri: focused output via workspace fallback: {}",
            focused.output
        );
        Some(focused.output.clone())
    }

    /// Focus a window by ID
    pub fn focus_window(&self, window_id: u64) -> bool {
        let request = NiriRequest::Action {
            action: NiriAction::FocusWindow { id: window_id },
        };

        // For actions, Niri returns "Handled" on success
        self.send_request::<String>(&request)
            .is_some_and(|s| s == "Handled")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let windows_req = NiriRequest::Simple("Windows".to_string());
        let json = serde_json::to_string(&windows_req).unwrap();
        assert_eq!(json, "\"Windows\"");

        let focus_req = NiriRequest::Action {
            action: NiriAction::FocusWindow { id: 42 },
        };
        let json = serde_json::to_string(&focus_req).unwrap();
        assert_eq!(json, r#"{"Action":{"FocusWindow":{"id":42}}}"#);
    }

    #[test]
    fn test_response_parsing() {
        // Windows response
        let response = r#"{"Ok":[{"id":1,"title":"Test","app_id":"test","workspace_id":1}]}"#;
        let parsed: NiriReply<Vec<NiriWindow>> = serde_json::from_str(response).unwrap();
        if let NiriReply::Ok { ok } = parsed {
            assert_eq!(ok.len(), 1);
            assert_eq!(ok[0].id, 1);
        } else {
            panic!("Expected Ok response");
        }

        // Error response
        let response = r#"{"Err":"Something went wrong"}"#;
        let parsed: NiriReply<Vec<NiriWindow>> = serde_json::from_str(response).unwrap();
        if let NiriReply::Err { err } = parsed {
            assert_eq!(err, "Something went wrong");
        } else {
            panic!("Expected Err response");
        }
    }

    #[test]
    fn test_focused_output_response() {
        let response = r#"{"Ok":{"name":"DP-1"}}"#;
        let parsed: NiriReply<NiriOutput> = serde_json::from_str(response).unwrap();
        if let NiriReply::Ok { ok } = parsed {
            assert_eq!(ok.name, "DP-1");
        } else {
            panic!("Expected Ok response");
        }
    }

    #[test]
    fn test_workspaces_response() {
        let response = r#"{"Ok":[{"id":1,"output":"DP-1","is_focused":true,"is_active":true},{"id":2,"output":"HDMI-1","is_focused":false,"is_active":false}]}"#;
        let parsed: NiriReply<Vec<NiriWorkspace>> = serde_json::from_str(response).unwrap();
        if let NiriReply::Ok { ok } = parsed {
            assert_eq!(ok.len(), 2);
            assert!(ok[0].is_focused);
            assert_eq!(ok[0].output, "DP-1");
        } else {
            panic!("Expected Ok response");
        }
    }
}
