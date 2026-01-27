# Plugin Cheat Sheet

Quick reference for Hamr plugin development.

## Manifest Template

**Simple stdio plugin:**

```json
{
  "name": "My Plugin",
  "description": "What it does",
  "icon": "star",
  "supportedPlatforms": ["niri", "hyprland"],
  "handler": {
    "type": "stdio",
    "path": "handler.py"
  },
  "frecency": "item"
}
```

**Socket-based daemon plugin:**

```json
{
  "name": "My Daemon",
  "description": "Real-time updates",
  "icon": "sync",
  "supportedPlatforms": ["niri", "hyprland"],
  "handler": {
    "type": "socket",
    "command": "python3 handler.py"
  },
  "daemon": {
    "enabled": true,
    "background": true,
    "restartOnCrash": true,
    "maxRestarts": 5
  }
}
```

## Pattern Matching Manifest

For plugins that show results instantly in main search:

```json
{
  "name": "Calculate",
  "description": "Calculator",
  "icon": "calculate",
  "supportedPlatforms": ["niri", "hyprland", "macos", "windows"],
  "handler": {
    "type": "stdio",
    "path": "handler.py"
  },
  "match": {
    "patterns": ["^=", "^[\\d\\.]+\\s*[\\+\\-\\*\\/]"],
    "priority": 100
  }
}
```

## Handler Skeleton

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

    if step == "initial":
        print(json.dumps({
            "type": "results",
            "results": [{"id": "1", "name": "Item", "icon": "star"}],
            "placeholder": "Search..."
        }))
        return

    if step == "search":
        # Filter items
        print(json.dumps({"type": "results", "results": [...]}))
        return

    if step == "action":
        item_id = selected.get("id", "")
        if item_id == "__back__":
            # Handle back navigation
            pass
        if item_id == "__plugin__":
            # Handle toolbar action
            pass
        # Handle item selection
        print(json.dumps({"type": "execute", "close": True}))

if __name__ == "__main__":
    main()
```

## Response Types

```python
# Results list
{"type": "results", "results": [{...}], "placeholder": "..."}

# Execute action
{"type": "execute", "notify": "Done", "close": True}
{"type": "execute", "copy": "text", "close": True}
{"type": "execute", "openUrl": "https://...", "close": True}
{"type": "execute", "open": "/path/to/file", "close": True}
{"type": "execute", "launch": "/usr/share/applications/app.desktop", "close": True}

# Card view
{"type": "card", "card": {"title": "...", "content": "...", "markdown": True}}

# Form
{"type": "form", "form": {"title": "...", "fields": [{...}]}, "context": "..."}

# Error
{"type": "error", "message": "Something went wrong"}

# Match result (for pattern matching plugins)
{"type": "match", "result": {"id": "...", "name": "result", "icon": "...", "copy": "..."}}
{"type": "match", "result": None}  # No valid result, hide

# No operation
{"type": "noop"}
```

## Result Item

```python
{
    "id": "unique-id",           # Required
    "name": "Display Name",      # Required
    "description": "Subtitle",
    "icon": "star",
    "iconType": "material",      # or "system"
    "thumbnail": "/path/to/img",
    "verb": "Open",
    "actions": [{"id": "copy", "name": "Copy", "icon": "content_copy"}],
    "badges": [{"text": "3", "color": "#f44336"}],
    "chips": [{"text": "Label", "icon": "tag"}]
}
```

## Index Item (for plugins with `index.enabled: true`)

**Note:** Only needed if your manifest has `index.enabled: true`. Most simple plugins don't need this.

```python
{
    "id": "app:firefox",         # Required
    "name": "Firefox",           # Required
    "description": "Web Browser",
    "icon": "firefox",
    "iconType": "system",
    "keywords": ["browser", "web"],
    "verb": "Open",
    "entryPoint": {              # Required - how to invoke handler from main search
        "step": "action",
        "selected": {"id": "app:firefox"}
    },
    "actions": [
        {
            "id": "private",
            "name": "Private Window",
            "icon": "security",
            "entryPoint": {      # Required for indexed item actions
                "step": "action",
                "selected": {"id": "app:firefox"},
                "action": "private"
            }
        }
    ]
}
```

## Slider Item

```python
{
    "id": "volume",
    "type": "slider",
    "name": "Volume",
    "icon": "volume_up",
    "value": 75,
    "min": 0,
    "max": 100,
    "step": 5,
    "unit": "%"
}
```

## Switch Item

```python
{
    "id": "mute",
    "type": "switch",
    "name": "Mute Volume",
    "icon": "volume_up",
    "value": False
}
```

## Plugin Actions (Toolbar)

```python
"pluginActions": [
    {"id": "add", "name": "Add", "icon": "add_circle"},
    {"id": "wipe", "name": "Wipe", "icon": "delete_sweep", "confirm": "Are you sure?"}
]
```

## Form Fields

```python
{"id": "name", "type": "text", "label": "Name", "required": True}
{"id": "content", "type": "textarea", "label": "Content", "rows": 6}
{"id": "email", "type": "email", "label": "Email"}
{"id": "pass", "type": "password", "label": "Password"}
{"id": "theme", "type": "select", "label": "Theme", "options": [{"id": "dark", "name": "Dark"}]}
{"id": "enabled", "type": "switch", "label": "Enabled", "default": True}
{"id": "level", "type": "slider", "label": "Level", "min": 0, "max": 100, "step": 10}
{"id": "data", "type": "hidden", "value": "..."}
```

## Input Steps

| Step      | When                           | Key Fields              |
| --------- | ------------------------------ | ----------------------- |
| `initial` | Plugin opens                   | -                       |
| `search`  | User types                     | `query`                 |
| `action`  | User selects                   | `selected.id`, `action` |
| `match`   | Pattern matched in main search | `query`                 |
| `form`    | Form submitted                 | `formData`              |
| `poll`    | Polling tick                   | `query`                 |
| `index`   | Indexing request               | `mode`                  |

## `entryPoint` (for indexed items only)

**Only needed for plugins with `index.enabled: true`.**

| Field      | Type   | Default    | Description                     |
| ---------- | ------ | ---------- | ------------------------------- |
| `step`     | string | `"action"` | Step type                       |
| `selected` | object | -          | Item info, e.g. `{"id": "..."}` |
| `action`   | string | -          | Action to perform               |
| `query`    | string | -          | Query string                    |

```python
"entryPoint": {"step": "action", "selected": {"id": "item-1"}, "action": "copy"}
```

- **Inside active plugin:** Hamr builds request directly - no `entryPoint` needed
- **From main search:** Hamr uses stored `entryPoint` to build request

## Special IDs

| ID                | Meaning                    |
| ----------------- | -------------------------- |
| `__back__`        | Back button/Escape         |
| `__plugin__`      | Plugin action clicked      |
| `__form_cancel__` | Form cancelled             |
| `__empty__`       | Non-actionable placeholder |

## Navigation

Hamr auto-increments depth when user clicks an item (not action button). Override with:

```python
# Drill down (depth +1)
{"type": "results", "navigateForward": True, ...}

# Go back (depth -1)
{"type": "results", "navigateBack": True, ...}

# Jump to specific depth
{"type": "results", "navigationDepth": 0, ...}

# Stay at current depth (for in-place updates like toggle, delete, sync)
{"type": "results", "navigateForward": False, ...}
```

**Important:** Actions that modify data but stay on the same view MUST use `navigateForward: False`.

## Testing

```bash
# Visual testing (recommended): Open Hamr and type /your-plugin

# Manual handler testing
echo '{"step": "initial"}' | ./handler.py | jq .
echo '{"step": "search", "query": "test"}' | ./handler.py | jq .
echo '{"step": "action", "selected": {"id": "item-1"}}' | ./handler.py | jq .

# Check logs
journalctl --user -u hamr -f
```

## Common Icons

| Category   | Icons                                                          |
| ---------- | -------------------------------------------------------------- |
| Actions    | `add`, `delete`, `edit`, `save`, `content_copy`, `open_in_new` |
| Files      | `folder`, `description`, `image`, `code`, `video_file`         |
| UI         | `search`, `settings`, `star`, `favorite`, `info`, `error`      |
| Navigation | `arrow_back`, `home`, `menu`, `close`                          |
| Status     | `check`, `warning`, `sync`, `downloading`                      |

## Manifest Options

| Field                   | Required | Values                                   | Description                                                                |
| ----------------------- | -------- | ---------------------------------------- | -------------------------------------------------------------------------- |
| `name`                  | Yes      | string                                   | Plugin display name                                                        |
| `description`           | Yes      | string                                   | Short description                                                          |
| `icon`                  | Yes      | string                                   | Material icon name                                                         |
| `supportedPlatforms`    | Yes      | `["niri", "hyprland"]`, etc              | Platform/compositor support (list all explicitly)                          |
| `handler.type`          | No       | `"stdio"`, `"socket"`                    | Handler communication type (default: `stdio`)                              |
| `handler.path`          | No       | string                                   | Script filename for `stdio` handlers (e.g., `handler.py`)                  |
| `handler.command`       | No       | string                                   | Command to run for `socket` handlers (e.g., `python3 handler.py`)          |
| `frecency`              | No       | `"item"`, `"plugin"`, `"none"`           | Usage tracking (see [Search Ranking](advanced-features.md#search-ranking)) |
| `inputMode`             | No       | `"realtime"`, `"submit"`                 | Default input mode (default: `realtime`)                                   |
| `hidden`                | No       | bool                                     | Hide from plugin list (prefix-only access)                                 |
| `daemon.enabled`        | No       | bool                                     | Enable daemon mode                                                         |
| `daemon.background`     | No       | bool                                     | Run always vs when open                                                    |
| `daemon.restartOnCrash` | No       | bool                                     | Auto-restart on crash                                                      |
| `daemon.maxRestarts`    | No       | number                                   | Max restart attempts (0 = unlimited)                                       |
| `index.enabled`         | No       | bool                                     | Enable indexing (requires daemon)                                          |
| `indexOnly`             | No       | bool                                     | No interactive mode                                                        |
| `match.patterns`        | No       | array                                    | Regex patterns for instant match                                           |
| `match.priority`        | No       | number                                   | Match priority (default: 50)                                               |
| `staticIndex`           | No       | array                                    | Static index items defined in manifest                                     |

## CLI Commands

```bash
hamr toggle                    # Toggle launcher
hamr plugin <name>             # Open plugin directly
hamr status <id> '<json>'      # Update plugin status
hamr audio play <sound>        # Play sound
```

See [CLI Reference](advanced-features.md#cli-reference) for full details.
