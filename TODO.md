# Hamr TODO

## Future Enhancements

- [ ] **Quicklinks inline query syntax**: Support `linkname query` in main search (e.g., `gh hyprland` opens GitHub search)
  - Currently quicklinks with `{query}` require clicking to open plugin first
  - Could detect quicklink match + space + text pattern and execute directly
  - Would need indexed items to support `acceptsArguments` pattern

## Plugin Index Protocol

### Completed

- [x] **Match patterns**: Auto-trigger plugin when query matches regex patterns from `manifest.json`
  - Implemented in `PluginRunner.qml`: `buildMatchPatternCache()`, `findMatchingPlugin()`
  - Integrated in `LauncherSearch.qml`: `matchPatternCheckTimer`, `startPluginWithQuery()`
  - Calculate plugin now handles all math via match patterns

- [x] **Delete CalcUtils.qml**: Calculator plugin now handles all math
  - Removed `modules/common/functions/CalcUtils.qml`
  - Removed inline math code from `LauncherSearch.qml`

- [x] **Index persistence**: Cache indexes to disk for faster startup
  - Added `Directories.pluginIndexCache` path (`~/.config/hamr/plugin-indexes.json`)
  - Added `FileView` for loading cache on startup
  - Added `saveIndexCache()` with debounced writes (1s delay)
  - Incremental reindex when cache exists, full reindex otherwise
  - Cache includes version, savedAt timestamp, and per-plugin index data

- [x] **Plugin index support**: Added index support to commonly used plugins
  - `clipboard` - 50 recent entries, reindex every 30s
  - `shell` - 30 recent commands, reindex every 1m  
  - `quicklinks` - All user quicklinks, reindex every 5m

- [x] **Index search isolation**: Search within specific plugin's index using prefix
  - Use `pluginId:query` pattern (e.g., `emoji:smile`, `apps:fire`, `clipboard:code`)
  - Added `parseIndexIsolationPrefix()` in LauncherSearch
  - Added `getIndexedItemsForPlugin()` and `getIndexedPluginIds()` in PluginRunner
  - Searches only that plugin's indexed items, ignoring other sources

- [x] **File watching for index reindex**: Watch plugin data files for changes
  - Added `watchFiles` array to manifest `index` config
  - Implemented `setupFileWatchers()` in PluginRunner.qml using FileView
  - Debounced reindex (500ms) to handle rapid file changes
  - Updated `quicklinks` and `shell` plugins to use watchFiles
  - More efficient than polling - only reindex when data changes

## Plugins to Add Index Support

- [x] `clipboard` - Index recent clipboard entries (reindex: 30s)
- [x] `shell` - Index shell history commands (watchFiles: history files)
- [x] `quicklinks` - Index user quicklinks (watchFiles: quicklinks.json)
- [x] `snippet` - Index saved snippets (watchFiles: snippets.json)
- [x] `notes` - Index note titles (watchFiles: notes.json)
- [x] `bitwarden` - Index vault item names (watchFiles: cache file, no passwords!)
