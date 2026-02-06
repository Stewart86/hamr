//! Error types for the hamr daemon.

use hamr_rpc::protocol::RpcError;

/// Errors that can occur in the daemon
#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Client not registered
    #[error("Client not registered")]
    NotRegistered,

    /// Client already registered
    #[error("Client already registered")]
    AlreadyRegistered,

    /// Plugin not found
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    /// UI client slot is occupied
    #[error("Another UI client is active")]
    UiOccupied,

    /// Not the active UI client
    #[error("Not the active UI")]
    NotActiveUi,

    /// Control or UI client required
    #[error("Control or UI client required")]
    ControlRequired,

    /// Core error
    #[error("Core error: {0}")]
    Core(#[from] hamr_core::Error),

    /// Codec error
    #[error("Codec error: {0}")]
    Codec(#[from] hamr_rpc::transport::CodecError),

    /// Invalid parameters
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    /// Method not found
    #[error("Method not found: {0}")]
    MethodNotFound(String),

    /// File watcher error
    #[error("Watcher error: {0}")]
    Watcher(#[from] notify::Error),
}

impl From<DaemonError> for RpcError {
    fn from(err: DaemonError) -> Self {
        match err {
            DaemonError::Io(e) => RpcError::internal_error(e.to_string()),
            DaemonError::Json(e) => RpcError::internal_error(e.to_string()),
            DaemonError::NotRegistered => RpcError::not_registered(),
            DaemonError::AlreadyRegistered => RpcError::already_registered(),
            DaemonError::PluginNotFound(id) => RpcError::plugin_not_found(id),
            DaemonError::UiOccupied => RpcError::ui_occupied(),
            DaemonError::NotActiveUi => RpcError::not_active_ui(),
            DaemonError::ControlRequired => RpcError::control_required(),
            DaemonError::Core(e) => RpcError::internal_error(e.to_string()),
            DaemonError::Codec(e) => RpcError::internal_error(e.to_string()),
            DaemonError::InvalidParams(msg) => RpcError::invalid_params(msg),
            DaemonError::MethodNotFound(name) => RpcError::new(
                hamr_rpc::protocol::METHOD_NOT_FOUND,
                format!("Method not found: {name}"),
            ),
            DaemonError::Watcher(ref e) => RpcError::internal_error(e.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, DaemonError>;

impl From<RpcError> for DaemonError {
    fn from(err: RpcError) -> Self {
        match err.code {
            hamr_rpc::protocol::NOT_REGISTERED => DaemonError::NotRegistered,
            hamr_rpc::protocol::ALREADY_REGISTERED => DaemonError::AlreadyRegistered,
            hamr_rpc::protocol::PLUGIN_NOT_FOUND => DaemonError::PluginNotFound(err.message),
            hamr_rpc::protocol::UI_OCCUPIED => DaemonError::UiOccupied,
            hamr_rpc::protocol::NOT_ACTIVE_UI => DaemonError::NotActiveUi,
            hamr_rpc::protocol::CONTROL_REQUIRED => DaemonError::ControlRequired,
            hamr_rpc::protocol::INVALID_PARAMS => DaemonError::InvalidParams(err.message),
            hamr_rpc::protocol::METHOD_NOT_FOUND => DaemonError::MethodNotFound(err.message),
            _ => DaemonError::Io(std::io::Error::other(err.message)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hamr_rpc::protocol;

    #[test]
    fn test_daemon_error_display_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = DaemonError::Io(io_err);
        assert!(err.to_string().contains("I/O error"));
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_daemon_error_display_json() {
        let json_str = "invalid json";
        let json_err = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let err = DaemonError::Json(json_err);
        assert!(err.to_string().contains("JSON error"));
    }

    #[test]
    fn test_daemon_error_display_not_registered() {
        let err = DaemonError::NotRegistered;
        assert_eq!(err.to_string(), "Client not registered");
    }

    #[test]
    fn test_daemon_error_display_already_registered() {
        let err = DaemonError::AlreadyRegistered;
        assert_eq!(err.to_string(), "Client already registered");
    }

    #[test]
    fn test_daemon_error_display_plugin_not_found() {
        let err = DaemonError::PluginNotFound("my-plugin".to_string());
        assert_eq!(err.to_string(), "Plugin not found: my-plugin");
    }

    #[test]
    fn test_daemon_error_display_ui_occupied() {
        let err = DaemonError::UiOccupied;
        assert_eq!(err.to_string(), "Another UI client is active");
    }

    #[test]
    fn test_daemon_error_display_not_active_ui() {
        let err = DaemonError::NotActiveUi;
        assert_eq!(err.to_string(), "Not the active UI");
    }

    #[test]
    fn test_daemon_error_display_control_required() {
        let err = DaemonError::ControlRequired;
        assert_eq!(err.to_string(), "Control or UI client required");
    }

    #[test]
    fn test_daemon_error_display_invalid_params() {
        let err = DaemonError::InvalidParams("missing field".to_string());
        assert_eq!(err.to_string(), "Invalid parameters: missing field");
    }

    #[test]
    fn test_daemon_error_display_method_not_found() {
        let err = DaemonError::MethodNotFound("unknown_method".to_string());
        assert_eq!(err.to_string(), "Method not found: unknown_method");
    }

    #[test]
    fn test_daemon_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err: DaemonError = io_err.into();
        assert!(matches!(err, DaemonError::Io(_)));
    }

    #[test]
    fn test_daemon_error_from_json_error() {
        let json_err = serde_json::from_str::<i32>("not a number").unwrap_err();
        let err: DaemonError = json_err.into();
        assert!(matches!(err, DaemonError::Json(_)));
    }

    #[test]
    fn test_daemon_error_to_rpc_error_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let daemon_err = DaemonError::Io(io_err);
        let rpc_err: RpcError = daemon_err.into();
        assert_eq!(rpc_err.code, protocol::INTERNAL_ERROR);
        assert!(rpc_err.message.contains("not found"));
    }

    #[test]
    fn test_daemon_error_to_rpc_error_json() {
        let json_err = serde_json::from_str::<i32>("abc").unwrap_err();
        let daemon_err = DaemonError::Json(json_err);
        let rpc_err: RpcError = daemon_err.into();
        assert_eq!(rpc_err.code, protocol::INTERNAL_ERROR);
    }

    #[test]
    fn test_daemon_error_to_rpc_error_not_registered() {
        let rpc_err: RpcError = DaemonError::NotRegistered.into();
        assert_eq!(rpc_err.code, protocol::NOT_REGISTERED);
        assert!(rpc_err.message.to_lowercase().contains("not registered"));
    }

    #[test]
    fn test_daemon_error_to_rpc_error_already_registered() {
        let rpc_err: RpcError = DaemonError::AlreadyRegistered.into();
        assert_eq!(rpc_err.code, protocol::ALREADY_REGISTERED);
        assert_eq!(rpc_err.message, "Already registered");
    }

    #[test]
    fn test_daemon_error_to_rpc_error_plugin_not_found() {
        let rpc_err: RpcError = DaemonError::PluginNotFound("test-plugin".to_string()).into();
        assert_eq!(rpc_err.code, protocol::PLUGIN_NOT_FOUND);
        assert!(rpc_err.message.contains("test-plugin"));
    }

    #[test]
    fn test_daemon_error_to_rpc_error_ui_occupied() {
        let rpc_err: RpcError = DaemonError::UiOccupied.into();
        assert_eq!(rpc_err.code, protocol::UI_OCCUPIED);
        assert!(rpc_err.message.to_lowercase().contains("ui"));
    }

    #[test]
    fn test_daemon_error_to_rpc_error_not_active_ui() {
        let rpc_err: RpcError = DaemonError::NotActiveUi.into();
        assert_eq!(rpc_err.code, protocol::NOT_ACTIVE_UI);
        assert!(rpc_err.message.to_lowercase().contains("ui"));
    }

    #[test]
    fn test_daemon_error_to_rpc_error_control_required() {
        let rpc_err: RpcError = DaemonError::ControlRequired.into();
        assert_eq!(rpc_err.code, protocol::CONTROL_REQUIRED);
        assert!(rpc_err.message.to_lowercase().contains("control"));
    }

    #[test]
    fn test_daemon_error_to_rpc_error_invalid_params() {
        let rpc_err: RpcError = DaemonError::InvalidParams("bad param".to_string()).into();
        assert_eq!(rpc_err.code, protocol::INVALID_PARAMS);
        assert_eq!(rpc_err.message, "bad param");
    }

    #[test]
    fn test_daemon_error_to_rpc_error_method_not_found() {
        let rpc_err: RpcError = DaemonError::MethodNotFound("foo".to_string()).into();
        assert_eq!(rpc_err.code, protocol::METHOD_NOT_FOUND);
    }

    #[test]
    fn test_rpc_error_to_daemon_error_not_registered() {
        let rpc_err = RpcError::not_registered();
        let daemon_err: DaemonError = rpc_err.into();
        assert!(matches!(daemon_err, DaemonError::NotRegistered));
    }

    #[test]
    fn test_rpc_error_to_daemon_error_already_registered() {
        let rpc_err = RpcError::already_registered();
        let daemon_err: DaemonError = rpc_err.into();
        assert!(matches!(daemon_err, DaemonError::AlreadyRegistered));
    }

    #[test]
    fn test_rpc_error_to_daemon_error_plugin_not_found() {
        let rpc_err = RpcError::plugin_not_found("some-plugin".to_string());
        let daemon_err: DaemonError = rpc_err.into();
        assert!(
            matches!(daemon_err, DaemonError::PluginNotFound(id) if id.contains("some-plugin"))
        );
    }

    #[test]
    fn test_rpc_error_to_daemon_error_ui_occupied() {
        let rpc_err = RpcError::ui_occupied();
        let daemon_err: DaemonError = rpc_err.into();
        assert!(matches!(daemon_err, DaemonError::UiOccupied));
    }

    #[test]
    fn test_rpc_error_to_daemon_error_not_active_ui() {
        let rpc_err = RpcError::not_active_ui();
        let daemon_err: DaemonError = rpc_err.into();
        assert!(matches!(daemon_err, DaemonError::NotActiveUi));
    }

    #[test]
    fn test_rpc_error_to_daemon_error_control_required() {
        let rpc_err = RpcError::control_required();
        let daemon_err: DaemonError = rpc_err.into();
        assert!(matches!(daemon_err, DaemonError::ControlRequired));
    }

    #[test]
    fn test_rpc_error_to_daemon_error_invalid_params() {
        let rpc_err = RpcError::invalid_params("missing key".to_string());
        let daemon_err: DaemonError = rpc_err.into();
        assert!(matches!(daemon_err, DaemonError::InvalidParams(msg) if msg == "missing key"));
    }

    #[test]
    fn test_rpc_error_to_daemon_error_method_not_found() {
        let rpc_err = RpcError::method_not_found();
        let daemon_err: DaemonError = rpc_err.into();
        assert!(matches!(daemon_err, DaemonError::MethodNotFound(_)));
    }

    #[test]
    fn test_rpc_error_to_daemon_error_unknown_code() {
        let rpc_err = RpcError {
            code: 9999,
            message: "unknown error".to_string(),
            data: None,
        };
        let daemon_err: DaemonError = rpc_err.into();
        assert!(matches!(daemon_err, DaemonError::Io(_)));
    }

    #[test]
    fn test_rpc_error_to_daemon_error_internal_error() {
        let rpc_err = RpcError::internal_error("something went wrong".to_string());
        let daemon_err: DaemonError = rpc_err.into();
        assert!(matches!(daemon_err, DaemonError::Io(_)));
    }

    #[test]
    fn test_roundtrip_not_registered() {
        let original = DaemonError::NotRegistered;
        let rpc: RpcError = original.into();
        let back: DaemonError = rpc.into();
        assert!(matches!(back, DaemonError::NotRegistered));
    }

    #[test]
    fn test_roundtrip_already_registered() {
        let original = DaemonError::AlreadyRegistered;
        let rpc: RpcError = original.into();
        let back: DaemonError = rpc.into();
        assert!(matches!(back, DaemonError::AlreadyRegistered));
    }

    #[test]
    fn test_roundtrip_ui_occupied() {
        let original = DaemonError::UiOccupied;
        let rpc: RpcError = original.into();
        let back: DaemonError = rpc.into();
        assert!(matches!(back, DaemonError::UiOccupied));
    }

    #[test]
    fn test_roundtrip_not_active_ui() {
        let original = DaemonError::NotActiveUi;
        let rpc: RpcError = original.into();
        let back: DaemonError = rpc.into();
        assert!(matches!(back, DaemonError::NotActiveUi));
    }

    #[test]
    fn test_roundtrip_control_required() {
        let original = DaemonError::ControlRequired;
        let rpc: RpcError = original.into();
        let back: DaemonError = rpc.into();
        assert!(matches!(back, DaemonError::ControlRequired));
    }

    #[test]
    fn test_roundtrip_plugin_not_found() {
        let original = DaemonError::PluginNotFound("roundtrip-plugin".to_string());
        let rpc: RpcError = original.into();
        let back: DaemonError = rpc.into();
        assert!(matches!(back, DaemonError::PluginNotFound(id) if id.contains("roundtrip-plugin")));
    }

    #[test]
    fn test_daemon_error_debug() {
        let err = DaemonError::NotRegistered;
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("NotRegistered"));
    }

    #[test]
    fn test_daemon_error_display_core() {
        let core_err = hamr_core::Error::Plugin("plugin failed".to_string());
        let err = DaemonError::Core(core_err);
        assert!(err.to_string().contains("Core error"));
        assert!(err.to_string().contains("plugin failed"));
    }

    #[test]
    fn test_daemon_error_display_codec() {
        let codec_err =
            hamr_rpc::transport::CodecError::Json(serde_json::from_str::<i32>("abc").unwrap_err());
        let err = DaemonError::Codec(codec_err);
        assert!(err.to_string().contains("Codec error"));
    }

    #[test]
    fn test_daemon_error_display_watcher() {
        let notify_err = notify::Error::generic("file not found");
        let err = DaemonError::Watcher(notify_err);
        assert!(err.to_string().contains("Watcher error"));
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_daemon_error_from_core_error() {
        let core_err = hamr_core::Error::Config("bad config".to_string());
        let err: DaemonError = core_err.into();
        assert!(matches!(err, DaemonError::Core(_)));
    }

    #[test]
    fn test_daemon_error_from_codec_error() {
        let invalid_bytes: Vec<u8> = vec![0xff];
        let codec_err =
            hamr_rpc::transport::CodecError::Utf8(std::str::from_utf8(&invalid_bytes).unwrap_err());
        let err: DaemonError = codec_err.into();
        assert!(matches!(err, DaemonError::Codec(_)));
    }

    #[test]
    fn test_daemon_error_to_rpc_error_core() {
        let core_err = hamr_core::Error::PluginNotFound("missing".to_string());
        let daemon_err = DaemonError::Core(core_err);
        let rpc_err: RpcError = daemon_err.into();
        assert_eq!(rpc_err.code, protocol::INTERNAL_ERROR);
        assert!(rpc_err.message.contains("missing"));
    }

    #[test]
    fn test_daemon_error_to_rpc_error_codec() {
        let codec_err = hamr_rpc::transport::CodecError::Io(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "pipe broken",
        ));
        let daemon_err = DaemonError::Codec(codec_err);
        let rpc_err: RpcError = daemon_err.into();
        assert_eq!(rpc_err.code, protocol::INTERNAL_ERROR);
        assert!(rpc_err.message.contains("pipe broken"));
    }

    #[test]
    fn test_daemon_error_to_rpc_error_watcher() {
        let notify_err = notify::Error::generic("watch failed");
        let daemon_err = DaemonError::Watcher(notify_err);
        let rpc_err: RpcError = daemon_err.into();
        assert_eq!(rpc_err.code, protocol::INTERNAL_ERROR);
        assert!(rpc_err.message.contains("watch failed"));
    }

    #[test]
    fn test_daemon_error_result_type_alias() {
        // Testing that Result<T> type alias compiles and resolves correctly
        #[allow(clippy::unnecessary_wraps)]
        fn returns_result() -> Result<i32> {
            Ok(42)
        }
        assert_eq!(returns_result().unwrap(), 42);
    }

    #[test]
    fn test_daemon_error_result_type_alias_err() {
        fn returns_err() -> Result<i32> {
            Err(DaemonError::NotRegistered)
        }
        assert!(returns_err().is_err());
    }
}
