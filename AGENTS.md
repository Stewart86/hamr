# AGENTS.md - AI Agent Guidelines for hamr

This document provides guidance for AI coding agents working on the Hamr launcher core library.

## Project Overview

The project consists of:

- `hamr-types`: Shared type definitions
- `hamr-rpc`: JSON-RPC protocol
- `hamr-core`: Platform-agnostic core library (search, plugins, frecency, index)
- `hamr-daemon`: Socket server wrapping core
- `hamr-tui`: TUI client (testing/reference)
- `hamr-cli`: CLI commands (legacy)
- `plugins/`: Python plugins using the SDK in `plugins/sdk/`

## Build & Test Commands

```bash
cargo build
cargo test -q              # Quiet mode - only shows summary to safe context
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

## Code Style Guidelines

## Key Architecture Patterns

### Event-Driven State Machine

- `CoreEvent` flows from UI to core
- `CoreUpdate` flows from core to UI
- Process events via `core.process(event).await`
- Poll for daemon updates via `core.poll_daemons().await`

### Plugin Protocol

Steps: `initial` -> `search` -> `action` -> `form`
Response types: `results`, `execute`, `card`, `form`, `index`, `status`, `update`, `error`

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

## Release Checklist

Release workflow policy:

1. **Use a release script**: Preferred flow is a single `scripts/release.sh X.Y.Z` entrypoint that bumps `Cargo.toml`, regenerates AUR metadata, validates the tree, and creates the release commit and tag
2. **Treat the git tag as release truth**: `vX.Y.Z` is the publish trigger for GitHub Releases and AUR publication
3. **Keep `Cargo.toml` aligned with the tag**: `flake.nix` reads `workspace.package.version` from `Cargo.toml`, so Nix and Cargo builds must match the tagged version
4. **Refresh the lockfile**: Run `cargo update -w` after bumping the version, and `cargo update` when intentionally refreshing dependencies
5. **Verify reproducible builds**: Run `cargo build --locked` and `cargo test -q` before tagging
6. **Avoid writing release metadata back to `main` from CI**: release automation should render AUR metadata from templates and the tag during publish, but must not commit generated release-version changes back to the protected branch
7. **Create the release commit and tag**: Commit the versioned source state first, then create annotated tag `vX.Y.Z`
8. **Push commit and tag**: `git push && git push --tags`

**Why this matters**: AUR packages build with `--locked` for reproducibility, Nix builds read the version from `Cargo.toml`, and release publishing should be derived from the tagged source state without CI mutating `main`.
