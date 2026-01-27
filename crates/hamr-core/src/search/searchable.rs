use crate::plugin::IndexItem;

/// A searchable item
#[derive(Debug, Clone)]
pub struct Searchable {
    /// Unique identifier
    pub id: String,

    /// Primary search text (name)
    pub name: String,

    /// Secondary search text (keywords)
    pub keywords: Vec<String>,

    /// Source of the searchable
    pub source: SearchableSource,

    /// Whether this is a history term match
    pub is_history_term: bool,
}

impl Searchable {
    /// Create from a plugin entry
    #[must_use]
    pub fn from_plugin(plugin_id: &str, name: &str, description: Option<&str>) -> Self {
        Self {
            id: plugin_id.to_string(),
            name: name.to_string(),
            keywords: description.map(|d| vec![d.to_string()]).unwrap_or_default(),
            source: SearchableSource::Plugin {
                id: plugin_id.to_string(),
            },
            is_history_term: false,
        }
    }
}

/// Source of a searchable
#[derive(Debug, Clone)]
// Boxing IndexedItem would add indirection for the common case; size difference is acceptable
#[allow(clippy::large_enum_variant)]
pub enum SearchableSource {
    /// A plugin entry
    Plugin { id: String },

    /// An indexed item from a plugin
    IndexedItem { plugin_id: String, item: IndexItem },
}

impl SearchableSource {
    /// Get the plugin ID
    #[must_use]
    pub fn plugin_id(&self) -> &str {
        match self {
            Self::Plugin { id } => id,
            Self::IndexedItem { plugin_id, .. } => plugin_id,
        }
    }
}

/// A search match result with a reference to the matched searchable.
/// Uses a lifetime to avoid cloning during search operations.
#[derive(Debug)]
pub struct SearchMatch<'a> {
    /// Reference to the matched searchable
    pub searchable: &'a Searchable,

    /// Fuzzy match score (0.0 - 1.0)
    pub score: f64,
}

impl SearchMatch<'_> {
    /// Get the plugin ID
    #[must_use]
    pub fn plugin_id(&self) -> &str {
        self.searchable.source.plugin_id()
    }

    /// Check if this is a history term match
    #[must_use]
    pub fn is_history_term(&self) -> bool {
        self.searchable.is_history_term
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // Exact float comparisons are intentional in tests
mod tests {
    use super::*;
    use crate::plugin::IndexItem;

    fn make_index_item(id: &str, name: &str) -> IndexItem {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "name": name
        }))
        .unwrap()
    }

    #[test]
    fn test_searchable_from_plugin_basic() {
        let searchable = Searchable::from_plugin("apps", "Applications", None);
        assert_eq!(searchable.id, "apps");
        assert_eq!(searchable.name, "Applications");
        assert!(searchable.keywords.is_empty());
        assert!(!searchable.is_history_term);
        assert!(matches!(searchable.source, SearchableSource::Plugin { id } if id == "apps"));
    }

    #[test]
    fn test_searchable_from_plugin_with_description() {
        let searchable = Searchable::from_plugin("calc", "Calculator", Some("Math calculations"));
        assert_eq!(searchable.id, "calc");
        assert_eq!(searchable.name, "Calculator");
        assert_eq!(searchable.keywords, vec!["Math calculations".to_string()]);
        assert!(!searchable.is_history_term);
    }

    #[test]
    fn test_searchable_source_plugin_id_for_plugin() {
        let source = SearchableSource::Plugin {
            id: "my-plugin".to_string(),
        };
        assert_eq!(source.plugin_id(), "my-plugin");
    }

    #[test]
    fn test_searchable_source_plugin_id_for_indexed_item() {
        let item = make_index_item("item1", "Test Item");
        let source = SearchableSource::IndexedItem {
            plugin_id: "apps".to_string(),
            item,
        };
        assert_eq!(source.plugin_id(), "apps");
    }

    #[test]
    fn test_search_match_plugin_id() {
        let searchable = Searchable::from_plugin("test-plugin", "Test", None);
        let search_match = SearchMatch {
            searchable: &searchable,
            score: 0.95,
        };
        assert_eq!(search_match.plugin_id(), "test-plugin");
    }

    #[test]
    fn test_search_match_is_history_term_false() {
        let searchable = Searchable::from_plugin("apps", "Apps", None);
        let search_match = SearchMatch {
            searchable: &searchable,
            score: 0.8,
        };
        assert!(!search_match.is_history_term());
    }

    #[test]
    fn test_search_match_is_history_term_true() {
        let mut searchable = Searchable::from_plugin("apps", "Apps", None);
        searchable.is_history_term = true;
        let search_match = SearchMatch {
            searchable: &searchable,
            score: 0.8,
        };
        assert!(search_match.is_history_term());
    }

    #[test]
    fn test_searchable_debug_format() {
        let searchable = Searchable::from_plugin("x", "X", Some("desc"));
        let debug_str = format!("{searchable:?}");
        assert!(debug_str.contains("Searchable"));
        assert!(debug_str.contains('x'));
    }

    #[test]
    fn test_search_match_score() {
        let searchable = Searchable::from_plugin("apps", "Apps", None);
        let search_match = SearchMatch {
            searchable: &searchable,
            score: 0.75,
        };
        assert_eq!(search_match.score, 0.75);
        assert_eq!(search_match.plugin_id(), "apps");
    }

    #[test]
    fn test_searchable_source_clone() {
        let source = SearchableSource::Plugin {
            id: "test".to_string(),
        };
        let cloned = source.clone();
        assert_eq!(cloned.plugin_id(), "test");
    }
}
