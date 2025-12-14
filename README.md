# Hamr

> "When all you have is a hammer, everything looks like a nail"

Hamr is an extensible launcher for Hyprland built with [Quickshell](https://quickshell.outfoxxed.me/). Extend it with plugins written in Python using a simple JSON protocol.

## Screenshots

| Overview | Plugins |
|----------|---------|
| ![Hamr Overview](assets/screenshots/hamr-overview.png) | ![Hamr Plugins](assets/screenshots/hamr-plugins.png) |

## Compatibility

Hamr works standalone but is **best used alongside [end-4's illogical-impulse](https://github.com/end-4/dots-hyprland)** dotfiles. Many built-in plugins (wallpaper switching, theme toggling, etc.) are designed to integrate with illogical-impulse's theming system.

## Credits

Hamr is extracted and adapted from [end-4's illogical-impulse](https://github.com/end-4/dots-hyprland). Major thanks to end-4 for the Material Design theming, fuzzy search, widget components, and overall architecture.

## Features

- **Frecency-based ranking** - Results sorted by frequency + recency (inspired by [zoxide](https://github.com/ajeetdsouza/zoxide))
- **Learned search affinity** - System learns your search shortcuts (type "q" to find QuickLinks if that's how you found it before)
- **Intent detection** - Auto-detects URLs, math expressions, and commands
- **Fuzzy matching** - Fast, typo-tolerant search powered by [fuzzysort](https://github.com/farzher/fuzzysort), includes desktop entry keywords (e.g., "whatsapp" finds ZapZap)
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
| `apps` | `/apps` | App drawer with categories (like rofi/dmenu) |
| `files` | `~` | File search with fd + fzf, thumbnails for images |
| `clipboard` | `;` | Clipboard history with image support |
| `shell` | `!` | Shell command history (zsh/bash/fish) |
| `bitwarden` | `/bitwarden` | Password manager with local caching |
| `quicklinks` | `/quicklinks` | Web search with customizable quicklinks |
| `dict` | `/dict` | Dictionary lookup with definitions |
| `notes` | `/notes` | Quick notes with multi-line content support |
| `pictures` | `/pictures` | Browse images with thumbnails |
| `screenshot` | `/screenshot` | Browse screenshots with OCR text search |
| `snippet` | `/snippet` | Text snippets for quick insertion |
| `todo` | `/todo` | Simple todo list manager |
| `wallpaper` | `/wallpaper` | Wallpaper selector (illogical-impulse) |
| `create-plugin` | `/create-plugin` | AI helper to create new plugins (requires [OpenCode](https://opencode.ai)) |

### Simple Actions (Scripts)

| Action | Description |
|--------|-------------|
| `screenshot-snip` | Take screenshot with grim + satty |
| `dark` | Switch to dark mode (illogical-impulse) |
| `light` | Switch to light mode (illogical-impulse) |
| `accentcolor` | Set accent color (illogical-impulse) |

## Installation

### Prerequisites

- [Hyprland](https://hyprland.org/) (required)
- [Quickshell](https://quickshell.outfoxxed.me/)
- Optional: [illogical-impulse](https://github.com/end-4/dots-hyprland) for full theme integration
- Optional: `fd`, `fzf`, `cliphist`, `wl-clipboard`, `bw` (Bitwarden CLI), `tesseract` (OCR) for respective plugins

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
- A **folder** with `manifest.json` + handler executable (multi-step plugins)
- An **executable script** (simple one-shot actions)

**Language agnostic:** Plugins communicate via JSON over stdin/stdout. Use Python, Bash, Go, Rust, Node.js - any language that can read/write JSON.

### What Plugins Can Do

| Capability | Description |
|------------|-------------|
| **Multi-step navigation** | Show lists, let users drill down, navigate back |
| **Rich cards** | Display markdown content (definitions, previews, help) |
| **Multi-field forms** | Forms with text, textarea, select, checkbox fields |
| **Image thumbnails** | Show image previews in result lists |
| **Action buttons** | Add context actions per item (copy, delete, open folder) |
| **Image browser** | Full image browser UI with directory navigation |
| **OCR text search** | Search images by text content (requires tesseract) |
| **Execute commands** | Run any shell command, optionally save to history |
| **Custom placeholders** | Change search bar placeholder text per step |
| **Live search** | Filter results as user types |
| **Submit mode** | Wait for Enter before processing (for text input, chat) |

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
  "placeholder": "Custom placeholder...",
  "inputMode": "realtime"
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
    "enableOcr": true,
    "actions": [{"id": "set_wallpaper", "name": "Set Wallpaper", "icon": "wallpaper"}]
  }
}
```

Set `enableOcr: true` to enable background OCR indexing for text search within images (requires tesseract).

**Show Form (multi-field input):**
```json
{
  "type": "form",
  "form": {
    "title": "Add Note",
    "submitLabel": "Save",
    "fields": [
      {"id": "title", "type": "text", "label": "Title", "required": true},
      {"id": "content", "type": "textarea", "label": "Content", "rows": 6}
    ]
  },
  "context": "add_note"
}
```

Handler receives `{"step": "form", "formData": {"title": "...", "content": "..."}}` on submit.
Field types: `text`, `textarea`, `select`, `checkbox`, `password`. Keyboard: `Esc` cancel, `Ctrl+Enter` submit.

#### Input Modes

| Mode | Behavior | Use Case |
|------|----------|----------|
| `realtime` | Search on every keystroke (default) | Fuzzy filtering, file search |
| `submit` | Search only on Enter | Text input, AI chat, adding items |

Set `"inputMode": "submit"` in results/card response to wait for Enter.

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

## Smart Search: Learned Shortcuts

Hamr learns your search habits and creates automatic shortcuts. No configuration needed.

**How it works:**
1. Type "ff", scroll to "Firefox", press Enter
2. Next time you type "ff", Firefox appears at the top
3. The system remembers the last 5 search terms for each item

**Ranking algorithm:**
1. **Learned shortcuts first** - Items where you've used that exact search term before rank highest
2. **Frecency decides ties** - Among learned shortcuts, most frequently/recently used wins
3. **Fuzzy matches last** - Items that match but you haven't searched that way before

This means an item you use 10 times with "ff" will beat a command named "ff" that you've never executed.

**What's tracked:**
- Apps, actions, workflows, quicklinks
- URLs, workflow executions (wallpaper changes, file opens, etc.)
- Each item stores up to 5 recent search terms

**Example:**
```
Type "ff" → select Firefox → "ff" recorded
Type "ff" → select Firefox → frecency increased
Type "ff" → Firefox is now first (beats other "ff" matches you never use)

Type "wl" → select "Set wallpaper: sunset.jpg" → "wl" recorded
Type "wl" → wallpaper action appears first
```

Terms naturally age out based on frecency, so your shortcuts stay relevant as habits change.

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

## Why Hamr?

### Comparison with Other Launchers

| | **Hamr** | **Vicinae** | **rofi/wofi** |
|---|---|---|---|
| **Plugin Language** | Any (Python, Bash, Go...) | TypeScript/React | Bash scripts |
| **Plugin Protocol** | Simple JSON stdin/stdout | Raycast-compatible SDK | Custom modes |
| **Linux Native** | Yes, built for Linux | Raycast shim layer | Yes |
| **Rich UI** | Cards, thumbnails, image browser | React components | Text-based |
| **Dependencies** | Quickshell only | Qt + Node.js runtime | Minimal |
| **Learning Curve** | ~10 min to first plugin | TypeScript + React knowledge | Script-based |

### Why Not Raycast Compatibility?

Launchers like [Vicinae](https://github.com/vicinaehq/vicinae) aim for Raycast extension compatibility, but this approach has limitations on Linux:

- **macOS-hardcoded paths** - Many Raycast extensions use paths like `~/Library/...` that don't exist on Linux ([example issue](https://github.com/vicinaehq/vicinae/issues/784))
- **macOS APIs** - Extensions often rely on macOS-specific APIs (AppleScript, Finder, Keychain)
- **Heavy runtime** - Requires Node.js + TypeScript toolchain for plugins

**Hamr takes a different approach:** plugins are native Linux scripts using a simple JSON protocol. Write in any language, use Linux-native tools (fd, fzf, wl-copy), and integrate with your existing dotfiles.

```python
# A complete Hamr plugin - no SDK, no build step
#!/usr/bin/env python3
import json, sys
print(json.dumps({"type": "results", "results": [{"id": "1", "name": "Hello"}]}))
```

This means browser bookmarks on Linux just read from `~/.config/google-chrome/Default/Bookmarks` directly - no compatibility layer needed.

### AI-Powered Raycast Conversion

Want functionality from a Raycast extension? Use the built-in `create-plugin` workflow:

```
/create-plugin
> Replicate this Raycast extension: https://github.com/raycast/extensions/tree/main/extensions/browser-bookmarks
```

The AI will analyze the Raycast extension, translate the patterns to Hamr's protocol, and create a native Linux plugin. See [`actions/AGENTS.md`](actions/AGENTS.md#converting-raycast-extensions) for the full conversion guide.

## License

MIT License. Based on [end-4/dots-hyprland](https://github.com/end-4/dots-hyprland).
