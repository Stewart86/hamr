//! Tests for configuration loading and defaults
//!
//! Tests the config system including:
//! - Config defaults
//! - Config serialization/deserialization
//! - Search config options
//! - Save/load round trips
//! - Directories management
//! - Action bar hints

use crate::config::{AppConfig, Config, Directories, SearchConfig};
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
fn test_config_parse_partial() {
    let json = r#"{
        "search": {
            "maxDisplayedResults": 30
        }
    }"#;

    let config: Config = serde_json::from_str(json).unwrap();

    assert_eq!(config.search.max_displayed_results, 30);
    assert_eq!(config.search.max_recent_items, 20);
    assert_eq!(config.search.prefix.plugins, "/");
}

#[test]
fn test_config_parse_full() {
    let json = r##"{
        "search": {
            "prefix": {
                "plugins": "\\",
                "app": "#",
                "emojis": ";",
                "math": "calc ",
                "shellCommand": "$",
                "webSearch": "g "
            },
            "maxDisplayedResults": 25,
            "maxRecentItems": 15,
            "maxResultsPerPlugin": 5,
            "pluginDebounceMs": 200,
            "diversityDecay": 0.8,
            "engineBaseUrl": "https://duckduckgo.com/?q=",
            "excludedSites": ["example.com", "test.org"],
            "actionBarHints": [
                {"prefix": "/", "plugin": "plugins", "description": "Browse plugins"}
            ]
        },
        "apps": {
            "terminal": "kitty",
            "fileManager": "nautilus",
            "browser": "firefox"
        }
    }"##;

    let config: Config = serde_json::from_str(json).unwrap();

    assert_eq!(config.search.prefix.plugins, "\\");
    assert_eq!(config.search.prefix.app, "#");
    assert_eq!(config.search.prefix.math, "calc ");
    assert_eq!(config.search.max_displayed_results, 25);
    assert_eq!(config.search.max_results_per_plugin, 5);
    assert_eq!(config.search.plugin_debounce_ms, 200);
    assert!((config.search.diversity_decay - 0.8).abs() < 0.001);
    assert!(config.search.engine_base_url.contains("duckduckgo"));
    assert_eq!(config.search.excluded_sites.len(), 2);

    assert_eq!(config.apps.terminal, Some("kitty".to_string()));
    assert_eq!(config.apps.file_manager, Some("nautilus".to_string()));
    assert_eq!(config.apps.browser, Some("firefox".to_string()));

    assert_eq!(config.action_bar_hints().len(), 1);
    assert_eq!(config.action_bar_hints()[0].prefix, "/");
}

#[test]
fn test_config_load_nonexistent() {
    let path = std::path::Path::new("/nonexistent/config.json");
    let config = Config::load(path).unwrap();

    assert_eq!(config.search.max_displayed_results, 16);
}

#[test]
fn test_config_save_load_roundtrip() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();

    let mut config = Config::default();
    config.search.max_displayed_results = 42;
    config.search.prefix.plugins = ">>".to_string();
    config.apps.terminal = Some("alacritty".to_string());

    config.save(path).unwrap();

    let loaded = Config::load(path).unwrap();

    assert_eq!(loaded.search.max_displayed_results, 42);
    assert_eq!(loaded.search.prefix.plugins, ">>");
    assert_eq!(loaded.apps.terminal, Some("alacritty".to_string()));
}

#[test]
fn test_config_load_invalid_json() {
    let temp_file = NamedTempFile::new().unwrap();
    std::fs::write(temp_file.path(), "not valid json").unwrap();

    let result = Config::load(temp_file.path());
    assert!(result.is_err());
}

#[test]
fn test_search_config_engine_url() {
    let config = SearchConfig::default();
    assert!(config.engine_base_url.starts_with("https://"));
}

#[test]
fn test_search_config_excluded_sites() {
    let json = r#"{
        "excludedSites": ["reddit.com", "pinterest.com", "quora.com"]
    }"#;

    let config: SearchConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.excluded_sites.len(), 3);
    assert!(config.excluded_sites.contains(&"reddit.com".to_string()));
}

#[test]
fn test_app_config_default() {
    let config = AppConfig::default();

    assert!(config.terminal.is_none());
    assert!(config.file_manager.is_none());
    assert!(config.browser.is_none());
}

#[test]
fn test_app_config_partial() {
    let json = r#"{"terminal": "foot"}"#;
    let config: AppConfig = serde_json::from_str(json).unwrap();

    assert_eq!(config.terminal, Some("foot".to_string()));
    assert!(config.file_manager.is_none());
}

#[test]
fn test_config_camelcase_serialization() {
    let config = Config::default();
    let json = serde_json::to_string(&config).unwrap();

    assert!(json.contains("maxDisplayedResults"));
    assert!(json.contains("maxRecentItems"));
    assert!(json.contains("pluginDebounceMs"));
    assert!(json.contains("diversityDecay"));
    assert!(json.contains("shellCommand"));
    assert!(json.contains("webSearch"));

    assert!(!json.contains("max_displayed_results"));
    assert!(!json.contains("plugin_debounce_ms"));
}

#[test]
fn test_search_prefixes_default() {
    let config = Config::default();
    let prefixes = &config.search.prefix;

    assert_eq!(prefixes.plugins.len(), 1);
    assert_eq!(prefixes.app.len(), 1);
    assert_eq!(prefixes.emojis.len(), 1);
    assert_eq!(prefixes.math.len(), 1);
    assert_eq!(prefixes.shell_command.len(), 1);
    assert_eq!(prefixes.web_search.len(), 1);
}

#[test]
fn test_search_prefixes_custom() {
    let json = r#"{
        "prefix": {
            "plugins": "plugin:",
            "app": "app:",
            "emojis": "emoji:",
            "math": "calc:",
            "shellCommand": "run:",
            "webSearch": "search:"
        }
    }"#;

    let config: SearchConfig = serde_json::from_str(json).unwrap();

    assert_eq!(config.prefix.plugins, "plugin:");
    assert_eq!(config.prefix.app, "app:");
    assert_eq!(config.prefix.emojis, "emoji:");
    assert_eq!(config.prefix.math, "calc:");
    assert_eq!(config.prefix.shell_command, "run:");
    assert_eq!(config.prefix.web_search, "search:");
}

#[test]
fn test_search_prefixes_partial_override() {
    let json = r#"{
        "prefix": {
            "plugins": ">>",
            "math": "=="
        }
    }"#;

    let config: SearchConfig = serde_json::from_str(json).unwrap();

    assert_eq!(config.prefix.plugins, ">>");
    assert_eq!(config.prefix.math, "==");
    assert_eq!(config.prefix.app, "@");
    assert_eq!(config.prefix.emojis, ":");
    assert_eq!(config.prefix.shell_command, "!");
    assert_eq!(config.prefix.web_search, "?");
}

#[test]
fn test_action_bar_hints_default() {
    let config = Config::default();
    // Default has 5 hints: ~->files, ;->clipboard, =->calculate, :->emoji, !->shell
    assert_eq!(config.action_bar_hints().len(), 5);
    assert_eq!(config.action_bar_hints()[0].prefix, "~");
    assert_eq!(config.action_bar_hints()[0].plugin, "files");
}

#[test]
fn test_action_bar_hints_parsing() {
    // Test the new format: actionBarHints inside search config
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
    // Test the legacy QML format: actionBarHintsJson as stringified JSON inside search config
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
fn test_directories_with_base() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path().to_path_buf();

    let dirs = Directories::with_base(base.clone());

    assert_eq!(dirs.config, base);
    assert_eq!(dirs.data, base);
    assert_eq!(dirs.cache, base);
    assert_eq!(dirs.user_plugins, base.join("plugins"));
    assert_eq!(dirs.config_file, base.join("config.json"));
    assert_eq!(dirs.state_file, base.join("state.json"));
    assert_eq!(dirs.colors_file, base.join("colors.json"));
    assert_eq!(dirs.index_cache, base.join("plugin-indexes.json"));
    assert_eq!(dirs.builtin_plugins, base.join("builtin-plugins"));
}

#[test]
fn test_directories_ensure_exists() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path().join("hamr-test");

    let dirs = Directories::with_base(base.clone());

    assert!(!dirs.config.exists());
    assert!(!dirs.user_plugins.exists());

    dirs.ensure_exists().unwrap();

    assert!(dirs.config.exists());
    assert!(dirs.data.exists());
    assert!(dirs.cache.exists());
    assert!(dirs.user_plugins.exists());
}

#[test]
fn test_directories_ensure_exists_idempotent() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path().to_path_buf();

    let dirs = Directories::with_base(base);

    dirs.ensure_exists().unwrap();
    dirs.ensure_exists().unwrap();
    dirs.ensure_exists().unwrap();
}

#[test]
fn test_config_max_displayed_results_range() {
    let json_small = r#"{"search": {"maxDisplayedResults": 1}}"#;
    let config: Config = serde_json::from_str(json_small).unwrap();
    assert_eq!(config.search.max_displayed_results, 1);

    let json_large = r#"{"search": {"maxDisplayedResults": 1000}}"#;
    let config: Config = serde_json::from_str(json_large).unwrap();
    assert_eq!(config.search.max_displayed_results, 1000);
}

#[test]
fn test_config_debounce_range() {
    let json_zero = r#"{"search": {"pluginDebounceMs": 0}}"#;
    let config: Config = serde_json::from_str(json_zero).unwrap();
    assert_eq!(config.search.plugin_debounce_ms, 0);

    let json_high = r#"{"search": {"pluginDebounceMs": 1000}}"#;
    let config: Config = serde_json::from_str(json_high).unwrap();
    assert_eq!(config.search.plugin_debounce_ms, 1000);
}

#[test]
fn test_config_diversity_decay_range() {
    let json_zero = r#"{"search": {"diversityDecay": 0.0}}"#;
    let config: Config = serde_json::from_str(json_zero).unwrap();
    assert!((config.search.diversity_decay - 0.0).abs() < 0.001);

    let json_one = r#"{"search": {"diversityDecay": 1.0}}"#;
    let config: Config = serde_json::from_str(json_one).unwrap();
    assert!((config.search.diversity_decay - 1.0).abs() < 0.001);

    let json_high = r#"{"search": {"diversityDecay": 0.95}}"#;
    let config: Config = serde_json::from_str(json_high).unwrap();
    assert!((config.search.diversity_decay - 0.95).abs() < 0.001);
}

#[test]
fn test_config_max_results_per_plugin() {
    let json_zero = r#"{"search": {"maxResultsPerPlugin": 0}}"#;
    let config: Config = serde_json::from_str(json_zero).unwrap();
    assert_eq!(config.search.max_results_per_plugin, 0);

    let json_limit = r#"{"search": {"maxResultsPerPlugin": 3}}"#;
    let config: Config = serde_json::from_str(json_limit).unwrap();
    assert_eq!(config.search.max_results_per_plugin, 3);
}

#[test]
fn test_config_empty_prefix() {
    let json = r#"{"search": {"prefix": {"math": ""}}}"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.search.prefix.math, "");
}

#[test]
fn test_config_multichar_prefix() {
    let json = r#"{"search": {"prefix": {"plugins": "plugin ", "math": "calc "}}}"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.search.prefix.plugins, "plugin ");
    assert_eq!(config.search.prefix.math, "calc ");
}

#[test]
fn test_config_engine_url_custom() {
    let json = r#"{"search": {"engineBaseUrl": "https://duckduckgo.com/?q="}}"#;
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
fn test_config_plugin_ranking_bonus_default_empty() {
    let json = r"{}";
    let config: Config = serde_json::from_str(json).unwrap();
    assert!(config.search.plugin_ranking_bonus.is_empty());
}
