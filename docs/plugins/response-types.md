# Response Types

Every handler response must include a `type` field. This page documents all available response types.

## Quick Reference

| Type | Purpose | When to Use |
|------|---------|-------------|
| [`results`](#results) | Show a list of items | Search results, menus, lists |
| [`execute`](#execute) | Run an action | Open files, copy text, close launcher |
| [`card`](#card) | Show rich content | Markdown text, definitions, details |
| [`form`](#form) | Show input form | Multi-field input, settings |
| [`imageBrowser`](#imagebrowser) | Image grid browser | Wallpapers, screenshots |
| [`gridBrowser`](#gridbrowser) | Generic grid | Emojis, icons |
| [`prompt`](#prompt) | Simple text prompt | Initial input request |
| [`error`](#error) | Show error message | Something went wrong |
| [`update`](#update) | Patch existing items | Slider adjustments, live updates |
| [`index`](#index) | Provide searchable items | Main search integration |
| [`noop`](#noop) | No UI change | Background operations |

---

## `results`

Display a list of selectable items. This is the most common response type.

![SCREENSHOT: results-basic.png - Basic results list with items showing icon, name, and description]

```python
{
    "type": "results",
    "results": [
        {
            "id": "unique-id",           # Required: identifies the item
            "name": "Display Name",      # Required: main text
            "description": "Subtitle",   # Optional: shown below name
            "icon": "star",              # Optional: Material icon name
            "iconType": "material",      # Optional: "material" (default) or "system"
            "thumbnail": "/path/to/img", # Optional: image preview (overrides icon)
            "verb": "Open",              # Optional: action text on hover
            "actions": [                 # Optional: secondary action buttons
                {"id": "copy", "name": "Copy", "icon": "content_copy"}
            ]
        }
    ],
    "placeholder": "Search...",          # Optional: search bar hint
    "inputMode": "realtime",             # Optional: "realtime" or "submit"
    "clearInput": true,                  # Optional: clear search text
    "context": "my-state",               # Optional: persist state
    "notify": "Action done",             # Optional: show notification
    "pluginActions": [...]               # Optional: toolbar buttons
}
```

### Result Item Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | **Required.** Unique identifier sent back on selection |
| `name` | string | **Required.** Primary display text |
| `description` | string | Secondary text below name |
| `icon` | string | Material icon name (e.g., `"star"`, `"folder"`) |
| `iconType` | string | `"material"` (default) or `"system"` (desktop app icons) |
| `thumbnail` | string | Image file path (overrides icon) |
| `verb` | string | Action text shown on hover (e.g., "Open", "Copy") |
| `actions` | array | Secondary action buttons (up to 4) |
| `badges` | array | Compact circular indicators (up to 5) |
| `chips` | array | Pill-shaped tags |
| `graph` | object | Line graph data (replaces icon) |
| `gauge` | object | Circular progress (replaces icon) |
| `progress` | object | Horizontal progress bar (replaces description) |
| `preview` | object | Side panel preview content |

### Response-Level Fields

| Field | Type | Description |
|-------|------|-------------|
| `placeholder` | string | Search bar placeholder text |
| `inputMode` | string | `"realtime"` (every keystroke) or `"submit"` (on Enter) |
| `clearInput` | bool | Clear the search bar |
| `context` | string | Custom state persisted across search calls |
| `notify` | string | Show notification toast |
| `pluginActions` | array | Toolbar buttons below search bar |
| `navigateForward` | bool | Increase navigation depth |
| `navigateBack` | bool | Decrease navigation depth |
| `navigationDepth` | int | Set exact navigation depth |

### Action Buttons

![SCREENSHOT: results-with-actions.png - Item showing action buttons on hover]

```python
"actions": [
    {"id": "edit", "name": "Edit", "icon": "edit"},
    {"id": "delete", "name": "Delete", "icon": "delete"}
]
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | Action identifier sent to handler |
| `name` | string | Yes | Button label/tooltip |
| `icon` | string | No | Material icon |
| `entryPoint` | object | No | For indexed items only (see below) |

When user clicks an action button, you receive:
```python
{
    "step": "action",
    "selected": {"id": "item-id"},
    "action": "edit"  # The action button ID
}
```

#### Actions on Indexed Items

**Note:** `entryPoint` is only needed for plugins with indexing enabled (`index.enabled: true` in manifest). For regular plugins without indexing, you don't need `entryPoint` - hamr builds the request directly from click context.

For indexed items that appear in main search, actions need an `entryPoint` so hamr knows how to invoke your handler:

```python
"actions": [
    {
        "id": "copy",
        "name": "Copy",
        "icon": "content_copy",
        "entryPoint": {
            "step": "action",
            "selected": {"id": "item-id"},
            "action": "copy"
        }
    }
]
```

See [Plugin Indexing](advanced-features.md#plugin-indexing) for details.

### Plugin Actions (Toolbar)

![SCREENSHOT: plugin-actions.png - Toolbar buttons below search bar]

```python
"pluginActions": [
    {"id": "add", "name": "Add", "icon": "add_circle"},
    {"id": "wipe", "name": "Wipe All", "icon": "delete_sweep", "confirm": "Are you sure?"}
]
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | Action ID sent to handler |
| `name` | string | Yes | Button label |
| `icon` | string | No | Material icon |
| `shortcut` | string | No | Keyboard shortcut hint (default: Ctrl+1-6) |
| `confirm` | string | No | Confirmation dialog message |
| `active` | bool | No | Highlight as active (for toggles) |

When clicked, you receive:
```python
{
    "step": "action",
    "selected": {"id": "__plugin__"},  # Special ID for plugin actions
    "action": "add"
}
```

**Example plugins:** [`quicklinks/`](../../plugins/quicklinks/), [`todo/`](../../plugins/todo/), [`clipboard/`](../../plugins/clipboard/)

---

## `execute`

Execute an action and optionally close the launcher.

```python
# Simple execution
{
    "type": "execute",
    "notify": "Done!",
    "close": true
}

# Launch application
{
    "type": "execute",
    "launch": "/usr/share/applications/firefox.desktop",
    "close": true
}

# Copy to clipboard
{
    "type": "execute",
    "copy": "text to copy",
    "notify": "Copied!",
    "close": true
}

# Open URL
{
    "type": "execute",
    "openUrl": "https://example.com",
    "close": true
}

# Open file/folder
{
    "type": "execute",
    "open": "/path/to/file",
    "close": true
}

# Type text (snippet expansion)
{
    "type": "execute",
    "typeText": "expanded text",
    "close": true
}
```

### Execute Fields

| Field | Type | Description |
|-------|------|-------------|
| `launch` | string | Desktop file path (runs `gio launch`) |
| `copy` | string | Text to copy (runs `wl-copy`) |
| `typeText` | string | Text to type (uses `ydotool`) |
| `openUrl` | string | URL to open (Qt.openUrlExternally) |
| `open` | string | File/folder path (runs `xdg-open`) |
| `notify` | string | Notification message |
| `sound` | string | Sound effect name (see [Sound Effects](#sound-effects)) |
| `close` | bool | Close launcher after execution |

### Running Custom Commands

For commands not covered by execute fields, run them in your handler:

```python
import subprocess

# Execute in handler
subprocess.Popen(["my-command", "arg1"], start_new_session=True)

# Return close response
print(json.dumps({"type": "execute", "close": True}))
```

**Example plugins:** [`files/`](../../plugins/files/), [`bitwarden/`](../../plugins/bitwarden/)

---

## `card`

Display markdown-formatted content with optional actions.

![SCREENSHOT: card-view.png - Card showing markdown content with action buttons]

```python
{
    "type": "card",
    "card": {
        "title": "Word Definition",
        "content": "**noun**\n\nA thing used for...",
        "markdown": true,
        "actions": [
            {"id": "copy", "name": "Copy", "icon": "content_copy"},
            {"id": "back", "name": "Back", "icon": "arrow_back"}
        ]
    },
    "context": "word-id",              # Preserve state for action handling
    "inputMode": "submit",             # Optional: wait for Enter
    "placeholder": "Type reply..."     # Optional: input hint
}
```

When user clicks a card action:
```python
{
    "step": "action",
    "selected": {"id": "word-id"},     # From context
    "action": "copy"
}
```

**Example plugins:** [`dict/`](../../plugins/dict/), [`notes/`](../../plugins/notes/)

---

## `form`

Display a multi-field input dialog.

![SCREENSHOT: form-dialog.png - Form with text fields, textarea, and submit button]

```python
{
    "type": "form",
    "form": {
        "title": "Add Note",
        "submitLabel": "Save",         # Default: "Submit"
        "cancelLabel": "Cancel",       # Default: "Cancel"
        "fields": [
            {
                "id": "title",
                "type": "text",
                "label": "Title",
                "placeholder": "Enter title...",
                "required": true
            },
            {
                "id": "content",
                "type": "textarea",
                "label": "Content",
                "rows": 6
            }
        ]
    },
    "context": "__add__"               # Passed to form submission
}
```

### Field Types

| Type | Description | Extra Fields |
|------|-------------|--------------|
| `text` | Single-line input | `placeholder`, `required`, `default`, `hint` |
| `textarea` | Multi-line input | `placeholder`, `required`, `default`, `rows`, `hint` |
| `email` | Email with validation | `placeholder`, `required`, `default`, `hint` |
| `password` | Masked input | `placeholder`, `required`, `hint` |
| `hidden` | Hidden field | `value` (required) |
| `select` | Dropdown | `options` (required), `default`, `hint` |
| `checkbox` | Checkbox | `default` (bool), `hint` |
| `switch` | Toggle switch | `default` (bool), `hint` |
| `slider` | Range slider | `min`, `max`, `step`, `unit`, `default`, `hint` |

### Select Options

```python
{
    "id": "theme",
    "type": "select",
    "label": "Theme",
    "options": [
        {"id": "light", "name": "Light"},
        {"id": "dark", "name": "Dark"},
        {"id": "system", "name": "System Default"}
    ],
    "default": "system"
}
```

### Form Submission

When user submits:
```python
{
    "step": "form",
    "formData": {
        "title": "My Note",
        "content": "Note content..."
    },
    "context": "__add__"
}
```

When user cancels:
```python
{
    "step": "action",
    "selected": {"id": "__form_cancel__"},
    "context": "__add__"
}
```

### Live Update Forms

For forms where changes apply immediately (no submit button):

```python
{
    "type": "form",
    "form": {
        "title": "Settings",
        "liveUpdate": true,
        "fields": [
            {"id": "opacity", "type": "slider", "label": "Opacity", "min": 0, "max": 1, "step": 0.05}
        ]
    }
}
```

On slider change, you receive `step: "formSlider"` with `fieldId` and `value`.

**Example plugins:** [`notes/`](../../plugins/notes/), [`bitwarden/`](../../plugins/bitwarden/), [`settings/`](../../plugins/settings/)

---

## `imageBrowser`

Open a rich image browser with thumbnails and directory navigation.

![SCREENSHOT: image-browser.png - Grid of image thumbnails with sidebar]

```python
{
    "type": "imageBrowser",
    "imageBrowser": {
        "directory": "~/Pictures/Wallpapers",
        "title": "Select Wallpaper",
        "enableOcr": false,            # Enable text search via OCR
        "actions": [
            {"id": "set_dark", "name": "Set (Dark)", "icon": "dark_mode"},
            {"id": "set_light", "name": "Set (Light)", "icon": "light_mode"}
        ]
    }
}
```

When user selects an image:
```python
{
    "step": "action",
    "selected": {
        "id": "imageBrowser",
        "path": "/full/path/to/image.jpg",
        "action": "set_dark"
    }
}
```

**Example plugins:** [`wallpaper/`](../../plugins/wallpaper/), [`screenshot/`](../../plugins/screenshot/)

---

## `gridBrowser`

Display items in a grid layout. Ideal for emojis, icons, or large item sets.

![SCREENSHOT: grid-browser.png - Grid of emoji items]

```python
{
    "type": "gridBrowser",
    "gridBrowser": {
        "title": "Select Emoji",
        "columns": 10,
        "cellAspectRatio": 1.0,
        "items": [
            {
                "id": "smile",
                "name": "grinning face",
                "icon": "smile",
                "iconType": "text",
                "keywords": ["happy", "smile"]
            }
        ],
        "actions": [
            {"id": "copy", "name": "Copy", "icon": "content_copy"}
        ]
    }
}
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `columns` | int | 8 | Number of columns |
| `cellAspectRatio` | float | 1.0 | Width/height ratio |
| `items` | array | required | Grid items |
| `actions` | array | [] | Action buttons |

When user selects:
```python
{
    "step": "action",
    "selected": {
        "id": "gridBrowser",
        "itemId": "smile",
        "action": "copy"
    }
}
```

**Example plugin:** [`emoji/`](../../plugins/emoji/)

---

## `prompt`

Display a simple text prompt. Typically used on initial load.

```python
{
    "type": "prompt",
    "prompt": {"text": "Enter word to define..."}
}
```

**Example plugin:** [`dict/`](../../plugins/dict/)

---

## `error`

Display an error message.

```python
{
    "type": "error",
    "message": "Failed to connect to API"
}
```

---

## `update`

Patch individual items without replacing the entire results array. Preserves selection and focus.

```python
{
    "type": "update",
    "items": [
        {
            "id": "volume",            # Item to update (matched by id)
            "gauge": {"value": 75, "max": 100, "label": "75%"},
            "icon": "volume_up"
        }
    ]
}
```

**Use cases:**
- Slider value changes
- Live status updates
- Badge/gauge updates

**Example plugin:** [`sound/`](../../plugins/sound/)

---

## `index`

Provide items for main search integration. See [Plugin Indexing](advanced-features.md#plugin-indexing).

```python
{
    "type": "index",
    "items": [
        {
            "id": "app:firefox",
            "name": "Firefox",
            "icon": "firefox",
            "iconType": "system",
            "keywords": ["browser", "web"],
            "execute": {
                "launch": "/usr/share/applications/firefox.desktop"
            }
        }
    ]
}
```

---

## `noop`

Signal that the action was handled but no UI update is needed.

```python
{
    "type": "noop"
}
```

**Use cases:**
- Slider adjustments (UI already shows new value)
- Background operations
- Toggle states with immediate visual feedback

---

## Input Modes

The `inputMode` field controls when search queries are sent:

| Mode | Behavior | Use Case |
|------|----------|----------|
| `realtime` | Every keystroke triggers search | Filtering, fuzzy search |
| `submit` | Only Enter triggers search | Text input, forms |

```python
# Realtime: filter as user types
{
    "type": "results",
    "results": filtered_items,
    "inputMode": "realtime"
}

# Submit: wait for Enter
{
    "type": "results",
    "results": [],
    "inputMode": "submit",
    "placeholder": "Type new item name, press Enter..."
}
```

---

## Context Persistence

Use `context` to maintain state across search calls:

```python
# Enter edit mode
if action == "edit":
    print(json.dumps({
        "type": "results",
        "context": f"__edit__:{item_id}",
        "inputMode": "submit",
        "placeholder": "Type new value..."
    }))

# Handle edit in search step
if step == "search" and context.startswith("__edit__:"):
    item_id = context.split(":")[1]
    save_item(item_id, query)
```

---

## Navigation

Hamr tracks navigation depth to show breadcrumbs and enable back navigation. The depth determines how many "levels deep" the user is in your plugin.

### Automatic Navigation

Hamr sets `pendingNavigation` automatically based on user interaction:

| User Action | Hamr Behavior |
|-------------|---------------|
| Click item (no action button) | Sets `pendingNavigation=true` → depth +1 |
| Click action button | No pending navigation → depth unchanged |
| Click `__back__` | Sets `pendingBack=true` → depth -1 |

If your response doesn't include navigation fields, Hamr applies the pending state automatically.

### Overriding Navigation

Use these response fields to explicitly control depth:

```python
# Drill into sub-view (depth +1)
{"type": "results", "navigateForward": True, ...}

# Return to parent (depth -1)  
{"type": "results", "navigateBack": True, ...}

# Jump to specific depth
{"type": "results", "navigationDepth": 0, ...}  # Root

# Prevent navigation (stay at current depth)
{"type": "results", "navigateForward": False, ...}
```

### When to Use `navigateForward: False`

**Important:** When an action modifies data but should stay on the same view, you must explicitly set `navigateForward: False`. Otherwise, Hamr's automatic navigation will increase the depth.

Common scenarios requiring `navigateForward: False`:

```python
# Toggle a todo item's done state
if action == "toggle":
    todos[idx]["done"] = not todos[idx]["done"]
    save_todos(todos)
    print(json.dumps({
        "type": "results",
        "results": get_todo_results(todos),
        "navigateForward": False  # Stay on same view
    }))

# Delete an item from the list
if action == "delete":
    del items[idx]
    save_items(items)
    print(json.dumps({
        "type": "results", 
        "results": get_items(),
        "navigateForward": False  # Stay on same view
    }))

# Sync/refresh data
if action == "sync":
    refresh_data()
    print(json.dumps({
        "type": "results",
        "results": get_results(),
        "navigateForward": False  # Stay on same view
    }))
```

Without `navigateForward: False`, these actions would incorrectly push a new navigation level, causing confusing breadcrumb behavior.

### Back Button

When user presses Escape or clicks Back:
```python
{
    "step": "action",
    "selected": {"id": "__back__"}
}
```

Handle it to return to the previous view:
```python
if selected.get("id") == "__back__":
    print(json.dumps({
        "type": "results",
        "results": get_parent_view()
        # navigateBack is automatic for __back__
    }))
```

**Example plugins:** [`todo/`](../../plugins/todo/), [`bitwarden/`](../../plugins/bitwarden/), [`clipboard/`](../../plugins/clipboard/)

---

## Sound Effects

Include sounds in execute responses:

```python
{
    "type": "execute",
    "sound": "complete",
    "notify": "Done!",
    "close": true
}
```

| Sound | Use Case |
|-------|----------|
| `alarm` | Timer/alarm completion |
| `timer` | Pomodoro, countdown |
| `complete` | Task done |
| `notification` | Alerts |
| `error` | Failed operations |
| `warning` | Caution alerts |

Custom sounds: Place in `~/.config/hamr/sounds/` (supports `.wav`, `.ogg`, `.mp3`, `.flac`)
