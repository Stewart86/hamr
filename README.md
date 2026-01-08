# Hamr

> "When all you have is a hammer, everything looks like a nail"

<div align="center">

```bash
paru -S hamr
```

[![AUR version](https://img.shields.io/aur/version/hamr)](https://aur.archlinux.org/packages/hamr)
[![Documentation](https://img.shields.io/badge/docs-stewart86.github.io%2Fhamr-blue)](https://stewart86.github.io/hamr)

</div>

Hamr is an extensible launcher for Wayland compositors built with [Quickshell](https://quickshell.outfoxxed.me/). Extend it with plugins in any language using a simple JSON protocol.

**Supported Compositors:** Hyprland, Niri

![Hamr Main View](assets/screenshots/hamr-main-view.png)

## Philosophy

- **Minimalist UI** - Clean, modern, no visual clutter
- **Zero Configuration** - Works out of the box with sensible defaults
- **Minimum Interactions** - Every feature optimized for fewest possible keystrokes
- **Learns Your Habits** - Frecency ranking means frequently-used items rise to the top automatically
- **Keyboard-First** - Full functionality without touching the mouse

## Features

- **Frecency-based ranking** - Results sorted by frequency + recency
- **Fuzzy matching** - Fast, typo-tolerant search
- **Smart suggestions** - Context-aware app suggestions based on time, workspace, and usage patterns
- **30+ built-in plugins** - Apps, clipboard, emoji, files, calculator, and more
- **Extensible** - Write plugins in Python, Bash, Go, or any language

[**View all features and plugins**](https://stewart86.github.io/hamr/features/)

## Quick Start

### Installation

```bash
# Arch Linux (AUR)
paru -S hamr

# Enable and start
systemctl --user enable --now hamr
```

[**Full installation guide**](https://stewart86.github.io/hamr/getting-started/installation/)

### Keybinding

Bind `hamr toggle` to a key in your compositor:

=== "Hyprland"

    ```bash
    # ~/.config/hypr/hyprland.conf
    exec-once = hamr
    bind = $mainMod, SPACE, exec, hamr toggle
    ```

=== "Niri"

    ```kdl
    // ~/.config/niri/config.kdl
    binds {
        Mod+Space { spawn "hamr" "toggle"; }
    }
    ```

### Usage

| Prefix | Function |
|--------|----------|
| (none) | Search apps and indexed items |
| `~` | File search |
| `;` | Clipboard history |
| `/` | Plugins |
| `=` | Calculator |
| `:` | Emoji |

## Creating Plugins

Plugins are simple scripts that communicate via JSON over stdin/stdout:

```python
#!/usr/bin/env python3
import json, sys

data = json.load(sys.stdin)
print(json.dumps({
    "type": "results",
    "results": [{"id": "1", "name": "Hello World", "icon": "waving_hand"}]
}))
```

[**Plugin development guide**](https://stewart86.github.io/hamr/plugins/)

## Documentation

- [Installation](https://stewart86.github.io/hamr/getting-started/installation/)
- [Configuration](https://stewart86.github.io/hamr/getting-started/configuration/)
- [Theming](https://stewart86.github.io/hamr/getting-started/theming/)
- [Building Plugins](https://stewart86.github.io/hamr/plugins/)
- [Features & Plugins](https://stewart86.github.io/hamr/features/)

## Privacy

Hamr is fully local and offline. **No data ever leaves your machine.** No network requests, analytics, or telemetry.

## Credits

Hamr was originally extracted and adapted from [end-4's illogical-impulse](https://github.com/end-4/dots-hyprland). Major thanks to end-4 for the Material Design theming, fuzzy search, widget components, and overall architecture.

## License

GPL-3.0 - See [LICENSE](LICENSE)
