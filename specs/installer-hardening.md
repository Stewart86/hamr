# Installer Hardening

## Responsibility
Ensure `install.sh` (and packaging flows invoking it) preserves user data, communicates actions clearly, and handles running services safely.

## Requirements
- [x] Preserve `~/.config/hamr`, user plugin directories, and other config data unless a new `--reset-user-data` flag is explicitly provided.
- [x] Add a dry-run mode (`--check`) that prints intended operations (build/install targets, service actions) without modifying the system.
- [x] Emit warnings/prompts before overwriting binaries or systemd units, with a `--yes`/`--non-interactive` flag to skip prompts for scripted installs.
- [x] Detect running hamr processes/services and provide instructions to stop/restart safely instead of blindly killing them.
- [ ] Update README/docs to describe new flags (`--check`, `--yes`, `--reset-user-data`), behavior, and expected outputs.

## Acceptance Criteria
- [x] Running the installer twice in a row leaves user plugins/config untouched by default.
- [x] `install.sh --check` exits 0 and leaves no filesystem changes (verified via `git status` or checksum snapshots).
- [x] Non-interactive installs (curl | bash, CI) still succeed with sensible defaults and log that user data was preserved.
- [ ] Docs describe all installer flags with examples.

## Edge Cases
- If stopping systemd services fails due to permissions, installer should warn but continue gracefully.
- Behavior with custom `HAMR_DIR` must still respect preservation/warning rules.

## Dependencies
- Docs refresh to communicate new behavior.
- Plugin security plan (installer reuses checksum warnings).

## Status: MOSTLY COMPLETE
Only docs update for new flags remains.
