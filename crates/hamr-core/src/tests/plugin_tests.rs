//! Tests for plugin manifest parsing and matching
//!
//! Tests the plugin system including:
//! - Manifest parsing (all fields)
//! - Prefix matching
//! - Pattern matching (regex)
//! - Daemon configuration
//! - Static index items
//! - Frecency modes

use crate::plugin::{DaemonConfig, FrecencyMode, HandlerType, InputMode, Manifest, Plugin};
use tempfile::TempDir;

#[test]
fn test_manifest_parse_minimal() {
    let json = r#"{"name": "Test Plugin"}"#;
    let manifest: Manifest = serde_json::from_str(json).unwrap();

    assert_eq!(manifest.name, "Test Plugin");
    assert!(manifest.description.is_none());
    assert!(manifest.icon.is_none());
    assert!(manifest.prefix.is_none());
    assert!(manifest.daemon.is_none());
    assert!(!manifest.hidden);
}

#[test]
fn test_manifest_parse_full() {
    let json = r#"{
        "name": "Calculator",
        "description": "Quick math calculations",
        "icon": "calculate",
        "prefix": "=",
        "daemon": {
            "enabled": true,
            "background": true,
            "restartOnCrash": true,
            "maxRestarts": 3
        },
        "frecency": "item",
        "inputMode": "realtime",
        "hidden": false
    }"#;

    let manifest: Manifest = serde_json::from_str(json).unwrap();

    assert_eq!(manifest.name, "Calculator");
    assert_eq!(
        manifest.description,
        Some("Quick math calculations".to_string())
    );
    assert_eq!(manifest.icon, Some("calculate".to_string()));
    assert_eq!(manifest.prefix, Some("=".to_string()));

    let daemon = manifest.daemon.unwrap();
    assert!(daemon.enabled);
    assert!(daemon.background);
    assert!(daemon.restart_on_crash);
    assert_eq!(daemon.max_restarts, Some(3));
}

#[test]
fn test_manifest_parse_match_pattern() {
    let json = r#"{
        "name": "URL Handler",
        "matchPattern": "^https?://"
    }"#;

    let manifest: Manifest = serde_json::from_str(json).unwrap();
    assert_eq!(manifest.match_pattern, Some("^https?://".to_string()));
}

#[test]
fn test_manifest_parse_match_config() {
    let json = r#"{
        "name": "Calculator",
        "match": {
            "patterns": ["^=", "^[\\d\\.]+\\s*[\\+\\-\\*\\/]"],
            "priority": 100
        }
    }"#;

    let manifest: Manifest = serde_json::from_str(json).unwrap();
    let match_config = manifest.match_config.unwrap();

    assert_eq!(match_config.patterns.len(), 2);
    assert!(match_config.patterns[0].contains('='));
    assert_eq!(match_config.priority, Some(100));
}

#[test]
fn test_manifest_parse_static_index() {
    let json = r#"{
        "name": "Power",
        "staticIndex": [
            {
                "id": "shutdown",
                "name": "Shutdown",
                "description": "Power off the system",
                "icon": "power_settings_new",
                "keywords": ["power", "off"]
            },
            {
                "id": "reboot",
                "name": "Reboot",
                "icon": "restart_alt",
                "entryPoint": {"step": "action", "selected": {"id": "reboot"}}
            }
        ]
    }"#;

    let manifest: Manifest = serde_json::from_str(json).unwrap();
    let static_index = manifest.static_index.unwrap();

    assert_eq!(static_index.len(), 2);

    assert_eq!(static_index[0].id, "shutdown");
    assert_eq!(static_index[0].name, "Shutdown");
    assert_eq!(
        static_index[0].description,
        Some("Power off the system".to_string())
    );
    let keywords = static_index[0].keywords.as_ref().unwrap();
    assert!(keywords.contains(&"power".to_string()));

    assert_eq!(static_index[1].id, "reboot");
    assert!(static_index[1].entry_point.is_some());
}

#[test]
fn test_manifest_parse_frecency_modes() {
    let json_item = r#"{"name": "Test", "frecency": "item"}"#;
    let manifest: Manifest = serde_json::from_str(json_item).unwrap();
    assert!(matches!(manifest.frecency, Some(FrecencyMode::Item)));

    let json_plugin = r#"{"name": "Test", "frecency": "plugin"}"#;
    let manifest: Manifest = serde_json::from_str(json_plugin).unwrap();
    assert!(matches!(manifest.frecency, Some(FrecencyMode::Plugin)));

    let json_none = r#"{"name": "Test", "frecency": "none"}"#;
    let manifest: Manifest = serde_json::from_str(json_none).unwrap();
    assert!(matches!(manifest.frecency, Some(FrecencyMode::None)));
}

#[test]
fn test_manifest_parse_input_modes() {
    let json_realtime = r#"{"name": "Test", "inputMode": "realtime"}"#;
    let manifest: Manifest = serde_json::from_str(json_realtime).unwrap();
    assert!(matches!(manifest.input_mode, Some(InputMode::Realtime)));

    let json_submit = r#"{"name": "Test", "inputMode": "submit"}"#;
    let manifest: Manifest = serde_json::from_str(json_submit).unwrap();
    assert!(matches!(manifest.input_mode, Some(InputMode::Submit)));
}

#[test]
fn test_manifest_hidden_plugin() {
    let json = r#"{"name": "Internal", "hidden": true}"#;
    let manifest: Manifest = serde_json::from_str(json).unwrap();
    assert!(manifest.hidden);
}

fn create_test_plugin(
    dir: &TempDir,
    name: &str,
    manifest_json: &str,
    has_handler: bool,
) -> std::path::PathBuf {
    let plugin_dir = dir.path().join(name);
    std::fs::create_dir_all(&plugin_dir).unwrap();

    let manifest_path = plugin_dir.join("manifest.json");
    std::fs::write(&manifest_path, manifest_json).unwrap();

    if has_handler {
        let handler_path = plugin_dir.join("handler.py");
        std::fs::write(&handler_path, "# test handler").unwrap();
    }

    plugin_dir
}

#[test]
fn test_plugin_load_basic() {
    let temp_dir = TempDir::new().unwrap();
    let manifest = r#"{"name": "Test Plugin"}"#;
    let plugin_path = create_test_plugin(&temp_dir, "test", manifest, true);

    let plugin = Plugin::load(plugin_path).unwrap();

    assert_eq!(plugin.id, "test");
    assert_eq!(plugin.manifest.name, "Test Plugin");
}

#[test]
fn test_plugin_load_missing_manifest() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("broken");
    std::fs::create_dir_all(&plugin_dir).unwrap();

    let result = Plugin::load(plugin_dir);
    assert!(result.is_err());
}

#[test]
fn test_plugin_load_invalid_manifest() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("broken");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    std::fs::write(plugin_dir.join("manifest.json"), "not valid json").unwrap();

    let result = Plugin::load(plugin_dir);
    assert!(result.is_err());
}

#[test]
fn test_plugin_load_missing_handler_without_static_index() {
    let temp_dir = TempDir::new().unwrap();
    let manifest = r#"{"name": "No Handler"}"#;
    let plugin_path = create_test_plugin(&temp_dir, "nohandler", manifest, false);

    let result = Plugin::load(plugin_path);
    assert!(
        result.is_err(),
        "Should require handler.py without staticIndex"
    );
}

#[test]
fn test_plugin_load_static_index_no_handler() {
    let temp_dir = TempDir::new().unwrap();
    let manifest = r#"{
        "name": "Static Only",
        "staticIndex": [{"id": "test", "name": "Test Item"}]
    }"#;
    let plugin_path = create_test_plugin(&temp_dir, "static", manifest, false);

    let plugin = Plugin::load(plugin_path).unwrap();
    assert_eq!(plugin.id, "static");
}

#[test]
fn test_plugin_matches_prefix() {
    let temp_dir = TempDir::new().unwrap();
    let manifest = r#"{"name": "Calculator", "prefix": "="}"#;
    let plugin_path = create_test_plugin(&temp_dir, "calc", manifest, true);

    let plugin = Plugin::load(plugin_path).unwrap();

    let result = plugin.matches_query("=5+5");
    assert_eq!(
        result,
        Some("5+5".to_string()),
        "Should return query without prefix"
    );

    let result = plugin.matches_query("5+5");
    assert!(result.is_none(), "Should not match without prefix");
}

#[test]
fn test_plugin_matches_pattern() {
    let temp_dir = TempDir::new().unwrap();
    let manifest = r#"{"name": "URL", "matchPattern": "^https?://"}"#;
    let plugin_path = create_test_plugin(&temp_dir, "url", manifest, true);

    let plugin = Plugin::load(plugin_path).unwrap();

    let result = plugin.matches_query("http://example.com");
    assert_eq!(result, Some("http://example.com".to_string()));

    let result = plugin.matches_query("https://example.com");
    assert_eq!(result, Some("https://example.com".to_string()));

    let result = plugin.matches_query("ftp://example.com");
    assert!(result.is_none());
}

#[test]
fn test_plugin_matches_pattern_array() {
    let temp_dir = TempDir::new().unwrap();
    let manifest = r#"{
        "name": "Calculator",
        "match": {
            "patterns": ["^=", "^[0-9]"]
        }
    }"#;
    let plugin_path = create_test_plugin(&temp_dir, "calc", manifest, true);

    let plugin = Plugin::load(plugin_path).unwrap();

    assert!(plugin.matches_query("=5+5").is_some());
    assert!(plugin.matches_query("5+5").is_some());
    assert!(plugin.matches_query("hello").is_none());
}

#[test]
fn test_plugin_matches_invalid_regex() {
    let temp_dir = TempDir::new().unwrap();
    let manifest = r#"{"name": "Bad", "matchPattern": "[invalid"}"#;
    let plugin_path = create_test_plugin(&temp_dir, "bad", manifest, true);

    let plugin = Plugin::load(plugin_path).unwrap();

    let result = plugin.matches_query("test");
    assert!(result.is_none());
}

#[test]
fn test_plugin_is_daemon() {
    let temp_dir = TempDir::new().unwrap();

    let manifest1 = r#"{"name": "Normal"}"#;
    let path1 = create_test_plugin(&temp_dir, "normal", manifest1, true);
    let plugin1 = Plugin::load(path1).unwrap();
    assert!(!plugin1.is_daemon());

    let manifest2 = r#"{"name": "Disabled Daemon", "daemon": {"enabled": false}}"#;
    let path2 = create_test_plugin(&temp_dir, "disabled", manifest2, true);
    let plugin2 = Plugin::load(path2).unwrap();
    assert!(!plugin2.is_daemon());

    let manifest3 = r#"{"name": "Enabled Daemon", "daemon": {"enabled": true}}"#;
    let path3 = create_test_plugin(&temp_dir, "enabled", manifest3, true);
    let plugin3 = Plugin::load(path3).unwrap();
    assert!(plugin3.is_daemon());
}

#[test]
fn test_plugin_is_background_daemon() {
    let temp_dir = TempDir::new().unwrap();

    let manifest1 = r#"{"name": "Foreground", "daemon": {"enabled": true, "background": false}}"#;
    let path1 = create_test_plugin(&temp_dir, "fg", manifest1, true);
    let plugin1 = Plugin::load(path1).unwrap();
    assert!(!plugin1.is_background_daemon());

    let manifest2 = r#"{"name": "Background", "daemon": {"enabled": true, "background": true}}"#;
    let path2 = create_test_plugin(&temp_dir, "bg", manifest2, true);
    let plugin2 = Plugin::load(path2).unwrap();
    assert!(plugin2.is_background_daemon());
}

#[test]
fn test_plugin_has_index() {
    let temp_dir = TempDir::new().unwrap();

    let manifest1 = r#"{"name": "No Index"}"#;
    let path1 = create_test_plugin(&temp_dir, "noindex", manifest1, true);
    let plugin1 = Plugin::load(path1).unwrap();
    assert!(!plugin1.has_index());

    let manifest2 = r#"{"name": "Static", "staticIndex": [{"id": "x", "name": "X"}]}"#;
    let path2 = create_test_plugin(&temp_dir, "static", manifest2, false);
    let plugin2 = Plugin::load(path2).unwrap();
    assert!(plugin2.has_index());

    let manifest3 = r#"{"name": "Daemon", "daemon": {"enabled": true}}"#;
    let path3 = create_test_plugin(&temp_dir, "daemon", manifest3, true);
    let plugin3 = Plugin::load(path3).unwrap();
    assert!(plugin3.has_index());
}

#[test]
fn test_manifest_parse_handler_socket() {
    let json = r#"{
        "name": "Timer",
        "icon": "timer",
        "handler": {
            "type": "socket",
            "command": "python handler.py"
        }
    }"#;

    let manifest: Manifest = serde_json::from_str(json).unwrap();

    assert_eq!(manifest.name, "Timer");
    assert_eq!(manifest.icon, Some("timer".to_string()));

    let handler = manifest.handler.as_ref().unwrap();
    assert!(matches!(handler.handler_type, HandlerType::Socket));
    assert_eq!(handler.command, Some("python handler.py".to_string()));
    assert!(handler.path.is_none());
}

#[test]
fn test_manifest_parse_handler_stdio() {
    let json = r#"{
        "name": "Calculator",
        "handler": {
            "type": "stdio",
            "path": "handler.py"
        }
    }"#;

    let manifest: Manifest = serde_json::from_str(json).unwrap();

    assert_eq!(manifest.name, "Calculator");

    let handler = manifest.handler.as_ref().unwrap();
    assert!(matches!(handler.handler_type, HandlerType::Stdio));
    assert_eq!(handler.path, Some("handler.py".to_string()));
    assert!(handler.command.is_none());
}

#[test]
fn test_manifest_parse_handler_default_type() {
    let json = r#"{
        "name": "Test",
        "handler": {
            "path": "handler.py"
        }
    }"#;

    let manifest: Manifest = serde_json::from_str(json).unwrap();

    let handler = manifest.handler.as_ref().unwrap();
    assert!(matches!(handler.handler_type, HandlerType::Stdio));
}

#[test]
fn test_manifest_is_socket() {
    let json_socket = r#"{"name": "Socket", "handler": {"type": "socket", "command": "cmd"}}"#;
    let manifest_socket: Manifest = serde_json::from_str(json_socket).unwrap();
    assert!(manifest_socket.is_socket());
    assert!(!manifest_socket.is_stdio());

    let json_stdio = r#"{"name": "Stdio", "handler": {"type": "stdio"}}"#;
    let manifest_stdio: Manifest = serde_json::from_str(json_stdio).unwrap();
    assert!(!manifest_stdio.is_socket());
    assert!(manifest_stdio.is_stdio());

    let json_no_handler = r#"{"name": "None"}"#;
    let manifest_no_handler: Manifest = serde_json::from_str(json_no_handler).unwrap();
    assert!(!manifest_no_handler.is_socket());
    assert!(manifest_no_handler.is_stdio());
}

#[test]
fn test_manifest_socket_command() {
    let json = r#"{
        "name": "Timer",
        "handler": {
            "type": "socket",
            "command": "python timer.py --port 5000"
        }
    }"#;

    let manifest: Manifest = serde_json::from_str(json).unwrap();
    assert_eq!(
        manifest.socket_command(),
        Some("python timer.py --port 5000")
    );
}

#[test]
fn test_manifest_socket_command_none() {
    let json = r#"{"name": "Test", "handler": {"type": "socket"}}"#;
    let manifest: Manifest = serde_json::from_str(json).unwrap();
    assert!(manifest.socket_command().is_none());
}

#[test]
fn test_manifest_handler_path() {
    let json = r#"{
        "name": "Calculator",
        "handler": {
            "path": "/usr/lib/plugins/calc.py"
        }
    }"#;

    let manifest: Manifest = serde_json::from_str(json).unwrap();
    assert_eq!(manifest.handler_path(), Some("/usr/lib/plugins/calc.py"));
}

#[test]
fn test_manifest_handler_path_none() {
    let json = r#"{"name": "Test", "handler": {"type": "stdio"}}"#;
    let manifest: Manifest = serde_json::from_str(json).unwrap();
    assert!(manifest.handler_path().is_none());
}

#[test]
fn test_daemon_config_defaults() {
    let json = r#"{"enabled": true}"#;
    let config: DaemonConfig = serde_json::from_str(json).unwrap();

    assert!(config.enabled);
    assert!(!config.background);
    assert!(!config.restart_on_crash);
    assert!(config.max_restarts.is_none());
}

#[test]
fn test_daemon_config_full() {
    let json = r#"{
        "enabled": true,
        "background": true,
        "restartOnCrash": true,
        "maxRestarts": 5
    }"#;
    let config: DaemonConfig = serde_json::from_str(json).unwrap();

    assert!(config.enabled);
    assert!(config.background);
    assert!(config.restart_on_crash);
    assert_eq!(config.max_restarts, Some(5));
}
