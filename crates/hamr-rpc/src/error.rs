//! Error types for the hamr-rpc crate.
//!
//! This module provides a unified error type for all RPC-related operations.

use crate::transport::CodecError;

/// Unified error type for RPC operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("RPC error {code}: {message}")]
    Rpc { code: i32, message: String },

    #[error("Connection closed")]
    Disconnected,

    #[error("Request timeout")]
    Timeout,

    #[error("Codec error: {0}")]
    Codec(#[from] CodecError),

    #[error("Unexpected response")]
    UnexpectedResponse,
}

impl Error {
    pub fn rpc(code: i32, message: impl Into<String>) -> Self {
        Self::Rpc {
            code,
            message: message.into(),
        }
    }
}

impl From<crate::protocol::RpcError> for Error {
    fn from(e: crate::protocol::RpcError) -> Self {
        Self::Rpc {
            code: e.code,
            message: e.message,
        }
    }
}

impl From<crate::client::ClientError> for Error {
    fn from(e: crate::client::ClientError) -> Self {
        match e {
            crate::client::ClientError::Io(e) => Self::Io(e),
            crate::client::ClientError::Codec(e) => Self::Codec(e),
            crate::client::ClientError::Json(e) => Self::Json(e),
            crate::client::ClientError::Rpc { code, message } => Self::Rpc { code, message },
            crate::client::ClientError::ConnectionClosed => Self::Disconnected,
            crate::client::ClientError::Timeout => Self::Timeout,
            crate::client::ClientError::UnexpectedResponse => Self::UnexpectedResponse,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::RpcError;

    #[test]
    fn test_error_rpc_factory() {
        let err = Error::rpc(-32600, "Invalid Request");
        match err {
            Error::Rpc { code, message } => {
                assert_eq!(code, -32600);
                assert_eq!(message, "Invalid Request");
            }
            _ => panic!("Expected Rpc error"),
        }
    }

    #[test]
    fn test_error_from_rpc_error() {
        let rpc_err = RpcError::method_not_found();
        let err: Error = rpc_err.into();

        match err {
            Error::Rpc { code, message } => {
                assert_eq!(code, -32601);
                assert!(message.contains("not found"));
            }
            _ => panic!("Expected Rpc error"),
        }
    }

    #[test]
    fn test_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "socket not found");
        let err: Error = io_err.into();

        assert!(matches!(err, Error::Io(_)));
        assert!(err.to_string().contains("socket not found"));
    }

    #[test]
    fn test_error_from_json_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let err: Error = json_err.into();

        assert!(matches!(err, Error::Json(_)));
    }

    #[test]
    fn test_error_from_codec_error() {
        let codec_err = CodecError::MessageTooLarge(999_999_999);
        let err: Error = codec_err.into();

        assert!(matches!(err, Error::Codec(_)));
        assert!(err.to_string().contains("999999999"));
    }

    #[test]
    fn test_error_display() {
        let err = Error::Disconnected;
        assert_eq!(err.to_string(), "Connection closed");

        let err = Error::Timeout;
        assert_eq!(err.to_string(), "Request timeout");

        let err = Error::UnexpectedResponse;
        assert_eq!(err.to_string(), "Unexpected response");

        let err = Error::rpc(-32000, "Not registered");
        assert!(err.to_string().contains("-32000"));
        assert!(err.to_string().contains("Not registered"));
    }

    #[test]
    fn test_error_from_client_error() {
        use crate::client::ClientError;

        // Test all ClientError variants
        let err: Error = ClientError::ConnectionClosed.into();
        assert!(matches!(err, Error::Disconnected));

        let err: Error = ClientError::Timeout.into();
        assert!(matches!(err, Error::Timeout));

        let err: Error = ClientError::UnexpectedResponse.into();
        assert!(matches!(err, Error::UnexpectedResponse));

        let err: Error = ClientError::Rpc {
            code: -32001,
            message: "Already registered".to_string(),
        }
        .into();
        match err {
            Error::Rpc { code, message } => {
                assert_eq!(code, -32001);
                assert_eq!(message, "Already registered");
            }
            _ => panic!("Expected Rpc error"),
        }
    }

    #[test]
    fn test_error_from_client_error_io() {
        use crate::client::ClientError;

        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broken");
        let client_err = ClientError::Io(io_err);
        let err: Error = client_err.into();
        assert!(matches!(err, Error::Io(_)));
        assert!(err.to_string().contains("pipe broken"));
    }

    #[test]
    fn test_error_from_client_error_codec() {
        use crate::client::ClientError;

        let codec_err = CodecError::MessageTooLarge(50_000_000);
        let client_err = ClientError::Codec(codec_err);
        let err: Error = client_err.into();
        assert!(matches!(err, Error::Codec(_)));
        assert!(err.to_string().contains("50000000"));
    }

    #[test]
    fn test_error_from_client_error_json() {
        use crate::client::ClientError;

        let json_err = serde_json::from_str::<serde_json::Value>("}{").unwrap_err();
        let client_err = ClientError::Json(json_err);
        let err: Error = client_err.into();
        assert!(matches!(err, Error::Json(_)));
    }

    #[test]
    fn test_error_debug_format() {
        let err = Error::Disconnected;
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("Disconnected"));

        let err = Error::rpc(-32000, "test error");
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("Rpc"));
        assert!(debug_str.contains("-32000"));
    }

    #[test]
    fn test_result_type_alias() {
        // Testing that Result<T> type alias compiles and resolves correctly
        #[allow(clippy::unnecessary_wraps)]
        fn returns_result() -> Result<i32> {
            Ok(42)
        }

        fn returns_error() -> Result<i32> {
            Err(Error::Timeout)
        }

        assert_eq!(returns_result().unwrap(), 42);
        assert!(matches!(returns_error(), Err(Error::Timeout)));
    }
}
