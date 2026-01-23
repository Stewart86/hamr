# Configuration

Hamr is configured via `~/.config/hamr/config.json`. Use the built-in settings plugin (`/settings`) to browse and modify options - no manual editing needed.

## Configuration Reference

| Category       | Option                   | Default                     | Description                                                                |
| -------------- | ------------------------ | --------------------------- | -------------------------------------------------------------------------- |
| **Apps**       | `terminal`               | _(auto-detected)_           | Terminal emulator for shell commands                                       |
|                | `fileManager`            | _(auto-detected)_           | File manager for file operations                                           |
|                | `browser`                | _(auto-detected)_           | Web browser for URL opening                                                |
| **Search**     | `maxDisplayedResults`    | `16`                        | Maximum results shown in launcher                                          |
|                | `maxRecentItems`         | `20`                        | Recent history items on empty search                                       |
|                | `maxResultsPerPlugin`    | `0`                         | Hard limit per plugin (0 = no limit)                                       |
|                | `pluginDebounceMs`       | `150`                       | Search input debounce (ms)                                                 |
|                | `diversityDecay`         | `0.7`                       | Decay for consecutive results from same plugin (0-1, lower = more diverse) |
|                | `engineBaseUrl`          | `https://www.google.com/search?q=` | Default search engine URL                                   |
|                | `excludedSites`          | `[]`                        | Sites to exclude from web search results                                   |
|                | `pluginRankingBonus`     | `{}`                        | Per-plugin score boosts (e.g., `{"apps": 200}`)                            |
| **Action Bar** | `actionBarHints`         | _see below_                 | Customizable action bar shortcuts                                          |

## Prefix Shortcuts

Search prefixes let you quickly jump to specific functionality by typing a character:

| Prefix | Option         | Function                     |
| ------ | -------------- | ---------------------------- |
| `/`    | `plugins`      | Search plugins by name       |
| `@`    | `app`          | Application search           |
| `:`    | `emojis`       | Emoji search                 |
| `=`    | `math`         | Calculator                   |
| `!`    | `shellCommand` | Shell command                |
| `?`    | `webSearch`    | Web search                   |

Configure these in `config.json` under the `search` section:

```json
{
  "search": {
    "plugins": "/",
    "app": "@",
    "emojis": ":",
    "math": "=",
    "shellCommand": "!",
    "webSearch": "?"
  }
}
```

## Action Bar Hints

The action bar shortcuts are fully customizable via `actionBarHints`. Default hints include:

```json
{
  "search": {
    "actionBarHints": [
      { "prefix": "~", "icon": "folder_open", "label": "Files", "plugin": "files" },
      { "prefix": ";", "icon": "content_paste", "label": "Clipboard", "plugin": "clipboard" },
      { "prefix": "=", "icon": "calculate", "label": "Calculate", "plugin": "calculate" },
      { "prefix": ":", "icon": "emoji_emotions", "label": "Emoji", "plugin": "emoji" },
      { "prefix": "!", "icon": "terminal", "label": "Shell", "plugin": "shell" }
    ]
  }
}
```

Each hint has:

- **prefix**: The trigger character (e.g., `~`, `;`, `:`)
- **icon**: Icon name (GTK-compatible icon names)
- **label**: Display name shown in the action bar
- **plugin**: Plugin ID to launch
- **description**: Optional description (shown in settings)

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

### Sway

```bash
# ~/.config/sway/config

# Open clipboard directly with Mod+V
bindsym $mod+V exec hamr plugin clipboard

# Open emoji picker with Mod+Period
bindsym $mod+Period exec hamr plugin emoji

# Open file search with Mod+E
bindsym $mod+E exec hamr plugin files
```

## File Structure

```
~/.config/hamr/
├── plugins/                     # User plugins (override built-in)
├── config.json                  # User configuration
├── quicklinks.json              # Custom quicklinks
└── plugin-indexes.json          # Plugin data and frecency (auto-generated)

~/.local/share/hamr/             # Installation directory (AUR/manual)
├── plugins/                     # Built-in plugins (read-only)
└── daemon                       # Hamr daemon binary
```

## Troubleshooting

### "I ran `hamr` but nothing appears"

This is expected. Hamr starts hidden and waits for a toggle signal. Make sure you:

1. Added the keybinding to your compositor config (see [Installation](installation.md))
2. Reloaded your compositor config
3. Press your keybind (e.g., Super key or Ctrl+Space)

### Check daemon status

```bash
# Check if daemon is running
hamr status

# View daemon logs
tail -f /tmp/hamr-daemon.log
```

### View logs

Hamr writes debug logs to `/tmp/` with symlinks to the latest:
- Daemon: `/tmp/hamr-daemon.log`
- TUI: `/tmp/hamr-tui.log`
- GTK: `/tmp/hamr-gtk.log`

```bash
# Follow daemon logs in real-time
tail -f /tmp/hamr-daemon.log

# View recent entries from both
tail -n 100 /tmp/hamr-daemon.log /tmp/hamr-tui.log

# Search for specific patterns
grep -i "error\|warn" /tmp/hamr-daemon.log
```

### Configuration issues

If your config isn't working:

1. Check JSON syntax: `python -m json.tool ~/.config/hamr/config.json`
2. Verify daemon picks up changes: `hamr daemon-reload`
3. Check for unknown fields in logs: `grep "unknown field" /tmp/hamr-daemon.log`

### Plugin not responding

```bash
# Check if plugin is connected
grep "plugin" /tmp/hamr-daemon.log | tail -20

# Look for action forwarding issues
grep "handle_item_selected\|Forwarding action" /tmp/hamr-daemon.log
```

### Warning about missing `colors.json`

This is harmless. Hamr uses built-in default colors. For dynamic theming from your wallpaper, install [matugen](https://github.com/InioX/matugen) and use the wallpaper plugin.

### Warning about missing `quicklinks.json`

This is harmless. Quicklinks are optional. To add quicklinks, create `~/.config/hamr/quicklinks.json`:

```json
[{ "name": "GitHub", "url": "https://github.com", "icon": "code" }]
```
