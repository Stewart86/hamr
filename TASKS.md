# Task Tracking

Agents: update the Status column as you work. Only work on tasks marked `todo`.
Set to `in-progress (agent N)` when you start, `done` when you finish.
Run `cargo check` after each change. Run `cargo test -q` for the affected crate.

## Round 1

| ID | Task | Files touched | Status | Priority |
|----|------|---------------|--------|----------|
| T01 | Add missing config validation keys (`suggestionStalenessHalfLifeDays`, `maxSuggestionAgeDays`) to `expected_core_config_keys()` in validation.rs | `hamr-core/src/config/validation.rs` | done | high |
| T02 | Replace `unreachable!()` with safe error returns in `handlers/mod.rs`. Both match arms (~line 219 and ~252). | `hamr-daemon/src/handlers/mod.rs` | done | high |
| T03 | Replace `.unwrap()` with `.unwrap_or_default()` on `SystemTime` in `suggestions.rs` line 147 | `hamr-core/src/engine/suggestions.rs` | done | medium |
| T04 | Include method name in `MethodNotFound` → `RpcError` conversion in daemon `error.rs` | `hamr-daemon/src/error.rs` | done | medium |
| T05 | Remove unused `anyhow.workspace = true` from hamr-core Cargo.toml | `hamr-core/Cargo.toml` | done | low |
| T06 | Remove unused `anyhow = "1"` from hamr-daemon Cargo.toml | `hamr-daemon/Cargo.toml` | done | low |
| T07 | Define magic string constants in `engine/mod.rs` and replace all usages across engine/mod.rs, engine/plugins.rs, index/store.rs, plugin/convert.rs, engine/suggestions.rs | `hamr-core/src/engine/*.rs`, `index/store.rs` | done | medium |
| T08 | Create `utils.rs` with shared `now_millis()`, declare mod in `lib.rs`, update 3 call sites, remove duplicates | `hamr-core/src/utils.rs` (new), `hamr-core/src/lib.rs`, `engine/mod.rs`, `index/store.rs`, `frecency/mod.rs` | done | medium |
| T09 | Deduplicate date/leap-year logic: move canonical functions to `utils.rs`, update store.rs and frecency/mod.rs to use them | `hamr-core/src/utils.rs`, `index/store.rs`, `frecency/mod.rs` | done | medium |
| T10 | Add `tokio::time::timeout` to `RpcClient::request()` in hamr-rpc client.rs, return `Error::Timeout` on expiry | `hamr-rpc/src/client.rs` | done | high |

## Round 2

| ID | Task | Files touched | Status | Priority |
|----|------|---------------|--------|----------|
| T11 | Fix poisoned Mutex::lock().unwrap() in plugin_watcher.rs and config_watcher.rs — replace with graceful error handling + log | `hamr-daemon/src/plugin_watcher.rs`, `hamr-daemon/src/config_watcher.rs` | done | high |
| T12 | Preserve notify::Error source chain in DaemonError::Watcher — change from `Watcher(String)` to `Watcher(#[from] notify::Error)`, remove manual From impl. Update any tests that match on the string. | `hamr-daemon/src/error.rs` | done | medium |
| T13 | Log warning on failed socket file removal at daemon shutdown (server.rs:519) — replace `let _ = std::fs::remove_file` with `if let Err(e)` + warn! | `hamr-daemon/src/server.rs` | done | medium |
| T14 | Extract hardcoded durations in server.rs to named constants: health check interval (5s), index poll interval (100ms), config reload lock timeout (5s) | `hamr-daemon/src/server.rs` | done | low |
| T15 | Extract hardcoded durations in watcher files to named constants: debounce duration (500ms) in plugin_watcher.rs and config_watcher.rs, reload settle delay (100ms) in config_watcher.rs | `hamr-daemon/src/plugin_watcher.rs`, `hamr-daemon/src/config_watcher.rs` | done | low |
| T16 | Replace `.expect("set above if None")` in transport.rs:59 with safe `let Some(...) else` pattern | `hamr-rpc/src/transport.rs` | done | medium |
| T17 | Add missing `PartialEq` derives to 16 public structs in hamr-types (Badge, Action, PluginAction, CardData, FormData, FormField, FormOption, MetadataItem, PreviewData, AmbientItem, FabOverride, ImageBrowserData, ImageItem, GridBrowserData, GridItem, SliderValue). Add `Eq` where no f64 fields. | `hamr-types/src/lib.rs` | done | low |
| T18 | Add `PartialEq, Eq` to PluginManifest derive in hamr-types | `hamr-types/src/lib.rs` | done | low |

## Round 3

| ID | Task | Files touched | Status | Priority |
|----|------|---------------|--------|----------|
| T19 | Extract `ensure_daemon_started` helper in `engine/plugins.rs` — the identical 8-line daemon-start check should become a method returning bool. | `hamr-core/src/engine/plugins.rs` | done | medium |
| T20 | Add `CoreUpdate::results(vec)` constructor in hamr-types — replaces verbose 7-field struct construction with `None` fields at 8+ call sites. | `hamr-types/src/lib.rs`, `hamr-core/src/engine/mod.rs`, `hamr-core/src/plugin/convert.rs` | done | medium |
| T21 | Remove identity Action mapping in `plugin/convert.rs` — two `.into_iter().map(field-by-field copy).collect()` should just be direct assignment. | `hamr-core/src/plugin/convert.rs` | done | low |
| T22 | Remove identity Action cloning in `engine/mod.rs` and `engine/suggestions.rs` — field-by-field clones should become `.clone()`. | `hamr-core/src/engine/mod.rs`, `hamr-core/src/engine/suggestions.rs` | done | low |
| T23 | Replace stringly-typed frecency mode — pass `FrecencyMode` enum directly instead of converting to `&str` and matching on strings. | `hamr-core/src/engine/mod.rs`, `hamr-core/src/index/store.rs` | done | medium |
| T24 | Make RPC request timeout configurable — add `request_timeout: Duration` field to `RpcClient`, default 30s. | `hamr-rpc/src/client.rs` | done | low |
| T25 | Remove dead `hamr_rpc::Error` type — unused by any external crate. Kept `pub type Result` with `ClientError`. | `hamr-rpc/src/error.rs`, `hamr-rpc/src/lib.rs` | done | low |

## Round 4

| ID | Task | Files touched | Status | Priority |
|----|------|---------------|--------|----------|
| T26 | Replace `.unwrap()` on `SystemTime` in `generate_session_id()` with `.unwrap_or_default()` | `hamr-core/src/engine/mod.rs` | done | high |
| T27 | Replace `.expect()` on `take_receiver()` in `engine/plugins.rs:29` with error propagation via `?` | `hamr-core/src/engine/plugins.rs` | done | high |
| T28 | Fix `date_string_from_epoch()` month=0 edge case — if loop exhausts all months, default to 12 (December) | `hamr-core/src/utils.rs` | done | high |
| T29 | Replace `.unwrap()` on `get_item_mut()` in `store.rs:324` with safe `let Some(...) else` + error log | `hamr-core/src/index/store.rs` | done | medium |
| T30 | Define `DEFAULT_PLUGIN_ICON`, `DEFAULT_VERB_OPEN`, `DEFAULT_VERB_SELECT`, `DEFAULT_ICON_TYPE` constants in `engine/mod.rs` and replace all 8+ occurrences of `"extension"`, `"Open"`, `"Select"`, `"material"` across engine/mod.rs, suggestions.rs, store.rs, convert.rs | `hamr-core/src/engine/mod.rs`, `engine/suggestions.rs`, `index/store.rs`, `plugin/convert.rs` | done | medium |
| T31 | Log warning on failed `process.kill()` — 2 sites in engine/mod.rs and plugins.rs silently drop kill errors | `hamr-core/src/engine/mod.rs`, `hamr-core/src/engine/plugins.rs` | done | medium |
| T32 | Remove identity PluginAction mapping in `plugin/convert.rs:217-231` — same type, field-by-field copy should be direct assignment | `hamr-core/src/plugin/convert.rs` | done | low |
| T33 | Remove unnecessary `manifest.clone()` in `Plugin::load` — compute `is_socket` before moving manifest | `hamr-core/src/plugin/mod.rs` | done | low |
| T34 | Remove dead public methods on PluginManager: `snapshot_map`, `snapshot_plugins`, `rescan_and_snapshot_map` — grep confirms no callers | `hamr-core/src/plugin/mod.rs` | done | low |
| T35 | Change `load_plugins_from(&PathBuf)` to `load_plugins_from(&Path)` in plugin/mod.rs | `hamr-core/src/plugin/mod.rs` | done | low |
| T36 | Add debug logging for silent `.ok()` drops on serde deserialization in `convert_update_items` (5 sites in convert.rs:571-585) | `hamr-core/src/plugin/convert.rs` | done | medium |
| T37 | Log warning on silent `serde_json::to_value().unwrap_or_default()` in engine/mod.rs (2 sites for form data serialization) | `hamr-core/src/engine/mod.rs` | done | low |

## Round 5

| ID | Task | Files touched | Status | Priority |
|----|------|---------------|--------|----------|
| T38 | Add `needs_on_demand_spawn()` and `resolve_and_spawn()` methods on `HandlerContext` — extracts shared directory resolution + spawn logic. Refactor 3 call sites (item.rs, query.rs, mod.rs) to use them. Add 6 unit tests. | `hamr-daemon/src/handlers/mod.rs`, `handlers/item.rs`, `handlers/query.rs` | done | medium |

## Round 6

**File ownership rules:** Each task lists the files it touches. No two `todo` tasks share files. Agents must only modify files listed in their task.

| ID | Task | Files touched | Status | Priority |
|----|------|---------------|--------|----------|
| T39 | Replace `now - item.effective_last_used()` with `now.saturating_sub(item.effective_last_used())` in `calculate_frecency` (store.rs:256). Add test for `last_used > now` edge case. | `hamr-core/src/index/store.rs` | done | high |
| T40 | Make `IndexStore::save` atomic — write to `path.with_extension("tmp")` then `std::fs::rename()` to the target. Serialization ref struct: create `IndexCacheRef<'a>` that borrows `&indexes` instead of `.clone()` to avoid full HashMap clone. | `hamr-core/src/index/store.rs` | done | high |
| T41 | Add Unix signal handling to daemon — use `tokio::signal::ctrl_c()` and `tokio::signal::unix::signal(SignalKind::terminate())` in a `tokio::select!` with the accept loop in `server::run()`. On signal, set `shutdown = true` and break. Collect spawned task `JoinHandle`s into a `JoinSet` and `join_all()` before socket cleanup. | `hamr-daemon/src/server.rs` | done | high |
| T42 | Fix tracing guard leak in daemon — replace `std::mem::forget(guard)` with returning guard from `setup_logging()` and holding it in `main()`. This ensures final log lines are flushed on shutdown. | `hamr-daemon/src/main.rs` | done | medium |
| T43 | Deduplicate time/date logic — extract `pub(crate) fn time_components_from_epoch(epoch_secs: u64) -> (u32 /*hour*/, usize /*weekday*/)` and constants `SECS_PER_DAY`, `SECS_PER_HOUR` in `utils.rs`. Update `build_suggestion_context()` in suggestions.rs to use it. Replace `generate_session_id()` body with `utils::now_millis()`. Replace test fixtures `now_millis()` with re-export of `utils::now_millis`. | `hamr-core/src/utils.rs`, `hamr-core/src/engine/suggestions.rs`, `hamr-core/src/engine/mod.rs` (only `generate_session_id` fn), `hamr-core/src/tests/fixtures.rs` | done | medium |
| T44 | Replace stringly-typed protocol fields with enums — (a) Define `IndexMode` enum (Full, Incremental) in protocol.rs. (b) Change `PluginResponse::Index::mode` from `Option<String>` to `Option<IndexMode>`. (c) Change `PluginResponse::Results::display_hint` from `Option<String>` to `Option<DisplayHint>` (use type from hamr-types). (d) Change `PluginInput::source` from `Option<String>` to `Option<ActionSource>` enum (Normal, Ambient). (e) Change `FormField::field_type` from `Option<String>` to `Option<FormFieldType>` (use type from hamr-types). (f) Remove `parse_display_hint()` and `parse_field_type()` from convert.rs. | `hamr-core/src/plugin/protocol.rs`, `hamr-core/src/plugin/convert.rs`, `hamr-core/src/engine/plugins.rs` | done | medium |
| T45 | Add `Default` to `PluginInput` (with `Step::Initial` as default step) and add factory methods: `PluginInput::search(query)`, `PluginInput::action(item_id)`, `PluginInput::initial()`. Simplify construction at all 16+ call sites using `..Default::default()` or factory methods. | `hamr-core/src/plugin/protocol.rs` (only `PluginInput` struct + impls), `hamr-core/src/engine/process.rs` | done | medium |
| T46 | Add missing `PartialEq` (and `Eq` where safe) derives to `CoreEvent`, `CoreUpdate`, `ResultItem`, `ResultPatch`, `ExecuteAction`, `PluginStatus`, `IconSpec` in hamr-types. Remove unused `use std::convert::TryFrom` import. Add `CoreUpdate::results_with_placeholder(results, placeholder)` constructor. Define slider default constants (`DEFAULT_SLIDER_MIN/MAX/STEP`) and use in `build_widget_from_flat` and `deserialize_progress_data`. | `hamr-types/src/lib.rs` | done | medium |
| T47 | RPC client reader task improvements — (a) Add logging on stream termination (`debug!` for normal EOF, `warn!` for codec errors). (b) Drain pending requests on clean stream EOF (not just on error path). (c) Extract channel capacity to `const NOTIFICATION_CHANNEL_CAPACITY: usize = 64`. (d) Restructure `match &msg` → `match msg` to avoid `resp.clone()`. | `hamr-rpc/src/client.rs` | done | medium |
| T48 | Reject zero-length RPC frames in transport decoder — add `if len == 0` check after `MAX_MESSAGE_SIZE` check, return descriptive `CodecError`. Add `Display` impl for `RequestId`. | `hamr-rpc/src/transport.rs`, `hamr-rpc/src/protocol.rs` | done | medium |
| T49 | Improve RPC error context — (a) Replace `ClientError::UnexpectedResponse` misuse in `send_event` (helpers.rs) with a descriptive error (new `ClientError::SerializationInvariant(String)` variant or inline Rpc error). (b) Avoid `obj.clone()` on happy path in `notification_to_update`. | `hamr-rpc/src/helpers.rs`, `hamr-rpc/src/error.rs` | done | medium |
| T50 | Pre-compile regex patterns in `Plugin::load()` — add `compiled_patterns: Vec<regex::Regex>` field to `Plugin`. Log warning on invalid patterns. Use cached regexes in `matches_query()`. Derive `PartialEq` on `Manifest` and sub-types (`MatchConfig`, `Handler`, `HandlerType`, `DaemonConfig`, `InputMode`). Replace JSON-string manifest comparison in `diff_plugins` with direct `!=`. Return error instead of `"unknown"` fallback for invalid plugin directory names. Define `MANIFEST_FILENAME` and `DEFAULT_HANDLER_FILENAME` constants. | `hamr-core/src/plugin/mod.rs`, `hamr-core/src/plugin/manifest.rs` | done | medium |
| T51 | Config validation and safety — (a) Add range validation for `diversity_decay` (clamp to 0.0-1.0 with warning). (b) Clamp `max_displayed_results` to ≥ 1. (c) Warn if `half_life > max_age`. (d) Add missing legacy prefix keys (`file`, `clipboard`, `shellHistory`) to `expected_core_config_keys()`. (e) Change `Directories::new()` to return `Result<Self>` instead of panicking with `.expect()`. | `hamr-core/src/config/settings.rs`, `hamr-core/src/config/validation.rs`, `hamr-core/src/config/dirs.rs`, `hamr-core/src/config/mod.rs` | done | medium |
| T52 | Scoring magic numbers → named constants — (a) In frecency/mod.rs: `MAX_FRECENCY_BOOST = 300.0`, `FRECENCY_MULTIPLIER = 10.0`, `HISTORY_BOOST = 200.0`, `MIN_SEQUENCE_CONFIDENCE = 0.1`, clamp `frecency_boost` lower bound to 0.0 with `.clamp()`. (b) In search/engine.rs: `EXACT_MATCH_BONUS = 500.0`, `PREFIX_MATCH_BASE = 250.0`. Add empty-query early return in `name_match_bonus`. | `hamr-core/src/frecency/mod.rs`, `hamr-core/src/search/engine.rs` | done | low |
| T53 | Silent index serialization fix + `chrono_lite_now` consolidation — (a) Replace `serde_json::to_value(items).unwrap_or_default()` in process.rs with match + warn! log. (b) Refactor `chrono_lite_now()` in store.rs to use `time_components_from_epoch()` from `utils.rs` (after T43 is done) and `SECS_PER_DAY`/`SECS_PER_HOUR` constants. NOTE: Depends on T43 completing first — only start this task after T43 is done. | `hamr-core/src/engine/process.rs` | done | medium |
| T54 | Daemon handler dedup and constants — (a) Extract `send_status_to_ui()` helper for the triplicated status+ambient notification pattern in handlers/mod.rs and handlers/plugin.rs. (b) Define constants for magic action IDs: `ACTION_BACK = "__back__"`, `ACTION_DISMISS = "__dismiss__"`, `ACTION_PLUGIN = "__plugin__"`. (c) Extract `PLUGIN_INIT_DELAY` constant for the 10ms sleep. (d) Replace remaining `unreachable!()` at line ~468 with safe error handling. | `hamr-daemon/src/handlers/mod.rs`, `hamr-daemon/src/handlers/plugin.rs` | done | medium |
| T55 | Watcher bridge thread tracking — store the bridging thread `JoinHandle` in both `PluginWatcher` and config watcher structs alongside `_watcher_thread`. Change `watch_config_file` parameter from `&PathBuf` to `&Path`. Remove fallback `"config.json"` string match or extract to constant. Rename `DEBOUNCE_DURATION` to `PLUGIN_DEBOUNCE_DURATION` / `CONFIG_DEBOUNCE_DURATION` for clarity. | `hamr-daemon/src/plugin_watcher.rs`, `hamr-daemon/src/config_watcher.rs` | done | low |
| T56 | Checksum resilience — (a) Use `continue` instead of `?` for malformed plugin entries in `ChecksumsData::load()` to avoid rejecting entire file. (b) Log warning on `compute_file_hash` I/O errors instead of silent `.ok()?`. | `hamr-core/src/plugin/checksum.rs` | done | low |
| T57 | CLI deduplication and error handling — (a) Extract `run_foreground(name)` helper to replace 4 duplicated binary-launch patterns. (b) Change `run_plugins_audit()` to return `Result<()>`. (c) Define `SKIP_PLUGIN_DIRS` constant for `"sdk"` / `"__pycache__"`. (d) Log warnings instead of `let _ =` for systemd command failures in `run_uninstall`. (e) Consolidate dev-detection: `is_dev_mode()` and `find_install_bin_dir()` use inconsistent heuristics. | `hamr-cli/src/main.rs` | done | low |
| T58 | Form data serialization dedup + ExecutionContext dedup — (a) Extract `fn serialize_form_data(data: &HashMap<String, String>) -> Value` helper in engine/mod.rs. (b) Replace manual `ExecutionContext` construction in `record_plugin_open()` with call to `build_execution_context()`. (c) Extract `fn active_plugin_info(&self) -> Option<(String, String)>` for the 4 duplicated `let Some(ref active) = ...` preambles. | `hamr-core/src/engine/mod.rs` (only the specified functions — does NOT touch `generate_session_id`, handled by T43) | done | low |
| T59 | Session ID uniqueness — replace timestamp-based `generate_session_id()` with atomic counter for guaranteed uniqueness. Define `static SESSION_COUNTER: AtomicU64`. NOTE: T43 changes `generate_session_id` first — coordinate or merge into T43. | `hamr-core/src/engine/mod.rs` (only `generate_session_id` fn) | done | low |
| T60 | Daemon IndexUpdate deserialization logging — replace `serde_json::from_value(items.clone()).unwrap_or_default()` in `forward_updates()` (server.rs:876) with match + warn! on failure. Extract duplicated "process updates and forward to UI" block in `forward_plugin_result` into helper. | `hamr-daemon/src/server.rs` (only forward_updates and forward_plugin_result functions — does NOT touch accept loop, handled by T41) | done | low |
