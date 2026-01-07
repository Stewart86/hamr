# Converting Raycast Extensions

This guide helps you port [Raycast](https://raycast.com) extensions to Hamr plugins.

## Architecture Comparison

| Aspect | Raycast | Hamr |
|--------|---------|------|
| **Language** | TypeScript/React | Any (Python recommended) |
| **UI Model** | React components | JSON responses |
| **Data Flow** | React hooks + state | stdin/stdout per step |
| **Platform** | macOS | Linux (Wayland) |

## Raycast Extension Structure

```
raycast-extension/
├── package.json          # Manifest + commands
├── src/
│   ├── index.tsx         # Main command (React)
│   ├── hooks/            # Data fetching
│   ├── components/       # Reusable UI
│   └── utils/            # Helpers
└── assets/               # Icons
```

## Component Mapping

| Raycast Component | Hamr Equivalent |
|-------------------|-----------------|
| `<List>` | `{"type": "results", "results": [...]}` |
| `<List.Item>` | `{"id": "...", "name": "...", "icon": "..."}` |
| `<List.Item.Detail>` | `{"type": "card", "card": {...}}` |
| `<Detail>` | `{"type": "card", "card": {...}}` |
| `<Grid>` | `{"type": "imageBrowser"}` or results with thumbnails |
| `<Form>` | `{"type": "form"}` or multi-step with submit mode |
| `<ActionPanel>` | `"actions": [...]` on result items |
| `Action.CopyToClipboard` | `{"copy": "text"}` |
| `Action.OpenInBrowser` | `{"openUrl": "url"}` |
| `Action.Push` | Return new results (navigation) |
| `showToast()` | `{"notify": "message"}` |
| `getPreferenceValues()` | Config file or environment |

## Hook Translation

| Raycast Hook | Hamr Equivalent |
|--------------|-----------------|
| `usePromise` | Fetch in handler, return results |
| `useCachedPromise` | Cache to JSON file |
| `useCachedState` | Use `context` field |
| `useState` | Use `context` for state |
| `useEffect` | Not needed (stateless calls) |

## Path Mapping (macOS to Linux)

| macOS | Linux |
|-------|-------|
| `~/Library/Application Support/Google/Chrome` | `~/.config/google-chrome` |
| `~/Library/Application Support/BraveSoftware/Brave-Browser` | `~/.config/BraveSoftware/Brave-Browser` |
| `~/Library/Application Support/Microsoft Edge` | `~/.config/microsoft-edge` |
| `~/Library/Application Support/Firefox` | `~/.mozilla/firefox` |
| `~/Library/Preferences` | `~/.config` |
| `~/Library/Caches` | `~/.cache` |

## API Mapping (macOS to Linux)

| Raycast/macOS | Linux Equivalent |
|---------------|------------------|
| `Clipboard.copy()` | `wl-copy` |
| `Clipboard.paste()` | `wl-paste` |
| `showHUD()` | `notify-send` |
| `open` command | `xdg-open` |
| `getFrontmostApplication()` | `hyprctl activewindow -j` |
| AppleScript | D-Bus or CLI tools |
| Keychain | `secret-tool` (libsecret) |

---

## Example Conversion

### Raycast (TypeScript/React)

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

### Hamr (Python)

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
                "copy": item["url"],
                "notify": "URL copied",
                "close": True
            }))
        else:
            print(json.dumps({
                "type": "execute",
                "openUrl": item["url"],
                "name": f"Open {item['title']}",
                "icon": "bookmark",
                "close": True
            }))

if __name__ == "__main__":
    main()
```

---

## Conversion Checklist

### 1. Identify Data Source

- [ ] API calls: Use `requests` or `subprocess`
- [ ] Local files: Update paths for Linux
- [ ] System APIs: Find Linux equivalents

### 2. Map UI Components

- [ ] `List` to results response
- [ ] `Detail`/`List.Item.Detail` to card response
- [ ] `Grid` to imageBrowser or thumbnails
- [ ] `Form` to form response or submit mode

### 3. Handle Actions

- [ ] `Action.OpenInBrowser` to `openUrl`
- [ ] `Action.CopyToClipboard` to `copy`
- [ ] `Action.Push` to return new results
- [ ] Custom actions to execute responses

### 4. Replace Platform APIs

- [ ] Clipboard: `wl-copy`/`wl-paste`
- [ ] Notifications: `notify-send`
- [ ] File paths: Linux equivalents
- [ ] Keychain: `secret-tool` or config file

### 5. Test Edge Cases

- [ ] Empty results
- [ ] Missing files/directories
- [ ] Network errors
- [ ] Permission errors

---

## Using AI to Convert

The [`create-plugin`](../../plugins/create-plugin/) workflow can help convert Raycast extensions:

1. Run `/create-plugin` in Hamr
2. Provide the Raycast extension URL
3. AI analyzes and creates Hamr equivalent

**Example prompt:**

```
Create a Hamr plugin that replicates the functionality of this Raycast extension:
https://github.com/raycast/extensions/tree/main/extensions/browser-bookmarks

Focus on Chrome and Firefox support for Linux.
```

---

## Common Patterns

### Preferences

Raycast:
```tsx
const { apiKey } = getPreferenceValues<Preferences>();
```

Hamr:
```python
import os
from pathlib import Path

CONFIG_FILE = Path.home() / ".config" / "hamr" / "my-plugin.json"

def get_config():
    if CONFIG_FILE.exists():
        return json.loads(CONFIG_FILE.read_text())
    return {}

api_key = os.environ.get("MY_API_KEY") or get_config().get("apiKey")
```

### Caching

Raycast:
```tsx
const { data, isLoading } = useCachedPromise(fetchData);
```

Hamr:
```python
import time

CACHE_FILE = Path.home() / ".cache" / "hamr" / "my-plugin.json"
CACHE_TTL = 300  # 5 minutes

def get_cached_data():
    if CACHE_FILE.exists():
        cache = json.loads(CACHE_FILE.read_text())
        if time.time() - cache.get("timestamp", 0) < CACHE_TTL:
            return cache.get("data")
    
    data = fetch_data()
    CACHE_FILE.parent.mkdir(parents=True, exist_ok=True)
    CACHE_FILE.write_text(json.dumps({"data": data, "timestamp": time.time()}))
    return data
```

### Detail View

Raycast:
```tsx
<List.Item.Detail markdown={`# ${item.title}\n\n${item.content}`} />
```

Hamr:
```python
{
    "type": "card",
    "card": {
        "title": item["title"],
        "content": item["content"],
        "markdown": True
    }
}
```

### Navigation (Push)

Raycast:
```tsx
<Action.Push title="View Details" target={<DetailView item={item} />} />
```

Hamr:
```python
# On action, return new results with navigateForward
if action == "view":
    print(json.dumps({
        "type": "results",
        "results": get_detail_items(item_id),
        "navigateForward": True,
        "context": f"detail:{item_id}"
    }))
```
