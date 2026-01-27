# Hamr Plugin Protocol Reference

This document describes the JSON protocol used between plugins and the hamr daemon.

## Transport

Plugins communicate using one of two transport types:

| Type   | Description                               | Use Case                   |
| ------ | ----------------------------------------- | -------------------------- |
| `stdio`  | JSON over stdin/stdout, line-delimited  | Simple plugins, daemons    |
| `socket` | JSON-RPC 2.0 over Unix socket           | Complex daemons with SDK   |

### stdio Transport

For `stdio` handlers, hamr spawns the handler process and communicates via:
- **Request**: JSON written to handler's stdin (one object per line for daemons)
- **Response**: JSON read from handler's stdout

### socket Transport

For `socket` handlers (using the Python SDK), the handler connects to hamr's Unix socket at `$XDG_RUNTIME_DIR/hamr.sock` and uses length-prefixed JSON-RPC 2.0 messages.

## Request Schema

Each request is a JSON object with the following common fields:

| Field       | Type   | Description                                |
| ----------- | ------ | ------------------------------------------ |
| `step`      | string | `"initial"`, `"search"`, `"action"`, `"form"`, `"match"`, `"index"` |
| `query`     | string | Current search bar text                    |
| `selected`  | object | Selected item info (for action step)       |
| `action`    | string | Action button ID (for action step)         |
| `formData`  | object | Form field values (for form step)          |
| `context`   | string | Plugin state from previous response        |
| `session`   | string | Unique session identifier                  |
| `source`    | string | `"ambient"` for ambient bar actions        |
| `value`     | number | Slider/switch value (for slider/switch changes) |

### Step Types

| Step         | When Triggered                                  |
| ------------ | ----------------------------------------------- |
| `initial`    | Plugin opens                                    |
| `search`     | User types (realtime) or presses Enter (submit) |
| `action`     | User selects item or clicks action              |
| `match`      | Pattern matched in main search                  |
| `form`       | Form submitted                                  |
| `index`      | Index request (for daemon plugins)              |

### Examples

```json
{ "step": "initial", "session": "abc123" }
```

```json
{ "step": "search", "query": "calc 1+1", "session": "abc123" }
```

```json
{ "step": "action", "selected": {"id": "item-1"}, "action": "copy", "session": "abc123" }
```

```json
{ "step": "match", "query": "2+2", "session": "abc123" }
```

```json
{ "step": "index", "mode": "full", "session": "abc123" }
```

## Response Schema

Responses are JSON objects with a `type` field.

| Type        | Purpose                  |
| ----------- | ------------------------ |
| `results`   | Display list of items    |
| `execute`   | Run action and/or close  |
| `match`     | Pattern match result     |
| `card`      | Display markdown content |
| `form`      | Show input form          |
| `index`     | Provide searchable items |
| `status`    | Update plugin status     |
| `update`    | Patch existing items     |
| `error`     | Show error message       |
| `noop`      | No UI change             |

### results

```json
{
    "type": "results",
    "results": [{ "id": "1", "name": "Result", "description": "Example" }],
    "placeholder": "Search...",
    "inputMode": "realtime",
    "context": "state",
    "pluginActions": [],
    "status": {}
}
```

Result item fields:

| Field       | Type   | Description                              |
| ----------- | ------ | ---------------------------------------- |
| `id`        | string | Unique identifier                        |
| `name`      | string | Primary display text                     |
| `description` | string | Secondary text                         |
| `icon`      | string | Icon name                                |
| `iconType`  | string | `"system"`, `"material"`, or `"text"`    |
| `thumbnail` | string | Image path                               |
| `verb`      | string | Action text on hover                     |
| `type`      | string | `"slider"` or `"switch"` for interactive |
| `resultType` | string | Alternative to `type` for socket plugins |
| `value`     | varies | Current value for slider/switch          |
| `badges`    | array  | Badge indicators                         |
| `chips`     | array  | Pill-shaped tags                         |
| `actions`   | array  | Action buttons                           |
| `gauge`     | object | Circular progress (replaces icon)        |
| `graph`     | object | Line graph (replaces icon)               |
| `progress`  | object | Progress bar (replaces description)      |
| `preview`   | object | Side panel content                       |

### execute

```json
{
    "type": "execute",
    "launch": "/path/to/app.desktop",
    "copy": "text",
    "openUrl": "https://...",
    "open": "/path/to/file",
    "typeText": "text to type",
    "notify": "Done!",
    "sound": "complete",
    "close": true
}
```

### match

```json
{
    "type": "match",
    "result": {
        "id": "calc_result",
        "name": "4",
        "description": "2+2",
        "icon": "calculate",
        "copy": "4",
        "priority": 100
    }
}
```

Return `{"type": "match", "result": null}` to hide the match result.

### form

```json
{
    "type": "form",
    "form": {
        "title": "Add Item",
        "fields": [...],
        "submitLabel": "Save"
    },
    "context": "__add__"
}
```

### index

```json
{
    "type": "index",
    "mode": "full",
    "items": [
        {
            "id": "app:firefox",
            "name": "Firefox",
            "icon": "firefox",
            "iconType": "system",
            "keywords": ["browser"],
            "entryPoint": {"step": "action", "selected": {"id": "app:firefox"}}
        }
    ]
}
```

### status

```json
{
    "type": "status",
    "status": {
        "badges": [{"text": "5"}],
        "chips": [{"text": "Running", "icon": "timer"}],
        "description": "5 pending tasks",
        "fab": {"chips": [{"text": "04:32"}], "priority": 10, "showFab": true},
        "ambient": [{"id": "timer-1", "name": "Timer", "description": "04:32"}]
    }
}
```

### update

```json
{
    "type": "update",
    "items": [
        {"id": "volume", "value": 75, "gauge": {"value": 75, "max": 100}}
    ]
}
```

### error

```json
{ "type": "error", "message": "Something went wrong" }
```

### noop

```json
{ "type": "noop" }
```

## Special IDs

| ID                | Context       | Meaning                    |
| ----------------- | ------------- | -------------------------- |
| `__back__`        | `selected.id` | Back button/Escape pressed |
| `__plugin__`      | `selected.id` | Plugin action clicked      |
| `__form_cancel__` | `selected.id` | Form cancelled             |
| `__empty__`       | `selected.id` | Non-actionable placeholder |
| `__dismiss__`     | `action`      | Ambient item dismissed     |

## Notes

- Unknown fields should be ignored for forward compatibility.
- All field names use camelCase.
