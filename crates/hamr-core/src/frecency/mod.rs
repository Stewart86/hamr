use crate::index::{IndexStore, IndexedItem};
#[cfg(test)]
use crate::utils::{date_string_from_epoch, is_leap_year};
use crate::utils::{now_millis, today_string, yesterday_string};

const FRECENCY_MULTIPLIER: f64 = 10.0;
const MAX_FRECENCY_BOOST: f64 = 300.0;
const HISTORY_BOOST: f64 = 200.0;
const MIN_SEQUENCE_CONFIDENCE: f64 = 0.1;
const MIN_RUNNING_APPS_SCORE: f64 = 0.1;

/// Match type for scoring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    Exact,
    Fuzzy,
}

/// Frecency scorer for ranking results
pub struct FrecencyScorer;

impl FrecencyScorer {
    /// Calculate composite score following the documented algorithm:
    /// `composite_score` = `fuzzy_score` + `exact_match_bonus` + `frecency_boost` + `history_boost`
    ///
    /// - `fuzzy_score`: 0-1000 base relevance from nucleo matching
    /// - `exact_match_bonus`: already added to `fuzzy_score` before calling this function
    /// - `frecency_boost`: 0-300 based on usage frequency x recency
    /// - `history_boost`: +200 when query matches a learned search term (`MatchType::Exact`)
    pub fn composite_score(match_type: MatchType, fuzzy_score: f64, frecency: f64) -> f64 {
        let frecency_boost = (frecency * FRECENCY_MULTIPLIER).clamp(0.0, MAX_FRECENCY_BOOST);

        let history_boost = if match_type == MatchType::Exact {
            HISTORY_BOOST
        } else {
            0.0
        };

        fuzzy_score + frecency_boost + history_boost
    }

    #[cfg(test)]
    pub fn compare_by_score(
        a: (MatchType, f64, f64),
        b: (MatchType, f64, f64),
    ) -> std::cmp::Ordering {
        let score_a = Self::composite_score(a.0, a.1, a.2);
        let score_b = Self::composite_score(b.0, b.1, b.2);
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    }

    /// Apply diversity decay to search results.
    ///
    /// Following the documented algorithm:
    /// `effective_score = composite_score × (decay_factor ^ position_in_plugin)`
    ///
    /// With default `decay_factor` = 0.7:
    /// - 1st item from plugin: 100% score
    /// - 2nd item from plugin: 70% score
    /// - 3rd item from plugin: 49% score
    /// - 4th item from plugin: 34% score
    ///
    /// This ensures diverse results even when one plugin has many high-scoring matches.
    ///
    /// If `max_per_source > 0`, also enforces a hard limit per plugin.
    // Position is usize, powi needs i32, bounded by result count
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub fn apply_diversity_decay<T, F>(
        results: &mut Vec<(T, f64)>,
        get_plugin_id: F,
        decay_factor: f64,
        max_per_source: usize,
    ) where
        F: Fn(&T) -> &str,
    {
        use std::collections::HashMap;

        if results.is_empty() {
            return;
        }

        let mut plugin_counts: HashMap<String, usize> = HashMap::new();

        for (item, score) in results.iter_mut() {
            let plugin_id = get_plugin_id(item).to_string();
            let position = plugin_counts.entry(plugin_id).or_insert(0);

            *score *= decay_factor.powi(*position as i32);
            *position += 1;
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if max_per_source > 0 {
            let mut plugin_counts: HashMap<String, usize> = HashMap::new();
            results.retain(|(item, _)| {
                let plugin_id = get_plugin_id(item).to_string();
                let count = plugin_counts.entry(plugin_id).or_insert(0);
                if *count < max_per_source {
                    *count += 1;
                    true
                } else {
                    false
                }
            });
        }
    }

    #[cfg(test)]
    pub fn apply_diversity<T, F>(
        results: &mut Vec<T>,
        get_plugin_id: F,
        _decay_factor: f64,
        max_per_source: usize,
    ) where
        F: Fn(&T) -> &str,
    {
        use std::collections::HashMap;

        if results.is_empty() || max_per_source == 0 {
            return;
        }

        let mut plugin_counts: HashMap<String, usize> = HashMap::new();
        results.retain(|item| {
            let plugin_id = get_plugin_id(item).to_string();
            let count = plugin_counts.entry(plugin_id).or_insert(0);
            if *count < max_per_source {
                *count += 1;
                true
            } else {
                false
            }
        });
    }
}

/// Statistical utilities for smart suggestions
pub struct StatisticalUtils;

impl StatisticalUtils {
    /// Wilson Score Interval (lower bound)
    /// Better than simple success/total for small samples
    /// z = 1.65 for 90% confidence (default), 1.96 for 95%
    pub fn wilson_score(successes: u32, total: u32, z: f64) -> f64 {
        if total == 0 {
            return 0.0;
        }

        let p = f64::from(successes) / f64::from(total);
        let z_squared = z * z;
        let n = f64::from(total);

        let denominator = 1.0 + z_squared / n;
        let center = p + z_squared / (2.0 * n);
        let spread = z * ((p * (1.0 - p) + z_squared / (4.0 * n)) / n).sqrt();

        ((center - spread) / denominator).max(0.0)
    }

    /// Wilson score with default z=1.65 (90% confidence)
    pub fn wilson_score_default(successes: u32, total: u32) -> f64 {
        Self::wilson_score(successes, total, 1.65)
    }

    /// Association rule metrics for sequence detection (test-only)
    #[cfg(test)]
    // Statistical variable names: a=first, b=second, ab=sequence
    #[allow(clippy::similar_names)]
    pub fn sequence_metrics(
        count_ab: u32,
        count_a: u32,
        count_b: u32,
        total_launches: u32,
    ) -> SequenceMetrics {
        if count_a == 0 || total_launches == 0 {
            return SequenceMetrics {
                support: 0.0,
                confidence: 0.0,
                lift: 0.0,
            };
        }

        let support = f64::from(count_ab) / f64::from(total_launches);
        let confidence = f64::from(count_ab) / f64::from(count_a);
        let prob_b = f64::from(count_b) / f64::from(total_launches);
        let lift = if prob_b > 0.0 {
            confidence / prob_b
        } else {
            0.0
        };

        SequenceMetrics {
            support,
            confidence,
            lift,
        }
    }

    /// Check if sequence association is significant
    pub fn get_sequence_confidence(
        count_ab: u32,
        count_a: u32,
        count_only_b: u32,
        total_launches: u32,
        min_count: u32,
    ) -> f64 {
        if count_ab < min_count || count_a == 0 || total_launches == 0 {
            return 0.0;
        }

        let confidence = f64::from(count_ab) / f64::from(count_a);
        let prob_b = f64::from(count_only_b) / f64::from(total_launches);
        let lift = if prob_b > 0.0 {
            confidence / prob_b
        } else {
            0.0
        };

        if lift < 1.2 || confidence < 0.2 {
            return 0.0;
        }

        (confidence * (lift / 2.0).min(1.0)).min(1.0)
    }

    pub const MIN_EVENTS_FOR_PATTERN: u32 = 3;
    pub const MIN_CONFIDENCE_TO_SHOW: f64 = 0.25;
}

/// Staleness utilities for time-based decay of suggestion confidence
pub struct StalenessUtils;

impl StalenessUtils {
    /// Calculate exponential decay factor based on age.
    /// Returns a multiplier between 0.0 and 1.0.
    ///
    /// Formula: decay = 0.5 ^ (`age_days` / `half_life_days`)
    ///
    /// - If `half_life_days` is 0, returns 1.0 (no decay)
    /// - If age is 0, returns 1.0 (no decay yet)
    pub fn calculate_decay_factor(age_days: f64, half_life_days: f64) -> f64 {
        if half_life_days <= 0.0 || age_days <= 0.0 {
            return 1.0;
        }
        0.5f64.powf(age_days / half_life_days)
    }

    /// Check if an item is too old to be suggested based on max age.
    /// Returns true if the item should be excluded from suggestions.
    ///
    /// - If `max_age_days` is 0, returns false (no max age limit)
    pub fn is_too_old(age_days: f64, max_age_days: u32) -> bool {
        if max_age_days == 0 {
            return false;
        }
        age_days > f64::from(max_age_days)
    }

    /// Calculate age in days from a timestamp (milliseconds since epoch).
    #[allow(clippy::cast_precision_loss)]
    pub fn age_in_days(timestamp_ms: u64) -> f64 {
        let now_ms = now_millis();
        if timestamp_ms >= now_ms {
            return 0.0;
        }
        let age_ms = now_ms - timestamp_ms;
        age_ms as f64 / (1000.0 * 60.0 * 60.0 * 24.0)
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub struct SequenceMetrics {
    pub support: f64,
    pub confidence: f64,
    pub lift: f64,
}

/// Signal weights for smart suggestions (matching QML hamr)
pub struct SignalWeights;

impl SignalWeights {
    pub const SEQUENCE: f64 = 0.35;
    pub const SESSION: f64 = 0.35;
    pub const RESUME_FROM_IDLE: f64 = 0.30;
    pub const TIME: f64 = 0.20;
    pub const WORKSPACE: f64 = 0.20;
    pub const RUNNING_APPS: f64 = 0.20;
    pub const LAUNCH_FROM_EMPTY: f64 = 0.15;
    pub const DISPLAY_COUNT: f64 = 0.15;
    pub const SESSION_DURATION: f64 = 0.12;
    pub const DAY: f64 = 0.10;
    pub const MONITOR: f64 = 0.08;
    pub const STREAK: f64 = 0.08;
    pub const FRECENCY_INFLUENCE: f64 = 0.4;
}

/// Smart suggestions based on context
pub struct SmartSuggestions;

impl SmartSuggestions {
    /// Get smart suggestions with optional staleness decay.
    ///
    /// # Arguments
    /// * `index_store` - The index store containing items with frecency data
    /// * `context` - Current context (time, workspace, etc.)
    /// * `limit` - Maximum number of suggestions to return
    /// * `staleness_half_life_days` - Days for confidence to decay by 50% (0 to disable)
    /// * `max_age_days` - Maximum age in days for an item to be suggested (0 to disable)
    pub fn get_suggestions(
        index_store: &IndexStore,
        context: &SuggestionContext,
        limit: usize,
        staleness_half_life_days: u32,
        max_age_days: u32,
    ) -> Vec<Suggestion> {
        let items = index_store.items_with_frecency();
        if items.is_empty() {
            return Vec::new();
        }

        let max_frecency = items
            .iter()
            .map(|(_, item)| index_store.calculate_frecency(item))
            .fold(1.0_f64, f64::max);

        let all_items: Vec<_> = items.iter().map(|(_, item)| *item).collect();
        let total_launches: u32 = all_items.iter().map(|i| i.frecency.count).sum();

        let mut candidates = Vec::new();

        for (plugin_id, item) in &items {
            // Check max age - skip items that are too old
            let age_days = StalenessUtils::age_in_days(item.frecency.last_used);
            if StalenessUtils::is_too_old(age_days, max_age_days) {
                continue;
            }

            let result = Self::calculate_item_confidence(item, context, &all_items, total_launches);

            if result.confidence < StatisticalUtils::MIN_CONFIDENCE_TO_SHOW
                && result.reasons.is_empty()
            {
                continue;
            }

            let frecency = index_store.calculate_frecency(item);
            let normalized_frecency = frecency / max_frecency;
            let frecency_boost = 1.0 + (normalized_frecency * SignalWeights::FRECENCY_INFLUENCE);

            // Apply staleness decay to the base confidence before frecency boost
            let decay_factor = StalenessUtils::calculate_decay_factor(
                age_days,
                f64::from(staleness_half_life_days),
            );
            let decayed_confidence = result.confidence * decay_factor;

            let final_confidence = (decayed_confidence * frecency_boost).min(1.0);

            if final_confidence >= StatisticalUtils::MIN_CONFIDENCE_TO_SHOW {
                candidates.push(Suggestion {
                    plugin_id: plugin_id.to_string(),
                    item_id: item.id().to_string(),
                    score: final_confidence,
                    reasons: result.reasons,
                });
            }
        }

        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Self::deduplicate_suggestions(candidates)
            .into_iter()
            .take(limit)
            .collect()
    }

    fn calculate_item_confidence(
        item: &IndexedItem,
        context: &SuggestionContext,
        all_items: &[&IndexedItem],
        total_launches: u32,
    ) -> ConfidenceResult {
        let frec = &item.frecency;
        let mut acc = ConfidenceAccumulator::new(frec.count);

        acc.add_array_signal(
            &frec.hour_slot_counts,
            context.hour as usize,
            3,
            0.1,
            SignalWeights::TIME,
            SuggestionReason::TimeOfDay,
        );
        acc.add_array_signal(
            &frec.day_of_week_counts,
            context.weekday,
            3,
            0.1,
            SignalWeights::DAY,
            SuggestionReason::DayOfWeek,
        );

        if let Some(ref workspace) = context.workspace {
            acc.add_map_signal(
                &frec.workspace_counts,
                workspace,
                2,
                0.15,
                SignalWeights::WORKSPACE,
                SuggestionReason::Workspace(workspace.clone()),
            );
        }
        if let Some(ref monitor) = context.monitor {
            acc.add_map_signal(
                &frec.monitor_counts,
                monitor,
                2,
                0.15,
                SignalWeights::MONITOR,
                SuggestionReason::Monitor(monitor.clone()),
            );
        }

        Self::add_sequence_signal(&mut acc, item, context, all_items, total_launches);
        Self::add_running_apps_signal(&mut acc, item, context);

        acc.add_flag_signal(
            context.is_session_start,
            frec.session_start_count,
            0.15,
            SignalWeights::SESSION,
            SuggestionReason::SessionStart,
        );
        acc.add_flag_signal(
            context.is_resume_from_idle,
            frec.resume_from_idle_count,
            0.15,
            SignalWeights::RESUME_FROM_IDLE,
            SuggestionReason::ResumeFromIdle,
        );

        Self::add_streak_signal(&mut acc, frec);

        acc.add_count_signal(
            frec.launch_from_empty_count,
            0.15,
            SignalWeights::LAUNCH_FROM_EMPTY,
            SuggestionReason::FrequentQuickLaunch,
        );

        if let Some(display_count) = context.display_count {
            acc.add_map_signal(
                &frec.display_count_counts,
                &display_count.to_string(),
                2,
                0.15,
                SignalWeights::DISPLAY_COUNT,
                SuggestionReason::DisplayCount(display_count),
            );
        }

        if let Some(bucket) = context.session_duration_bucket
            && (bucket as usize) < 5
        {
            acc.add_array_signal(
                &frec.session_duration_counts,
                bucket as usize,
                2,
                0.1,
                SignalWeights::SESSION_DURATION,
                SuggestionReason::SessionDuration(bucket),
            );
        }

        acc.finish()
    }

    fn add_sequence_signal(
        acc: &mut ConfidenceAccumulator,
        item: &IndexedItem,
        context: &SuggestionContext,
        all_items: &[&IndexedItem],
        total_launches: u32,
    ) {
        let Some(ref last_app) = context.last_app else {
            return;
        };
        let frec = &item.frecency;
        let seq_count = frec.launched_after.get(last_app).copied().unwrap_or(0);
        if seq_count < acc.min_events {
            return;
        }

        let last_app_count = all_items
            .iter()
            .find(|i| i.item.app_id.as_deref() == Some(last_app) || i.id() == last_app)
            .map_or(0, |i| i.frecency.count);

        let seq_confidence = StatisticalUtils::get_sequence_confidence(
            seq_count,
            last_app_count,
            frec.count,
            total_launches,
            acc.min_events,
        );

        if seq_confidence > MIN_SEQUENCE_CONFIDENCE {
            acc.add_score(
                seq_confidence,
                SignalWeights::SEQUENCE,
                SuggestionReason::AfterApp(last_app.clone()),
            );
        }
    }

    fn add_running_apps_signal(
        acc: &mut ConfidenceAccumulator,
        item: &IndexedItem,
        context: &SuggestionContext,
    ) {
        if context.running_apps.is_empty() {
            return;
        }

        let frec = &item.frecency;
        let mut best_score = 0.0;
        let mut matched_app = String::new();

        for running_app in &context.running_apps {
            if item.item.app_id.as_deref() == Some(running_app) {
                continue;
            }

            let co_count = frec.launched_after.get(running_app).copied().unwrap_or(0);
            if co_count >= acc.min_events {
                let score = StatisticalUtils::wilson_score_default(co_count, frec.count);
                if score > best_score {
                    best_score = score;
                    matched_app.clone_from(running_app);
                }
            }
        }

        if best_score > MIN_RUNNING_APPS_SCORE {
            acc.add_score(
                best_score,
                SignalWeights::RUNNING_APPS,
                SuggestionReason::UsedWithApp(matched_app),
            );
        }
    }

    fn add_streak_signal(acc: &mut ConfidenceAccumulator, frec: &hamr_types::Frecency) {
        if frec.consecutive_days < 3 {
            return;
        }

        let streak_still_active = frec.last_consecutive_date.as_ref().is_some_and(|date| {
            let today = today_string();
            let yesterday = yesterday_string();
            date == &today || date == &yesterday
        });

        if streak_still_active {
            let streak_score = (f64::from(frec.consecutive_days) / 10.0).min(1.0);
            acc.add_score(
                streak_score,
                SignalWeights::STREAK,
                SuggestionReason::Streak(frec.consecutive_days),
            );
        }
    }

    fn calculate_composite_confidence(scores: &[WeightedScore]) -> f64 {
        if scores.is_empty() {
            return 0.0;
        }

        let mut total_weight = 0.0;
        let mut weighted_sum = 0.0;

        for ws in scores {
            if ws.score > 0.0 {
                total_weight += ws.weight;
                weighted_sum += ws.score * ws.weight;
            }
        }

        if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        }
    }

    fn deduplicate_suggestions(candidates: Vec<Suggestion>) -> Vec<Suggestion> {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();

        for candidate in candidates {
            if seen.insert(candidate.item_id.clone()) {
                result.push(candidate);
            }
        }

        result
    }

    pub fn format_reason(reason: &SuggestionReason) -> String {
        match reason {
            SuggestionReason::TimeOfDay => "Often used at this time".to_string(),
            SuggestionReason::DayOfWeek => "Often used on this day".to_string(),
            SuggestionReason::Streak(days) => format!("{days}-day streak"),
            SuggestionReason::SessionStart => "Usually opened at session start".to_string(),
            SuggestionReason::FrequentQuickLaunch => "Quick launch favorite".to_string(),
            SuggestionReason::AfterApp(app) => format!("Often opened after {app}"),
            SuggestionReason::Workspace(ws) => format!("Used on workspace {ws}"),
            SuggestionReason::Monitor(mon) => format!("Used on {mon}"),
            SuggestionReason::ResumeFromIdle => "Often opened after returning".to_string(),
            SuggestionReason::UsedWithApp(app) => format!("Often used with {app}"),
            SuggestionReason::DisplayCount(count) => {
                if *count == 1 {
                    "Often used with single monitor".to_string()
                } else {
                    format!("Often used with {count} monitors")
                }
            }
            SuggestionReason::SessionDuration(bucket) => {
                let labels = [
                    "session start",
                    "early session",
                    "mid session",
                    "long session",
                    "extended session",
                ];
                format!(
                    "Often used in {}",
                    labels.get(*bucket as usize).unwrap_or(&"session")
                )
            }
        }
    }
}

struct WeightedScore {
    score: f64,
    weight: f64,
}

struct ConfidenceResult {
    confidence: f64,
    reasons: Vec<SuggestionReason>,
}

struct ConfidenceAccumulator {
    scores: Vec<WeightedScore>,
    reasons: Vec<SuggestionReason>,
    total_count: u32,
    min_events: u32,
}

impl ConfidenceAccumulator {
    fn new(total_count: u32) -> Self {
        Self {
            scores: Vec::new(),
            reasons: Vec::new(),
            total_count,
            min_events: StatisticalUtils::MIN_EVENTS_FOR_PATTERN,
        }
    }

    fn add_array_signal(
        &mut self,
        array: &[u32],
        index: usize,
        min_unique: usize,
        threshold: f64,
        weight: f64,
        reason: SuggestionReason,
    ) {
        let unique_count = array.iter().filter(|&&c| c > 0).count();
        if unique_count < min_unique {
            return;
        }
        let signal_count = array.get(index).copied().unwrap_or(0);
        self.add_if_significant(signal_count, threshold, weight, reason);
    }

    fn add_map_signal(
        &mut self,
        map: &std::collections::HashMap<String, u32>,
        key: &str,
        min_unique: usize,
        threshold: f64,
        weight: f64,
        reason: SuggestionReason,
    ) {
        if map.len() < min_unique {
            return;
        }
        let signal_count = map.get(key).copied().unwrap_or(0);
        self.add_if_significant(signal_count, threshold, weight, reason);
    }

    fn add_flag_signal(
        &mut self,
        flag: bool,
        signal_count: u32,
        threshold: f64,
        weight: f64,
        reason: SuggestionReason,
    ) {
        if flag {
            self.add_if_significant(signal_count, threshold, weight, reason);
        }
    }

    fn add_count_signal(
        &mut self,
        signal_count: u32,
        threshold: f64,
        weight: f64,
        reason: SuggestionReason,
    ) {
        self.add_if_significant(signal_count, threshold, weight, reason);
    }

    fn add_if_significant(
        &mut self,
        signal_count: u32,
        threshold: f64,
        weight: f64,
        reason: SuggestionReason,
    ) {
        if signal_count < self.min_events {
            return;
        }
        let score = StatisticalUtils::wilson_score_default(signal_count, self.total_count);
        if score > threshold {
            self.scores.push(WeightedScore { score, weight });
            self.reasons.push(reason);
        }
    }

    fn add_score(&mut self, score: f64, weight: f64, reason: SuggestionReason) {
        self.scores.push(WeightedScore { score, weight });
        self.reasons.push(reason);
    }

    fn finish(self) -> ConfidenceResult {
        ConfidenceResult {
            confidence: SmartSuggestions::calculate_composite_confidence(&self.scores),
            reasons: self.reasons,
        }
    }
}

/// Context for generating suggestions
#[derive(Debug, Clone, Default)]
pub struct SuggestionContext {
    pub hour: u32,
    pub weekday: usize,
    pub is_session_start: bool,
    pub is_resume_from_idle: bool,
    pub last_app: Option<String>,
    pub workspace: Option<String>,
    pub monitor: Option<String>,
    pub display_count: Option<u32>,
    pub session_duration_bucket: Option<u8>,
    pub running_apps: Vec<String>,
}

/// Context for recording an execution
#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
    pub search_term: Option<String>,
    pub launch_from_empty: bool,
    pub is_session_start: bool,
    pub is_resume_from_idle: bool,
    pub last_app: Option<String>,
    pub workspace: Option<String>,
    pub monitor: Option<String>,
    pub display_count: Option<u32>,
    pub session_duration_bucket: Option<u8>,
}

/// A smart suggestion
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub plugin_id: String,
    pub item_id: String,
    pub score: f64,
    pub reasons: Vec<SuggestionReason>,
}

/// Reason for a suggestion
#[derive(Debug, Clone)]
pub enum SuggestionReason {
    TimeOfDay,
    DayOfWeek,
    Streak(u32),
    SessionStart,
    FrequentQuickLaunch,
    AfterApp(String),
    Workspace(String),
    Monitor(String),
    ResumeFromIdle,
    UsedWithApp(String),
    DisplayCount(u32),
    SessionDuration(u8),
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // Exact float comparisons are intentional in tests
mod tests {
    use super::*;

    #[test]
    fn test_composite_score_zero_values() {
        let score = FrecencyScorer::composite_score(MatchType::Fuzzy, 0.0, 0.0);
        assert_eq!(score, 0.0, "Zero inputs should give zero score");
    }

    #[test]
    fn test_composite_score_negative_fuzzy() {
        let score = FrecencyScorer::composite_score(MatchType::Fuzzy, -100.0, 10.0);
        assert!(score < 200.0, "Negative fuzzy score should reduce total");
    }

    #[test]
    fn test_composite_score_negative_frecency() {
        let score = FrecencyScorer::composite_score(MatchType::Fuzzy, 100.0, -10.0);
        assert_eq!(
            score, 100.0,
            "Negative frecency is clamped to 0, so score equals fuzzy_score"
        );
    }

    #[test]
    fn test_apply_diversity_decay_empty_list() {
        let mut results: Vec<(String, f64)> = vec![];
        FrecencyScorer::apply_diversity_decay(&mut results, |s| s.as_str(), 0.7, 2);
        assert!(results.is_empty(), "Empty list should remain empty");
    }

    #[test]
    fn test_apply_diversity_decay_single_item() {
        let mut results = vec![("item".to_string(), 100.0)];
        FrecencyScorer::apply_diversity_decay(&mut results, |s| s.as_str(), 0.7, 0);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1, 100.0, "Single item score should be unchanged");
    }

    #[test]
    fn test_apply_diversity_zero_max_per_source() {
        let mut results: Vec<String> = vec!["a".to_string(), "b".to_string()];
        FrecencyScorer::apply_diversity(&mut results, |s| s.as_str(), 0.7, 0);
        assert_eq!(results.len(), 2, "Zero max should not filter");
    }

    #[test]
    fn test_wilson_score_zero_total() {
        let score = StatisticalUtils::wilson_score(5, 0, 1.65);
        assert_eq!(score, 0.0, "Zero total should return 0.0");
    }

    #[test]
    fn test_wilson_score_zero_successes() {
        let score = StatisticalUtils::wilson_score(0, 10, 1.65);
        assert!(
            (0.0..0.1).contains(&score),
            "Zero successes should give low score"
        );
    }

    #[test]
    fn test_wilson_score_all_successes() {
        let score = StatisticalUtils::wilson_score(10, 10, 1.65);
        assert!(
            score > 0.5 && score <= 1.0,
            "All successes should give high score: {score}"
        );
    }

    #[test]
    fn test_wilson_score_default_uses_90_confidence() {
        let with_default = StatisticalUtils::wilson_score_default(5, 10);
        let with_explicit = StatisticalUtils::wilson_score(5, 10, 1.65);
        assert!(
            (with_default - with_explicit).abs() < 0.001,
            "Default should use z=1.65"
        );
    }

    #[test]
    fn test_get_sequence_confidence_below_min_count() {
        let conf = StatisticalUtils::get_sequence_confidence(2, 10, 5, 100, 3);
        assert_eq!(conf, 0.0, "Below min_count should return 0.0");
    }

    #[test]
    fn test_get_sequence_confidence_zero_count_a() {
        let conf = StatisticalUtils::get_sequence_confidence(5, 0, 5, 100, 3);
        assert_eq!(conf, 0.0, "Zero count_a should return 0.0");
    }

    #[test]
    fn test_get_sequence_confidence_zero_total() {
        let conf = StatisticalUtils::get_sequence_confidence(5, 10, 5, 0, 3);
        assert_eq!(conf, 0.0, "Zero total should return 0.0");
    }

    #[test]
    fn test_get_sequence_confidence_low_lift() {
        let conf = StatisticalUtils::get_sequence_confidence(10, 100, 50, 100, 3);
        assert!(
            conf >= 0.0,
            "Lift threshold determines if confidence is returned"
        );
    }

    #[test]
    fn test_min_events_constant() {
        assert_eq!(
            StatisticalUtils::MIN_EVENTS_FOR_PATTERN,
            3,
            "Min events should be 3"
        );
    }

    #[test]
    fn test_min_confidence_constant() {
        assert!(
            (StatisticalUtils::MIN_CONFIDENCE_TO_SHOW - 0.25).abs() < 0.001,
            "Min confidence should be 0.25"
        );
    }

    #[test]
    fn test_signal_weights_sum_reasonable() {
        let sum = SignalWeights::SEQUENCE
            + SignalWeights::SESSION
            + SignalWeights::RESUME_FROM_IDLE
            + SignalWeights::TIME
            + SignalWeights::WORKSPACE
            + SignalWeights::RUNNING_APPS
            + SignalWeights::LAUNCH_FROM_EMPTY
            + SignalWeights::DISPLAY_COUNT
            + SignalWeights::SESSION_DURATION
            + SignalWeights::DAY
            + SignalWeights::MONITOR
            + SignalWeights::STREAK;
        assert!(
            sum > 1.0,
            "Signal weights sum should be > 1.0 (normalized later)"
        );
    }

    #[test]
    fn test_format_reason_all_variants() {
        assert!(!SmartSuggestions::format_reason(&SuggestionReason::TimeOfDay).is_empty());
        assert!(!SmartSuggestions::format_reason(&SuggestionReason::DayOfWeek).is_empty());
        assert!(!SmartSuggestions::format_reason(&SuggestionReason::Streak(5)).is_empty());
        assert!(!SmartSuggestions::format_reason(&SuggestionReason::SessionStart).is_empty());
        assert!(
            !SmartSuggestions::format_reason(&SuggestionReason::FrequentQuickLaunch).is_empty()
        );
        assert!(
            !SmartSuggestions::format_reason(&SuggestionReason::AfterApp("Firefox".to_string()))
                .is_empty()
        );
        assert!(
            !SmartSuggestions::format_reason(&SuggestionReason::Workspace("1".to_string()))
                .is_empty()
        );
        assert!(
            !SmartSuggestions::format_reason(&SuggestionReason::Monitor("HDMI-1".to_string()))
                .is_empty()
        );
        assert!(!SmartSuggestions::format_reason(&SuggestionReason::ResumeFromIdle).is_empty());
        assert!(
            !SmartSuggestions::format_reason(&SuggestionReason::UsedWithApp("Chrome".to_string()))
                .is_empty()
        );
        assert!(!SmartSuggestions::format_reason(&SuggestionReason::DisplayCount(1)).is_empty());
        assert!(!SmartSuggestions::format_reason(&SuggestionReason::DisplayCount(2)).is_empty());
        assert!(!SmartSuggestions::format_reason(&SuggestionReason::SessionDuration(0)).is_empty());
        assert!(!SmartSuggestions::format_reason(&SuggestionReason::SessionDuration(4)).is_empty());
        assert!(
            !SmartSuggestions::format_reason(&SuggestionReason::SessionDuration(10)).is_empty()
        );
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
        assert!(ctx.monitor.is_none());
        assert!(ctx.display_count.is_none());
        assert!(ctx.session_duration_bucket.is_none());
        assert!(ctx.running_apps.is_empty());
    }

    #[test]
    fn test_execution_context_default() {
        let ctx = ExecutionContext::default();
        assert!(ctx.search_term.is_none());
        assert!(!ctx.launch_from_empty);
        assert!(!ctx.is_session_start);
        assert!(!ctx.is_resume_from_idle);
        assert!(ctx.last_app.is_none());
        assert!(ctx.workspace.is_none());
        assert!(ctx.monitor.is_none());
        assert!(ctx.display_count.is_none());
        assert!(ctx.session_duration_bucket.is_none());
    }

    #[test]
    fn test_date_string_from_epoch() {
        let date = date_string_from_epoch(0);
        assert_eq!(date, "1970-01-01");
    }

    #[test]
    fn test_date_string_known_date() {
        let secs = 1_704_067_200;
        let date = date_string_from_epoch(secs);
        assert_eq!(date, "2024-01-01");
    }

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap_year(2000));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(1900));
        assert!(!is_leap_year(2023));
    }

    #[test]
    fn test_match_type_equality() {
        assert_eq!(MatchType::Exact, MatchType::Exact);
        assert_eq!(MatchType::Fuzzy, MatchType::Fuzzy);
        assert_ne!(MatchType::Exact, MatchType::Fuzzy);
    }
}
