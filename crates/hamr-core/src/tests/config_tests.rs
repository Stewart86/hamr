//! Tests for configuration loading and defaults
//!
//! Tests the config system including:
//! - Config defaults
//! - Config serialization/deserialization
//! - Search config options
//! - Save/load round trips
//! - Directories management
//! - Action bar hints

use crate::config::{AppConfig, Config, SearchConfig};
use tempfile::{NamedTempFile, TempDir};

#[test]
fn test_config_default() {
    let config = Config::default();

    assert_eq!(config.search.max_displayed_results, 16);
    assert_eq!(config.search.max_recent_items, 20);
    assert_eq!(config.search.max_results_per_plugin, 0);
    assert_eq!(config.search.plugin_debounce_ms, 150);
    assert!((config.search.diversity_decay - 0.7).abs() < 0.001);

    assert_eq!(config.search.prefix.plugins, "/");
    assert_eq!(config.search.prefix.app, "@");
    assert_eq!(config.search.prefix.emojis, ":");
    assert_eq!(config.search.prefix.math, "=");
    assert_eq!(config.search.prefix.shell_command, "!");
    assert_eq!(config.search.prefix.web_search, "?");

    assert!(config.apps.terminal.is_none());
    assert!(config.apps.file_manager.is_none());
    assert!(config.apps.browser.is_none());
}

#[test]
fn test_search_config_default() {
    let config = SearchConfig::default();

    assert_eq!(config.max_displayed_results, 16);
    assert!(!config.engine_base_url.is_empty());
    assert!(config.excluded_sites.is_empty());
}

#[test]
fn test_config_parse_minimal() {
    let json = r"{}";
    let config: Config = serde_json::from_str(json).unwrap();

    assert_eq!(config.search.max_displayed_results, 16);
}

#[test]
fn test_config_missing_fields() {
    let json = r#"{
        "search": {
            "maxDisplayedResults": 32
        }
    }"#;
    let config: Config = serde_json::from_str(json).unwrap();

    assert_eq!(config.search.max_displayed_results, 32);
    assert_eq!(config.search.max_recent_items, 20);
}

#[test]
fn test_config_load_nonexistent_returns_default() {
    let path = std::path::Path::new("/nonexistent/path/config.json");
    let config = Config::load(path).unwrap();
    assert_eq!(config.search.max_displayed_results, 16);
}

#[test]
fn test_config_load_valid_file() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"{{"search": {{"maxDisplayedResults": 25, "maxRecentItems": 15}}}}"#
    )
    .unwrap();

    let config = Config::load(file.path()).unwrap();
    assert_eq!(config.search.max_displayed_results, 25);
    assert_eq!(config.search.max_recent_items, 15);
}

#[test]
fn test_config_load_invalid_json() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "{{invalid json}}").unwrap();

    let result = Config::load(file.path());
    assert!(result.is_err());
}

#[test]
fn test_config_save_and_load_roundtrip() {
    let mut config = Config::default();
    config.search.max_displayed_results = 42;
    config.search.engine_base_url = "https://duckduckgo.com/?q=".to_string();

    let file = NamedTempFile::new().unwrap();
    config.save(file.path()).unwrap();

    let loaded = Config::load(file.path()).unwrap();
    assert_eq!(loaded.search.max_displayed_results, 42);
    assert_eq!(loaded.search.engine_base_url, "https://duckduckgo.com/?=");
}

#[test]
fn test_action_bar_hints_default() {
    let config = Config::default();
    assert_eq!(config.action_bar_hints().len(), 5);
}

#[test]
fn test_action_bar_hints_parsing() {
    let json = r#"{
        "search": {
            "actionBarHints": [
                {"prefix": "/", "plugin": "plugins"},
                {"prefix": "@", "plugin": "apps", "description": "Search apps"},
                {"prefix": ":", "plugin": "emoji", "description": "Insert emoji"}
            ]
        }
    }"#;

    let config: Config = serde_json::from_str(json).unwrap();

    assert_eq!(config.action_bar_hints().len(), 3);

    assert_eq!(config.action_bar_hints()[0].prefix, "/");
    assert_eq!(config.action_bar_hints()[0].plugin, "plugins");
    assert!(config.action_bar_hints()[0].description.is_none());

    assert_eq!(config.action_bar_hints()[1].prefix, "@");
    assert_eq!(config.action_bar_hints()[1].plugin, "apps");
    assert_eq!(
        config.action_bar_hints()[1].description,
        Some("Search apps".to_string())
    );
}

#[test]
fn test_action_bar_hints_json_legacy_format() {
    let json = r#"{
        "search": {
            "actionBarHintsJson": "[{\"prefix\": \";\", \"plugin\": \"clipboard\"}, {\"prefix\": \"~\", \"plugin\": \"files\"}]"
        }
    }"#;

    let config: Config = serde_json::from_str(json).unwrap();

    assert_eq!(config.action_bar_hints().len(), 2);
    assert_eq!(config.action_bar_hints()[0].prefix, ";");
    assert_eq!(config.action_bar_hints()[0].plugin, "clipboard");
    assert_eq!(config.action_bar_hints()[1].prefix, "~");
    assert_eq!(config.action_bar_hints()[1].plugin, "files");
}

#[test]
fn test_action_bar_hints_null_uses_defaults() {
    let json = r#"{"search": {"actionBarHints": null}}"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.action_bar_hints().len(), 5);
}

#[test]
fn test_action_bar_hints_empty_array_stays_empty() {
    let json = r#"{"search": {"actionBarHints": []}}"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert!(config.action_bar_hints().is_empty());
}

#[test]
fn test_search_config_default_prefixes() {
    let config = SearchConfig::default();
    assert_eq!(config.prefix.plugins, "/");
    assert_eq!(config.prefix.app, "@");
    assert_eq!(config.prefix.emojis, ":");
    assert_eq!(config.prefix.math, "=");
    assert_eq!(config.prefix.shell_command, "!");
    assert_eq!(config.prefix.web_search, "?");
    assert_eq!(config.engine_base_url, "https://www.google.com/search?q=");
}

#[test]
fn test_search_prefixes_deserialize() {
    let json = r##"{
        "plugins": "//",
        "app": "#",
        "emojis": "::",
        "math": "calc ",
        "shellCommand": "$",
        "webSearch": "g "
    }"##;
    let prefixes: crate::config::SearchPrefixes = serde_json::from_str(json).unwrap();
    assert_eq!(prefixes.plugins, "//");
    assert_eq!(prefixes.app, "#");
    assert_eq!(prefixes.emojis, "::");
    assert_eq!(prefixes.math, "calc ");
    assert_eq!(prefixes.shell_command, "$");
    assert_eq!(prefixes.web_search, "g ");
}

#[test]
fn test_app_config_default() {
    let config = AppConfig::default();
    assert!(config.terminal.is_none());
    assert!(config.file_manager.is_none());
    assert!(config.browser.is_none());
}

#[test]
fn test_app_config_deserialize() {
    let json = r#"{
        "terminal": "alacritty",
        "fileManager": "nautilus",
        "browser": "firefox"
    }"#;
    let config: AppConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.terminal, Some("alacritty".to_string()));
    assert_eq!(config.file_manager, Some("nautilus".to_string()));
    assert_eq!(config.browser, Some("firefox".to_string()));
}

#[test]
fn test_action_bar_hint_deserialize() {
    let json = r#"{
        "prefix": ";",
        "plugin": "clipboard",
        "label": "Clipboard",
        "icon": "clipboard-icon",
        "description": "Access clipboard history"
    }"#;
    let hint: crate::config::ActionBarHint = serde_json::from_str(json).unwrap();
    assert_eq!(hint.prefix, ";");
    assert_eq!(hint.plugin, "clipboard");
    assert_eq!(hint.label, Some("Clipboard".to_string()));
    assert_eq!(hint.icon, Some("clipboard-icon".to_string()));
    assert_eq!(
        hint.description,
        Some("Access clipboard history".to_string())
    );
}

#[test]
fn test_action_bar_hint_minimal() {
    let json = r#"{"prefix": "!", "plugin": "shell"}"#;
    let hint: crate::config::ActionBarHint = serde_json::from_str(json).unwrap();
    assert_eq!(hint.prefix, "!");
    assert_eq!(hint.plugin, "shell");
    assert!(hint.label.is_none());
    assert!(hint.icon.is_none());
    assert!(hint.description.is_none());
}

#[test]
fn test_engine_url() {
    let json = r#"{
        "search": {
            "engineBaseUrl": "https://duckduckgo.com/?q="
        }
    }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert!(config.search.engine_base_url.contains("duckduckgo"));
}

#[test]
fn test_config_multiple_excluded_sites() {
    let json = r#"{"search": {"excludedSites": ["reddit.com", "pinterest.com", "quora.com", "facebook.com"]}}"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.search.excluded_sites.len(), 4);
}

#[test]
fn test_config_plugin_ranking_bonus() {
    let json =
        r#"{"search": {"pluginRankingBonus": {"apps": 200, "settings": 150, "clipboard": 50}}}"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.search.plugin_ranking_bonus.len(), 3);
    assert_eq!(config.search.plugin_ranking_bonus.get("apps"), Some(&200.0));
    assert_eq!(
        config.search.plugin_ranking_bonus.get("settings"),
        Some(&150.0)
    );
    assert_eq!(
        config.search.plugin_ranking_bonus.get("clipboard"),
        Some(&50.0)
    );
    assert_eq!(config.search.plugin_ranking_bonus.get("unknown"), None);
}

#[test]
fn test_config_empty_json() {
    let json = "{}";
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.search.max_displayed_results, 16);
    assert_eq!(config.search.action_bar_hints.len(), 5);
}

#[test]
fn test_old_prefix_format_migration() {
    let json = r#"{
        "search": {
            "prefix": {
                "file": "~",
                "clipboard": ";",
                "shellHistory": "!"
            }
        }
    }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.search.action_bar_hints.len(), 5);
    assert!(config
        .search
        .action_bar_hints
        .iter()
        .any(|h| h.prefix == "~" && h.plugin == "files"));
}

#[test]
fn test_old_prefix_with_null_action_bar_hints() {
    let json = r#"{
        "search": {
            "prefix": {
                "file": "~",
                "clipboard": ";",
                "shellHistory": "!"
            },
            "actionBarHints": null
        }
    }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.search.action_bar_hints.len(), 5);
    assert!(config
        .search
        .action_bar_hints
        .iter()
        .any(|h| h.prefix == "~" && h.plugin == "files"));
}

#[test]
fn test_old_prefix_format_partial_migration() {
    let json = r#"{
        "search": {
            "prefix": {
                "file": "~",
                "clipboard": ";"
            }
        }
    }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.search.action_bar_hints.len(), 5);
}

#[test]
fn test_old_prefix_format_no_migration() {
    let json = r#"{
        "search": {
            "prefix": {
                "plugins": "/",
                "app": "@"
            }
        }
    }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.search.action_bar_hints.len(), 5);
}

#[test]
fn test_null_only_action_bar_hints() {
    let json = r#"{
        "search": {
            "actionBarHints": null
        }
    }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.search.action_bar_hints.len(), 5);
}
