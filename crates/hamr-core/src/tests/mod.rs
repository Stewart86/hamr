//! Test module for hamr-core
//!
//! This module contains comprehensive tests for:
//! - Frecency scoring and decay
//! - Smart suggestions (Wilson score, sequence detection, composite confidence)
//! - Search ranking (fuzzy + frecency + diversity)
//! - Plugin protocol (index, results, updates, browsers)
//! - Plugin manifest parsing and matching
//! - Index persistence and frecency tracking
//! - Configuration loading and defaults
//! - Entry point replay and action execution

// Test modules use exact float comparisons and test-specific casts
#![allow(clippy::float_cmp, clippy::cast_possible_truncation)]

mod config_reload_tests;
mod config_tests;
mod fixtures;
mod frecency_tests;
mod index_tests;
mod plugin_tests;
mod protocol_tests;
mod search_tests;
mod suggestions_tests;
