# Platform Support Clarity

## Responsibility
Document the exact environments the Rust/GTK launcher supports today and give guidance for unsupported setups.

## Requirements
- [ ] Update README and docs to list supported compositors (e.g., Hyprland, Niri, Sway, KDE w/ layer-shell) with explicit Wayland + wlr-layer-shell requirement.
- [ ] Introduce a dedicated Platform Support section noting that macOS/Windows are future work and linking to their tracking issue/roadmap.
- [ ] Describe the runtime error produced when `hamr-gtk` detects missing layer-shell support and outline remediation steps (install packages, switch compositor).
- [ ] Provide a troubleshooting table covering keybinding setup, service startup, and compositor-specific quirks.

## Acceptance Criteria
- [ ] README, docs home, installer output, and troubleshooting sections use the same support matrix language.
- [ ] Users encountering unsupported compositors can follow documented steps to resolve or understand limitations.
- [ ] Roadmap links (issues or docs) exist for planned platforms outside Wayland.

## Edge Cases
- Package names differ per distro; include distro-specific dependency names or link to upstream documentation.

## Dependencies
- Docs refresh for shared pages.
