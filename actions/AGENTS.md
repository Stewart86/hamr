# User Actions & Workflows

This directory contains custom actions and workflows for the Hamr launcher.

## Language Agnostic

Plugins communicate via **JSON over stdin/stdout** - use any language you prefer:

| Language | Use Case |
|----------|----------|
| **Python** | Recommended for most plugins - readable, batteries included |
| **Bash** | Simple scripts, system commands |
| **Go/Rust** | Performance-critical plugins, compiled binaries |
| **Node.js** | Web API integrations, existing npm packages |

The handler just needs to be executable and read JSON from stdin, write JSON to stdout.

## Directory Structure

```
~/.config/hamr/actions/
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
cat > ~/.config/hamr/actions/my-action << 'EOF'
#!/bin/bash
notify-send "Hello from my action!"
EOF
chmod +x ~/.config/hamr/actions/my-action

# 2. Appears as `/my-action` in launcher
```

**Examples:** [`screenshot-snip`](screenshot-snip), [`dark`](dark), [`light`](light), [`accentcolor`](accentcolor)

### Multi-Step Workflow (Folder)

```bash
# 1. Create folder with manifest and handler
mkdir ~/.config/hamr/actions/hello
cat > ~/.config/hamr/actions/hello/manifest.json << 'EOF'
{"name": "Hello", "description": "Greeting plugin", "icon": "waving_hand"}
EOF

# 2. Create handler (see template below)
touch ~/.config/hamr/actions/hello/handler.py
chmod +x ~/.config/hamr/actions/hello/handler.py
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

| Field | When Present | Description |
|-------|--------------|-------------|
| `step` | Always | `initial` on start, `search` on typing, `action` on click |
| `query` | `search` step | Current search bar text |
| `selected.id` | `action` step | ID of clicked item |
| `action` | `action` step | ID of action button (if clicked via action button) |
| `context` | After you set it | Persists your custom state across `search` calls |
| `replay` | History replay | `true` when action is replayed from search history |

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
            "icon": "material_icon",     # Optional: Material icon name
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
    "context": "my-state"                # Optional: persist state for search calls
}
```

**Example plugins:** [`quicklinks/`](quicklinks/handler.py), [`todo/`](todo/handler.py), [`bitwarden/`](bitwarden/handler.py)

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

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `directory` | string | required | Initial directory path (`~` expanded) |
| `title` | string | `""` | Title shown in sidebar |
| `enableOcr` | bool | `false` | Enable background OCR indexing for text search |
| `actions` | array | `[]` | Custom action buttons in toolbar |

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

| Mode | Behavior | Use Case |
|------|----------|----------|
| `realtime` | Every keystroke triggers `step: "search"` | Fuzzy filtering, file search |
| `submit` | Only Enter key triggers `step: "search"` | Text input, forms, chat |

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

| Use `command` | Use `entryPoint` |
|---------------|------------------|
| Opening files | API calls (passwords, tokens) |
| Copying static text | Dynamic data fetching |
| Running shell commands | Sensitive information |
| Setting wallpapers | State-dependent actions |

### When NOT to Track History

- CRUD on ephemeral state (todo toggle/delete)
- One-time confirmations
- AI chat responses

---

## IPC Calls

Refresh UI components after changes:

```python
import subprocess

def call_hamr_ipc(target, method, *args):
    subprocess.Popen(
        ["qs", "-c", "hamr", "ipc", "call", target, method] + list(args),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

# Example: refresh todo sidebar after adding item
call_hamr_ipc("todo", "refresh")
```

**Available targets:** Run `qs -c hamr ipc show` to list all.

**Example plugin:** [`todo/`](todo/handler.py) - Refreshes sidebar after changes

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

## Material Icons

Use any icon from [Material Symbols](https://fonts.google.com/icons).

| Category | Icons |
|----------|-------|
| Navigation | `arrow_back`, `home`, `menu`, `close` |
| Actions | `open_in_new`, `content_copy`, `delete`, `edit`, `save`, `add`, `check` |
| Files | `folder`, `description`, `image`, `video_file`, `code` |
| UI | `search`, `settings`, `star`, `favorite`, `info`, `error` |
| Special | `dark_mode`, `light_mode`, `wallpaper`, `key`, `person` |

---

## Built-in Plugins Reference

| Plugin | Trigger | Features | Key Patterns |
|--------|---------|----------|--------------|
| [`files/`](files/) | `~` | File search with fd+fzf, thumbnails | Results with thumbnails, action buttons |
| [`clipboard/`](clipboard/) | `;` | Clipboard history with images | Image thumbnails, wipe action |
| [`shell/`](shell/) | `!` | Shell command history | Simple results, execute commands |
| [`bitwarden/`](bitwarden/) | `/bitwarden` | Password manager | entryPoint replay, cache, error cards |
| [`quicklinks/`](quicklinks/) | `/quicklinks` | Web search quicklinks | Submit mode, context, CRUD |
| [`dict/`](dict/) | `/dict` | Dictionary lookup | Card response, API fetch |
| [`pictures/`](pictures/) | `/pictures` | Image browser | Thumbnails, multi-turn navigation |
| [`screenshot/`](screenshot/) | `/screenshot` | Screenshot browser | imageBrowser, enableOcr |
| [`snippet/`](snippet/) | `/snippet` | Text snippets | Submit mode for add |
| [`todo/`](todo/) | `/todo` | Todo list | Submit mode, IPC refresh, CRUD |
| [`wallpaper/`](wallpaper/) | `/wallpaper` | Wallpaper selector | imageBrowser, history tracking |
| [`create-plugin/`](create-plugin/) | `/create-plugin` | AI plugin creator | OpenCode integration |

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
