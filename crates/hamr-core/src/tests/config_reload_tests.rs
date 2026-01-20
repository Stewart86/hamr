//! Tests for config reload functionality

use crate::Result;
use crate::config::Config;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_reload_config_updates_values() -> Result<()> {
    // Create a temporary directory
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create initial config
    let initial_config = r#"{
        "search": {
            "maxDisplayedResults": 10
        }
    }"#;
    fs::write(&config_path, initial_config).unwrap();

    // Load the config
    let config1 = Config::load(&config_path)?;
    assert_eq!(config1.search.max_displayed_results, 10);

    // Update the config file
    let updated_config = r#"{
        "search": {
            "maxDisplayedResults": 20
        }
    }"#;
    fs::write(&config_path, updated_config).unwrap();

    // Reload the config
    let config2 = Config::load(&config_path)?;
    assert_eq!(config2.search.max_displayed_results, 20);

    Ok(())
}

#[test]
fn test_reload_config_preserves_defaults() -> Result<()> {
    // Create a temporary directory
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create partial config (missing some fields)
    let partial_config = r#"{
        "search": {
            "maxDisplayedResults": 15
        }
    }"#;
    fs::write(&config_path, partial_config).unwrap();

    // Load the config
    let config = Config::load(&config_path)?;
    assert_eq!(config.search.max_displayed_results, 15);
    // Should use defaults for unspecified fields
    assert_eq!(config.search.max_recent_items, 20); // default value
    assert_eq!(config.search.plugin_debounce_ms, 150); // default value

    Ok(())
}

#[test]
fn test_reload_config_handles_invalid_gracefully() -> Result<()> {
    // Create a temporary directory
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create valid initial config
    let initial_config = r#"{
        "search": {
            "maxDisplayedResults": 10
        }
    }"#;
    fs::write(&config_path, initial_config).unwrap();

    // Load the config
    let config1 = Config::load(&config_path)?;
    assert_eq!(config1.search.max_displayed_results, 10);

    // Write invalid JSON
    let invalid_config = "{ invalid json ]";
    fs::write(&config_path, invalid_config).unwrap();

    // Try to reload - should fail but that's expected
    let result = Config::load(&config_path);
    assert!(result.is_err(), "Loading invalid JSON should fail");

    Ok(())
}

#[test]
fn test_reload_config_all_fields() -> Result<()> {
    // Create a temporary directory
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create full config using a string to avoid raw string literal issues with special chars
    let full_config = "{\"search\":{\"maxDisplayedResults\":32,\"maxRecentItems\":50,\"maxResultsPerPlugin\":10,\"pluginDebounceMs\":250,\"diversityDecay\":0.8,\"engineBaseUrl\":\"https://duckduckgo.com/?q=\",\"excludedSites\":[\"example.com\",\"test.com\"],\"prefix\":{\"plugins\":\"!\",\"app\":\"#\",\"emojis\":\"&\",\"math\":\"$\",\"shellCommand\":\"%\",\"webSearch\":\"~\"}},\"apps\":{\"terminal\":\"alacritty\",\"fileManager\":\"nautilus\",\"browser\":\"firefox\"}}";
    fs::write(&config_path, full_config).unwrap();

    // Load the config
    let config = Config::load(&config_path)?;
    assert_eq!(config.search.max_displayed_results, 32);
    assert_eq!(config.search.max_recent_items, 50);
    assert_eq!(config.search.max_results_per_plugin, 10);
    assert_eq!(config.search.plugin_debounce_ms, 250);
    assert_eq!(config.search.diversity_decay, 0.8);
    assert_eq!(config.search.engine_base_url, "https://duckduckgo.com/?q=");
    assert_eq!(
        config.search.excluded_sites,
        vec!["example.com", "test.com"]
    );
    assert_eq!(config.search.prefix.plugins, "!");
    assert_eq!(config.search.prefix.app, "#");
    assert_eq!(config.search.prefix.emojis, "&");
    assert_eq!(config.search.prefix.math, "$");
    assert_eq!(config.search.prefix.shell_command, "%");
    assert_eq!(config.search.prefix.web_search, "~");
    assert_eq!(config.apps.terminal, Some("alacritty".to_string()));
    assert_eq!(config.apps.file_manager, Some("nautilus".to_string()));
    assert_eq!(config.apps.browser, Some("firefox".to_string()));

    Ok(())
}
