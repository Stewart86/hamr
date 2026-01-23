# Plugin SDK Guidance Refresh

## Responsibility
Document how the Python SDK aligns with the Rust release and give plugin authors clear manual testing steps with hamr-daemon/hamr-gtk.

## Requirements
- [ ] Update `plugins/sdk/README.md` and docs/plugins to describe how to launch hamr-daemon + hamr-gtk for manual plugin testing, including useful environment variables and logging tips.
- [ ] Document the release process for SDK/protocol changes: when protocol fields change, update README changelog + migration notes and ensure bundled plugins are updated accordingly.
- [ ] Adjust `pyproject.toml` to require Python 3.9+ and include minimal dev dependencies needed for docs/tests referencing the SDK.
- [ ] Add a manual plugin testing checklist (CONTRIBUTING or docs) outlining steps authors should follow (start daemon, observe logs, validate UI results, handle errors/headless `hamr-tui`).

## Acceptance Criteria
- [ ] Docs clearly describe the recommended manual testing workflow and link to relevant commands.
- [ ] `pyproject.toml` installation succeeds on Python 3.9+ and supports docs builds.
- [ ] Release instructions remind maintainers to update SDK docs whenever the protocol changes.

## Edge Cases
- Include guidance for environments without GTK (use `hamr-tui` or CLI to validate plugin responses).

## Dependencies
- Docs refresh (plugins section updated).
