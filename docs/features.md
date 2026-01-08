# Features

## Core Features

- **Frecency-based ranking** - Results sorted by frequency + recency (inspired by [zoxide](https://github.com/ajeetdsouza/zoxide))
- **Learned search affinity** - System learns your search shortcuts (type "q" to find QuickLinks if that's how you found it before)
- **Pattern-matched plugins** - Plugins can auto-trigger on patterns (e.g., math expressions, URLs) without explicit prefixes
- **Fuzzy matching** - Fast, typo-tolerant search powered by [fuzzysort](https://github.com/farzher/fuzzysort), includes desktop entry keywords
- **Extensible plugins** - Language-agnostic handlers with simple JSON protocol (Python, Bash, Go, Rust, etc.)
- **History tracking** - Search, plugin actions, and shell command history
- **Smart suggestions** - Context-aware app suggestions based on time, workspace, and usage patterns
- **Preview panel** - Drawer-style side panel shows rich previews (images, markdown, metadata) on hover/selection; pin previews to screen
- **Draggable & persistent position** - Drag the launcher anywhere on screen; position remembered across sessions
- **State restoration** - Click outside to dismiss, reopen within 30s to resume where you left off (configurable)
- **Live plugin updates** - Plugins can emit real-time updates via daemon mode (no full list refresh, preserves focus)
- **File watching** - Plugins can watch files/directories for changes with native inotify

## Prefix Shortcuts

| Prefix | Function          | Prefix | Function          |
| ------ | ----------------- | ------ | ----------------- |
| `~`    | File search       | `;`    | Clipboard history |
| `/`    | Actions & plugins | `!`    | Shell history     |
| `=`    | Calculator        | `:`    | Emoji picker      |

These shortcuts are fully customizable. See [Configuration](getting-started/configuration.md#prefix-shortcuts) for details.

## Smart Calculator

Type math expressions directly - no prefix needed. Examples: `2+2`, `sqrt(16)`, `10c` (celsius), `$50 to EUR`, `20% of 32`, `10ft to m`

Powered by [qalculate](https://qalculate.github.io/) - supports 150+ currencies, 100+ units, percentages, and advanced math.

---

## Built-in Plugins

All plugins are indexed and searchable directly from the main bar - no prefix required. Just type what you want (e.g., "clipboard", "emoji", "power") and Hamr finds it.

| Plugin            | Description                                                                |
| ----------------- | -------------------------------------------------------------------------- |
| `apps`            | App drawer with categories (like rofi/dmenu)                               |
| `bitwarden`       | Password manager with keyring integration                                  |
| `calculate`       | Calculator with currency, units, and temperature                           |
| `clipboard`       | Clipboard history with OCR search, filter by type                          |
| `create-plugin`   | AI helper to create new plugins (requires [OpenCode](https://opencode.ai)) |
| `dictionary`      | Dictionary lookup with definitions                                         |
| `emoji`           | Emoji picker with search                                                   |
| `files`           | File search with fd + fzf, thumbnails for images                           |
| `aur`             | Search and install packages from AUR (yay/paru)                            |
| `flathub`         | Search and install apps from Flathub                                       |
| `notes`           | Quick notes with multi-line content support                                |
| `pictures`        | Browse images with thumbnails                                              |
| `player`          | Media player controls via playerctl                                        |
| `power`           | System power and session controls                                          |
| `quicklinks`      | Web search with customizable quicklinks                                    |
| `screenrecord`    | Screen recording with auto-trim (wf-recorder)                              |
| `screenshot`      | Browse screenshots with OCR text search                                    |
| `settings`        | Configure Hamr launcher options                                            |
| `sound`           | System volume controls                                                     |
| `shell`           | Shell command history (zsh/bash/fish)                                      |
| `snippet`         | Text snippets for quick insertion                                          |
| `theme`           | Dark/light mode and accent color switching                                 |
| `timer`           | Countdown timers with presets, FAB display, and notifications              |
| `todo`            | Simple todo list manager (live updates via daemon)                         |
| `topcpu`          | Process monitor sorted by CPU usage (live daemon refresh)                  |
| `topmem`          | Process monitor sorted by memory usage (live daemon refresh)               |
| `url`             | Open URLs in browser (auto-detects domain patterns)                        |
| `wallpaper`       | Wallpaper selector (swww, hyprpaper, swaybg, feh)                          |
| `webapp`          | Install and manage web apps                                                |
| `whats-that-word` | Find words from descriptions or fix misspellings                           |
| `zoxide`          | Jump to frequently used directories                                        |
| `hyprland`        | Window management, dispatchers, and global shortcuts (Hyprland only)       |
| `niri`            | Window management and compositor actions (Niri only)                       |

---

## Compositor Integration

### Hyprland

The `hyprland` plugin provides natural language access to Hyprland window management - no need to memorize keybindings.

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
Every app that registers DBus global shortcuts becomes instantly searchable. That obscure "Toggle side panel" shortcut from your browser extension? Just type `side panel`.

Type `/hyprland` to browse all available commands.

### Niri

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

Type `/niri` to browse all available actions.

---

## Smart Search

Hamr learns your search habits and creates automatic shortcuts. No configuration needed.

**How it works:**

1. Type "ff", scroll to "Firefox", press Enter
2. Next time you type "ff", Firefox appears at the top
3. The system remembers the last 5 search terms for each item

**Ranking algorithm:**

1. **Learned shortcuts first** - Items where you've used that exact search term before rank highest
2. **Frecency decides ties** - Among learned shortcuts, most frequently/recently used wins
3. **Fuzzy matches last** - Items that match but you haven't searched that way before

---

## Smart Suggestions

When you open Hamr with an empty search, you may see suggested apps at the top marked with a sparkle icon.

**What triggers suggestions:**

| Signal            | Weight | Example                                           |
| ----------------- | ------ | ------------------------------------------------- |
| **App sequences** | High   | VS Code suggested after opening Terminal          |
| **Session start** | High   | Email client suggested right after login          |
| **Time of day**   | Medium | Slack suggested at 9am if you always open it then |
| **Workspace**     | Medium | Browser suggested on workspace 1                  |
| **Day of week**   | Low    | Personal apps suggested on weekends               |

**No configuration needed.** Suggestions appear automatically as patterns emerge.
