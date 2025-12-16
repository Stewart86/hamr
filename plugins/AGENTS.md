# Plugins & Workflows

This directory contains plugins and workflows for the Hamr launcher.

## Language Agnostic

Plugins communicate via **JSON over stdin/stdout** - use any language you prefer:

| Language    | Use Case                                                    |
| ----------- | ----------------------------------------------------------- |
| **Python**  | Recommended for most plugins - readable, batteries included |
| **Bash**    | Simple scripts, system commands                             |
| **Go/Rust** | Performance-critical plugins, compiled binaries             |
| **Node.js** | Web API integrations, existing npm packages                 |

The handler just needs to be executable and read JSON from stdin, write JSON to stdout.

## Directory Structure

```
~/.config/hamr/plugins/
├── AGENTS.md           # This file
├── simple-script       # Simple action (executable script)
├── workflow-name/      # Multi-step workflow (folder)
│   ├── manifest.json   # Workflow metadata
│   └── handler.py      # Workflow handler (any language)
```

---

## Quick Start

### Simple Action (Script)

```bash
# 1. Create executable script
cat > ~/.config/hamr/plugins/my-action << 'EOF'
#!/bin/bash
notify-send "Hello from my action!"
EOF
chmod +x ~/.config/hamr/plugins/my-action

# 2. Appears as `/my-action` in launcher
```

**Examples:** [`screenshot-snip`](screenshot-snip), [`dark`](dark), [`light`](light), [`accentcolor`](accentcolor)

### Multi-Step Workflow (Folder)

```bash
# 1. Create folder with manifest and handler
mkdir ~/.config/hamr/plugins/hello
cat > ~/.config/hamr/plugins/hello/manifest.json << 'EOF'
{"name": "Hello", "description": "Greeting plugin", "icon": "waving_hand"}
EOF

# 2. Create handler (see template below)
touch ~/.config/hamr/plugins/hello/handler.py
chmod +x ~/.config/hamr/plugins/hello/handler.py
```

---

## JSON Protocol Reference

### Input (stdin)

Your handler receives JSON on stdin with these fields:

```python
{
    "step": "initial|search|action",  # Current step type
    "query": "user typed text",        # Search bar content
    "selected": {"id": "item-id"},     # Selected item (for action step)
    "action": "action-button-id",      # Action button clicked (optional)
    "context": "custom-context",       # Your custom context (persists across searches)
    "session": "unique-session-id",    # Session identifier
    "replay": true                     # True when replaying from history (optional)
}
```

| Field         | When Present     | Description                                               |
| ------------- | ---------------- | --------------------------------------------------------- |
| `step`        | Always           | `initial` on start, `search` on typing, `action` on click |
| `query`       | `search` step    | Current search bar text                                   |
| `selected.id` | `action` step    | ID of clicked item                                        |
| `action`      | `action` step    | ID of action button (if clicked via action button)        |
| `context`     | After you set it | Persists your custom state across `search` calls          |
| `replay`      | History replay   | `true` when action is replayed from search history        |

### Output (stdout)

Respond with **one** JSON object. Choose a response type:

---

## Response Types

### 1. `results` - Show List

Display a list of selectable items.

```python
{
    "type": "results",
    "results": [
        {
            "id": "unique-id",           # Required: used for selection
            "name": "Display Name",      # Required: main text
            "description": "Subtitle",   # Optional: shown below name
            "icon": "material_icon",     # Optional: icon name (see Icon Types below)
            "iconType": "material",      # Optional: "material" (default) or "system"
            "thumbnail": "/path/to/img", # Optional: image (overrides icon)
            "verb": "Open",              # Optional: hover action text
            "actions": [                 # Optional: action buttons
                {"id": "copy", "name": "Copy", "icon": "content_copy"}
            ]
        }
    ],
    "inputMode": "realtime",             # Optional: "realtime" (default) or "submit"
    "placeholder": "Search...",          # Optional: search bar placeholder
    "clearInput": true,                  # Optional: clear search text
    "context": "my-state",               # Optional: persist state for search calls
    "pluginActions": [                   # Optional: plugin-level action bar buttons
        {"id": "add", "name": "Add", "icon": "add_circle", "shortcut": "Ctrl+1"},
        {"id": "wipe", "name": "Wipe All", "icon": "delete_sweep", "confirm": "Are you sure?"}
    ]
}
```

**Example plugins:** [`quicklinks/`](quicklinks/handler.py), [`todo/`](todo/handler.py), [`bitwarden/`](bitwarden/handler.py)

#### Plugin Actions (Toolbar Buttons)

The `pluginActions` field displays action buttons in a toolbar below the search bar. These are for plugin-level actions (e.g., "Add", "Wipe", "Refresh") that apply to the plugin itself, not specific items.

```python
"pluginActions": [
    {
        "id": "add",           # Required: action ID
        "name": "Add Item",    # Required: button label
        "icon": "add_circle",  # Required: material icon
        "shortcut": "Ctrl+1",  # Optional: keyboard shortcut (default: Ctrl+N)
        "confirm": "..."       # Optional: confirmation message (shows dialog before executing)
    }
]
```

| Field      | Type   | Required | Description                                       |
| ---------- | ------ | -------- | ------------------------------------------------- |
| `id`       | string | Yes      | Action ID sent to handler                         |
| `name`     | string | Yes      | Button label text                                 |
| `icon`     | string | Yes      | Material icon name                                |
| `shortcut` | string | No       | Keyboard shortcut (default: Ctrl+1 through Ctrl+6)|
| `confirm`  | string | No       | If set, shows confirmation dialog before executing|

**Receiving plugin action clicks:**

When user clicks a plugin action button (or confirms a dangerous action), handler receives:

```python
{
    "step": "action",
    "selected": {"id": "__plugin__"},   # Always "__plugin__" for plugin actions
    "action": "add",                     # The plugin action ID
    "context": "...",                    # Current context (if any)
    "session": "..."
}
```

**Example: Clipboard with Wipe action**

```python
def get_plugin_actions():
    return [
        {
            "id": "wipe",
            "name": "Wipe All",
            "icon": "delete_sweep",
            "confirm": "Wipe all clipboard history? This cannot be undone.",
            "shortcut": "Ctrl+1",
        }
    ]

def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    if step == "initial":
        print(json.dumps({
            "type": "results",
            "results": get_clipboard_entries(),
            "pluginActions": get_plugin_actions(),
        }))
        return

    if step == "action":
        # Plugin-level action (from toolbar)
        if selected.get("id") == "__plugin__" and action == "wipe":
            wipe_clipboard()
            print(json.dumps({
                "type": "execute",
                "execute": {"notify": "Clipboard wiped", "close": True}
            }))
            return
        
        # Item-specific actions...
```

**Best practices:**
- Maximum 6 actions (Ctrl+1 through Ctrl+6)
- Use `confirm` for dangerous/irreversible actions
- Hide actions during special modes (e.g., pass empty array during add mode)
- Common actions: Add, Refresh, Clear/Wipe, Settings, Export

**Example plugins:** [`clipboard/`](clipboard/handler.py), [`todo/`](todo/handler.py)

---

### 2. `card` - Show Rich Content

Display markdown-formatted content.

```python
{
    "type": "card",
    "card": {
        "title": "Card Title",
        "content": "**Markdown** content with *formatting*",
        "markdown": true
    },
    "inputMode": "submit",               # Optional: wait for Enter before next search
    "placeholder": "Type reply..."       # Optional: hint for input
}
```

**Example plugin:** [`dict/`](dict/handler.py) - Shows word definitions as markdown

---

### 3. `execute` - Run Command

Execute a shell command, optionally save to history.

```python
# Simple execution (no history)
{
    "type": "execute",
    "execute": {
        "command": ["xdg-open", "/path/to/file"],  # Shell command
        "notify": "File opened",                    # Optional: notification
        "close": true                               # Close launcher (true) or stay open (false)
    }
}

# With history tracking (searchable later)
{
    "type": "execute",
    "execute": {
        "command": ["xdg-open", "/path/to/file"],
        "name": "Open document.pdf",    # Required for history
        "icon": "description",           # Optional: icon in history
        "iconType": "material",          # Optional: "material" (default) or "system"
        "thumbnail": "/path/to/thumb",   # Optional: image preview
        "close": true
    }
}
```

**Example plugins:** [`files/`](files/handler.py), [`wallpaper/`](wallpaper/handler.py)

---

### 4. `execute` with `entryPoint` - Complex Replay

For actions that need handler logic on replay (API calls, sensitive data).

```python
{
    "type": "execute",
    "execute": {
        "name": "Copy password: GitHub",   # Required for history
        "icon": "key",
        "notify": "Password copied",
        "entryPoint": {                    # Stored for workflow replay
            "step": "action",
            "selected": {"id": "item_123"},
            "action": "copy_password"
        },
        "close": true
        # No "command" - entryPoint is used on replay
    }
}
```

On replay:

1. Workflow starts
2. Handler receives the stored `entryPoint` with `"replay": true`
3. Handler executes action (fetches fresh data from API)

**Example plugin:** [`bitwarden/`](bitwarden/handler.py) - Uses entryPoint for password copying (never stores passwords in command history)

---

### 5. `imageBrowser` - Image Selection UI

Open a rich image browser with thumbnails and directory navigation.

```python
{
    "type": "imageBrowser",
    "imageBrowser": {
        "directory": "~/Pictures/Wallpapers",  # Initial directory (~ expanded)
        "title": "Select Wallpaper",           # Sidebar title
        "enableOcr": false,                    # Enable text search via OCR (requires tesseract)
        "actions": [                           # Custom toolbar actions
            {"id": "set_dark", "name": "Set (Dark)", "icon": "dark_mode"},
            {"id": "set_light", "name": "Set (Light)", "icon": "light_mode"}
        ]
    }
}
```

| Field       | Type   | Default  | Description                                    |
| ----------- | ------ | -------- | ---------------------------------------------- |
| `directory` | string | required | Initial directory path (`~` expanded)          |
| `title`     | string | `""`     | Title shown in sidebar                         |
| `enableOcr` | bool   | `false`  | Enable background OCR indexing for text search |
| `actions`   | array  | `[]`     | Custom action buttons in toolbar               |

When user selects an image, handler receives:

```python
{
    "step": "action",
    "selected": {
        "id": "imageBrowser",           # Always "imageBrowser"
        "path": "/full/path/to/image",  # Selected image path
        "action": "set_dark"            # Action ID clicked
    }
}
```

**Example plugins:**

- [`wallpaper/`](wallpaper/handler.py) - Wallpaper selector with dark/light mode
- [`screenshot/`](screenshot/handler.py) - Screenshot browser with OCR text search (`enableOcr: true`)

---

### 6. `prompt` - Show Input Prompt

Display a simple text prompt.

```python
{
    "type": "prompt",
    "prompt": {"text": "Enter word to define..."}
}
```

**Example plugin:** [`dict/`](dict/handler.py) - Initial prompt for word input

---

### 7. `error` - Show Error

Display an error message.

```python
{
    "type": "error",
    "message": "Something went wrong"
}
```

---

## Input Modes

The `inputMode` field controls when search queries are sent to your handler:

| Mode       | Behavior                                  | Use Case                     |
| ---------- | ----------------------------------------- | ---------------------------- |
| `realtime` | Every keystroke triggers `step: "search"` | Fuzzy filtering, file search |
| `submit`   | Only Enter key triggers `step: "search"`  | Text input, forms, chat      |

**Key insight:** Execute directly in `submit` mode's search step - don't return results that require another Enter.

```python
# Realtime: filter results on each keystroke
if step == "search":
    filtered = [item for item in items if query.lower() in item.lower()]
    print(json.dumps({"type": "results", "results": filtered, "inputMode": "realtime"}))

# Submit: execute on Enter
if step == "search" and context == "__add_mode__":
    # User pressed Enter - add the item directly
    add_item(query)
    print(json.dumps({"type": "results", "results": get_all_items(), "clearInput": True}))
```

**Example plugins:**

- Realtime: [`files/`](files/handler.py), [`bitwarden/`](bitwarden/handler.py)
- Submit: [`quicklinks/`](quicklinks/handler.py) (search mode), [`todo/`](todo/handler.py) (add mode)

---

## Context Persistence

The `context` field lets you maintain state across `search` calls:

```python
# Enter edit mode - set context
if action == "edit":
    print(json.dumps({
        "type": "results",
        "context": f"__edit__:{item_id}",  # Will be sent back in search calls
        "inputMode": "submit",
        "placeholder": "Type new value...",
        "results": [...]
    }))

# Handle edit mode in search
if step == "search" and context.startswith("__edit__:"):
    item_id = context.split(":")[1]
    # Save the edit with query value
    save_item(item_id, query)
```

**Example plugin:** [`quicklinks/`](quicklinks/handler.py) - Uses context for edit mode, add mode, and search mode

---

## History Tracking

When `name` is provided in `execute`, the action is saved to search history.

### Simple Replay (command stored)

For actions replayable with a shell command:

```python
print(json.dumps({
    "type": "execute",
    "execute": {
        "command": ["xdg-open", "/path/to/file.png"],
        "name": "Open file.png",
        "icon": "image",
        "thumbnail": "/path/to/file.png",
        "close": True
    }
}))
```

### Complex Replay (entryPoint stored)

For actions needing handler logic (API calls, sensitive data):

```python
print(json.dumps({
    "type": "execute",
    "execute": {
        "name": "Copy password: GitHub",
        "entryPoint": {"step": "action", "selected": {"id": "123"}, "action": "copy"},
        "icon": "key",
        "close": True
        # No command - password never stored!
    }
}))
```

**Replay priority:** `command` (if present) > `entryPoint` (if provided)

### When to Use Each

| Use `command`          | Use `entryPoint`              |
| ---------------------- | ----------------------------- |
| Opening files          | API calls (passwords, tokens) |
| Copying static text    | Dynamic data fetching         |
| Running shell commands | Sensitive information         |
| Setting wallpapers     | State-dependent actions       |

### When NOT to Track History

- CRUD on ephemeral state (todo toggle/delete)
- One-time confirmations
- AI chat responses

---

## IPC Calls

Hamr and other Quickshell configs expose IPC targets for inter-process communication.

### Hamr IPC Targets

```bash
# List available hamr targets
qs -c hamr ipc show

# Toggle launcher visibility
qs -c hamr ipc call hamr toggle

# Open/close launcher
qs -c hamr ipc call hamr open
qs -c hamr ipc call hamr close

# Start a specific workflow directly
qs -c hamr ipc call hamr workflow bitwarden

# Refresh shell history
qs -c hamr ipc call shellHistoryService update
```

### Calling IPC from Python Handlers

```python
import subprocess

def call_ipc(config, target, method, *args):
    """Call IPC on any Quickshell config"""
    subprocess.Popen(
        ["qs", "-c", config, "ipc", "call", target, method] + list(args),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

# Hamr IPC examples
call_ipc("hamr", "hamr", "toggle")
call_ipc("hamr", "shellHistoryService", "update")
```

### Cross-Config IPC (External Shells)

Handlers can also call IPC on other Quickshell configs running on the system.
This is useful for syncing state with external UI components.

```python
# Example: Refresh end-4/ii sidebar after todo changes
# The todo sidebar lives in the "ii" config, not hamr
call_ipc("ii", "todo", "refresh")
```

**Example plugin:** [`todo/`](todo/handler.py) - Calls `qs -c ii ipc call todo refresh` to update end-4's sidebar widget after adding/editing/deleting tasks

---

## Launch Timestamp API

Hamr writes a timestamp file every time it opens. This is useful for plugins that need to know when hamr was launched (e.g., for trimming recordings to remove hamr UI).

### File Location

```
~/.cache/hamr/launch_timestamp
```

### File Format

Unix timestamp in milliseconds (e.g., `1734567890123`)

### Reading from Python

```python
from pathlib import Path
import time

LAUNCH_TIMESTAMP_FILE = Path.home() / ".cache" / "hamr" / "launch_timestamp"

def get_hamr_launch_time() -> int:
    """Get timestamp (ms) when hamr was last opened."""
    try:
        return int(LAUNCH_TIMESTAMP_FILE.read_text().strip())
    except (FileNotFoundError, ValueError):
        return int(time.time() * 1000)

# Calculate time since hamr opened
launch_time_ms = get_hamr_launch_time()
now_ms = int(time.time() * 1000)
time_since_launch_ms = now_ms - launch_time_ms
```

### Use Cases

| Use Case           | Description                                           |
| ------------------ | ----------------------------------------------------- |
| Screen recording   | Trim end of recording to remove hamr UI when stopping |
| Activity tracking  | Log when user invokes the launcher                    |
| Performance timing | Measure plugin response time relative to launch       |

**Example plugin:** [`screenrecord/`](screenrecord/handler.py) - Uses launch timestamp to calculate how much to trim from the end of recordings

---

## Handler Template

```python
#!/usr/bin/env python3
import json
import sys

def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")
    context = input_data.get("context", "")

    # ===== INITIAL: Show starting view =====
    if step == "initial":
        print(json.dumps({
            "type": "results",
            "results": [
                {"id": "item1", "name": "First Item", "icon": "star"},
                {"id": "item2", "name": "Second Item", "icon": "favorite"},
            ],
            "placeholder": "Search items..."
        }))
        return

    # ===== SEARCH: Filter or handle text input =====
    if step == "search":
        # Filter results based on query
        items = get_items()  # Your data source
        filtered = [i for i in items if query.lower() in i["name"].lower()]
        print(json.dumps({
            "type": "results",
            "results": filtered,
            "inputMode": "realtime"
        }))
        return

    # ===== ACTION: Handle selection =====
    if step == "action":
        item_id = selected.get("id", "")

        # Back navigation
        if item_id == "__back__":
            print(json.dumps({
                "type": "results",
                "results": get_initial_results(),
                "clearInput": True
            }))
            return

        # Execute action
        print(json.dumps({
            "type": "execute",
            "execute": {
                "command": ["notify-send", f"Selected: {item_id}"],
                "name": f"Do action: {item_id}",
                "icon": "check",
                "close": True
            }
        }))

if __name__ == "__main__":
    main()
```

---

## Icon Types

### Material Icons (default)

Use any icon from [Material Symbols](https://fonts.google.com/icons). This is the default when `iconType` is not specified.

| Category   | Icons                                                                   |
| ---------- | ----------------------------------------------------------------------- |
| Navigation | `arrow_back`, `home`, `menu`, `close`                                   |
| Actions    | `open_in_new`, `content_copy`, `delete`, `edit`, `save`, `add`, `check` |
| Files      | `folder`, `description`, `image`, `video_file`, `code`                  |
| UI         | `search`, `settings`, `star`, `favorite`, `info`, `error`               |
| Special    | `dark_mode`, `light_mode`, `wallpaper`, `key`, `person`                 |

### System Icons

For desktop application icons from `.desktop` files, set `"iconType": "system"`:

```python
{
    "id": "app-id",
    "name": "Google Chrome",
    "icon": "google-chrome",      # System icon name from .desktop file
    "iconType": "system"          # Required for system icons
}
```

**Common system icon patterns:**

- Reverse domain: `org.gnome.Calculator`, `com.discordapp.Discord`
- Kebab-case: `google-chrome`, `visual-studio-code`
- Simple names: `btop`, `blueman`, `firefox`

**Auto-detection:** If `iconType` is not specified, icons with `.` or `-` are assumed to be system icons. For simple names like `btop`, you must explicitly set `"iconType": "system"`.

**Example plugin:** [`apps/`](apps/) - App launcher using system icons

---

## Built-in Plugins Reference

| Plugin                             | Trigger          | Features                            | Key Patterns                            |
| ---------------------------------- | ---------------- | ----------------------------------- | --------------------------------------- |
| [`apps/`](apps/)                   | `/apps`          | App drawer with categories          | System icons, category navigation       |
| [`files/`](files/)                 | `~`              | File search with fd+fzf, thumbnails | Results with thumbnails, action buttons |
| [`clipboard/`](clipboard/)         | `;`              | Clipboard history, OCR search       | Thumbnails, OCR, filter actions         |
| [`shell/`](shell/)                 | `!`              | Shell command history               | Simple results, execute commands        |
| [`bitwarden/`](bitwarden/)         | `/bitwarden`     | Password manager                    | entryPoint replay, cache, error cards   |
| [`quicklinks/`](quicklinks/)       | `/quicklinks`    | Web search quicklinks               | Submit mode, context, CRUD              |
| [`dict/`](dict/)                   | `/dict`          | Dictionary lookup                   | Card response, API fetch                |
| [`pictures/`](pictures/)           | `/pictures`      | Image browser                       | Thumbnails, multi-turn navigation       |
| [`screenshot/`](screenshot/)       | `/screenshot`    | Screenshot browser                  | imageBrowser, enableOcr                 |
| [`screenrecord/`](screenrecord/)   | `/screenrecord`  | Screen recorder                     | Launch timestamp API, ffmpeg trim       |
| [`snippet/`](snippet/)             | `/snippet`       | Text snippets                       | Submit mode for add                     |
| [`todo/`](todo/)                   | `/todo`          | Todo list                           | Submit mode, IPC refresh, CRUD          |
| [`wallpaper/`](wallpaper/)         | `/wallpaper`     | Wallpaper selector                  | imageBrowser, history tracking          |
| [`create-plugin/`](create-plugin/) | `/create-plugin` | AI plugin creator                   | OpenCode integration                    |

---

## Keyboard Navigation

Users navigate with:

- **Ctrl+J/K** - Move down/up
- **Ctrl+L** or **Enter** - Select
- **Escape** - Exit workflow / close launcher

---

## Tips

1. **Always handle `__back__`** - Users expect back navigation
2. **Use `close: true`** only for final actions
3. **Keep results under 50** - Performance
4. **Use thumbnails sparingly** - They load images
5. **Use `placeholder`** - Helps users know what to type
6. **Use `context`** - Preserve state across search calls
7. **Debug with** `journalctl --user -f` - Check for errors
8. **Test edge cases** - Empty results, errors, special characters

---

## Testing Plugins

Use [`test-harness`](test-harness) to test your plugin without the UI. It simulates Hamr's stdin/stdout communication and validates responses against the Hamr protocol schema.

### HAMR_TEST_MODE Requirement

**Important:** The test-harness requires `HAMR_TEST_MODE=1` to be set. This prevents accidental API calls to external services during testing.

```bash
# Set before running test-harness
export HAMR_TEST_MODE=1

# Or inline
HAMR_TEST_MODE=1 ./test-harness ./handler.py initial
```

When `HAMR_TEST_MODE=1` is set, handlers should return **mock data** instead of calling real APIs. This ensures:
- No accidental charges to paid APIs
- No authentication required for tests
- Fast, deterministic test execution
- CI/CD compatibility

```python
# In your handler.py
import os

TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

def get_data():
    if TEST_MODE:
        return {"mock": "data"}  # Return mock data
    return call_real_api()        # Call real API only in production
```

### Basic Usage

```bash
# Test initial step
./test-harness ./my-plugin/handler.py initial

# Test search
./test-harness ./my-plugin/handler.py search --query "test"

# Test action
./test-harness ./my-plugin/handler.py action --id "item-1"

# Test action with action button
./test-harness ./my-plugin/handler.py action --id "item-1" --action "edit"

# Test with context (from previous response)
./test-harness ./my-plugin/handler.py search --query "new value" --context "__edit__:item-1"
```

### Workflow Testing

Each call is stateless. Use the response to craft your next call:

```bash
# Step 1: Get initial results
$ ./test-harness ./quicklinks/handler.py initial
{"type": "results", "results": [{"id": "google", ...}], ...}

# Step 2: Select an item
$ ./test-harness ./quicklinks/handler.py action --id "google"
{"type": "results", "context": "__search__:google", "inputMode": "submit", ...}

# Step 3: Enter search (using context from step 2)
$ ./test-harness ./quicklinks/handler.py search --query "hello" --context "__search__:google"
{"type": "execute", "execute": {"command": ["xdg-open", "..."], "close": true}}
```

### Commands

| Command                              | Description     |
| ------------------------------------ | --------------- |
| `initial`                            | Workflow start  |
| `search --query "..."`               | Search input    |
| `action --id "..." [--action "..."]` | Item selection  |
| `form --data '{...}'`                | Form submission |
| `replay --id "..." --action "..."`   | History replay  |
| `raw --input '{...}'`                | Raw JSON input  |

### Validation

The tool validates all responses. Invalid responses exit with code 1:

```bash
$ ./test-harness ./broken-handler.py initial
Error: Result item [0] missing required 'id'
Response type: results
Field: results[0].id
Expected: string
```

### Piping with jq

```bash
# Get all result IDs
./test-harness ./handler.py initial | jq -r '.results[].id'

# Check response type
./test-harness ./handler.py action --id "x" | jq -r '.type'

# Chain calls using context
CONTEXT=$(./test-harness ./handler.py action --id "__add__" | jq -r '.context')
./test-harness ./handler.py search --query "test" --context "$CONTEXT"
```

### Options

| Flag                | Description                   |
| ------------------- | ----------------------------- |
| `--timeout SECONDS` | Handler timeout (default: 10) |
| `--show-input`      | Print input JSON to stderr    |
| `--show-stderr`     | Print handler's stderr        |

### Writing Test Scripts

Use [`test-helpers.sh`](test-helpers.sh) for reusable test utilities:

```bash
#!/bin/bash
source "$(dirname "$0")/../test-helpers.sh"

TEST_NAME="My Plugin Tests"
HANDLER="$(dirname "$0")/handler.py"

test_initial() {
    local result=$(hamr_test initial)
    assert_type "$result" "results"
    assert_has_result "$result" "__add__"
}

test_search() {
    local result=$(hamr_test search --query "test")
    assert_contains "$result" "test"
}

run_tests test_initial test_search
```

### Test Helpers

| Function                         | Description                  |
| -------------------------------- | ---------------------------- |
| `hamr_test <cmd> [args]`         | Run handler via test-harness |
| `assert_type "$r" "results"`     | Assert response type         |
| `assert_has_result "$r" "id"`    | Assert result exists         |
| `assert_json "$r" '.path' "val"` | Assert JSON field            |
| `assert_submit_mode "$r"`        | Assert submit input mode     |
| `assert_contains "$r" "text"`    | Assert substring             |
| `run_tests fn1 fn2 ...`          | Run tests with summary       |

### File Naming Convention

Files prefixed with `test-` are excluded from Hamr's action list:

- `test-harness` - CLI test runner
- `test-helpers.sh` - Shared test utilities
- `*/test.sh` - Plugin test scripts (in subdirectories)

---

## AI-Assisted Plugin Development

The `test-harness` is designed for AI agents to build and verify plugins. AI can use it to:

1. **Validate handler output** - Ensure JSON responses conform to the Hamr protocol
2. **Test multi-step workflows** - Simulate user interactions without the UI
3. **Iterate on fixes** - Get immediate feedback on schema errors
4. **Verify mock data** - Test handlers return correct mock responses in test mode

### Workflow for AI Plugin Development

```
1. Create handler.py with basic structure
2. Run test-harness to validate initial response
3. Fix any schema errors reported
4. Test search and action steps
5. Implement mock data for HAMR_TEST_MODE
6. Create test.sh for automated testing
```

### Example: AI Building a Plugin

**Step 1: Create handler and test initial step**

```bash
HAMR_TEST_MODE=1 ./test-harness ./my-plugin/handler.py initial
```

If the handler outputs invalid JSON or missing required fields, test-harness exits with code 1 and shows the error:

```
Error: Result item [0] missing required 'id'
Response type: results
Field: results[0].id
Expected: string
```

**Step 2: Fix the error and re-run**

```bash
HAMR_TEST_MODE=1 ./test-harness ./my-plugin/handler.py initial
# Now outputs valid JSON - exit code 0
```

**Step 3: Test search step**

```bash
HAMR_TEST_MODE=1 ./test-harness ./my-plugin/handler.py search --query "test"
```

**Step 4: Test action step (using IDs from previous response)**

```bash
HAMR_TEST_MODE=1 ./test-harness ./my-plugin/handler.py action --id "item-1"
```

**Step 5: Test with context (for multi-step workflows)**

```bash
HAMR_TEST_MODE=1 ./test-harness ./my-plugin/handler.py action --id "__add__"
# Response includes: "context": "__add_mode__"

HAMR_TEST_MODE=1 ./test-harness ./my-plugin/handler.py search --query "new item" --context "__add_mode__"
```

### Schema Validation

The test-harness validates all response types against the Hamr protocol:

| Response Type   | Required Fields                          |
| --------------- | ---------------------------------------- |
| `results`       | `type`, `results[]` with `id` and `name` |
| `card`          | `type`, `card.content`                   |
| `execute`       | `type`, `execute` object                 |
| `imageBrowser`  | `type`, `imageBrowser.directory`         |
| `form`          | `type`, `form.fields[]` with `id`, `type`|
| `prompt`        | `type`, `prompt` object                  |
| `error`         | `type`, `message`                        |

### Exit Codes

| Code | Meaning                              |
| ---- | ------------------------------------ |
| 0    | Valid response                       |
| 1    | Invalid JSON, schema error, or timeout |

### AI Development Tips

1. **Always set HAMR_TEST_MODE=1** - Required by test-harness
2. **Implement mock data early** - Test handlers without real API calls
3. **Use `--show-input`** - Debug what JSON is sent to the handler
4. **Use `--show-stderr`** - See Python errors and debug output
5. **Pipe to jq** - Extract specific fields for verification
6. **Check exit codes** - Non-zero means validation failed

```bash
# Debug flags
HAMR_TEST_MODE=1 ./test-harness ./handler.py initial --show-input --show-stderr

# Check specific fields
HAMR_TEST_MODE=1 ./test-harness ./handler.py initial | jq '.results[0].id'

# Verify response type
HAMR_TEST_MODE=1 ./test-harness ./handler.py action --id "x" | jq -r '.type'

# Check exit code in scripts
if HAMR_TEST_MODE=1 ./test-harness ./handler.py initial > /dev/null 2>&1; then
    echo "Valid response"
else
    echo "Invalid response"
fi
```

### Mock Data Pattern

Handlers should check `HAMR_TEST_MODE` and return predictable mock data:

```python
#!/usr/bin/env python3
import json
import os
import sys

TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

# Mock data for testing
MOCK_ITEMS = [
    {"id": "mock-1", "name": "Mock Item 1", "value": "test-value-1"},
    {"id": "mock-2", "name": "Mock Item 2", "value": "test-value-2"},
]

def fetch_items():
    """Fetch items from API or return mock data in test mode."""
    if TEST_MODE:
        return MOCK_ITEMS
    # Real API call here
    return call_real_api()

def copy_to_clipboard(text):
    """Copy text to clipboard (skip in test mode)."""
    if TEST_MODE:
        return  # Don't actually copy in tests
    subprocess.run(["wl-copy", text], check=False)

def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    
    if step == "initial":
        items = fetch_items()
        print(json.dumps({
            "type": "results",
            "results": [
                {"id": item["id"], "name": item["name"], "icon": "star"}
                for item in items
            ]
        }))
        return
    
    # ... rest of handler

if __name__ == "__main__":
    main()
```

### Creating test.sh for CI/CD

After the handler works, create a test script for automated testing:

```bash
#!/bin/bash
# my-plugin/test.sh

# IMPORTANT: Must set HAMR_TEST_MODE before sourcing test-helpers.sh
export HAMR_TEST_MODE=1

source "$(dirname "$0")/../test-helpers.sh"

TEST_NAME="My Plugin Tests"
HANDLER="$(dirname "$0")/handler.py"

test_initial_returns_results() {
    local result=$(hamr_test initial)
    assert_type "$result" "results"
    assert_has_result "$result" "mock-1"
    assert_has_result "$result" "mock-2"
}

test_search_filters() {
    local result=$(hamr_test search --query "Item 1")
    assert_contains "$result" "Mock Item 1"
}

test_action_executes() {
    local result=$(hamr_test action --id "mock-1")
    assert_type "$result" "execute"
}

run_tests \
    test_initial_returns_results \
    test_search_filters \
    test_action_executes
```

Run with: `./my-plugin/test.sh`

---

## Converting Raycast Extensions

Hamr can replicate functionality from [Raycast](https://raycast.com) extensions. When porting a Raycast extension, understand these key differences:

### Architecture Comparison

| Aspect        | Raycast             | Hamr                     |
| ------------- | ------------------- | ------------------------ |
| **Language**  | TypeScript/React    | Any (Python recommended) |
| **UI Model**  | React components    | JSON responses           |
| **Data Flow** | React hooks + state | stdin/stdout per step    |
| **Platform**  | macOS               | Linux (Wayland/Hyprland) |

### Raycast Extension Structure

```
raycast-extension/
├── package.json          # Manifest + commands + preferences
├── src/
│   ├── index.tsx         # Main command (React component)
│   ├── hooks/            # Data fetching hooks
│   ├── components/       # Reusable UI
│   └── utils/            # Helper functions
└── assets/               # Icons
```

### Component Mapping

| Raycast Component        | Hamr Equivalent                                            |
| ------------------------ | ---------------------------------------------------------- |
| `<List>`                 | `{"type": "results", "results": [...]}`                    |
| `<List.Item>`            | `{"id": "...", "name": "...", "icon": "..."}`              |
| `<List.Item.Detail>`     | `{"type": "card", "card": {...}}`                          |
| `<Detail>`               | `{"type": "card", "card": {...}}`                          |
| `<Grid>`                 | `{"type": "imageBrowser", ...}` or results with thumbnails |
| `<Form>`                 | Multi-step workflow with `inputMode: "submit"`             |
| `<ActionPanel>`          | `"actions": [...]` array on result items                   |
| `Action.CopyToClipboard` | `{"command": ["wl-copy", "text"]}`                         |
| `Action.OpenInBrowser`   | `{"command": ["xdg-open", "url"]}`                         |
| `Action.Push`            | Return new results (multi-turn navigation)                 |
| `showToast()`            | `{"notify": "message"}` in execute                         |
| `getPreferenceValues()`  | Read from config file or environment                       |

### Hook Translation

| Raycast Hook       | Hamr Equivalent                            |
| ------------------ | ------------------------------------------ |
| `usePromise`       | Fetch data in handler, return results      |
| `useCachedPromise` | Cache to JSON file, check on each call     |
| `useCachedState`   | Use `context` field or file-based cache    |
| `useState`         | Use `context` field for state across steps |
| `useEffect`        | Not needed - each call is stateless        |

### Path Mapping (macOS → Linux)

| macOS Path                                                  | Linux Path                              |
| ----------------------------------------------------------- | --------------------------------------- |
| `~/Library/Application Support/Google/Chrome`               | `~/.config/google-chrome`               |
| `~/Library/Application Support/BraveSoftware/Brave-Browser` | `~/.config/BraveSoftware/Brave-Browser` |
| `~/Library/Application Support/Microsoft Edge`              | `~/.config/microsoft-edge`              |
| `~/Library/Application Support/Chromium`                    | `~/.config/chromium`                    |
| `~/Library/Application Support/Arc`                         | `~/.config/arc`                         |
| `~/Library/Application Support/Vivaldi`                     | `~/.config/vivaldi`                     |
| `~/Library/Safari/Bookmarks.plist`                          | N/A (Safari not on Linux)               |
| `~/.mozilla/firefox`                                        | `~/.mozilla/firefox` (same)             |
| `~/Library/Preferences`                                     | `~/.config`                             |
| `~/Library/Caches`                                          | `~/.cache`                              |

### API Mapping (macOS → Linux)

| Raycast/macOS API           | Linux Equivalent                              |
| --------------------------- | --------------------------------------------- |
| `Clipboard.copy()`          | `wl-copy` (Wayland) or `xclip` (X11)          |
| `Clipboard.paste()`         | `wl-paste` or `xclip -o`                      |
| `Clipboard.read()`          | `wl-paste` or `xclip -selection clipboard -o` |
| `showHUD()`                 | `notify-send`                                 |
| `open` (command)            | `xdg-open`                                    |
| `getFrontmostApplication()` | `hyprctl activewindow -j`                     |
| `getSelectedFinderItems()`  | Not directly available                        |
| AppleScript                 | Not available - use D-Bus or CLI tools        |
| Keychain                    | `secret-tool` (libsecret) or file-based       |

### Example: Raycast List → Hamr Results

**Raycast (TypeScript/React):**

```tsx
import { List, ActionPanel, Action } from "@raycast/api";

export default function Command() {
    const items = [
        { id: "1", title: "First", url: "https://example.com" },
        { id: "2", title: "Second", url: "https://example.org" },
    ];

    return (
        <List searchBarPlaceholder="Search bookmarks...">
            {items.map((item) => (
                <List.Item
                    key={item.id}
                    title={item.title}
                    subtitle={item.url}
                    actions={
                        <ActionPanel>
                            <Action.OpenInBrowser url={item.url} />
                            <Action.CopyToClipboard content={item.url} />
                        </ActionPanel>
                    }
                />
            ))}
        </List>
    );
}
```

**Hamr (Python):**

```python
#!/usr/bin/env python3
import json
import sys

def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    items = [
        {"id": "1", "title": "First", "url": "https://example.com"},
        {"id": "2", "title": "Second", "url": "https://example.org"},
    ]

    if step in ("initial", "search"):
        query = input_data.get("query", "").lower()
        filtered = [i for i in items if query in i["title"].lower()] if query else items

        print(json.dumps({
            "type": "results",
            "results": [
                {
                    "id": item["id"],
                    "name": item["title"],
                    "description": item["url"],
                    "icon": "bookmark",
                    "actions": [
                        {"id": "open", "name": "Open", "icon": "open_in_new"},
                        {"id": "copy", "name": "Copy URL", "icon": "content_copy"},
                    ]
                }
                for item in filtered
            ],
            "placeholder": "Search bookmarks..."
        }))
        return

    if step == "action":
        item_id = selected.get("id")
        item = next((i for i in items if i["id"] == item_id), None)
        if not item:
            return

        if action == "copy":
            print(json.dumps({
                "type": "execute",
                "execute": {
                    "command": ["wl-copy", item["url"]],
                    "notify": "URL copied",
                    "close": True
                }
            }))
        else:  # Default: open
            print(json.dumps({
                "type": "execute",
                "execute": {
                    "command": ["xdg-open", item["url"]],
                    "name": f"Open {item['title']}",
                    "icon": "bookmark",
                    "close": True
                }
            }))

if __name__ == "__main__":
    main()
```

### Conversion Checklist

When converting a Raycast extension:

1. **Identify the data source**
    - [ ] API calls → Use `requests` or `subprocess`
    - [ ] Local files → Update paths for Linux
    - [ ] System APIs → Find Linux equivalents

2. **Map UI components**
    - [ ] `List` → results response
    - [ ] `Detail`/`List.Item.Detail` → card response
    - [ ] `Grid` → imageBrowser or thumbnails
    - [ ] `Form` → multi-step with submit mode

3. **Handle actions**
    - [ ] `Action.OpenInBrowser` → `xdg-open`
    - [ ] `Action.CopyToClipboard` → `wl-copy`
    - [ ] `Action.Push` → return new results
    - [ ] Custom actions → map to execute commands

4. **Replace platform APIs**
    - [ ] Clipboard → `wl-copy`/`wl-paste`
    - [ ] Notifications → `notify-send`
    - [ ] File paths → Linux equivalents
    - [ ] Keychain → `secret-tool` or config file

5. **Test edge cases**
    - [ ] Empty results
    - [ ] Missing files/directories
    - [ ] Network errors
    - [ ] Permission errors

### Using AI to Convert

The [`create-plugin`](create-plugin/) workflow can help convert Raycast extensions:

1. Run `/create-plugin` in Hamr
2. Provide the Raycast extension URL (e.g., `https://github.com/raycast/extensions/tree/main/extensions/browser-bookmarks`)
3. The AI will analyze the extension and create a Hamr equivalent

Example prompt:

```
Create a Hamr plugin that replicates the functionality of this Raycast extension:
https://github.com/raycast/extensions/tree/main/extensions/browser-bookmarks

Focus on Chrome and Firefox support for Linux.
```
