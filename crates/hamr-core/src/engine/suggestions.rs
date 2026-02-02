//! Recent items and smart suggestions for the launcher.
//!
//! Builds the initial results shown when the launcher opens with an empty query.

use crate::frecency::{SmartSuggestions, SuggestionContext};
use crate::index::IndexedItem;
use crate::plugin::Manifest;
use hamr_types::{Action, ResultType, SearchResult};
use tracing::debug;

use super::HamrCore;

impl HamrCore {
    /// Get recent items and smart suggestions (uses cache if available).
    pub(super) fn get_recent_and_suggestions(&mut self) -> Vec<SearchResult> {
        if !self.state.cached_recent.is_empty() {
            return self.state.cached_recent.clone();
        }

        self.build_recent_and_suggestions()
    }

    /// Get list of all plugins sorted by frecency (for plugin list mode).
    pub(super) fn get_plugin_list(&self) -> Vec<SearchResult> {
        let mut plugin_results: Vec<(SearchResult, u32)> = self
            .plugins
            .all()
            .filter(|p| !p.manifest.hidden)
            .map(|plugin| {
                let frecency_score = self
                    .index
                    .get_item(&plugin.id, "__plugin__")
                    .map_or(0, |item| item.frecency.count);

                let result = SearchResult {
                    id: plugin.id.clone(),
                    name: plugin.manifest.name.clone(),
                    description: plugin.manifest.description.clone(),
                    icon: Some(
                        plugin
                            .manifest
                            .icon
                            .clone()
                            .unwrap_or_else(|| "extension".to_string()),
                    ),
                    icon_type: None,
                    verb: Some("Open".to_string()),
                    result_type: ResultType::Plugin,
                    plugin_id: Some(plugin.id.clone()),
                    ..Default::default()
                };

                (result, frecency_score)
            })
            .collect();

        plugin_results.sort_by(|a, b| b.1.cmp(&a.1));

        plugin_results.into_iter().map(|(r, _)| r).collect()
    }

    /// Rebuild the cached recent/suggestions list.
    /// Called on `LauncherClosed` so results are ready for next open.
    pub(super) fn rebuild_recent_cache(&mut self) {
        debug!("Rebuilding recent cache in background");
        self.state.cached_recent = self.build_recent_and_suggestions();
    }

    /// Invalidate the cached recent list (called when frecency changes).
    pub(super) fn invalidate_recent_cache(&mut self) {
        self.state.cached_recent.clear();
    }

    /// Build the recent and suggestions list (the actual computation).
    fn build_recent_and_suggestions(&self) -> Vec<SearchResult> {
        let mut results = Vec::new();

        let context = self.build_suggestion_context();
        let suggestions = SmartSuggestions::get_suggestions(
            &self.index,
            &context,
            2,
            self.config.search.suggestion_staleness_half_life_days,
            self.config.search.max_suggestion_age_days,
        );

        for suggestion in suggestions {
            if let Some(item) = self
                .index
                .get_item(&suggestion.plugin_id, &suggestion.item_id)
            {
                let reason = suggestion
                    .reasons
                    .first()
                    .map(SmartSuggestions::format_reason)
                    .unwrap_or_default();

                let mut result = indexed_item_to_search_result(
                    item,
                    &suggestion.plugin_id,
                    ResultType::Suggestion,
                    Some(reason),
                );
                result.verb = Some("Open".to_string());
                results.push(result);
            }
        }

        let recent = self.index.items_with_frecency();
        let max_recent = self.config.search.max_recent_items;

        for (plugin_id, item) in recent.into_iter().take(max_recent) {
            if item.is_plugin_entry {
                if results.iter().any(|r| r.id == plugin_id) {
                    continue;
                }

                if let Some(plugin) = self.plugins.get(plugin_id) {
                    results.push(plugin_to_search_result(plugin_id, &plugin.manifest));
                }
                continue;
            }

            if results.iter().any(|r| r.id == item.id()) {
                continue;
            }

            results.push(indexed_item_to_search_result(
                item,
                plugin_id,
                ResultType::Recent,
                None,
            ));
        }

        results
    }

    /// Build suggestion context from current time.
    // Time math: u64 secs -> usize weekday (0-6), u32 hour (0-23)
    #[allow(clippy::unused_self, clippy::cast_possible_truncation)]
    pub(super) fn build_suggestion_context(&self) -> SuggestionContext {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let secs_in_day = 86400u64;
        let days_since_epoch = now / secs_in_day;
        let weekday = ((days_since_epoch + 3) % 7) as usize;
        let secs_today = now % secs_in_day;
        let hour = (secs_today / 3600) as u32;

        SuggestionContext {
            hour,
            weekday,
            ..Default::default()
        }
    }
}

/// Convert an indexed item to a `SearchResult` with the given base result type.
/// Slider/Switch types are now derived from the `widget` field via `is_slider()/is_switch()`.
pub(super) fn indexed_item_to_search_result(
    item: &IndexedItem,
    plugin_id: &str,
    default_result_type: ResultType,
    suggestion_reason: Option<String>,
) -> SearchResult {
    let result_type = default_result_type;

    let actions: Vec<Action> = item
        .item
        .actions
        .iter()
        .map(|a| Action {
            id: a.id.clone(),
            name: a.name.clone(),
            icon: a.icon.clone(),
            icon_type: a.icon_type.clone(),
            keep_open: a.keep_open,
        })
        .collect();

    SearchResult {
        id: item.id().to_string(),
        name: item.name().to_string(),
        description: item.item.description.clone(),
        icon: Some(
            item.item
                .icon
                .clone()
                .unwrap_or_else(|| "extension".to_string()),
        ),
        icon_type: item.item.icon_type.clone(),
        thumbnail: item.item.thumbnail.clone(),
        verb: item.item.verb.clone(),
        result_type,
        plugin_id: Some(plugin_id.to_string()),
        app_id: item.item.app_id.clone(),
        app_id_fallback: item.item.app_id_fallback.clone(),
        actions,
        badges: item.item.badges.clone(),
        chips: item.item.chips.clone(),
        widget: item.item.widget.clone(),
        is_suggestion: suggestion_reason.is_some(),
        suggestion_reason,
        ..Default::default()
    }
}

/// Convert a plugin manifest to a `SearchResult` for plugin-entry items.
pub(super) fn plugin_to_search_result(plugin_id: &str, manifest: &Manifest) -> SearchResult {
    SearchResult {
        id: plugin_id.to_string(),
        name: manifest.name.clone(),
        description: manifest.description.clone(),
        icon: Some(
            manifest
                .icon
                .clone()
                .unwrap_or_else(|| "extension".to_string()),
        ),
        icon_type: Some("material".to_string()),
        verb: Some("Open".to_string()),
        result_type: ResultType::Recent,
        plugin_id: Some(plugin_id.to_string()),
        ..Default::default()
    }
}
