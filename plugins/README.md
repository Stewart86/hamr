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
├── static-plugin/      # Static plugin (no handler)
│   └── manifest.json   # Plugin metadata with staticIndex
├── dynamic-plugin/     # Dynamic plugin (with handler)
│   ├── manifest.json   # Plugin metadata
│   └── handler.py      # Plugin handler (executable, any language)
```

**Handler requirements (for dynamic plugins):**
- Must be executable (`chmod +x handler.py`)
- Must have a shebang (e.g., `#!/usr/bin/env python3`, `#!/bin/bash`, `#!/usr/bin/env node`)
- Default name is `handler.py`, but you can specify any name via `"handler"` in manifest.json

---

## Compositor Support

Hamr supports multiple Wayland compositors. By default, plugins are assumed to only work on Hyprland. To specify compositor compatibility, add the `supportedCompositors` field to your manifest:

```json
{
  "name": "My Plugin",
  "supportedCompositors": ["*"],
  ...
}
```

| Value | Description |
|-------|-------------|
| `["*"]` | Works on all compositors (universal) |
| `["hyprland"]` | Hyprland only (default if not specified) |
| `["niri"]` | Niri only |
| `["hyprland", "niri"]` | Specific compositors |

Plugins using compositor-specific APIs should specify their requirements:
- Uses `hyprctl` → `["hyprland"]`
- Uses `niri msg` → `["niri"]`
- Uses generic tools (wl-copy, notify-send) → `["*"]`

---

## Frecency (Search History)

Hamr tracks plugin and item usage to show recently used items in the main search. The `frecency` manifest field controls how your plugin's usage is recorded:

```json
{
  "name": "My Plugin",
  "frecency": "item"
}
```

| Value | Behavior | Use Case |
|-------|----------|----------|
| `"item"` | Track individual item usage (default) | Apps, sound sliders, clipboard items, emojis |
| `"plugin"` | Track plugin usage only | Todo, notes, bitwarden - shows plugin in Recent, not individual items |
| `"none"` | Don't track usage | Monitoring plugins (topcpu, topmem), ephemeral actions (power) |

### How Frecency Works

- **`"item"` mode (default)**: When user executes an item, that specific item appears in the "Recent" section of main search. Good for plugins where users repeatedly access the same items (specific apps, specific clipboard entries).

- **`"plugin"` mode**: When user interacts with the plugin, the plugin itself appears in "Recent" (not individual items). Good for plugins where items are ephemeral or sensitive (todos change frequently, passwords shouldn't be in history).

- **`"none"` mode**: Plugin usage is not recorded at all. Good for monitoring/status plugins where frecency doesn't make sense.

### Examples

```json
// Apps plugin - track which apps are launched
{"name": "Apps", "frecency": "item"}

// Todo plugin - just track that user opened todos
{"name": "Todo", "frecency": "plugin"}

// CPU monitor - don't track, just monitoring
{"name": "Top CPU", "frecency": "none"}
```

### Default Behavior

If `frecency` is not specified, it defaults to `"item"`. Most plugins should use the default unless:
- Items are sensitive (passwords, tokens) → use `"plugin"`
- Items are ephemeral (calculations, definitions) → use `"plugin"` or `"none"`
- Plugin is for monitoring/status → use `"none"`

---

## Quick Start

### Static Plugin (No Handler)

For simple actions, use `staticIndex` in the manifest - no handler script needed:

```bash
# 1. Create folder with manifest
mkdir ~/.config/hamr/plugins/my-action
cat > ~/.config/hamr/plugins/my-action/manifest.json << 'EOF'
{
  "name": "My Action",
  "description": "A simple action",
  "icon": "star",
  "staticIndex": [
    {
      "id": "greet",
      "name": "Say Hello",
      "description": "Show a greeting notification",
      "icon": "waving_hand",
      "keywords": ["hello", "greet"],
      "execute": {
        "command": ["notify-send", "Hello from my action!"],
        "close": true
      }
    }
  ]
}
EOF

# 2. "Say Hello" appears in main search
```

**Examples:** [`accentcolor/`](accentcolor/), [`theme/`](theme/), [`snip/`](snip/), [`colorpick/`](colorpick/)

### Dynamic Plugin (With Handler)

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

### Custom Handler Name

By default, Hamr looks for `handler.py`. To use a different filename or language:

```json
{
  "name": "My Plugin",
  "handler": "handler.js"
}
```

```json
{
  "name": "Fast Plugin",
  "handler": "handler"
}
```

The handler must be executable with a proper shebang.

---

## JSON Protocol Reference

### Input (stdin)

Your handler receives JSON on stdin with these fields:

```python
{
    "step": "initial|search|action|form",  # Current step type
    "query": "user typed text",             # Search bar content
    "selected": {"id": "item-id"},          # Selected item (for action step)
    "action": "action-button-id",           # Action button clicked (optional)
    "context": "custom-context",            # Your custom context (persists across steps)
    "formData": {"field": "value"},         # Form field values (for form step)
    "session": "unique-session-id",         # Session identifier
    "replay": true                          # True when replaying from history (optional)
}
```

| Field         | When Present     | Description                                                        |
| ------------- | ---------------- | ------------------------------------------------------------------ |
| `step`        | Always           | `initial` on start, `search` on typing, `action` on click, `form` on submit |
| `query`       | `search` step    | Current search bar text                                            |
| `selected.id` | `action` step    | ID of clicked item                                                 |
| `action`      | `action` step    | ID of action button (if clicked via action button)                 |
| `context`     | After you set it | Persists your custom state across steps                            |
| `formData`    | `form` step      | Object with field id → value pairs from form submission            |
| `replay`      | History replay   | `true` when action is replayed from search history                 |

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
            "verb": "Open",              # Optional: primary action text (shown on hover, triggered by Enter/click)
            "actions": [                 # Optional: up to 4 secondary action buttons
                {"id": "copy", "name": "Copy", "icon": "content_copy"}
            ]
        }
    ],
    "inputMode": "realtime",             # Optional: "realtime" (default) or "submit"
    "placeholder": "Search...",          # Optional: search bar placeholder
    "clearInput": true,                  # Optional: clear search text
    "context": "my-state",               # Optional: persist state for search calls
    "notify": "Action completed",        # Optional: show notification toast
    "pluginActions": [                   # Optional: plugin-level action bar buttons
        {"id": "add", "name": "Add", "icon": "add_circle"},
        {"id": "wipe", "name": "Wipe All", "icon": "delete_sweep", "confirm": "Are you sure?"}
    ]
}
```

**Result item fields:**

| Field | Description |
|-------|-------------|
| `verb` | Primary action text shown on hover. Triggered by Enter or click. Use contextual verbs like "Done" / "Undone" for todos, "Open" for files, "Copy" for clipboard items. |
| `actions` | Up to 4 secondary action buttons. Each needs `id`, `name`, and `icon`. Shown as icon buttons on hover. |
| `badges` | Up to 5 compact badges shown beside item name. See Visual Enhancements below. |
| `chips` | Pill-shaped tags for longer text shown beside item name. See Visual Enhancements below. |
| `graph` | Line graph data shown in place of icon. See Visual Enhancements below. |
| `gauge` | Circular progress indicator shown in place of icon. See Visual Enhancements below. |
| `progress` | Horizontal progress bar shown below name (replaces description). See Visual Enhancements below. |

**Example plugins:** [`quicklinks/`](quicklinks/handler.py), [`todo/`](todo/handler.py), [`bitwarden/`](bitwarden/handler.py)

---

### 1b. `update` - Patch Individual Items

Update individual result items without replacing the entire results array. Useful for incremental updates (e.g., slider adjustments, status changes) that should preserve selection and focus.

```python
{
    "type": "update",
    "items": [
        {
            "id": "volume",           # Required: item to update (matched by id)
            "gauge": {                # Optional: update gauge
                "value": 75,
                "max": 100,
                "label": "75%"
            },
            "icon": "volume_up",      # Optional: update icon
            "badges": [               # Optional: update badges
                {"text": "M", "background": "#f44336"}
            ],
            "progress": {             # Optional: update progress bar
                "value": 50,
                "max": 100,
                "label": "50%"
            }
        },
        {
            "id": "mic",              # Update another item
            "gauge": {"value": 60, "max": 100, "label": "60%"}
        }
    ]
}
```

**Key differences from `results`:**

| Aspect | `results` | `update` |
|--------|-----------|----------|
| **Array replacement** | Replaces entire `pluginResults` array | Patches individual items in place |
| **Selection** | Resets to first item | Preserves current selection |
| **Focus** | Loses focus state | Maintains focus |
| **Use case** | New view, full refresh | Incremental updates, slider changes |

**When to use `update`:**

- Slider value changes (volume, brightness, progress)
- Reactive updates to existing items (badges, gauges, progress bars)
- Live data that doesn't need full list refresh (e.g., status changes)

**Example plugin:** [`sound/`](sound/handler.py) - Uses `update` for volume/microphone slider adjustments

---

### Slider Items

Slider items are a special result type for adjustable values (volume, brightness, etc.).

```python
{
    "type": "results",
    "results": [
        {
            "id": "volume",
            "type": "slider",           # Makes this a slider item
            "name": "Volume",
            "icon": "volume_up",
            "value": 75,                # Current value
            "min": 0,                   # Minimum value
            "max": 100,                 # Maximum value
            "step": 5,                  # Step increment
            "unit": "%",                # Optional: unit suffix (e.g., "%", "px", "ms")
            "displayValue": "75%"       # Optional: override display text entirely
        }
    ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `value` | number | Current slider value |
| `min` | number | Minimum value |
| `max` | number | Maximum value |
| `step` | number | Step increment (also determines decimal precision) |
| `unit` | string | Unit suffix appended to value (e.g., `"%"`, `"px"`, `"ms"`) |
| `displayValue` | string | Override display text entirely (ignores unit) |

**Receiving slider changes:**

When user drags the slider or clicks +/- buttons, handler receives:

```python
{
    "step": "action",
    "selected": {"id": "volume"},
    "action": "slider",
    "value": 80,                         # New value
    "context": "...",
    "session": "..."
}
```

Handler should update the actual value (e.g., system volume) and return updated results:

```python
if action == "slider":
    item_id = selected.get("id")
    new_value = input_data.get("value", 0)
    
    # Apply the change
    set_volume(item_id, new_value)
    
    # Return updated results
    return {
        "type": "results",
        "results": get_all_sliders(),  # With updated values
        "navigateForward": False        # Don't change navigation
    }
```

---

### Visual Enhancements

Result items can display additional visual elements for quick data overview.

#### Badges

Small circular indicators shown beside the item name (like avatar initials or status dots). Max 5 badges per item. Background is always the theme default; use `color` to tint text/icons.

```python
{
    "id": "task-1",
    "name": "Review PR",
    "icon": "task",
    "badges": [
        {"text": "JD"},                        # Initials
        {"text": "!", "color": "#f44336"},     # Alert (red text)
        {"icon": "verified", "color": "#4caf50"},  # Icon badge (green)
        {"image": "/path/to/avatar.png"},      # Avatar image
    ]
}
```

| Badge Field | Type | Description |
|-------------|------|-------------|
| `text` | string | 1-3 characters (displayed as initials) |
| `image` | string | Image path for avatar (overrides text) |
| `icon` | string | Material icon name (overrides text) |
| `color` | string | Text/icon color (hex, e.g., "#f44336") |

#### Chips

Compact pill-shaped tags for longer text shown beside the item name. Use for labels, categories, or status text that needs more than 1-3 characters.

```python
{
    "id": "task-1",
    "name": "Review PR",
    "icon": "task",
    "chips": [
        {"text": "In Progress"},                  # Simple label
        {"text": "Frontend", "icon": "code"},     # Chip with icon
        {"text": "Urgent", "color": "#f44336"},   # Colored text
    ]
}
```

| Chip Field | Type | Description |
|------------|------|-------------|
| `text` | string | Label text (longer than badges) |
| `icon` | string | Optional material icon before text |
| `color` | string | Text/icon color (hex, e.g., "#f44336") |

#### Graph

Simple line graph shown in place of the icon. Use for trends, history data.

```python
{
    "id": "cpu-monitor",
    "name": "CPU Usage",
    "graph": {
        "data": [45, 52, 48, 61, 55, 50, 47],  # Y values (array of numbers)
        "min": 0,                               # Optional: min Y value
        "max": 100                              # Optional: max Y value
    }
}
```

If `min`/`max` not provided, auto-scales from data.

#### Gauge

Circular progress indicator shown in place of the icon. Use for percentages, quotas, levels.

```python
{
    "id": "disk-usage",
    "name": "Disk Space",
    "gauge": {
        "value": 75,           # Current value
        "max": 100,            # Maximum value
        "label": "75%"         # Optional: center label text
    }
}
```

#### Progress Bar

Horizontal progress bar shown below the item name (replaces description). Use for download progress, sync status, or any linear progress indicator.

```python
{
    "id": "download-1",
    "name": "Downloading file.zip",
    "icon": "downloading",
    "progress": {
        "value": 65,           # Current value
        "max": 100,            # Maximum value
        "label": "65%",        # Optional: text shown beside bar
        "color": "#4caf50"     # Optional: custom bar color (hex)
    }
}
```

| Progress Field | Type | Description |
|----------------|------|-------------|
| `value` | number | Current progress value |
| `max` | number | Maximum value (default: 100) |
| `label` | string | Optional text shown beside the bar |
| `color` | string | Optional custom color (hex, e.g., "#4caf50") |

**Note:** When `progress` is provided, it replaces the `description` field in the item display.

**Priority:** If multiple visual elements are provided, priority is: `graph` > `gauge` > `thumbnail` > `icon`. Progress bar is independent and shows below the name.

---

### Plugin Actions (Toolbar Buttons)

The `pluginActions` field displays action buttons in a toolbar below the search bar. These are for plugin-level actions (e.g., "Add", "Wipe", "Refresh") that apply to the plugin itself, not specific items.

```python
"pluginActions": [
    {
        "id": "add",           # Required: action ID
        "name": "Add Item",    # Required: button label
        "icon": "add_circle",  # Required: material icon
        "shortcut": "Ctrl+1",  # Optional: displayed shortcut (default: Ctrl+N)
        "confirm": "..."       # Optional: confirmation message (shows dialog before executing)
    }
]
```

| Field      | Type   | Required | Description                                        |
| ---------- | ------ | -------- | -------------------------------------------------- |
| `id`       | string | Yes      | Action ID sent to handler                          |
| `name`     | string | Yes      | Button label text                                  |
| `icon`     | string | Yes      | Material icon name                                 |
| `shortcut` | string | No       | Keyboard shortcut (default: Ctrl+1 through Ctrl+6) |
| `confirm`  | string | No       | If set, shows confirmation dialog before executing |
| `active`   | bool   | No       | Highlight button as active (for toggle filters)    |

**Keyboard shortcuts:** Ctrl+1 through Ctrl+6 execute plugin actions directly.

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

**Toggle filters with `active` field:**

```python
def get_plugin_actions(active_filter: str = "") -> list[dict]:
    return [
        {
            "id": "filter_images",
            "name": "Images",
            "icon": "image",
            "active": active_filter == "images",  # Highlighted when active
        },
        {
            "id": "filter_text",
            "name": "Text",
            "icon": "text_fields",
            "active": active_filter == "text",
        },
    ]
```

**Best practices:**
- Maximum 6 actions (Ctrl+1 through Ctrl+6)
- Use `confirm` for dangerous/irreversible actions
- Hide actions during special modes (e.g., pass empty array during add mode)
- Use `active` for toggle/filter buttons to show current state
- Common actions: Add, Refresh, Clear/Wipe, Settings, Export

**Example plugins:** [`clipboard/`](clipboard/handler.py), [`todo/`](todo/handler.py), [`notes/`](notes/handler.py)

---

### 2. `card` - Show Rich Content

Display markdown-formatted content with optional action buttons.

```python
{
    "type": "card",
    "card": {
        "title": "Card Title",
        "content": "**Markdown** content with *formatting*",
        "markdown": true,
        "actions": [                      # Optional: action buttons
            {"id": "edit", "name": "Edit", "icon": "edit"},
            {"id": "copy", "name": "Copy", "icon": "content_copy"},
            {"id": "back", "name": "Back", "icon": "arrow_back"}
        ]
    },
    "context": "item-id",                 # Optional: preserve state for action handling
    "inputMode": "submit",                # Optional: wait for Enter before next search
    "placeholder": "Type reply..."        # Optional: hint for input
}
```

**Card with actions:** When user clicks a card action, handler receives:

```python
{
    "step": "action",
    "selected": {"id": "item-id"},        # From context field
    "action": "edit",                      # The action ID clicked
    "context": "item-id"
}
```

**Example plugins:** [`dict/`](dict/handler.py) - Word definitions, [`notes/`](notes/handler.py) - Note viewer with edit/copy/delete

---

### 3. `execute` - Run Command

Execute a shell command, optionally save to history, play sounds.

```python
# Simple execution (no history)
{
    "type": "execute",
    "execute": {
        "command": ["xdg-open", "/path/to/file"],  # Shell command
        "notify": "File opened",                    # Optional: notification
        "sound": "complete",                        # Optional: play sound effect
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

#### Execute Fields

| Field | Type | Description |
|-------|------|-------------|
| `command` | string[] | Shell command to execute |
| `notify` | string | Notification message (via notify-send) |
| `sound` | string | Sound effect to play (see Sound Effects below) |
| `close` | bool | Close launcher after execution |
| `name` | string | Display name for history tracking |
| `icon` | string | Icon for history entry |
| `iconType` | string | `"material"` (default) or `"system"` |
| `thumbnail` | string | Image path for history preview |
| `entryPoint` | object | Entry point for complex replay (see below) |

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

### 6. `gridBrowser` - Generic Grid Selection UI

Open a grid-based selection UI for items like emojis, icons, or any collection where grid display is more efficient than a list.

```python
{
    "type": "gridBrowser",
    "gridBrowser": {
        "title": "Select Emoji",               # Title shown in header
        "items": [                              # Required: grid items
            {
                "id": "😀",                    # Required: unique identifier
                "name": "grinning face",       # Required: searchable name
                "icon": "😀",                  # Display icon (text, material, or image path)
                "iconType": "text",            # "text" (emoji), "material", or "image"
                "keywords": ["happy", "smile"] # Optional: additional search terms
            }
        ],
        "columns": 10,                         # Grid columns (default: 8)
        "cellAspectRatio": 1.0,                # Cell aspect ratio (default: 1.0)
        "actions": [                           # Optional: action buttons
            {"id": "copy", "name": "Copy", "icon": "content_copy"},
            {"id": "insert", "name": "Insert", "icon": "keyboard"}
        ]
    }
}
```

| Field             | Type   | Default  | Description                                  |
| ----------------- | ------ | -------- | -------------------------------------------- |
| `title`           | string | `""`     | Title shown in header                        |
| `items`           | array  | required | Array of grid items                          |
| `columns`         | int    | `8`      | Number of columns in grid                    |
| `cellAspectRatio` | float  | `1.0`    | Width/height ratio of cells                  |
| `actions`         | array  | `[]`     | Custom action buttons (Ctrl+1-6 shortcuts)   |

**Item fields:**

| Field      | Type     | Required | Description                                      |
| ---------- | -------- | -------- | ------------------------------------------------ |
| `id`       | string   | Yes      | Unique identifier (sent on selection)            |
| `name`     | string   | Yes      | Display name (used for filtering)                |
| `icon`     | string   | No       | Icon to display (text/emoji, material icon, or image path) |
| `iconType` | string   | No       | `"text"` (default), `"material"`, or `"image"`   |
| `keywords` | string[] | No       | Additional search/filter terms                   |

When user selects an item, handler receives:

```python
{
    "step": "action",
    "selected": {
        "id": "gridBrowser",     # Always "gridBrowser"
        "itemId": "😀",          # Selected item's id
        "action": "copy"         # Action ID clicked (or default action)
    }
}
```

**Keyboard navigation:**
- Arrow keys / hjkl - Navigate grid
- Enter - Select with default action
- Ctrl+1-6 - Execute action buttons
- Escape - Cancel / go back

**Example plugin:** [`emoji/`](emoji/handler.py) - Emoji picker with 10-column grid

---

### 7. `preview` - Side Panel Preview

Add a `preview` field to result items to show rich content in a side panel when the item is hovered or selected. Users can pin previews to the screen.

```python
{
    "type": "results",
    "results": [
        {
            "id": "image-1",
            "name": "sunset.jpg",
            "icon": "image",
            "thumbnail": "/path/to/sunset.jpg",
            "preview": {
                "type": "image",                    # "image", "markdown", "text", or "metadata"
                "content": "/path/to/sunset.jpg",  # Image path or text content
                "title": "Sunset Photo",           # Panel title
                "metadata": [                      # Optional: key-value pairs
                    {"label": "Size", "value": "3840x2160"},
                    {"label": "Date", "value": "2024-01-15"}
                ],
                "actions": [                       # Optional: action buttons
                    {"id": "open", "name": "Open", "icon": "open_in_new"},
                    {"id": "copy", "name": "Copy", "icon": "content_copy"}
                ],
                "detachable": true                 # Allow pinning to screen (default: true)
            }
        }
    ]
}
```

| Preview Type | Content Field | Description |
|--------------|---------------|-------------|
| `image` | File path | Shows image with optional metadata below |
| `markdown` | Markdown text | Renders markdown content |
| `text` | Plain text | Monospace text display |
| `metadata` | (uses metadata array) | Key-value pairs only |

**Behavior:**
- Panel slides out as a drawer from the launcher side
- Shows on mouse hover or keyboard selection
- Pin button detaches preview to a floating panel that persists after launcher closes
- Detached panels are draggable and independently closable

**Example plugins:** [`pictures/`](pictures/handler.py) - Image preview with metadata, [`notes/`](notes/handler.py) - Markdown preview

---

### 8. `form` - Multi-Field Input Dialog

Display a form dialog for collecting multiple inputs at once.

```python
{
    "type": "form",
    "form": {
        "title": "Add New Note",           # Dialog title
        "submitLabel": "Save",             # Submit button text (default: "Submit")
        "cancelLabel": "Cancel",           # Cancel button text (default: "Cancel")
        "fields": [
            {
                "id": "title",             # Field identifier (used in formData)
                "type": "text",            # Field type: "text" or "textarea"
                "label": "Title",          # Field label
                "placeholder": "Enter...", # Placeholder text
                "required": True,          # Validation (default: false)
                "default": ""              # Pre-filled value
            },
            {
                "id": "content",
                "type": "textarea",
                "label": "Content",
                "placeholder": "Enter content...\n\nSupports multiple lines.",
                "rows": 6,                 # Textarea height (default: 4)
                "default": ""
            }
        ]
    },
    "context": "__add__"                   # Context passed to form submission
}
```

**Field types:**

| Type | Description | Extra Fields |
|------|-------------|--------------|
| `text` | Single-line text input | `placeholder`, `required`, `default`, `hint` |
| `textarea` | Multi-line text input | `placeholder`, `required`, `default`, `rows`, `hint` |
| `email` | Email input with validation | `placeholder`, `required`, `default`, `hint` |
| `password` | Masked password input | `placeholder`, `required`, `hint` |
| `hidden` | Hidden field (not displayed) | `value` (required) |
| `switch` | Toggle switch (on/off) | `default` (bool), `hint` |
| `slider` | Range slider | `min`, `max`, `step`, `default`, `hint` |

**Field properties:**

| Property | Type | Description |
|----------|------|-------------|
| `id` | string | Field identifier (key in formData) |
| `type` | string | Field type (see table above) |
| `label` | string | Display label |
| `placeholder` | string | Placeholder text |
| `required` | bool | Validation: field must have value |
| `default` | string | Pre-filled value |
| `hint` | string | Help text shown below field |
| `rows` | int | Textarea height (default: 4) |
| `value` | string | Value for hidden fields |
| `min` | number | Minimum value (slider) |
| `max` | number | Maximum value (slider) |
| `step` | number | Step increment (slider) |

**Switch and slider fields:**

```python
{
    "type": "form",
    "form": {
        "title": "Settings",
        "fields": [
            {
                "id": "notifications",
                "type": "switch",
                "label": "Enable notifications",
                "default": True,
                "hint": "Receive alerts when tasks complete"
            },
            {
                "id": "volume",
                "type": "slider",
                "label": "Volume",
                "min": 0,
                "max": 100,
                "step": 5,
                "unit": "%",
                "default": 75,
                "hint": "Adjust audio level"
            }
        ]
    }
}
```

**Live update forms:**

For forms where changes should apply immediately (no submit button), set `liveUpdate: true`:

```python
{
    "type": "form",
    "form": {
        "title": "Appearance",
        "liveUpdate": True,  # Changes apply on slider release
        "fields": [
            {
                "id": "opacity",
                "type": "slider",
                "label": "Opacity",
                "min": 0,
                "max": 1,
                "step": 0.05,
                "default": 0.8,
            }
        ]
    },
    "context": "liveform:appearance"
}
```

When a slider value changes in a live form, handler receives:

```python
{
    "step": "formSlider",
    "fieldId": "opacity",
    "value": 0.75,
    "context": "liveform:appearance",
    "session": "..."
}
```

Handler should apply the change and return `noop`:

```python
if step == "formSlider":
    field_id = input_data.get("fieldId", "")
    value = input_data.get("value", 0)
    
    # Apply the change
    save_setting(field_id, value)
    
    # Return noop - UI already shows new value
    print(json.dumps({"type": "noop"}))
    return
```

**Example plugin:** [`settings/`](settings/handler.py) - Uses live form for appearance settings

When a switch value changes in a live form, handler receives:

```python
{
    "step": "formSwitch",
    "fieldId": "enableFeature",
    "value": true,
    "context": "liveform:settings",
    "session": "..."
}
```

Handler should apply the change and return `noop`:

```python
if step == "formSwitch":
    field_id = input_data.get("fieldId", "")
    value = input_data.get("value", False)
    
    # Apply the change
    save_setting(field_id, value)
    
    # Return noop - UI already shows new value
    print(json.dumps({"type": "noop"}))
    return
```

**Hidden fields for multi-step forms:**

Use hidden fields to pass data through multi-step form workflows (e.g., 2FA):

```python
# Step 1: User enters email/password, but 2FA is required
# Step 2: Show 2FA form with hidden fields to preserve credentials
{
    "type": "form",
    "form": {
        "title": "Two-Factor Authentication",
        "fields": [
            {"id": "email", "type": "hidden", "value": email},
            {"id": "password", "type": "hidden", "value": password},
            {"id": "code", "type": "text", "label": "2FA Code", "placeholder": "Enter code"},
        ]
    }
}
```

**Example plugin:** [`bitwarden/`](bitwarden/handler.py) - Uses email, password, hidden fields for login flow

**Receiving form submission:**

When the user submits the form, the handler receives:

```python
{
    "step": "form",                        # Step is "form"
    "formData": {                          # User's input keyed by field id
        "title": "My Note Title",
        "content": "Note content here..."
    },
    "context": "__add__",                  # Context you set earlier
    "session": "..."
}
```

**Handling form cancellation:**

When the user cancels the form, the handler receives an action step:

```python
{
    "step": "action",
    "selected": {"id": "__form_cancel__"},
    "context": "__add__",
    "session": "..."
}
```

**Example: Add/Edit workflow**

```python
def show_add_form():
    print(json.dumps({
        "type": "form",
        "form": {
            "title": "Add Item",
            "submitLabel": "Save",
            "fields": [
                {"id": "name", "type": "text", "label": "Name", "required": True},
                {"id": "notes", "type": "textarea", "label": "Notes", "rows": 4}
            ]
        },
        "context": "__add__"
    }))

def show_edit_form(item):
    print(json.dumps({
        "type": "form",
        "form": {
            "title": "Edit Item",
            "submitLabel": "Save",
            "fields": [
                {"id": "name", "type": "text", "label": "Name", "required": True, "default": item["name"]},
                {"id": "notes", "type": "textarea", "label": "Notes", "rows": 4, "default": item.get("notes", "")}
            ]
        },
        "context": f"__edit__:{item['id']}"  # Encode item ID in context
    }))

def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    context = input_data.get("context", "")
    form_data = input_data.get("formData", {})
    
    # Handle form submission
    if step == "form":
        if context == "__add__":
            name = form_data.get("name", "").strip()
            if name:
                create_item(name, form_data.get("notes", ""))
                # Return to list view
                print(json.dumps({
                    "type": "results",
                    "results": get_all_items(),
                    "navigateBack": True
                }))
            else:
                print(json.dumps({"type": "error", "message": "Name is required"}))
            return
        
        if context.startswith("__edit__:"):
            item_id = context.split(":", 1)[1]
            update_item(item_id, form_data)
            print(json.dumps({
                "type": "results",
                "results": get_all_items(),
                "navigateBack": True
            }))
            return
    
    # Handle form cancellation
    if step == "action" and input_data.get("selected", {}).get("id") == "__form_cancel__":
        print(json.dumps({
            "type": "results",
            "results": get_all_items()
        }))
        return
```

**Best practices:**

- Use `context` to distinguish between add vs edit modes
- Encode item IDs in context for edit operations (e.g., `__edit__:item_123`)
- Return `navigateBack: True` after successful submission to go back to list view
- Handle `__form_cancel__` to gracefully return to previous view
- Use `required: True` for mandatory fields

**Example plugin:** [`notes/`](notes/handler.py) - Add/edit notes with title and content fields

---

### 9. `prompt` - Show Input Prompt

Display a simple text prompt.

```python
{
    "type": "prompt",
    "prompt": {"text": "Enter word to define..."}
}
```

**Example plugin:** [`dict/`](dict/handler.py) - Initial prompt for word input

---

### 10. `error` - Show Error

Display an error message.

```python
{
    "type": "error",
    "message": "Something went wrong"
}
```

---

### 11. `noop` - No Operation

Signal that the action was handled but no UI update is needed. Use this for optimistic updates where the UI already reflects the change (e.g., slider adjustments).

```python
{
    "type": "noop"
}
```

**When to use `noop`:**

| Use Case | Why |
|----------|-----|
| Slider value changes | UI already shows new position from drag |
| Toggle states with immediate visual feedback | Checkbox/switch already toggled |
| Background operations | Action completed, no UI change needed |

**Important:** Even when returning `noop`, your handler should still handle errors properly. If an error occurs during the action, return an `error` response instead:

```python
def handle_slider(item_id: str, value: int) -> None:
    try:
        apply_value(item_id, value)
        print(json.dumps({"type": "noop"}))
    except Exception as e:
        print(json.dumps({"type": "error", "message": str(e)}))
```

**Example plugin:** [`sound/`](sound/handler.py) - Uses `noop` for volume slider adjustments

---

## Polling (Auto-Refresh)

**⚠️ Deprecated:** Polling is legacy for existing plugins. For new plugins that need live updates, use [Daemon Mode](#daemon-mode-persistent-processes) instead. Daemons are more efficient and support bidirectional communication with hamr.

For plugins that need periodic updates (e.g., process monitors, system stats), use the polling API.

### Enable Polling in manifest.json

```json
{
  "name": "Top CPU",
  "description": "Processes sorted by CPU usage",
  "icon": "speed",
  "poll": 2000
}
```

The `poll` field is the interval in milliseconds (e.g., `2000` = refresh every 2 seconds).

### Handle the `poll` Step

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

### Polling Behavior

| Aspect | Behavior |
|--------|----------|
| **When runs** | Only when plugin is active and not busy |
| **Input** | `step: "poll"` with last `query` for filtering |
| **Output** | Same format as `search` step |
| **Dynamic control** | Override with `pollInterval` in response |

### Dynamic Poll Interval

Override polling from response (enable/disable dynamically):

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

**Example plugins:** [`topcpu/`](topcpu/handler.py), [`topmem/`](topmem/handler.py)

---

## Daemon Mode (Persistent Processes)

For plugins that need live updates, file watching, or persistent state, use daemon mode instead of polling. Daemons are more efficient as they maintain a single long-running process instead of spawning a new process for each request.

### Enable Daemon in manifest.json

```json
{
  "name": "My Plugin",
  "daemon": {
    "enabled": true,
    "background": false
  }
}
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | false | Enable daemon mode |
| `background` | bool | false | Run always (true) or only when plugin is open (false) |

**Daemon lifecycle:**

- **`background: false`** - Daemon starts when plugin opens, stops when it closes. Use for live data displays (process monitors, media players).
- **`background: true`** - Daemon starts when hamr launches, runs always. Use for status updates, file watching (todo counts, clipboard).

### Daemon Handler Pattern

Daemon handlers use a persistent event loop instead of single request-response:

```python
#!/usr/bin/env python3
import json
import os
import signal
import select
import sys
import time

def emit(data):
    """Emit JSON response to stdout (line-buffered)."""
    print(json.dumps(data), flush=True)

def main():
    # Graceful shutdown
    signal.signal(signal.SIGTERM, lambda s, f: sys.exit(0))
    signal.signal(signal.SIGINT, lambda s, f: sys.exit(0))
    
    current_query = ""
    last_refresh = 0
    refresh_interval = 2.0  # seconds
    
    while True:
        # Non-blocking stdin read with timeout
        readable, _, _ = select.select([sys.stdin], [], [], 0.5)
        
        if readable:
            line = sys.stdin.readline()
            if not line:
                break  # EOF - hamr closed connection
            
            try:
                request = json.loads(line.strip())
            except json.JSONDecodeError:
                continue
            
            step = request.get("step", "")
            
            if step == "initial":
                current_query = ""
                emit({
                    "type": "results",
                    "results": get_results(),
                    "placeholder": "Search...",
                })
                last_refresh = time.time()
                continue
            
            if step == "search":
                current_query = request.get("query", "")
                emit({
                    "type": "results",
                    "results": get_results(current_query),
                })
                last_refresh = time.time()
                continue
            
            if step == "action":
                # Handle actions...
                emit({"type": "results", "results": get_results(current_query)})
                continue
        
        # Periodic refresh (daemon-driven updates)
        now = time.time()
        if now - last_refresh >= refresh_interval:
            emit({
                "type": "results",
                "results": get_results(current_query),
            })
            last_refresh = now

if __name__ == "__main__":
    main()
```

### Key Differences from Request-Response

| Aspect | Request-Response | Daemon |
|--------|-----------------|--------|
| Process lifecycle | New process per request | Single persistent process |
| stdin | `json.load(sys.stdin)` | `readline()` in loop |
| stdout | Single `print(json.dumps(...))` | Multiple `emit()` calls |
| State | Stateless | Persistent (`current_query`, etc.) |
| Updates | On user action only | Can push updates anytime |

### Daemon Response Types

Daemons can emit any standard response type, plus special daemon-only types:

```python
# Push status update (updates plugin badge in main list)
emit({"type": "status", "status": {"badges": [{"text": "5"}], "chips": [{"text": "5 tasks"}]}})

# Push incremental index update (updates searchable items in main search)
emit({
    "type": "index",
    "mode": "incremental",
    "items": [new_item],      # New items to add
    "remove": ["old_id"],     # Item IDs to remove
})
```

### Background Daemon Responsibilities

Background daemons (`background: true`) should emit updates when their data changes:

| Update Type | When to Emit | Purpose |
|-------------|--------------|---------|
| `index` (full) | On daemon startup | Makes items searchable from main launcher immediately |
| `status` | On data change | Updates badge/chip on plugin entry in main list |
| `index` (incremental) | On data change | Makes new items searchable from main launcher |
| `results` | When plugin is open and data changes | Live updates to the visible list |

**Note:** Background daemons that emit their own index don't need `"index": {"enabled": true}` in the manifest. The daemon handles indexing directly.

**Example: Daemon startup**

```python
def main():
    # ... signal handlers ...
    
    # Emit initial status and full index on startup
    emit_status()
    entries = get_all_entries()[:100]
    indexed_ids = {f"item:{get_id(e)}" for e in entries}
    emit({
        "type": "index",
        "mode": "full",
        "items": [entry_to_index_item(e) for e in entries],
    })
    
    # ... main loop ...
```

**Example: On data change**

```python
# When data changes:
if current_mtime != last_db_mtime:
    last_db_mtime = current_mtime
    
    # 1. Update status badge (item count)
    emit_status()
    
    # 2. Update index (new items become searchable)
    indexed_ids = emit_incremental_index(indexed_ids)
    
    # 3. If plugin is open, refresh results list
    if plugin_active:
        respond(get_results(current_query, current_context))
```

### Background Daemon Use Cases

| Use Case | background | Example |
|----------|------------|---------|
| Process monitor | false | topcpu, topmem |
| Media player | false | player |
| Task count badge | true | todo |
| Clipboard watcher | true | clipboard |
| File sync status | true | bitwarden |

### Migration from Polling

To convert a polling plugin to daemon:

1. Remove `"poll": N` from manifest.json
2. Add `"daemon": {"enabled": true, "background": false}`
3. Wrap handler in event loop with `select.select()`
4. Use `emit()` helper with `flush=True`
5. Add signal handlers for graceful shutdown
6. Move periodic refresh logic inside the loop

**Example plugins:** [`topcpu/`](topcpu/handler.py), [`topmem/`](topmem/handler.py), [`todo/`](todo/handler.py)

### File Watching with inotify

For plugins that need to watch files for changes (e.g., config files, data files), use native inotify via ctypes. This is more efficient than polling and provides instant updates.

**Required imports and constants:**

```python
import ctypes
import ctypes.util
import struct

# inotify constants
IN_CLOSE_WRITE = 0x00000008
IN_MOVED_TO = 0x00000080
IN_CREATE = 0x00000100
```

**Create inotify watcher:**

```python
def create_inotify_fd(watch_path: Path) -> int | None:
    """Create inotify fd watching a directory. Returns fd or None."""
    try:
        libc_name = ctypes.util.find_library("c")
        if not libc_name:
            return None
        libc = ctypes.CDLL(libc_name, use_errno=True)

        inotify_init = libc.inotify_init
        inotify_init.argtypes = []
        inotify_init.restype = ctypes.c_int

        inotify_add_watch = libc.inotify_add_watch
        inotify_add_watch.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_uint32]
        inotify_add_watch.restype = ctypes.c_int

        fd = inotify_init()
        if fd < 0:
            return None

        watch_path.mkdir(parents=True, exist_ok=True)
        watch_dir = str(watch_path).encode()
        mask = IN_CLOSE_WRITE | IN_MOVED_TO | IN_CREATE
        wd = inotify_add_watch(fd, watch_dir, mask)
        if wd < 0:
            os.close(fd)
            return None

        return fd
    except Exception:
        return None
```

**Read inotify events:**

```python
def read_inotify_events(fd: int) -> list[str]:
    """Read inotify events and return list of filenames that changed."""
    filenames = []
    try:
        buf = os.read(fd, 4096)
        offset = 0
        while offset < len(buf):
            wd, mask, cookie, length = struct.unpack_from("iIII", buf, offset)
            offset += 16
            if length > 0:
                name = buf[offset : offset + length].rstrip(b"\x00").decode()
                filenames.append(name)
                offset += length
    except (OSError, struct.error):
        pass
    return filenames
```

**Use in main loop:**

```python
def main():
    signal.signal(signal.SIGTERM, lambda s, f: sys.exit(0))
    signal.signal(signal.SIGINT, lambda s, f: sys.exit(0))

    data = load_data()
    inotify_fd = create_inotify_fd(DATA_FILE.parent)

    if inotify_fd is not None:
        target_filename = DATA_FILE.name

        while True:
            readable, _, _ = select.select([sys.stdin, inotify_fd], [], [], 1.0)

            # Handle stdin
            stdin_ready = any(
                (f if isinstance(f, int) else f.fileno()) == sys.stdin.fileno()
                for f in readable
            )
            if stdin_ready:
                line = sys.stdin.readline()
                if not line:
                    break
                # ... handle request ...

            # Handle file changes
            if inotify_fd in readable:
                changed_files = read_inotify_events(inotify_fd)
                if target_filename in changed_files:
                    data = load_data()
                    emit({"type": "results", "results": get_results(data)})
    else:
        # Fallback to mtime polling if inotify unavailable
        # ... polling loop ...
```

**Key points:**

- Watch the **parent directory**, not the file itself (files get replaced on write)
- Use `IN_CLOSE_WRITE | IN_MOVED_TO | IN_CREATE` mask to catch all write patterns
- Always include a **fallback to mtime polling** for systems without inotify
- Emit full `results` response on file change, not just `status`

**Example plugin:** [`todo/`](todo/handler.py) - File watching with inotify + mtime fallback

---

## Plugin Status (Dynamic Badges)

Plugins can display dynamic status information (badges, description override) on their entry in the main launcher list. This is useful for showing counts, alerts, or quick summaries without opening the plugin.

### Setting Status via Response

Include a `status` field in your `results` or `index` response:

```python
# In any results response
print(json.dumps({
    "type": "results",
    "results": [...],
    "status": {
        "badges": [
            {"text": "5", "background": "#f44336", "color": "#ffffff"}
        ],
        "description": "5 pending tasks"  # Optional: overrides manifest description
    }
}))

# In index response
print(json.dumps({
    "type": "index",
    "items": [...],
    "status": {
        "badges": [{"text": str(len(items))}]
    }
}))
```

### Setting Status via CLI (External Updates)

For updates from external processes or daemons:

```bash
# Update status from any script
hamr status todo '{"badges": [{"text": "5"}]}'

# With description override
hamr status todo '{"badges": [{"text": "!"}], "description": "Action required"}'
```

### Status Fields

| Field | Type | Description |
|-------|------|-------------|
| `badges` | array | Circular indicators (1-3 chars, icons) |
| `chips` | array | Pill-shaped tags for longer text |
| `description` | string | Override the manifest description temporarily |

### Badge Format

Badges are circular indicators for short content (1-3 characters):

```python
{
    "text": "5",                    # 1-3 characters
    "background": "#f44336",        # Optional: background color
    "color": "#ffffff",             # Optional: text color
    "icon": "star"                  # Optional: material icon instead of text
}
```

### Chip Format

Chips are pill-shaped tags for longer text (a few words):

```python
{
    "text": "5 tasks",              # Longer descriptive text
    "icon": "task_alt",             # Optional: material icon before text
    "background": "#4caf50",        # Optional: background color
    "color": "#ffffff"              # Optional: text color
}
```

### Use Cases

| Use Case | Format | Example |
|----------|--------|---------|
| Unread count | Badge | `{"badges": [{"text": "12"}]}` |
| Alert/warning | Badge | `{"badges": [{"text": "!", "background": "#f44336"}]}` |
| Status indicator | Badge | `{"badges": [{"icon": "sync"}]}` |
| Task count | Chip | `{"chips": [{"text": "5 tasks", "icon": "task_alt"}]}` |
| Status text | Chip | `{"chips": [{"text": "Syncing...", "icon": "sync"}]}` |

### When Status Updates

- **On handler response**: Status updates when plugin returns results/index with `status` field
- **On reindex**: Plugins with file/event watchers update on data changes
- **Via IPC**: External scripts can push updates anytime via `hamr status`

**Live updates**: Status badges update in real-time without flickering or resetting selection. External processes can push frequent updates (e.g., progress indicators) and the UI will smoothly reflect changes.

**Example plugins:** [`todo/`](todo/handler.py) - Shows pending task count as chip

---

## FAB Override (Minimized Launcher Display)

When the launcher is minimized, it shows a floating action button (FAB) with the hamr icon and "hamr" text. Plugins can override this display to show dynamic content like timer countdowns, active task counts, or status indicators.

### Setting FAB Override via Status

Include a `fab` field in your status update:

```python
# In results/index response
print(json.dumps({
    "type": "results",
    "results": [...],
    "status": {
        "badges": [...],
        "fab": {
            "chips": [{"text": "04:32", "icon": "timer"}],
            "priority": 10
        }
    }
}))

# Daemon push update
emit({
    "type": "status",
    "status": {
        "fab": {
            "badges": [{"text": "3", "icon": "task_alt"}],
            "priority": 5
        }
    }
})
```

### FAB Override Fields

| Field | Type | Description |
|-------|------|-------------|
| `chips` | array | Chip widgets to display (same format as result chips) |
| `badges` | array | Badge widgets to display (same format as result badges) |
| `priority` | number | Priority for conflict resolution (higher wins, default: 0) |
| `showFab` | bool | If true and launcher is closed, force FAB visible (default: false) |

### Chip/Badge Format

Uses the same format as result items:

```python
# Chips - for text labels
{"text": "04:32", "icon": "timer", "color": "#4caf50"}

# Badges - for compact indicators
{"text": "3", "color": "#f44336"}
{"icon": "sync"}
```

### Priority Resolution

When multiple plugins set FAB overrides, the highest priority wins:

| Plugin | Priority | Shown |
|--------|----------|-------|
| Timer (active countdown) | 10 | Yes |
| Todo (pending count) | 5 | No |
| Media player (now playing) | 3 | No |

### Clearing FAB Override

To remove your plugin's FAB override, set `fab` to `null`:

```python
emit({
    "type": "status",
    "status": {
        "fab": null  # Clears this plugin's override
    }
})
```

### Showing FAB Automatically

Use `showFab: true` to make the FAB appear when the launcher is completely closed:

```python
# Timer starts - show FAB immediately
emit({
    "type": "status",
    "status": {
        "fab": {
            "chips": [{"text": "25:00", "icon": "timer"}],
            "priority": 10,
            "showFab": true  # Force FAB visible
        }
    }
})

# Timer tick - just update, don't force show
emit({
    "type": "status",
    "status": {
        "fab": {
            "chips": [{"text": "24:59", "icon": "timer"}],
            "priority": 10
            # No showFab - user may have closed it
        }
    }
})
```

### Use Cases

| Use Case | Content | Priority | showFab |
|----------|---------|----------|---------|
| Timer starts | `{"chips": [{"text": "25:00", "icon": "timer"}]}` | 10 | true |
| Timer tick | `{"chips": [{"text": "24:59", "icon": "timer"}]}` | 10 | false |
| Pomodoro session | `{"chips": [{"text": "Focus", "icon": "psychology"}]}` | 10 | true |
| Pending tasks | `{"badges": [{"text": "5"}]}` | 5 | false |
| Media now playing | `{"chips": [{"text": "Song Title"}]}` | 3 | false |
| Sync in progress | `{"badges": [{"icon": "sync"}]}` | 8 | false |

**Best practices:**
- Use `showFab: true` only on initial activation (timer start, not every tick)
- Use higher priority for time-sensitive content (timers, alarms)
- Clear override when activity completes
- Keep content concise (FAB has limited space)
- Use chips for text, badges for counts/icons

**Example plugin:** [`timer/`](timer/handler.py) - Countdown timer with FAB display

---

## Ambient Items (Persistent Status Bar)

Ambient items are persistent status displays shown in the action bar below the search input. They replace the shortcut hints when active and remain visible regardless of search query. Useful for showing ongoing activities like timers, downloads, or background tasks. When items overflow, they marquee/scroll automatically.

### Setting Ambient Items

Include an `ambient` array in your status update:

```python
emit({
    "type": "status",
    "status": {
        "ambient": [
            {
                "id": "timer-1",
                "name": "Focus Timer",
                "description": "24:32 remaining",
                "icon": "timer",
                "chips": [{"text": "Pomodoro"}],
                "badges": [{"icon": "pause"}],
                "actions": [
                    {"id": "pause", "icon": "pause", "name": "Pause"},
                    {"id": "stop", "icon": "stop", "name": "Stop"}
                ],
                "duration": 0
            }
        ]
    }
})
```

### Ambient Item Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier for the item |
| `name` | string | Primary text label |
| `description` | string | Secondary text (subtitle) |
| `icon` | string | Material icon name |
| `chips` | array | Chip widgets (same format as result chips) |
| `badges` | array | Badge widgets (same format as result badges) |
| `actions` | array | Action buttons (up to 3) |
| `duration` | number | Auto-remove after ms (0 = permanent until cleared) |

### Action Handling

When user clicks an action button on an ambient item:

```python
{
    "step": "action",
    "selected": {"id": "timer-1"},
    "action": "pause",
    "source": "ambient"
}
```

When user clicks the dismiss (X) button:

```python
{
    "step": "action",
    "selected": {"id": "timer-1"},
    "action": "__dismiss__",
    "source": "ambient"
}
```

### Clearing Ambient Items

```python
# Clear all ambient items for this plugin
emit({"type": "status", "status": {"ambient": null}})
```

### Multiple Ambient Items

Plugins can display multiple ambient items:

```python
emit({
    "type": "status",
    "status": {
        "ambient": [
            {"id": "timer-1", "name": "Focus", "description": "24:32", "icon": "timer"},
            {"id": "timer-2", "name": "Break", "description": "4:59", "icon": "coffee"}
        ]
    }
})
```

### Behavior

- **Visibility**: Only shown in main search view (hidden when inside a plugin)
- **Searchable**: No - ambient items are status displays, not search results
- **Dismissable**: Yes - user can click X to dismiss, plugin receives `__dismiss__` action
- **Auto-expire**: Set `duration` > 0 to auto-remove after specified milliseconds
- **Layout**: Compact single-line items in action bar, auto-scroll when overflowing
- **Replaces shortcuts**: When ambient items are shown, the shortcut hints are hidden

### Use Cases

| Use Case | Example |
|----------|---------|
| Active timer | `{"name": "Focus", "description": "24:32", "icon": "timer", "actions": [{"id": "pause", "icon": "pause"}]}` |
| Download progress | `{"name": "Downloading", "description": "45%", "icon": "download", "duration": 0}` |
| Background sync | `{"name": "Syncing", "description": "3 items", "icon": "sync", "duration": 0}` |
| Notification | `{"name": "Reminder", "description": "Meeting in 5 min", "icon": "event", "duration": 30000}` |

**Best practices:**
- Use `duration` for temporary notifications that should auto-dismiss
- Keep content concise (single line layout)
- Provide relevant actions (pause, stop, cancel)
- Handle `__dismiss__` to clean up resources (stop timers, cancel tasks)
- Clear ambient items when activity completes

**Example plugin:** [`timer/`](timer/handler.py) - Active timers in ambient bar with pause/stop actions

---

## Sound Effects

Hamr provides an audio service for playing sound effects. Sounds are useful for timer completions, notifications, errors, and other feedback.

### Playing Sounds from Plugins

Include the `sound` field in an `execute` response:

```python
{
    "type": "execute",
    "execute": {
        "sound": "alarm",              # Predefined sound name
        "notify": "Timer finished!",
        "close": true
    }
}

# Or with a custom sound file
{
    "type": "execute",
    "execute": {
        "sound": "/path/to/custom.wav",  # Absolute path
        "close": true
    }
}
```

### Predefined Sounds

| Sound | Description | Use Case |
|-------|-------------|----------|
| `alarm` | Alarm clock sound | Timer/alarm completion |
| `timer` | Timer completion | Pomodoro, countdown |
| `complete` | Success/completion | Task done, download finished |
| `notification` | Notification chime | Alerts, messages |
| `error` | Error sound | Failed operations |
| `warning` | Warning sound | Caution alerts |

### Sound Discovery Priority

Hamr searches for sounds in this order:

1. **User sounds**: `~/.config/hamr/sounds/` (custom sounds)
2. **Ocean theme**: `/usr/share/sounds/ocean/stereo/` (modern KDE sounds)
3. **Freedesktop**: `/usr/share/sounds/freedesktop/stereo/` (classic fallback)

### Custom Sounds

Place custom sound files in `~/.config/hamr/sounds/`:

```bash
~/.config/hamr/sounds/
├── alarm.wav       # Overrides predefined "alarm"
├── timer.ogg       # Overrides predefined "timer"
└── my-sound.mp3    # Custom sound, use path or name
```

Supported formats: `.oga`, `.ogg`, `.wav`, `.mp3`, `.flac`

### CLI Commands

```bash
# Play sounds
hamr audio play alarm              # Play predefined sound
hamr audio play /path/to/file.wav  # Play custom file

# Control
hamr audio status                  # Show available sounds
hamr audio enable                  # Enable sound effects
hamr audio disable                 # Disable sound effects
hamr audio reload                  # Re-discover sound files
```

### IPC Commands

```bash
# Direct IPC calls
qs ipc -c hamr call audio play alarm
qs ipc -c hamr call audio status
qs ipc -c hamr call audio enable
qs ipc -c hamr call audio disable
qs ipc -c hamr call audio reload
```

### Configuration

In `~/.config/hamr/config.json`:

```json
{
  "audio": {
    "enabled": true
  }
}
```

### Sound Theme Packages

For best experience, install a sound theme:

```bash
# Recommended (modern KDE Plasma sounds)
sudo pacman -S ocean-sound-theme

# Fallback (classic sounds)
sudo pacman -S sound-theme-freedesktop
```

---

## Plugin Indexing

Plugins can provide searchable items that appear in the main launcher search without needing to open the plugin first.

### Index-Only Plugins

Some plugins exist solely to provide indexed items - they have no interactive mode and shouldn't appear in the `/` plugin list. There are two ways to create index-only plugins:

#### 1. Static Index (No Handler)

For simple, static items defined directly in the manifest. No `handler.py` needed.

```json
{
  "name": "Theme",
  "description": "Switch between light and dark mode",
  "icon": "contrast",
  "staticIndex": [
    {
      "id": "dark",
      "name": "Dark Mode",
      "description": "Switch to dark color scheme",
      "icon": "dark_mode",
      "verb": "Switch",
      "keywords": ["dark", "theme", "night"],
      "execute": {
        "command": ["gsettings", "set", "org.gnome.desktop.interface", "color-scheme", "prefer-dark"],
        "name": "Dark Mode",
        "notify": "Dark mode activated",
        "close": true
      }
    }
  ]
}
```

**Use cases:** Theme switching, quick actions, launcher shortcuts

**Example plugins:** [`theme/`](theme/), [`accentcolor/`](accentcolor/), [`colorpick/`](colorpick/), [`snip/`](snip/)

#### 2. Index-Only Daemon

For dynamic items that need a handler (e.g., watching files, querying databases), but no interactive mode.

```json
{
  "name": "Zoxide",
  "description": "Jump to frequently used directories",
  "icon": "folder_special",
  "indexOnly": true,
  "daemon": {
    "enabled": true,
    "background": true
  }
}
```

The handler emits `index` responses but returns an error for other steps:

```python
def handle_request(input_data: dict) -> None:
    step = input_data.get("step", "initial")
    
    if step == "index":
        items = get_indexed_items()
        print(json.dumps({"type": "index", "items": items}))
        return
    
    # No interactive mode
    print(json.dumps({"type": "error", "message": "This plugin is index-only"}))
```

**Use cases:** Directory jumpers (zoxide), file indexers, database queries

**Example plugins:** [`zoxide/`](zoxide/)

### Enable Indexing in manifest.json

For plugins that have both interactive mode AND provide indexed items:

```json
{
  "name": "Apps",
  "description": "Browse and launch applications",
  "icon": "apps",
  "index": {
    "enabled": true
  }
}
```

### Handle the `index` Step

```python
if step == "index":
    items = get_all_items()
    print(json.dumps({
        "type": "index",
        "items": [
            {
                "id": "app:firefox",
                "name": "Firefox",
                "description": "Web Browser",
                "icon": "firefox",
                "iconType": "system",
                "keywords": ["browser", "web", "internet"],
                "appId": "firefox",
                "verb": "Open",
                "execute": {
                    "command": ["gio", "launch", "/usr/share/applications/firefox.desktop"]
                }
            }
        ]
    }))
    return
```

### Index Item Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier |
| `name` | string | Display name |
| `description` | string | Subtitle |
| `icon` | string | Icon name |
| `iconType` | string | `"material"` or `"system"` |
| `keywords` | string[] | Additional search terms |
| `appId` | string | App identifier (for grouping) |
| `verb` | string | Action text (e.g., "Open", "Focus") |
| `execute.command` | string[] | Command to run when selected |
| `execute.name` | string | Display name for history tracking |
| `execute.notify` | string | Notification message after execution |
| `execute.close` | bool | Close launcher after execution (default: false) |
| `keepOpen` | bool | Keep launcher open after execution (default: false) |
| `actions` | array | Secondary action buttons |
| `entryPoint` | object | Entry point for plugin drill-down (see below) |

### Index Item Actions

Actions on index items can include direct execution commands or entry points for plugin navigation:

```python
"actions": [
    # Direct command execution (no handler invocation)
    {
        "id": "copy",
        "name": "Copy",
        "icon": "content_copy",
        "command": ["wl-copy", "text to copy"],  # Runs directly
    },
    # Entry point (invokes handler)
    {
        "id": "edit",
        "name": "Edit",
        "icon": "edit",
        "entryPoint": {
            "step": "action",
            "selected": {"id": "item-id"},
            "action": "edit",
        },
        "keepOpen": True,  # Keep launcher open
    },
]
```

| Action Field | Type | Description |
|--------------|------|-------------|
| `id` | string | Action identifier |
| `name` | string | Action label |
| `icon` | string | Material icon |
| `command` | string[] | Direct shell command (no handler) |
| `entryPoint` | object | Handler entry point for complex actions |
| `keepOpen` | bool | Keep launcher open after action |

### Index Item `entryPoint`

For indexed items that need to open the plugin UI (e.g., view details, edit):

```python
{
    "id": "note:123",
    "name": "My Note",
    "icon": "sticky_note_2",
    "verb": "View",
    # Opens plugin with this entry point instead of executing
    "entryPoint": {
        "step": "action",
        "selected": {"id": "123"},
        "action": "view",
    },
    "keepOpen": True,  # Required for entryPoint to work
}
```

**Example plugins:** [`notes/`](notes/handler.py), [`bitwarden/`](bitwarden/handler.py)

### Incremental Indexing

For efficient updates, plugins can support incremental indexing. Instead of returning all items, return only new and removed items.

**Input fields for incremental mode:**

```python
{
    "step": "index",
    "mode": "incremental",           # "full" (default) or "incremental"
    "indexedIds": ["id1", "id2"],    # Previously indexed item IDs
}
```

**Handler implementation:**

```python
if step == "index":
    mode = input_data.get("mode", "full")
    indexed_ids = set(input_data.get("indexedIds", []))
    
    # Get current items
    current_items = get_all_items()
    current_ids = {item["id"] for item in current_items}
    
    if mode == "incremental" and indexed_ids:
        # Find new items (in current but not previously indexed)
        new_ids = current_ids - indexed_ids
        new_items = [item for item in current_items if item["id"] in new_ids]
        
        # Find removed items (previously indexed but no longer exist)
        removed_ids = list(indexed_ids - current_ids)
        
        print(json.dumps({
            "type": "index",
            "mode": "incremental",
            "items": new_items,
            "remove": removed_ids,  # IDs to remove from index
        }))
    else:
        # Full reindex
        print(json.dumps({
            "type": "index",
            "items": [item_to_index(i) for i in current_items],
        }))
    return
```

**Response fields:**

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | Always `"index"` |
| `mode` | string | `"incremental"` for delta updates |
| `items` | array | New items to add to index |
| `remove` | array | Item IDs to remove from index |

**Example plugins:** [`clipboard/`](clipboard/handler.py), [`apps/`](apps/handler.py), [`bitwarden/`](bitwarden/handler.py)

### Automatic Reindexing

**⚠️ Deprecated:** The `watchFiles`, `watchDirs`, and `watchHyprlandEvents` manifest options are deprecated. Plugins should instead use [Daemon Mode](#daemon-mode-persistent-processes) with native inotify file watching. This gives plugins full control over caching and more efficient event handling.

#### Periodic Reindexing

Reindex on a schedule (for plugins that don't need instant updates):

```json
{
  "index": {
    "enabled": true,
    "reindex": "5m"
  }
}
```

Values: `"30s"`, `"5m"`, `"1h"`, `"never"` (disable periodic reindex)

### Manual Reindexing via IPC

```bash
# Reindex a specific plugin
qs -c hamr ipc call pluginRunner reindex apps

# Reindex all plugins
qs -c hamr ipc call pluginRunner reindexAll
```

**Example plugins:** [`apps/`](apps/handler.py), [`windows/`](windows/handler.py)

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
            "placeholder": "Search items...",
            "pluginActions": [  # Optional: toolbar buttons
                {"id": "add", "name": "Add", "icon": "add_circle"},
            ]
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

        # Plugin-level action (from toolbar, Ctrl+1)
        if item_id == "__plugin__" and action == "add":
            # Handle add action...
            print(json.dumps({
                "type": "results",
                "results": [...],
                "inputMode": "submit",
                "placeholder": "Type new item name...",
                "pluginActions": []  # Hide actions during add mode
            }))
            return

        # Back navigation - only needed for plugins with nested views
        # For flat plugins (no drill-down), you can omit this
        if item_id == "__back__":
            print(json.dumps({
                "type": "results",
                "results": get_initial_results(),
                "clearInput": True,
                "context": "",
                "navigationDepth": 0  # Return to root
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

### Bash Handler Template

```bash
#!/bin/bash
# handler (no extension, executable)

INPUT=$(cat)
STEP=$(echo "$INPUT" | jq -r '.step // "initial"')
QUERY=$(echo "$INPUT" | jq -r '.query // ""')
SELECTED_ID=$(echo "$INPUT" | jq -r '.selected.id // ""')

case "$STEP" in
    initial)
        cat << 'EOF'
{"type": "results", "results": [
    {"id": "item1", "name": "First Item", "icon": "star"},
    {"id": "item2", "name": "Second Item", "icon": "favorite"}
]}
EOF
        ;;
    action)
        echo '{"type": "execute", "execute": {"command": ["notify-send", "Selected: '"$SELECTED_ID"'"], "close": true}}'
        ;;
esac
```

### Node.js Handler Template

```javascript
#!/usr/bin/env node
// handler.js (executable)

const fs = require('fs');
const input = JSON.parse(fs.readFileSync(0, 'utf-8'));
const { step = 'initial', query = '', selected = {}, action = '' } = input;

if (step === 'initial') {
    console.log(JSON.stringify({
        type: 'results',
        results: [
            { id: 'item1', name: 'First Item', icon: 'star' },
            { id: 'item2', name: 'Second Item', icon: 'favorite' }
        ]
    }));
} else if (step === 'action') {
    console.log(JSON.stringify({
        type: 'execute',
        execute: { command: ['notify-send', `Selected: ${selected.id}`], close: true }
    }));
}
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

| Plugin                             | Trigger          | Features                            | Key Patterns                                   |
| ---------------------------------- | ---------------- | ----------------------------------- | ---------------------------------------------- |
| [`apps/`](apps/)                   | `/apps`          | App drawer with categories          | System icons, category navigation, indexing    |
| [`windows/`](windows/)             | `/windows`       | Switch between open windows         | Hyprland events, indexing, window focus        |
| [`files/`](files/)                 | `~`              | File search with fd+fzf, thumbnails | Results with thumbnails, action buttons        |
| [`clipboard/`](clipboard/)         | `;`              | Clipboard history with images       | Image thumbnails, pluginActions with confirm   |
| [`shell/`](shell/)                 | `!`              | Shell command history               | Simple results, execute commands               |
| [`bitwarden/`](bitwarden/)         | `/bitwarden`     | Password manager                    | entryPoint replay, cache, error cards          |
| [`quicklinks/`](quicklinks/)       | `/quicklinks`    | Web search quicklinks               | Submit mode, context, CRUD, pluginActions      |
| [`dict/`](dict/)                   | `/dict`          | Dictionary lookup                   | Card response, API fetch                       |
| [`pictures/`](pictures/)           | `/pictures`      | Image browser                       | Thumbnails, multi-turn navigation              |
| [`screenshot/`](screenshot/)       | `/screenshot`    | Screenshot browser                  | imageBrowser, enableOcr                        |
| [`screenrecord/`](screenrecord/)   | `/screenrecord`  | Screen recorder                     | Launch timestamp API, ffmpeg trim              |
| [`snippet/`](snippet/)             | `/snippet`       | Text snippets                       | Submit mode, pluginActions                     |
| [`todo/`](todo/)                   | `/todo`          | Todo list                           | Daemon mode, status chips, CRUD, pluginActions |
| [`wallpaper/`](wallpaper/)         | `/wallpaper`     | Wallpaper selector                  | imageBrowser, history tracking                 |
| [`create-plugin/`](create-plugin/) | `/create-plugin` | AI plugin creator                   | OpenCode integration                           |
| [`notes/`](notes/)                 | `/notes`         | Quick notes manager                 | Form API, pluginActions                        |
| [`player/`](player/)               | `/player`        | Media player controls               | playerctl, indexed actions                     |
| [`sound/`](sound/)                 | `/sound`         | System volume controls              | wpctl, keepOpen, indexed actions               |
| [`timer/`](timer/)                 | `/timer`         | Countdown timers with presets       | Daemon, FAB override, ambient items, sounds    |
| [`topcpu/`](topcpu/)               | `/topcpu`        | Process monitor (CPU)               | Daemon mode, process management                |
| [`topmem/`](topmem/)               | `/topmem`        | Process monitor (memory)            | Daemon mode, process management                |

---

## Keyboard Navigation

Users navigate with:

- **Ctrl+J/K** - Move down/up
- **Ctrl+L** or **Enter** - Select
- **Escape** - Go back one step (single) / close plugin entirely (double-tap)
- **Ctrl+1 through Ctrl+6** - Execute plugin actions (toolbar buttons)
- **Tab / Shift+Tab** - Cycle through item action buttons
- **Ctrl+Shift+H/L** - Decrease/increase slider value (for slider items)

### Back Navigation

Hamr tracks navigation depth automatically and provides a Back button in the UI.

**Escape key behavior:**
- **Single Escape**: Go back one level (sends `__back__` to handler)
- **Double Escape** (within 300ms): Close plugin entirely, regardless of depth
- **At initial view** (depth = 0): Single Escape closes the plugin

**Back button behavior:**
- Same as single Escape - goes back one level

**Navigation depth increases when:**
- Handler returns `navigateForward: true`

**Navigation depth decreases when:**
- Handler returns `navigateBack: true`, OR
- Handler returns `navigationDepth: N` (sets absolute depth)

**Navigation depth does NOT change when:**
- No navigation flags are set (action modified view, didn't navigate)
- Execute responses (close or stay open, don't affect depth)

**Explicit navigation control:**
```python
# Drill down into sub-view (depth +1)
{"type": "results", "results": [...], "navigateForward": True}

# Return to parent view (depth -1)
{"type": "results", "results": [...], "navigateBack": True}

# Jump to specific depth (e.g., breadcrumb click to go back multiple levels)
{"type": "results", "results": [...], "navigationDepth": 0}  # Jump to root
{"type": "results", "results": [...], "navigationDepth": 2}  # Jump to level 2

# Explicitly prevent navigation (for in-place updates like toggle, sync, filter)
{"type": "results", "results": [...], "navigateForward": False}
```

**When to use `navigateForward: False`:**

Use this when an action modifies the view but should NOT increase navigation depth:
- Toggling item state (e.g., todo done/undone)
- Applying filters (e.g., show only images)
- Refreshing/syncing data
- Any action where pressing Back should NOT undo the action

```python
# Example: Toggle filter without affecting navigation
if action == "filter_images":
    new_filter = "" if context == "images" else "images"
    print(json.dumps({
        "type": "results",
        "results": get_filtered_results(new_filter),
        "context": new_filter,
        "navigateForward": False,  # Don't push to navigation stack
    }))
```

**Example plugins:** [`clipboard/`](clipboard/handler.py) - Filter toggle, [`todo/`](todo/handler.py) - Task toggle, [`bitwarden/`](bitwarden/handler.py) - Vault sync

**For plugins with nested views** (e.g., folder browser, category drill-down), handle `__back__` to return to the previous view:

```python
if step == "action":
    item_id = selected.get("id", "")

    # Handle back navigation (sent by Escape key or Back button)
    if item_id == "__back__":
        # Calculate previous level from context
        prev_level = calculate_previous_level(context)
        prev_results = get_results_for_level(prev_level)
        
        print(json.dumps({
            "type": "results",
            "results": prev_results,
            "context": get_context_for_level(prev_level),
            "navigationDepth": prev_level,  # Set exact depth
            "clearInput": True,
        }))
        return
    
    # Drill down into sub-view
    if item_id.startswith("folder:"):
        print(json.dumps({
            "type": "results",
            "results": get_folder_contents(item_id),
            "context": f"{level + 1}:{path}",
            "navigateForward": True,  # Increment depth
            "clearInput": True,
        }))
        return
```

**For flat plugins** (e.g., clipboard, todo), you don't need to handle `__back__`. The UI will close the plugin when depth is 0.

**For nested plugins** (e.g., apps with categories, file browser):
- Set `navigateForward: true` when drilling into a sub-view
- Set `navigationDepth: N` in `__back__` handler to set exact depth (or use `navigateBack: true` to decrement by 1)

**Important:** Don't add explicit `__back__` items to your results list. Hamr provides a Back button in the UI automatically.

---

## Special Item IDs

Hamr reserves certain item ID prefixes for special handling:

| ID Pattern | Sent By | Purpose |
|------------|---------|---------|
| `__back__` | Hamr | User pressed Escape or Back button |
| `__plugin__` | Hamr | Plugin action button clicked (with `action` field) |
| `__form_cancel__` | Hamr | User cancelled a form dialog |
| `__empty__` | Handler | Non-actionable placeholder for empty state |
| `__info__` | Handler | Non-actionable informational item |
| `__add__` | Handler | Convention for "add new item" action |

**Empty state placeholder:**

```python
if not results:
    results = [{
        "id": "__empty__",
        "name": "No items found",
        "icon": "search_off",
        "description": "Try a different search term",
    }]

# In action step, ignore or close
if item_id == "__empty__":
    print(json.dumps({"type": "execute", "execute": {"close": True}}))
    return
```

**Example plugins:** [`todo/`](todo/handler.py), [`apps/`](apps/handler.py), [`hyprland/`](hyprland/handler.py)

---

## Tips

1. **Handle `__back__`** - Return to previous/initial view when user presses Escape or clicks Back
2. **Don't add `__back__` to results** - Hamr's UI provides a Back button automatically
3. **Use `close: true`** only for final actions
4. **Keep results under 50** - Performance
5. **Use thumbnails sparingly** - They load images
6. **Use `placeholder`** - Helps users know what to type
7. **Use `context`** - Preserve state across search calls
8. **Use `pluginActions`** - Move common actions (Add, Wipe) to the toolbar
9. **Use `confirm`** - Require confirmation for dangerous plugin actions
10. **Debug with** `journalctl --user -f` - Check for errors
11. **Test edge cases** - Empty results, errors, special characters

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
| `gridBrowser`   | `type`, `gridBrowser.items[]` with `id` and `name` |
| `form`          | `type`, `form.fields[]` with `id`, `type`|
| `prompt`        | `type`, `prompt` object                  |
| `error`         | `type`, `message`                        |
| `noop`          | `type` only                              |

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
