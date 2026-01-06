# Hamr

> "When all you have is a hammer, everything looks like a nail"

<div align="center">

```bash
paru -S hamr
```

[![AUR version](https://img.shields.io/aur/version/hamr)](https://aur.archlinux.org/packages/hamr)

</div>

Hamr is an extensible launcher for Wayland compositors built with [Quickshell](https://quickshell.outfoxxed.me/). Extend it with plugins in any language using a simple JSON protocol.

**Supported Compositors:** Hyprland, Niri

![Hamr Main View](assets/screenshots/hamr-main-view.png)

## Philosophy

**Minimalist UI** - Clean, modern, no visual clutter. Just a search bar and results. No sidebars, tabs, or menus unless absolutely necessary.

**Zero Configuration** - Works out of the box with sensible defaults. Settings exist but you should never need to touch them.

**Minimum Interactions** - Every feature optimized for fewest possible keystrokes. Search, Enter, done.

**Learns Your Habits** - Frecency ranking means frequently-used items rise to the top automatically. Type "ff" once to launch Firefox, and "ff" becomes your shortcut forever.

**Keyboard-First** - Full functionality without touching the mouse. Vim-style navigation (Ctrl+J/K), quick action shortcuts (Ctrl+1-6), and muscle-memory-friendly bindings.

**Your Shortcuts, Your Way** - First time, type "move to workspace 3". Next time, just "w3". Hamr learns your patterns and creates personal shortcuts automatically. No configuration, no aliases to maintain.

## Features

- **Frecency-based ranking** - Results sorted by frequency + recency (inspired by [zoxide](https://github.com/ajeetdsouza/zoxide))
- **Learned search affinity** - System learns your search shortcuts (type "q" to find QuickLinks if that's how you found it before)
- **Pattern-matched plugins** - Plugins can auto-trigger on patterns (e.g., math expressions, URLs) without explicit prefixes
- **Fuzzy matching** - Fast, typo-tolerant search powered by [fuzzysort](https://github.com/farzher/fuzzysort), includes desktop entry keywords (e.g., "whatsapp" finds ZapZap)
- **Extensible plugins** - Language-agnostic handlers with simple JSON protocol (Python, Bash, Go, Rust, etc.)
- **History tracking** - Search, plugin actions, and shell command history
- **Smart suggestions** - Context-aware app suggestions based on time, workspace, and usage patterns
- **Preview panel** - Drawer-style side panel shows rich previews (images, markdown, metadata) on hover/selection; pin previews to screen
- **Draggable & persistent position** - Drag the launcher anywhere on screen; position remembered across sessions
- **State restoration** - Click outside to dismiss, reopen within 30s to resume where you left off (configurable)
- **Live plugin updates** - Plugins can emit real-time updates via daemon mode (no full list refresh, preserves focus)
- **File watching** - Plugins can watch files/directories for changes with native inotify

### Prefix Shortcuts

| Prefix | Function | Prefix | Function |
|--------|----------|--------|----------|
| `~` | File search | `;` | Clipboard history |
| `/` | Actions & plugins | `!` | Shell history |
| `=` | Calculator | `:` | Emoji picker |

These shortcuts are fully customizable. See [Customizing Prefix Shortcuts](#customizing-prefix-shortcuts) for details.

### Smart Calculator

Type math expressions directly - no prefix needed. Examples: `2+2`, `sqrt(16)`, `10c` (celsius), `$50 to EUR`, `20% of 32`, `10ft to m`

Powered by [qalculate](https://qalculate.github.io/) - supports 150+ currencies, 100+ units, percentages, and advanced math.

### Built-in Plugins

All plugins are indexed and searchable directly from the main bar - no prefix required. Just type what you want (e.g., "clipboard", "emoji", "power") and Hamr finds it. Prefix shortcuts like `/`, `~`, `;` are optional conveniences, not requirements.

| Plugin | Description |
|--------|-------------|
| `apps` | App drawer with categories (like rofi/dmenu) |
| `bitwarden` | Password manager with keyring integration |
| `calculate` | Calculator with currency, units, and temperature |
| `clipboard` | Clipboard history with OCR search, filter by type |
| `create-plugin` | AI helper to create new plugins (requires [OpenCode](https://opencode.ai)) |
| `dict` | Dictionary lookup with definitions |
| `emoji` | Emoji picker with search |
| `files` | File search with fd + fzf, thumbnails for images |
| `aur` | Search and install packages from AUR (yay/paru) |
| `flathub` | Search and install apps from Flathub |
| `notes` | Quick notes with multi-line content support |
| `pictures` | Browse images with thumbnails |
| `player` | Media player controls via playerctl (play/pause, next, prev, shuffle, loop) |
| `power` | System power and session controls (shutdown, reboot, suspend, logout) |
| `quicklinks` | Web search with customizable quicklinks |
| `screenrecord` | Screen recording with auto-trim (wf-recorder) |
| `screenshot` | Browse screenshots with OCR text search |
| `settings` | Configure Hamr launcher options |
| `sound` | System volume controls (volume up/down, mute, mic mute) |
| `shell` | Shell command history (zsh/bash/fish) |
| `snippet` | Text snippets for quick insertion |
| `theme` | Dark/light mode and accent color switching |
| `timer` | Countdown timers with presets, FAB display, and notifications |
| `todo` | Simple todo list manager (live updates via daemon) |
| `topcpu` | Process monitor sorted by CPU usage (live daemon refresh) |
| `topmem` | Process monitor sorted by memory usage (live daemon refresh) |
| `url` | Open URLs in browser (auto-detects domain patterns) |
| `wallpaper` | Wallpaper selector (swww, hyprpaper, swaybg, feh) |
| `webapp` | Install and manage web apps |
| `whats-that-word` | Find words from descriptions or fix misspellings |
| `zoxide` | Jump to frequently used directories |
| `hyprland` | Window management, dispatchers, and global shortcuts (Hyprland only) |
| `niri` | Window management and compositor actions (Niri only) |

### Hyprland Integration

Forgot which keybinding moves a window to workspace 3? Can't remember the shortcut for toggling floating mode? No problem. Just type what you want in plain English and Hamr handles the rest.

The `hyprland` plugin provides natural language access to Hyprland window management - no need to memorize keybindings or dig through config files.

**Window Management:**
- `toggle floating`, `fullscreen`, `maximize`, `pin`, `center window`
- `close window`, `focus left/right/up/down`
- `move window left/right/up/down`, `swap left/right/up/down`

**Workspace Navigation:**
- `workspace 3`, `go to 5`, `next workspace`, `previous workspace`
- `move to 2`, `move to workspace 4 silent`
- `scratchpad`, `empty workspace`

**Window Groups (Tabs):**
- `create group` - Make current window a group
- `join group left/right` - Add window to adjacent group
- `remove from group`, `next in group`, `prev in group`

**Global Shortcuts:**
Every app that registers DBus global shortcuts becomes instantly searchable. That obscure "Toggle side panel" shortcut from your browser extension? Just type `side panel`. The screen recording hotkey you set up months ago? Type `record`. No need to remember Ctrl+Alt+Shift+whatever - describe what you want and Hamr finds it.

**Monitor Control:**
- `next monitor`, `prev monitor`
- `move workspace to monitor`, `swap workspaces`

Commands are saved to history for quick access. Even better: after using "move to workspace 3" a few times, just type `w3` or `m3` - Hamr learns your shortcuts automatically. Type `/hyprland` to browse all available commands, or search directly from the main bar.

### Niri Integration

The `niri` plugin provides natural language access to Niri window management, optimized for Niri's scrollable tiling layout.

**Window Management:**
- `close window`, `fullscreen`, `toggle floating`, `center column`
- `maximize column`, `toggle tabbed` (column tabbed display)

**Column Operations:**
- `consume window` - Add window to the right into the focused column
- `expel window` - Move focused window out of column
- `expand column` - Expand column to available width
- `focus column left/right`, `move column left/right`

**Window Movement:**
- `focus window up/down`, `move window up/down`
- `swap window left/right`

**Workspace Navigation:**
- `focus workspace up/down`, `focus workspace previous`
- `move window to workspace up/down`
- `move column to workspace up/down`

**Monitor Control:**
- `focus monitor left/right/up/down/next/previous`
- `move window to monitor left/right`
- `move column to monitor left/right`

**Utilities:**
- `toggle overview`, `screenshot`, `screenshot window`, `screenshot screen`
- `show hotkey overlay`, `switch keyboard layout`
- `power off monitors`, `power on monitors`

Type `/niri` to browse all available actions, or search directly from the main bar.

## Installation

**Requirements:** Linux with a supported Wayland compositor (Hyprland or Niri)

### Arch Linux (AUR)

```bash
# Using paru (or yay, etc.)
paru -S hamr
```

<details>
<summary><strong>Manual installation (from source)</strong></summary>

```bash
# Clone the repository
git clone https://github.com/stewart86/hamr.git
cd hamr

# Run the install script (auto-installs dependencies)
./install.sh
```

The install script will:
- Check for missing dependencies and offer to install them via your AUR helper (paru, yay, etc.)
- Create a symlink at `~/.config/quickshell/hamr`
- Set up the user plugins directory at `~/.config/hamr/plugins/`

</details>

<details>
<summary><strong>What gets installed</strong></summary>

**Required dependencies:**
| Category | Packages |
|----------|----------|
| Core | `quickshell` >= 0.2.1 (or `quickshell-git`) |
| Python | `python`, `python-click`, `python-loguru`, `python-tqdm`, `python-gobject`, `gnome-desktop-4` |
| Clipboard | `wl-clipboard`, `cliphist` |
| File search | `fd`, `fzf` |
| Desktop | `xdg-utils`, `libnotify`, `gtk3`, `libpulse`, `jq` |
| Compositor | `hyprland` or `niri` |
| Calculator | `libqalculate` |
| Fonts | `ttf-material-symbols-variable`, `ttf-jetbrains-mono-nerd`, `ttf-readex-pro` |

**Optional dependencies:**
- `matugen` - Material You color generation from wallpapers
- `tesseract` - OCR for screenshot text search
- `imagemagick` - Alternative thumbnail generation
- `bitwarden-cli` - Bitwarden password manager plugin
- `python-keyring` - Secure session storage for Bitwarden plugin
- `slurp` - Screen region selection
- `wf-recorder` - Screen recording

</details>

<details>
<summary><strong>Manual dependency installation</strong></summary>

If you prefer to install dependencies manually:

```bash
# Using paru (or yay, etc.)
paru -S quickshell python python-click python-loguru python-tqdm \
    python-gobject gnome-desktop-4 wl-clipboard cliphist fd fzf \
    xdg-utils libnotify gtk3 libpulse jq libqalculate \
    ttf-material-symbols-variable ttf-jetbrains-mono-nerd ttf-readex-pro

# Plus your compositor (one of):
paru -S hyprland  # or
paru -S niri

# Optional
paru -S matugen-bin tesseract imagemagick bitwarden-cli slurp wf-recorder
```

</details>

### Other Distributions

<details>
<summary><strong>Fedora / Ubuntu / Debian</strong></summary>

Hamr requires [Quickshell](https://quickshell.outfoxxed.me/) which must be built from source on non-Arch distros. See the [Quickshell documentation](https://quickshell.outfoxxed.me/docs/getting-started/installation/) for build instructions.

Once Quickshell is installed, clone Hamr and install dependencies manually:

```bash
git clone https://github.com/stewart86/hamr.git
cd hamr

# Create config directories
mkdir -p ~/.config/quickshell ~/.config/hamr/plugins
ln -s "$(pwd)" ~/.config/quickshell/hamr

# Install Python dependencies
pip install click loguru tqdm PyGObject

# Install system packages (example for Fedora)
sudo dnf install fd-find fzf wl-clipboard jq qalculate

# Install fonts (see font section below)
```

</details>

<details>
<summary><strong>Font Installation (Non-Arch)</strong></summary>

| Font | Purpose |
|------|---------|
| **Material Symbols Rounded** | Icons throughout the UI |
| **JetBrains Mono NF** | Monospace text and Nerd Font icons |
| **Readex Pro** | Reading/content text |

Download from:
- [JetBrains Mono Nerd Font](https://github.com/ryanoasis/nerd-fonts/releases) - Download `JetBrainsMono.zip`
- [Material Symbols](https://github.com/google/material-design-icons/tree/master/variablefont)
- [Readex Pro](https://fonts.google.com/specimen/Readex+Pro)

```bash
mkdir -p ~/.local/share/fonts
# Extract/copy downloaded fonts to ~/.local/share/fonts/
fc-cache -fv
```

</details>

### Post-Installation Setup

Hamr starts hidden and listens for a toggle signal. Configure your compositor to toggle Hamr with a keybinding.

<details open>
<summary><strong>Hyprland</strong></summary>

Add to `~/.config/hypr/hyprland.conf`:

```bash
# Autostart hamr
exec-once = hamr

# Toggle Hamr with Super key (tap to toggle)
bind = SUPER, SUPER_L, global, quickshell:hamrToggle
bindr = SUPER, SUPER_L, global, quickshell:hamrToggleRelease

# Or toggle with Ctrl+Space
bind = Ctrl, Space, global, quickshell:hamrToggle
```

Reload config: `hyprctl reload`

</details>

<details>
<summary><strong>Niri</strong></summary>

**1. Enable systemd service (recommended):**

```bash
systemctl --user enable hamr.service
systemctl --user add-wants niri.service hamr.service
systemctl --user start hamr.service
```

**2. Add keybinding to `~/.config/niri/config.kdl`:**

```kdl
binds {
    // Toggle Hamr with Ctrl+Space
    Ctrl+Space { spawn "hamr" "toggle"; }

    // Or with Mod+Space (Super key)
    Mod+Space { spawn "hamr" "toggle"; }
}
```

**Alternative: Manual autostart (without systemd)**

If you prefer not to use systemd, add to `~/.config/niri/config.kdl`:

```kdl
spawn-at-startup "hamr"
```

</details>

**Start Hamr manually (for testing):**

```bash
hamr
```

After starting, press your keybind (e.g., Ctrl+Space) to open Hamr.

### Direct Plugin Keybindings

You can bind keys to open specific plugins directly:

```bash
hamr plugin <plugin_name>
```

<details open>
<summary><strong>Hyprland</strong></summary>

Add to `~/.config/hypr/hyprland.conf`:

```bash
# Open clipboard directly with Mod+V
bind = SUPER, V, exec, hamr plugin clipboard

# Open emoji picker with Mod+Period
bind = SUPER, Period, exec, hamr plugin emoji

# Open file search with Mod+E
bind = SUPER, E, exec, hamr plugin files
```

</details>

<details>
<summary><strong>Niri</strong></summary>

Add to `~/.config/niri/config.kdl`:

```kdl
binds {
    // Open clipboard directly with Mod+V
    Mod+V { spawn "hamr" "plugin" "clipboard"; }

    // Open emoji picker with Mod+Period
    Mod+Period { spawn "hamr" "plugin" "emoji"; }

    // Open file search with Mod+E
    Mod+E { spawn "hamr" "plugin" "files"; }
}
```

</details>

Available plugins: `apps`, `bitwarden`, `calculate`, `clipboard`, `emoji`, `files`, `notes`, `pictures`, `power`, `quicklinks`, `screenshot`, `settings`, `shell`, `snippet`, `todo`, `wallpaper`, `hyprland`, `niri`, etc.

### Updating

**AUR:**
```bash
paru -Syu hamr
```

**Manual installation:**
```bash
cd /path/to/hamr
./install.sh --update
```

### Uninstalling

**AUR:**
```bash
sudo pacman -R hamr
# Optionally remove user data:
rm -rf ~/.config/hamr
```

**Manual installation:**
```bash
./install.sh --uninstall
```

<details>
<summary><strong>Troubleshooting</strong></summary>

**"I ran `hamr` but nothing appears"**

This is expected. Hamr starts hidden and waits for a toggle signal. Make sure you:
1. Added the keybinding to your compositor config (see Post-Installation Setup above)
2. Reloaded your compositor config
3. Press your keybind (e.g., Super key or Ctrl+Space)

**Check dependencies**

```bash
./install.sh --check
```

**View logs**

```bash
journalctl --user -u quickshell -f
```

**Crash with Qt version mismatch**

```
WARN: Quickshell was built against Qt 6.10.0 but the system has updated to Qt 6.10.1...
```

This happens when Qt is updated but Quickshell wasn't rebuilt. Fix by rebuilding Quickshell:

```bash
paru -S quickshell --rebuild
# or for quickshell-git:
paru -S quickshell-git --rebuild
```

**Warning about missing `colors.json`**

```
WARN: Read of colors.json failed: File does not exist
```

This is harmless. Hamr uses built-in default colors. For dynamic theming from your wallpaper, install [matugen](https://github.com/InioX/matugen) and use the wallpaper plugin.

**Warning about missing `quicklinks.json`**

```
WARN: Read of quicklinks.json failed: File does not exist
```

This is harmless. Quicklinks are optional. To add quicklinks, create `~/.config/hamr/quicklinks.json`:
```json
[
  {"name": "GitHub", "url": "https://github.com", "icon": "code"}
]
```

</details>

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

Hamr uses Material Design colors for its UI. Colors can come from:

1. **matugen (recommended)** - Auto-generates colors from your wallpaper. Install [matugen](https://github.com/InioX/matugen) and set a wallpaper via the wallpaper plugin
2. **Custom colors.json** - Set path in config: `"paths": {"colorsJson": "~/.config/hamr/colors.json"}`
3. **Built-in defaults** - Dark theme fallback when no colors.json found

**Setting up matugen:**

```bash
# Install matugen (Arch)
paru -S matugen-bin

# Set a wallpaper - colors are generated automatically
# Type /wallpaper in Hamr
```

**Creating a custom colors.json:**

The file should contain Material Design 3 color tokens. See [Material Theme Builder](https://material-foundation.github.io/material-theme-builder/) to generate a theme.

</details>

## Creating Plugins

Hamr loads plugins from two locations:
- **Built-in plugins**: `<hamr>/plugins/` - Included with Hamr, read-only
- **User plugins**: `~/.config/hamr/plugins/` - Your custom plugins

User plugins with the same name as built-in plugins will override them.

Each plugin is a **folder** containing:
- `manifest.json` - Plugin metadata and configuration
- `handler.py` (optional) - Handler script for dynamic plugins

For simple actions, use `staticIndex` in the manifest - no handler needed.

**Language agnostic:** Plugins communicate via JSON over stdin/stdout. Use Python, Bash, Go, Rust, Node.js - any language that can read/write JSON.

**Full documentation:** See [`plugins/README.md`](plugins/README.md) for the complete protocol reference, response types, indexing, daemon mode, forms, and more.

### What Plugins Can Do

| Capability | Description |
|------------|-------------|
| **Multi-step navigation** | Show lists, let users drill down, navigate back |
| **Rich cards** | Display markdown content (definitions, previews, help) |
| **Preview panel** | Side panel with image/markdown/text preview, pinnable to screen |
| **Multi-field forms** | Forms with text, textarea, select, checkbox fields |
| **Image thumbnails** | Show image previews in result lists |
| **Action buttons** | Add context actions per item (copy, delete, open folder) |
| **Plugin action bar** | Toolbar buttons for plugin-level actions (Add, Wipe) with Ctrl+1-6 shortcuts |
| **Confirmation dialogs** | Inline confirmation for dangerous actions (e.g., "Wipe All") |
| **Image browser** | Full image browser UI with directory navigation |
| **OCR text search** | Search images by text content (requires tesseract) |
| **Execute commands** | Run any shell command, optionally save to history |
| **Custom placeholders** | Change search bar placeholder text per step |
| **Live search** | Filter results as user types |
| **Submit mode** | Wait for Enter before processing (for text input, chat) |
| **Daemon mode** | Persistent processes with live updates (async file watching, real-time data) |
| **File watching** | Plugins watch files/directories with inotify for instant updates |
| **Incremental updates** | `update` response type patches individual items without full refresh |
| **Visual enhancements** | Badges, chips, gauges, progress bars, graphs for rich result displays |

<details>
<summary><strong>Quick Start: Hello World Plugin</strong></summary>

```bash
mkdir -p ~/.config/hamr/plugins/hello
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
chmod +x ~/.config/hamr/plugins/hello/handler.py
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
  "inputMode": "realtime",
  "pluginActions": [
    {"id": "add", "name": "Add", "icon": "add_circle"},
    {"id": "wipe", "name": "Wipe", "icon": "delete_sweep", "confirm": "Are you sure?"}
  ]
}
```

**Update Individual Items (preserves selection/focus):**
```json
{
  "type": "update",
  "items": [
    {
      "id": "item-1",
      "gauge": {"value": 75, "max": 100},
      "badges": [{"text": "5"}]
    }
  ]
}
```

**Show Card:**
```json
{
  "type": "card",
  "card": {"title": "Title", "content": "Markdown **content**", "markdown": true}
}
```

**Execute Action (Safe API):**
```json
{
  "type": "execute",
  "open": "/path/to/file",
  "name": "Open file.pdf",
  "close": true
}
```

Available safe actions: `launch` (desktop file), `copy` (clipboard), `typeText` (type via ydotool), `openUrl`, `open` (xdg-open), `notify`, `sound`.

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

**Open Grid Browser (for emoji, icon selection, etc.):**
```json
{
  "type": "gridBrowser",
  "gridBrowser": {
    "title": "Select Emoji",
    "items": [
      {"id": "😀", "name": "grinning face", "icon": "😀", "iconType": "text", "keywords": ["happy"]}
    ],
    "columns": 10,
    "cellAspectRatio": 1.0,
    "actions": [{"id": "copy", "name": "Copy", "icon": "content_copy"}]
  }
}
```

Handler receives `{"step": "action", "selected": {"id": "gridBrowser", "itemId": "😀", "action": "copy"}}` on selection.

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

#### Plugin Actions (Toolbar)

Add `pluginActions` to display toolbar buttons below the search bar. Useful for plugin-level operations like "Add", "Wipe All", "Refresh".

```json
"pluginActions": [
  {"id": "add", "name": "Add", "icon": "add_circle"},
  {"id": "wipe", "name": "Wipe All", "icon": "delete_sweep", "confirm": "Wipe all? This cannot be undone."}
]
```

- Keyboard shortcuts: Ctrl+1 through Ctrl+6
- Use `confirm` field for dangerous actions (shows inline confirmation dialog)
- Handler receives `{"step": "action", "selected": {"id": "__plugin__"}, "action": "add"}`

#### Daemon Mode (Live Updates)

For plugins that need real-time updates or file watching, use daemon mode instead of polling:

```json
{
  "name": "My Plugin",
  "daemon": {
    "enabled": true,
    "background": false
  }
}
```

- **`background: false`** - Daemon runs only when plugin is active (e.g., live data displays)
- **`background: true`** - Daemon always runs (e.g., status badges, file watching)

Daemon handlers use an event loop to handle multiple requests and emit updates asynchronously. See [`plugins/README.md`](plugins/README.md#daemon-mode-persistent-processes) for full documentation with examples.

**Benefits over polling:**
- Single persistent process (more efficient)
- Real-time updates via inotify (instant, not scheduled)
- Bidirectional communication (hamr can control daemon behavior)
- Supports both foreground and background modes
- File watching with fallback to mtime polling

**Example daemon response (live update):**
```python
# Emit update to individual items without full list refresh
emit({"type": "update", "items": [{"id": "volume", "gauge": {"value": 75, "max": 100}}]})

# Emit status update for plugin badge in main list
emit({"type": "status", "status": {"badges": [{"text": "5"}]}})

# Emit incremental index update (adds items to search)
emit({"type": "index", "mode": "incremental", "items": [{"id": "new-item", "name": "..."}]})
```

</details>

<details>
<summary><strong>Static Index (No Handler)</strong></summary>

For simple actions that don't need dynamic logic, use `staticIndex` in the manifest:

```json
{
  "name": "Screenshot",
  "description": "Capture screen region",
  "icon": "screenshot",
  "staticIndex": [
    {
      "id": "snip",
      "name": "Screenshot Snip",
      "description": "Capture and copy screen region",
      "icon": "screenshot",
      "keywords": ["screenshot", "capture", "snip"],
      "execute": {
        "command": ["bash", "-c", "grim -g \"$(slurp)\" - | wl-copy"],
        "notify": "Screenshot copied",
        "close": true
      }
    }
  ]
}
```

Items appear directly in main search - no handler script needed.

**Example plugins:** [`accentcolor/`](plugins/accentcolor/), [`theme/`](plugins/theme/), [`snip/`](plugins/snip/)

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

## Smart Suggestions

When you open Hamr with an empty search, you may see suggested apps at the top marked with a sparkle icon. These combine your usage frequency with contextual predictions.

**What triggers suggestions:**

| Signal | Weight | Example |
|--------|--------|---------|
| **App sequences** | High | VS Code suggested after opening Terminal |
| **Session start** | High | Email client suggested right after login |
| **Time of day** | Medium | Slack suggested at 9am if you always open it then |
| **Workspace** | Medium | Browser suggested on workspace 1 |
| **Day of week** | Low | Personal apps suggested on weekends |

**How it works:**

1. Every app launch records context (time, workspace, monitor, previous app)
2. Patterns are detected using [Wilson score intervals](https://en.wikipedia.org/wiki/Binomial_proportion_confidence_interval#Wilson_score_interval) (statistically sound for small samples)
3. Context signals combine with frecency (frequency + recency) into a final score
4. Apps above 25% confidence appear as suggestions (max 2)

**No configuration needed.** Suggestions appear automatically as patterns emerge. Use Hamr normally and it learns your habits within a few days.

## Configuration

Hamr is configured via `~/.config/hamr/config.json`. Use the built-in settings plugin (`/settings`) to browse and modify options - no manual editing needed.

### Customizing Prefix Shortcuts

The action bar shortcuts are fully customizable. Edit `~/.config/hamr/config.json` to change which prefixes appear and which plugins they trigger:

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
- **plugin**: Plugin ID to launch (e.g., `files`, `clipboard`, `emoji`) or `action` for plugin search mode

You can reorder, remove, or add hints. For example, to replace emoji with notes:

```json
{ "prefix": ":", "icon": "note", "label": "Notes", "plugin": "notes" }
```

<details>
<summary><strong>Configuration Reference</strong></summary>

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
| **Appearance** | `backgroundTransparency` | `0.2` | Background transparency (0-1) |
| | `launcherXRatio` | `0.5` | Horizontal position (0=left, 1=right) |
| | `launcherYRatio` | `0.1` | Vertical position (0=top, 1=bottom) |
| | `fontScale` | `1` | Font scaling factor  (0.75=min, 1.5=max) |
| **Sizes** | `searchWidth` | `580` | Search bar width (px) |
| | `maxResultsHeight` | `600` | Max results container height (px) |
| **Paths** | `wallpaperDir` | `""` | Custom wallpaper directory (empty = ~/Pictures/Wallpapers) |
| | `colorsJson` | `""` | Custom colors.json path (empty = ~/.config/hamr/colors.json) |

</details>

<details>
<summary><strong>File Structure</strong></summary>

```
~/.config/quickshell/hamr/       # Symlink to cloned repo
├── shell.qml                    # Entry point
├── GlobalStates.qml             # UI state
├── plugins/                     # Built-in plugins (read-only)
├── modules/
│   ├── common/                  # Appearance, Config, widgets
│   ├── launcher/                # Launcher UI components
│   └── imageBrowser/            # Image browser UI
├── services/                    # LauncherSearch, PluginRunner, etc.
└── scripts/                     # Thumbnail generation, OCR indexing

~/.config/hamr/
├── plugins/                     # User plugins (override built-in)
├── config.json                  # User configuration
├── quicklinks.json              # Custom quicklinks
└── plugin-indexes.json          # Plugin data and frecency (auto-generated)
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

The AI will analyze the Raycast extension, translate the patterns to Hamr's protocol, and create a native Linux plugin. See [`plugins/AGENTS.md`](plugins/AGENTS.md#converting-raycast-extensions) for the full conversion guide.

## Privacy

Hamr is fully local and offline. **No data ever leaves your machine.**

| Data | Location | Purpose |
|------|----------|---------|
| Plugin data | `~/.config/hamr/plugin-indexes.json` | Frecency, search terms, smart suggestions |
| Configuration | `~/.config/hamr/config.json` | User preferences |
| Clipboard history | Via `cliphist` (system) | Clipboard search |

**What's tracked for smart suggestions:**
- App launch counts and timestamps
- Time of day / day of week patterns
- Workspace and monitor associations
- App sequence patterns (which app follows another)

**What's NOT tracked:**
- No network requests, analytics, or telemetry
- No file contents or document text
- No keystrokes or input outside Hamr
- No data shared with plugins (they only receive search queries)

To clear all history: `rm ~/.config/hamr/plugin-indexes.json`

## Credits

Hamr was originally extracted and adapted from [end-4's illogical-impulse](https://github.com/end-4/dots-hyprland). Major thanks to end-4 for the Material Design theming, fuzzy search, widget components, and overall architecture that inspired this project.

Hamr is now **fully standalone** and works out of the box on any supported Wayland compositor (Hyprland, Niri).

## License

This project is licensed under the **GNU General Public License v3.0** (GPL-3.0).

Hamr is a derivative work based on [end-4/dots-hyprland](https://github.com/end-4/dots-hyprland), which is also licensed under GPL-3.0. Major thanks to end-4 for the Material Design theming, fuzzy search, widget components, and overall architecture that made this project possible.

See the [LICENSE](LICENSE) file for the full license text.
