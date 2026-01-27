# Hamr Plugin SDK

The Hamr Plugin SDK provides tools for building plugins that extend the launcher's functionality.

## Plugin Types

Hamr supports two plugin communication models:

| Type   | Use Case                           | Communication               |
| ------ | ---------------------------------- | --------------------------- |
| stdio  | Simple, stateless plugins          | JSON over stdin/stdout      |
| socket | Stateful, long-running plugins     | JSON-RPC over Unix socket   |

## Quick Start

### 1. Create Plugin Directory

```bash
mkdir -p ~/.local/share/hamr/plugins/my-plugin
cd ~/.local/share/hamr/plugins/my-plugin
```

### 2. Create Manifest (`manifest.json`)

```json
{
  "name": "My Plugin",
  "description": "A simple example plugin",
  "icon": "extension",
  "frecency": "plugin",
  "handler": {
    "type": "stdio",
    "command": "python3 handler.py"
  },
  "match": {
    "prefix": "my:",
    "priority": 50
  }
}
```

**Manifest Fields:**

| Field              | Required | Description                                       |
| ------------------ | -------- | ------------------------------------------------- |
| `name`             | Yes      | Display name                                      |
| `description`      | No       | Short description                                 |
| `icon`             | No       | Material icon name                                |
| `frecency`         | No       | `"plugin"` (default) or `"none"`                  |
| `handler.type`     | Yes      | `"stdio"` or `"socket"`                           |
| `handler.command`  | Yes      | Command to run the plugin                         |
| `match.prefix`     | No       | Trigger prefix (e.g., `"my:"`)                    |
| `match.patterns`   | No       | Regex patterns to match                           |
| `match.priority`   | No       | Higher = checked first (default: 0)              |

### 3. Create Handler (`handler.py`)

#### Stdio Plugin (Simple)

```python
#!/usr/bin/env python3
import json
import sys

def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "")

    if step == "initial":
        # Show initial UI when plugin is activated
        print(json.dumps({
            "type": "prompt",
            "prompt": {"text": "Enter your search..."}
        }))
        return

    if step == "search":
        # Return search results
        print(json.dumps({
            "type": "results",
            "results": [
                {
                    "id": "result-1",
                    "name": f"Result for: {query}",
                    "description": "Click to select",
                    "icon": "star"
                }
            ],
            "inputMode": "realtime"
        }))
        return

    if step == "action":
        # Handle item selection
        selected = input_data.get("selected", {})
        print(json.dumps({
            "type": "execute",
            "copy": selected.get("name", ""),
            "notify": "Copied!",
            "close": True
        }))
        return

if __name__ == "__main__":
    main()
```

#### Socket Plugin (Stateful)

```python
#!/usr/bin/env python3
import sys
sys.path.insert(0, "/usr/share/hamr/plugins/sdk")
from hamr_sdk import HamrPlugin

plugin = HamrPlugin(
    id="my-plugin",
    name="My Plugin",
    description="A socket-based plugin",
    icon="extension"
)

@plugin.on_search
def handle_search(query: str, context: str | None) -> list[dict]:
    return [{"id": "1", "name": f"Result: {query}", "icon": "star"}]

@plugin.on_action
def handle_action(item_id: str, action: str | None, context: str | None, source: str | None) -> dict:
    return plugin.copy_and_close("Hello, World!")

plugin.run()
```

## Response Types

### Results

```python
{
    "type": "results",
    "results": [
        {"id": "1", "name": "Item", "description": "...", "icon": "star"}
    ],
    "inputMode": "realtime",  # or "submit"
    "placeholder": "Search..."
}
```

### Execute (Action)

```python
{
    "type": "execute",
    "copy": "text to copy",      # Copy to clipboard
    "launch": "app.desktop",     # Launch desktop file
    "openUrl": "https://...",    # Open URL
    "close": True,               # Close launcher
    "hide": True,                # Hide (keep running)
    "notify": "Message"          # Show notification
}
```

### Card (Detail View)

```python
{
    "type": "card",
    "card": {
        "title": "Card Title",
        "content": "Plain text content",
        "markdown": "# Markdown content",
        "actions": [
            {"id": "copy", "name": "Copy", "icon": "content_copy"}
        ]
    }
}
```

### Form

```python
{
    "type": "form",
    "form": {
        "title": "Settings",
        "fields": [
            {"type": "text", "id": "name", "label": "Name", "value": ""},
            {"type": "switch", "id": "enabled", "label": "Enable", "value": True},
            {"type": "slider", "id": "volume", "label": "Volume", "min": 0, "max": 100, "value": 50}
        ]
    }
}
```

## Testing Plugins

### 1. Start the Daemon

```bash
# In terminal 1: Start the daemon
cargo run -p hamr-daemon
```

### 2. Test with CLI

```bash
# In terminal 2: Test your plugin

# Test a stdio plugin directly
cargo run -p hamr-cli -- test my-plugin "search query"

# Watch daemon logs for debugging
tail -f /tmp/hamr-daemon.log
```

### 3. Test with TUI

```bash
# In terminal 2: Start the TUI
cargo run -p hamr-tui

# Type your plugin prefix (e.g., "my:") to activate
# Watch TUI logs in another terminal
tail -f /tmp/hamr-tui.log
```

### 4. Enable Debug Logging

```bash
# For Python plugins
HAMR_PLUGIN_DEBUG=1 cargo run -p hamr-daemon

# For verbose Rust logging
RUST_LOG=debug cargo run -p hamr-daemon
```

## SDK Response Builders

The socket SDK provides convenience methods for building responses:

```python
from hamr_sdk import HamrPlugin

plugin = HamrPlugin(...)

# In handlers:
plugin.results([...])              # Build results response
plugin.execute(copy="text")        # Build execute response
plugin.card("Title", content="")   # Build card response
plugin.form({...})                 # Build form response
plugin.copy_and_close("text")      # Convenience: copy and close
plugin.open_url("https://...")     # Convenience: open URL
plugin.error("message")            # Build error response
```

## Best Practices

1. **Keep responses fast** - Users expect instant results
2. **Use realtime input mode** - For search-as-you-type experience
3. **Handle errors gracefully** - Return error responses, not exceptions
4. **Use icons consistently** - Material icon names (e.g., `star`, `search`, `settings`)
5. **Test incrementally** - Use `tail -f /tmp/hamr-daemon.log` while developing

## Examples

See the bundled plugins in `/usr/share/hamr/plugins/` for real-world examples:

- `calculate/` - Math, currency, and unit conversion (stdio)
- `emoji/` - Emoji picker (stdio)
- `shell/` - Run shell commands (socket)
- `clipboard/` - Clipboard history (socket)
- `settings/` - Launcher settings (socket)
