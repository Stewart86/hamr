//! Tests for `IndexStore`: persistence, frecency tracking, searchable building
//!
//! Tests the index storage system including:
//! - Save/load round trips
//! - Full and incremental index updates
//! - Frecency recording and calculation
//! - History term generation
//! - Plugin-level vs item-level frecency modes

use super::fixtures::*;
use crate::frecency::ExecutionContext;
use crate::index::IndexStore;
use crate::plugin::FrecencyMode;
use crate::search::SearchableSource;
use tempfile::NamedTempFile;

#[test]
fn test_index_store_new() {
    let store = IndexStore::new();
    assert_eq!(store.plugin_ids().count(), 0, "New store should be empty");
    assert!(!store.is_dirty(), "New store should not be dirty");
}

#[test]
fn test_index_store_update_full() {
    let mut store = IndexStore::new();

    let items = vec![
        make_index_item("app1", "App One"),
        make_index_item("app2", "App Two"),
    ];

    store.update_full("apps", items);

    assert!(store.is_dirty(), "Store should be dirty after update");
    assert_eq!(store.get_items("apps").len(), 2);
    assert!(store.get_item("apps", "app1").is_some());
    assert!(store.get_item("apps", "app2").is_some());
}

#[test]
fn test_index_store_update_full_preserves_frecency() {
    let mut store = IndexStore::new();

    store.update_full("apps", vec![make_index_item("app1", "App One")]);

    let context = ExecutionContext {
        search_term: Some("app".to_string()),
        ..Default::default()
    };
    store.record_execution("apps", "app1", &context, None);

    let mut new_item = make_index_item("app1", "App One Updated");
    new_item.description = Some("New description".to_string());
    store.update_full("apps", vec![new_item]);

    let item = store.get_item("apps", "app1").unwrap();
    assert_eq!(item.frecency.count, 1, "Frecency count should be preserved");
    assert!(item.frecency.last_used > 0, "Last used should be preserved");
    assert_eq!(
        item.item.description,
        Some("New description".to_string()),
        "Item data should be updated"
    );
}

#[test]
fn test_index_store_update_incremental() {
    let mut store = IndexStore::new();

    store.update_full(
        "apps",
        vec![
            make_index_item("app1", "App One"),
            make_index_item("app2", "App Two"),
            make_index_item("app3", "App Three"),
        ],
    );

    store.update_incremental(
        "apps",
        vec![make_index_item("app4", "App Four")],
        vec!["app2".to_string()],
    );

    assert!(store.get_item("apps", "app1").is_some());
    assert!(
        store.get_item("apps", "app2").is_none(),
        "app2 should be removed"
    );
    assert!(store.get_item("apps", "app3").is_some());
    assert!(
        store.get_item("apps", "app4").is_some(),
        "app4 should be added"
    );
}

#[test]
fn test_index_store_update_incremental_updates_existing() {
    let mut store = IndexStore::new();

    store.update_full("apps", vec![make_index_item("app1", "Old Name")]);

    let mut updated = make_index_item("app1", "New Name");
    updated.description = Some("Updated description".to_string());
    store.update_incremental("apps", vec![updated], vec![]);

    let item = store.get_item("apps", "app1").unwrap();
    assert_eq!(item.name(), "New Name");
    assert_eq!(
        item.item.description,
        Some("Updated description".to_string())
    );
}

#[test]
fn test_index_store_patch_items() {
    use hamr_types::WidgetData;
    let mut store = IndexStore::new();

    let item = make_index_item("slider1", "Volume").with_slider(50.0, 0.0, 100.0, 1.0, None);
    store.update_full("sound", vec![item]);

    // Patching widget field directly
    let patch = serde_json::json!({
        "widget": {
            "type": "slider",
            "value": 75.0,
            "min": 0.0,
            "max": 100.0,
            "step": 1.0
        }
    });
    store.patch_items("sound", vec![("slider1".to_string(), patch)]);

    let item = store.get_item("sound", "slider1").unwrap();
    let Some(WidgetData::Slider { value, .. }) = &item.item.widget else {
        panic!("Expected Slider widget");
    };
    assert_eq!(*value, 75.0, "Value should be patched");
}

#[test]
fn test_index_store_get_items_nonexistent() {
    let store = IndexStore::new();
    let items = store.get_items("nonexistent");
    assert!(items.is_empty(), "Nonexistent plugin should return empty");
}

#[test]
fn test_index_store_get_item_nonexistent() {
    let store = IndexStore::new();
    assert!(store.get_item("apps", "nonexistent").is_none());
}

#[test]
fn test_index_store_save_load_roundtrip() {
    let mut store = IndexStore::new();

    let mut item = make_index_item("firefox", "Firefox");
    item.keywords = Some(vec!["browser".to_string(), "web".to_string()]);
    store.update_full("apps", vec![item]);

    let context = ExecutionContext {
        search_term: Some("fire".to_string()),
        launch_from_empty: true,
        workspace: Some("dev".to_string()),
        ..Default::default()
    };
    store.record_execution("apps", "firefox", &context, None);

    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();
    store.save(path).unwrap();

    let loaded = IndexStore::load(path).unwrap();

    let item = loaded.get_item("apps", "firefox").unwrap();
    assert_eq!(item.name(), "Firefox");
    assert_eq!(item.frecency.count, 1);
    assert!(item.frecency.last_used > 0);
    assert!(
        item.frecency
            .recent_search_terms
            .contains(&"fire".to_string())
    );
    assert_eq!(item.frecency.launch_from_empty_count, 1);
    assert_eq!(item.frecency.workspace_counts.get("dev"), Some(&1));
}

#[test]
fn test_index_store_load_nonexistent() {
    let path = std::path::Path::new("/nonexistent/path/to/cache.json");
    let store = IndexStore::load(path).unwrap();
    assert_eq!(store.plugin_ids().count(), 0, "Should create empty store");
}

#[test]
fn test_index_store_save_creates_directory() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("subdir").join("cache.json");

    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);
    store.save(&path).unwrap();

    assert!(path.exists(), "File should be created");
}

#[test]
fn test_index_store_save_not_dirty() {
    let mut store = IndexStore::new();

    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();

    store.save(path).unwrap();

    assert!(!store.is_dirty());
}

#[test]
fn test_record_execution_basic() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);

    let context = ExecutionContext::default();
    store.record_execution("apps", "app1", &context, None);

    let item = store.get_item("apps", "app1").unwrap();
    assert_eq!(item.frecency.count, 1);
    assert!(item.frecency.last_used > 0);
}

#[test]
fn test_record_execution_increments_count() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);

    let context = ExecutionContext::default();
    store.record_execution("apps", "app1", &context, None);
    store.record_execution("apps", "app1", &context, None);
    store.record_execution("apps", "app1", &context, None);

    let item = store.get_item("apps", "app1").unwrap();
    assert_eq!(item.frecency.count, 3);
}

#[test]
fn test_record_execution_search_term() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("firefox", "Firefox")]);

    let context = ExecutionContext {
        search_term: Some("fire".to_string()),
        ..Default::default()
    };
    store.record_execution("apps", "firefox", &context, None);

    let item = store.get_item("apps", "firefox").unwrap();
    assert!(
        item.frecency
            .recent_search_terms
            .contains(&"fire".to_string())
    );
}

#[test]
fn test_record_execution_search_terms_limited() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);

    for i in 0..15 {
        let context = ExecutionContext {
            search_term: Some(format!("term{i}")),
            ..Default::default()
        };
        store.record_execution("apps", "app1", &context, None);
    }

    let item = store.get_item("apps", "app1").unwrap();
    assert!(
        item.frecency.recent_search_terms.len() <= 10,
        "Should be limited to 10 terms"
    );
    assert!(item.frecency.recent_search_terms[0].contains("14"));
}

#[test]
fn test_record_execution_search_term_deduplication() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);

    let context = ExecutionContext {
        search_term: Some("fire".to_string()),
        ..Default::default()
    };

    store.record_execution("apps", "app1", &context, None);
    store.record_execution("apps", "app1", &context, None);
    store.record_execution("apps", "app1", &context, None);

    let item = store.get_item("apps", "app1").unwrap();
    let fire_count = item
        .frecency
        .recent_search_terms
        .iter()
        .filter(|t| *t == "fire")
        .count();
    assert_eq!(fire_count, 1, "Duplicate terms should be deduplicated");
}

#[test]
fn test_record_execution_launch_from_empty() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);

    let context = ExecutionContext {
        launch_from_empty: true,
        ..Default::default()
    };
    store.record_execution("apps", "app1", &context, None);

    let item = store.get_item("apps", "app1").unwrap();
    assert_eq!(item.frecency.launch_from_empty_count, 1);
}

#[test]
fn test_record_execution_session_start() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);

    let context = ExecutionContext {
        is_session_start: true,
        ..Default::default()
    };
    store.record_execution("apps", "app1", &context, None);

    let item = store.get_item("apps", "app1").unwrap();
    assert_eq!(item.frecency.session_start_count, 1);
}

#[test]
fn test_record_execution_resume_from_idle() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);

    let context = ExecutionContext {
        is_resume_from_idle: true,
        ..Default::default()
    };
    store.record_execution("apps", "app1", &context, None);

    let item = store.get_item("apps", "app1").unwrap();
    assert_eq!(item.frecency.resume_from_idle_count, 1);
}

#[test]
fn test_record_execution_workspace() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);

    let context = ExecutionContext {
        workspace: Some("dev".to_string()),
        ..Default::default()
    };
    store.record_execution("apps", "app1", &context, None);
    store.record_execution("apps", "app1", &context, None);

    let context2 = ExecutionContext {
        workspace: Some("default".to_string()),
        ..Default::default()
    };
    store.record_execution("apps", "app1", &context2, None);

    let item = store.get_item("apps", "app1").unwrap();
    assert_eq!(item.frecency.workspace_counts.get("dev"), Some(&2));
    assert_eq!(item.frecency.workspace_counts.get("default"), Some(&1));
}

#[test]
fn test_record_execution_monitor() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);

    let context = ExecutionContext {
        monitor: Some("eDP-1".to_string()),
        ..Default::default()
    };
    store.record_execution("apps", "app1", &context, None);

    let item = store.get_item("apps", "app1").unwrap();
    assert_eq!(item.frecency.monitor_counts.get("eDP-1"), Some(&1));
}

#[test]
fn test_record_execution_last_app() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("vscode", "VSCode")]);

    let context = ExecutionContext {
        last_app: Some("firefox.desktop".to_string()),
        ..Default::default()
    };
    store.record_execution("apps", "vscode", &context, None);
    store.record_execution("apps", "vscode", &context, None);

    let item = store.get_item("apps", "vscode").unwrap();
    assert_eq!(
        item.frecency.launched_after.get("firefox.desktop"),
        Some(&2)
    );
}

#[test]
fn test_record_execution_launched_after_limited() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);

    for i in 0..10 {
        let context = ExecutionContext {
            last_app: Some(format!("app{i}.desktop")),
            ..Default::default()
        };
        store.record_execution("apps", "app1", &context, None);
    }

    let item = store.get_item("apps", "app1").unwrap();
    assert!(
        item.frecency.launched_after.len() <= 5,
        "Should keep only top 5 sequence associations"
    );
}

#[test]
fn test_record_execution_display_count() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);

    let context = ExecutionContext {
        display_count: Some(2),
        ..Default::default()
    };
    store.record_execution("apps", "app1", &context, None);

    let item = store.get_item("apps", "app1").unwrap();
    assert_eq!(item.frecency.display_count_counts.get("2"), Some(&1));
}

#[test]
fn test_record_execution_session_duration_bucket() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);

    let context = ExecutionContext {
        session_duration_bucket: Some(2),
        ..Default::default()
    };
    store.record_execution("apps", "app1", &context, None);

    let item = store.get_item("apps", "app1").unwrap();
    assert_eq!(item.frecency.session_duration_counts[2], 1);
}

#[test]
fn test_record_execution_hour_slot() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);

    let context = ExecutionContext::default();
    store.record_execution("apps", "app1", &context, None);

    let item = store.get_item("apps", "app1").unwrap();
    let total: u32 = item.frecency.hour_slot_counts.iter().sum();
    assert_eq!(total, 1, "One hour slot should be incremented");
}

#[test]
fn test_record_execution_day_of_week() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);

    let context = ExecutionContext::default();
    store.record_execution("apps", "app1", &context, None);

    let item = store.get_item("apps", "app1").unwrap();
    let total: u32 = item.frecency.day_of_week_counts.iter().sum();
    assert_eq!(total, 1, "One day slot should be incremented");
}

#[test]
fn test_record_execution_consecutive_days_first_use() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);

    let context = ExecutionContext::default();
    store.record_execution("apps", "app1", &context, None);

    let item = store.get_item("apps", "app1").unwrap();
    assert_eq!(
        item.frecency.consecutive_days, 1,
        "First use should set streak to 1"
    );
    assert!(item.frecency.last_consecutive_date.is_some());
}

#[test]
fn test_record_execution_same_day_no_increment() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);

    let context = ExecutionContext::default();
    store.record_execution("apps", "app1", &context, None);
    store.record_execution("apps", "app1", &context, None);

    let item = store.get_item("apps", "app1").unwrap();
    assert_eq!(
        item.frecency.consecutive_days, 1,
        "Same day should not increment streak"
    );
}

#[test]
fn test_record_execution_frecency_none() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);

    let context = ExecutionContext::default();
    store.record_execution("apps", "app1", &context, Some(&FrecencyMode::None));

    let item = store.get_item("apps", "app1").unwrap();
    assert_eq!(item.frecency.count, 0, "frecency: none should not record");
}

#[test]
fn test_record_execution_frecency_plugin() {
    let mut store = IndexStore::new();
    store.update_full("notes", vec![make_index_item("note1", "Note")]);

    let context = ExecutionContext::default();
    store.record_execution("notes", "note1", &context, Some(&FrecencyMode::Plugin));

    let plugin_entry = store.get_item("notes", "__plugin__");
    assert!(plugin_entry.is_some(), "Should create __plugin__ entry");
    assert_eq!(plugin_entry.unwrap().frecency.count, 1);

    let item = store.get_item("notes", "note1").unwrap();
    assert_eq!(
        item.frecency.count, 0,
        "Item frecency should not be tracked in plugin mode"
    );
}

#[test]
fn test_record_execution_plugin_entry_skipped_for_item_mode() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);

    let context = ExecutionContext::default();
    store.record_execution("apps", "__plugin__", &context, Some(&FrecencyMode::Item));

    let plugin_entry = store.get_item("apps", "__plugin__");
    assert!(plugin_entry.is_none());
}

#[test]
fn test_calculate_frecency_zero_count() {
    let store = IndexStore::new();
    let item = make_indexed_item("app1", "App");
    let frecency = store.calculate_frecency(&item);
    assert_eq!(frecency, 0.0, "Zero count should give zero frecency");
}

#[test]
fn test_calculate_frecency_recent_usage() {
    let store = IndexStore::new();

    let recent = make_indexed_item_with_frecency("recent", "Recent", 10, hours_ago(0));
    let frecency = store.calculate_frecency(&recent);
    assert!(
        (frecency - 40.0).abs() < 1.0,
        "Count 10 * 4x = 40, got {frecency}"
    );

    let today = make_indexed_item_with_frecency("today", "Today", 10, hours_ago(12));
    let frecency = store.calculate_frecency(&today);
    assert!(
        (frecency - 20.0).abs() < 1.0,
        "Count 10 * 2x = 20, got {frecency}"
    );

    let week = make_indexed_item_with_frecency("week", "Week", 10, days_ago(3));
    let frecency = store.calculate_frecency(&week);
    assert!(
        (frecency - 10.0).abs() < 1.0,
        "Count 10 * 1x = 10, got {frecency}"
    );

    let old = make_indexed_item_with_frecency("old", "Old", 10, days_ago(14));
    let frecency = store.calculate_frecency(&old);
    assert!(
        (frecency - 5.0).abs() < 1.0,
        "Count 10 * 0.5x = 5, got {frecency}"
    );
}

#[test]
fn test_items_with_frecency_sorted() {
    let mut store = IndexStore::new();

    store.update_full(
        "apps",
        vec![
            make_index_item("low", "Low Usage"),
            make_index_item("high", "High Usage"),
            make_index_item("medium", "Medium Usage"),
        ],
    );

    let context = ExecutionContext::default();
    for _ in 0..10 {
        store.record_execution("apps", "high", &context, None);
    }
    for _ in 0..5 {
        store.record_execution("apps", "medium", &context, None);
    }
    store.record_execution("apps", "low", &context, None);

    let items = store.items_with_frecency();

    assert!(items.len() == 3);
    assert_eq!(items[0].1.id(), "high", "Highest frecency should be first");
    assert_eq!(items[1].1.id(), "medium");
    assert_eq!(items[2].1.id(), "low", "Lowest frecency should be last");
}

#[test]
fn test_build_searchables_basic() {
    let mut store = IndexStore::new();

    store.update_full(
        "apps",
        vec![
            make_index_item("firefox", "Firefox"),
            make_index_item("chrome", "Chrome"),
        ],
    );

    let searchables = store.build_searchables();

    assert_eq!(searchables.len(), 2);
    assert!(searchables.iter().any(|s| s.id == "firefox"));
    assert!(searchables.iter().any(|s| s.id == "chrome"));
}

#[test]
fn test_build_searchables_with_keywords() {
    let mut store = IndexStore::new();

    let mut item = make_index_item("firefox", "Firefox");
    item.keywords = Some(vec!["browser".to_string(), "web".to_string()]);
    store.update_full("apps", vec![item]);

    let searchables = store.build_searchables();

    let firefox = searchables.iter().find(|s| s.id == "firefox").unwrap();
    assert!(firefox.keywords.contains(&"browser".to_string()));
    assert!(firefox.keywords.contains(&"web".to_string()));
}

#[test]
fn test_build_searchables_history_terms() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("firefox", "Firefox")]);

    let context1 = ExecutionContext {
        search_term: Some("fire".to_string()),
        ..Default::default()
    };
    store.record_execution("apps", "firefox", &context1, None);

    let context2 = ExecutionContext {
        search_term: Some("ff".to_string()),
        ..Default::default()
    };
    store.record_execution("apps", "firefox", &context2, None);

    let searchables = store.build_searchables();

    assert_eq!(searchables.len(), 3);

    let history_terms: Vec<_> = searchables.iter().filter(|s| s.is_history_term).collect();
    assert_eq!(history_terms.len(), 2);

    assert!(
        history_terms.iter().any(|s| s.name == "fire"),
        "Should have history term 'fire'"
    );
    assert!(
        history_terms.iter().any(|s| s.name == "ff"),
        "Should have history term 'ff'"
    );
}

#[test]
fn test_build_searchables_skips_plugin_entry() {
    let mut store = IndexStore::new();
    store.update_full("notes", vec![make_index_item("note1", "Note")]);

    let context = ExecutionContext::default();
    store.record_execution("notes", "note1", &context, Some(&FrecencyMode::Plugin));

    let searchables = store.build_searchables();

    assert!(
        !searchables.iter().any(|s| s.id == "__plugin__"),
        "__plugin__ entries should be skipped"
    );
}

#[test]
fn test_build_searchables_source_has_plugin_id() {
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "App")]);

    let searchables = store.build_searchables();

    let app = searchables.iter().find(|s| s.id == "app1").unwrap();
    match &app.source {
        SearchableSource::IndexedItem { plugin_id, .. } => {
            assert_eq!(plugin_id, "apps");
        }
        SearchableSource::Plugin { .. } => panic!("Expected IndexedItem source"),
    }
}

#[test]
fn test_index_store_stats() {
    let mut store = IndexStore::new();

    store.update_full(
        "apps",
        vec![
            make_index_item("app1", "App 1"),
            make_index_item("app2", "App 2"),
            make_index_item("app3", "App 3"),
        ],
    );

    store.update_full("notes", vec![make_index_item("note1", "Note 1")]);

    let stats = store.stats();

    assert_eq!(stats.plugin_count, 2);
    assert_eq!(stats.item_count, 4);

    assert_eq!(stats.items_per_plugin[0].0, "apps");
    assert_eq!(stats.items_per_plugin[0].1, 3);
    assert_eq!(stats.items_per_plugin[1].0, "notes");
    assert_eq!(stats.items_per_plugin[1].1, 1);
}

#[test]
fn test_all_items_iterator() {
    let mut store = IndexStore::new();

    store.update_full(
        "apps",
        vec![
            make_index_item("app1", "App 1"),
            make_index_item("app2", "App 2"),
        ],
    );

    store.update_full("notes", vec![make_index_item("note1", "Note 1")]);

    let items: Vec<_> = store.all_items().collect();

    assert_eq!(items.len(), 3);

    let app_items: Vec<_> = items.iter().filter(|(pid, _)| *pid == "apps").collect();
    let note_items: Vec<_> = items.iter().filter(|(pid, _)| *pid == "notes").collect();

    assert_eq!(app_items.len(), 2);
    assert_eq!(note_items.len(), 1);
}

#[test]
fn test_all_items_skips_plugin_entries() {
    let mut store = IndexStore::new();
    store.update_full("notes", vec![make_index_item("note1", "Note")]);

    let context = ExecutionContext::default();
    store.record_execution("notes", "note1", &context, Some(&FrecencyMode::Plugin));

    let items: Vec<_> = store.all_items().collect();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].1.id(), "note1");
}

#[test]
fn test_items_with_frecency_includes_plugin_entries() {
    let mut store = IndexStore::new();

    // Create a plugin with frecency: "plugin" mode
    store.update_full("settings", vec![make_index_item("setting1", "Setting 1")]);

    // Record execution in plugin mode - creates __plugin__ entry
    let context = ExecutionContext::default();
    store.record_execution(
        "settings",
        "setting1",
        &context,
        Some(&FrecencyMode::Plugin),
    );

    // Also create a regular item with frecency
    store.update_full("apps", vec![make_index_item("app1", "App 1")]);
    store.record_execution("apps", "app1", &context, None);

    let items = store.items_with_frecency();

    // Should include both the __plugin__ entry and the regular item
    assert_eq!(
        items.len(),
        2,
        "Should include plugin entry and regular item"
    );

    let plugin_entry = items.iter().find(|(_, item)| item.id() == "__plugin__");
    assert!(
        plugin_entry.is_some(),
        "__plugin__ entries should be included in items_with_frecency"
    );

    let regular_item = items.iter().find(|(_, item)| item.id() == "app1");
    assert!(
        regular_item.is_some(),
        "Regular items should also be included"
    );
}

#[test]
fn test_items_with_frecency_plugin_entry_has_correct_plugin_id() {
    let mut store = IndexStore::new();
    store.update_full("todo", vec![make_index_item("task1", "Task 1")]);

    let context = ExecutionContext::default();
    store.record_execution("todo", "task1", &context, Some(&FrecencyMode::Plugin));

    let items = store.items_with_frecency();

    let (plugin_id, item) = items
        .iter()
        .find(|(_, item)| item.id() == "__plugin__")
        .unwrap();

    assert_eq!(
        *plugin_id, "todo",
        "Plugin entry should have correct plugin_id"
    );
    assert!(item.is_plugin_entry, "Should be marked as plugin entry");
}

#[test]
fn test_items_with_frecency_mixed_modes() {
    let mut store = IndexStore::new();

    // Plugin with frecency: "plugin" mode
    store.update_full("notes", vec![make_index_item("note1", "Note 1")]);
    let context = ExecutionContext::default();
    store.record_execution("notes", "note1", &context, Some(&FrecencyMode::Plugin));
    store.record_execution("notes", "note1", &context, Some(&FrecencyMode::Plugin));

    // Plugin with frecency: "item" mode (default)
    store.update_full(
        "apps",
        vec![
            make_index_item("app1", "App 1"),
            make_index_item("app2", "App 2"),
        ],
    );
    store.record_execution("apps", "app1", &context, None);
    store.record_execution("apps", "app2", &context, None);

    let items = store.items_with_frecency();

    // Should have: __plugin__ (notes), app1, app2
    assert_eq!(items.len(), 3);

    let plugin_entries: Vec<_> = items.iter().filter(|(_, i)| i.is_plugin_entry).collect();
    let regular_items: Vec<_> = items.iter().filter(|(_, i)| !i.is_plugin_entry).collect();

    assert_eq!(plugin_entries.len(), 1, "Should have one plugin entry");
    assert_eq!(regular_items.len(), 2, "Should have two regular items");
}

#[test]
fn test_last_dirty_at_updated_on_record() {
    let mut store = IndexStore::new();

    // Initially should be 0
    assert_eq!(store.last_dirty_at(), 0, "Should be 0 for new store");

    store.update_full("apps", vec![make_index_item("app1", "App")]);
    let after_update = store.last_dirty_at();
    assert!(after_update > 0, "Should be set after update_full");

    // Small delay to ensure timestamp changes
    std::thread::sleep(std::time::Duration::from_millis(10));

    let context = ExecutionContext::default();
    store.record_execution("apps", "app1", &context, None);

    assert!(
        store.last_dirty_at() >= after_update,
        "last_dirty_at should be updated after recording"
    );
    assert!(store.is_dirty(), "Should be dirty after recording");
}

#[test]
fn test_index_store_save_load_preserves_slider_values() {
    use hamr_types::WidgetData;

    let mut store = IndexStore::new();

    // Create slider item using builder method
    let item = make_index_item("volume", "Volume Control").with_slider(
        75.0,
        0.0,
        100.0,
        5.0,
        Some("75%".to_string()),
    );

    store.update_full("sound", vec![item]);

    // Save to temp file
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();
    store.save(path).unwrap();

    // Load and verify
    let loaded = IndexStore::load(path).unwrap();
    let loaded_item = loaded.get_item("sound", "volume").unwrap();

    assert!(
        loaded_item.item.is_slider(),
        "Item should be detected as slider via is_slider()"
    );
    let Some(WidgetData::Slider {
        value,
        min,
        max,
        step,
        display_value,
    }) = &loaded_item.item.widget
    else {
        panic!("Expected Slider widget");
    };
    assert_eq!(*value, 75.0, "value should be preserved");
    assert_eq!(*min, 0.0, "min should be preserved");
    assert_eq!(*max, 100.0, "max should be preserved");
    assert_eq!(*step, 5.0, "step should be preserved");
    assert_eq!(
        display_value,
        &Some("75%".to_string()),
        "display_value should be preserved"
    );
}

#[test]
fn test_deserialize_v1_format_migrates_frecency() {
    use crate::index::IndexedItem;

    // v1 format with flat underscore-prefixed fields
    let v1_json = r#"{
        "id": "firefox",
        "name": "Firefox",
        "_count": 15,
        "_lastUsed": 1737012000000,
        "_recentSearchTerms": ["browser", "web"],
        "_hourSlotCounts": [0,0,0,0,0,0,0,0,0,5,3,2,0,0,0,0,0,0,0,0,0,0,0,0],
        "_dayOfWeekCounts": [1,2,3,4,5,0,0],
        "_consecutiveDays": 3,
        "_launchFromEmptyCount": 5,
        "_workspaceCounts": {"dev": 10, "default": 5}
    }"#;

    let item: IndexedItem = serde_json::from_str(v1_json).expect("Should parse v1 format");

    // Frecency data should be migrated to nested struct
    assert_eq!(item.frecency.count, 15);
    assert_eq!(item.frecency.last_used, 1_737_012_000_000);
    assert_eq!(item.frecency.recent_search_terms, vec!["browser", "web"]);
    assert_eq!(item.frecency.hour_slot_counts[9], 5);
    assert_eq!(item.frecency.hour_slot_counts[10], 3);
    assert_eq!(item.frecency.day_of_week_counts[0], 1);
    assert_eq!(item.frecency.consecutive_days, 3);
    assert_eq!(item.frecency.launch_from_empty_count, 5);
    assert_eq!(item.frecency.workspace_counts.get("dev"), Some(&10));
}

#[test]
fn test_deserialize_v2_format_direct() {
    use crate::index::IndexedItem;

    // v2 format with nested frecency object
    let v2_json = r#"{
        "id": "firefox",
        "name": "Firefox",
        "frecency": {
            "count": 15,
            "lastUsed": 1737012000000,
            "recentSearchTerms": ["browser"],
            "consecutiveDays": 5,
            "launchFromEmptyCount": 3
        }
    }"#;

    let item: IndexedItem = serde_json::from_str(v2_json).expect("Should parse v2 format");

    assert_eq!(item.frecency.count, 15);
    assert_eq!(item.frecency.last_used, 1_737_012_000_000);
    assert_eq!(item.frecency.recent_search_terms, vec!["browser"]);
    assert_eq!(item.frecency.consecutive_days, 5);
    assert_eq!(item.frecency.launch_from_empty_count, 3);
}

#[test]
fn test_serialize_always_v2_format() {
    let item = make_indexed_item_with_frecency("test", "Test", 5, hours_ago(1));
    let json = serde_json::to_string(&item).expect("Should serialize");

    // Should have nested "frecency" not flat "_count"
    assert!(json.contains("\"frecency\""), "Should have nested frecency");
    assert!(
        !json.contains("\"_count\""),
        "Should not have flat _count field"
    );
    assert!(
        !json.contains("\"_lastUsed\""),
        "Should not have flat _lastUsed field"
    );
}

#[test]
fn test_v1_is_plugin_entry_migration() {
    use crate::index::IndexedItem;

    // v1 format with _isPluginEntry
    let v1_json = r#"{
        "id": "__plugin__",
        "name": "notes",
        "_count": 10,
        "_isPluginEntry": true
    }"#;

    let item: IndexedItem = serde_json::from_str(v1_json).expect("Should parse");
    assert!(item.is_plugin_entry, "_isPluginEntry should be migrated");
}

#[test]
fn test_v2_is_plugin_entry() {
    use crate::index::IndexedItem;

    // v2 format with isPluginEntry (camelCase)
    let v2_json = r#"{
        "id": "__plugin__",
        "name": "notes",
        "frecency": {"count": 10},
        "isPluginEntry": true
    }"#;

    let item: IndexedItem = serde_json::from_str(v2_json).expect("Should parse");
    assert!(item.is_plugin_entry, "isPluginEntry should be read");
}

#[test]
fn test_index_cache_v1_to_v2_roundtrip() {
    use crate::index::IndexCache;

    // Simulate loading a v1 cache (no version field defaults to v2 during deserialize)
    // but containing v1 item format
    let v1_cache_json = r#"{
        "savedAt": 1737012000000,
        "indexes": {
            "apps": {
                "items": [
                    {
                        "id": "firefox",
                        "name": "Firefox",
                        "_count": 15,
                        "_lastUsed": 1737012000000
                    }
                ],
                "lastIndexed": 1737012000000
            }
        }
    }"#;

    let cache: IndexCache = serde_json::from_str(v1_cache_json).expect("Should parse v1 cache");

    // Items should be migrated
    let item = &cache.indexes["apps"].items[0];
    assert_eq!(item.frecency.count, 15);
    assert_eq!(item.frecency.last_used, 1_737_012_000_000);

    // Re-serialize should be v2 format
    let v2_json = serde_json::to_string(&cache).expect("Should serialize");
    assert!(v2_json.contains("\"frecency\""));
    assert!(!v2_json.contains("\"_count\""));
}
