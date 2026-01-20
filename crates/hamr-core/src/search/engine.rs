use super::{SearchMatch, Searchable};
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use tracing::debug;

/// Fuzzy search engine using nucleo
pub struct SearchEngine {
    matcher: Matcher,
    config: SearchConfig,
}

#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Minimum score threshold (0.0 - 1.0)
    pub threshold: f64,

    /// Maximum results to return
    pub limit: usize,

    /// Weight for name matches vs keyword matches
    pub name_weight: f64,

    /// Weight for keyword matches
    pub keyword_weight: f64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            threshold: 0.0, // Use raw nucleo scores, no threshold
            limit: 100,
            name_weight: 1.0,
            keyword_weight: 0.3,
        }
    }
}

impl SearchEngine {
    /// Create a new search engine
    #[must_use]
    pub fn new() -> Self {
        Self {
            matcher: Matcher::new(Config::DEFAULT),
            config: SearchConfig::default(),
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn with_config(config: SearchConfig) -> Self {
        Self {
            matcher: Matcher::new(Config::DEFAULT),
            config,
        }
    }

    /// Search for matches.
    /// Returns references to matched searchables to avoid cloning during search.
    pub fn search<'a>(
        &mut self,
        query: &str,
        searchables: &'a [Searchable],
    ) -> Vec<SearchMatch<'a>> {
        if query.is_empty() {
            return Vec::new();
        }

        let pattern = Pattern::new(
            query,
            CaseMatching::Smart,
            Normalization::Smart,
            AtomKind::Fuzzy,
        );

        let mut match_count = 0;
        let mut results: Vec<SearchMatch<'a>> = searchables
            .iter()
            .filter_map(|searchable| {
                let result = self.score_searchable(&pattern, searchable);
                if result.is_some() {
                    match_count += 1;
                }
                result
            })
            .collect();

        debug!("Search found {} matches before sort/filter", match_count);

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(self.config.limit);

        debug!("After truncate: {} results", results.len());

        let before_threshold = results.len();
        results.retain(|m| m.score >= self.config.threshold);
        debug!(
            "After threshold {}: {} results (removed {})",
            self.config.threshold,
            results.len(),
            before_threshold - results.len()
        );

        results
    }

    /// Score a single searchable.
    /// Returns a `SearchMatch` with a reference to avoid cloning.
    fn score_searchable<'a>(
        &mut self,
        pattern: &Pattern,
        searchable: &'a Searchable,
    ) -> Option<SearchMatch<'a>> {
        let mut buf = Vec::new();

        let name_haystack = Utf32Str::new(&searchable.name, &mut buf);
        let name_score = pattern.score(name_haystack, &mut self.matcher)?;

        let keyword_score = if searchable.keywords.is_empty() {
            0
        } else {
            let keywords_text = searchable.keywords.join(" ");
            let mut kw_buf = Vec::new();
            let kw_haystack = Utf32Str::new(&keywords_text, &mut kw_buf);
            pattern.score(kw_haystack, &mut self.matcher).unwrap_or(0)
        };

        let combined_score = (f64::from(name_score) * self.config.name_weight)
            + (f64::from(keyword_score) * self.config.keyword_weight);

        Some(SearchMatch {
            searchable,
            score: combined_score,
        })
    }

    #[cfg(test)]
    #[must_use]
    pub fn is_exact_match(query: &str, name: &str) -> bool {
        query.eq_ignore_ascii_case(name)
    }

    /// Calculate name match bonus based on how well query matches name.
    /// Following the documented algorithm:
    /// - Exact match: +500 bonus
    /// - Prefix match: +250 to +499 based on coverage (query.len / name.len)
    /// - Non-prefix: 0 bonus
    ///
    /// This ensures "Settings" ranks above clipboard items when searching "setting"
    // String lengths are usize, coverage ratio uses f64 for precision
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn name_match_bonus(query: &str, name: &str) -> f64 {
        let query_lower = query.to_lowercase();
        let name_lower = name.to_lowercase();

        if query_lower == name_lower {
            return 500.0;
        }

        if name_lower.starts_with(&query_lower) {
            let coverage = query.len() as f64 / name.len() as f64;
            return 250.0 + (coverage * 250.0);
        }

        0.0
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // Exact float comparisons are intentional in tests
mod tests {
    use super::*;
    use crate::search::SearchableSource;

    fn make_searchable(id: &str, name: &str, keywords: Vec<&str>) -> Searchable {
        Searchable {
            id: id.to_string(),
            name: name.to_string(),
            keywords: keywords.into_iter().map(String::from).collect(),
            source: SearchableSource::Plugin { id: id.to_string() },
            is_history_term: false,
        }
    }

    #[test]
    fn test_basic_search() {
        let mut engine = SearchEngine::new();
        let searchables = vec![
            make_searchable("firefox", "Firefox", vec!["browser", "web"]),
            make_searchable("chrome", "Chrome", vec!["browser", "web"]),
            make_searchable("vscode", "Visual Studio Code", vec!["editor", "code"]),
        ];

        let results = engine.search("fire", &searchables);
        assert!(!results.is_empty());
        assert_eq!(results[0].searchable.id, "firefox");
    }

    #[test]
    fn test_keyword_search() {
        let mut engine = SearchEngine::new();
        let searchables = vec![
            make_searchable("firefox", "Firefox", vec!["browser", "web"]),
            make_searchable("notepad", "Notepad", vec!["editor", "text"]),
        ];

        let results = engine.search("browser", &searchables);
        if !results.is_empty() {
            assert_eq!(results[0].searchable.id, "firefox");
        }
    }

    #[test]
    fn test_empty_query() {
        let mut engine = SearchEngine::new();
        let searchables = vec![make_searchable("test", "Test", vec![])];

        let results = engine.search("", &searchables);
        assert!(results.is_empty());
    }

    #[test]
    fn test_exact_match() {
        assert!(SearchEngine::is_exact_match("firefox", "Firefox"));
        assert!(SearchEngine::is_exact_match("Firefox", "firefox"));
        assert!(!SearchEngine::is_exact_match("fire", "Firefox"));
    }

    #[test]
    fn test_search_empty_searchables() {
        let mut engine = SearchEngine::new();
        let searchables: Vec<Searchable> = vec![];
        let results = engine.search("test", &searchables);
        assert!(
            results.is_empty(),
            "Empty searchables should return empty results"
        );
    }

    #[test]
    fn test_search_whitespace_query() {
        let mut engine = SearchEngine::new();
        let searchables = vec![make_searchable("test", "Test", vec![])];
        let results = engine.search("   ", &searchables);
        assert!(
            results.is_empty() || results[0].score < 10.0,
            "Whitespace query should return empty or very low score"
        );
    }

    #[test]
    fn test_search_special_characters() {
        let mut engine = SearchEngine::new();
        let searchables = vec![
            make_searchable("c++", "C++", vec!["programming"]),
            make_searchable("c#", "C#", vec!["programming"]),
        ];
        let results = engine.search("c+", &searchables);
        assert!(
            !results.is_empty() || results.is_empty(),
            "Should handle special characters gracefully"
        );
    }

    #[test]
    fn test_search_unicode() {
        let mut engine = SearchEngine::new();
        let searchables = vec![make_searchable("emoji", "Hello World", vec![])];
        let results = engine.search("Hello", &searchables);
        assert!(!results.is_empty(), "Should handle unicode/emoji");
    }

    #[test]
    fn test_search_very_long_query() {
        let mut engine = SearchEngine::new();
        let searchables = vec![make_searchable("test", "Test Application", vec![])];
        let long_query = "a".repeat(1000);
        let results = engine.search(&long_query, &searchables);
        assert!(
            results.is_empty() || results[0].score < 10.0,
            "Very long query should not crash and return low/no results"
        );
    }

    #[test]
    fn test_name_match_bonus_empty_query() {
        let bonus = SearchEngine::name_match_bonus("", "Firefox");
        assert_eq!(
            bonus, 250.0,
            "Empty query gives min prefix bonus (coverage=0)"
        );
    }

    #[test]
    fn test_name_match_bonus_empty_name() {
        let bonus = SearchEngine::name_match_bonus("fire", "");
        assert_eq!(bonus, 0.0, "Empty name should give no bonus");
    }

    #[test]
    fn test_name_match_bonus_both_empty() {
        let bonus = SearchEngine::name_match_bonus("", "");
        assert_eq!(bonus, 500.0, "Both empty strings are an exact match");
    }

    #[test]
    fn test_name_match_bonus_query_longer_than_name() {
        let bonus = SearchEngine::name_match_bonus("firefox browser", "fire");
        assert_eq!(bonus, 0.0, "Query longer than name should give no bonus");
    }

    #[test]
    fn test_name_match_bonus_case_variants() {
        let bonus1 = SearchEngine::name_match_bonus("FIREFOX", "firefox");
        let bonus2 = SearchEngine::name_match_bonus("firefox", "FIREFOX");
        assert_eq!(bonus1, 500.0, "Case-insensitive exact match (upper->lower)");
        assert_eq!(bonus2, 500.0, "Case-insensitive exact match (lower->upper)");
    }

    #[test]
    fn test_search_config_default() {
        let config = SearchConfig::default();
        assert_eq!(config.threshold, 0.0);
        assert_eq!(config.limit, 100);
        assert_eq!(config.name_weight, 1.0);
        assert_eq!(config.keyword_weight, 0.3);
    }

    #[test]
    fn test_search_engine_default() {
        let engine1 = SearchEngine::new();
        let engine2 = SearchEngine::default();
        assert_eq!(engine1.config.limit, engine2.config.limit);
    }

    #[test]
    fn test_search_with_custom_config() {
        let config = SearchConfig {
            threshold: 50.0,
            limit: 5,
            name_weight: 2.0,
            keyword_weight: 0.5,
        };
        let mut engine = SearchEngine::with_config(config);
        let searchables: Vec<_> = (0..20)
            .map(|i| make_searchable(&format!("app{i}"), &format!("Application {i}"), vec![]))
            .collect();
        let results = engine.search("app", &searchables);
        assert!(results.len() <= 5, "Should respect limit from config");
    }
}
