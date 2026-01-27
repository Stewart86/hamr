# Visual Elements

Hamr provides rich visual elements for result items. This page covers sliders, switches, badges, gauges, and more.

## Slider Items

Sliders let users adjust numeric values (volume, brightness, etc.).

![Slider showing volume control](images/slider-item.png)

**Simple format (stdio plugins):**

```python
{
    "type": "results",
    "results": [
        {
            "id": "volume",
            "type": "slider",           # Makes this a slider
            "name": "Volume",
            "icon": "volume_up",
            "value": 75,                # Current value
            "min": 0,                   # Minimum
            "max": 100,                 # Maximum
            "step": 5,                  # Increment
            "unit": "%"                 # Optional: suffix
        }
    ]
}
```

**Extended format (socket plugins with gauge):**

```python
{
    "type": "results",
    "results": [
        {
            "id": "volume",
            "name": "Volume",
            "icon": "volume_up",
            "resultType": "slider",     # Alternative to "type"
            "value": {                  # Object for extended control
                "value": 75,
                "min": 0,
                "max": 100,
                "step": 5,
                "displayValue": "75%"
            },
            "gauge": {                  # Optional: show gauge alongside
                "value": 75,
                "max": 100,
                "label": "75%"
            }
        }
    ]
}
```

| Field          | Type          | Description                                        |
| -------------- | ------------- | -------------------------------------------------- |
| `type`         | string        | `"slider"` (simple format)                         |
| `resultType`   | string        | `"slider"` (extended format)                       |
| `value`        | number/object | Current value (number or object with min/max/step) |
| `min`          | number        | Minimum value (simple format)                      |
| `max`          | number        | Maximum value (simple format)                      |
| `step`         | number        | Step increment (also determines decimal precision) |
| `unit`         | string        | Unit suffix (e.g., `"%"`, `"px"`, `"ms"`)          |
| `displayValue` | string        | Override display text entirely                     |
| `gauge`        | object        | Optional gauge to display alongside slider         |

### Handling Slider Changes

When user drags the slider:

```python
{
    "step": "action",
    "selected": {"id": "volume"},
    "action": "slider",
    "value": 80                         # New value
}
```

Handle it and return updated results:

```python
if action == "slider":
    item_id = selected.get("id")
    new_value = input_data.get("value", 0)

    # Apply the change
    set_volume(item_id, new_value)

    # Return updated results
    print(json.dumps({
        "type": "results",
        "results": get_all_sliders(),
        "navigateForward": False  # Don't change navigation
    }))
```

**Example plugin:** [`sound/`](https://github.com/stewart86/hamr/tree/main/plugins/sound)

---

## Switch Items

Switches are boolean toggles (mute, enable/disable).

![Switch showing mute toggle](images/switch-item.png)

**Simple format (stdio plugins):**

```python
{
    "type": "results",
    "results": [
        {
            "id": "mute",
            "type": "switch",           # Makes this a switch
            "name": "Mute Volume",      # Action description
            "description": "Mute audio output",
            "icon": "volume_up",        # Shows current state
            "value": False              # Current state
        }
    ]
}
```

**Extended format (socket plugins):**

```python
{
    "type": "results",
    "results": [
        {
            "id": "volume-mute",
            "name": "Mute Volume",
            "description": "Mute system audio output",
            "icon": "volume_up",
            "resultType": "switch",     # Alternative to "type"
            "value": False
        }
    ]
}
```

### Naming Convention

The name describes the **action**, the icon shows the **current state**:

| State     | Name            | Icon         |
| --------- | --------------- | ------------ |
| Not muted | "Mute Volume"   | `volume_up`  |
| Muted     | "Unmute Volume" | `volume_off` |

### Handling Switch Changes

When user toggles:

```python
{
    "step": "action",
    "selected": {"id": "mute"},
    "action": "switch",
    "value": True                       # New value after toggle
}
```

Return an `update` response:

```python
if action == "switch":
    new_value = input_data.get("value", False)
    set_mute(new_value)

    print(json.dumps({
        "type": "update",
        "items": [
            {
                "id": "mute",
                "value": new_value,
                "name": "Unmute Volume" if new_value else "Mute Volume",
                "icon": "volume_off" if new_value else "volume_up"
            }
        ]
    }))
```

**Example plugin:** [`sound/`](https://github.com/stewart86/hamr/tree/main/plugins/sound)

---

## Badges

Small circular indicators beside the item name. Max 5 per item.

![Item with avatar badges and status indicator](images/badges.png)

```python
{
    "id": "task-1",
    "name": "Review PR",
    "icon": "task",
    "badges": [
        {"text": "JD"},                        # Initials
        {"text": "!", "color": "#f44336"},     # Alert (red text)
        {"icon": "verified", "color": "#4caf50"},  # Icon badge
        {"image": "/path/to/avatar.png"}       # Avatar image
    ]
}
```

| Field   | Type   | Description                    |
| ------- | ------ | ------------------------------ |
| `text`  | string | 1-3 characters (initials)      |
| `icon`  | string | Material icon (overrides text) |
| `image` | string | Image path (overrides text)    |
| `color` | string | Text/icon color (hex)          |

**Note:** Background is always theme default. Use `color` to tint text/icons.

---

## Chips

Pill-shaped tags for longer text. Show beside the item name.

![Item with category chips](images/chips.png)

```python
{
    "id": "task-1",
    "name": "Review PR",
    "icon": "task",
    "chips": [
        {"text": "In Progress"},                  # Simple label
        {"text": "Frontend", "icon": "code"},     # With icon
        {"text": "Urgent", "color": "#f44336"}    # Colored
    ]
}
```

| Field   | Type   | Description               |
| ------- | ------ | ------------------------- |
| `text`  | string | Label text                |
| `icon`  | string | Optional icon before text |
| `color` | string | Text/icon color (hex)     |

---

## Gauge

Circular progress indicator shown in place of the icon.

![Item with circular gauge showing 62%](images/gauge.png)

```python
{
    "id": "disk",
    "name": "Disk Space",
    "gauge": {
        "value": 75,           # Current value
        "max": 100,            # Maximum value
        "label": "75%"         # Center label
    }
}
```

| Field   | Type   | Description            |
| ------- | ------ | ---------------------- |
| `value` | number | Current value          |
| `max`   | number | Maximum value          |
| `label` | string | Center text (optional) |

---

## Graph

Line graph shown in place of the icon. Good for trends/history.

![Item with CPU usage graph](images/graph.png)

```python
{
    "id": "cpu",
    "name": "CPU Usage",
    "graph": {
        "data": [45, 52, 48, 61, 55, 50, 47],  # Y values
        "min": 0,                               # Optional: min Y
        "max": 100                              # Optional: max Y
    }
}
```

If `min`/`max` not provided, auto-scales from data.

---

## Progress Bar

Horizontal progress bar shown below the name. Replaces description.

![Item with download progress bar](images/progress-bar.png)

```python
{
    "id": "download",
    "name": "Downloading file.zip",
    "icon": "downloading",
    "progress": {
        "value": 65,           # Current value
        "max": 100,            # Maximum value
        "label": "65%",        # Text beside bar
        "color": "#4caf50"     # Custom bar color
    }
}
```

| Field   | Type   | Description                  |
| ------- | ------ | ---------------------------- |
| `value` | number | Current progress             |
| `max`   | number | Maximum value (default: 100) |
| `label` | string | Text shown beside bar        |
| `color` | string | Custom bar color (hex)       |

---

## Preview Panel

Add a `preview` field to show rich content in a side panel on hover/selection.

![Item with image preview in side panel](images/preview-panel.png)

```python
{
    "id": "image-1",
    "name": "sunset.jpg",
    "icon": "image",
    "preview": {
        "type": "image",                    # "image", "markdown", "text", "metadata"
        "content": "/path/to/sunset.jpg",
        "title": "Sunset Photo",
        "metadata": [
            {"label": "Size", "value": "3840x2160"},
            {"label": "Date", "value": "2024-01-15"}
        ],
        "actions": [
            {"id": "open", "name": "Open", "icon": "open_in_new"}
        ],
        "detachable": true                  # Allow pinning
    }
}
```

### Preview Types

| Type       | Content Field         | Description                        |
| ---------- | --------------------- | ---------------------------------- |
| `image`    | File path             | Shows image with optional metadata |
| `markdown` | Markdown text         | Renders markdown                   |
| `text`     | Plain text            | Monospace display                  |
| `metadata` | (uses metadata array) | Key-value pairs only               |

### Detachable Previews

Users can pin previews to a floating panel that persists after launcher closes.

![Detached preview panel floating on desktop](images/preview-panel-detached.png)

**Example plugins:** [`pictures/`](https://github.com/stewart86/hamr/tree/main/plugins/pictures), [`notes/`](https://github.com/stewart86/hamr/tree/main/plugins/notes)

---

## Visual Priority

If multiple visual elements are set on an item:

**Icon area priority:** `graph` > `gauge` > `thumbnail` > `icon`

**Progress bar:** Independent, replaces description.

---

## Icon Types

### Material Icons (Default)

Use any icon from [Material Symbols](https://fonts.google.com/icons).

```python
{"id": "1", "name": "Item", "icon": "star"}
```

Common icons:

| Category   | Icons                                                  |
| ---------- | ------------------------------------------------------ |
| Navigation | `arrow_back`, `home`, `menu`, `close`                  |
| Actions    | `open_in_new`, `content_copy`, `delete`, `edit`, `add` |
| Files      | `folder`, `description`, `image`, `video_file`         |
| UI         | `search`, `settings`, `star`, `favorite`, `info`       |

### System Icons

For desktop application icons, set `iconType: "system"`:

```python
{
    "id": "chrome",
    "name": "Google Chrome",
    "icon": "google-chrome",      # From .desktop file
    "iconType": "system"
}
```

**Auto-detection:** Icons with `.` or `-` are assumed system icons. Simple names like `btop` need explicit `iconType: "system"`.

---

## Thumbnails

Show image previews instead of icons:

```python
{
    "id": "file-1",
    "name": "photo.jpg",
    "thumbnail": "/path/to/photo.jpg"
}
```

**Use sparingly** - thumbnails load images and impact performance.

---

## Updating Visual Elements

Use `type: "update"` to patch items without replacing the entire list:

```python
{
    "type": "update",
    "items": [
        {
            "id": "volume",
            "gauge": {"value": 80, "max": 100, "label": "80%"},
            "badges": [{"text": "M", "color": "#f44336"}]
        }
    ]
}
```

This preserves selection and focus - ideal for:

- Slider adjustments
- Live status updates
- Real-time data changes

**Example plugin:** [`sound/`](https://github.com/stewart86/hamr/tree/main/plugins/sound)
