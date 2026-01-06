# Unified Execution Model Specification

## Overview

Simplify plugin development by routing ALL item executions through the plugin handler via `entryPoint`. This eliminates the need for plugins to specify shell commands in indexed items - the handler receives the execution request and decides what to do.

## Status: Implemented

The unified execution model has been implemented. Key changes:

1. **Single execution function**: `PluginRunner.executeAction(pluginId, entryPoint, keepOpen)`
2. **Safe execute API**: No arbitrary command execution - only whitelisted actions
3. **Merged recording**: `ContextTracker.recordLaunch()` moved into `recordExecution()` for apps
4. **Plugins browser**: `/` prefix now uses standard plugin flow instead of special exclusive mode

## Motivation

### Previous Complexity

Three different execution paths in `ResultFactory.qml`:

1. **`Quickshell.execDetached(command)`** - Direct shell execution
   - Used when `item.execute.command` is defined
   - Bypasses handler entirely
   - Security risk: arbitrary command execution

2. **`PluginRunner.executeEntryPoint(pluginId, entryPoint)`** - Handler execution, launcher stays open

3. **`PluginRunner.replayAction(pluginId, entryPoint)`** - Handler execution, launcher closes

### Problems

- Plugin authors must decide: "Do I use `execute.command` or `entryPoint`?"
- Different code paths = different behaviors (frecency recording, etc.)
- Some plugins use `execute.command` for simplicity (apps, power)
- Other plugins use `entryPoint` for security (bitwarden - never store passwords)
- ResultFactory has complex if/else logic to handle all cases

## Unified Model

### Single Execution Path

**Every execution goes through the handler via `entryPoint`.**

```
User selects item
    → Hamr calls handler with entryPoint
    → Handler executes (via safe API response)
    → Hamr processes response (launch, copy, type, etc.)
```

### Benefits

1. **Simpler plugin development** - Handler does everything, no need to embed commands in index
2. **Single code path** in ResultFactory
3. **Handler controls execution** - Can do pre/post processing, validation, logging
4. **Security** - No arbitrary command execution, only whitelisted actions
5. **Flexibility** - Handler can decide at runtime what to do

## Implementation

### PluginRunner.executeAction()

Single unified function replacing `executeEntryPoint()` and `replayAction()`:

```javascript
function executeAction(pluginId, entryPoint, keepOpen = false) {
    // Start plugin process
    // Send entryPoint to handler
    // keepOpen: true = launcher stays open (interactive)
    // keepOpen: false = launcher closes after execution (replay mode)
}
```

### Safe Execute Response API

Handlers return execution instructions using **safe, whitelisted actions** (no arbitrary commands):

```python
# Launch an app
{"type": "execute", "launch": "/usr/share/applications/firefox.desktop", "close": True}

# Copy to clipboard and notify
{"type": "execute", "copy": "text to copy", "notify": "Copied!", "close": True}

# Type text (snippet expansion)
{"type": "execute", "typeText": "expanded text", "close": True}

# Focus existing window
{"type": "execute", "focusApp": "firefox", "close": True}

# Open URL
{"type": "execute", "openUrl": "https://example.com", "close": True}

# Open file/folder
{"type": "execute", "open": "/path/to/file", "close": True}

# Just close (handler already did the work)
{"type": "execute", "close": True}

# Show notification only (no close)
{"type": "execute", "notify": "Task completed", "sound": "complete"}
```

### processExecuteAction() - Safe API Handler

Located in `PluginRunner.qml`, handles all whitelisted actions:

```javascript
function processExecuteAction(exec, pluginId) {
    if (exec.launch) Quickshell.execDetached(["gio", "launch", exec.launch]);
    if (exec.focusApp) WindowManager.focusWindow(...);
    if (exec.copy) Quickshell.execDetached(["wl-copy", exec.copy]);
    if (exec.typeText) {
        if (exec.close) {
            // Defer until launcher closes
            root.pendingTypeText = exec.typeText;
        } else {
            Quickshell.execDetached(["ydotool", "type", "--clearmodifiers", "--", exec.typeText]);
        }
    }
    if (exec.openUrl) Qt.openUrlExternally(exec.openUrl);
    if (exec.open) Quickshell.execDetached(["xdg-open", exec.open]);
    if (exec.sound) AudioService.playSound(exec.sound);
    if (exec.notify) Quickshell.execDetached(["notify-send", "-a", pluginName, exec.notify]);
    if (exec.close) GlobalStates.launcherOpen = false;
}
```

### ResultFactory Simplification

**Before (~100 lines of if/else):**
```javascript
if (capturedIsApp) {
    // Window focus logic...
    if (currentWindowCount === 0) {
        PluginRunner.recordExecution(...);
        ContextTracker.recordLaunch(capturedAppId);
        if (capturedItem.execute?.command) {
            Quickshell.execDetached(capturedItem.execute.command);
        }
    } else if (currentWindowCount === 1) {
        // ...
    }
} else {
    if (capturedItem.entryPoint) {
        if (capturedItem.keepOpen) {
            PluginRunner.executeEntryPoint(...);
        } else {
            PluginRunner.replayAction(...);
        }
        return;
    }
    if (capturedItem.execute?.command) {
        PluginRunner.recordExecution(...);
        Quickshell.execDetached(capturedItem.execute.command);
    }
    // ... more branches
}
```

**After (~15 lines):**
```javascript
// Record execution (frecency + context tracking for apps)
PluginRunner.recordExecution(capturedPluginId, capturedItem.id, capturedQuery, capturedLaunchFromEmpty);

if (capturedIsApp) {
    // Window management stays in hamr (compositor-specific)
    const windows = WindowManager.getWindowsForApp(appId);
    if (windows.length === 0) {
        PluginRunner.executeAction(pluginId, entryPoint, false);
    } else if (windows.length === 1) {
        WindowManager.focusWindow(windows[0]);
    } else {
        GlobalStates.openWindowPicker(...);
    }
} else {
    // Non-app items: execute via handler
    const entryPoint = capturedItem.entryPoint ?? {
        step: "action",
        selected: { id: capturedItem.id }
    };
    PluginRunner.executeAction(pluginId, entryPoint, keepOpen);
}
```

### Merged recordLaunch into recordExecution

`ContextTracker.recordLaunch()` is now called inside `recordExecution()` for apps:

```javascript
function recordExecution(pluginId, itemId, searchTerm, launchFromEmpty) {
    // ... existing frecency logic ...
    
    // For apps plugin, also track for sequence detection
    if (pluginId === "apps") {
        if (item.appId) {
            ContextTracker.recordLaunch(item.appId);  // Moved here
        }
        const context = ContextTracker.getContext();
        context.launchFromEmpty = launchFromEmpty ?? false;
        root.updateItemSmartFields(item, context);
    }
    
    // ... save to disk ...
}
```

## Plugins Browser Refactor

The `/` prefix for browsing plugins has been standardized to use the normal plugin flow instead of a special "action" exclusive mode.

### Changes Made

1. **Created `plugins/plugins/` plugin** - Lists available plugins
2. **Handler receives plugin list via context** - `context.plugins` contains all plugins
3. **New response type: `startPlugin`** - Handler returns `{"type": "startPlugin", "pluginId": "..."}`
4. **Removed "action" exclusive mode** - No more special case in LauncherSearch
5. **Renamed Config properties** - `prefix.action` → `prefix.plugins`, `SearchPrefixType.Action` → `SearchPrefixType.Plugins`

### plugins/plugins/handler.py

```python
def handle_request(request):
    plugins = request.get("context", {}).get("plugins", [])
    
    if step in ("initial", "search"):
        # Filter and return plugin list
        results = [{"id": p["id"], "name": p["name"], ...} for p in filtered]
        emit({"type": "results", "results": results, ...})
    
    if step == "action":
        # Start selected plugin
        emit({"type": "startPlugin", "pluginId": selected_id})
```

### PluginRunner Context Injection

When starting the `plugins` plugin, hamr injects the plugin list:

```javascript
if (pluginId === "plugins") {
    input.context = {
        plugins: root.plugins
            .filter(p => p.id !== "plugins")
            .map(p => ({
                id: p.id,
                name: p.manifest?.name ?? p.id,
                description: p.manifest?.description ?? "",
                icon: p.manifest?.icon ?? "extension"
            }))
            .sort((a, b) => a.name.localeCompare(b.name))
    };
}
```

## Execute Response API

```typescript
interface ExecuteResponse {
    type: "execute";
    
    // === OS-Level Actions (hamr provides these) ===
    launch?: string;      // Desktop file path
    focusApp?: string;    // App ID for window focus
    copy?: string;        // Text to copy to clipboard
    typeText?: string;    // Text to type via input simulation
    notify?: string;      // Notification message
    sound?: string;       // Sound name or path
    openUrl?: string;     // URL to open in browser
    open?: string;        // File/folder to open
    
    // === UI Control ===
    close?: boolean;      // Close launcher after execution
    
    // === History Tracking ===
    name?: string;        // Name for history
    entryPoint?: object;  // Entry point for replay
    icon?: string;
    iconType?: "material" | "system";
}
```

### What Handlers Do vs Hamr Does

| Action | Handler Does | Hamr Does |
|--------|--------------|-----------|
| Launch app | Returns `launch` | Runs `gio launch` detached |
| Focus window | Returns `focusApp` | Calls compositor API (Hyprland/Niri) |
| Copy text | Returns `copy` | Runs `wl-copy` |
| Type text | Returns `typeText` | Types via `ydotool` (requires ydotoold daemon) |
| Notify | Returns `notify` | Runs `notify-send` |
| Play sound | Returns `sound` | Uses AudioService |
| Open URL | Returns `openUrl` | Uses `Qt.openUrlExternally` |
| Open file | Returns `open` | Runs `xdg-open` |

### Security

- **No arbitrary command execution** - Handlers cannot run arbitrary shell commands
- **Whitelisted actions only** - Only the defined API methods are supported
- **Hamr controls execution** - All OS interaction goes through hamr's safe wrappers

## Files Changed

### Hamr Core
- `services/PluginRunner.qml` - Added `executeAction()`, `processExecuteAction()`, `startPlugin` response handler, context injection for plugins browser
- `services/ResultFactory.qml` - Simplified to use `executeAction()`, removed direct `execDetached`
- `services/LauncherSearch.qml` - Removed "action" exclusive mode special case, fixed spread operators
- `modules/common/Config.qml` - Renamed `prefix.action` to `prefix.plugins`, updated hints
- `modules/launcher/SearchBar.qml` - Renamed `SearchPrefixType.Action` to `Plugins`, removed action mode handling
- `modules/launcher/SearchWidget.qml` - Updated prefix reference
- `modules/launcher/WindowPicker.qml` - Removed explicit `ContextTracker.recordLaunch()` call
- `services/NiriService.qml` - Fixed spread operators (`.slice()`, `.concat()`)

### New Plugin
- `plugins/plugins/manifest.json` - Plugin manifest
- `plugins/plugins/handler.py` - Lists and launches plugins

## Migration: Updating Plugins

Plugins need to be updated to use the new safe API instead of `execute.command`:

**Before:**
```python
emit({"type": "execute", "execute": {"command": ["gio", "launch", path], "close": True}})
```

**After:**
```python
emit({"type": "execute", "launch": path, "close": True})
```

### Plugins to Update
- `plugins/apps/handler.py` - Use `launch` instead of `command`
- `plugins/power/handler.py` - Execute commands in handler, return `{"type": "execute", "close": True}`
- `plugins/emoji/handler.py` - Use `copy` action
- `plugins/clipboard/handler.py` - Use `copy`/`type` actions
- `plugins/snippet/handler.py` - Use `type` action

## Testing Checklist

- [ ] Apps launch from main search
- [ ] Apps launch from Recent items
- [ ] App window focus (single window)
- [ ] App window picker (multiple windows)
- [ ] `/` prefix opens plugins browser
- [ ] Selecting plugin from browser starts that plugin
- [ ] Power actions work
- [ ] Frecency recording works
- [ ] Context tracking (app sequences) works
- [ ] Plugins that already use entryPoint still work
