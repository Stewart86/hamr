# Hamr Stability & Maintainability Improvements Spec

This document describes changes made (and in progress) to improve hamr's stability
and maintainability. Task tracking is in `TASKS.md`.

---

## Round 1: Core Safety & Deduplication

### Area 1: Missing Config Validation Keys ✓
Added `suggestionStalenessHalfLifeDays` and `maxSuggestionAgeDays` to
`expected_core_config_keys()` to stop false "Unknown config field" warnings.

### Area 2: Replace unreachable!() in Handler Dispatch ✓
Replaced two `_ => unreachable!()` arms in `handlers/mod.rs` with safe
`Err(DaemonError::MethodNotFound(...))` returns to prevent daemon crashes.

### Area 3: SystemTime unwrap → unwrap_or_default ✓
Replaced `.unwrap()` on `SystemTime::now().duration_since(UNIX_EPOCH)` in
`suggestions.rs` with `.unwrap_or_default()`.

### Area 4: Preserve Method Name in RpcError ✓
Changed `MethodNotFound → RpcError` conversion in daemon `error.rs` to include
the method name in the error message instead of discarding it.

### Area 5–6: Remove Dead anyhow Dependencies ✓
Removed unused `anyhow` from both `hamr-core/Cargo.toml` and `hamr-daemon/Cargo.toml`.

### Area 7: Magic String Constants ✓
Defined `pub(crate)` constants (`ID_PLUGIN_ENTRY`, `ID_BACK`, `ID_FORM_CANCEL`,
`PREFIX_PATTERN_MATCH`, `PREFIX_MATCH_PREVIEW`, `ID_DISMISS`) in `engine/mod.rs`
and replaced all scattered string literals across engine, index, and plugin modules.

### Area 8–9: Deduplicate Utils (now_millis, date_string, is_leap_year) ✓
Created `crates/hamr-core/src/utils.rs` with shared `now_millis()`,
`date_string_from_epoch()`, `is_leap_year()`, `today_string()`, `yesterday_string()`.
Removed 3 duplicate `now_millis` and 2 duplicate date-string implementations.

### Area 10: RPC Request Timeout ✓
Wrapped `rx.await` in `RpcClient::request()` with `tokio::time::timeout(30s)`.
Returns `ClientError::Timeout` on expiry.

---

## Round 2: Daemon Hardening & Type Safety

### Poisoned Mutex Handling ✓
Replaced `debounce.lock().unwrap()` in `plugin_watcher.rs` and `config_watcher.rs`
with `let Ok(...) else { error!(...); return; }` to prevent cascade panics.

### Preserve notify::Error Source Chain ✓
Changed `DaemonError::Watcher(String)` to `Watcher(#[from] notify::Error)`,
removing the manual `From` impl that flattened the error to a string.

### Socket Removal Logging ✓
Replaced `let _ = std::fs::remove_file(&path)` at daemon shutdown with
`if let Err(e) = ... { warn!(...) }` to surface stale socket issues.

### Named Duration Constants ✓
Extracted hardcoded durations in `server.rs` (`HEALTH_CHECK_INTERVAL`,
`INDEX_POLL_INTERVAL`, `CONFIG_RELOAD_LOCK_TIMEOUT`) and watcher files
(`DEBOUNCE_DURATION`, `RELOAD_SETTLE_DELAY`) into module-level constants.

### Transport Decoder Safety ✓
Replaced `.expect("set above if None")` in `transport.rs` with
`let Some(length) = ... else { return Ok(None); }`.

### PartialEq/Eq Derives ✓
Added `PartialEq` (and `Eq` where safe) to 18 public structs in `hamr-types`
including Badge, Action, PluginAction, CardData, FormData, FormField, FormOption,
MetadataItem, PreviewData, AmbientItem, FabOverride, PluginManifest, and more.

---

## Round 3: Reduce Duplication & Improve Type Safety

### ensure_daemon_started Helper ✓
Extracted duplicated 8-line daemon-start check in `engine/plugins.rs` into
`ensure_daemon_started()` method, called from `send_plugin_slider_change` and
`send_plugin_switch_toggle`.

### CoreUpdate::results() Constructor ✓
Added `CoreUpdate::results(vec)` in `hamr-types`, replacing 7 verbose all-None
struct literal constructions across engine and plugin modules.

### Remove Identity Action Copies/Clones ✓
Eliminated 4 sites where `Action` was mapped/cloned field-by-field into the same
`Action` type — replaced with direct assignment or `.clone()`.

### FrecencyMode Enum (not strings) ✓
Changed `record_execution`/`record_execution_with_item` to accept
`Option<&FrecencyMode>` instead of `Option<&str>`. Replaced string comparisons
with exhaustive `match` on the enum.

### Configurable RPC Timeout ✓
Added `request_timeout: Duration` field to `RpcClient` with `set_request_timeout()`
setter. Default remains 30 seconds.

### Remove Dead rpc::Error Type ✓
Removed the unused `Error` enum from `hamr-rpc/src/error.rs` (near-duplicate of
`ClientError`, unused by any external crate). Kept `pub type Result<T>` using
`ClientError`.

---

## Round 4: Plugin Safety & Code Cleanup

### SystemTime unwrap in generate_session_id ✓
Replaced `.unwrap()` with `.unwrap_or_default()` in `generate_session_id()`.

### take_receiver() expect → error propagation ✓
Replaced `.expect("Receiver should exist")` in `engine/plugins.rs` with
`.ok_or_else(|| Error::Process(...))? ` for safe error propagation.

### date_string_from_epoch month=0 Edge Case ✓
Fixed edge case where the month loop could leave `month=0` producing invalid
dates. Added `if month == 0 { month = 12; }` guard.

### Safe unwrap on get_item_mut ✓
Replaced `.unwrap()` on `get_item_mut()` in `store.rs` with `let Some(...) else`
pattern that logs and returns instead of panicking.

### Log process.kill() Failures ✓
Replaced `let _ = process.kill().await` in `engine/mod.rs` and `plugins.rs`
with `if let Err(e) = ... { warn!(...) }`.

### Form Data Serialization Logging ✓
Replaced silent `serde_json::to_value().unwrap_or_default()` with match blocks
that log at `warn` level on serialization failure.

### Identity PluginAction Mapping Removed ✓
Removed field-by-field identity copy of `PluginAction` in `convert.rs`.

### Plugin Manifest Clone Eliminated ✓
Computed `is_socket` before struct construction in `Plugin::load()` to avoid
cloning the manifest.

### Dead PluginManager Methods Removed ✓
Removed `snapshot_map()`, `snapshot_plugins()`, `rescan_and_snapshot_map()` —
zero callers confirmed by grep.

### &PathBuf → &Path ✓
Changed `load_plugins_from(&PathBuf)` to `load_plugins_from(&Path)`.

### Serde Deserialization Debug Logging ✓
Added `deserialize_field<T>` helper in `convert.rs` that logs at `debug` level
when plugin update fields fail to deserialize, replacing 5 silent `.ok()` drops.

### Default String Constants ✓
Defined `DEFAULT_PLUGIN_ICON`, `DEFAULT_VERB_OPEN`, `DEFAULT_VERB_SELECT`,
`DEFAULT_ICON_TYPE` constants, replacing 11 scattered string literals across 4 files.

---

## Round 5: Daemon Handler Dedup

### On-Demand Spawn Logic Consolidated ✓
Added `needs_on_demand_spawn()` and `resolve_and_spawn()` methods on
`HandlerContext`. The shared directory resolution + spawn logic was duplicated
in 3 handler sites (`item.rs`, `query.rs`, `mod.rs`). Each caller now uses
the shared methods but keeps its own distinct control flow:
- `item.rs`: Returns bool (spawn attempted)
- `query.rs`: Returns `Ok(())` early after spawn
- `mod.rs`: Sets `spawned_on_demand = true`, falls through on dir-not-found

Added 6 unit tests covering: socket plugin detection, background exclusion,
non-socket exclusion, unknown plugin handling, already-spawned detection,
and connected plugin detection.

---

## Round 6: Stability, Type Safety & Deduplication

### Area 1: Saturating Subtraction in calculate_frecency
`u64 - u64` in `calculate_frecency` (store.rs:256) can underflow if
`effective_last_used() > now` (clock skew, corrupted data). Use
`now.saturating_sub(...)` to prevent panic in debug / wrap in release.

### Area 2: Non-Atomic Index Save
`IndexStore::save` uses `std::fs::write` which truncates then writes. A
crash mid-write corrupts the frecency index. Write to a `.tmp` file then
atomically `rename()`.

### Area 3: Deduplicate Time/Date Logic
`chrono_lite_now()` in store.rs and `build_suggestion_context()` in
suggestions.rs both reimplement weekday/hour calculation from epoch. Extract
shared `time_components_from_epoch()` to `utils.rs`. Also replace
`generate_session_id()` manual epoch calc with `utils::now_millis()`.
Define `SECS_PER_DAY` and `SECS_PER_HOUR` constants in `utils.rs`.

### Area 4: Stringly-Typed Protocol Fields → Enums
Multiple protocol fields are `Option<String>` requiring manual string
matching: `input_mode`, `display_hint`, `field_type`, `source`, `Index.mode`.
Replace with proper enums: `InputMode`, `DisplayHint`, `FormFieldType`,
`ActionSource`, `IndexMode`. Remove associated parse functions.

### Area 5: PluginInput Default + Factory Methods
`PluginInput` has 9 fields (most `None`), constructed verbosely at 16+ sites.
Add `Default` derive and factory methods (`PluginInput::search()`,
`PluginInput::action()`, `PluginInput::initial()`).

### Area 6: Missing PartialEq Derives
`CoreEvent`, `CoreUpdate`, `ResultItem`, `ResultPatch`, `ExecuteAction`,
`PluginStatus`, `IconSpec` all lack `PartialEq`, hindering test assertions.

### Area 7: Daemon Signal Handling
No SIGTERM/SIGINT handler — daemon terminates abruptly without saving
index, cleaning socket, or stopping plugin processes. Use
`tokio::signal::ctrl_c()` in `select!` with accept loop.

### Area 8: Daemon Background Task Tracking
5+ `tokio::spawn` tasks have their `JoinHandle`s discarded. On shutdown,
tasks race against cleanup. Collect handles and `join_all()` after setting
shutdown flag.

### Area 9: RPC Reader Task Improvements
Reader task has no logging on stream termination, and pending requests are
not drained on clean EOF (only on error). Add logging and drain pending on
all exit paths.

### Area 10: Regex Caching in Plugin Matching
`Plugin::matches_query()` compiles regex on every keystroke for every
plugin. Pre-compile patterns at load time and cache them.

### Area 11: Manifest Comparison via PartialEq
`diff_plugins` compares manifests by serializing both to JSON strings via
`.ok()`. Derive `PartialEq` on `Manifest` and sub-types for direct
structural comparison.

### Area 12: Zero-Length RPC Frame Rejection
Transport decoder accepts zero-length frames, producing confusing JSON
parse errors. Reject explicitly with a descriptive error.

### Area 13: Silent Error in Index Serialization
`serde_json::to_value(items).unwrap_or_default()` in process.rs silently
produces null on serialization failure, potentially corrupting index.

### Area 14: Scoring Magic Numbers → Constants
Inline scoring constants (150.0 plugin bonus, 500.0 exact match, 250.0
prefix bonus, frecency recency multipliers 4.0/2.0/1.0/0.5, etc.) should
be named constants for tuning and readability.

### Area 15: Config Value Validation
No range validation for `diversity_decay` (should be 0.0-1.0),
`max_displayed_results` (should be ≥ 1), or consistency between
`suggestion_staleness_half_life_days` and `max_suggestion_age_days`.

### Area 16: Directories::new() Panic → Result
`ProjectDirs::from(...).expect(...)` panics when XDG dirs can't be
determined. Change to return `Result` for graceful handling.

### Area 17: Session ID Uniqueness
`generate_session_id()` uses only `SystemTime::now().as_millis()`. Two
calls within the same millisecond produce identical IDs. Use an atomic
counter for guaranteed uniqueness.

### Area 18: Daemon Status/Ambient Forward Deduplication
"Send status + ambient notification to UI" pattern is repeated 3 times
across `handlers/mod.rs` and `handlers/plugin.rs`. Extract helper.

### Area 19: Duplicate form_data Serialization
Identical `serde_json::to_value(&form_data)` match block in both
`handle_form_submitted` and `handle_form_field_changed`.

### Area 20: ExecutionContext Construction Dedup
`record_plugin_open()` manually constructs `ExecutionContext` duplicating
`build_execution_context()`.

### Area 21: CLI Foreground Runner Dedup
4 `run_*` functions share identical "find_binary → Command::new → status →
check" pattern. Extract `run_foreground()` helper.

### Area 22: Tracing Guard Leak
`std::mem::forget(guard)` in daemon main.rs leaks the tracing NonBlocking
guard, losing final log lines on shutdown.

### Area 23: Watcher Bridge Thread Tracking
`PluginWatcher::spawn()` and config watcher create bridging threads whose
`JoinHandle`s are discarded. Store them for lifecycle management.

### Area 24: Plugin ID "unknown" Fallback
`Plugin::load()` falls back to ID `"unknown"` silently when directory name
is invalid. Multiple such plugins overwrite each other. Return error instead.

### Area 25: RPC Error Context for Event Serialization
`send_event` reuses `ClientError::UnexpectedResponse` for two different
serialization invariant violations. Add descriptive error variant.

### Area 26: Daemon Handler Magic Strings
`"__back__"`, `"__dismiss__"`, `"__plugin__"` used as sentinel action IDs
in handlers without named constants.

### Area 27: Missing Config Validation Keys for Legacy Prefix Fields
`search.prefix.file`, `search.prefix.clipboard`, `search.prefix.shellHistory`
not included in `expected_core_config_keys()`, causing false warnings.

### Area 28: Frecency Boost Lower Bound Clamping
`frecency_boost` capped at 300.0 upper bound but not at 0.0. Use `.clamp()`.

### Area 29: ChecksumsData Resilience
Single malformed plugin entry in checksums.json causes entire file to be
rejected. Use `continue` instead of `?` to skip bad entries.

### Area 30: Index Save HashMap Clone
`IndexStore::save` clones entire `indexes` HashMap for serialization.
Use a reference-borrowing struct to avoid the allocation.

### Area 31: CoreUpdate::results_with_placeholder Constructor
3 locations construct `CoreUpdate::Results` with identical boilerplate
differing only in `results` and `placeholder`. Add helper constructor.
