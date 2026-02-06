//! Config validation - warns about unknown fields

use serde_json::Value;
use std::collections::HashSet;
use tracing::warn;

/// Validate JSON config and warn about unknown fields.
pub fn warn_unknown_fields(content: &str, config_name: &str) {
    let Ok(value) = serde_json::from_str::<Value>(content) else {
        return;
    };

    let expected = expected_core_config_keys();
    let unknowns = find_unknown_keys(&value, &expected, "");

    for path in unknowns {
        warn!("Unknown config field in {config_name}: {path}");
    }
}

/// Validate GTK config JSON and warn about unknown fields.
pub fn warn_unknown_gtk_fields(content: &str, config_name: &str) {
    let Ok(value) = serde_json::from_str::<Value>(content) else {
        return;
    };

    let expected = expected_gtk_config_keys();
    let unknowns = find_unknown_keys(&value, &expected, "");

    for path in unknowns {
        warn!("Unknown config field in {config_name}: {path}");
    }
}

/// Find unknown keys in JSON value compared to expected keys.
/// Returns paths like "search.unknownField" for unknown fields.
fn find_unknown_keys(value: &Value, expected: &ExpectedKeys, prefix: &str) -> Vec<String> {
    let mut unknowns = Vec::new();

    let Value::Object(obj) = value else {
        return unknowns;
    };

    for (key, child) in obj {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };

        if let Some(nested) = expected.nested.get(key.as_str()) {
            unknowns.extend(find_unknown_keys(child, nested, &path));
        } else if !expected.fields.contains(key.as_str()) {
            unknowns.push(path);
        }
    }

    unknowns
}

/// Expected keys for a config section.
/// `fields` are leaf fields, `nested` are nested objects with their own expected keys.
struct ExpectedKeys {
    fields: HashSet<&'static str>,
    nested: std::collections::HashMap<&'static str, ExpectedKeys>,
}

impl ExpectedKeys {
    fn new(fields: &[&'static str]) -> Self {
        Self {
            fields: fields.iter().copied().collect(),
            nested: std::collections::HashMap::new(),
        }
    }

    fn with_nested(mut self, key: &'static str, nested: ExpectedKeys) -> Self {
        self.nested.insert(key, nested);
        self
    }
}

/// Expected keys for hamr-core Config (settings.rs)
fn expected_core_config_keys() -> ExpectedKeys {
    let prefix_keys = ExpectedKeys::new(&[
        "plugins",
        "app",
        "emojis",
        "math",
        "shellCommand",
        "webSearch",
        "file",
        "clipboard",
        "shellHistory",
    ]);

    let search_keys = ExpectedKeys::new(&[
        "maxDisplayedResults",
        "maxRecentItems",
        "maxResultsPerPlugin",
        "pluginDebounceMs",
        "diversityDecay",
        "engineBaseUrl",
        "excludedSites",
        "actionBarHints",
        "actionBarHintsJson",
        "pluginRankingBonus",
        "suggestionStalenessHalfLifeDays",
        "maxSuggestionAgeDays",
    ])
    .with_nested("prefix", prefix_keys);

    let apps_keys = ExpectedKeys::new(&["terminal", "fileManager", "browser"]);

    ExpectedKeys::new(&[])
        .with_nested("search", search_keys)
        .with_nested("apps", apps_keys)
}

/// Expected keys for hamr-gtk Config (config.rs)
fn expected_gtk_config_keys() -> ExpectedKeys {
    let grid_keys = ExpectedKeys::new(&["columns", "itemWidth", "spacing"]);

    let appearance_keys = ExpectedKeys::new(&[
        "backgroundTransparency",
        "contentTransparency",
        "launcherXRatio",
        "launcherYRatio",
        "fontScale",
        "uiScale",
        "defaultResultView",
    ])
    .with_nested("grid", grid_keys);

    let sizes_keys = ExpectedKeys::new(&["searchWidth", "maxResultsHeight"]);

    let fonts_keys = ExpectedKeys::new(&["main", "monospace", "icon"]);

    let behavior_keys = ExpectedKeys::new(&["clickOutsideAction", "stateRestoreWindowMs"]);

    ExpectedKeys::new(&[])
        .with_nested("appearance", appearance_keys)
        .with_nested("sizes", sizes_keys)
        .with_nested("fonts", fonts_keys)
        .with_nested("behavior", behavior_keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_core_config_no_warnings() {
        let json = r#"{
            "search": {
                "maxDisplayedResults": 20,
                "prefix": {
                    "plugins": "/"
                }
            },
            "apps": {
                "terminal": "alacritty"
            }
        }"#;

        let value: Value = serde_json::from_str(json).unwrap();
        let expected = expected_core_config_keys();
        let unknowns = find_unknown_keys(&value, &expected, "");

        assert!(
            unknowns.is_empty(),
            "Expected no unknowns, got: {unknowns:?}"
        );
    }

    #[test]
    fn test_unknown_top_level_field() {
        let json = r#"{
            "search": {},
            "unknownSection": {}
        }"#;

        let value: Value = serde_json::from_str(json).unwrap();
        let expected = expected_core_config_keys();
        let unknowns = find_unknown_keys(&value, &expected, "");

        assert_eq!(unknowns, vec!["unknownSection"]);
    }

    #[test]
    fn test_unknown_nested_field() {
        let json = r#"{
            "search": {
                "maxDisplayedResults": 20,
                "typoField": 100
            }
        }"#;

        let value: Value = serde_json::from_str(json).unwrap();
        let expected = expected_core_config_keys();
        let unknowns = find_unknown_keys(&value, &expected, "");

        assert_eq!(unknowns, vec!["search.typoField"]);
    }

    #[test]
    fn test_unknown_deeply_nested_field() {
        let json = r#"{
            "search": {
                "prefix": {
                    "plugins": "/",
                    "unknownPrefix": "!"
                }
            }
        }"#;

        let value: Value = serde_json::from_str(json).unwrap();
        let expected = expected_core_config_keys();
        let unknowns = find_unknown_keys(&value, &expected, "");

        assert_eq!(unknowns, vec!["search.prefix.unknownPrefix"]);
    }

    #[test]
    fn test_multiple_unknown_fields() {
        let json = r#"{
            "search": {
                "typo1": 1,
                "typo2": 2
            },
            "badSection": {}
        }"#;

        let value: Value = serde_json::from_str(json).unwrap();
        let expected = expected_core_config_keys();
        let unknowns = find_unknown_keys(&value, &expected, "");

        assert!(unknowns.contains(&"search.typo1".to_string()));
        assert!(unknowns.contains(&"search.typo2".to_string()));
        assert!(unknowns.contains(&"badSection".to_string()));
    }

    #[test]
    fn test_valid_gtk_config_no_warnings() {
        let json = r#"{
            "appearance": {
                "backgroundTransparency": 0.2,
                "grid": {
                    "columns": 5
                }
            },
            "sizes": {
                "searchWidth": 640
            },
            "fonts": {
                "main": "Sans"
            },
            "behavior": {
                "clickOutsideAction": "close"
            }
        }"#;

        let value: Value = serde_json::from_str(json).unwrap();
        let expected = expected_gtk_config_keys();
        let unknowns = find_unknown_keys(&value, &expected, "");

        assert!(
            unknowns.is_empty(),
            "Expected no unknowns, got: {unknowns:?}"
        );
    }

    #[test]
    fn test_unknown_gtk_field() {
        let json = r#"{
            "appearance": {
                "unknownAppearance": true
            }
        }"#;

        let value: Value = serde_json::from_str(json).unwrap();
        let expected = expected_gtk_config_keys();
        let unknowns = find_unknown_keys(&value, &expected, "");

        assert_eq!(unknowns, vec!["appearance.unknownAppearance"]);
    }

    #[test]
    fn test_warn_unknown_fields_does_not_panic_on_valid() {
        let json = r#"{"search": {}}"#;
        warn_unknown_fields(json, "test");
    }

    #[test]
    fn test_warn_unknown_fields_does_not_panic_on_invalid_json() {
        let json = "not valid json";
        warn_unknown_fields(json, "test");
    }

    #[test]
    fn test_empty_config_no_warnings() {
        let json = "{}";
        let value: Value = serde_json::from_str(json).unwrap();
        let expected = expected_core_config_keys();
        let unknowns = find_unknown_keys(&value, &expected, "");
        assert!(unknowns.is_empty());
    }
}
