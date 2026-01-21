use crate::Result;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Main configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct Config {
    #[serde(default)]
    pub search: SearchConfig,

    #[serde(default)]
    pub apps: AppConfig,
}

impl Config {
    /// Get action bar hints (from search config)
    #[must_use]
    pub fn action_bar_hints(&self) -> &[ActionBarHint] {
        &self.search.action_bar_hints
    }
}

/// Default action bar hints mapping prefix -> plugin
fn default_action_bar_hints() -> Vec<ActionBarHint> {
    vec![
        ActionBarHint {
            prefix: "~".to_string(),
            plugin: "files".to_string(),
            label: Some("Files".to_string()),
            icon: Some("folder_open".to_string()),
            description: None,
        },
        ActionBarHint {
            prefix: ";".to_string(),
            plugin: "clipboard".to_string(),
            label: Some("Clipboard".to_string()),
            icon: Some("content_paste".to_string()),
            description: None,
        },
        ActionBarHint {
            prefix: "=".to_string(),
            plugin: "calculate".to_string(),
            label: Some("Calculate".to_string()),
            icon: Some("calculate".to_string()),
            description: None,
        },
        ActionBarHint {
            prefix: ":".to_string(),
            plugin: "emoji".to_string(),
            label: Some("Emoji".to_string()),
            icon: Some("emoji_emotions".to_string()),
            description: None,
        },
        ActionBarHint {
            prefix: "!".to_string(),
            plugin: "shell".to_string(),
            label: Some("Shell".to_string()),
            icon: Some("terminal".to_string()),
            description: None,
        },
    ]
}

/// Custom deserializer for `SearchConfig` that migrates old prefix format to `action_bar_hints`
impl<'de> Deserialize<'de> for SearchConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SearchConfigRaw {
            #[serde(default)]
            pub prefix: SearchPrefixes,

            #[serde(default = "default_max_results")]
            pub max_displayed_results: usize,

            #[serde(default = "default_max_recent")]
            pub max_recent_items: usize,

            #[serde(default = "default_max_per_plugin")]
            pub max_results_per_plugin: usize,

            #[serde(default = "default_debounce")]
            pub plugin_debounce_ms: u64,

            #[serde(default = "default_diversity_decay")]
            pub diversity_decay: f64,

            #[serde(default = "default_engine_url")]
            pub engine_base_url: String,

            #[serde(default)]
            pub excluded_sites: Vec<String>,

            #[serde(default)]
            pub action_bar_hints: Option<Vec<ActionBarHint>>,

            #[serde(default)]
            pub action_bar_hints_json: Option<String>,

            #[serde(default)]
            pub plugin_ranking_bonus: HashMap<String, f64>,
        }

        let raw: SearchConfigRaw = SearchConfigRaw::deserialize(deserializer)?;

        let action_bar_hints = match &raw.action_bar_hints {
            Some(hints) if !hints.is_empty() => hints.clone(),
            _ => {
                if let Some(json_str) = &raw.action_bar_hints_json {
                    serde_json::from_str(json_str).map_err(|e| {
                        D::Error::custom(format!("Failed to parse actionBarHintsJson: {e}"))
                    })?
                } else {
                    migrate_old_prefix_to_hints(&raw.prefix)
                }
            }
        };

        Ok(Self {
            prefix: raw.prefix,
            max_displayed_results: raw.max_displayed_results,
            max_recent_items: raw.max_recent_items,
            max_results_per_plugin: raw.max_results_per_plugin,
            plugin_debounce_ms: raw.plugin_debounce_ms,
            diversity_decay: raw.diversity_decay,
            engine_base_url: raw.engine_base_url,
            excluded_sites: raw.excluded_sites,
            action_bar_hints,
            plugin_ranking_bonus: raw.plugin_ranking_bonus,
        })
    }
}

/// Migrate old prefix format (file, clipboard, `shell_history`) to `action_bar_hints`
fn migrate_old_prefix_to_hints(prefixes: &SearchPrefixes) -> Vec<ActionBarHint> {
    let mut hints = Vec::new();

    if let Some(ref file_prefix) = prefixes.file {
        hints.push(ActionBarHint {
            prefix: file_prefix.clone(),
            plugin: "files".to_string(),
            label: Some("Files".to_string()),
            icon: Some("folder_open".to_string()),
            description: None,
        });
    }

    if let Some(ref clipboard_prefix) = prefixes.clipboard {
        hints.push(ActionBarHint {
            prefix: clipboard_prefix.clone(),
            plugin: "clipboard".to_string(),
            label: Some("Clipboard".to_string()),
            icon: Some("content_paste".to_string()),
            description: None,
        });
    }

    if let Some(ref shell_history_prefix) = prefixes.shell_history {
        hints.push(ActionBarHint {
            prefix: shell_history_prefix.clone(),
            plugin: "shell".to_string(),
            label: Some("Shell".to_string()),
            icon: Some("terminal".to_string()),
            description: None,
        });
    }

    if hints.is_empty() {
        default_action_bar_hints()
    } else {
        let used_prefixes: std::collections::HashSet<String> =
            hints.iter().map(|h| h.prefix.clone()).collect();

        let defaults = default_action_bar_hints();
        for hint in defaults {
            if !used_prefixes.contains(&hint.prefix) {
                hints.push(hint);
            }
        }
        hints
    }
}

impl Config {
    /// Load config from file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or contains invalid JSON.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)?;
        super::validation::warn_unknown_fields(&content, "config.json");
        let config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save config to file.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchConfig {
    #[serde(default)]
    pub prefix: SearchPrefixes,

    #[serde(default = "default_max_results")]
    pub max_displayed_results: usize,

    #[serde(default = "default_max_recent")]
    pub max_recent_items: usize,

    #[serde(default = "default_max_per_plugin")]
    pub max_results_per_plugin: usize,

    #[serde(default = "default_debounce")]
    pub plugin_debounce_ms: u64,

    #[serde(default = "default_diversity_decay")]
    pub diversity_decay: f64,

    #[serde(default = "default_engine_url")]
    pub engine_base_url: String,

    #[serde(default)]
    pub excluded_sites: Vec<String>,

    /// Action bar hints - supports both array format and legacy stringified JSON
    #[serde(default, alias = "actionBarHintsJson")]
    pub action_bar_hints: Vec<ActionBarHint>,

    /// Per-plugin ranking bonus - allows users to boost specific plugins
    /// Example: {"apps": 200, "settings": 150} gives apps +200 and settings +150 to their scores
    #[serde(default)]
    pub plugin_ranking_bonus: HashMap<String, f64>,
}

fn default_max_results() -> usize {
    16
}
fn default_max_recent() -> usize {
    20
}
fn default_max_per_plugin() -> usize {
    0
}
fn default_debounce() -> u64 {
    150
}
fn default_diversity_decay() -> f64 {
    0.7
}
fn default_engine_url() -> String {
    "https://www.google.com/search?q=".to_string()
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            prefix: SearchPrefixes::default(),
            max_displayed_results: default_max_results(),
            max_recent_items: default_max_recent(),
            max_results_per_plugin: default_max_per_plugin(),
            plugin_debounce_ms: default_debounce(),
            diversity_decay: default_diversity_decay(),
            engine_base_url: default_engine_url(),
            excluded_sites: Vec::new(),
            action_bar_hints: default_action_bar_hints(),
            plugin_ranking_bonus: HashMap::new(),
        }
    }
}

/// Search prefix configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPrefixes {
    #[serde(default = "default_plugins_prefix")]
    pub plugins: String,

    #[serde(default = "default_app_prefix")]
    pub app: String,

    #[serde(default = "default_emoji_prefix")]
    pub emojis: String,

    #[serde(default = "default_math_prefix")]
    pub math: String,

    #[serde(default = "default_shell_prefix")]
    pub shell_command: String,

    #[serde(default = "default_web_prefix")]
    pub web_search: String,

    #[serde(default)]
    pub file: Option<String>,

    #[serde(default)]
    pub clipboard: Option<String>,

    #[serde(default)]
    pub shell_history: Option<String>,
}

fn default_plugins_prefix() -> String {
    "/".to_string()
}
fn default_app_prefix() -> String {
    "@".to_string()
}
fn default_emoji_prefix() -> String {
    ":".to_string()
}
fn default_math_prefix() -> String {
    "=".to_string()
}
fn default_shell_prefix() -> String {
    "!".to_string()
}
fn default_web_prefix() -> String {
    "?".to_string()
}

impl Default for SearchPrefixes {
    fn default() -> Self {
        Self {
            plugins: default_plugins_prefix(),
            app: default_app_prefix(),
            emojis: default_emoji_prefix(),
            math: default_math_prefix(),
            shell_command: default_shell_prefix(),
            web_search: default_web_prefix(),
            file: None,
            clipboard: None,
            shell_history: None,
        }
    }
}

/// App-specific configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default)]
    pub terminal: Option<String>,

    #[serde(default)]
    pub file_manager: Option<String>,

    #[serde(default)]
    pub browser: Option<String>,
}

/// Action bar hint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionBarHint {
    pub prefix: String,
    pub plugin: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.search.max_displayed_results, 16);
        assert_eq!(config.search.max_recent_items, 20);
        assert_eq!(config.search.max_results_per_plugin, 0);
        assert_eq!(config.search.plugin_debounce_ms, 150);
        assert!((config.search.diversity_decay - 0.7).abs() < f64::EPSILON);
        // Default has 5 action_bar_hints: ~->files, ;->clipboard, =->calculate, :->emoji, !->shell
        assert_eq!(config.search.action_bar_hints.len(), 5);
        assert!(config.search.plugin_ranking_bonus.is_empty());
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
        assert_eq!(loaded.search.engine_base_url, "https://duckduckgo.com/?q=");
    }

    #[test]
    fn test_config_action_bar_hints() {
        let json = r#"{
            "search": {
                "actionBarHints": [
                    {"prefix": ";", "plugin": "clipboard", "label": "Clipboard"}
                ]
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        let hints = config.action_bar_hints();
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].prefix, ";");
        assert_eq!(hints[0].plugin, "clipboard");
    }

    #[test]
    fn test_action_bar_hints_stringified_json() {
        let json = r#"{
            "search": {
                "actionBarHintsJson": "[{\"prefix\": \"!\", \"plugin\": \"shell\"}]"
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        let hints = config.action_bar_hints();
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].prefix, "!");
        assert_eq!(hints[0].plugin, "shell");
    }

    #[test]
    fn test_action_bar_hints_null_uses_defaults() {
        // null is treated same as missing field - uses defaults
        let json = r#"{"search": {"actionBarHints": null}}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.action_bar_hints().len(), 5);
    }

    #[test]
    fn test_action_bar_hints_empty_array_stays_empty() {
        // Explicit empty array means no hints (user wants to disable defaults)
        let json = r#"{"search": {"actionBarHints": []}}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.action_bar_hints().is_empty());
    }

    #[test]
    fn test_search_config_default() {
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
        let prefixes: SearchPrefixes = serde_json::from_str(json).unwrap();
        assert_eq!(prefixes.plugins, "//");
        assert_eq!(prefixes.app, "#");
        assert_eq!(prefixes.emojis, "::");
        assert_eq!(prefixes.math, "calc ");
        assert_eq!(prefixes.shell_command, "$");
        assert_eq!(prefixes.web_search, "g ");
    }

    #[test]
    fn test_search_prefixes_default() {
        let prefixes = SearchPrefixes::default();
        assert_eq!(prefixes.plugins, "/");
        assert_eq!(prefixes.app, "@");
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
        let hint: ActionBarHint = serde_json::from_str(json).unwrap();
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
        let hint: ActionBarHint = serde_json::from_str(json).unwrap();
        assert_eq!(hint.prefix, "!");
        assert_eq!(hint.plugin, "shell");
        assert!(hint.label.is_none());
        assert!(hint.icon.is_none());
        assert!(hint.description.is_none());
    }

    #[test]
    fn test_plugin_ranking_bonus() {
        let json = r#"{
            "search": {
                "pluginRankingBonus": {
                    "apps": 200,
                    "settings": 150.5
                }
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.search.plugin_ranking_bonus.get("apps"), Some(&200.0));
        assert_eq!(
            config.search.plugin_ranking_bonus.get("settings"),
            Some(&150.5)
        );
    }

    #[test]
    fn test_excluded_sites() {
        let json = r#"{
            "search": {
                "excludedSites": ["facebook.com", "twitter.com"]
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.search.excluded_sites.len(), 2);
        assert!(config
            .search
            .excluded_sites
            .contains(&"facebook.com".to_string()));
    }

    #[test]
    fn test_config_empty_json() {
        let json = "{}";
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.search.max_displayed_results, 16);
        // Empty JSON uses defaults including action_bar_hints
        assert_eq!(config.search.action_bar_hints.len(), 5);
    }
}
