# AGENTS.md - AI Agent Guidelines for hamr

This document provides guidance for AI coding agents working on the Hamr launcher core library.

## Project Overview

Hamr is a Rust rewrite of Hamr, a desktop launcher originally built with QML/Quickshell. The goal is to create a platform-agnostic core with native UI implementations (GTK4 for Linux, future macOS/Windows support).

**IMPORTANT**: This is a standalone project. Do NOT look at or modify files in the old QML project (`~/Projects/Personal/Qml/hamr/`). All code, including plugins, lives within this repository.

The project consists of:

- `hamr-types`: Shared type definitions
- `hamr-rpc`: JSON-RPC protocol
- `hamr-core`: Platform-agnostic core library (search, plugins, frecency, index)
- `hamr-daemon`: Socket server wrapping core
- `hamr-tui`: TUI client (testing/reference)
- `hamr-cli`: CLI commands (legacy)
- `plugins/`: Python plugins using the SDK in `plugins/sdk/`

See `ARCHITECTURE.md` for detailed architecture and implementation status.

## Build & Test Commands

```bash
# Build everything
cargo build

# Build specific crate
cargo build -p hamr-core
cargo build -p hamr-cli

# Run all tests (~955 tests across all crates)
cargo test -q              # Quiet mode - only shows summary

# Run tests for specific crates
cargo test -q -p hamr-core    # Core library
cargo test -q -p hamr-rpc     # RPC protocol/transport
cargo test -q -p hamr-daemon  # Daemon session/handlers/error
cargo test -q -p hamr-tui     # TUI widgets/render
cargo test -q -p hamr-types   # Shared types

# Run a single test by name (partial match)
cargo test -q -p hamr-core test_composite_score_exact
cargo test -q -p hamr-core frecency

# Run tests by module
cargo test -q -p hamr-core suggestions   # Smart suggestions tests
cargo test -q -p hamr-core index_tests   # Index persistence tests
cargo test -q -p hamr-core plugin_tests  # Plugin manifest tests
cargo test -q -p hamr-core config_tests  # Config loading tests

# Run tests with verbose output (when debugging failures)
cargo test -p hamr-core -- --nocapture

# Check without building (compile errors only)
cargo check

# Format code
cargo fmt

# Lint with clippy (includes cargo check + additional lints)
cargo clippy
cargo clippy -- -W clippy::all

# Note: `cargo check` only verifies compilation, while `cargo clippy`
# also catches common mistakes, non-idiomatic code, and performance issues
```

## CLI Testing Commands

```bash
# Test index search
cargo run -p hamr-cli -- query "shutdown"

# Test daemon plugin (e.g., shell)
cargo run -p hamr-cli -- test shell "ls"

# Test non-daemon plugin
cargo run -p hamr-cli -- test calculate "12+12"

# Show index statistics
cargo run -p hamr-cli -- index

# Debug mode (verbose logging)
cargo run -p hamr-cli -- -d test shell "ls"

# Interactive search mode
cargo run -p hamr-cli -- search
```

## Daemon Commands

```bash
# Start the daemon
cargo run -p hamr-daemon

# Start with custom socket path
cargo run -p hamr-daemon -- --socket-path /tmp/my-hamr.sock

# Debug mode (verbose logging)
RUST_LOG=debug cargo run -p hamr-daemon

# Run tests
cargo test -p hamr-daemon
```

## TUI Commands

```bash
# Start the TUI (requires daemon running)
cargo run -p hamr-tui

# Debug mode (logs to file since TUI uses terminal)
RUST_LOG=debug cargo run -p hamr-tui

# Run tests
cargo test -p hamr-tui
```

## Debugging

### Log Files

In debug builds, both daemon and TUI automatically log to timestamped files in `/tmp/`:

| Component | Log File Pattern                       | Symlink                |
| --------- | -------------------------------------- | ---------------------- |
| Daemon    | `/tmp/hamr-daemon-YYYYMMDD_HHMMSS.log` | `/tmp/hamr-daemon.log` |
| TUI       | `/tmp/hamr-tui-YYYYMMDD_HHMMSS.log`    | `/tmp/hamr-tui.log`    |

The symlinks always point to the most recent log file for easy access.

### Reading Logs

```bash
# Follow daemon logs in real-time
tail -f /tmp/hamr-daemon.log

# Follow TUI logs in real-time (in a separate terminal)
tail -f /tmp/hamr-tui.log

# View recent entries from both
tail -n 100 /tmp/hamr-daemon.log /tmp/hamr-tui.log

# Search for specific patterns
grep -i "error\|warn" /tmp/hamr-daemon.log
grep "handle_action" /tmp/hamr-daemon.log

# View timestamped log files (sorted by date)
ls -lt /tmp/hamr-daemon-*.log | head -5
ls -lt /tmp/hamr-tui-*.log | head -5
```

### Debug Mode

Debug builds automatically:

- Log at `debug` level (verbose)
- Write logs to timestamped files
- Include file/line numbers in log output

Release builds:

- Log at `info` level
- Write to stderr only

Override log level with `RUST_LOG`:

```bash
RUST_LOG=trace cargo run -p hamr-daemon  # Most verbose
RUST_LOG=hamr=debug cargo run -p hamr-daemon  # Debug for hamr crates only
RUST_LOG=warn cargo run -p hamr-daemon  # Warnings and errors only
```

### Common Debugging Scenarios

**Plugin not responding:**

```bash
# Check if plugin is connected
grep "plugin" /tmp/hamr-daemon.log | tail -20

# Look for action forwarding
grep "handle_item_selected\|Forwarding action" /tmp/hamr-daemon.log
```

**Deserialization errors:**

```bash
# TUI logs show JSON parsing failures
grep -i "deserialization\|error" /tmp/hamr-tui.log
```

**Form/settings issues:**

```bash
# Trace form field changes
grep -i "form\|slider\|switch" /tmp/hamr-daemon.log
```

## Code Style Guidelines

### Rust Edition & Toolchain

- **Edition**: 2024
- **Resolver**: 2
- Uses workspace-level dependency management

### Import Organization

Organize imports in this order, separated by blank lines:

1. `crate::` imports (internal modules)
2. `std::` imports
3. External crate imports
4. `use tracing::{...}` for logging

```rust
use crate::config::{Config, Directories};
use crate::frecency::{FrecencyScorer, MatchType};
use crate::plugin::{PluginInput, PluginManager};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
```

### Naming Conventions

- **Types/Structs/Enums**: PascalCase (`HamrCore`, `SearchResult`, `PluginResponse`)
- **Functions/Methods**: snake_case (`handle_query_changed`, `calculate_frecency`)
- **Constants**: SCREAMING_SNAKE_CASE
- **Enum Variants**: PascalCase (`Step::Initial`, `MatchType::Exact`)
- **Module files**: snake_case, use `mod.rs` for multi-file modules

### Error Handling

- Use `thiserror` for library errors in `error.rs`
- Define custom `Error` enum with `#[error("...")]` messages
- Use type alias: `pub type Result<T> = std::result::Result<T, Error>`
- CLI uses `anyhow::Result` for ergonomic error handling
- Use `?` operator for propagation, explicit `match` only when handling

```rust
#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Plugin not found: {0}")]
    PluginNotFound(String),
}
```

### Serde Patterns

- Use `#[serde(rename_all = "camelCase")]` for JSON protocol types
- Use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields
- Use `#[serde(default)]` with custom default functions when needed
- Tagged enums: `#[serde(tag = "type", rename_all = "lowercase")]`

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}
```

### Async Patterns

- Use `tokio` runtime with `#[tokio::main]` and `#[tokio::test]`
- `Arc<Mutex<T>>` for shared mutable state across async contexts
- Use `tokio::time::timeout` for time-bounded operations
- Async functions: `async fn method_name(&mut self) -> Result<T>`

### Struct Defaults

- Derive `Default` when all fields have sensible defaults
- Use `..Default::default()` for partial struct initialization
- Create helper functions like `fn default_verb() -> String`

### Documentation & Comments

- Module-level docs with `//!` for test modules
- Doc comments `///` on public items that provide context beyond the name
- **Minimize inline comments** - code should be self-documenting

**Comment Rules:**

- **DO NOT** add comments that describe "what" the code does (the code is self-documenting)
- **DO NOT** add comments that restate the obvious (e.g., `// Create widget`, `// Return result`)
- **DO NOT** add section header comments (e.g., `// === Section ===`)
- **DO** add comments explaining "why" something is done a certain way
- **DO** add comments for complex algorithms, workarounds, or edge cases
- **DO** keep doc comments on public APIs that provide useful context

```rust
// BAD - obvious from code
// Create the search engine
let engine = SearchEngine::new();
// Set the query
engine.set_query(query);
// Return results
return results;

// GOOD - explains "why"
// Delay 100ms because compositor needs time to finish animation
tokio::time::sleep(Duration::from_millis(100)).await;

// GOOD - documents workaround
// Workaround for GTK4 issue where focus isn't transferred on first click
widget.grab_focus();
```

## Project Structure

```
crates/
  hamr-core/
    src/
      lib.rs           # Public exports, module declarations
      engine.rs        # HamrCore state machine (main orchestration)
      types.rs         # CoreEvent, CoreUpdate, SearchResult types
      error.rs         # Error enum with thiserror
      actions/mod.rs   # Execute actions (command, URL, clipboard)
      config/          # Configuration and directories
      frecency/        # Frecency scoring, smart suggestions
      index/           # Index persistence, frecency tracking
      plugin/          # Plugin management, IPC protocol
      search/          # Nucleo fuzzy search engine
      tests/           # Test module (cfg(test))
        mod.rs
        fixtures.rs    # Test helper factories
        frecency_tests.rs
        search_tests.rs
        protocol_tests.rs
  hamr-cli/
    src/main.rs        # CLI binary entry point
```

## Key Architecture Patterns

### Event-Driven State Machine

- `CoreEvent` flows from UI to core
- `CoreUpdate` flows from core to UI
- Process events via `core.process(event).await`
- Poll for daemon updates via `core.poll_daemons().await`

### Plugin Protocol

Steps: `initial` -> `search` -> `action` -> `form`

Response types: `results`, `execute`, `card`, `form`, `index`, `status`, `update`, `error`

### Testing Patterns

Use fixtures for test data:

```rust
use super::fixtures::*;

#[test]
fn test_frecency_decay() {
    let recent = make_indexed_item_with_frecency("id", "name", 10, hours_ago(1));
    // ...
}
```

Time helpers: `now_millis()`, `hours_ago(n)`, `days_ago(n)`

## When Adding Features

1. **Types**: Add to `types.rs` or relevant module's type file
2. **Protocol changes**: Update `plugin/protocol.rs`
3. **New modules**: Follow pattern of `mod.rs` + specialized files
4. **Tests**: Add to `tests/` with fixtures in `fixtures.rs`
5. **CLI commands**: Add variant to `Commands` enum, match arm in `main()`

## When Modifying Search

- `SearchEngine` uses nucleo matcher
- Composite scoring in `FrecencyScorer::composite_score()`
- Diversity limiting via `FrecencyScorer::apply_diversity()`

## When Modifying Plugins

- `PluginManager` handles discovery
- `PluginProcess` handles IPC
- Manifest parsing in `manifest.rs`
- Protocol types in `protocol.rs`

## GTK4 Gotchas (hamr-gtk)

### Overlay Widget Positioning

When using `gtk4::Overlay` with overlay children that need precise positioning:

- **Problem**: By default, overlay children are NOT included in size measurements
- **Symptom**: Overlay appears misaligned/shifted relative to main child
- **Solution**: Call `overlay.set_measure_overlay(&child, true)` after `add_overlay()`

```rust
let overlay = Overlay::new();
overlay.set_child(Some(&main_widget));
overlay.add_overlay(&overlay_widget);
// IMPORTANT: Include overlay in size calculations for proper alignment
overlay.set_measure_overlay(&overlay_widget, true);
```

### CSS margin vs padding for Overlays

- **margin** on overlay children causes positioning issues (pushes element asymmetrically)
- **padding** is internal and doesn't affect overlay positioning
- Prefer `padding` for spacing inside overlay children

## Commit Workflow

Before committing changes, always run the following to ensure code quality:

```bash
# 1. Run all tests
cargo test -q

# 2. Format code (CI will fail if not formatted)
cargo fmt

# 3. Check formatting was applied
cargo fmt -- --check

# 4. Run clippy for additional linting (optional but recommended)
cargo clippy -- -W clippy::all
```

### Commit Message Format

Use conventional commits with descriptive messages:

```bash
# Features
feat: add staleness decay for smart suggestions

# Bug fixes
fix: correct off-by-one error in suggestion scoring

# Refactoring
refactor: extract decay calculation to StalenessUtils

# Tests
test: add staleness decay unit tests

# Documentation
docs: update AGENTS.md with commit workflow

# Style/formatting
style: run cargo fmt to fix formatting issues
```

For significant changes, include a detailed body explaining the "why":

```bash
git commit -m "feat: add staleness decay for smart suggestions

Implements time-based revalidation for all smart suggestion categories
to prevent stale favorites from persisting indefinitely.

Features:
- Exponential decay with configurable half-life (default 14 days)
- Max age cutoff for very old items (default 60 days)
- Applies universally to all suggestion types

Resolves issue where old 'Quick launch favorite' items remained
suggested even after switching to other items."
```

## Release Checklist

Before tagging a release:

1. **Update workspace packages in `Cargo.lock`**: Run `cargo update -w` after bumping version to ensure workspace member versions are current
2. **Update all dependencies**: Run `cargo update` to update external crate dependencies
3. **Verify build**: Run `cargo build --locked` to confirm AUR/reproducible builds work
4. **Run tests**: `cargo test -q`
5. **Bump version**: Update `version` in root `Cargo.toml` (workspace members inherit it)
6. **Commit**: `git add Cargo.toml Cargo.lock && git commit -m "chore: release vX.Y.Z"`
7. **Tag**: `git tag -a vX.Y.Z -m "vX.Y.Z"`
8. **Push**: `git push && git push --tags`

**Why this matters**: AUR packages build with `--locked` for reproducibility. If `Cargo.lock` is stale (especially after version bump), the build fails with "lock file needs to be updated" errors.

## Dependencies (Key Crates)

| Category       | Crate                 | Purpose                  |
| -------------- | --------------------- | ------------------------ |
| Async          | `tokio`               | Async runtime            |
| Serialization  | `serde`, `serde_json` | JSON protocol            |
| Fuzzy search   | `nucleo`              | High-perf fuzzy matching |
| Error handling | `thiserror`           | Typed errors             |
| Logging        | `tracing`             | Structured logging       |
| CLI            | `clap`                | Argument parsing         |
