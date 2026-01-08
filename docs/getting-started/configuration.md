# Configuration

Hamr is configured via `~/.config/hamr/config.json`. Use the built-in settings plugin (`/settings`) to browse and modify options - no manual editing needed.

## Configuration Reference

| Category | Option | Default | Description |
|----------|--------|---------|-------------|
| **Apps** | `terminal` | `ghostty` | Terminal emulator for shell commands |
| | `terminalArgs` | `--class=floating.terminal` | Arguments passed to terminal |
| | `shell` | `zsh` | Shell for command execution (zsh, bash, fish) |
| **Behavior** | `stateRestoreWindowMs` | `30000` | Time (ms) to preserve state after soft close (0 to disable) |
| | `clickOutsideAction` | `intuitive` | Click outside behavior: `intuitive`, `close`, or `minimize` |
| **Search** | `maxDisplayedResults` | `16` | Maximum results shown in launcher |
| | `maxRecentItems` | `20` | Recent history items on empty search |
| | `debounceMs` | `50` | Search input debounce (ms) |
| | `diversityDecay` | `0.7` | Decay for consecutive results from same plugin (0-1, lower = more diverse) |
| | `maxResultsPerPlugin` | `0` | Hard limit per plugin (0 = no limit) |
| **Appearance** | `backgroundTransparency` | `0.2` | Background transparency (0-1) |
| | `launcherXRatio` | `0.5` | Horizontal position (0=left, 1=right) |
| | `launcherYRatio` | `0.1` | Vertical position (0=top, 1=bottom) |
| | `fontScale` | `1` | Font scaling factor (0.75=min, 1.5=max) |
| **Sizes** | `searchWidth` | `580` | Search bar width (px) |
| | `maxResultsHeight` | `600` | Max results container height (px) |
| **Paths** | `wallpaperDir` | `""` | Custom wallpaper directory (empty = ~/Pictures/Wallpapers) |
| | `colorsJson` | `""` | Custom colors.json path (empty = ~/.config/hamr/colors.json) |

## Prefix Shortcuts

The action bar shortcuts are fully customizable. Edit `~/.config/hamr/config.json`:

```json
{
  "search": {
    "actionBarHints": [
      { "prefix": "~", "icon": "folder", "label": "Files", "plugin": "files" },
      { "prefix": ";", "icon": "content_paste", "label": "Clipboard", "plugin": "clipboard" },
      { "prefix": "/", "icon": "extension", "label": "Plugins", "plugin": "action" },
      { "prefix": "!", "icon": "terminal", "label": "Shell", "plugin": "shell" },
      { "prefix": "=", "icon": "calculate", "label": "Math", "plugin": "calculate" },
      { "prefix": ":", "icon": "emoji_emotions", "label": "Emoji", "plugin": "emoji" }
    ]
  }
}
```

Each hint has:

- **prefix**: The trigger character (e.g., `~`, `;`, `:`)
- **icon**: [Material Symbol](https://fonts.google.com/icons) name
- **label**: Display name shown in the action bar
- **plugin**: Plugin ID to launch or `action` for plugin search mode

## Direct Plugin Keybindings

You can bind keys to open specific plugins directly:

```bash
hamr plugin <plugin_name>
```

### Hyprland

```bash
# ~/.config/hypr/hyprland.conf

# Open clipboard directly with Mod+V
bind = SUPER, V, exec, hamr plugin clipboard

# Open emoji picker with Mod+Period
bind = SUPER, Period, exec, hamr plugin emoji

# Open file search with Mod+E
bind = SUPER, E, exec, hamr plugin files
```

### Niri

```kdl
// ~/.config/niri/config.kdl
binds {
    // Open clipboard directly with Mod+V
    Mod+V { spawn "hamr" "plugin" "clipboard"; }

    // Open emoji picker with Mod+Period
    Mod+Period { spawn "hamr" "plugin" "emoji"; }

    // Open file search with Mod+E
    Mod+E { spawn "hamr" "plugin" "files"; }
}
```

## File Structure

```
~/.config/hamr/
├── plugins/                     # User plugins (override built-in)
├── config.json                  # User configuration
├── quicklinks.json              # Custom quicklinks
└── plugin-indexes.json          # Plugin data and frecency (auto-generated)

~/.local/share/hamr/             # Installation directory (AUR/manual)
├── shell.qml                    # Entry point
├── plugins/                     # Built-in plugins (read-only)
├── modules/                     # UI components
└── services/                    # Core services
```

## Troubleshooting

### "I ran `hamr` but nothing appears"

This is expected. Hamr starts hidden and waits for a toggle signal. Make sure you:

1. Added the keybinding to your compositor config (see [Installation](installation.md))
2. Reloaded your compositor config
3. Press your keybind (e.g., Super key or Ctrl+Space)

### Check dependencies

```bash
hamr --check-deps
```

### View logs

```bash
journalctl --user -u hamr -f
```

### Crash with Qt version mismatch

```
WARN: Quickshell was built against Qt 6.10.0 but the system has updated to Qt 6.10.1...
```

This happens when Qt is updated but Quickshell wasn't rebuilt:

```bash
paru -S quickshell --rebuild
# or for quickshell-git:
paru -S quickshell-git --rebuild
```

### Warning about missing `colors.json`

This is harmless. Hamr uses built-in default colors. For dynamic theming from your wallpaper, install [matugen](https://github.com/InioX/matugen) and use the wallpaper plugin.

### Warning about missing `quicklinks.json`

This is harmless. Quicklinks are optional. To add quicklinks, create `~/.config/hamr/quicklinks.json`:

```json
[
  {"name": "GitHub", "url": "https://github.com", "icon": "code"}
]
```
