# Rust GTK Docs Refresh

## Responsibility
Provide accurate installation/configuration guidance for the Rust/GTK launcher targeting supported Wayland compositors.

## Requirements
- [x] Replace `docs/getting-started/installation.md` with instructions covering Rust build requirements, Wayland prerequisites (GTK4 + layer shell), curl installer, AUR package, and manual builds (including keybinding setup).
- [x] Update `docs/getting-started/configuration.md` to describe the current config schema, settings plugin, file locations, and CLI helpers actually used by the Rust launcher.
- [x] Refresh `docs/index.md` and README quick-start sections to showcase GTK features, highlight supported compositors, and remove Quickshell/QML wording.
- [x] Add a migration note directing legacy QML users to the previous branch/tag or archived docs.
- [x] Prune MkDocs navigation so only refreshed pages appear for this release (hide or remove stale sections).

## Acceptance Criteria
- [x] `mkdocs build` succeeds with no broken links and no QML references outside the migration notice.
- [x] README, docs landing page, and installation guide consistently mention supported compositors and Rust toolchain requirements.
- [x] Installation/configuration pages include dependency names per major distro (Arch, Fedora, Debian/Ubuntu) and verified commands.

## Edge Cases
- Distros missing packaged `gtk4-layer-shell`: document manual build instructions or known community packages.
- Provide both systemd-managed and manual launch instructions so users without systemd are covered.

## Dependencies
- Installer hardening changes (docs must match new flags/behavior).
- Platform support clarity language.

## Status: COMPLETE
