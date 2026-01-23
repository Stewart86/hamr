# Implementation Plan

## Fixes
- [x] [Fix]: Correct CLI command references to use `hamr` instead of `hamr-daemon`/`hamr-gtk`/`hamr-cli`
  - Acceptance: All docs and examples use `hamr` binary for user-facing CLI commands; no `hamr-daemon query`, `hamr-cli`, etc.
  - Files: docs/getting-started/configuration.md, README.md, any other affected docs

## Docs Refresh (COMPLETE)
- [x] Update installation guide and README quick start for Rust/GTK
- [x] Refresh configuration guide with current schema/settings plugin details
- [x] Rewrite landing page + migration note; hide stale MkDocs sections

## Installer Hardening
- [x] Preserve user config/plugins by default (`--reset-user-data` override) - DONE in install.sh
- [x] Dry-run (`--check`) + verbose summary + overwrite prompts (`--yes` bypass) - DONE in install.sh
- [x] Running-process detection and safe stop/restart - DONE in install.sh
- [x] Document installer flags in README and docs
  - Acceptance: README has table of flags (`--check`, `--yes`, `--reset-user-data`) with descriptions
  - Files: README.md, docs/getting-started/installation.md

## Platform Support Clarity
- [x] Add compositor support matrix table
  - Acceptance: Table with columns (Compositor | Status | Notes) in README and docs
  - Content: Hyprland/Niri/Sway (supported), KDE Wayland (supported), GNOME/X11 (not supported)
  - Files: README.md, docs/getting-started/installation.md
- [x] Add troubleshooting table
  - Acceptance: Table with columns (Symptom | Cause | Solution) with 5+ common issues
  - Files: README.md or new docs/getting-started/troubleshooting.md
- [x] Document layer-shell package names per distro
  - Acceptance: Arch/Fedora/Ubuntu package names listed
  - Files: docs/getting-started/installation.md

## Plugin Security Enhancements
- [x] Create `scripts/generate-plugin-checksums.sh`
  - Acceptance: Script outputs `plugins/checksums.json` with SHA256 per plugin file
  - Files: scripts/generate-plugin-checksums.sh
- [x] Add checksum generation to release workflow
  - Acceptance: `checksums.json` included in release artifacts
  - Files: .github/workflows/release.yml
- [x] Add runtime checksum verification in daemon
  - Acceptance: Daemon logs warning on plugin checksum mismatch
  - Files: crates/hamr-daemon/src/... or crates/hamr-core/src/plugin/...
- [x] Add `hamr plugins audit` CLI command
  - Acceptance: Command lists plugins with verified/modified/unknown status
  - Files: crates/hamr-cli/src/main.rs
- [x] Update SECURITY.md with plugin trust model
  - Acceptance: Documents checksum system and audit command
  - Files: SECURITY.md

## Plugin SDK Guidance
- [x] Update `plugins/sdk/README.md` with quick start and testing workflow
  - Acceptance: README has: manifest template, handler example, testing steps with `cargo run -p hamr-daemon`
  - Reference: existing `plugins/sdk/hamr_sdk.py` docstring for example usage
  - Files: plugins/sdk/README.md
- [x] Verify `pyproject.toml` Python 3.9+ requirement
  - Acceptance: Check `requires-python` field in pyproject.toml
  - Files: pyproject.toml (if exists) or plugins/sdk/pyproject.toml
- [x] Add plugin testing checklist to CONTRIBUTING.md
  - Acceptance: Numbered steps documenting how to manually test plugins (for human developers to follow)
  - Reference: AGENTS.md "Debugging" section for log tail commands
  - Files: CONTRIBUTING.md

## Release Readiness Checklist
- [x] Create `scripts/release-check.sh`
  - Acceptance: Script runs fmt/clippy/test/build/mkdocs and exits non-zero on failure
  - Files: scripts/release-check.sh
- [x] Create `docs/releases/checklist.md`
  - Acceptance: Numbered manual steps for smoke testing before release
  - Files: docs/releases/checklist.md
- [x] Link checklist from CONTRIBUTING.md and release workflow
  - Acceptance: Links added to both files
  - Files: CONTRIBUTING.md, .github/workflows/release.yml

## Logging & Telemetry Docs
- [x] Create `docs/getting-started/logging.md`
  - Acceptance: Documents log paths, env vars (`RUST_LOG`, `HAMR_PLUGIN_DEBUG`), privacy statement
  - Reference: AGENTS.md "Debugging" section for log path patterns
  - Files: docs/getting-started/logging.md
- [ ] Add logging guide to mkdocs nav
  - Acceptance: Page appears in docs navigation
  - Files: mkdocs.yml
- [ ] Link logging guide from README and installation docs
  - Acceptance: Links added to troubleshooting sections
  - Files: README.md, docs/getting-started/installation.md

## Cleanup Tasks
- [ ] Run `mkdocs build` - verify no broken links
- [ ] Run `cargo fmt --all -- --check`
- [ ] Run `cargo clippy --all-targets`
- [ ] Run `cargo test --all`
- [ ] Run `python -m compileall plugins` (optional)
