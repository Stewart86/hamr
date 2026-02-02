//! Tests for frecency scoring, decay, and suggestions

use super::fixtures::*;
use crate::frecency::{FrecencyScorer, MatchType, StalenessUtils};
use crate::index::IndexStore;
use hamr_types::Frecency;

#[test]
fn test_composite_score_exact_match_beats_fuzzy() {
    // Exact match (history term) should score higher than fuzzy match
    let exact_score = FrecencyScorer::composite_score(MatchType::Exact, 100.0, 10.0);
    let fuzzy_score = FrecencyScorer::composite_score(MatchType::Fuzzy, 100.0, 10.0);

    assert!(
        exact_score > fuzzy_score,
        "Exact match ({exact_score}) should beat fuzzy match ({fuzzy_score})"
    );
}

#[test]
fn test_composite_score_frecency_boost() {
    // Higher frecency should boost score
    let low_frecency = FrecencyScorer::composite_score(MatchType::Fuzzy, 100.0, 0.0);
    let high_frecency = FrecencyScorer::composite_score(MatchType::Fuzzy, 100.0, 50.0);

    assert!(
        high_frecency > low_frecency,
        "Higher frecency ({high_frecency}) should beat lower ({low_frecency})"
    );
}

#[test]
fn test_composite_score_frecency_capped() {
    // Frecency boost should be capped at 300 to prevent domination
    let at_cap = FrecencyScorer::composite_score(MatchType::Fuzzy, 100.0, 30.0);
    let over_cap = FrecencyScorer::composite_score(MatchType::Fuzzy, 100.0, 1000.0);

    // frecency=30 gives 30*10=300 (at cap)
    // frecency=1000 gives 1000*10=10000 but capped at 300
    // Both should have same frecency boost (300)
    assert_eq!(
        at_cap, over_cap,
        "Frecency should be capped at 300: at_cap={at_cap}, over_cap={over_cap}"
    );
}

#[test]
fn test_composite_score_fuzzy_still_matters() {
    // A much better fuzzy score should overcome frecency boost
    // high_fuzzy: 500 + 0 = 500
    // low_fuzzy: 50 + 300 (capped) = 350
    let high_fuzzy_low_freq = FrecencyScorer::composite_score(MatchType::Fuzzy, 500.0, 0.0);
    let low_fuzzy_high_freq = FrecencyScorer::composite_score(MatchType::Fuzzy, 50.0, 100.0);

    assert!(
        high_fuzzy_low_freq > low_fuzzy_high_freq,
        "High fuzzy score ({high_fuzzy_low_freq}) should beat low fuzzy with high freq ({low_fuzzy_high_freq})"
    );
}

#[test]
fn test_diversity_limits_per_source() {
    // Results should be limited per plugin source
    struct TestResult {
        plugin_id: String,
    }

    let mut results: Vec<TestResult> = vec![
        TestResult {
            plugin_id: "apps".into(),
        },
        TestResult {
            plugin_id: "apps".into(),
        },
        TestResult {
            plugin_id: "apps".into(),
        },
        TestResult {
            plugin_id: "apps".into(),
        },
        TestResult {
            plugin_id: "notes".into(),
        },
        TestResult {
            plugin_id: "notes".into(),
        },
    ];

    FrecencyScorer::apply_diversity(
        &mut results,
        |r| r.plugin_id.as_str(),
        0.8,
        2, // max 2 per source
    );

    let apps_count = results.iter().filter(|r| r.plugin_id == "apps").count();
    let notes_count = results.iter().filter(|r| r.plugin_id == "notes").count();

    assert_eq!(apps_count, 2, "Should limit apps to 2");
    assert_eq!(notes_count, 2, "Should limit notes to 2");
}

#[test]
fn test_diversity_preserves_order() {
    struct TestResult {
        id: String,
        plugin_id: String,
    }

    let mut results: Vec<TestResult> = vec![
        TestResult {
            id: "1".into(),
            plugin_id: "apps".into(),
        },
        TestResult {
            id: "2".into(),
            plugin_id: "apps".into(),
        },
        TestResult {
            id: "3".into(),
            plugin_id: "notes".into(),
        },
    ];

    FrecencyScorer::apply_diversity(&mut results, |r| r.plugin_id.as_str(), 0.8, 1);

    // First item from each plugin should be kept, in original order
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, "1");
    assert_eq!(results[1].id, "3");
}

#[test]
fn test_diversity_decay_applies_exponential_decay() {
    // Test that diversity decay applies exponential decay to scores
    // With decay_factor = 0.7:
    // - 1st item from plugin: 100% score
    // - 2nd item from plugin: 70% score
    // - 3rd item from plugin: 49% score

    struct TestItem {
        id: String,
        plugin_id: String,
    }

    let mut results: Vec<(TestItem, f64)> = vec![
        (
            TestItem {
                id: "apps1".into(),
                plugin_id: "apps".into(),
            },
            1000.0,
        ),
        (
            TestItem {
                id: "apps2".into(),
                plugin_id: "apps".into(),
            },
            900.0,
        ),
        (
            TestItem {
                id: "apps3".into(),
                plugin_id: "apps".into(),
            },
            800.0,
        ),
        (
            TestItem {
                id: "notes1".into(),
                plugin_id: "notes".into(),
            },
            700.0,
        ),
    ];

    FrecencyScorer::apply_diversity_decay(
        &mut results,
        |item| item.plugin_id.as_str(),
        0.7,
        0, // no hard limit
    );

    // apps1: 1000 * 0.7^0 = 1000
    // apps2: 900 * 0.7^1 = 630
    // apps3: 800 * 0.7^2 = 392
    // notes1: 700 * 0.7^0 = 700

    // After decay and re-sort, order should be: apps1 (1000), notes1 (700), apps2 (630), apps3 (392)
    assert_eq!(results[0].0.id, "apps1");
    assert!((results[0].1 - 1000.0).abs() < 0.01);

    assert_eq!(results[1].0.id, "notes1");
    assert!((results[1].1 - 700.0).abs() < 0.01);

    assert_eq!(results[2].0.id, "apps2");
    assert!((results[2].1 - 630.0).abs() < 0.01);

    assert_eq!(results[3].0.id, "apps3");
    assert!((results[3].1 - 392.0).abs() < 0.01);
}

#[test]
fn test_diversity_decay_with_max_per_source() {
    struct TestItem {
        plugin_id: String,
    }

    let mut results: Vec<(TestItem, f64)> = vec![
        (
            TestItem {
                plugin_id: "apps".into(),
            },
            1000.0,
        ),
        (
            TestItem {
                plugin_id: "apps".into(),
            },
            900.0,
        ),
        (
            TestItem {
                plugin_id: "apps".into(),
            },
            800.0,
        ),
        (
            TestItem {
                plugin_id: "notes".into(),
            },
            700.0,
        ),
    ];

    FrecencyScorer::apply_diversity_decay(
        &mut results,
        |item| item.plugin_id.as_str(),
        0.7,
        2, // max 2 per source
    );

    // After decay and filtering, should have: apps1, notes1, apps2 (apps3 filtered out)
    assert_eq!(results.len(), 3);
    let apps_count = results
        .iter()
        .filter(|(item, _)| item.plugin_id == "apps")
        .count();
    assert_eq!(apps_count, 2, "Should limit apps to 2");
}

#[test]
fn test_compare_by_score_ordering() {
    use std::cmp::Ordering;

    // Higher score should come first (Less means better)
    let a = (MatchType::Exact, 100.0, 50.0);
    let b = (MatchType::Fuzzy, 100.0, 50.0);

    let cmp = FrecencyScorer::compare_by_score(a, b);
    assert_eq!(cmp, Ordering::Less, "Exact should come before fuzzy");
}

#[test]
fn test_frecency_decay_recent_usage() {
    // Create index store and test frecency calculation
    let store = IndexStore::new();

    // Item used 1 hour ago should have higher frecency than item used 1 week ago
    let recent = make_indexed_item_with_frecency("recent", "Recent App", 10, hours_ago(1));
    let old = make_indexed_item_with_frecency("old", "Old App", 10, days_ago(7));

    let recent_frecency = store.calculate_frecency(&recent);
    let old_frecency = store.calculate_frecency(&old);

    assert!(
        recent_frecency > old_frecency,
        "Recent item ({recent_frecency}) should have higher frecency than old item ({old_frecency})"
    );
}

#[test]
fn test_frecency_decay_tiers() {
    let store = IndexStore::new();

    // Test the four decay tiers: <1h, <24h, <7d, >7d
    let very_recent = make_indexed_item_with_frecency("a", "A", 10, hours_ago(0));
    let today = make_indexed_item_with_frecency("b", "B", 10, hours_ago(12));
    let this_week = make_indexed_item_with_frecency("c", "C", 10, days_ago(3));
    let old = make_indexed_item_with_frecency("d", "D", 10, days_ago(14));

    let f1 = store.calculate_frecency(&very_recent);
    let f2 = store.calculate_frecency(&today);
    let f3 = store.calculate_frecency(&this_week);
    let f4 = store.calculate_frecency(&old);

    assert!(f1 > f2, "<1h ({f1}) should beat <24h ({f2})");
    assert!(f2 > f3, "<24h ({f2}) should beat <7d ({f3})");
    assert!(f3 > f4, "<7d ({f3}) should beat >7d ({f4})");
}

#[test]
fn test_frecency_zero_count() {
    let store = IndexStore::new();

    // Item with zero count should have zero frecency
    let unused = make_indexed_item_with_frecency("unused", "Unused", 0, hours_ago(1));
    let frecency = store.calculate_frecency(&unused);

    assert_eq!(frecency, 0.0, "Zero count should give zero frecency");
}

#[test]
fn test_suggestions_time_of_day() {
    // Test that hour slot counts are correctly set and accessed
    let mut hour_slot_counts = [0; 24];
    hour_slot_counts[14] = 15; // Heavily used at 2pm
    hour_slot_counts[10] = 5; // Also used at 10am

    let frecency = Frecency {
        count: 20,
        last_used: hours_ago(1),
        hour_slot_counts,
        ..Default::default()
    };
    let item = make_indexed_item_with_full_frecency("app1", "Afternoon App", frecency);

    // Verify the hour slot data is correctly stored
    assert_eq!(item.frecency.hour_slot_counts[14], 15);
    assert_eq!(item.frecency.hour_slot_counts[10], 5);
    assert_eq!(item.frecency.hour_slot_counts[0], 0); // Unused slot
}

#[test]
fn test_suggestions_streak_bonus() {
    let frecency = Frecency {
        count: 10,
        last_used: hours_ago(1),
        consecutive_days: 5, // 5-day streak
        ..Default::default()
    };
    let item = make_indexed_item_with_full_frecency("streak", "Daily App", frecency);

    // Verify the item has streak data
    assert_eq!(item.frecency.consecutive_days, 5);
}

#[test]
fn test_staleness_decay_factor_no_decay() {
    // Zero half-life means no decay
    let factor = StalenessUtils::calculate_decay_factor(10.0, 0.0);
    assert_eq!(factor, 1.0, "Zero half-life should mean no decay");

    // Zero age means no decay
    let factor = StalenessUtils::calculate_decay_factor(0.0, 14.0);
    assert_eq!(factor, 1.0, "Zero age should mean no decay");
}

#[test]
fn test_staleness_decay_factor_at_half_life() {
    // At exactly half-life, confidence should be 50%
    let factor = StalenessUtils::calculate_decay_factor(14.0, 14.0);
    assert!(
        (factor - 0.5).abs() < 0.001,
        "At half-life, factor should be 0.5, got {factor}"
    );
}

#[test]
fn test_staleness_decay_factor_at_double_half_life() {
    // At 2x half-life, confidence should be 25% (0.5^2)
    let factor = StalenessUtils::calculate_decay_factor(28.0, 14.0);
    assert!(
        (factor - 0.25).abs() < 0.001,
        "At 2x half-life, factor should be 0.25, got {factor}"
    );
}

#[test]
fn test_staleness_max_age_disabled() {
    // Zero max age means no max age limit
    assert!(
        !StalenessUtils::is_too_old(100.0, 0),
        "Zero max age should not filter any age"
    );
    assert!(
        !StalenessUtils::is_too_old(1000.0, 0),
        "Zero max age should not filter any age"
    );
}

#[test]
fn test_staleness_max_age_cutoff() {
    // 60 day max age
    assert!(
        !StalenessUtils::is_too_old(30.0, 60),
        "30 days should be within 60 day limit"
    );
    assert!(
        !StalenessUtils::is_too_old(60.0, 60),
        "Exactly 60 days should be within limit"
    );
    assert!(
        StalenessUtils::is_too_old(61.0, 60),
        "61 days should exceed 60 day limit"
    );
    assert!(
        StalenessUtils::is_too_old(100.0, 60),
        "100 days should exceed 60 day limit"
    );
}

#[test]
fn test_staleness_age_in_days_recent() {
    // Age of current timestamp should be 0
    let now_ms = crate::frecency::now_millis_frecency();
    let age = StalenessUtils::age_in_days(now_ms);
    assert!(age < 0.001, "Current timestamp should have near-zero age");
}

#[test]
fn test_staleness_age_in_days_old() {
    // 30 days ago in milliseconds
    let days_30_ms = 30.0 * 24.0 * 60.0 * 60.0 * 1000.0;
    let old_timestamp = crate::frecency::now_millis_frecency() - days_30_ms as u64;
    let age = StalenessUtils::age_in_days(old_timestamp);
    assert!(
        (age - 30.0).abs() < 0.1,
        "Age should be approximately 30 days, got {age}"
    );
}
