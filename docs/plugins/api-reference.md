# API Reference

Complete schema reference for Hamr plugin development.

## Manifest Schema

The `manifest.json` file defines your plugin's metadata and capabilities.

### Required Fields

| Field                | Type   | Description                                                           |
| -------------------- | ------ | --------------------------------------------------------------------- |
| `name`               | string | Display name shown in launcher                                        |
| `description`        | string | Short description                                                     |
| `icon`               | string | Material icon name                                                    |
| `supportedPlatforms` | array  | `["niri", "hyprland"]`, `["macos"]`, `["windows"]`, etc. (list all explicitly) |

### Optional Fields

| Field          | Type   | Default      | Description                                             |
| -------------- | ------ | ------------ | ------------------------------------------------------- |
| `handler`      | object | see below    | Handler configuration (type, command)                   |
| `frecency`     | string | `"item"`     | Usage tracking: `"item"`, `"plugin"`, or `"none"`       |
| `inputMode`    | string | `"realtime"` | Default input mode: `"realtime"` or `"submit"`          |
| `hidden`       | bool   | `false`      | Hide plugin from plugin list (prefix-only access)       |
| `staticIndex`  | array  | -            | Static index items defined in manifest (see below)      |
| `match`        | object | -            | Pattern matching configuration (see below)              |
| `matchPattern` | string | -            | Legacy single regex pattern (use `match` instead)       |

### Handler Configuration

The `handler` field specifies how the plugin communicates with hamr:

**stdio handler (default):**

```json
{
  "handler": {
    "type": "stdio"
  }
}
```

For stdio plugins, Hamr runs `handler.py` in the plugin directory by default, so the `handler` field can be omitted entirely.

**socket handler:**

```json
{
  "handler": {
    "type": "socket",
    "command": "python3 handler.py"
  }
}
```

| Field     | Type   | Default   | Description                               |
| --------- | ------ | --------- | ----------------------------------------- |
| `type`    | string | `"stdio"` | `"stdio"` or `"socket"`                   |
| `path`    | string | -         | Reserved for stdio handlers; Hamr runs `handler.py` by default |
| `command` | string | -         | Command to run the handler (for socket)   |

**Handler Types:**

| Type     | Description                                                    | Use Case                              |
| -------- | -------------------------------------------------------------- | ------------------------------------- |
| `stdio`  | JSON over stdin/stdout, new process per request or daemon loop | Simple plugins, stateless operations  |
| `socket` | JSON-RPC 2.0 over Unix socket, persistent connection           | Complex daemons, real-time updates    |

**Note:** For `stdio` handlers, if `daemon.enabled: true`, the handler runs as a persistent process reading JSON lines from stdin. For `socket` handlers, the handler connects to hamr's socket and uses JSON-RPC 2.0.

### Daemon Configuration

```json
{
  "daemon": {
    "enabled": true,
    "background": true,
    "restartOnCrash": true,
    "maxRestarts": 5
  }
}
```

| Field                 | Type | Default | Description                                            |
| --------------------- | ---- | ------- | ------------------------------------------------------ |
| `daemon.enabled`      | bool | `false` | Enable persistent daemon mode                          |
| `daemon.background`   | bool | `false` | Run always (`true`) or only when plugin open (`false`) |
| `daemon.restartOnCrash` | bool | `false` | Automatically restart daemon if it crashes           |
| `daemon.maxRestarts`  | int  | `0`     | Maximum restart attempts (0 = unlimited)               |

### Index Configuration

```json
{
  "index": {
    "enabled": true
  }
}
```

| Field           | Type | Default | Description                                      |
| --------------- | ---- | ------- | ------------------------------------------------ |
| `index.enabled` | bool | `false` | Enable main search integration (requires daemon) |

### Pattern Matching Configuration

```json
{
  "match": {
    "patterns": ["^=", "^[\\d\\.]+\\s*[\\+\\-]"],
    "priority": 100
  }
}
```

| Field            | Type   | Default | Description                                      |
| ---------------- | ------ | ------- | ------------------------------------------------ |
| `match.patterns` | array  | -       | Regex patterns to trigger instant match          |
| `match.priority` | number | `50`    | Higher priority wins when multiple plugins match |

### Static Index Configuration

Define searchable items directly in the manifest without a handler:

```json
{
  "staticIndex": [
    {
      "id": "shutdown",
      "name": "Shutdown",
      "description": "Power off the system",
      "icon": "power_settings_new",
      "keywords": ["power", "off", "halt"],
      "verb": "Execute",
      "entryPoint": {
        "step": "action",
        "selected": { "id": "shutdown" }
      }
    }
  ]
}
```

| Field        | Type   | Required | Description                          |
| ------------ | ------ | -------- | ------------------------------------ |
| `id`         | string | Yes      | Unique identifier                    |
| `name`       | string | Yes      | Display name (searchable)            |
| `description`| string | No       | Subtitle text                        |
| `icon`       | string | No       | Material icon name                   |
| `iconType`   | string | No       | `"material"` (default) or `"system"` |
| `keywords`   | array  | No       | Additional search terms              |
| `verb`       | string | No       | Action text (e.g., "Execute")        |
| `entryPoint` | object | No       | Handler invocation data              |

### Complete Manifest Example

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
  "name": "Timer",
  "description": "Countdown timers",
  "icon": "timer",
  "supportedPlatforms": ["niri", "hyprland", "macos", "windows"],
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
  "frecency": "plugin"
}
```

**Pattern-matching plugin:**

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
  },
  "frecency": "plugin"
}
```

---

## Request Schema

Every handler invocation receives a JSON object on stdin.

### Common Fields

| Field      | Type   | Always Present | Description                         |
| ---------- | ------ | -------------- | ----------------------------------- |
| `step`     | string | Yes            | Request type (see below)            |
| `query`    | string | Yes            | Current search bar text             |
| `selected` | object | No             | Selected item info                  |
| `action`   | string | No             | Action button ID                    |
| `context`  | string | No             | Plugin state from previous response |
| `session`  | string | Yes            | Unique session identifier           |

### Step Types

| Step         | When Triggered                                  | Key Fields           |
| ------------ | ----------------------------------------------- | -------------------- |
| `initial`    | Plugin opens                                    | -                    |
| `search`     | User types (realtime) or presses Enter (submit) | `query`              |
| `action`     | User selects item or clicks action              | `selected`, `action` |
| `match`      | Pattern matched in main search                  | `query`              |
| `form`       | Form submitted                                  | `formData`           |
| `formSlider` | Live form slider changed                        | `fieldId`, `value`   |
| `poll`       | Polling tick                                    | `query`              |
| `index`      | Index request                                   | `mode`, `indexedIds` |

### Request Examples

**Initial:**

```json
{ "step": "initial", "query": "", "session": "abc123" }
```

**Search:**

```json
{ "step": "search", "query": "firefox", "session": "abc123" }
```

**Action (item click):**

```json
{ "step": "action", "selected": { "id": "item-1" }, "session": "abc123" }
```

**Action (action button click):**

```json
{
  "step": "action",
  "selected": { "id": "item-1" },
  "action": "copy",
  "session": "abc123"
}
```

**Match:**

```json
{ "step": "match", "query": "2+2", "session": "abc123" }
```

**Form:**

```json
{
  "step": "form",
  "formData": { "title": "Note", "content": "..." },
  "context": "__add__",
  "session": "abc123"
}
```

**Index:**

```json
{ "step": "index", "mode": "full", "session": "abc123" }
```

---

## Response Schema

Every response must be a single JSON object with a `type` field.

### Response Types

| Type           | Purpose                     |
| -------------- | --------------------------- |
| `results`      | Display list of items       |
| `execute`      | Run action and/or close     |
| `match`        | Return pattern match result |
| `card`         | Display markdown content    |
| `form`         | Show input form             |
| `imageBrowser` | Image grid browser          |
| `gridBrowser`  | Generic grid layout         |
| `prompt`       | Simple text prompt          |
| `update`       | Patch existing items        |
| `index`        | Provide searchable items    |
| `status`       | Update plugin status        |
| `error`        | Show error message          |
| `noop`         | No UI change                |

---

## Results Response

```json
{
  "type": "results",
  "results": [],
  "placeholder": "Search...",
  "inputMode": "realtime",
  "clearInput": false,
  "context": "",
  "notify": "",
  "pluginActions": [],
  "navigateForward": null,
  "navigateBack": null,
  "navigationDepth": null,
  "status": {}
}
```

| Field             | Type   | Required | Default      | Description                           |
| ----------------- | ------ | -------- | ------------ | ------------------------------------- |
| `type`            | string | Yes      | -            | Must be `"results"`                   |
| `results`         | array  | Yes      | -            | Array of result items                 |
| `placeholder`     | string | No       | -            | Search bar hint text                  |
| `inputMode`       | string | No       | `"realtime"` | `"realtime"` or `"submit"`            |
| `clearInput`      | bool   | No       | `false`      | Clear search bar text                 |
| `context`         | string | No       | -            | State persisted across calls          |
| `notify`          | string | No       | -            | Show notification toast               |
| `pluginActions`   | array  | No       | `[]`         | Toolbar action buttons                |
| `navigateForward` | bool   | No       | -            | Increment navigation depth            |
| `navigateBack`    | bool   | No       | -            | Decrement navigation depth            |
| `navigationDepth` | int    | No       | -            | Set exact navigation depth            |
| `status`          | object | No       | -            | Plugin status update                  |
| `activate`        | bool   | No       | -            | Activate plugin for multi-step flows  |

### Result Item Schema

```json
{
  "id": "unique-id",
  "name": "Display Name",
  "description": "Subtitle text",
  "icon": "star",
  "iconType": "material",
  "thumbnail": "/path/to/image.png",
  "verb": "Open",
  "actions": [],
  "badges": [],
  "chips": [],
  "graph": {},
  "gauge": {},
  "progress": {},
  "preview": {}
}
```

| Field              | Type   | Required | Default      | Description                                     |
| ------------------ | ------ | -------- | ------------ | ----------------------------------------------- |
| `id`               | string | Yes      | -            | Unique identifier                               |
| `name`             | string | Yes      | -            | Primary display text                            |
| `description`      | string | No       | -            | Secondary text                                  |
| `icon`             | string | No       | -            | Material icon name                              |
| `iconType`         | string | No       | `"material"` | `"material"`, `"system"`, `"text"`, or `"path"` |
| `thumbnail`        | string | No       | -            | Image path (overrides icon)                     |
| `verb`             | string | No       | -            | Action text on hover                            |
| `actions`          | array  | No       | `[]`         | Secondary action buttons (max 4)                |
| `badges`           | array  | No       | `[]`         | Circular indicators (max 5)                     |
| `chips`            | array  | No       | `[]`         | Pill-shaped tags                                |
| `graph`            | object | No       | -            | Line graph (replaces icon)                      |
| `gauge`            | object | No       | -            | Circular progress (replaces icon)               |
| `progress`         | object | No       | -            | Progress bar (replaces description)             |
| `preview`          | object | No       | -            | Side panel content                              |
| `keepOpen`         | bool   | No       | `false`      | Keep launcher open after selection              |
| `displayHint`      | string | No       | `"auto"`     | View hint: `"auto"`, `"list"`, `"grid"`, `"large_grid"` |
| `isSuggestion`     | bool   | No       | `false`      | Mark as smart suggestion (shows reason)         |
| `suggestionReason` | string | No       | -            | Why this was suggested (e.g., "Often used at 9am") |
| `hasOcr`           | bool   | No       | `false`      | Item has OCR-searchable text (images)           |
| `keywords`         | array  | No       | -            | Additional search terms (index items only)      |
| `entryPoint`       | object | No       | -            | Handler invocation data (index items only)      |
| `appId`            | string | No       | -            | App ID for window matching (apps only)          |
| `appIdFallback`    | string | No       | -            | Fallback app ID (apps only)                     |

### Slider Item Schema

There are two ways to define sliders:

**Simple format (stdio plugins):**

```json
{
  "id": "volume",
  "type": "slider",
  "name": "Volume",
  "description": "",
  "icon": "volume_up",
  "value": 75,
  "min": 0,
  "max": 100,
  "step": 5,
  "unit": "%"
}
```

**Extended format (socket plugins with gauge):**

```json
{
  "id": "volume",
  "name": "Volume",
  "icon": "volume_up",
  "resultType": "slider",
  "value": {
    "value": 75,
    "min": 0,
    "max": 100,
    "step": 5,
    "displayValue": "75%"
  },
  "gauge": {
    "value": 75,
    "max": 100,
    "label": "75%"
  }
}
```

| Field         | Type          | Required | Default | Description                              |
| ------------- | ------------- | -------- | ------- | ---------------------------------------- |
| `id`          | string        | Yes      | -       | Unique identifier                        |
| `type`        | string        | No       | -       | `"slider"` (simple format)               |
| `resultType`  | string        | No       | -       | `"slider"` (extended format)             |
| `name`        | string        | Yes      | -       | Label text                               |
| `description` | string        | No       | -       | Subtitle                                 |
| `icon`        | string        | No       | -       | Material icon                            |
| `value`       | number/object | Yes      | -       | Current value or value object            |
| `min`         | number        | No       | `0`     | Minimum value (simple format)            |
| `max`         | number        | No       | `100`   | Maximum value (simple format)            |
| `step`        | number        | No       | `1`     | Step increment (simple format)           |
| `unit`        | string        | No       | -       | Unit suffix (simple format)              |
| `gauge`       | object        | No       | -       | Gauge display (extended format)          |

### Switch Item Schema

There are two ways to define switches:

**Simple format (stdio plugins):**

```json
{
  "id": "mute",
  "type": "switch",
  "name": "Mute",
  "description": "",
  "icon": "volume_off",
  "value": false
}
```

**Extended format (socket plugins):**

```json
{
  "id": "volume-mute",
  "name": "Mute Volume",
  "description": "Mute system audio output",
  "icon": "volume_up",
  "resultType": "switch",
  "value": false
}
```

| Field         | Type   | Required | Default | Description                  |
| ------------- | ------ | -------- | ------- | ---------------------------- |
| `id`          | string | Yes      | -       | Unique identifier            |
| `type`        | string | No       | -       | `"switch"` (simple format)   |
| `resultType`  | string | No       | -       | `"switch"` (extended format) |
| `name`        | string | Yes      | -       | Label text                   |
| `description` | string | No       | -       | Subtitle                     |
| `icon`        | string | No       | -       | Material icon                |
| `value`       | bool   | Yes      | -       | Current state                |

### Action Button Schema

```json
{
  "id": "copy",
  "name": "Copy",
  "icon": "content_copy",
  "entryPoint": {}
}
```

| Field        | Type   | Required | Description            |
| ------------ | ------ | -------- | ---------------------- |
| `id`         | string | Yes      | Action identifier      |
| `name`       | string | Yes      | Button label/tooltip   |
| `icon`       | string | No       | Material icon          |
| `entryPoint` | object | No       | For indexed items only |

### Plugin Action Schema

```json
{
  "id": "add",
  "name": "Add",
  "icon": "add_circle",
  "shortcut": "Ctrl+1",
  "confirm": "Are you sure?",
  "active": false
}
```

| Field      | Type   | Required | Default    | Description                 |
| ---------- | ------ | -------- | ---------- | --------------------------- |
| `id`       | string | Yes      | -          | Action identifier           |
| `name`     | string | Yes      | -          | Button label                |
| `icon`     | string | No       | -          | Material icon               |
| `shortcut` | string | No       | `"Ctrl+N"` | Keyboard shortcut hint      |
| `confirm`  | string | No       | -          | Confirmation dialog message |
| `active`   | bool   | No       | `false`    | Highlight as active         |

### Badge Schema

```json
{
  "text": "5",
  "icon": "star",
  "image": "/path/to/avatar.png",
  "color": "#ffffff"
}
```

| Field   | Type   | Required | Description                      |
| ------- | ------ | -------- | -------------------------------- |
| `text`  | string | No       | 1-3 character text               |
| `icon`  | string | No       | Material icon (overrides text)   |
| `image` | string | No       | Image path (overrides text/icon) |
| `color` | string | No       | Text/icon color                  |

### Chip Schema

```json
{
  "text": "Label",
  "icon": "tag",
  "color": "#ffffff",
  "background": "#4caf50"
}
```

| Field        | Type   | Required | Description      |
| ------------ | ------ | -------- | ---------------- |
| `text`       | string | Yes      | Chip text        |
| `icon`       | string | No       | Material icon    |
| `color`      | string | No       | Text/icon color  |
| `background` | string | No       | Background color |

### Graph Schema

```json
{
  "values": [10, 25, 15, 30, 20],
  "color": "#4caf50",
  "max": 100
}
```

| Field    | Type   | Required | Default | Description      |
| -------- | ------ | -------- | ------- | ---------------- |
| `values` | array  | Yes      | -       | Array of numbers |
| `color`  | string | No       | theme   | Line color       |
| `max`    | number | No       | auto    | Maximum Y value  |

### Gauge Schema

```json
{
  "value": 75,
  "max": 100,
  "label": "75%",
  "color": "#4caf50"
}
```

| Field   | Type   | Required | Default | Description   |
| ------- | ------ | -------- | ------- | ------------- |
| `value` | number | Yes      | -       | Current value |
| `max`   | number | No       | `100`   | Maximum value |
| `label` | string | No       | -       | Center label  |
| `color` | string | No       | theme   | Arc color     |

### Progress Schema

```json
{
  "value": 50,
  "max": 100,
  "label": "Downloading...",
  "color": "#2196f3"
}
```

| Field   | Type   | Required | Default | Description   |
| ------- | ------ | -------- | ------- | ------------- |
| `value` | number | Yes      | -       | Current value |
| `max`   | number | No       | `100`   | Maximum value |
| `label` | string | No       | -       | Text label    |
| `color` | string | No       | theme   | Bar color     |

### Preview Schema

```json
{
  "type": "markdown",
  "content": "# Preview\n\nContent here...",
  "detached": false
}
```

| Field      | Type   | Required | Description                          |
| ---------- | ------ | -------- | ------------------------------------ |
| `type`     | string | Yes      | `"markdown"`, `"image"`, or `"code"` |
| `content`  | string | Yes      | Preview content                      |
| `detached` | bool   | No       | Show as floating panel               |
| `language` | string | No       | Code language (for `"code"` type)    |

---

## Execute Response

```json
{
  "type": "execute",
  "launch": "/path/to/app.desktop",
  "copy": "text to copy",
  "typeText": "text to type",
  "openUrl": "https://example.com",
  "open": "/path/to/file",
  "notify": "Done!",
  "sound": "complete",
  "close": true
}
```

| Field      | Type   | Required | Description               |
| ---------- | ------ | -------- | ------------------------- |
| `type`     | string | Yes      | Must be `"execute"`       |
| `launch`   | string | No       | Desktop file path         |
| `copy`     | string | No       | Text to copy to clipboard |
| `typeText` | string | No       | Text to type via ydotool  |
| `openUrl`  | string | No       | URL to open               |
| `open`     | string | No       | File/folder path to open  |
| `notify`   | string | No       | Notification message      |
| `sound`    | string | No       | Sound effect name         |
| `close`    | bool   | No       | Close launcher            |

### Sound Names

| Sound          | Description            |
| -------------- | ---------------------- |
| `alarm`        | Timer/alarm completion |
| `timer`        | Pomodoro, countdown    |
| `complete`     | Task done              |
| `notification` | Alerts                 |
| `error`        | Failed operations      |
| `warning`      | Caution alerts         |

---

## Match Response

```json
{
  "type": "match",
  "result": {
    "id": "calc_result",
    "name": "4",
    "description": "2+2",
    "icon": "calculate",
    "verb": "Copy",
    "copy": "4",
    "openUrl": "",
    "notify": "Copied: 4",
    "close": true,
    "priority": 100,
    "actions": []
  }
}
```

| Field    | Type        | Required | Description                    |
| -------- | ----------- | -------- | ------------------------------ |
| `type`   | string      | Yes      | Must be `"match"`              |
| `result` | object/null | Yes      | Match result or `null` to hide |

### Match Result Fields

| Field         | Type   | Required | Description                |
| ------------- | ------ | -------- | -------------------------- |
| `id`          | string | Yes      | Unique identifier          |
| `name`        | string | Yes      | Primary text (the result)  |
| `description` | string | No       | Secondary text (the input) |
| `icon`        | string | No       | Material icon              |
| `verb`        | string | No       | Action text                |
| `copy`        | string | No       | Copy on selection          |
| `openUrl`     | string | No       | Open URL on selection      |
| `notify`      | string | No       | Notification after action  |
| `close`       | bool   | No       | Close launcher             |
| `priority`    | number | No       | Ranking priority           |
| `actions`     | array  | No       | Secondary action buttons   |

---

## Card Response

```json
{
  "type": "card",
  "card": {
    "title": "Definition",
    "content": "**noun**\n\nMeaning...",
    "markdown": true,
    "actions": []
  },
  "context": "word-id",
  "inputMode": "submit",
  "placeholder": "Type reply..."
}
```

| Field         | Type   | Required | Description                |
| ------------- | ------ | -------- | -------------------------- |
| `type`        | string | Yes      | Must be `"card"`           |
| `card`        | object | Yes      | Card content               |
| `context`     | string | No       | State for action handling  |
| `inputMode`   | string | No       | `"realtime"` or `"submit"` |
| `placeholder` | string | No       | Input hint                 |

### Card Object Fields

| Field      | Type   | Required | Description        |
| ---------- | ------ | -------- | ------------------ |
| `title`    | string | No       | Card title         |
| `content`  | string | Yes      | Card content       |
| `markdown` | bool   | No       | Render as markdown |
| `actions`  | array  | No       | Action buttons     |

---

## Form Response

```json
{
  "type": "form",
  "form": {
    "title": "Add Item",
    "submitLabel": "Save",
    "cancelLabel": "Cancel",
    "liveUpdate": false,
    "fields": []
  },
  "context": "__add__"
}
```

| Field     | Type   | Required | Description                |
| --------- | ------ | -------- | -------------------------- |
| `type`    | string | Yes      | Must be `"form"`           |
| `form`    | object | Yes      | Form definition            |
| `context` | string | No       | State passed to submission |

### Form Object Fields

| Field         | Type   | Required | Default    | Description               |
| ------------- | ------ | -------- | ---------- | ------------------------- |
| `title`       | string | No       | -          | Form title                |
| `submitLabel` | string | No       | `"Submit"` | Submit button text        |
| `cancelLabel` | string | No       | `"Cancel"` | Cancel button text        |
| `liveUpdate`  | bool   | No       | `false`    | Apply changes immediately |
| `fields`      | array  | Yes      | -          | Form field definitions    |

### Form Field Types

#### Text Field

```json
{
  "id": "name",
  "type": "text",
  "label": "Name",
  "placeholder": "Enter name...",
  "required": true,
  "default": "",
  "hint": "Helper text"
}
```

#### Textarea Field

```json
{
  "id": "content",
  "type": "textarea",
  "label": "Content",
  "placeholder": "",
  "required": false,
  "default": "",
  "rows": 6,
  "hint": ""
}
```

#### Email Field

```json
{
  "id": "email",
  "type": "email",
  "label": "Email",
  "placeholder": "user@example.com",
  "required": true,
  "default": "",
  "hint": ""
}
```

#### Password Field

```json
{
  "id": "password",
  "type": "password",
  "label": "Password",
  "placeholder": "",
  "required": true,
  "hint": ""
}
```

#### Hidden Field

```json
{
  "id": "item_id",
  "type": "hidden",
  "value": "abc123"
}
```

#### Select Field

```json
{
  "id": "theme",
  "type": "select",
  "label": "Theme",
  "options": [
    { "id": "light", "name": "Light" },
    { "id": "dark", "name": "Dark" }
  ],
  "default": "dark",
  "hint": ""
}
```

#### Checkbox Field

```json
{
  "id": "agree",
  "type": "checkbox",
  "label": "I agree to terms",
  "default": false,
  "hint": ""
}
```

#### Switch Field

```json
{
  "id": "enabled",
  "type": "switch",
  "label": "Enabled",
  "default": true,
  "hint": ""
}
```

#### Slider Field

```json
{
  "id": "volume",
  "type": "slider",
  "label": "Volume",
  "min": 0,
  "max": 100,
  "step": 5,
  "unit": "%",
  "default": 50,
  "hint": ""
}
```

### Form Field Common Properties

| Field      | Type   | Required | Description      |
| ---------- | ------ | -------- | ---------------- |
| `id`       | string | Yes      | Field identifier |
| `type`     | string | Yes      | Field type       |
| `label`    | string | No       | Display label    |
| `required` | bool   | No       | Validation       |
| `default`  | varies | No       | Default value    |
| `hint`     | string | No       | Helper text      |

---

## Image Browser Response

```json
{
  "type": "imageBrowser",
  "imageBrowser": {
    "directory": "~/Pictures",
    "title": "Select Image",
    "enableOcr": false,
    "actions": []
  }
}
```

| Field       | Type   | Required | Description            |
| ----------- | ------ | -------- | ---------------------- |
| `directory` | string | Yes      | Directory to browse    |
| `title`     | string | No       | Browser title          |
| `enableOcr` | bool   | No       | Enable OCR text search |
| `actions`   | array  | No       | Action buttons         |

---

## Grid Browser Response

```json
{
  "type": "gridBrowser",
  "gridBrowser": {
    "title": "Select Item",
    "columns": 8,
    "cellAspectRatio": 1.0,
    "items": [],
    "actions": []
  }
}
```

| Field             | Type   | Required | Default | Description        |
| ----------------- | ------ | -------- | ------- | ------------------ |
| `title`           | string | No       | -       | Browser title      |
| `columns`         | int    | No       | `8`     | Number of columns  |
| `cellAspectRatio` | float  | No       | `1.0`   | Width/height ratio |
| `items`           | array  | Yes      | -       | Grid items         |
| `actions`         | array  | No       | `[]`    | Action buttons     |

### Grid Item Schema

```json
{
  "id": "item-1",
  "name": "Item Name",
  "icon": "star",
  "iconType": "material",
  "keywords": ["search", "terms"]
}
```

---

## Index Response

```json
{
  "type": "index",
  "mode": "full",
  "items": [],
  "remove": []
}
```

| Field    | Type   | Required | Description                      |
| -------- | ------ | -------- | -------------------------------- |
| `type`   | string | Yes      | Must be `"index"`                |
| `mode`   | string | No       | `"full"` or `"incremental"`      |
| `items`  | array  | Yes      | Items to index                   |
| `remove` | array  | No       | Item IDs to remove (incremental) |

### Index Item Schema

```json
{
  "id": "app:firefox",
  "name": "Firefox",
  "description": "Web Browser",
  "icon": "firefox",
  "iconType": "system",
  "keywords": ["browser", "web"],
  "verb": "Open",
  "entryPoint": {
    "step": "action",
    "selected": { "id": "app:firefox" }
  },
  "actions": []
}
```

| Field         | Type   | Required | Description                |
| ------------- | ------ | -------- | -------------------------- |
| `id`          | string | Yes      | Unique identifier          |
| `name`        | string | Yes      | Display name (searchable)  |
| `description` | string | No       | Subtitle                   |
| `icon`        | string | No       | Icon name                  |
| `iconType`    | string | No       | `"material"` or `"system"` |
| `keywords`    | array  | No       | Additional search terms    |
| `verb`        | string | No       | Action text                |
| `entryPoint`  | object | Yes      | How to invoke handler      |
| `actions`     | array  | No       | Secondary actions          |

### Entry Point Schema

```json
{
  "step": "action",
  "selected": { "id": "item-id" },
  "action": "copy",
  "query": ""
}
```

| Field      | Type   | Required | Default    | Description        |
| ---------- | ------ | -------- | ---------- | ------------------ |
| `step`     | string | No       | `"action"` | Step type          |
| `selected` | object | No       | -          | Selected item info |
| `action`   | string | No       | -          | Action to perform  |
| `query`    | string | No       | -          | Query string       |

---

## Status Response

```json
{
  "type": "status",
  "status": {
    "badges": [],
    "chips": [],
    "description": "5 pending tasks",
    "fab": {},
    "ambient": []
  }
}
```

| Field         | Type   | Description                   |
| ------------- | ------ | ----------------------------- |
| `badges`      | array  | Badge indicators              |
| `chips`       | array  | Chip tags                     |
| `description` | string | Override manifest description |
| `fab`         | object | FAB override (see below)      |
| `ambient`     | array  | Ambient items (see below)     |

### FAB Override Schema

```json
{
  "chips": [{ "text": "04:32", "icon": "timer" }],
  "badges": [],
  "priority": 10,
  "showFab": true
}
```

| Field      | Type   | Description             |
| ---------- | ------ | ----------------------- |
| `chips`    | array  | Chip widgets            |
| `badges`   | array  | Badge widgets           |
| `priority` | number | Higher wins if multiple |
| `showFab`  | bool   | Force FAB visible       |

### Ambient Item Schema

```json
{
  "id": "timer-1",
  "name": "Focus Timer",
  "description": "24:32 remaining",
  "icon": "timer",
  "actions": [{ "id": "pause", "icon": "pause", "name": "Pause" }],
  "duration": 0
}
```

| Field         | Type   | Description                     |
| ------------- | ------ | ------------------------------- |
| `id`          | string | Unique identifier               |
| `name`        | string | Primary text                    |
| `description` | string | Secondary text                  |
| `icon`        | string | Material icon                   |
| `actions`     | array  | Action buttons                  |
| `duration`    | number | Auto-dismiss ms (0 = permanent) |

---

## Update Response

```json
{
  "type": "update",
  "items": [
    {
      "id": "volume",
      "gauge": { "value": 75, "max": 100 }
    }
  ]
}
```

| Field   | Type   | Required | Description          |
| ------- | ------ | -------- | -------------------- |
| `type`  | string | Yes      | Must be `"update"`   |
| `items` | array  | Yes      | Partial item updates |

---

## Error Response

```json
{
  "type": "error",
  "message": "Failed to connect"
}
```

| Field     | Type   | Required | Description       |
| --------- | ------ | -------- | ----------------- |
| `type`    | string | Yes      | Must be `"error"` |
| `message` | string | Yes      | Error message     |

---

## Noop Response

```json
{
  "type": "noop"
}
```

No UI change. Use for background operations or when UI already reflects the change (e.g., slider adjustments).

---

## Prompt Response

```json
{
  "type": "prompt",
  "prompt": {
    "text": "Enter search term..."
  }
}
```

| Field         | Type   | Required | Description        |
| ------------- | ------ | -------- | ------------------ |
| `type`        | string | Yes      | Must be `"prompt"` |
| `prompt.text` | string | Yes      | Prompt message     |

---

## Special IDs

| ID                | Context       | Meaning                    |
| ----------------- | ------------- | -------------------------- |
| `__back__`        | `selected.id` | Back button/Escape pressed |
| `__plugin__`      | `selected.id` | Plugin action clicked      |
| `__form_cancel__` | `selected.id` | Form cancelled             |
| `__empty__`       | `selected.id` | Non-actionable placeholder |
| `__dismiss__`     | `action`      | Ambient item dismissed     |
