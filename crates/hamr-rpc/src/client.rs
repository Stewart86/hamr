//! RPC client helper for connecting to the hamr daemon.
//!
//! Provides a convenient wrapper for establishing connections and sending/receiving
//! JSON-RPC messages over Unix sockets.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::UnixStream;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_util::codec::Framed;

use crate::protocol::{
    ClientRole, Message, Notification, RegisterParams, RegisterResult, Request, RequestId,
    Response, RpcError,
};
use crate::transport::{CodecError, JsonRpcCodec};

fn runtime_dir() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR").map_or_else(|_| std::env::temp_dir(), PathBuf::from)
}

fn is_dev_socket() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };

    let Some(parent) = exe.parent() else {
        return false;
    };

    if !parent.ends_with("target/debug") {
        return false;
    }

    let Some(name) = exe.file_name().and_then(|file| file.to_str()) else {
        return false;
    };

    matches!(name, "hamr" | "hamr-daemon" | "hamr-gtk" | "hamr-tui")
}

/// Get the socket path for the hamr daemon in dev mode.
#[must_use]
pub fn dev_socket_path() -> PathBuf {
    runtime_dir().join("hamr-dev.sock")
}

/// Get the socket path for the hamr daemon.
///
/// On Linux, prefers `$XDG_RUNTIME_DIR` for proper runtime file handling.
/// Falls back to the system temp directory for cross-platform compatibility.
#[must_use]
pub fn socket_path() -> PathBuf {
    let socket_name = if is_dev_socket() {
        "hamr-dev.sock"
    } else {
        "hamr.sock"
    };

    runtime_dir().join(socket_name)
}

/// Errors that can occur with the RPC client
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Codec error: {0}")]
    Codec(#[from] CodecError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("RPC error: {code} - {message}")]
    Rpc { code: i32, message: String },

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Request timeout")]
    Timeout,

    #[error("Unexpected response type")]
    UnexpectedResponse,
}

impl From<RpcError> for ClientError {
    fn from(e: RpcError) -> Self {
        ClientError::Rpc {
            code: e.code,
            message: e.message,
        }
    }
}

/// Pending request waiting for a response
type PendingRequest = oneshot::Sender<Result<Response, ClientError>>;

/// RPC client for communicating with the hamr daemon
pub struct RpcClient {
    sender: Arc<Mutex<futures_util::stream::SplitSink<Framed<UnixStream, JsonRpcCodec>, Message>>>,
    incoming_rx: mpsc::Receiver<Message>,
    pending: Arc<Mutex<HashMap<RequestId, PendingRequest>>>,
    next_id: AtomicU64,
    session_id: Option<String>,
}

impl RpcClient {
    /// Connect to the hamr daemon at the default socket path.
    ///
    /// # Errors
    ///
    /// Returns `ClientError::Io` if the socket connection fails.
    pub async fn connect() -> Result<Self, ClientError> {
        Self::connect_to(socket_path()).await
    }

    /// Connect to the hamr daemon at a custom socket path.
    ///
    /// # Errors
    ///
    /// Returns `ClientError::Io` if the socket connection fails.
    pub async fn connect_to(path: PathBuf) -> Result<Self, ClientError> {
        let stream = UnixStream::connect(&path).await?;
        let framed = Framed::new(stream, JsonRpcCodec::new());
        let (sink, stream) = framed.split();

        let pending: Arc<Mutex<HashMap<RequestId, PendingRequest>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_clone = pending.clone();

        let (incoming_tx, incoming_rx) = mpsc::channel(64);

        tokio::spawn(async move {
            let mut stream = stream;
            while let Some(result) = stream.next().await {
                match result {
                    Ok(msg) => match &msg {
                        Message::Response(resp) => {
                            let mut pending = pending_clone.lock().await;
                            if let Some(tx) = pending.remove(&resp.id) {
                                let _ = tx.send(Ok(resp.clone()));
                            }
                        }
                        Message::Request(_) | Message::Notification(_) => {
                            if incoming_tx.send(msg).await.is_err() {
                                break;
                            }
                        }
                    },
                    Err(e) => {
                        let mut pending = pending_clone.lock().await;
                        for (_, tx) in pending.drain() {
                            let _ = tx.send(Err(ClientError::Codec(CodecError::Io(
                                std::io::Error::other(e.to_string()),
                            ))));
                        }
                        break;
                    }
                }
            }
        });

        Ok(Self {
            sender: Arc::new(Mutex::new(sink)),
            incoming_rx,
            pending,
            next_id: AtomicU64::new(1),
            session_id: None,
        })
    }

    /// Register this client with the daemon.
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC request fails or the daemon rejects registration.
    pub async fn register(&mut self, role: ClientRole) -> Result<String, ClientError> {
        let params = RegisterParams { role };
        let result: RegisterResult = self
            .request("register", Some(serde_json::to_value(params)?))
            .await?;

        self.session_id = Some(result.session_id.clone());
        Ok(result.session_id)
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Send an RPC request and wait for a response.
    ///
    /// # Errors
    ///
    /// Returns an error if sending fails, the connection closes, or deserialization fails.
    pub async fn request<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<T, ClientError> {
        let id = RequestId::Number(self.next_id.fetch_add(1, Ordering::SeqCst));
        let request = Request::new(method, params, id.clone());

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            pending.insert(id, tx);
        }

        {
            let mut sender = self.sender.lock().await;
            sender.send(Message::Request(request)).await?;
        }

        let response = tokio::time::timeout(Duration::from_secs(30), rx)
            .await
            .map_err(|_| ClientError::Timeout)?
            .map_err(|_| ClientError::ConnectionClosed)??;

        if let Some(error) = response.error {
            return Err(error.into());
        }

        let result = response.result.ok_or(ClientError::UnexpectedResponse)?;
        Ok(serde_json::from_value(result)?)
    }

    /// Send a notification (no response expected).
    ///
    /// # Errors
    ///
    /// Returns `ClientError::Codec` if sending fails.
    pub async fn notify(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<(), ClientError> {
        let notification = Notification::new(method, params);
        let mut sender = self.sender.lock().await;
        sender.send(Message::Notification(notification)).await?;
        Ok(())
    }

    pub async fn recv(&mut self) -> Option<Message> {
        self.incoming_rx.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_path() {
        let path = socket_path();
        assert!(path.ends_with("hamr.sock"));
    }

    #[test]
    fn test_dev_socket_path() {
        let path = dev_socket_path();
        assert!(path.ends_with("hamr-dev.sock"));
    }

    #[test]
    fn test_client_error_from_rpc_error() {
        let rpc_err = RpcError::not_registered();
        let client_err: ClientError = rpc_err.into();

        match client_err {
            ClientError::Rpc { code, message } => {
                assert_eq!(code, crate::protocol::NOT_REGISTERED);
                assert!(message.contains("Not registered"));
            }
            _ => panic!("Expected Rpc error"),
        }
    }

    #[test]
    fn test_client_error_display() {
        let err = ClientError::ConnectionClosed;
        assert_eq!(err.to_string(), "Connection closed");

        let err = ClientError::Timeout;
        assert_eq!(err.to_string(), "Request timeout");

        let err = ClientError::UnexpectedResponse;
        assert_eq!(err.to_string(), "Unexpected response type");

        let err = ClientError::Rpc {
            code: -32000,
            message: "Not registered".to_string(),
        };
        assert!(err.to_string().contains("-32000"));
        assert!(err.to_string().contains("Not registered"));
    }

    #[test]
    fn test_client_error_from_io() {
        let io_err =
            std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "connection refused");
        let client_err: ClientError = io_err.into();
        assert!(matches!(client_err, ClientError::Io(_)));
    }

    #[test]
    fn test_client_error_from_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
        let client_err: ClientError = json_err.into();
        assert!(matches!(client_err, ClientError::Json(_)));
    }

    #[test]
    fn test_client_error_from_codec() {
        let codec_err = crate::transport::CodecError::MessageTooLarge(100_000_000);
        let client_err: ClientError = codec_err.into();
        assert!(matches!(client_err, ClientError::Codec(_)));
        assert!(client_err.to_string().contains("100000000"));
    }
}
