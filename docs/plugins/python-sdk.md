# Python SDK Reference

The Python SDK (`hamr_sdk.py`) provides helpers for building socket-based daemon plugins that communicate with the Hamr daemon via JSON-RPC 2.0.

## Installation

The SDK is included in the `plugins/sdk/` directory. Import it in your plugin:

```python
import sys
from pathlib import Path

# Add parent directory to path to import SDK
sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin
```

## Quick Start

```python
from sdk.hamr_sdk import HamrPlugin

plugin = HamrPlugin(
    id="my-plugin",
    name="My Plugin",
    description="A socket-based plugin",
    icon="extension"
)

@plugin.on_search
def handle_search(query: str, context: str | None) -> dict:
    return HamrPlugin.results([
        {"id": "1", "name": "Result", "icon": "star"}
    ])

@plugin.on_action
def handle_action(item_id: str, action: str | None, context: str | None, source: str | None) -> dict:
    return HamrPlugin.copy_and_close("Hello")

plugin.run()
```

## HamrPlugin Class

### Constructor

```python
HamrPlugin(
    id: str,                          # Plugin identifier (matches manifest)
    name: str,                         # Display name
    description: str | None = None,    # Plugin description
    icon: str | None = None,           # Material icon name
    prefix: str | None = None,         # Search prefix (e.g., "!")
    priority: int = 0,                 # Search result priority
    socket_path: str | None = None,    # Custom socket path (defaults to $XDG_RUNTIME_DIR/hamr.sock)
    debug: bool | None = None,         # Enable debug logging (defaults to HAMR_PLUGIN_DEBUG env var)
)
```

### Handler Decorators

Register handlers using decorators:

| Decorator | Handler Signature | Description |
|-----------|------------------|-------------|
| `@plugin.on_initial` | `(params: dict) -> dict` | Called when plugin is opened |
| `@plugin.on_search` | `(query: str, context: str \| None) -> dict` | Called on search input |
| `@plugin.on_action` | `(item_id: str, action: str \| None, context: str \| None, source: str \| None) -> dict` | Called on item selection or action |
| `@plugin.on_form_submitted` | `(form_data: dict, context: str \| None) -> dict` | Called on form submission |
| `@plugin.on_slider_changed` | `(slider_id: str, value: float) -> dict` | Called on slider value change |
| `@plugin.on_switch_toggled` | `(switch_id: str, value: float) -> dict` | Called on switch toggle (value is `1.0` or `0.0`) |

Handlers can be sync or async:

```python
@plugin.on_search
def sync_handler(query: str, context: str | None) -> dict:
    return HamrPlugin.results([...])

@plugin.on_search
async def async_handler(query: str, context: str | None) -> dict:
    result = await fetch_data(query)
    return HamrPlugin.results([...])
```

### Background Tasks

Add background coroutines that run alongside message handling:

```python
@plugin.add_background_task
async def monitor_changes(p: HamrPlugin):
    """Background task receives plugin instance for sending updates."""
    while True:
        await asyncio.sleep(1)
        # Send status update
        await p.send_status({"chips": [{"text": "Updated"}]})
```

Background tasks are started after registration and run until the plugin exits.

### Notification Methods

Send notifications to the daemon (no response expected):

```python
# Send search results
await plugin.send_results(results=[...], **kwargs)

# Send status update (badges, chips, ambient items)
await plugin.send_status({"chips": [...], "badges": [...], "ambient": [...]})

# Send index items for search indexing
await plugin.send_index(items=[...])

# Request action execution
await plugin.send_execute({"type": "copy", "text": "Hello"})

# Send partial result updates (patches)
await plugin.send_update(patches=[{"id": "item1", "description": "Updated"}])
```

## Response Builders

Static methods for building properly-typed response dictionaries:

### `HamrPlugin.results()`

Build a results response:

```python
HamrPlugin.results(
    items: list[dict],                          # Required: result items
    input_mode: str | None = None,              # "realtime" for keystroke updates
    status: dict | None = None,                 # Status with chips/badges/ambient
    context: str | None = None,                 # Context passed to subsequent handlers
    placeholder: str | None = None,             # Search input placeholder
    clear_input: bool = False,                  # Clear search input
    navigate_forward: bool | None = None,       # Push navigation state
    plugin_actions: list[dict] | None = None,   # Action bar actions
    navigation_depth: int | None = None,        # Navigation depth hint
    display_hint: str | None = None,            # "auto", "list", "grid", "large_grid"
)
```

### `HamrPlugin.form()`

Build a form response:

```python
HamrPlugin.form(
    form: dict,                    # Form definition with title, fields
    context: str | None = None,    # Context for form submission
)
```

Example form:

```python
HamrPlugin.form({
    "title": "Settings",
    "fields": [
        {"type": "text", "id": "name", "label": "Name", "value": "Default"},
        {"type": "switch", "id": "enabled", "label": "Enable feature", "value": True},
        {"type": "slider", "id": "volume", "label": "Volume", "value": 50, "min": 0, "max": 100},
    ],
    "submitLabel": "Save"
})
```

### `HamrPlugin.card()`

Build a card response (detail view):

```python
HamrPlugin.card(
    title: str,                                  # Required: card title
    content: str | None = None,                  # Plain text content
    markdown: str | None = None,                 # Markdown content
    actions: list[dict] | None = None,           # Action buttons
    status: dict | None = None,                  # Status indicators
    kind: str | None = None,                     # Card kind
    blocks: list[dict] | None = None,            # Card blocks (pill, separator, etc.)
    max_height: int | None = None,               # Max height in pixels
    show_details: bool | None = None,            # Show details section
    allow_toggle_details: bool | None = None,    # Allow toggling details
)
```

### `HamrPlugin.execute()`

Build an execute response:

```python
HamrPlugin.execute(
    launch: str | None = None,       # Desktop file to launch
    copy: str | None = None,         # Text to copy to clipboard
    url: str | None = None,          # URL to open
    close: bool = False,             # Close the launcher
    hide: bool = False,              # Hide the launcher
    type_text: str | None = None,    # Text to type (ydotool)
    play_sound: str | None = None,   # Sound to play
)
```

### Convenience Methods

```python
# Close the launcher
HamrPlugin.close()

# Copy text and close
HamrPlugin.copy_and_close(text: str)

# Launch desktop file and close
HamrPlugin.launch_and_close(desktop_file: str)

# Open URL (optionally close)
HamrPlugin.open_url(url: str, close: bool = True)

# Return error
HamrPlugin.error(message: str, details: str | None = None)
```

## Action Handler Patterns

### Primary Action

When `action` is `None`, the user pressed Enter on the item:

```python
@plugin.on_action
def handle_action(item_id: str, action: str | None, context: str | None, source: str | None):
    if action is None:
        # Primary action (Enter pressed)
        return HamrPlugin.copy_and_close(get_value(item_id))
    elif action == "delete":
        delete_item(item_id)
        return build_results()
    elif action == "edit":
        return show_edit_form(item_id)
```

### Ambient Actions

When `source == "ambient"`, the action came from the ambient bar. Return status updates only (not results):

```python
@plugin.on_action
async def handle_action(item_id: str, action: str | None, context: str | None, source: str | None):
    if source == "ambient":
        # Handle ambient bar action
        perform_action(item_id, action)
        # Update status but don't return results (would open plugin view)
        await plugin.send_status(build_status())
        return {}
    
    # Regular action
    return HamrPlugin.results([...])
```

## Status Updates

### Badges and Chips

```python
await plugin.send_status({
    "badges": [
        {"text": "3", "color": "#4caf50"},     # Count badge
        {"icon": "warning", "color": "orange"} # Icon badge
    ],
    "chips": [
        {"text": "Running", "icon": "timer"},
        {"text": "2 active"}
    ]
})
```

### Ambient Items

Persistent items shown in the ambient bar:

```python
await plugin.send_status({
    "ambient": [
        {
            "id": "timer:123",
            "name": "Meeting",
            "description": "05:30",
            "icon": "timer",
            "actions": [
                {"id": "pause", "icon": "pause", "name": "Pause"},
                {"id": "delete", "icon": "delete", "name": "Delete"}
            ]
        }
    ]
})
```

### FAB Override

Override the floating action button when launcher is closed:

```python
await plugin.send_status({
    "fab": {
        "chips": [{"text": "05:30", "icon": "timer"}],
        "showFab": True,  # Force FAB visible
        "priority": 10    # Higher priority wins
    }
})
```

## Indexing

Daemon plugins can emit index items for main search:

```python
# On startup, emit full index
await plugin.send_index(items=[
    {
        "id": "item:1",
        "name": "My Item",
        "description": "Description",
        "icon": "star",
        "keywords": ["keyword1", "keyword2"],
        "entryPoint": {
            "step": "action",
            "selected": {"id": "item:1"}
        }
    }
])
```

The `entryPoint` field allows items to skip directly to action step when selected from main search.

## Debugging

Enable debug logging:

```bash
# Via environment variable
HAMR_PLUGIN_DEBUG=1 python3 handler.py

# Or in code
plugin = HamrPlugin(id="...", debug=True)
```

Debug messages go to stderr:

```
[my-plugin] Connecting to /run/user/1000/hamr.sock
[my-plugin] Connected
[my-plugin] Registered: {'ok': True}
[my-plugin] Received message: {'method': 'search', 'params': {...}}
```

## Complete Example: Timer Plugin

```python
#!/usr/bin/env python3
import asyncio
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

plugin = HamrPlugin(
    id="timer",
    name="Timer",
    description="Countdown timers",
    icon="timer"
)

timers = []

@plugin.on_initial
async def handle_initial(params=None):
    return HamrPlugin.results(
        get_timer_results(),
        placeholder="Enter duration (e.g., 5m, 1h30m)..."
    )

@plugin.on_search
async def handle_search(query: str, context: str | None):
    return HamrPlugin.results(
        get_timer_results(query),
        input_mode="realtime"
    )

@plugin.on_action
async def handle_action(item_id: str, action: str | None, context: str | None, source: str | None):
    if source == "ambient":
        # Ambient bar action - only send status update
        handle_timer_action(item_id, action)
        await plugin.send_status(get_status())
        return {}
    
    # Regular action
    handle_timer_action(item_id, action)
    return HamrPlugin.results(get_timer_results())

@plugin.add_background_task
async def tick_timers(p: HamrPlugin):
    while True:
        await asyncio.sleep(1)
        update_timers()
        await p.send_status(get_status())

plugin.run()
```

## Manifest Configuration

For socket-based daemon plugins, use this manifest structure:

```json
{
  "name": "My Plugin",
  "description": "Plugin description",
  "icon": "extension",
  "handler": {
    "type": "socket",
    "command": "python3 handler.py"
  },
  "daemon": {
    "enabled": true,
    "background": true,
    "restartOnCrash": true,
    "maxRestarts": 5
  },
  "supportedPlatforms": ["niri", "hyprland"]
}
```

See [Plugin Index](index.md) for full manifest documentation.
