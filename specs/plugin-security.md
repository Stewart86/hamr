# Plugin Security Enhancements

## Responsibility
Detect tampering with bundled plugins and document the trust model so users understand the risks of user-installed plugins.

## Requirements
- [ ] Generate a checksum manifest for bundled plugins during release packaging; include it in artifacts for installer/daemon use.
- [ ] Verify the manifest at runtime (daemon/core) and log warnings when built-in plugin files no longer match expected checksums; provide an opt-in fail-fast flag for strict mode.
- [ ] Update installer to compare destination plugin files against the manifest, warning users when local modifications exist while still preserving their data.
- [ ] Extend `SECURITY.md` and docs with explanations of plugin discovery directories, checksum behavior, and recommended audit steps before running third-party plugins.
- [ ] Add a CLI command (e.g., `hamr plugins audit`) that lists plugins, their source directories, and checksum status so users can quickly inspect their installation.

## Acceptance Criteria
- [ ] Startup logs include checksum validation success or actionable warnings when mismatches occur.
- [ ] Installer outputs warnings when shipping plugins differ from on-disk versions but leaves user files intact.
- [ ] CLI audit command reports each plugin with trust status and exits non-zero if mismatches are found (optional flag).
- [ ] Security documentation references the checksum system and audit command.

## Edge Cases
- Manifest must cover all shipped files (scripts + manifests) and degrade gracefully if missing (log info instead of panic).
- Audit command must work headless (no GTK dependency) and handle installations without manifest on first release.

## Dependencies
- Installer hardening (shares warning surface).
- Docs refresh (security section references new behavior).
