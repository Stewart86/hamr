# Platform Support Clarity

## Responsibility
Document the exact environments the Rust/GTK launcher supports today and give guidance for unsupported setups.

## Requirements
- [ ] Create support matrix table in README and docs with columns: Compositor | Status | Notes
  - Hyprland: Supported
  - Niri: Supported  
  - Sway: Supported
  - KDE Plasma (Wayland): Supported (requires layer-shell)
  - GNOME: Not supported (no wlr-layer-shell)
  - X11: Not supported
  - macOS/Windows: Future work
- [ ] Add troubleshooting table with columns: Symptom | Cause | Solution
  - "Layer shell not supported" error -> Compositor lacks wlr-layer-shell -> Switch compositor or use TUI
  - Launcher doesn't appear -> Keybinding not set -> Add to compositor config
  - Daemon not running -> Service not started -> `systemctl --user start hamr-daemon`
- [ ] Document layer-shell package names per distro:
  - Arch: `gtk4-layer-shell`
  - Fedora: `gtk4-layer-shell`
  - Ubuntu/Debian: `libgtk-4-layer-shell-dev` (or build from source)
- [ ] Link to GitHub issue for macOS/Windows tracking

## Acceptance Criteria
- [ ] Same support matrix appears in README and docs/getting-started/installation.md
- [ ] Troubleshooting table has at least 5 common issues with solutions
- [ ] Users on unsupported compositors get clear "not supported" message with alternatives

## Edge Cases
- Some compositors have partial layer-shell support - document known limitations

## Dependencies
- None (can be done independently)
