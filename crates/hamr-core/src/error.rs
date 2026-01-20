use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    #[error("No active plugin")]
    NoActivePlugin,

    #[error("Process error: {0}")]
    Process(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = Error::Io(io_err);
        assert!(err.to_string().contains("IO error"));
        assert!(err.to_string().contains("file missing"));
    }

    #[test]
    fn test_error_display_json() {
        let json_err = serde_json::from_str::<String>("not valid json").unwrap_err();
        let err = Error::Json(json_err);
        assert!(err.to_string().contains("JSON error"));
    }

    #[test]
    fn test_error_display_plugin() {
        let err = Error::Plugin("handler crashed".to_string());
        assert_eq!(err.to_string(), "Plugin error: handler crashed");
    }

    #[test]
    fn test_error_display_config() {
        let err = Error::Config("missing field".to_string());
        assert_eq!(err.to_string(), "Config error: missing field");
    }

    #[test]
    fn test_error_display_plugin_not_found() {
        let err = Error::PluginNotFound("calculator".to_string());
        assert_eq!(err.to_string(), "Plugin not found: calculator");
    }

    #[test]
    fn test_error_display_no_active_plugin() {
        let err = Error::NoActivePlugin;
        assert_eq!(err.to_string(), "No active plugin");
    }

    #[test]
    fn test_error_display_process() {
        let err = Error::Process("spawn failed".to_string());
        assert_eq!(err.to_string(), "Process error: spawn failed");
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
        assert!(err.to_string().contains("access denied"));
    }

    #[test]
    fn test_from_json_error() {
        let json_err = serde_json::from_str::<i32>("\"not a number\"").unwrap_err();
        let err: Error = json_err.into();
        assert!(matches!(err, Error::Json(_)));
    }

    #[test]
    fn test_result_type_alias() {
        fn returns_error() -> Result<()> {
            Err(Error::NoActivePlugin)
        }
        assert!(returns_error().is_err());
    }

    #[test]
    fn test_error_debug_format() {
        let err = Error::Plugin("test".to_string());
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("Plugin"));
        assert!(debug_str.contains("test"));
    }
}
