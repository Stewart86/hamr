//! Test fixtures and helpers

use crate::index::IndexedItem;
use crate::plugin::IndexItem;
use crate::search::{Searchable, SearchableSource};
use hamr_types::Frecency;

/// Create a mock `IndexItem` (now a type alias to `hamr_types::ResultItem`)
pub fn make_index_item(id: &str, name: &str) -> IndexItem {
    IndexItem {
        id: id.to_string(),
        name: name.to_string(),
        verb: Some("Open".to_string()),
        ..Default::default()
    }
}

/// Create an `IndexItem` with keywords
pub fn make_index_item_with_keywords(id: &str, name: &str, keywords: Vec<&str>) -> IndexItem {
    let mut item = make_index_item(id, name);
    item.keywords = Some(keywords.into_iter().map(String::from).collect());
    item
}

/// Create a slider `IndexItem` with value and badges
pub fn make_slider_index_item(id: &str, name: &str, value: f64) -> IndexItem {
    use hamr_types::Badge;
    IndexItem {
        id: id.to_string(),
        name: name.to_string(),
        description: Some("Volume control".to_string()),
        icon: Some("audio-volume-high".to_string()),
        icon_type: Some("icon".to_string()),
        verb: Some("Adjust".to_string()),
        badges: vec![Badge {
            text: Some("54%".to_string()),
            icon: None,
            color: None,
        }],
        keep_open: true,
        ..Default::default()
    }
    .with_slider(value, 0.0, 100.0, 5.0, None)
}

/// Create a Searchable from a slider item
pub fn make_slider_searchable(id: &str, name: &str, value: f64, plugin_id: &str) -> Searchable {
    Searchable {
        id: id.to_string(),
        name: name.to_string(),
        keywords: Vec::new(),
        source: SearchableSource::IndexedItem {
            plugin_id: plugin_id.to_string(),
            item: make_slider_index_item(id, name, value),
        },
        is_history_term: false,
    }
}

/// Create an `IndexedItem` with default frecency
pub fn make_indexed_item(id: &str, name: &str) -> IndexedItem {
    IndexedItem::new(make_index_item(id, name))
}

/// Create an `IndexedItem` with specified frecency count and `last_used`
pub fn make_indexed_item_with_frecency(
    id: &str,
    name: &str,
    count: u32,
    last_used: u64,
) -> IndexedItem {
    let mut item = IndexedItem::new(make_index_item(id, name));
    item.frecency = Frecency {
        count,
        last_used,
        ..Default::default()
    };
    item
}

/// Create an `IndexedItem` with full frecency data
pub fn make_indexed_item_with_full_frecency(
    id: &str,
    name: &str,
    frecency: Frecency,
) -> IndexedItem {
    let mut item = IndexedItem::new(make_index_item(id, name));
    item.frecency = frecency;
    item
}

/// Create a Searchable for testing
pub fn make_searchable(id: &str, name: &str, plugin_id: &str) -> Searchable {
    Searchable {
        id: id.to_string(),
        name: name.to_string(),
        keywords: Vec::new(),
        source: SearchableSource::IndexedItem {
            plugin_id: plugin_id.to_string(),
            item: make_index_item(id, name),
        },
        is_history_term: false,
    }
}

/// Create a Searchable with keywords
pub fn make_searchable_with_keywords(
    id: &str,
    name: &str,
    plugin_id: &str,
    keywords: Vec<&str>,
) -> Searchable {
    let keywords_strings: Vec<String> = keywords
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    Searchable {
        id: id.to_string(),
        name: name.to_string(),
        keywords: keywords_strings.clone(),
        source: SearchableSource::IndexedItem {
            plugin_id: plugin_id.to_string(),
            item: make_index_item_with_keywords(id, name, keywords),
        },
        is_history_term: false,
    }
}

/// Create a Searchable that is a history term
pub fn make_history_searchable(id: &str, term: &str, plugin_id: &str) -> Searchable {
    Searchable {
        id: id.to_string(),
        name: term.to_string(),
        keywords: Vec::new(),
        source: SearchableSource::IndexedItem {
            plugin_id: plugin_id.to_string(),
            item: make_index_item(id, term),
        },
        is_history_term: true,
    }
}

/// Create a plugin Searchable
pub fn make_plugin_searchable(id: &str, name: &str) -> Searchable {
    Searchable {
        id: id.to_string(),
        name: name.to_string(),
        keywords: Vec::new(),
        source: SearchableSource::Plugin { id: id.to_string() },
        is_history_term: false,
    }
}

/// Create a plugin history term Searchable (for plugins with frecency: "plugin" mode)
pub fn make_plugin_history_searchable(id: &str, term: &str) -> Searchable {
    Searchable {
        id: id.to_string(),
        name: term.to_string(),
        keywords: Vec::new(),
        source: SearchableSource::Plugin { id: id.to_string() },
        is_history_term: true,
    }
}

/// Get current timestamp in milliseconds
pub fn now_millis() -> u64 {
    crate::utils::now_millis()
}

/// Get timestamp for N hours ago
pub fn hours_ago(hours: u64) -> u64 {
    now_millis().saturating_sub(hours * 60 * 60 * 1000)
}

/// Get timestamp for N days ago
pub fn days_ago(days: u64) -> u64 {
    now_millis().saturating_sub(days * 24 * 60 * 60 * 1000)
}
