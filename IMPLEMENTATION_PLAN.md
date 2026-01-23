# Implementation Plan

## Fixes
- [x] [Fix]: Correct CLI command references to use `hamr` instead of `hamr-daemon`/`hamr-gtk`/`hamr-cli`
  - Acceptance: All docs and examples use `hamr` binary for user-facing CLI commands; no `hamr-daemon query`, `hamr-cli`, etc.
  - Files: docs/getting-started/configuration.md, README.md, any other affected docs

## Docs Refresh
- [x] Update installation guide and README quick start for Rust/GTK (Wayland deps, curl script, AUR, manual build, keybindings).
  - Acceptance: `mkdocs build` clean; instructions verified on at least one distro; README matches guide.
  - Files: docs/getting-started/installation.md, README.md, mkdocs.yml (nav)
- [x] Refresh configuration guide with current schema/settings plugin details and remove QML references.
  - Acceptance: lists real config paths/options; screenshots/text updated; mkdocs links resolved.
  - Files: docs/getting-started/configuration.md
- [x] Rewrite landing page + migration note; hide stale MkDocs sections.
  - Acceptance: docs/index.md + mkdocs nav only reference GTK content; migration section links to old branch/tag.
  - Files: docs/index.md, mkdocs.yml

## Installer Hardening
- [ ] Preserve user config/plugins by default and add `--reset-user-data` override.
  - Acceptance: repeated installs leave `~/.config/hamr` untouched unless flag set.
  - Files: install.sh
- [ ] Add dry-run (`--check`) + verbose summary, plus prompts for overwrites with `--yes` bypass.
  - Acceptance: `install.sh --check` makes no changes; interactive run shows prompts.
  - Files: install.sh
- [ ] Improve running-process detection and messaging (`--force` for overrides); update docs for new flags.
  - Acceptance: installer logs instructions instead of blindly killing; README/docs describe flags.
  - Files: install.sh, README.md, docs/getting-started/installation.md

## Platform Support Clarity
- [ ] Document supported compositors/requirements and troubleshooting table.
  - Acceptance: README + docs share same support matrix and troubleshooting guidance.
  - Files: README.md, docs/index.md, docs/getting-started/installation.md (or new troubleshooting page)

## Plugin Security Enhancements
- [ ] Generate plugin checksum manifest during packaging and ship with releases.
  - Acceptance: manifest produced by script, included in release artifacts, and referenced by installer/daemon.
  - Files: scripts/*, release workflow, pkg assets
- [ ] Verify checksums at runtime + installer warnings; add CLI audit command and doc updates.
  - Acceptance: daemon logs mismatches; `hamr plugins audit` reports status; docs/security updated.
  - Files: crates/hamr-core or hamr-daemon, crates/hamr-cli, install.sh, SECURITY.md, docs/security section

## Plugin SDK Guidance
- [ ] Refresh SDK README/docs with manual testing workflow + env vars/logging tips.
  - Acceptance: docs/plugins and SDK README align; manual steps clearly outlined.
  - Files: plugins/sdk/README.md, docs/plugins/*, CONTRIBUTING.md
- [ ] Update `pyproject.toml` Python requirement and ensure docs/tests referencing SDK install cleanly.
  - Acceptance: `pip install -e .` works on Python 3.9+; optional `python -m compileall plugins` succeeds.
  - Files: pyproject.toml, docs build instructions
- [ ] Add manual plugin testing checklist (docs/CONTRIBUTING) referenced from README.
  - Acceptance: checklist exists and is linked.
  - Files: docs/plugins or docs/contributing, README.md

## Release Readiness Checklist
- [ ] Create release checklist doc and helper script (`scripts/release-check.sh`).
  - Acceptance: checklist published under docs/releases; script runs fmt/clippy/test/mkdocs and is referenced in docs and release workflow.
  - Files: docs/releases/checklist.md, scripts/release-check.sh, CONTRIBUTING.md, .github/workflows/release.yml

## Logging & Telemetry Docs
- [ ] Add logging guide covering log paths, env vars, privacy statements; link from README/troubleshooting/installer docs.
  - Acceptance: logging page exists, mkdocs build passes, references added where needed.
  - Files: docs/getting-started/logging.md (or similar), README.md, docs/getting-started/installation.md

## Cleanup Tasks
- [ ] Run `mkdocs build` and spell/markdown checks after all doc updates; ensure nav coherent.
- [ ] Run `cargo fmt`, `cargo clippy --all-targets`, `cargo test --all`, and `python -m compileall plugins` (optional) before final release checklist commit.
