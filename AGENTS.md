# AGENTS.md - Hamr Launcher Development

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
- Quicklinks support loaded from `~/.config/illogical-impulse/quicklinks.json`
- Intent detection to auto-detect commands, math, URLs, and file searches
- Tiered ranking system with category-based prioritization
- Search history persistence with aging and pruning
- File search using fd + fzf integration
- Tab completion support properties

**b966e2d5** - `feat: add support for custom user action scripts`
- Custom actions by placing executable scripts in `~/.config/illogical-impulse/actions/`
- Script filename becomes the action name
- Use `/script-name` in search bar to execute

### In Development (this directory, not yet committed):

**Multi-Step Workflow System**
- New `services/WorkflowRunner.qml` - Manages bidirectional JSON communication with workflow handlers
- Workflows are folders in `~/.config/illogical-impulse/actions/` containing:
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

### Overview UI Files
- `modules/ii/overview/SearchWidget.qml` - Search results container, shows card or list
- `modules/ii/overview/SearchItem.qml` - Individual search result item
- `modules/ii/overview/SearchBar.qml` - Search input field
- `modules/ii/overview/WorkflowCard.qml` - Rich card display for workflow responses
- `modules/ii/overview/Overview.qml` - Main overview panel

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
{"step": "initial|search|action", "query": "...", "selected": {"id": "..."}, "action": "...", "session": "..."}
```

**Output from handler (stdout):**
```json
// Show results (multi-turn: stays open)
// Optional: placeholder = custom search bar placeholder, clearInput = clear search text
// Optional: context = set lastSelectedItem for subsequent search calls (useful for edit modes)
{"type": "results", "results": [...], "placeholder": "Search...", "clearInput": true, "context": "__edit__:itemId"}

// Show card (stays open)
{"type": "card", "card": {"title": "...", "content": "...", "markdown": true}}

// Execute command (close: true = close overview)
{"type": "execute", "execute": {"command": ["cmd", "arg"], "notify": "message", "close": true}}

// Execute with history tracking (action becomes fuzzy-searchable)
{"type": "execute", "execute": {"command": ["cmd", "arg"], "name": "Action Name", "icon": "icon", "thumbnail": "/path", "close": true}}

// Open image browser (for image/wallpaper selection)
{"type": "imageBrowser", "imageBrowser": {"directory": "~/Pictures", "title": "Select Image", "actions": [{"id": "action_id", "name": "Action Name", "icon": "icon"}]}}

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
1. Create folder: `~/.config/illogical-impulse/actions/myworkflow/`
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

### Workflow Execution History
When a workflow action includes `name` in the execute response, it's saved to search history:

```python
# This action will be saved and fuzzy-searchable
print(json.dumps({
    "type": "execute",
    "execute": {
        "command": ["xdg-open", "/path/to/file.png"],
        "name": "Open file.png",        # Required for history
        "icon": "image",                 # Optional
        "thumbnail": "/path/to/file.png", # Optional
        "close": True
    }
}))
```

**How it works:**
- Fuzzy search matches against `"WorkflowName ActionName"` (e.g., "Pictures Open file.png")
- Type "pic" or "file" to find previously executed workflow actions
- Clicking re-executes the stored command directly (no workflow steps needed)
- Frecency ranking applies (frequently used actions rank higher)

**When to use:**
- File operations (open, copy, move)
- Frequently repeated actions
- Actions users might want to quickly repeat

**When NOT to use:**
- One-time operations
- AI chat responses
- Actions with side effects that shouldn't be repeated blindly

## Built-in Workflows

### files/ - File Search
Triggered by `~` prefix. Uses fd + fzf for fast fuzzy file search.

```python
# Result format
{
    "id": "/full/path/to/file",
    "name": "filename.txt",
    "description": "~/path/to/folder",  # Shown as subtitle
    "icon": "description",               # File type icon
    "thumbnail": "/path/to/image.png",   # For images
    "actions": [
        {"id": "open_folder", "name": "Open folder", "icon": "folder_open"},
        {"id": "copy_path", "name": "Copy path", "icon": "content_copy"},
        {"id": "delete", "name": "Delete", "icon": "delete"}
    ]
}
```

### quicklinks/ - Web Search Quicklinks
Search the web with predefined quicklinks. Supports add/edit/delete.

Features:
- Browse quicklinks, fuzzy search by name or alias
- Search with `{query}` placeholder in URL
- Add new quicklinks (name + URL)
- Edit existing quicklink URLs
- Delete quicklinks

### shell/ - Shell History
Search and execute commands from shell history.

### pictures/ - Image Browser
Browse images with thumbnails, open/copy/delete actions.

### wallpaper/ - Wallpaper Selector
Uses the `imageBrowser` response type to show a rich image browser UI.
- Opens image browser with dark/light mode actions
- User selects image and action, handler receives selection
- Sets wallpaper via switchwall.sh

### dict/ - Dictionary Lookup
Dictionary lookup returning card with definition.

## Image Browser Response Type

The `imageBrowser` response opens a rich image browser UI with thumbnails, directory navigation, and custom actions. When user selects an image, the selection is sent back to the handler.

### Opening Image Browser

```python
print(json.dumps({
    "type": "imageBrowser",
    "imageBrowser": {
        "directory": "~/Pictures/Wallpapers",  # Initial directory (~ expanded)
        "title": "Select Wallpaper",           # Title shown in sidebar
        "actions": [                           # Custom action buttons in toolbar
            {"id": "set_dark", "name": "Set (Dark Mode)", "icon": "dark_mode"},
            {"id": "set_light", "name": "Set (Light Mode)", "icon": "light_mode"},
        ]
    }
}))
```

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
