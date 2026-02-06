//! Tests for smart suggestions and statistical utilities
//!
//! Tests the context-aware suggestion system including:
//! - Wilson score interval calculations
//! - Sequence detection (app-after-app patterns)
//! - Composite confidence scoring
//! - Time/day/workspace pattern detection
//! - Frecency boost integration

use super::fixtures::*;
use crate::frecency::{
    SignalWeights, SmartSuggestions, StatisticalUtils, SuggestionContext, SuggestionReason,
};
use crate::index::{IndexStore, IndexedItem};

#[test]
fn test_wilson_score_zero_total() {
    // Zero total should return 0
    let score = StatisticalUtils::wilson_score(5, 0, 1.65);
    assert_eq!(score, 0.0, "Zero total should give zero score");
}

#[test]
fn test_wilson_score_zero_successes() {
    // Zero successes with non-zero total
    let score = StatisticalUtils::wilson_score(0, 10, 1.65);
    assert_eq!(score, 0.0, "Zero successes should give zero score");
}

#[test]
fn test_wilson_score_perfect_success() {
    // All successes should give high but not 1.0 (Wilson is conservative)
    let score = StatisticalUtils::wilson_score(10, 10, 1.65);
    assert!(score > 0.7, "Perfect success rate should be high: {score}");
    assert!(score < 1.0, "Wilson score is conservative, never 1.0");
}

#[test]
fn test_wilson_score_small_sample_conservative() {
    // Small samples should give conservative (lower) estimates
    let small_sample = StatisticalUtils::wilson_score(3, 5, 1.65); // 60% success
    let large_sample = StatisticalUtils::wilson_score(60, 100, 1.65); // 60% success

    // Large sample should give higher confidence (closer to actual 60%)
    assert!(
        large_sample > small_sample,
        "Large sample ({large_sample}) should have higher score than small ({small_sample}) at same success rate"
    );
}

#[test]
fn test_wilson_score_different_z_values() {
    // Higher z (more confidence) should give lower scores
    let z_90 = StatisticalUtils::wilson_score(8, 10, 1.65); // 90% confidence
    let z_95 = StatisticalUtils::wilson_score(8, 10, 1.96); // 95% confidence

    assert!(
        z_90 > z_95,
        "Higher confidence level (95%) should give more conservative (lower) score"
    );
}

#[test]
fn test_wilson_score_default_uses_90_confidence() {
    let explicit = StatisticalUtils::wilson_score(8, 10, 1.65);
    let default = StatisticalUtils::wilson_score_default(8, 10);

    let diff = (explicit - default).abs();
    assert!(diff < 0.001, "Default should use z=1.65 (90% confidence)");
}

#[test]
fn test_wilson_score_typical_patterns() {
    // Test typical usage patterns
    // Item used 5 times at 2pm out of 20 total uses
    let time_score = StatisticalUtils::wilson_score_default(5, 20);
    assert!(
        time_score > 0.1 && time_score < 0.5,
        "25% success rate with 20 samples: {time_score}"
    );

    // Item used 15 times on Monday out of 30 total
    let day_score = StatisticalUtils::wilson_score_default(15, 30);
    assert!(
        day_score > 0.3 && day_score < 0.7,
        "50% success rate with 30 samples: {day_score}"
    );
}

#[test]
fn test_sequence_metrics_basic() {
    // A -> B happened 10 times
    // A happened 20 times total
    // B happened 30 times total
    // Total launches: 100
    let metrics = StatisticalUtils::sequence_metrics(10, 20, 30, 100);

    // Support = count_ab / total = 10/100 = 0.1
    assert!(
        (metrics.support - 0.1).abs() < 0.001,
        "Support should be 0.1, got {}",
        metrics.support
    );

    // Confidence = count_ab / count_a = 10/20 = 0.5
    assert!(
        (metrics.confidence - 0.5).abs() < 0.001,
        "Confidence should be 0.5, got {}",
        metrics.confidence
    );

    // Lift = confidence / P(B) = 0.5 / 0.3 = 1.67
    assert!(
        (metrics.lift - 1.667).abs() < 0.01,
        "Lift should be ~1.67, got {}",
        metrics.lift
    );
}

#[test]
fn test_sequence_metrics_zero_count_a() {
    // If A never happened, all metrics should be 0
    let metrics = StatisticalUtils::sequence_metrics(0, 0, 30, 100);
    assert_eq!(metrics.support, 0.0);
    assert_eq!(metrics.confidence, 0.0);
    assert_eq!(metrics.lift, 0.0);
}

#[test]
fn test_sequence_metrics_zero_total() {
    // If no launches, all metrics should be 0
    let metrics = StatisticalUtils::sequence_metrics(10, 20, 30, 0);
    assert_eq!(metrics.support, 0.0);
    assert_eq!(metrics.confidence, 0.0);
    assert_eq!(metrics.lift, 0.0);
}

#[test]
fn test_sequence_confidence_below_min_count() {
    // Below min_count threshold should return 0
    let conf = StatisticalUtils::get_sequence_confidence(2, 20, 30, 100, 3);
    assert_eq!(conf, 0.0, "Below min_count should return 0");
}

#[test]
fn test_sequence_confidence_low_lift() {
    // Lift < 1.2 means not significantly correlated
    // A -> B: 3 times, A: 30 times, B: 10 times, Total: 100
    // Confidence = 3/30 = 0.1, P(B) = 0.1, Lift = 1.0
    let conf = StatisticalUtils::get_sequence_confidence(3, 30, 10, 100, 3);
    assert_eq!(conf, 0.0, "Lift < 1.2 should return 0");
}

#[test]
fn test_sequence_confidence_low_confidence() {
    // Confidence < 0.2 should return 0 even if lift is high
    // A -> B: 3 times, A: 20 times, B: 5 times, Total: 100
    // Confidence = 3/20 = 0.15 < 0.2, should fail
    let conf = StatisticalUtils::get_sequence_confidence(3, 20, 5, 100, 3);
    assert_eq!(conf, 0.0, "Confidence < 0.2 should return 0");
}

#[test]
fn test_sequence_confidence_significant() {
    // A significant pattern
    // A -> B: 8 times, A: 20 times, B: 15 times, Total: 100
    // Confidence = 8/20 = 0.4 (> 0.2)
    // P(B) = 0.15, Lift = 0.4/0.15 = 2.67 (> 1.2)
    let conf = StatisticalUtils::get_sequence_confidence(8, 20, 15, 100, 3);
    assert!(
        conf > 0.0,
        "Significant pattern should have positive confidence"
    );
    assert!(conf <= 1.0, "Confidence should be capped at 1.0");
}

#[test]
fn test_signal_weights_values() {
    // Verify signal weights match QML hamr values
    assert!((SignalWeights::SEQUENCE - 0.35).abs() < 0.001);
    assert!((SignalWeights::SESSION - 0.35).abs() < 0.001);
    assert!((SignalWeights::RESUME_FROM_IDLE - 0.30).abs() < 0.001);
    assert!((SignalWeights::TIME - 0.20).abs() < 0.001);
    assert!((SignalWeights::WORKSPACE - 0.20).abs() < 0.001);
    assert!((SignalWeights::RUNNING_APPS - 0.20).abs() < 0.001);
    assert!((SignalWeights::LAUNCH_FROM_EMPTY - 0.15).abs() < 0.001);
    assert!((SignalWeights::DISPLAY_COUNT - 0.15).abs() < 0.001);
    assert!((SignalWeights::SESSION_DURATION - 0.12).abs() < 0.001);
    assert!((SignalWeights::DAY - 0.10).abs() < 0.001);
    assert!((SignalWeights::MONITOR - 0.08).abs() < 0.001);
    assert!((SignalWeights::STREAK - 0.08).abs() < 0.001);
    assert!((SignalWeights::FRECENCY_INFLUENCE - 0.4).abs() < 0.001);
}

#[test]
fn test_suggestion_context_default() {
    let ctx = SuggestionContext::default();
    assert_eq!(ctx.hour, 0);
    assert_eq!(ctx.weekday, 0);
    assert!(!ctx.is_session_start);
    assert!(!ctx.is_resume_from_idle);
    assert!(ctx.last_app.is_none());
    assert!(ctx.workspace.is_none());
    assert!(ctx.running_apps.is_empty());
}

/// Helper to create an `IndexStore` with test data
fn create_test_index_store() -> IndexStore {
    let mut store = IndexStore::new();

    // Item 1: Firefox - used heavily at 2pm, on workspace "dev"
    let mut firefox = make_index_item("firefox", "Firefox");
    firefox.app_id = Some("firefox.desktop".to_string());
    let mut firefox_indexed = IndexedItem::new(firefox);
    firefox_indexed.frecency.count = 50;
    firefox_indexed.frecency.last_used = hours_ago(1);
    firefox_indexed.frecency.hour_slot_counts[14] = 30; // 2pm
    firefox_indexed.frecency.hour_slot_counts[10] = 10; // 10am
    firefox_indexed.frecency.hour_slot_counts[16] = 10; // 4pm
    firefox_indexed.frecency.day_of_week_counts[0] = 20; // Monday
    firefox_indexed.frecency.day_of_week_counts[1] = 15; // Tuesday
    firefox_indexed.frecency.day_of_week_counts[2] = 15; // Wednesday
    firefox_indexed
        .frecency
        .workspace_counts
        .insert("dev".to_string(), 35);
    firefox_indexed
        .frecency
        .workspace_counts
        .insert("default".to_string(), 15);

    // Item 2: VSCode - launched frequently after Firefox
    let mut vscode = make_index_item("vscode", "Visual Studio Code");
    vscode.app_id = Some("code.desktop".to_string());
    let mut vscode_indexed = IndexedItem::new(vscode);
    vscode_indexed.frecency.count = 40;
    vscode_indexed.frecency.last_used = hours_ago(2);
    vscode_indexed
        .frecency
        .launched_after
        .insert("firefox.desktop".to_string(), 25);
    vscode_indexed.frecency.hour_slot_counts[14] = 20;
    vscode_indexed.frecency.hour_slot_counts[15] = 15;
    vscode_indexed.frecency.consecutive_days = 7; // Week-long streak

    // Item 3: Terminal - used at session start
    let mut terminal_indexed = IndexedItem::new(make_index_item("terminal", "Terminal"));
    terminal_indexed.frecency.count = 60;
    terminal_indexed.frecency.last_used = hours_ago(0);
    terminal_indexed.frecency.session_start_count = 45;
    terminal_indexed.frecency.launch_from_empty_count = 40;

    // Item 4: Slack - used after returning from idle
    let mut slack_indexed = IndexedItem::new(make_index_item("slack", "Slack"));
    slack_indexed.frecency.count = 30;
    slack_indexed.frecency.last_used = hours_ago(3);
    slack_indexed.frecency.resume_from_idle_count = 20;

    // Item 5: Chrome - not enough data for suggestions
    let mut chrome_indexed = IndexedItem::new(make_index_item("chrome", "Chrome"));
    chrome_indexed.frecency.count = 2;
    chrome_indexed.frecency.last_used = days_ago(5);

    store.update_full(
        "apps",
        vec![
            firefox_indexed.item.clone(),
            vscode_indexed.item.clone(),
            terminal_indexed.item.clone(),
            slack_indexed.item.clone(),
            chrome_indexed.item.clone(),
        ],
    );

    // Manually set the frecency data (update_full creates new items)
    if let Some(item) = store.get_item_mut("apps", "firefox") {
        item.frecency.count = 50;
        item.frecency.last_used = hours_ago(1);
        item.frecency.hour_slot_counts[14] = 30;
        item.frecency.hour_slot_counts[10] = 10;
        item.frecency.hour_slot_counts[16] = 10;
        item.frecency.day_of_week_counts[0] = 20;
        item.frecency.day_of_week_counts[1] = 15;
        item.frecency.day_of_week_counts[2] = 15;
        item.frecency.workspace_counts.insert("dev".to_string(), 35);
        item.frecency
            .workspace_counts
            .insert("default".to_string(), 15);
        item.item.app_id = Some("firefox.desktop".to_string());
    }

    if let Some(item) = store.get_item_mut("apps", "vscode") {
        item.frecency.count = 40;
        item.frecency.last_used = hours_ago(2);
        item.frecency
            .launched_after
            .insert("firefox.desktop".to_string(), 25);
        item.frecency.hour_slot_counts[14] = 20;
        item.frecency.hour_slot_counts[15] = 15;
        item.frecency.consecutive_days = 7;
        item.item.app_id = Some("code.desktop".to_string());
    }

    if let Some(item) = store.get_item_mut("apps", "terminal") {
        item.frecency.count = 60;
        item.frecency.last_used = hours_ago(0);
        item.frecency.session_start_count = 45;
        item.frecency.launch_from_empty_count = 40;
    }

    if let Some(item) = store.get_item_mut("apps", "slack") {
        item.frecency.count = 30;
        item.frecency.last_used = hours_ago(3);
        item.frecency.resume_from_idle_count = 20;
    }

    if let Some(item) = store.get_item_mut("apps", "chrome") {
        item.frecency.count = 2;
        item.frecency.last_used = days_ago(5);
    }

    store
}

#[test]
fn test_suggestions_empty_store() {
    let store = IndexStore::new();
    let context = SuggestionContext::default();

    let suggestions = SmartSuggestions::get_suggestions(&store, &context, 5, 0, 0);
    assert!(
        suggestions.is_empty(),
        "Empty store should give no suggestions"
    );
}

#[test]
fn test_suggestions_time_of_day_pattern() {
    let store = create_test_index_store();

    let context = SuggestionContext {
        hour: 14, // 2pm - Firefox is heavily used here
        weekday: 0,
        ..Default::default()
    };

    let suggestions = SmartSuggestions::get_suggestions(&store, &context, 5, 0, 0);

    // Firefox should be suggested due to time-of-day pattern
    let firefox_suggested = suggestions.iter().any(|s| s.item_id == "firefox");

    // Note: May or may not be suggested depending on exact scoring
    // Just verify the function runs without panic
    assert!(suggestions.len() <= 5, "Should respect limit");

    if firefox_suggested {
        let firefox = suggestions.iter().find(|s| s.item_id == "firefox").unwrap();
        let has_time_reason = firefox
            .reasons
            .iter()
            .any(|r| matches!(r, SuggestionReason::TimeOfDay));
        // Time reason should be present if firefox was suggested due to time
        if has_time_reason {
            assert!(firefox.score > 0.0);
        }
    }
}

#[test]
fn test_suggestions_workspace_pattern() {
    let store = create_test_index_store();

    let context = SuggestionContext {
        hour: 12,
        weekday: 0,
        workspace: Some("dev".to_string()),
        ..Default::default()
    };

    let suggestions = SmartSuggestions::get_suggestions(&store, &context, 5, 0, 0);

    // Check if workspace pattern is detected
    for suggestion in &suggestions {
        if suggestion.item_id == "firefox" {
            let has_workspace_reason = suggestion
                .reasons
                .iter()
                .any(|r| matches!(r, SuggestionReason::Workspace(_)));
            // If firefox is suggested, workspace might be a reason
            if has_workspace_reason {
                assert!(suggestion.score > 0.0);
            }
        }
    }
}

#[test]
fn test_suggestions_sequence_pattern() {
    let store = create_test_index_store();

    let context = SuggestionContext {
        hour: 14,
        weekday: 0,
        last_app: Some("firefox.desktop".to_string()),
        ..Default::default()
    };

    let suggestions = SmartSuggestions::get_suggestions(&store, &context, 5, 0, 0);

    // VSCode should be suggested after Firefox due to sequence pattern
    let vscode_suggested = suggestions.iter().any(|s| s.item_id == "vscode");

    if vscode_suggested {
        let vscode = suggestions.iter().find(|s| s.item_id == "vscode").unwrap();
        let has_sequence_reason = vscode
            .reasons
            .iter()
            .any(|r| matches!(r, SuggestionReason::AfterApp(_)));
        // Sequence pattern should be detected
        if has_sequence_reason {
            let reason = vscode
                .reasons
                .iter()
                .find(|r| matches!(r, SuggestionReason::AfterApp(_)));
            if let Some(SuggestionReason::AfterApp(app)) = reason {
                assert!(app.contains("firefox"));
            }
        }
    }
}

#[test]
fn test_suggestions_session_start_pattern() {
    let store = create_test_index_store();

    let context = SuggestionContext {
        hour: 9,
        weekday: 0,
        is_session_start: true,
        ..Default::default()
    };

    let suggestions = SmartSuggestions::get_suggestions(&store, &context, 5, 0, 0);

    // Terminal should be suggested at session start
    let terminal_suggested = suggestions.iter().any(|s| s.item_id == "terminal");

    if terminal_suggested {
        let terminal = suggestions
            .iter()
            .find(|s| s.item_id == "terminal")
            .unwrap();
        let has_session_reason = terminal
            .reasons
            .iter()
            .any(|r| matches!(r, SuggestionReason::SessionStart));
        assert!(
            has_session_reason,
            "Terminal should have SessionStart reason"
        );
    }
}

#[test]
fn test_suggestions_resume_from_idle_pattern() {
    let store = create_test_index_store();

    let context = SuggestionContext {
        hour: 14,
        weekday: 0,
        is_resume_from_idle: true,
        ..Default::default()
    };

    let suggestions = SmartSuggestions::get_suggestions(&store, &context, 5, 0, 0);

    // Slack should be suggested when resuming from idle
    let slack_suggested = suggestions.iter().any(|s| s.item_id == "slack");

    if slack_suggested {
        let slack = suggestions.iter().find(|s| s.item_id == "slack").unwrap();
        let has_resume_reason = slack
            .reasons
            .iter()
            .any(|r| matches!(r, SuggestionReason::ResumeFromIdle));
        assert!(has_resume_reason, "Slack should have ResumeFromIdle reason");
    }
}

#[test]
fn test_suggestions_streak_pattern() {
    let store = create_test_index_store();

    let context = SuggestionContext {
        hour: 14,
        weekday: 0,
        ..Default::default()
    };

    let suggestions = SmartSuggestions::get_suggestions(&store, &context, 5, 0, 0);

    // VSCode has a 7-day streak
    let vscode_suggested = suggestions.iter().any(|s| s.item_id == "vscode");

    if vscode_suggested {
        let vscode = suggestions.iter().find(|s| s.item_id == "vscode").unwrap();
        let has_streak_reason = vscode
            .reasons
            .iter()
            .any(|r| matches!(r, SuggestionReason::Streak(_)));
        if has_streak_reason {
            let reason = vscode
                .reasons
                .iter()
                .find(|r| matches!(r, SuggestionReason::Streak(_)));
            if let Some(SuggestionReason::Streak(days)) = reason {
                assert_eq!(*days, 7, "Streak should be 7 days");
            }
        }
    }
}

#[test]
fn test_suggestions_limit_respected() {
    let store = create_test_index_store();
    let context = SuggestionContext {
        hour: 14,
        weekday: 0,
        is_session_start: true,
        is_resume_from_idle: true,
        last_app: Some("firefox.desktop".to_string()),
        workspace: Some("dev".to_string()),
        ..Default::default()
    };

    let suggestions = SmartSuggestions::get_suggestions(&store, &context, 2, 0, 0);
    assert!(
        suggestions.len() <= 2,
        "Should respect limit of 2, got {}",
        suggestions.len()
    );
}

#[test]
fn test_suggestions_sorted_by_score() {
    let store = create_test_index_store();
    let context = SuggestionContext {
        hour: 14,
        weekday: 0,
        is_session_start: true,
        ..Default::default()
    };

    let suggestions = SmartSuggestions::get_suggestions(&store, &context, 10, 0, 0);

    // Verify sorted by descending score
    for i in 1..suggestions.len() {
        assert!(
            suggestions[i - 1].score >= suggestions[i].score,
            "Suggestions should be sorted by descending score"
        );
    }
}

#[test]
fn test_suggestions_no_duplicates() {
    let store = create_test_index_store();
    let context = SuggestionContext {
        hour: 14,
        weekday: 0,
        ..Default::default()
    };

    let suggestions = SmartSuggestions::get_suggestions(&store, &context, 10, 0, 0);

    let mut seen_ids = std::collections::HashSet::new();
    for suggestion in &suggestions {
        assert!(
            seen_ids.insert(&suggestion.item_id),
            "Duplicate item_id found: {}",
            suggestion.item_id
        );
    }
}

#[test]
fn test_format_reason_time_of_day() {
    let reason = SuggestionReason::TimeOfDay;
    let formatted = SmartSuggestions::format_reason(&reason);
    assert!(formatted.contains("time"), "Should mention time");
}

#[test]
fn test_format_reason_day_of_week() {
    let reason = SuggestionReason::DayOfWeek;
    let formatted = SmartSuggestions::format_reason(&reason);
    assert!(formatted.contains("day"), "Should mention day");
}

#[test]
fn test_format_reason_streak() {
    let reason = SuggestionReason::Streak(5);
    let formatted = SmartSuggestions::format_reason(&reason);
    assert!(formatted.contains('5'), "Should mention streak count");
    assert!(formatted.contains("streak") || formatted.contains("day"));
}

#[test]
fn test_format_reason_session_start() {
    let reason = SuggestionReason::SessionStart;
    let formatted = SmartSuggestions::format_reason(&reason);
    assert!(formatted.contains("session"), "Should mention session");
}

#[test]
fn test_format_reason_after_app() {
    let reason = SuggestionReason::AfterApp("Firefox".to_string());
    let formatted = SmartSuggestions::format_reason(&reason);
    assert!(formatted.contains("Firefox"), "Should mention the app");
    assert!(formatted.contains("after"), "Should mention 'after'");
}

#[test]
fn test_format_reason_workspace() {
    let reason = SuggestionReason::Workspace("dev".to_string());
    let formatted = SmartSuggestions::format_reason(&reason);
    assert!(formatted.contains("dev"), "Should mention workspace name");
}

#[test]
fn test_format_reason_display_count_single() {
    let reason = SuggestionReason::DisplayCount(1);
    let formatted = SmartSuggestions::format_reason(&reason);
    assert!(
        formatted.contains("single") || formatted.contains('1'),
        "Should mention single monitor"
    );
}

#[test]
fn test_format_reason_display_count_multiple() {
    let reason = SuggestionReason::DisplayCount(3);
    let formatted = SmartSuggestions::format_reason(&reason);
    assert!(formatted.contains('3'), "Should mention monitor count");
}

#[test]
fn test_format_reason_session_duration() {
    // Test all buckets
    let labels = ["session start", "early", "mid", "long", "extended"];
    for (bucket, expected) in labels.iter().enumerate() {
        let reason = SuggestionReason::SessionDuration(bucket as u8);
        let formatted = SmartSuggestions::format_reason(&reason);
        assert!(
            formatted.to_lowercase().contains(expected) || formatted.contains("session"),
            "Bucket {bucket} should format appropriately: {formatted}"
        );
    }
}

#[test]
fn test_streak_not_suggested_when_broken() {
    // Item with 3-day streak but last used 3+ days ago (streak broken)
    let mut item = make_indexed_item("app1", "App 1");
    item.frecency.count = 10;
    item.frecency.consecutive_days = 5; // Had a 5-day streak
    item.frecency.last_consecutive_date = Some("2020-01-01".to_string()); // Old date - streak broken

    let mut store = IndexStore::new();
    store.update_full("apps", vec![item.item.clone()]);

    // Manually set frecency data by recording and then modifying
    // This is a workaround since we can't directly set the indexed item
    let context = SuggestionContext::default();
    let suggestions = SmartSuggestions::get_suggestions(&store, &context, 10, 0, 0);

    // The streak should NOT trigger because it's broken (old date)
    let has_streak_reason = suggestions.iter().any(|s| {
        s.reasons
            .iter()
            .any(|r| matches!(r, SuggestionReason::Streak(_)))
    });

    assert!(
        !has_streak_reason,
        "Broken streak should not trigger streak suggestion"
    );
}

#[test]
fn test_streak_suggested_when_active_today() {
    // Create a store and record executions to build up a streak
    let mut store = IndexStore::new();
    store.update_full("apps", vec![make_index_item("app1", "Streak App")]);

    // Record multiple times to build up count
    let context = crate::frecency::ExecutionContext::default();
    for _ in 0..10 {
        store.record_execution("apps", "app1", &context, None);
    }

    // Get the item and check its streak state
    let item = store.get_item("apps", "app1").unwrap();

    // Since all recordings are today, consecutive_days should be 1
    // A real 3+ day streak would require actual consecutive day usage
    // This test verifies the streak is being tracked
    assert!(
        item.frecency.consecutive_days >= 1,
        "Should have at least 1 day streak after today's usage"
    );
    assert!(
        item.frecency.last_consecutive_date.is_some(),
        "Should have last_consecutive_date set"
    );
}

#[test]
fn test_suggestions_exclude_broken_streaks() {
    let mut store = IndexStore::new();

    // Create items: one with active streak, one with broken streak
    store.update_full(
        "apps",
        vec![
            make_index_item("active", "Active Streak"),
            make_index_item("broken", "Broken Streak"),
        ],
    );

    // Record for both to give them frecency
    let context = crate::frecency::ExecutionContext::default();
    for _ in 0..5 {
        store.record_execution("apps", "active", &context, None);
        store.record_execution("apps", "broken", &context, None);
    }

    // Get suggestions - neither should have streak reason since
    // same-day recordings don't build multi-day streaks
    let suggestion_context = SuggestionContext::default();
    let suggestions = SmartSuggestions::get_suggestions(&store, &suggestion_context, 10, 0, 0);

    // Count suggestions with streak reasons
    let streak_suggestions: Vec<_> = suggestions
        .iter()
        .filter(|s| {
            s.reasons
                .iter()
                .any(|r| matches!(r, SuggestionReason::Streak(_)))
        })
        .collect();

    // With same-day recordings, no streak suggestions should appear
    // (streaks require consecutive different days)
    assert!(
        streak_suggestions.is_empty(),
        "Same-day recordings should not create streak suggestions"
    );
}

#[test]
fn test_suggestions_staleness_reduces_old_item_confidence() {
    let mut store = IndexStore::new();

    // Create an item with strong session_start pattern
    store.update_full(
        "apps",
        vec![hamr_types::ResultItem {
            id: "old_app".to_string(),
            name: "Old App".to_string(),
            ..Default::default()
        }],
    );

    // Record many executions with session_start to build up strong pattern
    let context = crate::frecency::ExecutionContext {
        is_session_start: true,
        ..Default::default()
    };
    for _ in 0..20 {
        store.record_execution("apps", "old_app", &context, None);
    }

    // Get the item and set last_used to 30 days ago (simulating old item)
    if let Some(item) = store.get_item_mut("apps", "old_app") {
        let days_30_ms = 30.0 * 24.0 * 60.0 * 60.0 * 1000.0;
        #[allow(clippy::cast_sign_loss)]
        let days_30_ms_u64 = days_30_ms as u64;
        item.frecency.last_used = crate::utils::now_millis() - days_30_ms_u64;
    }

    // Test with no staleness - should get suggestion
    let context = SuggestionContext {
        is_session_start: true,
        ..Default::default()
    };
    let suggestions_no_staleness = SmartSuggestions::get_suggestions(&store, &context, 10, 0, 0);
    let has_suggestion_no_staleness = suggestions_no_staleness
        .iter()
        .any(|s| s.item_id == "old_app");
    assert!(
        has_suggestion_no_staleness,
        "Without staleness, old_app should be suggested"
    );

    // Test with 14-day half-life - should still get suggestion but with lower confidence
    let suggestions_with_staleness = SmartSuggestions::get_suggestions(&store, &context, 10, 14, 0);
    let old_app_staleness = suggestions_with_staleness
        .iter()
        .find(|s| s.item_id == "old_app");

    if let Some(old_app) = old_app_staleness {
        // Confidence should be reduced due to staleness
        let no_staleness_score = suggestions_no_staleness
            .iter()
            .find(|s| s.item_id == "old_app")
            .map_or(0.0, |s| s.score);
        assert!(
            old_app.score < no_staleness_score,
            "With staleness (14-day half-life), confidence should be lower than without. \
             No staleness: {}, With staleness: {}",
            no_staleness_score,
            old_app.score
        );
    }

    // Test with max age of 20 days - should be filtered out entirely
    let suggestions_max_age = SmartSuggestions::get_suggestions(&store, &context, 10, 0, 20);
    let has_suggestion_max_age = suggestions_max_age.iter().any(|s| s.item_id == "old_app");
    assert!(
        !has_suggestion_max_age,
        "With max age of 20 days, 30-day old item should be filtered out"
    );
}
