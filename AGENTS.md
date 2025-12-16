# AGENTS.md - Hamr Launcher Development

## Quick Reference for AI Agents

**Testing:**
```bash
journalctl --user -u quickshell -f          # View logs (Quickshell auto-reloads on file change)
```

**Code Style (QML - modules/, services/):**
- Pragmas: `pragma Singleton` and `pragma ComponentBehavior: Bound` for singletons
- Imports: Quickshell/Qt imports first, then `qs.*` project imports
- Properties: Use `readonly property var` for computed, typed (`list<string>`, `int`) when possible
- Naming: `camelCase` for properties/functions, `id: root` for root element

**Code Style (Python - plugins/handler.py):**
- Imports: stdlib first (`json`, `os`, `sys`, `subprocess`, `pathlib`), then third-party
- Types: Use `list[dict]`, `str`, `bool` (Python 3.9+ style, not `List[Dict]`)
- Naming: `snake_case` functions/variables, `UPPER_SNAKE` constants
- Test mode: Check `TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"` for mock data
- Errors: Return `{"type": "error", "message": "..."}` JSON, don't raise exceptions

---

## Project Scope

This is the **hamr** launcher - a standalone search bar / launcher for Quickshell.

## Repository

This repo lives at:
```
~/Projects/Personal/Qml/hamr/
```

Symlinked to `~/.config/quickshell/` for testing.

## Workflow

1. **Develop & Test**: Make changes in this directory
2. **Test**: Reload with `pkill -f 'qs -c hamr' && qs -c hamr`
3. **Commit**: Commit directly to this repo

## Commit History (Our Progress)

### Committed to repo:

**cd4ae2cc** - `feat(launcher): add frecency-based ranking, quicklinks, and intent detection`
- Frecency scoring system inspired by zoxide for ranking search results
- Quicklinks support loaded from `~/.config/hamr/quicklinks.json`
- Intent detection to auto-detect commands, math, URLs, and file searches
- Tiered ranking system with category-based prioritization
- Search history persistence with aging and pruning
- File search using fd + fzf integration
- Tab completion support properties

**b966e2d5** - `feat: add support for custom user action scripts`
- Custom actions by placing executable scripts in `~/.config/hamr/plugins/`
- Script filename becomes the action name
- Use `/script-name` in search bar to execute

### In Development (this directory, not yet committed):

**Multi-Step Workflow System**
- New `services/WorkflowRunner.qml` - Manages bidirectional JSON communication with workflow handlers
- Workflows are folders in `~/.config/hamr/plugins/` containing:
  - `manifest.json` - Workflow metadata (name, description, icon)
  - `handler.py` - Python script using JSON protocol
- New `modules/ii/overview/WorkflowCard.qml` - Card UI for rich content display (markdown support)
- Updated `LauncherSearch.qml` - Workflow integration, startWorkflow(), exitWorkflow()
- Updated `SearchWidget.qml` - Shows WorkflowCard when workflow returns card response
- Updated `SearchItem.qml` - WorkflowResult no longer auto-closes, handler decides via `close: true`
- Updated `Overview.qml` - Escape exits workflow first, click-outside-to-close, hide workspaces when workflow active, listens to executeCommand signal for close
- Updated `LauncherSearchResult.qml` - Added ResultType enum, workflow properties, thumbnail support
- Example workflows: `files/`, `quicklinks/`, `shell/`, `dict/`, `pictures/`
- Updated `SearchItem.qml` - Added comment/description display below item name

**File Search (converted to workflow)**
- Removed built-in file search from `LauncherSearch.qml`
- New `files/` workflow handles file search via fd + fzf
- Typing `~` starts the files workflow
- Shows recent files on initial, fuzzy search on typing
- Actions: Open, Open folder, Copy path, Delete (trash)
- Thumbnails for images

**Shell History Integration**
- New `services/ShellHistory.qml` service
- Auto-detects shell (zsh/bash/fish) from `$SHELL`
- Parses shell-specific history formats
- `!` prefix for exclusive shell history search
- Config options in `Config.qml` under `search.shellHistory`

## Files We Work On

### Primary Files (LauncherSearch)
- `services/LauncherSearch.qml` - Main search logic, result ranking, intent detection
- `services/ShellHistory.qml` - Shell command history service (zsh/bash/fish support)
- `services/WorkflowRunner.qml` - Multi-step workflow execution service

### Launcher UI Files
- `modules/launcher/SearchWidget.qml` - Search results container, shows card or list
- `modules/launcher/SearchItem.qml` - Individual search result item
- `modules/launcher/SearchBar.qml` - Search input field
- `modules/launcher/WorkflowCard.qml` - Rich card display for workflow responses
- `modules/launcher/Launcher.qml` - Main launcher panel

### Supporting Files (may need minor edits)
- `modules/common/Config.qml` - Configuration options (search prefixes, shellHistory settings)
- `modules/common/models/LauncherSearchResult.qml` - Result model with workflow properties
- `shell.qml` - Service initialization

### Reference Files (read-only for understanding)
- `services/Cliphist.qml` - Pattern reference for similar services
- `services/AppSearch.qml` - App search implementation
- `modules/common/Directories.qml` - Path definitions

## Current Features

### Shell History Integration
- **Auto-detection**: Detects shell from `$SHELL` (zsh, bash, fish)
- **History parsing**: Handles shell-specific formats
  - Zsh: Extended format `: TIMESTAMP:DURATION;COMMAND`
  - Bash: Plain text, one command per line
  - Fish: YAML-like `- cmd: COMMAND`
- **Prefix mode**: `!` prefix filters to shell history only
- **Mixed mode**: Shell history appears in tier3 (below recent apps/actions)

### Configuration
```javascript
// In Config.qml under search
property string shellHistory: "!"  // Prefix
property JsonObject shellHistory: JsonObject {
    property bool enable: true
    property string shell: "auto"  // "auto", "zsh", "bash", "fish"
    property string customHistoryPath: ""
    property int maxEntries: 500
}
```

### Ranking Tiers
1. **Tier 1**: Intent-specific (Command execution, Math results, URLs)
2. **Tier 2**: Apps, Actions, Workflows, Quicklinks (with frecency)
3. **Tier 3**: Workflow Executions, Shell History, URL History, Clipboard, Emoji
4. **Tier 4**: Web Search (fallback)

## IPC API

Hamr exposes IPC targets via Quickshell's `IpcHandler`. These can be called from external scripts or other Quickshell configs.

### Available Targets

| Target | Method | Description |
|--------|--------|-------------|
| `hamr` | `toggle()` | Toggle launcher visibility |
| `hamr` | `open()` | Open the launcher |
| `hamr` | `close()` | Close the launcher |
| `hamr` | `openWith(prefix)` | Open with prefix (**TODO**: not yet implemented) |
| `hamr` | `workflow(name)` | Start a specific workflow by name |
| `shellHistoryService` | `update()` | Refresh shell history entries |

### CLI Usage

```bash
# List all available IPC targets
qs -c hamr ipc show

# Toggle launcher
qs -c hamr ipc call hamr toggle

# Start bitwarden workflow directly
qs -c hamr ipc call hamr workflow bitwarden

# Refresh shell history
qs -c hamr ipc call shellHistoryService update
```

### Hotkeys for Workflows

Use the `workflow` IPC method to bind hotkeys directly to your favorite plugins. This opens hamr and immediately starts the specified workflow.

**Hyprland example** (`~/.config/hypr/hyprland.conf`):
```bash
# Open bitwarden password manager with Super+P
bind = SUPER, P, exec, qs -c hamr ipc call hamr workflow bitwarden

# Open clipboard history with Super+V
bind = SUPER, V, exec, qs -c hamr ipc call hamr workflow clipboard

# Open file browser with Super+E
bind = SUPER, E, exec, qs -c hamr ipc call hamr workflow files

# Open screenshot tool with Super+Shift+S
bind = SUPER SHIFT, S, exec, qs -c hamr ipc call hamr workflow screenshot
```

**Other compositors/WMs**: Use your compositor's keybind config to execute the same `qs -c hamr ipc call hamr workflow <name>` command.

To find available workflow names, check `~/.config/hamr/plugins/` - each folder name is a workflow ID.

### Cross-Config IPC

Handlers can call IPC on other Quickshell configs. For example, the todo plugin refreshes end-4's sidebar:

```bash
# Refresh todo sidebar in ii config (end-4 shell)
qs -c ii ipc call todo refresh
```

See [`plugins/AGENTS.md`](plugins/AGENTS.md) for Python helper functions.

## Testing Commands

```bash
# Quickshell auto-reloads on file change when running in debug mode
# No manual reload needed during development

# View quickshell logs
journalctl --user -u quickshell -f

# Check shell history parsing
cat ~/.zsh_history | sed 's/^: [0-9]*:[0-9]*;//' | tac | awk '!seen[$0]++' | head -20
```

## Code Patterns

### Adding a new search category
1. Add intent type in `LauncherSearch.qml`: `readonly property var intent`
2. Add category in: `readonly property var category`
3. Update `detectIntent()` for prefix detection
4. Update `getTierConfig()` for ranking placement
5. Add exclusive mode handler (if using prefix)
6. Add results to categorized results section

### Service pattern (like ShellHistory)
```qml
pragma Singleton
pragma ComponentBehavior: Bound

Singleton {
    property list<string> entries: []
    readonly property var preparedEntries: entries.map(item => ({
        name: Fuzzy.prepare(item),
        originalItem: item
    }))
    
    function fuzzyQuery(search: string): var {
        if (search.trim() === "") return entries.slice(0, 50);
        return Fuzzy.go(search, preparedEntries, {
            all: true, key: "name", limit: 50
        }).map(r => r.obj.originalItem);
    }
}
```

## Workflow System

### Architecture
- **Handler is in full control** - decides what to show next, when to close
- **UI is dumb** - renders what handler returns, forwards clicks back to handler
- **Protocol is simple** - `results`/`card` = stay open, `execute` with `close: true` = done

### JSON Protocol

**Input to handler (stdin):**
```json
{"step": "initial|search|action|form|poll", "query": "...", "selected": {"id": "..."}, "action": "...", "context": "...", "formData": {...}, "session": "..."}
```

- `step: "poll"`: Periodic refresh request (only sent if `poll` is set in manifest or response)

- `context`: Custom context string set by handler via response (persists across search calls)
- `formData`: Object containing field values when `step: "form"` (form submission)

**Output from handler (stdout):**
```json
// Show results (multi-turn: stays open)
// inputMode: "realtime" (default) = search on every keystroke
//            "submit" = search only when user presses Enter (for text input, AI chat)
// Optional: placeholder = custom search bar placeholder, clearInput = clear search text
// Optional: context = set workflow context for subsequent search calls (useful for multi-step flows like edit/search modes)
{"type": "results", "results": [...], "inputMode": "realtime", "placeholder": "Search...", "clearInput": true, "context": "__edit__:itemId"}

// Show card (stays open)
// inputMode works the same way for cards - controls when next search is triggered
{"type": "card", "card": {"title": "...", "content": "...", "markdown": true}, "inputMode": "submit", "placeholder": "Type reply..."}

// Execute command (close: true = close overview)
{"type": "execute", "execute": {"command": ["cmd", "arg"], "notify": "message", "close": true}}

// Execute with history tracking - Simple (direct command replay)
{"type": "execute", "execute": {"command": ["cmd", "arg"], "name": "Action Name", "icon": "icon", "thumbnail": "/path", "close": true}}

// Execute with history tracking - Complex (workflow replay via entryPoint)
{"type": "execute", "execute": {"name": "Action Name", "entryPoint": {"step": "action", "selected": {"id": "item_id"}, "action": "do_something"}, "icon": "icon", "close": true}}

// Open image browser (for image/wallpaper selection)
{"type": "imageBrowser", "imageBrowser": {"directory": "~/Pictures", "title": "Select Image", "enableOcr": false, "actions": [{"id": "action_id", "name": "Action Name", "icon": "icon"}]}}

// Show form (multi-field input)
{"type": "form", "form": {"title": "Form Title", "submitLabel": "Save", "cancelLabel": "Cancel", "fields": [...]}, "context": "form_context"}

// Show prompt
{"type": "prompt", "prompt": {"text": "Enter something..."}}

// Error
{"type": "error", "message": "Error message"}
```

### Result Properties
```python
{
    "id": "unique-id",           # Required: used for selection
    "name": "Display name",      # Required: shown in result
    "description": "Subtext",    # Optional: shown below name
    "icon": "material_icon",     # Optional: material icon name
    "thumbnail": "/path/to/img", # Optional: image thumbnail (takes priority over icon)
    "verb": "Open",              # Optional: action text on hover
    "actions": [                 # Optional: action buttons
        {"id": "action-id", "name": "Action", "icon": "icon_name"}
    ]
}
```

### Input Modes

The `inputMode` field controls when the UI sends search queries to your handler:

| Mode | Behavior | Use Case |
|------|----------|----------|
| `realtime` | Every keystroke triggers `step: "search"` | Fuzzy filtering, file search |
| `submit` | Only Enter key triggers `step: "search"` | Text input, AI chat, adding items |

**Key insight:** Input mode is a property of the *current step*, not the workflow. The same workflow can use different modes for different steps:

```python
# Fuzzy search mode - realtime filtering
if step == "initial":
    print(json.dumps({
        "type": "results",
        "results": get_items(),
        "inputMode": "realtime",  # Filter on every keystroke
        "placeholder": "Search items..."
    }))

# Add item mode - submit on Enter
if selected_id == "__add__":
    print(json.dumps({
        "type": "results",
        "results": [],
        "inputMode": "submit",  # Only send on Enter
        "placeholder": "Type new item... (Enter to add)"
    }))

# AI chat mode - submit on Enter, show card response
if step == "search" and context == "chat":
    response = call_ai(query)
    print(json.dumps({
        "type": "card",
        "card": {"title": "AI", "content": response, "markdown": True},
        "inputMode": "submit",  # Wait for Enter before sending reply
        "placeholder": "Type reply... (Enter to send)",
        "clearInput": True
    }))
```

**Visual indication:** Use placeholder text to hint at the mode:
- Realtime: "Search files..." 
- Submit: "Type your message... (Enter to send)"

**Key insight for submit mode:** When user presses Enter, execute the action directly in the `step: "search"` handler - don't return results that require another Enter press. Single Enter = action executed.

### Polling (Auto-Refresh)

For plugins that need periodic updates (e.g., process monitors, system stats), use the polling API:

**1. Set poll interval in manifest.json:**
```json
{
  "name": "Top CPU",
  "description": "Processes sorted by CPU usage",
  "icon": "speed",
  "poll": 2000
}
```

**2. Handle the `poll` step in your handler:**
```python
# Poll: refresh with current query (called periodically by PluginRunner)
if step == "poll":
    processes = get_processes()
    print(json.dumps({
        "type": "results",
        "results": get_process_results(processes, query),
    }))
    return
```

**Polling behavior:**
- Timer only runs when plugin is active and not busy (waiting for response)
- `step: "poll"` is sent with the last `query` for filtering context
- Handler should return same format as `search` step
- Can be disabled dynamically via response: `"pollInterval": 0`

**Dynamic poll interval:** Override from response to enable/disable polling:
```python
# Start polling (e.g., after entering monitoring mode)
print(json.dumps({
    "type": "results",
    "results": [...],
    "pollInterval": 1000  # Enable 1s polling
}))

# Stop polling (e.g., when showing detail view)
print(json.dumps({
    "type": "results",
    "results": [...],
    "pollInterval": 0  # Disable polling
}))
```

### Multi-Turn Flow
1. User clicks item → `selectItem(id, action)` → handler receives `step: "action"`
2. Handler can respond with:
   - New `results` → UI shows new list (navigation, drill-down)
   - `card` → UI shows rich content
   - `execute` with `close: false` → run command, stay open
   - `execute` with `close: true` → run command, close overview

### Example: Multi-Turn Handler
```python
def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    selected = input_data.get("selected", {})
    
    if step == "initial":
        # Show initial list
        print(json.dumps({"type": "results", "results": [...]}))
        return
    
    if step == "action":
        item_id = selected.get("id", "")
        
        # Back button - return to list
        if item_id == "__back__":
            print(json.dumps({"type": "results", "results": [...]}))
            return
        
        # Final action - close
        if item_id == "do-something":
            print(json.dumps({
                "type": "execute",
                "execute": {"command": ["cmd"], "close": True}
            }))
            return
        
        # Navigate to detail view (multi-turn)
        print(json.dumps({"type": "results", "results": [
            {"id": "__back__", "name": "Back", "icon": "arrow_back"},
            {"id": "do-something", "name": "Do Something", "icon": "play_arrow"},
        ]}))
```

### Creating a New Workflow
1. Create folder: `~/.config/hamr/plugins/myworkflow/`
2. Create `manifest.json`:
   ```json
   {"name": "My Workflow", "description": "Does something", "icon": "extension"}
   ```
3. Create `handler.py` (must be executable):
   ```python
   #!/usr/bin/env python3
   import json, sys
   
   input_data = json.load(sys.stdin)
   # Handle steps...
   print(json.dumps({"type": "results", "results": [...]}))
   ```
4. Reload quickshell to detect new workflow folder

### Match Patterns (Auto-Trigger)

Plugins can define regex patterns in `manifest.json` that auto-trigger the plugin when the user's query matches. This allows plugins to intercept specific input patterns without requiring a prefix.

**Manifest format:**
```json
{
  "name": "Calculate",
  "description": "Calculator with math, currency, units",
  "icon": "calculate",
  "match": {
    "patterns": [
      "^=",                                    // Explicit = prefix
      "^[\\d\\.]+\\s*[\\+\\-\\*\\/\\^\\%]",   // Number + operator (5+, 3*)
      "^\\(",                                  // Opening parenthesis
      "^(sin|cos|sqrt)\\s*\\(",               // Math functions
      "^[$\u20ac\u00a3]",                            // Currency symbols
      "^\\d+\\.?\\d*\\s*(USD|EUR|GBP)"        // Amount + currency code
    ],
    "priority": 100
  }
}
```

**How it works:**
1. Patterns are compiled to RegExp on plugin load
2. When user types (min 2 chars), patterns are checked
3. Highest priority matching plugin is auto-started
4. Plugin receives `step: "search"` with the full query

**Pattern rules:**
- Case-insensitive matching (`i` flag)
- Must escape special regex chars in JSON (`\\d` not `\d`)
- Patterns are tested against full query string
- First pattern to match wins within a plugin

**Priority:**
- Higher `priority` value = checked first
- Default priority is 0
- If multiple plugins match with same priority, first match wins

**Example: Calculator patterns**

| Pattern | Matches | Purpose |
|---------|---------|---------|
| `^=` | `=5+3` | Explicit calculator prefix |
| `^[\d\.]+\s*[\+\-\*\/]` | `5+`, `3.14*` | Number + operator |
| `^\(` | `(5+3)` | Expression in parens |
| `^[$]` | `$50` | USD currency |
| `^\d+\s*c$` | `10c` | Temperature (Celsius) |

**Testing patterns:**
```bash
# Test if pattern matches
python3 -c "import re; print(re.match(r'^[\d\.]+\s*[\+\-\*\/]', '5+3', re.I))"

# Test plugin with matched query
echo '{"step": "search", "query": "5+3"}' | python3 plugins/calculate/handler.py
```

### Workflow Execution History

When a workflow action includes `name` in the execute response, it's saved to search history and becomes fuzzy-searchable.

#### Hybrid Replay System

The history system supports two replay strategies:

| Strategy | Field | Behavior | Use Case |
|----------|-------|----------|----------|
| **Simple** | `command` | Direct shell execution | File open, clipboard copy, simple commands |
| **Complex** | `entryPoint` | Re-invokes workflow handler | Actions requiring handler logic, API calls, state |

**Replay priority:** `command` (if non-empty) > `entryPoint` (if provided)

#### Simple Replay (Direct Command)

For actions that can be replayed with a simple shell command:

```python
print(json.dumps({
    "type": "execute",
    "execute": {
        "command": ["xdg-open", "/path/to/file.png"],  # Stored for direct replay
        "name": "Open file.png",        # Required for history
        "icon": "image",                 # Optional
        "thumbnail": "/path/to/file.png", # Optional
        "close": True
    }
}))
```

**On replay:** Executes `["xdg-open", "/path/to/file.png"]` directly via shell.

#### Complex Replay (via entryPoint)

For actions that need workflow handler logic (API calls, dynamic data, etc.):

```python
print(json.dumps({
    "type": "execute",
    "execute": {
        "name": "Copy password for GitHub",
        "entryPoint": {                  # Stored for workflow replay
            "step": "action",
            "selected": {"id": "item_abc123"},
            "action": "copy_password"
        },
        "icon": "key",
        "close": True
        # No "command" field - forces entryPoint replay
    }
}))
```

**On replay:** 
1. Starts the workflow
2. Sends the stored `entryPoint` as input to handler
3. Handler receives: `{"step": "action", "selected": {"id": "item_abc123"}, "action": "copy_password", "replay": true, "session": "..."}`
4. Handler processes and returns response (execute, results, etc.)

#### entryPoint Structure

```python
{
    "step": "action",           # Required: step type to send
    "selected": {"id": "..."},  # Optional: selected item context
    "action": "...",            # Optional: action ID
    "query": "..."              # Optional: search query (for step: "search")
}
```

The `replay: true` flag is added automatically to help handlers distinguish replay from normal flow.

#### Example: Bitwarden Password Manager

```python
def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")
    is_replay = input_data.get("replay", False)
    
    # Handle copy password action
    if step == "action" and action == "copy_password":
        item_id = selected.get("id", "")
        
        # Fetch password from Bitwarden API (can't store in command!)
        password = bw_get_password(item_id)
        item_name = bw_get_item_name(item_id)
        
        # Copy to clipboard
        subprocess.run(["wl-copy", password])
        
        print(json.dumps({
            "type": "execute",
            "execute": {
                "name": f"Copy password for {item_name}",
                "entryPoint": {  # For replay - re-fetches password
                    "step": "action",
                    "selected": {"id": item_id},
                    "action": "copy_password"
                },
                "icon": "key",
                "notify": "Password copied",
                "close": True
                # No command - password shouldn't be stored in history!
            }
        }))
```

#### Search History JSON Structure

```json
{
  "history": [
    {
      "type": "workflowExecution",
      "key": "bitwarden:Copy password for GitHub",
      "name": "Copy password for GitHub",
      "workflowId": "bitwarden",
      "workflowName": "Bitwarden",
      "command": [],
      "entryPoint": {
        "step": "action",
        "selected": {"id": "item_abc123"},
        "action": "copy_password"
      },
      "icon": "key",
      "thumbnail": "",
      "count": 5,
      "lastUsed": 1765514312774
    }
  ]
}
```

#### When to Use Each Strategy

| Use Simple (`command`) | Use Complex (`entryPoint`) |
|------------------------|---------------------------|
| Opening files | API calls (passwords, tokens) |
| Copying static text | Dynamic data fetching |
| Running shell commands | Actions with side effects |
| Setting wallpapers | Multi-step confirmations |
| Any idempotent action | State-dependent actions |

#### Best Practices

1. **Prefer `command` when possible** - Direct execution is faster and works offline
2. **Use `entryPoint` for sensitive data** - Never store passwords/tokens in command
3. **Always provide `name`** - Required for history tracking
4. **Include `icon`/`thumbnail`** - Better visual recognition in search results
5. **Handle `replay: true`** - Skip confirmations, go straight to action

## Built-in Workflows

> **Full plugin API documentation:** See [`plugins/AGENTS.md`](plugins/AGENTS.md) for complete JSON protocol reference and examples.

| Plugin | Trigger | Key Patterns Demonstrated |
|--------|---------|---------------------------|
| `files/` | `~` | Results with thumbnails, action buttons, fd+fzf integration |
| `clipboard/` | `;` | Image thumbnails, OCR search, filter by type |
| `shell/` | `!` | Simple results, execute commands |
| `bitwarden/` | `/bitwarden` | entryPoint replay, cache, error cards |
| `quicklinks/` | `/quicklinks` | Submit mode, context persistence, CRUD |
| `dict/` | `/dict` | Card response, API fetch |
| `notes/` | `/notes` | Form API for multi-field input (title + content) |
| `pictures/` | `/pictures` | Thumbnails, multi-turn navigation |
| `screenshot/` | `/screenshot` | imageBrowser with `enableOcr: true` |
| `snippet/` | `/snippet` | Submit mode for text input |
| `todo/` | `/todo` | Submit mode, IPC refresh, CRUD |
| `wallpaper/` | `/wallpaper` | imageBrowser, history tracking |
| `topcpu/` | `/topcpu` | Polling API, process management |
| `topmem/` | `/topmem` | Polling API, process management |

## Index Search Isolation

Search within a specific plugin's index using the `pluginId:query` prefix pattern. This allows focused search without starting the full plugin UI.

**Usage:**
```
emoji:smile      # Search only emoji index for "smile"
apps:fire        # Search only apps index for "fire"  
clipboard:code   # Search only clipboard index for "code"
shell:git        # Search only shell history for "git"
quicklinks:gh    # Search only quicklinks for "gh"
```

**Available prefixes** (plugins with index support):
- `apps:` - Installed applications
- `emoji:` - Emoji picker (1800+ emojis)
- `clipboard:` - Recent clipboard entries
- `shell:` - Shell command history
- `quicklinks:` - User quicklinks

**How it works:**
1. Type `pluginId:` to enter isolation mode
2. Everything after the colon is the search query
3. Results show only items from that plugin's index
4. Press Backspace to exit isolation mode (removes the prefix)

**Technical details:**
- Uses `parseIndexIsolationPrefix()` in LauncherSearch.qml
- Calls `PluginRunner.getIndexedItemsForPlugin(pluginId)`
- Only plugins with `index.enabled: true` in manifest support isolation

## Image Browser Response Type

The `imageBrowser` response opens a rich image browser UI with thumbnails, directory navigation, and custom actions. When user selects an image, the selection is sent back to the handler.

### Opening Image Browser

```python
print(json.dumps({
    "type": "imageBrowser",
    "imageBrowser": {
        "directory": "~/Pictures/Wallpapers",  # Initial directory (~ expanded)
        "title": "Select Wallpaper",           # Title shown in sidebar
        "enableOcr": False,                    # Optional: enable OCR text search (requires tesseract)
        "actions": [                           # Custom action buttons in toolbar
            {"id": "set_dark", "name": "Set (Dark Mode)", "icon": "dark_mode"},
            {"id": "set_light", "name": "Set (Light Mode)", "icon": "light_mode"},
        ]
    }
}))
```

**Image Browser Options:**
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `directory` | string | required | Initial directory path (`~` expanded) |
| `title` | string | `""` | Title shown in the sidebar |
| `enableOcr` | bool | `false` | Enable background OCR indexing for text search (requires tesseract) |
| `actions` | array | `[]` | Custom action buttons shown in toolbar |

### Receiving Selection

When user clicks an image (or clicks an action button), handler receives:

```python
{
    "step": "action",
    "selected": {
        "id": "imageBrowser",           # Always "imageBrowser" for this response type
        "path": "/full/path/to/image.jpg",  # Selected image path
        "action": "set_dark"            # ID of clicked action (first action if image clicked)
    },
    "session": "..."
}
```

### Example: Wallpaper Workflow

```python
#!/usr/bin/env python3
import json
import sys
from pathlib import Path

def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    selected = input_data.get("selected", {})

    # Initial or search: open image browser
    if step in ("initial", "search"):
        print(json.dumps({
            "type": "imageBrowser",
            "imageBrowser": {
                "directory": str(Path.home() / "Pictures" / "Wallpapers"),
                "title": "Select Wallpaper",
                "actions": [
                    {"id": "set_dark", "name": "Set (Dark Mode)", "icon": "dark_mode"},
                    {"id": "set_light", "name": "Set (Light Mode)", "icon": "light_mode"},
                ]
            }
        }))
        return

    # Handle image browser selection
    if step == "action" and selected.get("id") == "imageBrowser":
        file_path = selected.get("path", "")
        action_id = selected.get("action", "set_dark")
        mode = "dark" if action_id == "set_dark" else "light"
        
        print(json.dumps({
            "type": "execute",
            "execute": {
                "command": ["switchwall.sh", "--image", file_path, "--mode", mode],
                "name": f"Set wallpaper: {Path(file_path).name}",
                "icon": "wallpaper",
                "thumbnail": file_path,
                "close": True
            }
        }))

if __name__ == "__main__":
    main()
```

## Form Response Type

The `form` response displays a multi-field input form, replacing the sequential single-line input workflow. This is ideal for creating/editing items with multiple fields (notes, settings, etc.).

### Showing a Form

```python
print(json.dumps({
    "type": "form",
    "form": {
        "title": "Add New Note",
        "submitLabel": "Save",      # Optional, defaults to "Submit"
        "cancelLabel": "Cancel",    # Optional, defaults to "Cancel"
        "fields": [
            {
                "id": "title",
                "type": "text",
                "label": "Title",
                "placeholder": "Enter title...",
                "required": True,
                "default": ""
            },
            {
                "id": "content",
                "type": "textarea",
                "label": "Content",
                "placeholder": "Enter content...",
                "rows": 6,           # Optional, textarea height
                "default": ""
            }
        ]
    },
    "context": "__add__"  # Optional: helps handler identify form purpose
}))
```

### Field Types

| Type | Description | Properties |
|------|-------------|------------|
| `text` | Single-line text input | `placeholder`, `required`, `default`, `hint` |
| `textarea` | Multi-line text input | `placeholder`, `rows`, `required`, `default`, `hint` |
| `select` | Dropdown selection | `options: [{id, name}]`, `required`, `default` |
| `checkbox` | Boolean toggle | `label`, `default` |
| `password` | Hidden text input | `placeholder`, `required`, `default` |

### Field Properties

```python
{
    "id": "field_id",          # Required: unique identifier for this field
    "type": "text",            # Required: field type
    "label": "Field Label",    # Optional: shown above field
    "placeholder": "...",      # Optional: placeholder text
    "required": True,          # Optional: validation (default: false)
    "default": "...",          # Optional: initial value
    "hint": "Help text",       # Optional: shown below field
    "rows": 6,                 # Optional: textarea height (default: 4)
    "options": [               # Required for select type
        {"id": "opt1", "name": "Option 1"},
        {"id": "opt2", "name": "Option 2"}
    ]
}
```

### Receiving Form Submission

When user submits the form, handler receives:

```python
{
    "step": "form",
    "formData": {
        "title": "My Note Title",
        "content": "Line 1\nLine 2\nLine 3",
        "tags": "work, ideas"
    },
    "context": "__add__",
    "session": "..."
}
```

### Handling Form Cancel

When user clicks Cancel, handler receives:

```python
{
    "step": "action",
    "selected": {"id": "__form_cancel__"},
    "context": "__add__",
    "session": "..."
}
```

### Example: Notes Plugin with Form

```python
def show_add_form(title_default=""):
    """Show form for adding a new note"""
    print(json.dumps({
        "type": "form",
        "form": {
            "title": "Add New Note",
            "submitLabel": "Save",
            "fields": [
                {
                    "id": "title",
                    "type": "text",
                    "label": "Title",
                    "placeholder": "Enter note title...",
                    "required": True,
                    "default": title_default,
                },
                {
                    "id": "content",
                    "type": "textarea",
                    "label": "Content",
                    "placeholder": "Enter note content...\n\nSupports multiple lines.",
                    "rows": 6,
                },
            ],
        },
        "context": "__add__",
    }))

def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    context = input_data.get("context", "")
    form_data = input_data.get("formData", {})

    # Handle form submission
    if step == "form" and context == "__add__":
        title = form_data.get("title", "").strip()
        content = form_data.get("content", "")
        
        if title:
            save_note(title, content)
            print(json.dumps({
                "type": "results",
                "results": get_notes(),
                "clearInput": True,
                "context": "",
            }))
        else:
            print(json.dumps({"type": "error", "message": "Title is required"}))
        return

    # Handle form cancel
    if step == "action" and selected.get("id") == "__form_cancel__":
        print(json.dumps({
            "type": "results",
            "results": get_notes(),
            "clearInput": True,
        }))
        return
```

### Keyboard Shortcuts

- **Enter** in text fields: Move to next field
- **Ctrl+Enter** in textarea: Submit form
- **Tab**: Move between fields
- **Escape**: Cancel form (triggers `__form_cancel__`)

### When to Use Forms vs Submit Mode

| Use Form | Use Submit Mode |
|----------|-----------------|
| Multiple fields (title + content) | Single text input |
| Structured data (settings, config) | Free-form search/add |
| Edit existing items | Quick add by name |
| Complex input with validation | Simple confirmation |
