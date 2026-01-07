# Plugin Development Guide

This is a quick reference for AI agents developing Hamr plugins. For complete documentation, see [docs/PLUGINS.md](../docs/PLUGINS.md).

## Documentation

| Document | Description |
|----------|-------------|
| **[Getting Started](../docs/PLUGINS.md)** | Build your first plugin |
| [Response Types](../docs/plugins/response-types.md) | `results`, `execute`, `card`, `form`, etc. |
| [Visual Elements](../docs/plugins/visual-elements.md) | Sliders, switches, badges, gauges |
| [Advanced Features](../docs/plugins/advanced-features.md) | Daemon mode, indexing, search ranking, CLI |
| [Cheat Sheet](../docs/plugins/CHEATSHEET.md) | Quick reference |

## Quick Start

```bash
# Create plugin directory
mkdir -p ~/.config/hamr/plugins/hello

# Create manifest
cat > ~/.config/hamr/plugins/hello/manifest.json << 'EOF'
{
  "name": "Hello",
  "description": "My first plugin",
  "icon": "waving_hand",
  "supportedCompositors": ["*"]
}
EOF

# Create handler
cat > ~/.config/hamr/plugins/hello/handler.py << 'EOF'
#!/usr/bin/env python3
import json
import sys

input_data = json.load(sys.stdin)
step = input_data.get("step", "initial")

if step == "initial":
    print(json.dumps({
        "type": "results",
        "results": [
            {"id": "greet", "name": "Say Hello", "icon": "waving_hand"}
        ]
    }))
elif step == "action":
    print(json.dumps({
        "type": "execute",
        "notify": "Hello!",
        "close": True
    }))
EOF

chmod +x ~/.config/hamr/plugins/hello/handler.py
```

## JSON Protocol

### Input (stdin)

```python
{
    "step": "initial|search|action",
    "query": "search text",
    "selected": {"id": "item-id"},
    "action": "action-button-id",
    "context": "custom-state"
}
```

### Output (stdout)

```python
# Show list
{"type": "results", "results": [...]}

# Execute action
{"type": "execute", "notify": "Done", "close": True}

# Show card
{"type": "card", "card": {"title": "...", "content": "..."}}

# Show form
{"type": "form", "form": {"title": "...", "fields": [...]}}

# Show error
{"type": "error", "message": "..."}
```

## Result Item

```python
{
    "id": "unique-id",           # Required
    "name": "Display Name",      # Required
    "description": "Subtitle",
    "icon": "star",              # Material icon
    "iconType": "material",      # or "system"
    "thumbnail": "/path/to/img",
    "verb": "Open",
    "actions": [{"id": "copy", "name": "Copy", "icon": "content_copy"}]
}
```

## Execute Fields

```python
{
    "type": "execute",
    "launch": "/path/to/app.desktop",
    "copy": "text to copy",
    "openUrl": "https://...",
    "open": "/path/to/file",
    "notify": "Notification message",
    "close": True
}
```

## Navigation

Hamr auto-increments depth when user clicks an item. Override with:

```python
{"type": "results", "navigateForward": True, ...}   # Drill down
{"type": "results", "navigateBack": True, ...}      # Go back
{"type": "results", "navigateForward": False, ...}  # Stay (for toggle/delete)
```

**Important:** Actions that modify data but stay on same view MUST use `navigateForward: False`.

## Manifest Fields

```json
{
  "name": "Plugin Name",
  "description": "Description",
  "icon": "star",
  "supportedCompositors": ["*"],
  "frecency": "item",
  "daemon": {"enabled": true, "background": false},
  "index": {"enabled": true},
  "indexOnly": false
}
```

| Field | Description |
|-------|-------------|
| `supportedCompositors` | `["*"]`, `["hyprland"]`, `["niri"]` |
| `frecency` | `"item"` (default), `"plugin"`, `"none"` |
| `daemon.background` | `true` = always run, `false` = only when open |
| `index.enabled` | Enable main search indexing (requires daemon) |
| `indexOnly` | No interactive mode, only provides index |

## Testing

```bash
# Visual testing (recommended)
./dev  # Run hamr from repo, type /your-plugin

# Manual testing
echo '{"step": "initial"}' | ./handler.py | jq .
echo '{"step": "search", "query": "test"}' | ./handler.py | jq .
echo '{"step": "action", "selected": {"id": "item-1"}}' | ./handler.py | jq .

# View logs
journalctl --user -u hamr -f
```

## Built-in Plugins

Study these for patterns:

| Plugin | Features |
|--------|----------|
| [`todo/`](todo/) | CRUD, daemon, status badges, `navigateForward: False` |
| [`clipboard/`](clipboard/) | Daemon, inotify, OCR, thumbnails |
| [`bitwarden/`](bitwarden/) | Forms, caching, entryPoint |
| [`emoji/`](emoji/) | gridBrowser |
| [`wallpaper/`](wallpaper/) | imageBrowser |
| [`sound/`](sound/) | Sliders, `update` response |
| [`apps/`](apps/) | System icons, indexing |

## Common Patterns

### Daemon with File Watching

See [Advanced Features: Daemon Mode](../docs/plugins/advanced-features.md#daemon-mode)

### Plugin Indexing

See [Advanced Features: Plugin Indexing](../docs/plugins/advanced-features.md#plugin-indexing)

### Search Ranking

See [Advanced Features: Search Ranking](../docs/plugins/advanced-features.md#search-ranking)

### CLI Commands

See [Advanced Features: CLI Reference](../docs/plugins/advanced-features.md#cli-reference)
