mod store;

pub use store::IndexStore;

use crate::plugin::IndexItem;
use hamr_types::Frecency;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

/// Index data for a single plugin
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginIndex {
    #[serde(default)]
    pub items: Vec<IndexedItem>,

    /// When the index was last updated
    #[serde(default, rename = "lastIndexed")]
    pub last_indexed: u64,
}

/// An indexed item with frecency data.
///
/// Stores all frecency fields in a nested `frecency` struct (v2 format).
/// Deserializer handles migration from v1 format (flat underscore-prefixed fields).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexedItem {
    /// Base item data
    #[serde(flatten)]
    pub item: IndexItem,

    /// Unified frecency data (v2 format)
    #[serde(default)]
    pub frecency: Frecency,

    /// Is this a plugin-level entry (for frecency: "plugin" mode)
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_plugin_entry: bool,
}

// Serde skip_serializing_if requires &bool signature
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(b: &bool) -> bool {
    !*b
}

impl<'de> Deserialize<'de> for IndexedItem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: serde_json::Value = serde_json::Value::deserialize(deserializer)?;

        let item: IndexItem =
            serde_json::from_value(value.clone()).map_err(serde::de::Error::custom)?;

        // Check if this is v2 format (nested frecency object with required fields)
        // v2 has a "frecency" object, v1 has flat "_count", "_lastUsed" fields
        let frecency = if let Some(frec_val) = value.get("frecency")
            && frec_val.is_object()
            && frec_val.get("count").is_some()
        {
            // v2 format: deserialize nested frecency directly
            serde_json::from_value(frec_val.clone()).unwrap_or_default()
        } else {
            // v1 format: migrate from flat underscore-prefixed fields
            migrate_v1_frecency(&value)
        };

        // isPluginEntry can come from either _isPluginEntry (v1) or isPluginEntry (v2)
        let is_plugin_entry = value
            .get("isPluginEntry")
            .or_else(|| value.get("_isPluginEntry"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        Ok(IndexedItem {
            item,
            frecency,
            is_plugin_entry,
        })
    }
}

/// Migrate v1 flat underscore-prefixed frecency fields to v2 Frecency struct
// JSON u64 -> u32 for frecency counts (bounded by realistic usage)
#[allow(clippy::cast_possible_truncation)]
fn migrate_v1_frecency(value: &serde_json::Value) -> Frecency {
    let get_u32 = |key: &str| -> u32 {
        value
            .get(key)
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as u32
    };
    let get_u64 = |key: &str| -> u64 {
        value
            .get(key)
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
    };

    let recent_search_terms: Vec<String> = value
        .get("_recentSearchTerms")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let hour_slot_counts: [u32; 24] = value
        .get("_hourSlotCounts")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or([0; 24]);

    let day_of_week_counts: [u32; 7] = value
        .get("_dayOfWeekCounts")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or([0; 7]);

    let last_consecutive_date: Option<String> = value.get("_lastConsecutiveDate").and_then(|v| {
        if v.is_null() {
            None
        } else {
            serde_json::from_value(v.clone()).ok()
        }
    });

    let workspace_counts: HashMap<String, u32> = value
        .get("_workspaceCounts")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let monitor_counts: HashMap<String, u32> = value
        .get("_monitorCounts")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let launched_after: HashMap<String, u32> = value
        .get("_launchedAfter")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let display_count_counts: HashMap<String, u32> = value
        .get("_displayCountCounts")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let session_duration_counts: [u32; 5] = value
        .get("_sessionDurationCounts")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or([0; 5]);

    Frecency {
        count: get_u32("_count"),
        last_used: get_u64("_lastUsed"),
        recent_search_terms,
        hour_slot_counts,
        day_of_week_counts,
        consecutive_days: get_u32("_consecutiveDays"),
        last_consecutive_date,
        launch_from_empty_count: get_u32("_launchFromEmptyCount"),
        session_start_count: get_u32("_sessionStartCount"),
        workspace_counts,
        monitor_counts,
        launched_after,
        resume_from_idle_count: get_u32("_resumeFromIdleCount"),
        display_count_counts,
        session_duration_counts,
    }
}

impl IndexedItem {
    pub fn new(item: IndexItem) -> Self {
        Self {
            item,
            frecency: Frecency::default(),
            is_plugin_entry: false,
        }
    }

    pub fn id(&self) -> &str {
        &self.item.id
    }

    pub fn name(&self) -> &str {
        &self.item.name
    }

    pub fn effective_count(&self) -> u32 {
        self.frecency.count
    }

    pub fn effective_last_used(&self) -> u64 {
        self.frecency.last_used
    }
}

/// Cache structure for persistence (used by `load`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexCache {
    /// Cache format version (2 = nested frecency)
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(rename = "savedAt")]
    pub saved_at: u64,
    pub indexes: HashMap<String, PluginIndex>,
}

/// Borrowing variant of `IndexCache` for zero-copy serialization in `save`
#[derive(Serialize)]
pub(crate) struct IndexCacheRef<'a> {
    pub version: u32,
    #[serde(rename = "savedAt")]
    pub saved_at: u64,
    pub indexes: &'a HashMap<String, PluginIndex>,
}

fn default_version() -> u32 {
    2
}

impl Default for IndexCache {
    fn default() -> Self {
        Self {
            version: 2,
            saved_at: 0,
            indexes: HashMap::new(),
        }
    }
}
