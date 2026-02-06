use super::{IndexCache, IndexedItem, PluginIndex};
use crate::Result;
use crate::engine::{DEFAULT_PLUGIN_ICON, DEFAULT_VERB_OPEN, ID_PLUGIN_ENTRY};
use crate::frecency::ExecutionContext;
use crate::plugin::{FrecencyMode, IndexItem};
use crate::search::{Searchable, SearchableSource};
use crate::utils::{date_string_from_epoch, now_millis, yesterday_string};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, error, info, warn};

/// Stores and manages plugin indexes
pub struct IndexStore {
    indexes: HashMap<String, PluginIndex>,
    dirty: bool,
    /// Timestamp (ms) when index was last modified - for debounced saving
    last_dirty_at: u64,
}

impl IndexStore {
    /// Create a new empty index store
    pub fn new() -> Self {
        Self {
            indexes: HashMap::new(),
            dirty: false,
            last_dirty_at: 0,
        }
    }

    /// Load indexes from cache file
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            debug!("Index cache not found at {}", path.display());
            return Ok(Self::new());
        }

        debug!("Loading index cache from {}", path.display());
        let content = std::fs::read_to_string(path)?;
        let cache: IndexCache = match serde_json::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                warn!(
                    "Failed to parse index cache: {} (at line {}, column {})",
                    e,
                    e.line(),
                    e.column()
                );
                return Ok(Self::new());
            }
        };

        let total_items: usize = cache.indexes.values().map(|p| p.items.len()).sum();
        let items_with_frecency: usize = cache
            .indexes
            .values()
            .flat_map(|p| p.items.iter())
            .filter(|i| i.effective_count() > 0)
            .count();
        info!(
            "Loaded {} plugin indexes from cache ({} total items, {} with frecency)",
            cache.indexes.len(),
            total_items,
            items_with_frecency
        );

        Ok(Self {
            indexes: cache.indexes,
            dirty: false,
            last_dirty_at: 0,
        })
    }

    /// Save indexes to cache file (always saves as v2 format)
    pub fn save(&mut self, path: &Path) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }

        let cache = IndexCache {
            version: 2,
            saved_at: now_millis(),
            indexes: self.indexes.clone(),
        };

        let content = serde_json::to_string(&cache)?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, content)?;
        self.dirty = false;

        debug!("Saved {} plugin indexes to cache", self.indexes.len());
        Ok(())
    }

    /// Update index for a plugin (full replace)
    pub fn update_full(&mut self, plugin_id: &str, items: Vec<IndexItem>) {
        let existing = self.indexes.get(plugin_id);

        let items = items
            .into_iter()
            .map(|item| {
                let existing_item =
                    existing.and_then(|idx| idx.items.iter().find(|i| i.id() == item.id));

                if let Some(ex) = existing_item {
                    IndexedItem {
                        item,
                        frecency: ex.frecency.clone(),
                        is_plugin_entry: ex.is_plugin_entry,
                    }
                } else {
                    IndexedItem::new(item)
                }
            })
            .collect();

        self.indexes.insert(
            plugin_id.to_string(),
            PluginIndex {
                items,
                last_indexed: now_millis(),
            },
        );
        self.mark_dirty();
    }

    /// Update index incrementally
    pub fn update_incremental(
        &mut self,
        plugin_id: &str,
        add_items: Vec<IndexItem>,
        remove_ids: Vec<String>,
    ) {
        let index = self.indexes.entry(plugin_id.to_string()).or_default();

        let remove_set: std::collections::HashSet<_> = remove_ids.into_iter().collect();
        index.items.retain(|item| !remove_set.contains(item.id()));

        for item in add_items {
            if let Some(existing) = index.items.iter_mut().find(|i| i.id() == item.id) {
                existing.item = item;
            } else {
                index.items.push(IndexedItem::new(item));
            }
        }

        index.last_indexed = now_millis();
        self.mark_dirty();
    }

    #[cfg(test)]
    pub fn patch_items(&mut self, plugin_id: &str, patches: Vec<(String, serde_json::Value)>) {
        let Some(index) = self.indexes.get_mut(plugin_id) else {
            return;
        };

        for (id, patch) in patches {
            if let Some(item) = index.items.iter_mut().find(|i| i.id() == id)
                && let Ok(mut current) = serde_json::to_value(&item.item)
            {
                if let serde_json::Value::Object(ref mut obj) = current
                    && let serde_json::Value::Object(patch_obj) = patch
                {
                    for (key, value) in patch_obj {
                        obj.insert(key, value);
                    }
                }
                if let Ok(updated) = serde_json::from_value(current) {
                    item.item = updated;
                }
            }
        }
        self.mark_dirty();
    }

    #[cfg(test)]
    pub fn get_items(&self, plugin_id: &str) -> &[IndexedItem] {
        self.indexes
            .get(plugin_id)
            .map_or(&[], |idx| idx.items.as_slice())
    }

    /// Get a specific item
    pub fn get_item(&self, plugin_id: &str, item_id: &str) -> Option<&IndexedItem> {
        self.indexes
            .get(plugin_id)
            .and_then(|idx| idx.items.iter().find(|i| i.id() == item_id))
    }

    /// Get a mutable reference to an item
    pub fn get_item_mut(&mut self, plugin_id: &str, item_id: &str) -> Option<&mut IndexedItem> {
        self.mark_dirty();
        self.indexes
            .get_mut(plugin_id)
            .and_then(|idx| idx.items.iter_mut().find(|i| i.id() == item_id))
    }

    /// Get all indexed plugin IDs
    pub fn plugin_ids(&self) -> impl Iterator<Item = &str> {
        self.indexes.keys().map(String::as_str)
    }

    #[cfg(test)]
    pub fn all_items(&self) -> impl Iterator<Item = (&str, &IndexedItem)> {
        self.indexes.iter().flat_map(|(plugin_id, index)| {
            index
                .items
                .iter()
                .filter(|item| item.id() != ID_PLUGIN_ENTRY)
                .map(move |item| (plugin_id.as_str(), item))
        })
    }

    /// Get items with frecency data (sorted by frecency)
    /// Includes both regular items AND `ID_PLUGIN_ENTRY` entries (for plugins with frecency: "plugin")
    pub fn items_with_frecency(&self) -> Vec<(&str, &IndexedItem)> {
        let mut items: Vec<_> = self
            .all_items_including_plugins()
            .filter(|(_, item)| item.effective_count() > 0)
            .collect();

        items.sort_by(|a, b| {
            let freq_a = self.calculate_frecency(a.1);
            let freq_b = self.calculate_frecency(b.1);
            freq_b
                .partial_cmp(&freq_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        items
    }

    /// Get all items including `ID_PLUGIN_ENTRY` entries (for frecency listing)
    fn all_items_including_plugins(&self) -> impl Iterator<Item = (&str, &IndexedItem)> {
        self.indexes.iter().flat_map(|(plugin_id, index)| {
            index
                .items
                .iter()
                .map(move |item| (plugin_id.as_str(), item))
        })
    }

    /// Calculate frecency score for an item
    // Time diff is u64 millis, convert to f64 hours for recency calculation
    #[allow(clippy::unused_self, clippy::cast_precision_loss)]
    pub fn calculate_frecency(&self, item: &IndexedItem) -> f64 {
        let count = f64::from(item.effective_count());
        if count == 0.0 {
            return 0.0;
        }

        let now = now_millis();
        let hours_since_use = (now - item.effective_last_used()) as f64 / (1000.0 * 60.0 * 60.0);

        let recency_multiplier = if hours_since_use < 1.0 {
            4.0
        } else if hours_since_use < 24.0 {
            2.0
        } else if hours_since_use < 168.0 {
            1.0
        } else {
            0.5
        };

        count * recency_multiplier
    }

    /// Record an item execution
    /// Updates frecency fields directly on item (with underscore prefix) to match QML hamr format
    pub fn record_execution(
        &mut self,
        plugin_id: &str,
        item_id: &str,
        context: &ExecutionContext,
        frecency_mode: Option<&FrecencyMode>,
    ) {
        self.record_execution_with_item(plugin_id, item_id, context, frecency_mode, None);
    }

    /// Record execution with optional item data for auto-indexing
    /// If the item doesn't exist in the index and `fallback_item` is provided,
    /// the item will be added to the index automatically.
    pub fn record_execution_with_item(
        &mut self,
        plugin_id: &str,
        item_id: &str,
        context: &ExecutionContext,
        frecency_mode: Option<&FrecencyMode>,
        fallback_item: Option<&hamr_types::ResultItem>,
    ) {
        let mode = frecency_mode.unwrap_or(&FrecencyMode::Item);

        match mode {
            FrecencyMode::None => return,
            FrecencyMode::Plugin => {
                self.record_plugin_execution(plugin_id, context);
                return;
            }
            FrecencyMode::Item => {}
        }

        if item_id == ID_PLUGIN_ENTRY {
            return;
        }

        if self.get_item(plugin_id, item_id).is_none() {
            if let Some(result_item) = fallback_item {
                let index = self.indexes.entry(plugin_id.to_string()).or_default();
                let new_item = IndexedItem::new(result_item.clone());
                debug!("Auto-indexing item from results: {}/{}", plugin_id, item_id);
                index.items.push(new_item);
            } else {
                warn!(
                    "Cannot record execution: item not found {}/{}",
                    plugin_id, item_id
                );
                return;
            }
        }

        let Some(item) = self.get_item_mut(plugin_id, item_id) else {
            error!(
                "Item unexpectedly missing after insertion: {}/{}",
                plugin_id, item_id
            );
            return;
        };
        Self::update_item_frecency(item, context);
        let count = item.frecency.count;
        self.mark_dirty();
        debug!(
            "Recorded execution: {}/{} (count={}, mode=item)",
            plugin_id, item_id, count
        );
    }

    /// Record plugin-level frecency (for frecency: "plugin" mode)
    fn record_plugin_execution(&mut self, plugin_id: &str, context: &ExecutionContext) {
        let index = self.indexes.entry(plugin_id.to_string()).or_default();

        let plugin_entry = index.items.iter_mut().find(|i| i.id() == ID_PLUGIN_ENTRY);

        let count = if let Some(item) = plugin_entry {
            Self::update_item_frecency(item, context);
            item.frecency.count
        } else {
            let mut new_item = IndexedItem::new(hamr_types::ResultItem {
                id: ID_PLUGIN_ENTRY.to_string(),
                name: plugin_id.to_string(),
                icon: Some(DEFAULT_PLUGIN_ICON.to_string()),
                verb: Some(DEFAULT_VERB_OPEN.to_string()),
                ..Default::default()
            });
            new_item.is_plugin_entry = true;
            Self::update_item_frecency(&mut new_item, context);
            let c = new_item.frecency.count;
            index.items.push(new_item);
            c
        };

        self.mark_dirty();
        debug!(
            "Recorded execution: {}/{} (count={}, mode=plugin)",
            plugin_id, ID_PLUGIN_ENTRY, count
        );
    }

    /// Update frecency fields on an item (using unified frecency struct)
    fn update_item_frecency(item: &mut IndexedItem, context: &ExecutionContext) {
        let now = now_millis();
        let frec = &mut item.frecency;

        frec.count += 1;
        frec.last_used = now;

        if let Some(ref term) = context.search_term
            && !term.is_empty()
        {
            frec.recent_search_terms.retain(|t| t != term);
            frec.recent_search_terms.insert(0, term.clone());
            frec.recent_search_terms.truncate(10);
        }

        let now_dt = chrono_lite_now();
        frec.hour_slot_counts[now_dt.hour as usize] += 1;
        frec.day_of_week_counts[now_dt.weekday as usize] += 1;

        if context.launch_from_empty {
            frec.launch_from_empty_count += 1;
        }

        if context.is_session_start {
            frec.session_start_count += 1;
        }

        if context.is_resume_from_idle {
            frec.resume_from_idle_count += 1;
        }

        if let Some(ref workspace) = context.workspace {
            *frec.workspace_counts.entry(workspace.clone()).or_insert(0) += 1;
        }

        if let Some(ref monitor) = context.monitor {
            *frec.monitor_counts.entry(monitor.clone()).or_insert(0) += 1;
        }

        if let Some(ref last_app) = context.last_app {
            *frec.launched_after.entry(last_app.clone()).or_insert(0) += 1;
            if frec.launched_after.len() > 5 {
                let mut entries: Vec<_> = frec.launched_after.drain().collect();
                entries.sort_by(|a, b| b.1.cmp(&a.1));
                entries.truncate(5);
                frec.launched_after = entries.into_iter().collect();
            }
        }

        if let Some(display_count) = context.display_count {
            let key = display_count.to_string();
            *frec.display_count_counts.entry(key).or_insert(0) += 1;
        }

        if let Some(bucket) = context.session_duration_bucket
            && (bucket as usize) < 5
        {
            frec.session_duration_counts[bucket as usize] += 1;
        }

        let today = now_dt.date_string();
        if frec.last_consecutive_date.as_deref() != Some(&today) {
            let yesterday = yesterday_string();
            if frec.last_consecutive_date.as_deref() == Some(&yesterday) {
                frec.consecutive_days += 1;
            } else {
                frec.consecutive_days = 1;
            }
            frec.last_consecutive_date = Some(today);
        }
    }

    /// Build searchables from all indexed items
    pub fn build_searchables(&self, _plugin_name_map: &HashMap<String, String>) -> Vec<Searchable> {
        let mut searchables = Vec::new();

        for (plugin_id, index) in &self.indexes {
            for item in &index.items {
                if item.id() == ID_PLUGIN_ENTRY {
                    continue;
                }

                searchables.push(Searchable {
                    id: item.id().to_string(),
                    name: item.name().to_string(),
                    keywords: item.item.keywords.clone().unwrap_or_default(),
                    source: SearchableSource::IndexedItem {
                        plugin_id: plugin_id.clone(),
                        item: item.item.clone(),
                    },
                    is_history_term: false,
                });

                for term in &item.frecency.recent_search_terms {
                    searchables.push(Searchable {
                        id: item.id().to_string(),
                        name: term.clone(),
                        keywords: Vec::new(),
                        source: SearchableSource::IndexedItem {
                            plugin_id: plugin_id.clone(),
                            item: item.item.clone(),
                        },
                        is_history_term: true,
                    });
                }
            }
        }

        searchables
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn last_dirty_at(&self) -> u64 {
        self.last_dirty_at
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.last_dirty_at = now_millis();
    }

    pub fn stats(&self) -> crate::engine::IndexStats {
        let mut items_per_plugin: Vec<(String, usize)> = self
            .indexes
            .iter()
            .map(|(id, index)| (id.clone(), index.items.len()))
            .collect();

        items_per_plugin.sort_by(|a, b| b.1.cmp(&a.1));

        crate::engine::IndexStats {
            plugin_count: self.indexes.len(),
            item_count: items_per_plugin.iter().map(|(_, c)| c).sum(),
            items_per_plugin,
        }
    }
}

impl Default for IndexStore {
    fn default() -> Self {
        Self::new()
    }
}

struct SimpleDt {
    hour: u32,
    weekday: usize,
    date_string: String,
}

impl SimpleDt {
    fn date_string(&self) -> String {
        self.date_string.clone()
    }
}

// Time calculations: u64 secs -> u32 hour, usize weekday
#[allow(clippy::cast_possible_truncation)]
fn chrono_lite_now() -> SimpleDt {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let seconds_today = now % 86400;
    let hour = (seconds_today / 3600) as u32;
    let days_since_epoch = now / 86400;
    let weekday = ((days_since_epoch + 3) % 7) as usize;
    let date_string = date_string_from_epoch(now);

    SimpleDt {
        hour,
        weekday,
        date_string,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // Exact float comparisons are intentional in tests
mod tests {
    use super::*;
    use crate::frecency::ExecutionContext;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_index_store_new() {
        let store = IndexStore::new();
        assert!(!store.is_dirty());
        assert_eq!(store.last_dirty_at(), 0);
    }

    #[test]
    fn test_index_store_default() {
        let store = IndexStore::default();
        assert!(!store.is_dirty());
    }

    #[test]
    fn test_load_nonexistent_path() {
        let result = IndexStore::load(Path::new("/nonexistent/path/index.json"));
        assert!(result.is_ok(), "Should succeed with empty store");
        let store = result.unwrap();
        assert!(!store.is_dirty());
    }

    #[test]
    fn test_load_invalid_json() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{{invalid json}}").unwrap();
        let result = IndexStore::load(file.path());
        assert!(result.is_ok(), "Should handle invalid JSON gracefully");
    }

    #[test]
    fn test_load_empty_file() {
        let file = NamedTempFile::new().unwrap();
        let result = IndexStore::load(file.path());
        assert!(result.is_ok(), "Should handle empty file");
    }

    #[test]
    fn test_save_when_not_dirty() {
        let mut store = IndexStore::new();
        let file = NamedTempFile::new().unwrap();
        let result = store.save(file.path());
        assert!(result.is_ok());
        let content = std::fs::read_to_string(file.path()).unwrap_or_default();
        assert!(content.is_empty(), "Should not write when not dirty");
    }

    #[test]
    fn test_update_full_empty_items() {
        let mut store = IndexStore::new();
        store.update_full("test_plugin", vec![]);
        assert!(store.is_dirty());
        assert!(store.get_items("test_plugin").is_empty());
    }

    #[test]
    fn test_update_incremental_empty_add_remove() {
        let mut store = IndexStore::new();
        store.update_incremental("test_plugin", vec![], vec![]);
        assert!(store.is_dirty());
    }

    #[test]
    fn test_update_incremental_remove_nonexistent() {
        let mut store = IndexStore::new();
        store.update_incremental("test_plugin", vec![], vec!["nonexistent".to_string()]);
        assert!(store.get_items("test_plugin").is_empty());
    }

    #[test]
    fn test_get_item_nonexistent_plugin() {
        let store = IndexStore::new();
        assert!(store.get_item("nonexistent", "item").is_none());
    }

    #[test]
    fn test_get_item_nonexistent_item() {
        let mut store = IndexStore::new();
        store.update_full(
            "plugin",
            vec![hamr_types::ResultItem {
                id: "item1".to_string(),
                name: "Item 1".to_string(),
                ..Default::default()
            }],
        );
        assert!(store.get_item("plugin", "nonexistent").is_none());
    }

    #[test]
    fn test_plugin_ids_empty() {
        let store = IndexStore::new();
        assert_eq!(store.plugin_ids().count(), 0);
    }

    #[test]
    fn test_calculate_frecency_zero_count() {
        let store = IndexStore::new();
        let item = IndexedItem::new(hamr_types::ResultItem {
            id: "test".to_string(),
            name: "Test".to_string(),
            ..Default::default()
        });
        let frecency = store.calculate_frecency(&item);
        assert_eq!(frecency, 0.0, "Zero count should give zero frecency");
    }

    #[test]
    fn test_calculate_frecency_very_old_item() {
        let store = IndexStore::new();
        let mut item = IndexedItem::new(hamr_types::ResultItem {
            id: "test".to_string(),
            name: "Test".to_string(),
            ..Default::default()
        });
        item.frecency.count = 10;
        item.frecency.last_used = 1000;
        let frecency = store.calculate_frecency(&item);
        assert!(frecency > 0.0, "Should have some frecency");
        assert!(frecency < 10.0, "Very old item should have low frecency");
    }

    #[test]
    fn test_record_execution_mode_none() {
        let mut store = IndexStore::new();
        store.update_full(
            "plugin",
            vec![hamr_types::ResultItem {
                id: "item".to_string(),
                name: "Item".to_string(),
                ..Default::default()
            }],
        );
        let initial_count = store.get_item("plugin", "item").unwrap().frecency.count;
        store.record_execution(
            "plugin",
            "item",
            &ExecutionContext::default(),
            Some(&FrecencyMode::None),
        );
        let final_count = store.get_item("plugin", "item").unwrap().frecency.count;
        assert_eq!(initial_count, final_count, "mode=none should not update");
    }

    #[test]
    fn test_record_execution_mode_plugin() {
        let mut store = IndexStore::new();
        store.record_execution(
            "plugin",
            "any_item",
            &ExecutionContext::default(),
            Some(&FrecencyMode::Plugin),
        );
        assert!(
            store.get_item("plugin", ID_PLUGIN_ENTRY).is_some(),
            "plugin mode should create ID_PLUGIN_ENTRY entry"
        );
    }

    #[test]
    fn test_record_execution_item_not_found_no_fallback() {
        let mut store = IndexStore::new();
        store.record_execution("plugin", "nonexistent", &ExecutionContext::default(), None);
        assert!(
            store.get_item("plugin", "nonexistent").is_none(),
            "Should not create item without fallback"
        );
    }

    #[test]
    fn test_record_execution_with_fallback() {
        let mut store = IndexStore::new();
        let fallback = hamr_types::ResultItem {
            id: "new_item".to_string(),
            name: "New Item".to_string(),
            ..Default::default()
        };
        store.record_execution_with_item(
            "plugin",
            "new_item",
            &ExecutionContext::default(),
            None,
            Some(&fallback),
        );
        assert!(
            store.get_item("plugin", "new_item").is_some(),
            "Should create item from fallback"
        );
    }

    #[test]
    fn test_items_with_frecency_empty() {
        let store = IndexStore::new();
        let items = store.items_with_frecency();
        assert!(items.is_empty());
    }

    #[test]
    fn test_items_with_frecency_filters_zero_count() {
        let mut store = IndexStore::new();
        store.update_full(
            "plugin",
            vec![hamr_types::ResultItem {
                id: "item".to_string(),
                name: "Item".to_string(),
                ..Default::default()
            }],
        );
        let items = store.items_with_frecency();
        assert!(items.is_empty(), "Should filter items with zero frecency");
    }

    #[test]
    fn test_build_searchables_empty() {
        let store = IndexStore::new();
        let searchables = store.build_searchables(&HashMap::new());
        assert!(searchables.is_empty());
    }

    #[test]
    fn test_build_searchables_excludes_plugin_entry() {
        let mut store = IndexStore::new();
        store.record_execution(
            "plugin",
            "ignored",
            &ExecutionContext::default(),
            Some(&FrecencyMode::Plugin),
        );
        let searchables = store.build_searchables(&HashMap::new());
        assert!(
            !searchables.iter().any(|s| s.id == ID_PLUGIN_ENTRY),
            "Should exclude ID_PLUGIN_ENTRY entries"
        );
    }

    #[test]
    fn test_stats_empty() {
        let store = IndexStore::new();
        let stats = store.stats();
        assert_eq!(stats.plugin_count, 0);
        assert_eq!(stats.item_count, 0);
        assert!(stats.items_per_plugin.is_empty());
    }

    #[test]
    fn test_now_millis_reasonable() {
        let now = now_millis();
        assert!(now > 1_700_000_000_000, "Timestamp should be after 2023");
    }

    #[test]
    fn test_chrono_lite_now_valid_values() {
        let dt = chrono_lite_now();
        assert!(dt.hour < 24, "Hour should be 0-23");
        assert!(dt.weekday < 7, "Weekday should be 0-6");
        assert!(
            !dt.date_string.is_empty(),
            "Date string should not be empty"
        );
    }

    #[test]
    fn test_yesterday_string_different_from_today() {
        let dt = chrono_lite_now();
        let yesterday = yesterday_string();
        assert_ne!(
            dt.date_string, yesterday,
            "Yesterday should differ from today"
        );
    }
}
