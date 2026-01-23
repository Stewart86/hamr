# Logging & Telemetry Documentation

## Responsibility
Explain where logs live, how to adjust verbosity, and clarify privacy expectations (no telemetry) without changing runtime behavior.

## Requirements
- [ ] Create `docs/getting-started/logging.md` with:
  - Log file locations table (reference AGENTS.md "Debugging" section for paths)
  - Symlink behavior explanation
  - Timestamped files pattern
- [ ] Document environment variables:
  - `RUST_LOG=debug|info|warn|error` for verbosity
  - `RUST_LOG=hamr=debug` for hamr-only debug
  - `HAMR_PLUGIN_DEBUG=1` for plugin SDK debug output
- [ ] Add privacy statement: no telemetry sent, logs contain queries/plugin IDs, redact before sharing
- [ ] Add links to logging guide from:
  - README.md troubleshooting section
  - docs/getting-started/installation.md
  - docs/getting-started/configuration.md

## Source of Truth
- Log path patterns defined in AGENTS.md "Debugging" section
- Env vars defined in `plugins/sdk/hamr_sdk.py` (`HAMR_PLUGIN_DEBUG`)
- Rust logging uses standard `RUST_LOG` env var via tracing crate

## Acceptance Criteria
- [ ] `docs/getting-started/logging.md` exists and `mkdocs build` succeeds
- [ ] Content matches what's documented in AGENTS.md
- [ ] Privacy statement explicitly says "no data leaves your machine"

## Edge Cases
- Mention tmpfs implications (logs wiped on reboot)
- Include archive command example in docs

## Dependencies
- None (standalone docs task)
