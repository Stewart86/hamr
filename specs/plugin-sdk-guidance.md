# Plugin SDK Guidance Refresh

## Responsibility
Document how the Python SDK aligns with the Rust release and give plugin authors clear manual testing steps.

## Requirements
- [ ] Update `plugins/sdk/README.md` with:
  - Quick start: create plugin dir, copy manifest template, implement handler.py
  - Testing workflow: `cargo run -p hamr-daemon` in terminal 1, test plugin in terminal 2
  - Environment variables: `HAMR_PLUGIN_DEBUG=1` for verbose SDK logging
  - Example manifest.json with all required fields
- [ ] Verify `pyproject.toml` requires Python 3.9+ (check `requires-python` field)
- [ ] Create manual testing checklist in CONTRIBUTING.md:
  1. Start daemon: `cargo run -p hamr-daemon`
  2. Tail logs: `tail -f /tmp/hamr-daemon.log`
  3. Start GTK or TUI: `cargo run -p hamr-gtk` or `cargo run -p hamr-tui`
  4. Type query that triggers your plugin
  5. Verify results appear and actions work
  6. Check logs for errors
- [ ] Document headless testing with TUI for environments without GTK

## Acceptance Criteria
- [ ] `plugins/sdk/README.md` has working quick start that new developer can follow
- [ ] `pip install -e plugins/sdk/` succeeds on Python 3.9+
- [ ] CONTRIBUTING.md has "Plugin Testing" section with checklist
- [ ] `python -m compileall plugins` succeeds with no syntax errors

## Edge Cases
- Plugins that need external dependencies (PIL, dbus) - document optional deps pattern

## Dependencies
- None (standalone docs task)
