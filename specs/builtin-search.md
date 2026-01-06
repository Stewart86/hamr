# Builtin Search Feature Specification

## Overview

Automatically use hamr's unified search algorithm (fuzzy matching + frecency + learned shortcuts) for all plugins that provide indexed items, instead of requiring handlers to implement their own search logic.

## Motivation

Currently, plugins implement their own search logic in handlers:
- Basic fuzzy matching without frecency
- No learned shortcuts support (typing "ff" for Firefox)
- No history term boosting
- Duplicated effort across plugins

With builtin search, plugins automatically get hamr's full ranking algorithm:
1. **Learned shortcuts first** - Items where user typed that exact term before
2. **Frecency decides ties** - Most frequently/recently used wins
3. **Fuzzy matches last** - Items that match but weren't searched that way before

## Behavior

### Automatic Activation

Builtin search is **automatically enabled** for any plugin that has indexed items. No manifest configuration required.

```
Has index (pluginIndexes[pluginId].items.length > 0) → Builtin search
No index → Handler search
```

### Hybrid Mode: Prepend Handler Results

When a plugin has **both** indexed items and a search handler, hamr:
1. Calls builtin search on indexed items
2. Calls handler's search step
3. **Prepends** handler results before builtin results

This allows plugins to inject custom results (like "Add: {query}" for todo) while still benefiting from builtin fuzzy+frecency search on their items.

```
Plugin has index + search handler → Handler results + Builtin results
Plugin has index only            → Builtin results only
Plugin has search handler only   → Handler results only
```

**Example: Todo plugin**
```
User types "buy milk"
→ Handler returns: [{ "Add: buy milk" }]
→ Builtin search returns: [{ "buy groceries" }, { "milk reminder" }]
→ Final results: [{ "Add: buy milk" }, { "buy groceries" }, { "milk reminder" }]
```

### Step Handling

| Step | Handled By | Purpose |
|------|------------|---------|
| `index` | Handler | Provide items to search |
| `initial` | Handler | Custom landing UI (categories, etc.) |
| `search` | **Hamr + Handler** | Builtin search + optional handler prepend |
| `action` | Handler | Handle item selection |

For search step:
- If plugin has index: Hamr does builtin search
- If plugin also has search handler: Handler results are prepended
- Handler can return empty results to use pure builtin search

### Search Scope

Builtin search **always searches ALL of the plugin's indexed items**, regardless of navigation context (`pluginContext`). This provides consistent "find anything" behavior.

**Example flow:**
```
User enters /apps
    → Handler shows: [All Apps, Internet, Development, Settings, ...]

User types "fire"
    → Builtin search finds "Firefox" from ALL indexed apps
    → Shows: [Firefox] (frecency + fuzzy ranked)

User clears search, selects "Settings" category  
    → Handler shows: [System Settings, Display, Network, ...]
    
User types "net"
    → Builtin search finds "Network" from ALL indexed apps
    → Shows: [Network, Firefox, ...] (searches all, not just Settings)
```

### Empty Query Handling

When query is empty:
- Send to handler (show initial UI, category contents, etc.)

This preserves browsing functionality while search always uses builtin algorithm.

## Implementation

### PluginRunner.qml Changes

#### 1. Modify `search()` function

```javascript
function search(query) {
    if (!root.activePlugin) return;
    
    const pluginId = root.activePlugin.id;
    const hasIndex = root.pluginIndexes[pluginId]?.items?.length > 0;
    
    // Use builtin search if plugin has indexed items and query is not empty
    if (hasIndex && query.trim() !== "") {
        root.doBuiltinSearch(pluginId, query);
        return;
    }
    
    // ... existing handler logic (for empty query or plugins without index)
}
```

#### 2. Add `doBuiltinSearch()` function

```javascript
function doBuiltinSearch(pluginId, query) {
    const indexData = root.pluginIndexes[pluginId];
    if (!indexData?.items) {
        root.pluginResults = [];
        return;
    }
    
    // Build searchables with history terms
    const searchables = [];
    for (const item of indexData.items) {
        // Skip special entries
        if (item.id === "__plugin__" || item._isPluginEntry) continue;
        
        // Add main item searchable
        searchables.push({
            name: Fuzzy.prepare(item.name),
            keywords: item.keywords?.length > 0 ? Fuzzy.prepare(item.keywords.join(" ")) : null,
            item: item,
            isHistoryTerm: false
        });
        
        // Add history term entries (learned shortcuts)
        const recentTerms = item._recentSearchTerms ?? [];
        for (const term of recentTerms) {
            searchables.push({
                name: Fuzzy.prepare(term),
                item: item,
                isHistoryTerm: true,
                matchedTerm: term
            });
        }
    }
    
    if (searchables.length === 0) {
        root.pluginResults = [];
        return;
    }
    
    // Fuzzy search with frecency scoring
    const fuzzyResults = Fuzzy.go(query, searchables, {
        keys: ["name", "keywords"],
        limit: 100,
        threshold: 0.25,
        scoreFn: (result) => {
            const searchable = result.obj;
            
            // Multi-field scoring
            const nameScore = result[0]?.score ?? 0;
            const keywordsScore = result[1]?.score ?? 0;
            const baseScore = nameScore * 1.0 + keywordsScore * 0.3;
            
            // Frecency boost
            const frecency = root.getItemFrecency(pluginId, searchable.item.id);
            const frecencyBoost = Math.min(frecency * 0.02, 0.3);
            
            // History term boost (learned shortcuts)
            const historyBoost = searchable.isHistoryTerm ? 0.2 : 0;
            
            return baseScore + frecencyBoost + historyBoost;
        }
    });
    
    // Deduplicate (same item may match via name and history term)
    const seen = new Set();
    const results = [];
    for (const match of fuzzyResults) {
        const item = match.obj.item;
        if (seen.has(item.id)) continue;
        seen.add(item.id);
        results.push(item);
    }
    
    root.pluginResults = results;
}
```

### Handler Simplification

Plugins with indexes can simplify their `search` step:

**Option 1: Remove search step entirely (pure builtin search)**
```python
def handle_request(request):
    step = request.get("step", "initial")
    
    if step == "index":
        # Provide items - hamr will search them
        
    if step == "initial":
        # Show landing UI (optional)
        
    # search step - HANDLED BY HAMR AUTOMATICALLY
        
    if step == "action":
        # Handle item selection
```

**Option 2: Prepend custom results (hybrid search)**
```python
def handle_request(request):
    step = request.get("step", "initial")
    query = request.get("query", "")
    
    if step == "index":
        # Provide items to search
        return {"type": "index", "results": todos}
        
    if step == "search":
        # Only return "extra" results - hamr appends builtin results
        if query:
            return {"type": "results", "results": [
                {"id": "__add__", "name": f"Add: {query}", "icon": "add_circle"}
            ]}
        return {"type": "results", "results": []}
        
    if step == "action":
        # Handle item selection
```

## Affected Plugins

All plugins with indexes automatically benefit:

| Plugin | Items | Benefit |
|--------|-------|---------|
| `apps` | ~50-200 | Learned shortcuts ("ff" → Firefox) |
| `clipboard` | ~100 | Frecency-ranked clipboard items |
| `emoji` | ~1800 | Most-used emojis rank higher |
| `bitwarden` | varies | Frequently accessed passwords first |
| `zoxide` | ~50 | Already frecency-based, gets shortcuts |
| `shell` | ~100 | Recent commands rank higher |
| `sound` | ~5 | Quick access to volume controls |
| `niri`/`hyprland` | ~90 | Learned "w3" for "workspace 3" |
| `todo` | varies | Frecency for repeated tasks |
| `notes` | varies | Recent notes first |
| `quicklinks` | varies | Frequently used links |
| `snippet` | varies | Most-used snippets |

## Performance

**No caching needed initially:**
- `pluginIndexes` data already in memory
- Building searchables for 50-200 items: sub-millisecond
- `Fuzzy.go()` is optimized, already searches 2600+ items in main search
- Can add caching later if needed for large plugins (emoji with 1800 items)

## Edge Cases

### 1. Plugin without index
Falls back to handler search (existing behavior).

### 2. Plugin with daemon
Works the same - daemon provides index via `index` response, builtin search uses it.

### 3. Plugin context during search
Builtin search ignores `pluginContext` - always searches all items.
Handler still receives context for `action` step.

### 4. Empty search results
Return empty array, UI shows "No results" state.

### 5. staticIndex plugins
Plugins using `staticIndex` in manifest also get builtin search since their items are loaded into `pluginIndexes`.

### 6. Handler prepend with empty results
If handler returns empty results for search step, only builtin results are shown.

### 7. Deduplication
If handler returns an item that also exists in builtin results (same `id`), the handler version takes precedence (it appears first, builtin duplicate is filtered).

## Migration Guide

### Plugins that need migration

Plugins that currently do their own filtering in search step should:
1. Add `index` step to return all searchable items
2. Simplify `search` step to only return "extra" results (or remove entirely)

**Before (todo plugin):**
```python
if step == "search":
    if query:
        # Manual filtering - REMOVE THIS
        filtered = [t for t in todos if query.lower() in t["content"].lower()]
        results = [{"id": "__add__", "name": f"Add: {query}"}] + filtered
```

**After (todo plugin):**
```python
if step == "index":
    # Return all todos for builtin search
    return {"type": "index", "results": [
        {"id": f"todo:{i}", "name": t["content"], ...} for i, t in enumerate(todos)
    ]}

if step == "search":
    # Only return the "Add" shortcut - hamr appends matching todos
    if query:
        return {"type": "results", "results": [
            {"id": "__add__", "name": f"Add: {query}", "icon": "add_circle"}
        ]}
```

## Future Enhancements

1. **Category-aware builtin search** - Optionally filter by context
2. **Custom scoring weights** - Per-plugin frecency influence via manifest
3. **Search result caching** - For very large indexes
4. **Prefix boost** - Exact prefix matches rank higher
