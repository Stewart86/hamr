# Logging & Telemetry Documentation

## Responsibility
Explain where logs live, how to adjust verbosity, and clarify privacy expectations (no telemetry) without changing runtime behavior.

## Requirements
- [ ] Add a logging guide (new page or section under docs/getting-started) covering hamr-daemon, hamr-gtk, and hamr-tui log locations, how to tail them, and retention behavior (tmp symlinks).
- [ ] Document environment variables (`RUST_LOG`, `HAMR_PLUGIN_DEBUG`, etc.) for adjusting verbosity and how to disable file logging if desired.
- [ ] State explicitly that no telemetry is sent; list what local data might be present in logs (queries, plugin ids) so users can redact before sharing.
- [ ] Link the logging guide from README troubleshooting, installer instructions, and other relevant docs sections.

## Acceptance Criteria
- [ ] Users can follow the docs to locate each component’s logs and adjust verbosity.
- [ ] Privacy statement clarifies data stays local unless logs are shared manually.
- [ ] Logging doc referenced from README and troubleshooting sections.

## Edge Cases
- Mention tmpfs implications (logs wiped on reboot) and how to archive logs manually when needed.

## Dependencies
- Docs refresh for cross-linking.
