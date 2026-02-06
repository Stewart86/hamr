use serde::{Deserialize, Serialize};

/// Plugin manifest (manifest.json)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub name: String,

    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub icon: Option<String>,

    #[serde(default)]
    pub prefix: Option<String>,

    #[serde(default)]
    pub match_pattern: Option<String>,

    #[serde(default, rename = "match")]
    pub match_config: Option<MatchConfig>,

    #[serde(default)]
    pub handler: Option<Handler>,

    #[serde(default)]
    pub daemon: Option<DaemonConfig>,

    #[serde(default)]
    pub frecency: Option<FrecencyMode>,

    #[serde(default)]
    pub static_index: Option<Vec<StaticIndexItem>>,

    #[serde(default)]
    pub input_mode: Option<InputMode>,

    #[serde(default)]
    pub hidden: bool,

    /// Supported platforms for this plugin
    /// Values: "*" (all), "niri", "hyprland", "macos", "windows"
    #[serde(default)]
    pub supported_platforms: Option<Vec<String>>,
}

/// Handler type for plugin communication
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HandlerType {
    /// Standard input/output communication
    #[default]
    Stdio,
    /// Socket-based communication
    Socket,
}

/// Handler configuration for plugin execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Handler {
    /// Type of handler (stdio or socket)
    #[serde(default, rename = "type")]
    pub handler_type: HandlerType,

    /// Path to handler script (for stdio plugins)
    #[serde(default)]
    pub path: Option<String>,

    /// Command to run (for socket plugins)
    #[serde(default)]
    pub command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DaemonConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub background: bool,

    #[serde(default)]
    pub restart_on_crash: bool,

    #[serde(default)]
    pub max_restarts: Option<u32>,
}

/// Frecency tracking mode
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrecencyMode {
    /// Track individual item usage
    #[default]
    Item,
    /// Track plugin usage only
    Plugin,
    /// Don't track frecency
    None,
}

/// Input mode for search
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputMode {
    /// Search on every keystroke
    #[default]
    Realtime,
    /// Search only on submit
    Submit,
}

impl From<InputMode> for hamr_types::InputMode {
    fn from(mode: InputMode) -> Self {
        match mode {
            InputMode::Realtime => hamr_types::InputMode::Realtime,
            InputMode::Submit => hamr_types::InputMode::Submit,
        }
    }
}

/// Match configuration for pattern-based plugin activation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchConfig {
    #[serde(default)]
    pub patterns: Vec<String>,

    #[serde(default)]
    pub priority: Option<i32>,
}

/// Static index item (defined in manifest)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StaticIndexItem {
    pub id: String,
    pub name: String,

    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub icon: Option<String>,

    #[serde(default)]
    pub icon_type: Option<String>,

    #[serde(default)]
    pub keywords: Option<Vec<String>>,

    #[serde(default)]
    pub verb: Option<String>,

    #[serde(default)]
    pub entry_point: Option<serde_json::Value>,
}

impl Manifest {
    /// Check if this is a socket plugin
    #[must_use]
    pub fn is_socket(&self) -> bool {
        self.handler
            .as_ref()
            .is_some_and(|h| matches!(h.handler_type, HandlerType::Socket))
    }

    /// Check if this is a stdio plugin
    #[must_use]
    pub fn is_stdio(&self) -> bool {
        self.handler
            .as_ref()
            .is_none_or(|h| matches!(h.handler_type, HandlerType::Stdio))
    }

    /// Get the handler command for socket plugins
    #[must_use]
    pub fn socket_command(&self) -> Option<&str> {
        self.handler.as_ref().and_then(|h| h.command.as_deref())
    }

    /// Get the handler path for stdio plugins
    #[must_use]
    pub fn handler_path(&self) -> Option<&str> {
        self.handler.as_ref().and_then(|h| h.path.as_deref())
    }

    /// Check if this plugin supports the given platform
    /// Platforms must be explicitly listed - no wildcards supported
    #[must_use]
    pub fn supports_platform(&self, platform: &str) -> bool {
        match &self.supported_platforms {
            None => {
                // No platforms specified - plugin won't load
                // All plugins should explicitly declare their supported platforms
                false
            }
            Some(platforms) => platforms.iter().any(|p| p.eq_ignore_ascii_case(platform)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_manifest(name: &str) -> Manifest {
        Manifest {
            name: name.to_string(),
            description: None,
            icon: None,
            prefix: None,
            match_pattern: None,
            match_config: None,
            handler: None,
            daemon: None,
            frecency: None,
            static_index: None,
            input_mode: None,
            hidden: false,
            supported_platforms: None,
        }
    }

    #[test]
    fn test_is_socket_with_socket_handler() {
        let mut manifest = minimal_manifest("test");
        manifest.handler = Some(Handler {
            handler_type: HandlerType::Socket,
            path: None,
            command: Some("./run.sh".to_string()),
        });
        assert!(manifest.is_socket());
        assert!(!manifest.is_stdio());
    }

    #[test]
    fn test_is_stdio_with_stdio_handler() {
        let mut manifest = minimal_manifest("test");
        manifest.handler = Some(Handler {
            handler_type: HandlerType::Stdio,
            path: Some("handler.py".to_string()),
            command: None,
        });
        assert!(manifest.is_stdio());
        assert!(!manifest.is_socket());
    }

    #[test]
    fn test_is_stdio_with_no_handler() {
        let manifest = minimal_manifest("test");
        assert!(manifest.is_stdio());
        assert!(!manifest.is_socket());
    }

    #[test]
    fn test_socket_command_returns_command() {
        let mut manifest = minimal_manifest("test");
        manifest.handler = Some(Handler {
            handler_type: HandlerType::Socket,
            path: None,
            command: Some("./my-daemon".to_string()),
        });
        assert_eq!(manifest.socket_command(), Some("./my-daemon"));
    }

    #[test]
    fn test_socket_command_returns_none_without_handler() {
        let manifest = minimal_manifest("test");
        assert_eq!(manifest.socket_command(), None);
    }

    #[test]
    fn test_socket_command_returns_none_without_command() {
        let mut manifest = minimal_manifest("test");
        manifest.handler = Some(Handler {
            handler_type: HandlerType::Socket,
            path: None,
            command: None,
        });
        assert_eq!(manifest.socket_command(), None);
    }

    #[test]
    fn test_handler_path_returns_path() {
        let mut manifest = minimal_manifest("test");
        manifest.handler = Some(Handler {
            handler_type: HandlerType::Stdio,
            path: Some("handler.py".to_string()),
            command: None,
        });
        assert_eq!(manifest.handler_path(), Some("handler.py"));
    }

    #[test]
    fn test_handler_path_returns_none_without_handler() {
        let manifest = minimal_manifest("test");
        assert_eq!(manifest.handler_path(), None);
    }

    #[test]
    fn test_handler_path_returns_none_without_path() {
        let mut manifest = minimal_manifest("test");
        manifest.handler = Some(Handler {
            handler_type: HandlerType::Stdio,
            path: None,
            command: None,
        });
        assert_eq!(manifest.handler_path(), None);
    }

    #[test]
    fn test_supports_platform_with_matching_platform() {
        let mut manifest = minimal_manifest("test");
        manifest.supported_platforms = Some(vec!["niri".to_string(), "hyprland".to_string()]);
        assert!(manifest.supports_platform("niri"));
        assert!(manifest.supports_platform("hyprland"));
    }

    #[test]
    fn test_supports_platform_case_insensitive() {
        let mut manifest = minimal_manifest("test");
        manifest.supported_platforms = Some(vec!["Niri".to_string()]);
        assert!(manifest.supports_platform("niri"));
        assert!(manifest.supports_platform("NIRI"));
        assert!(manifest.supports_platform("Niri"));
    }

    #[test]
    fn test_supports_platform_returns_false_for_unmatched() {
        let mut manifest = minimal_manifest("test");
        manifest.supported_platforms = Some(vec!["niri".to_string()]);
        assert!(!manifest.supports_platform("hyprland"));
        assert!(!manifest.supports_platform("macos"));
    }

    #[test]
    fn test_supports_platform_returns_false_when_none() {
        let manifest = minimal_manifest("test");
        assert!(!manifest.supports_platform("niri"));
        assert!(!manifest.supports_platform("any"));
    }

    #[test]
    fn test_supports_platform_with_empty_list() {
        let mut manifest = minimal_manifest("test");
        manifest.supported_platforms = Some(vec![]);
        assert!(!manifest.supports_platform("niri"));
    }

    #[test]
    fn test_manifest_deserialize_minimal() {
        let json = r#"{"name": "test-plugin"}"#;
        let manifest: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.name, "test-plugin");
        assert!(manifest.description.is_none());
        assert!(manifest.handler.is_none());
        assert!(!manifest.hidden);
    }

    #[test]
    fn test_manifest_deserialize_full() {
        let json = r#"{
            "name": "my-plugin",
            "description": "A test plugin",
            "icon": "plugin-icon",
            "prefix": "@",
            "handler": {
                "type": "socket",
                "command": "./run.sh"
            },
            "daemon": {
                "enabled": true,
                "background": true,
                "restartOnCrash": true,
                "maxRestarts": 5
            },
            "frecency": "plugin",
            "inputMode": "submit",
            "hidden": true,
            "supportedPlatforms": ["niri", "hyprland"]
        }"#;
        let manifest: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.name, "my-plugin");
        assert_eq!(manifest.description, Some("A test plugin".to_string()));
        assert_eq!(manifest.icon, Some("plugin-icon".to_string()));
        assert_eq!(manifest.prefix, Some("@".to_string()));
        assert!(manifest.is_socket());
        assert!(manifest.daemon.as_ref().unwrap().enabled);
        assert!(manifest.daemon.as_ref().unwrap().background);
        assert!(manifest.daemon.as_ref().unwrap().restart_on_crash);
        assert_eq!(manifest.daemon.as_ref().unwrap().max_restarts, Some(5));
        assert!(matches!(manifest.frecency, Some(FrecencyMode::Plugin)));
        assert!(matches!(manifest.input_mode, Some(InputMode::Submit)));
        assert!(manifest.hidden);
        assert!(manifest.supports_platform("niri"));
    }

    #[test]
    fn test_handler_type_default_is_stdio() {
        let handler_type = HandlerType::default();
        assert!(matches!(handler_type, HandlerType::Stdio));
    }

    #[test]
    fn test_frecency_mode_default_is_item() {
        let mode = FrecencyMode::default();
        assert!(matches!(mode, FrecencyMode::Item));
    }

    #[test]
    fn test_input_mode_default_is_realtime() {
        let mode = InputMode::default();
        assert!(matches!(mode, InputMode::Realtime));
    }

    #[test]
    fn test_handler_deserialize_stdio() {
        let json = r#"{"type": "stdio", "path": "handler.py"}"#;
        let handler: Handler = serde_json::from_str(json).unwrap();
        assert!(matches!(handler.handler_type, HandlerType::Stdio));
        assert_eq!(handler.path, Some("handler.py".to_string()));
    }

    #[test]
    fn test_handler_deserialize_socket() {
        let json = r#"{"type": "socket", "command": "./daemon"}"#;
        let handler: Handler = serde_json::from_str(json).unwrap();
        assert!(matches!(handler.handler_type, HandlerType::Socket));
        assert_eq!(handler.command, Some("./daemon".to_string()));
    }

    #[test]
    fn test_daemon_config_deserialize() {
        let json = r#"{
            "enabled": true,
            "background": false,
            "restartOnCrash": true,
            "maxRestarts": 3
        }"#;
        let config: DaemonConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert!(!config.background);
        assert!(config.restart_on_crash);
        assert_eq!(config.max_restarts, Some(3));
    }

    #[test]
    fn test_daemon_config_defaults() {
        let json = r"{}";
        let config: DaemonConfig = serde_json::from_str(json).unwrap();
        assert!(!config.enabled);
        assert!(!config.background);
        assert!(!config.restart_on_crash);
        assert!(config.max_restarts.is_none());
    }

    #[test]
    fn test_match_config_deserialize() {
        let json = r#"{
            "patterns": ["http://", "https://"],
            "priority": 100
        }"#;
        let config: MatchConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.patterns.len(), 2);
        assert_eq!(config.patterns[0], "http://");
        assert_eq!(config.priority, Some(100));
    }

    #[test]
    fn test_static_index_item_deserialize() {
        let json = r#"{
            "id": "item1",
            "name": "Test Item",
            "description": "A test item",
            "icon": "test-icon",
            "iconType": "file",
            "keywords": ["test", "item"],
            "verb": "Open",
            "entryPoint": {"action": "open"}
        }"#;
        let item: StaticIndexItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.id, "item1");
        assert_eq!(item.name, "Test Item");
        assert_eq!(item.description, Some("A test item".to_string()));
        assert_eq!(item.icon, Some("test-icon".to_string()));
        assert_eq!(item.icon_type, Some("file".to_string()));
        assert_eq!(
            item.keywords,
            Some(vec!["test".to_string(), "item".to_string()])
        );
        assert_eq!(item.verb, Some("Open".to_string()));
        assert!(item.entry_point.is_some());
    }

    #[test]
    fn test_static_index_item_minimal() {
        let json = r#"{"id": "x", "name": "X"}"#;
        let item: StaticIndexItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.id, "x");
        assert_eq!(item.name, "X");
        assert!(item.description.is_none());
        assert!(item.keywords.is_none());
    }

    #[test]
    fn test_frecency_mode_deserialize_all_variants() {
        assert!(matches!(
            serde_json::from_str::<FrecencyMode>(r#""item""#).unwrap(),
            FrecencyMode::Item
        ));
        assert!(matches!(
            serde_json::from_str::<FrecencyMode>(r#""plugin""#).unwrap(),
            FrecencyMode::Plugin
        ));
        assert!(matches!(
            serde_json::from_str::<FrecencyMode>(r#""none""#).unwrap(),
            FrecencyMode::None
        ));
    }

    #[test]
    fn test_input_mode_deserialize_all_variants() {
        assert!(matches!(
            serde_json::from_str::<InputMode>(r#""realtime""#).unwrap(),
            InputMode::Realtime
        ));
        assert!(matches!(
            serde_json::from_str::<InputMode>(r#""submit""#).unwrap(),
            InputMode::Submit
        ));
    }
}
