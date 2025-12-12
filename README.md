# Hamr

> "When all you have is a hammer, everything looks like a nail"

Hamr is an extensible launcher for Hyprland built with [Quickshell](https://quickshell.outfoxxed.me/). Extend it with plugins written in Python using a simple JSON protocol.

## Compatibility

Hamr works standalone but is **best used alongside [end-4's illogical-impulse](https://github.com/end-4/dots-hyprland)** dotfiles. Many built-in plugins (wallpaper switching, theme toggling, etc.) are designed to integrate with illogical-impulse's theming system.

## Credits

Hamr is extracted and adapted from [end-4's illogical-impulse](https://github.com/end-4/dots-hyprland). Major thanks to end-4 for the Material Design theming, fuzzy search, widget components, and overall architecture.

## Features

- **Frecency-based ranking** - Results sorted by frequency + recency (inspired by [zoxide](https://github.com/ajeetdsouza/zoxide))
- **Intent detection** - Auto-detects URLs, math expressions, and commands
- **Fuzzy matching** - Fast, typo-tolerant search powered by [fuzzysort](https://github.com/farzher/fuzzysort)
- **Extensible plugins** - Python handlers with simple JSON protocol
- **History tracking** - Search, plugin actions, and shell command history

### Prefix Shortcuts

| Prefix | Function | Prefix | Function |
|--------|----------|--------|----------|
| `~` | File search | `;` | Clipboard history |
| `/` | Actions & plugins | `!` | Shell history |
| `=` | Math calculation | `:` | Emoji picker |

### Built-in Plugins

| Plugin | Trigger | Description |
|--------|---------|-------------|
| `files` | `~` | File search with fd + fzf, thumbnails for images |
| `clipboard` | `;` | Clipboard history with image support |
| `shell` | `!` | Shell command history (zsh/bash/fish) |
| `quicklinks` | `/quicklinks` | Web search with customizable quicklinks |
| `dict` | `/dict` | Dictionary lookup with definitions |
| `wallpaper` | `/wallpaper` | Wallpaper selector (illogical-impulse) |

## Installation

### Prerequisites

- [Hyprland](https://hyprland.org/) (required)
- [Quickshell](https://quickshell.outfoxxed.me/)
- Optional: [illogical-impulse](https://github.com/end-4/dots-hyprland) for full theme integration
- Optional: `fd`, `fzf`, `cliphist`, `wl-clipboard` for respective plugins

### Steps

```bash
# 1. Clone and copy to Quickshell config
git clone https://github.com/stewart86/hamr.git
mkdir -p ~/.config/quickshell
cp -r hamr ~/.config/quickshell/hamr

# 2. Symlink plugins
mkdir -p ~/.config/hamr
ln -s ~/.config/quickshell/hamr/actions ~/.config/hamr/actions

# 3. Add Hyprland keybinding (~/.config/hypr/hyprland.conf)
# bind = Super, Super_L, global, quickshell:hamrToggle

# 4. Start Quickshell
qs -c hamr

# 5. (Optional) Auto-start with Hyprland (~/.config/hypr/hyprland.conf)
# exec-once = qs -c hamr
```

<details>
<summary><strong>Clipboard Support (Optional)</strong></summary>

```bash
# Install cliphist (Arch: pacman -S cliphist wl-clipboard)

# Add to Hyprland startup
exec-once = wl-paste --type text --watch cliphist store
exec-once = wl-paste --type image --watch cliphist store
```

</details>

<details>
<summary><strong>Theming</strong></summary>

Hamr uses Material Design colors from `~/.local/state/user/generated/colors.json`.

- **With illogical-impulse**: Colors are automatically generated from your wallpaper
- **Standalone**: Hamr uses built-in default colors (dark theme)

</details>

## Creating Plugins

Plugins live in `~/.config/hamr/actions/`. Each plugin is either:
- A **folder** with `manifest.json` + `handler.py` (multi-step plugins)
- An **executable script** (simple one-shot actions)

### What Plugins Can Do

| Capability | Description |
|------------|-------------|
| **Multi-step navigation** | Show lists, let users drill down, navigate back |
| **Rich cards** | Display markdown content (definitions, previews, help) |
| **Image thumbnails** | Show image previews in result lists |
| **Action buttons** | Add context actions per item (copy, delete, open folder) |
| **Image browser** | Full image browser UI with directory navigation |
| **Execute commands** | Run any shell command, optionally save to history |
| **Custom placeholders** | Change search bar placeholder text per step |
| **Live search** | Filter results as user types |

<details>
<summary><strong>Quick Start: Hello World Plugin</strong></summary>

```bash
mkdir -p ~/.config/hamr/actions/hello
```

**manifest.json:**
```json
{
  "name": "Hello World",
  "description": "A simple greeting plugin",
  "icon": "waving_hand"
}
```

**handler.py:**
```python
#!/usr/bin/env python3
import json
import sys

def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")

    if step == "initial":
        print(json.dumps({
            "type": "results",
            "results": [
                {"id": "english", "name": "Hello!", "icon": "language"},
                {"id": "spanish", "name": "Hola!", "icon": "language"},
            ]
        }))
        return

    if step == "action":
        selected_id = input_data.get("selected", {}).get("id", "")
        print(json.dumps({
            "type": "card",
            "card": {
                "title": "Greeting",
                "content": f"You selected: {selected_id}",
                "markdown": True
            }
        }))

if __name__ == "__main__":
    main()
```

```bash
chmod +x ~/.config/hamr/actions/hello/handler.py
```

Type `/hello` to try it!

</details>

<details>
<summary><strong>JSON Protocol Reference</strong></summary>

#### Input (stdin)

```json
{
  "step": "initial|search|action",
  "query": "user's search text",
  "selected": {"id": "selected-item-id"},
  "action": "action-button-id",
  "session": "unique-session-id"
}
```

#### Output (stdout)

**Show Results:**
```json
{
  "type": "results",
  "results": [
    {
      "id": "unique-id",
      "name": "Display Name",
      "description": "Optional subtitle",
      "icon": "material_icon_name",
      "thumbnail": "/path/to/image.png",
      "actions": [{"id": "copy", "name": "Copy", "icon": "content_copy"}]
    }
  ],
  "placeholder": "Custom placeholder..."
}
```

**Show Card:**
```json
{
  "type": "card",
  "card": {"title": "Title", "content": "Markdown **content**", "markdown": true}
}
```

**Execute Command:**
```json
{
  "type": "execute",
  "execute": {
    "command": ["xdg-open", "/path/to/file"],
    "name": "Open file.pdf",
    "close": true
  }
}
```

**Open Image Browser:**
```json
{
  "type": "imageBrowser",
  "imageBrowser": {
    "directory": "~/Pictures",
    "title": "Select Image",
    "actions": [{"id": "set_wallpaper", "name": "Set Wallpaper", "icon": "wallpaper"}]
  }
}
```

</details>

<details>
<summary><strong>Simple Actions (Scripts)</strong></summary>

For one-shot actions, create executable scripts directly:

```bash
#!/bin/bash
# ~/.config/hamr/actions/screenshot
grim -g "$(slurp)" - | wl-copy
notify-send "Screenshot copied"
```

Appears in search as `/screenshot`.

</details>

## Configuration

`~/.config/hamr/config.json`:

```json
{
  "search": {
    "webSearch": {"name": "DuckDuckGo", "url": "https://duckduckgo.com/?q={query}"},
    "prefix": {
      "action": "/", "clipboard": ";", "emojis": ":",
      "math": "=", "shellCommand": "!", "webSearch": "?"
    }
  }
}
```

<details>
<summary><strong>File Structure</strong></summary>

```
~/.config/quickshell/hamr/
├── shell.qml                    # Entry point
├── GlobalStates.qml             # UI state
├── actions/                     # Built-in plugins
├── modules/
│   ├── common/                  # Appearance, Config, widgets
│   ├── launcher/                # Launcher UI components
│   └── imageBrowser/            # Image browser UI
└── services/                    # LauncherSearch, WorkflowRunner, etc.
```

</details>

## License

MIT License. Based on [end-4/dots-hyprland](https://github.com/end-4/dots-hyprland).
