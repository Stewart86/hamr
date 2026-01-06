pragma Singleton
pragma ComponentBehavior: Bound

import qs
import QtQuick
import Qt.labs.folderlistmodel
import Quickshell
import Quickshell.Io
import qs.modules.common
import qs.modules.common.functions
import qs.services

/**
 * PluginRunner - Multi-step action plugin execution service
 * 
 * Manages bidirectional JSON communication with plugin handler scripts.
 * Plugins are folders in ~/.config/hamr/plugins/ containing:
 *   - manifest.json: Plugin metadata and configuration
 *   - handler.py: Executable script that processes JSON protocol
 * 
 * Protocol:
 *   Input (stdin to script):
 *     { "step": "initial|search|action", "query": "...", "selected": {...}, "action": "...", "session": "..." }
 *   
 *   Output (stdout from script):
 *     { "type": "results|card|execute|prompt|error", ... }
 */
Singleton {
    id: root

    // ==================== ACTIVE PLUGIN STATE ====================
    property var activePlugin: null  // { id, path, manifest, session }
    property var pluginResults: []   // Current results from plugin
    property var pluginCard: null    // Card to display (title, content, markdown)
    property var pluginForm: null    // Form to display (title, fields, submitLabel)
    property string pluginPrompt: "" // Custom prompt text
    property string pluginPlaceholder: "" // Custom placeholder text for search bar
    property bool pluginBusy: false  // True while waiting for script response
    property string pluginError: ""  // Last error message
    property var lastSelectedItem: null // Last selected item (persisted across search calls)
    property string pluginContext: ""  // Custom context string for multi-step flows
    
    // Navigation depth - tracks how many steps into the plugin we are
    // Incremented on action/selection steps, decremented on back
    // When depth is 0, back/Escape closes the plugin entirely
    property int navigationDepth: 0
    
    // Pending typeText - waits for launcher to close before typing
    property string pendingTypeText: ""
    
    // Flags to track pending navigation actions for depth management
    property bool pendingNavigation: false  // True when action may navigate forward
    property bool pendingBack: false        // True when goBack() called (back navigation)
    
    // Plugin-level actions (toolbar buttons, not item-specific)
    // Each action: { id, name, icon, confirm?: string }
    // If confirm is set, show confirmation dialog before executing
    property var pluginActions: []
    
    // Input mode: "realtime" (every keystroke) or "submit" (only on Enter)
    // Handler controls this via response - allows different modes per step
    property string inputMode: "realtime"
    
    
    // Version counter - increments on any result item update to trigger UI re-evaluation
    // SearchItem depends on this to re-evaluate visual properties (gauge, progress, badges, etc.)
    property int resultsVersion: 0
    
    // Pending builtin search results - used for hybrid search (handler prepend + builtin)
    // When a plugin has both indexed items and a search handler, we:
    // 1. Do builtin search and store results here
    // 2. Call handler's search step
    // 3. Prepend handler results to these builtin results
    property var _pendingBuiltinResults: null
    
    // Track last search query - sent with action step so handlers can filter results
    property string _lastSearchQuery: ""
    
    // Track handler prepend results from last search (e.g., "Add" item for todo)
    // Used to restore these when re-filtering after action
    property var _lastHandlerPrependResults: []
    
     // Replay mode: when true, plugin is running a replay action (no UI needed)
     // Process should complete even if launcher closes
     property bool replayMode: false
     
    // Store plugin info for replay mode (activePlugin may be cleared before response)
    property var replayPluginInfo: null

    // ==================== DAEMON SUPPORT ====================
    // Track running daemon processes: { pluginId: { process, restartCount, isBackground, inputWriter } }
    property var runningDaemons: ({})
    
    // Signal when plugin produces results
    signal resultsReady(var results)
    signal cardReady(var card)
    signal formReady(var form)
    signal executeCommand(var command)
    signal pluginClosed()
    signal clearInputRequested()  // Signal to clear the search input
    
    // Signal when a trackable action is executed (has name field)
    // Payload: { name, command, entryPoint, icon, thumbnail, workflowId, workflowName }
    // - command: Direct shell command for simple replay (optional)
    // - entryPoint: Plugin step to replay for complex actions (optional)
    signal actionExecuted(var actionInfo)
    
    // Signal when plugin index is updated (for LauncherSearch to rebuild searchables)
    signal pluginIndexChanged(string pluginId)

    // ==================== PLUGIN STATUS ====================
    // Plugins can provide dynamic status (badges, description override) that
    // appears on their entry in the main launcher list.
    // Updated via: response.status field, or IPC call "hamr status <plugin> <json>"
    
    // Status per plugin: { pluginId: { badges: [...], description: "..." } }
    // Using a non-reactive internal object to avoid triggering full results rebuild
    property var _pluginStatusesInternal: ({})
    
    // Version counter - increments on any status update to trigger badge refresh
    // SearchItem depends on this to re-evaluate badges without rebuilding results list
    property int statusVersion: 0
    
    function getPluginStatus(pluginId) {
        return root._pluginStatusesInternal[pluginId] ?? null;
    }
    
    function updatePluginStatus(pluginId, status) {
        if (!status || typeof status !== "object") return;
        
        root._pluginStatusesInternal[pluginId] = {
            badges: status.badges ?? [],
            chips: status.chips ?? [],
            description: status.description ?? null
        };
        root.statusVersion++;
        
        // Handle FAB override
        if (status.fab !== undefined) {
            GlobalStates.updateFabOverride(pluginId, status.fab);
        }
        
        // Handle ambient items
        if (status.ambient !== undefined) {
            root.updateAmbientItems(pluginId, status.ambient);
        }
    }
    
    // ==================== AMBIENT ITEMS ====================
    // Persistent items shown above search bar in main view
    // Each plugin can have multiple ambient items
    
    property var _ambientItemsInternal: ({})
    property int ambientVersion: 0
    
    function getAmbientItems() {
        const allItems = [];
        for (const pluginId in root._ambientItemsInternal) {
            const items = root._ambientItemsInternal[pluginId] ?? [];
            for (const item of items) {
                allItems.push(Object.assign({}, item, { pluginId: pluginId }));
            }
        }
        return allItems;
    }
    
    function updateAmbientItems(pluginId, items) {
        if (items === null || items === undefined) {
            if (root._ambientItemsInternal[pluginId]) {
                delete root._ambientItemsInternal[pluginId];
                root._ambientItemsInternal = Object.assign({}, root._ambientItemsInternal);
                root.ambientVersion++;
            }
            return;
        }
        
        if (!Array.isArray(items)) {
            items = [items];
        }
        
        // Set up auto-remove timers for items with duration
        const processedItems = items.map(item => {
            if (item.duration && item.duration > 0) {
                Qt.callLater(() => {
                    root._scheduleAmbientRemoval(pluginId, item.id, item.duration);
                });
            }
            return item;
        });
        
        root._ambientItemsInternal[pluginId] = processedItems;
        root._ambientItemsInternal = Object.assign({}, root._ambientItemsInternal);
        root.ambientVersion++;
    }
    
    function removeAmbientItem(pluginId, itemId) {
        const items = root._ambientItemsInternal[pluginId];
        if (!items) return;
        
        const filtered = items.filter(item => item.id !== itemId);
        if (filtered.length === 0) {
            delete root._ambientItemsInternal[pluginId];
        } else {
            root._ambientItemsInternal[pluginId] = filtered;
        }
        root._ambientItemsInternal = Object.assign({}, root._ambientItemsInternal);
        root.ambientVersion++;
    }
    
    property var _ambientTimers: ({})
    
    function _scheduleAmbientRemoval(pluginId, itemId, duration) {
        const key = `${pluginId}:${itemId}`;
        if (root._ambientTimers[key]) {
            root._ambientTimers[key].destroy();
        }
        
        const timer = Qt.createQmlObject(
            `import QtQuick; Timer { interval: ${duration}; repeat: false; running: true }`,
            root
        );
        timer.triggered.connect(() => {
            root.removeAmbientItem(pluginId, itemId);
            timer.destroy();
            delete root._ambientTimers[key];
        });
        root._ambientTimers[key] = timer;
    }
    
    function handleAmbientAction(pluginId, itemId, actionId) {
        root.writeToDaemonStdin(pluginId, {
            step: "action",
            selected: { id: itemId },
            action: actionId,
            source: "ambient"
        });
    }

    // ==================== PLUGIN INDEXING ====================
    // Plugins provide searchable items in two ways:
    // 1. staticIndex in manifest.json - items loaded directly, no handler needed
    // 2. Daemon plugins emit {"type": "index"} messages autonomously
    
    // Indexed items per plugin: { pluginId: { items: [...], lastIndexed: timestamp } }
    property var pluginIndexes: ({})
    
    // Handle index response from daemon plugin
    // Preserve frecency fields when merging index items
    function mergeItemPreservingFrecency(existingItem, newItem) {
        const merged = Object.assign({}, newItem);
        // Preserve all frecency fields (prefixed with _)
        if (existingItem) {
            for (const key of Object.keys(existingItem)) {
                if (key.startsWith('_') && merged[key] === undefined) {
                    merged[key] = existingItem[key];
                }
            }
        }
        return merged;
    }
    
    function handleIndexResponse(pluginId, response) {
        if (!response || response.type !== "index") {
            console.warn(`[PluginRunner] Invalid index response from ${pluginId}`);
            return;
        }
        
        const isIncremental = response.mode === "incremental";
        const itemCount = response.items?.length ?? 0;
        const now = Date.now();
        
        // Build map of existing items for frecency preservation
        const existingItems = root.pluginIndexes[pluginId]?.items ?? [];
        const existingMap = new Map(existingItems.map(item => [item.id, item]));
        
        if (isIncremental && root.pluginIndexes[pluginId]) {
            // Incremental: merge new items, remove deleted
            const newItems = response.items ?? [];
            const removeIds = new Set(response.remove ?? []);
            
            // Debug: check if we're removing items with frecency
            for (const removeId of removeIds) {
                const existingItem = existingMap.get(removeId);
                if (existingItem?._count > 0) {
                    console.log(`[PluginRunner] WARNING: removing item with frecency: ${pluginId}/${removeId} count=${existingItem._count}`);
                }
            }
            
            // Remove deleted items
            let merged = existingItems.filter(item => !removeIds.has(item.id));
            
            // Update or add new items, preserving frecency
            const mergedIds = new Set(merged.map(item => item.id));
            for (const item of newItems) {
                if (mergedIds.has(item.id)) {
                    // Update existing - preserve frecency
                    merged = merged.map(i => i.id === item.id 
                        ? root.mergeItemPreservingFrecency(i, item) 
                        : i);
                } else {
                    // Add new
                    merged.push(item);
                }
            }
            
            root.pluginIndexes[pluginId] = {
                items: merged,
                lastIndexed: now
            };
        } else {
            // Full: replace items but preserve frecency from existing
            const newItems = (response.items ?? []).map(item => 
                root.mergeItemPreservingFrecency(existingMap.get(item.id), item)
            );
            
            // Preserve __plugin__ entry if it exists (for plugin-level frecency)
            const pluginEntry = existingMap.get("__plugin__");
            if (pluginEntry) {
                newItems.push(pluginEntry);
            }
            
            root.pluginIndexes[pluginId] = {
                items: newItems,
                lastIndexed: now
            };
        }
        
        // Update plugin status if provided in index response
        if (response.status) {
            root.updatePluginStatus(pluginId, response.status);
        }
        
        // Notify listeners (LauncherSearch) that index changed
        root.pluginIndexChanged(pluginId);
        
        // Save cache to disk (debounced)
        root.saveIndexCache();
    }
    
    // Load static index items from plugin manifests (no handler needed)
    function loadStaticIndexes() {
        for (const plugin of root.plugins) {
            const staticIndex = plugin.manifest?.staticIndex;
            if (!staticIndex || !Array.isArray(staticIndex) || staticIndex.length === 0) {
                continue;
            }
            
            // Enrich items with plugin metadata
            const items = staticIndex.map(item => Object.assign({}, item, {
                _pluginId: plugin.id,
                _pluginName: plugin.manifest?.name ?? plugin.id
            }));
            
            root.pluginIndexes[plugin.id] = {
                items: items,
                lastIndexed: Date.now()
            };
            
            root.pluginIndexChanged(plugin.id);
        }
    }
    
    // Get all indexed items across all plugins (for LauncherSearch)
    function getAllIndexedItems() {
        const allItems = [];
        const pluginMap = new Map(root.plugins.map(p => [p.id, p]));
        for (const [pluginId, indexData] of Object.entries(root.pluginIndexes)) {
            const plugin = pluginMap.get(pluginId);
            const pluginName = plugin?.manifest?.name ?? pluginId;
            
            for (const item of (indexData.items ?? [])) {
                // Skip __plugin__ entries (used for plugin-level frecency, not search)
                if (item.id === "__plugin__" || item._isPluginEntry) continue;
                
                const enrichedItem = Object.assign({}, item, {
                    _pluginId: pluginId,
                    _pluginName: pluginName
                });
                allItems.push(enrichedItem);
            }
        }
        return allItems;
    }
    
    // Get indexed items for a specific plugin (for isolated search)
    function getIndexedItemsForPlugin(pluginId) {
        const indexData = root.pluginIndexes[pluginId];
        if (!indexData?.items) return [];
        
        const plugin = root.plugins.find(p => p.id === pluginId);
        const pluginName = plugin?.manifest?.name ?? pluginId;
        
        return indexData.items.map(item => Object.assign({}, item, {
            _pluginId: pluginId,
            _pluginName: pluginName
        }));
    }
    
    // Get list of plugins that have indexed items (for prefix autocomplete)
    function getIndexedPluginIds() {
        return Object.keys(root.pluginIndexes).filter(id => 
            root.pluginIndexes[id]?.items?.length > 0
        );
    }
    
    // Get a single indexed item by plugin ID and item ID
    function getIndexedItem(pluginId, itemId) {
        const indexData = root.pluginIndexes[pluginId];
        if (!indexData?.items) return null;
        return indexData.items.find(item => item.id === itemId) ?? null;
    }
    
    // ==================== FRECENCY & LIVE UPDATES ====================
    // Index items store frecency data directly (prefixed with _):
    //   _count, _lastUsed, _recentSearchTerms, _hourSlotCounts, etc.
    // This makes index the single source of truth for both display data and frecency.
    
    // Version counter for UI reactivity - increments on any index item update
    // SearchItem depends on this to re-evaluate live values (gauge, slider, etc.)
    property int indexVersion: 0
    
    // Record execution - updates frecency based on plugin's frecency mode
    // Manifest frecency modes:
    //   "item" (default) - Track individual item usage (apps, sound sliders)
    //   "plugin" - Track plugin usage only, not individual items (todo, notes)
    //   "none" - Don't track frecency at all
    // launchFromEmpty: boolean - true if launched without search query (for smart suggestions)
    function recordExecution(pluginId, itemId, searchTerm, launchFromEmpty) {
        const plugin = root.plugins.find(p => p.id === pluginId);
        const frecencyMode = plugin?.manifest?.frecency ?? "item";
        
        if (frecencyMode === "none") {
            return;
        }
        
        const now = Date.now();
        
        if (frecencyMode === "plugin") {
            // Plugin-level frecency: record on the plugin's metadata, not items
            if (!root.pluginIndexes[pluginId]) {
                root.pluginIndexes[pluginId] = { items: [] };
            }
            const indexData = root.pluginIndexes[pluginId];
            
            // Store on a special __plugin__ entry in the index
            let pluginEntry = indexData.items.find(item => item.id === "__plugin__");
            if (!pluginEntry) {
                // Create a virtual plugin entry for frecency tracking
                pluginEntry = {
                    id: "__plugin__",
                    name: plugin?.manifest?.name ?? pluginId,
                    icon: plugin?.manifest?.icon ?? "extension",
                    verb: "Open",
                    _isPluginEntry: true
                };
                indexData.items.push(pluginEntry);
            }
            pluginEntry._count = (pluginEntry._count ?? 0) + 1;
            pluginEntry._lastUsed = now;
        } else {
            // Item-level frecency (default)
            // Skip __plugin__ calls for item-level plugins (they only track specific items)
            if (itemId === "__plugin__") {
                return;
            }
            
            const indexData = root.pluginIndexes[pluginId];
            if (!indexData?.items) {
                return;
            }
            
            const item = indexData.items.find(item => item.id === itemId);
            if (!item) {
                return;
            }
            
            item._count = (item._count ?? 0) + 1;
            item._lastUsed = now;
            
            // Update recent search terms
            if (searchTerm) {
                let terms = item._recentSearchTerms ?? [];
                terms = terms.filter(t => t !== searchTerm);
                terms.unshift(searchTerm);
                item._recentSearchTerms = terms.slice(0, 10);
            }
            
            // Update smart fields for apps plugin (contextual tracking)
            if (pluginId === "apps") {
                // Record app launch for sequence tracking (uses appId, not itemId)
                if (item.appId) {
                    ContextTracker.recordLaunch(item.appId);
                }
                const context = ContextTracker.getContext();
                context.launchFromEmpty = launchFromEmpty ?? false;
                root.updateItemSmartFields(item, context);
            }
        }
        
        // Save to disk (debounced)
        root.saveIndexCache();
    }
    
    // Update smart/contextual fields on an item
    function updateItemSmartFields(item, context) {
        const now = Date.now();
        const hour = Math.floor(new Date(now).getHours());
        const day = new Date(now).getDay();
        
        // Hour slot counts
        if (!item._hourSlotCounts) item._hourSlotCounts = new Array(24).fill(0);
        item._hourSlotCounts[hour] = (item._hourSlotCounts[hour] ?? 0) + 1;
        
        // Day of week counts (Monday=0, Sunday=6 to match ContextTracker)
        const adjustedDay = day === 0 ? 6 : day - 1;
        if (!item._dayOfWeekCounts) item._dayOfWeekCounts = new Array(7).fill(0);
        item._dayOfWeekCounts[adjustedDay] = (item._dayOfWeekCounts[adjustedDay] ?? 0) + 1;
        
        // Workspace counts
        if (context.workspace) {
            if (!item._workspaceCounts) item._workspaceCounts = {};
            item._workspaceCounts[context.workspace] = (item._workspaceCounts[context.workspace] ?? 0) + 1;
        }
        
        // Monitor counts
        if (context.monitor) {
            if (!item._monitorCounts) item._monitorCounts = {};
            item._monitorCounts[context.monitor] = (item._monitorCounts[context.monitor] ?? 0) + 1;
        }
        
        // Launched after (sequence tracking)
        if (context.lastApp) {
            if (!item._launchedAfter) item._launchedAfter = {};
            item._launchedAfter[context.lastApp] = (item._launchedAfter[context.lastApp] ?? 0) + 1;
            const entries = Object.entries(item._launchedAfter);
            if (entries.length > 5) {
                entries.sort((a, b) => b[1] - a[1]);
                item._launchedAfter = Object.fromEntries(entries.slice(0, 5));
            }
        }
        
        // Session start tracking
        if (context.isSessionStart) {
            item._sessionStartCount = (item._sessionStartCount ?? 0) + 1;
        }
        
        // Resume from idle tracking
        if (context.isResumeFromIdle) {
            item._resumeFromIdleCount = (item._resumeFromIdleCount ?? 0) + 1;
        }
        
        // Launch from empty (no search query) tracking
        if (context.launchFromEmpty) {
            item._launchFromEmptyCount = (item._launchFromEmptyCount ?? 0) + 1;
        }
        
        // Display count tracking
        if (context.displayCount) {
            if (!item._displayCountCounts) item._displayCountCounts = {};
            const key = String(context.displayCount);
            item._displayCountCounts[key] = (item._displayCountCounts[key] ?? 0) + 1;
        }
        
        // Session duration bucket tracking
        if (context.sessionDurationBucket >= 0) {
            if (!item._sessionDurationCounts) item._sessionDurationCounts = new Array(5).fill(0);
            item._sessionDurationCounts[context.sessionDurationBucket] = 
                (item._sessionDurationCounts[context.sessionDurationBucket] ?? 0) + 1;
        }
        
        // Consecutive days tracking
        const today = new Date().toISOString().split('T')[0];
        const yesterday = new Date(Date.now() - 86400000).toISOString().split('T')[0];
        if (item._lastConsecutiveDate === today) {
            // Already used today
        } else if (item._lastConsecutiveDate === yesterday) {
            item._consecutiveDays = (item._consecutiveDays ?? 0) + 1;
            item._lastConsecutiveDate = today;
        } else {
            item._consecutiveDays = 1;
            item._lastConsecutiveDate = today;
        }
    }
    
    // Patch indexed items (for live updates like slider changes from daemon)
    function patchIndexItems(pluginId, patches) {
        if (!patches || !Array.isArray(patches)) return;
        
        const indexData = root.pluginIndexes[pluginId];
        if (!indexData?.items) return;
        
        let patchedCount = 0;
        for (const patch of patches) {
            if (!patch.id) continue;
            const item = indexData.items.find(i => i.id === patch.id);
            if (item) {
                // Merge patch into item (preserves frecency fields)
                Object.assign(item, patch);
                patchedCount++;
            }
        }
        
        // Trigger UI update
        root.indexVersion++;
        
        // Save to disk (debounced)
        root.saveIndexCache();
    }
    
    // Get frecency score for an indexed item (used by FrecencyScorer)
    function getItemFrecency(pluginId, itemId) {
        const item = root.getIndexedItem(pluginId, itemId);
        if (!item) return 0;
        
        const count = item._count ?? 0;
        const lastUsed = item._lastUsed ?? 0;
        if (count === 0) return 0;
        
        const now = Date.now();
        const hoursSinceUse = (now - lastUsed) / (1000 * 60 * 60);
        
        let recencyMultiplier;
        if (hoursSinceUse < 1) recencyMultiplier = 4;
        else if (hoursSinceUse < 24) recencyMultiplier = 2;
        else if (hoursSinceUse < 168) recencyMultiplier = 1;
        else recencyMultiplier = 0.5;
        
        return count * recencyMultiplier;
    }
    
    // Get all items with frecency data (for building history searchables)
    function getItemsWithFrecency() {
        const items = [];
        for (const [pluginId, indexData] of Object.entries(root.pluginIndexes)) {
            for (const item of (indexData.items ?? [])) {
                if (item._count > 0) {
                    items.push({
                        pluginId,
                        item
                    });
                }
            }
        }
        // Sort by frecency (most recent/frequent first)
        items.sort((a, b) => {
            const freqA = root.getItemFrecency(a.pluginId, a.item.id);
            const freqB = root.getItemFrecency(b.pluginId, b.item.id);
            return freqB - freqA;
        });
        return items;
    }
    
    // ==================== BUILTIN SEARCH ====================
    // Automatically use hamr's search algorithm for plugins with indexed items.
    // Provides fuzzy matching + frecency + learned shortcuts.
    // 
    // Hybrid mode: If plugin has both indexed items AND a search handler,
    // handler results are prepended to builtin results. This allows plugins
    // to inject custom results (like "Add: {query}" for todo) while still
    // benefiting from builtin fuzzy+frecency search.
    
    function doBuiltinSearch(pluginId, query) {
        const indexData = root.pluginIndexes[pluginId];
        if (!indexData?.items) {
            root.pluginResults = [];
            root._pendingBuiltinResults = null;
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
            root._pendingBuiltinResults = null;
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
        const builtinResults = [];
        for (const match of fuzzyResults) {
            const item = match.obj.item;
            if (seen.has(item.id)) continue;
            seen.add(item.id);
            builtinResults.push(item);
        }
        
        // Check if plugin has a handler (not staticIndex only)
        // If so, also call handler and merge results (handler prepends)
        const manifest = root.activePlugin?.manifest;
        const hasHandler = manifest && !manifest.staticIndex;
        
        if (hasHandler) {
            // Store builtin results, call handler, merge when response arrives
            root._pendingBuiltinResults = builtinResults;
            root.callHandlerSearch(pluginId, query);
        } else {
            // No handler - just use builtin results
            root._pendingBuiltinResults = null;
            root.pluginResults = builtinResults;
        }
    }
    
    // Builtin search without calling handler - used after action responses
    // to filter results without re-calling handler (avoids flicker)
    // prependResults: optional results to prepend (e.g., "Add" item from previous response)
    function doBuiltinSearchOnly(pluginId, query, prependResults) {
        const indexData = root.pluginIndexes[pluginId];
        if (!indexData?.items) {
            return;
        }
        
        const searchables = [];
        for (const item of indexData.items) {
            if (item.id === "__plugin__" || item._isPluginEntry) continue;
            
            searchables.push({
                name: Fuzzy.prepare(item.name),
                keywords: item.keywords?.length > 0 ? Fuzzy.prepare(item.keywords.join(" ")) : null,
                item: item,
                isHistoryTerm: false
            });
            
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
            root.pluginResults = prependResults ?? [];
            return;
        }
        
        const fuzzyResults = Fuzzy.go(query, searchables, {
            keys: ["name", "keywords"],
            limit: 100,
            threshold: 0.25,
            scoreFn: (result) => {
                const searchable = result.obj;
                const nameScore = result[0]?.score ?? 0;
                const keywordsScore = result[1]?.score ?? 0;
                const baseScore = nameScore * 1.0 + keywordsScore * 0.3;
                const frecency = root.getItemFrecency(pluginId, searchable.item.id);
                const frecencyBoost = Math.min(frecency * 0.02, 0.3);
                const historyBoost = searchable.isHistoryTerm ? 0.2 : 0;
                return baseScore + frecencyBoost + historyBoost;
            }
        });
        
        const seen = new Set();
        const builtinResults = [];
        for (const match of fuzzyResults) {
            const item = match.obj.item;
            if (seen.has(item.id)) continue;
            seen.add(item.id);
            builtinResults.push(item);
        }
        
        // Prepend any special results (like "Add" item)
        if (prependResults && prependResults.length > 0) {
            const prependIds = new Set(prependResults.map(r => r.id));
            const dedupedBuiltin = builtinResults.filter(r => !prependIds.has(r.id));
            root.pluginResults = prependResults.concat(dedupedBuiltin);
        } else {
            root.pluginResults = builtinResults;
        }
        root.resultsVersion++;
    }
    
    // Call handler's search step for hybrid mode
    function callHandlerSearch(pluginId, query) {
        const input = {
            step: "search",
            query: query,
            session: root.activePlugin?.session ?? ""
        };
        
        if (root.lastSelectedItem) {
            input.selected = { id: root.lastSelectedItem };
        }
        if (root.pluginContext) {
            input.context = root.pluginContext;
        }
        
        const isDaemonPlugin = root.activePlugin?.manifest?.daemon?.enabled;
        if (isDaemonPlugin && root.runningDaemons[pluginId]) {
            root.pluginBusy = true;
            root.writeToDaemonStdin(pluginId, input);
        } else {
            sendToPlugin(input);
        }
    }
    
    // Check if plugin has indexed items (for builtin search decision)
    function hasIndexedItems(pluginId) {
        const indexData = root.pluginIndexes[pluginId];
        if (!indexData?.items) return false;
        // Check for actual items (not just __plugin__ entry)
        return indexData.items.some(item => item.id !== "__plugin__" && !item._isPluginEntry);
    }
    
    // ==================== INDEX PERSISTENCE ====================
    // Cache indexes to disk for faster startup.
    // On startup: load cache, then trigger incremental reindex.
    // After indexing: save cache to disk.
    // =============================================================
    
    property bool indexCacheLoaded: false
    
    // Load cached indexes from disk
    FileView {
        id: indexCacheFile
        path: Directories.pluginIndexCache
        
        onLoaded: {
            try {
                const data = JSON.parse(indexCacheFile.text());
                if (data.indexes && typeof data.indexes === "object") {
                    root.pluginIndexes = data.indexes;
                    
                    for (const pluginId of Object.keys(data.indexes)) {
                        root.pluginIndexChanged(pluginId);
                    }
                }
            } catch (e) {
                console.warn("[PluginRunner] Failed to parse index cache:", e);
            }
            root.indexCacheLoaded = true;
        }
        
        onLoadFailed: error => {
            if (error !== FileViewError.FileNotFound) {
                console.warn("[PluginRunner] Failed to load index cache:", error);
            }
            root.indexCacheLoaded = true;
        }
    }
    
    // Save indexes to disk (debounced to avoid excessive writes)
    Timer {
        id: saveIndexCacheTimer
        interval: 1000  // Wait 1 second after last index change before saving
        onTriggered: root.doSaveIndexCache()
    }
    
    function saveIndexCache() {
        saveIndexCacheTimer.restart();
    }
    
    function doSaveIndexCache() {
        const data = {
            version: 1,
            savedAt: Date.now(),
            indexes: root.pluginIndexes
        };
        const json = JSON.stringify(data);
        // Use FileView.setText to write
        indexCacheFile.setText(json);
    }

     // ==================== DAEMON LIFECYCLE ====================
     
     // Start a daemon for a plugin (spawn once, keep running)
     function startDaemon(pluginId) {
         const plugin = root.plugins.find(p => p.id === pluginId);
         if (!plugin || !plugin.manifest || !plugin.manifest.daemon?.enabled) {
             return false;
         }
         
         // Already running
         if (root.runningDaemons[pluginId]) {
             return true;
         }
         
         const daemonConfig = plugin.manifest.daemon;
         const handlerPath = plugin.manifest._handlerPath ?? (plugin.path + "/handler.py");
         
         // Create persistent daemon process
         const process = Qt.createQmlObject(`
             import Quickshell.Io
             Process {
                 running: true
                 stdinEnabled: true
                 command: ["${handlerPath}"]
                 workingDirectory: "${plugin.path}"
                 
                 stdout: SplitParser {
                     splitMarker: "\\n"
                     onRead: data => root.handleDaemonStdout("${pluginId}", data)
                 }
                 
                 stderr: SplitParser {
                     onRead: data => console.warn("[Daemon ${pluginId}] stderr:", data)
                 }
                 
                 onExited: (code, status) => root.onDaemonExit("${pluginId}", code, status)
             }
         `, root, "daemon_" + pluginId);
         
         root.runningDaemons[pluginId] = {
             process: process,
             restartCount: 0,
             isBackground: daemonConfig.background ?? false,
             config: daemonConfig,
             plugin: plugin,
             handlerPath: handlerPath,
             session: generateSessionId()
         };
         
         return true;
     }
    
     // Stop a daemon process
     function stopDaemon(pluginId) {
         const daemon = root.runningDaemons[pluginId];
         if (!daemon) return false;
         
         if (daemon.process) {
             daemon.process.running = false;
             daemon.process.destroy();
         }
         
         delete root.runningDaemons[pluginId];
         return true;
     }
    
    // Start all background daemons (call on hamr startup)
    function startBackgroundDaemons() {
        if (!root.pluginsLoaded) return;
        
        for (const plugin of root.plugins) {
            const daemonConfig = plugin.manifest?.daemon;
            if (daemonConfig?.enabled && daemonConfig?.background) {
                root.startDaemon(plugin.id);
            }
        }
    }
    
    // Stop all running daemons (call on hamr shutdown)
    function stopAllDaemons() {
        const pluginIds = Object.keys(root.runningDaemons);
        for (const pluginId of pluginIds) {
            root.stopDaemon(pluginId);
        }
    }
    
     // Write command to daemon stdin
     function writeToDaemonStdin(pluginId, command) {
         const daemon = root.runningDaemons[pluginId];
         if (!daemon || !daemon.process) {
             console.warn(`[PluginRunner] Cannot write to daemon ${pluginId}: not registered or process missing`);
             return false;
         }
         
         // Preserve session from daemon
         if (daemon.session) {
             command.session = daemon.session;
         }
         
         const json = JSON.stringify(command) + "\n";
         daemon.process.write(json);
         return true;
     }
    
     // Parse daemon stdout line and emit response
     function handleDaemonStdout(pluginId, data) {
         if (!data || data.trim() === "") return;
         
         try {
             const response = JSON.parse(data.trim());
             root.handleDaemonOutput(pluginId, response);
         } catch (e) {
             console.warn(`[PluginRunner] Failed to parse daemon output from ${pluginId}: ${e}`);
         }
     }
     
     // Handle daemon output/response
      function handleDaemonOutput(pluginId, response) {
         // Only process if this plugin is currently active
         const isActive = root.activePlugin?.id === pluginId;
         
         if (!response || !response.type) {
             console.warn(`[PluginRunner] Invalid daemon output from ${pluginId}`);
             return;
         }
         
           switch (response.type) {
               case "results":
               case "card":
               case "form":
               case "prompt":
               case "error":
               case "imageBrowser":
               case "gridBrowser":
                   // Always update status if provided (for FAB/ambient updates)
                   if (response.status) {
                       root.updatePluginStatus(pluginId, response.status);
                   }
                   // Only process UI responses if plugin is active
                   if (isActive) {
                       root.handlePluginResponse(response);
                   }
                   break;
               
               case "update":
                   // Always update status if provided
                   if (response.status) {
                       root.updatePluginStatus(pluginId, response.status);
                   }
                   // Patch the index with updated item data (for live updates)
                   if (response.items && Array.isArray(response.items)) {
                       root.patchIndexItems(pluginId, response.items);
                   }
                   // Also process UI update if plugin is active
                   if (isActive) {
                       root.handlePluginResponse(response);
                   }
                   break;
              
              case "status":
                  // Status updates always processed
                  root.updatePluginStatus(pluginId, response.status);
                  break;
              
              case "index":
                  // Index updates always processed
                  root.handleIndexResponse(pluginId, response);
                  break;
              
              case "execute":
                  // Execute responses always processed (for sounds, notifications)
                  root.handleExecuteResponse(response, pluginId);
                  break;
              
              default:
                  console.warn(`[PluginRunner] Unknown daemon response type: ${response.type}`);
          }
     }
    
    // Handle execute response from daemon - safe API only, no arbitrary commands
    function handleExecuteResponse(response, pluginId) {
        // Properties are directly on response (new format), not nested in execute
        root.processExecuteAction(response, pluginId);
    }
    
    // Process execute action using safe, whitelisted API
    function processExecuteAction(exec, pluginId) {
        // Launch .desktop file (detached)
        if (exec.launch) {
            Quickshell.execDetached(["gio", "launch", exec.launch]);
        }
        
        // Focus app window (compositor-specific)
        if (exec.focusApp) {
            const windows = WindowManager.getWindowsForApp(exec.focusApp);
            if (windows.length === 1) {
                WindowManager.focusWindow(windows[0]);
            } else if (windows.length > 1) {
                // Multiple windows - focus first for now
                // TODO: Could open picker here
                WindowManager.focusWindow(windows[0]);
            }
        }
        
        // Copy to clipboard
        if (exec.copy) {
            Quickshell.execDetached(["wl-copy", exec.copy]);
        }
        
        // Type text (input simulation via ydotool)
        if (exec.typeText) {
            if (exec.close) {
                // Defer typing until launcher closes and focus returns to target window
                root.pendingTypeText = exec.typeText;
            } else {
                // Type immediately
                Quickshell.execDetached(["ydotool", "type", "--clearmodifiers", "--", exec.typeText]);
            }
        }
        
        // Open URL
        if (exec.openUrl) {
            Qt.openUrlExternally(exec.openUrl);
        }
        
        // Open file/folder
        if (exec.open) {
            Quickshell.execDetached(["xdg-open", exec.open]);
        }
        
        // Play sound
        if (exec.sound) {
            AudioService.playSound(exec.sound);
        }
        
        // Show notification
        if (exec.notify) {
            const pluginName = root.activePlugin?.manifest?.name 
                ?? root.replayPluginInfo?.name 
                ?? "hamr";
            Quickshell.execDetached(["notify-send", "-a", pluginName, exec.notify]);
        }
        
        // Close launcher if requested
        if (exec.close && root.activePlugin?.id === pluginId) {
            root.executeCommand(exec);
        }
    }
    
    // Handle daemon crash/exit (for future use when full daemon communication is implemented)
    function onDaemonExit(pluginId, exitCode, exitStatus) {
        const daemon = root.runningDaemons[pluginId];
        if (!daemon) return;
        
        const config = daemon.config;
        const shouldRestart = config.restartOnCrash && 
                            daemon.restartCount < (config.maxRestarts ?? 3);
        
        if (shouldRestart) {
            daemon.restartCount++;
            console.log(`[PluginRunner] Restarting daemon for ${pluginId} (attempt ${daemon.restartCount}/${config.maxRestarts})`);
            
            // Reset restart count after successful run (when daemon runs for a while)
            daemon.process = null;
            
            // Restart after a short delay
            Qt.callLater(() => {
                root.startDaemon(pluginId);
            }, 1000);
        } else {
            console.log(`[PluginRunner] Daemon for ${pluginId} won't restart (maxRestarts reached or disabled)`);
            delete root.runningDaemons[pluginId];
            
            // Notify user if plugin was active
            if (root.activePlugin?.id === pluginId) {
                root.pluginError = "Plugin daemon crashed and won't restart";
            }
        }
    }
    
    // ==================== PLUGIN DISCOVERY ====================
    
    // Loaded plugins from both built-in and user plugins directories
    // Each plugin: { id, path, manifest: { name, description, icon, ... }, isBuiltin: bool }
    // User plugins override built-in plugins with the same id
    property var plugins: []
    // Plugins sorted by match priority (highest first) for efficient pattern matching
    property var pluginsByPriority: plugins.slice().sort((a, b) => 
        (b.manifest?.match?.priority ?? 0) - (a.manifest?.match?.priority ?? 0)
    )
    property var pendingManifestLoads: []
    property bool pluginsLoaded: false  // True when all manifests have been loaded
    property string pendingPluginStart: ""  // Plugin ID to start once loaded
    property bool builtinFolderReady: false
    property bool userFolderReady: false
    
    // Force refresh plugins - call this when launcher opens to detect new plugins
    // This works around FolderListModel not detecting changes in symlinked directories
    function refreshPlugins() {
        // Touch folder properties to force re-scan
        const builtinFolder = builtinPluginsFolder.folder;
        const userFolder = userPluginsFolder.folder;
        builtinPluginsFolder.folder = "";
        userPluginsFolder.folder = "";
        builtinPluginsFolder.folder = builtinFolder;
        userPluginsFolder.folder = userFolder;
    }
    
    // Load plugins from both directories
    // User plugins override built-in plugins with the same id
    function loadPlugins() {
        if (!root.builtinFolderReady || !root.userFolderReady) return;
        
        root.pendingManifestLoads = [];
        root.pluginsLoaded = false;
        
        const seenIds = new Set();
        
        // Load user plugins first (higher priority)
        for (let i = 0; i < userPluginsFolder.count; i++) {
            const fileName = userPluginsFolder.get(i, "fileName");
            const filePath = userPluginsFolder.get(i, "filePath");
            if (fileName && filePath) {
                seenIds.add(fileName);
                root.pendingManifestLoads.push({
                    id: fileName,
                    path: FileUtils.trimFileProtocol(filePath),
                    isBuiltin: false
                });
            }
        }
        
        // Load built-in plugins (skip if user has same id)
        for (let i = 0; i < builtinPluginsFolder.count; i++) {
            const fileName = builtinPluginsFolder.get(i, "fileName");
            const filePath = builtinPluginsFolder.get(i, "filePath");
            if (fileName && filePath && !seenIds.has(fileName)) {
                root.pendingManifestLoads.push({
                    id: fileName,
                    path: FileUtils.trimFileProtocol(filePath),
                    isBuiltin: true
                });
            }
        }
        
        root.plugins = [];
        
        if (root.pendingManifestLoads.length > 0) {
            loadNextManifest();
        } else {
            root.pluginsLoaded = true;
        }
    }
    
    function loadNextManifest() {
        if (root.pendingManifestLoads.length === 0) {
            root.pluginsLoaded = true;
            
            // Start background daemons after plugins are loaded
            root.startBackgroundDaemons();
            
            // Start pending plugin if one was requested before loading finished
            if (root.pendingPluginStart !== "") {
                const pluginId = root.pendingPluginStart;
                root.pendingPluginStart = "";
                root.startPlugin(pluginId);
            }
            // Load static index items from manifests (no handler needed)
            root.loadStaticIndexes();
            return;
        }
        
        const plugin = root.pendingManifestLoads.shift();
        manifestLoader.pluginId = plugin.id;
        manifestLoader.pluginPath = plugin.path;
        manifestLoader.isBuiltin = plugin.isBuiltin;
        manifestLoader.command = ["cat", plugin.path + "/manifest.json"];
        manifestLoader.running = true;
    }
    
    Process {
        id: manifestLoader
        property string pluginId: ""
        property string pluginPath: ""
        property string outputBuffer: ""
        
        stdout: SplitParser {
            splitMarker: ""
            onRead: data => {
                manifestLoader.outputBuffer += data;
            }
        }
        
        property bool isBuiltin: false
        
        onExited: (exitCode, exitStatus) => {
            if (exitCode !== 0) {
                console.warn(`[PluginRunner] Failed to load manifest for ${manifestLoader.pluginId}: exit code ${exitCode}`);
            }
            if (exitCode === 0 && manifestLoader.outputBuffer.trim()) {
                try {
                    const manifest = JSON.parse(manifestLoader.outputBuffer.trim());
                    
                    // Determine handler path (language-agnostic)
                    // Priority: manifest.handler > executable "handler" > "handler.py"
                    if (manifest.handler) {
                        manifest._handlerPath = manifestLoader.pluginPath + "/" + manifest.handler;
                    } else {
                        // Default to handler.py for backward compatibility
                        manifest._handlerPath = manifestLoader.pluginPath + "/handler.py";
                    }
                    
                    // Skip if plugin already exists (prevents duplicates from race conditions)
                    if (!root.plugins.some(p => p.id === manifestLoader.pluginId)) {
                        // Check compositor compatibility
                        // Default to ["hyprland"] if not specified (backward compatibility)
                        const supportedCompositors = manifest.supportedCompositors ?? ["hyprland"];
                        const currentCompositor = CompositorService.compositor;
                        
                        // Check if plugin supports current compositor
                        // "*" means all compositors are supported
                        const isSupported = supportedCompositors.includes("*") || 
                                           supportedCompositors.includes(currentCompositor);
                        
                        if (!isSupported) {
                            console.log(`[PluginRunner] Skipping plugin ${manifestLoader.pluginId}: not supported on ${currentCompositor} (supports: ${supportedCompositors.join(", ")})`);
                        } else {
                            const newPlugin = {
                                id: manifestLoader.pluginId,
                                path: manifestLoader.pluginPath,
                                manifest: manifest,
                                isBuiltin: manifestLoader.isBuiltin
                            };
                            
                            const updated = root.plugins.slice();
                            updated.push(newPlugin);
                            root.plugins = updated;
                            
                            // Build match pattern cache if plugin has patterns
                            root.buildMatchPatternCache(newPlugin);
                        }
                    }
                } catch (e) {
                    console.warn(`[PluginRunner] Failed to parse manifest for ${manifestLoader.pluginId}:`, e);
                }
            }
            
            manifestLoader.outputBuffer = "";
            root.loadNextManifest();
        }
    }
    
    // Watch for built-in plugin folders
    FolderListModel {
        id: builtinPluginsFolder
        folder: Qt.resolvedUrl(Directories.builtinPlugins)
        showDirs: true
        showFiles: false
        showHidden: false
        sortField: FolderListModel.Name
        onCountChanged: root.loadPlugins()
        onStatusChanged: {
            if (status === FolderListModel.Ready) {
                root.builtinFolderReady = true;
                root.loadPlugins();
            }
        }
    }
    
    // Watch for user plugin folders
    FolderListModel {
        id: userPluginsFolder
        folder: Qt.resolvedUrl(Directories.userPlugins)
        showDirs: true
        showFiles: false
        showHidden: false
        sortField: FolderListModel.Name
        onCountChanged: root.loadPlugins()
        onStatusChanged: {
            if (status === FolderListModel.Ready) {
                root.userFolderReady = true;
                root.loadPlugins();
            }
        }
    }
    


     // ==================== PLUGIN EXECUTION ====================
     
      // Start a plugin
      function startPlugin(pluginId) {
          // Queue if plugins not loaded yet (or still loading)
          if (!root.pluginsLoaded || !root.builtinFolderReady || !root.userFolderReady) {
              root.pendingPluginStart = pluginId;
              return true;  // Return true to indicate it will start
          }
          
          const plugin = root.plugins.find(w => w.id === pluginId);
          if (!plugin || !plugin.manifest) {
              return false;
          }
          
          const session = generateSessionId();
          
          root.activePlugin = {
              id: plugin.id,
              path: plugin.path,
              manifest: plugin.manifest,
              session: session
          };
          root.pluginResults = [];
          root.pluginCard = null;
          root.pluginForm = null;
          root.pluginPrompt = plugin.manifest.steps?.initial?.prompt ?? "";
          root.pluginPlaceholder = "";  // Reset placeholder on plugin start
          root.pluginError = "";
          root.inputMode = "realtime";  // Default to realtime, handler can change
          
          // Build initial input
          const input = { step: "initial", session: session };
          
          // For plugins browser, include available plugins list
          if (pluginId === "plugins") {
              input.context = {
                  plugins: root.plugins
                      .filter(p => p.id !== "plugins")  // Exclude self
                      .map(p => ({
                          id: p.id,
                          name: p.manifest?.name ?? p.id,
                          description: p.manifest?.description ?? "",
                          icon: p.manifest?.icon ?? "extension"
                      }))
                      .sort((a, b) => a.name.localeCompare(b.name))
              };
          }
          
          // For daemon plugins, start daemon if not already running
          const daemonConfig = plugin.manifest.daemon;
          if (daemonConfig?.enabled) {
              root.startDaemon(pluginId);
              // Send initial step through daemon
              root.writeToDaemonStdin(pluginId, input);
          } else {
              // Use request-response model for non-daemon plugins
              sendToPlugin(input);
          }
          
          return true;
      }
    
      // Send search query to active plugin
      function search(query) {
          if (!root.activePlugin) {
              return;
          }
          
          // Track query for action step (so handlers can filter results)
          root._lastSearchQuery = query;
          
          const pluginId = root.activePlugin.id;
          
          // Use builtin search if plugin has indexed items and query is not empty
          // This provides fuzzy matching + frecency + learned shortcuts automatically
          if (query.trim() !== "" && root.hasIndexedItems(pluginId)) {
              root.doBuiltinSearch(pluginId, query);
              return;
          }
          
          // Fall back to handler for empty query or plugins without index
          const input = {
              step: "search",
              query: query,
              session: root.activePlugin.session
          };
          
          // Include last selected item for context (useful for multi-step plugins)
          if (root.lastSelectedItem) {
              input.selected = { id: root.lastSelectedItem };
          }
          
          // Include plugin context if set (for multi-step flows like search mode, edit mode)
          if (root.pluginContext) {
              input.context = root.pluginContext;
          }
          
          // For plugins browser, include available plugins list
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
          
          // Use daemon if running, otherwise spawn new process
          const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
          if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
              root.pluginBusy = true;
              root.writeToDaemonStdin(root.activePlugin.id, input);
          } else {
              sendToPlugin(input);
          }
      }
    
        // Select an item and optionally execute an action
         function selectItem(itemId, actionId) {
             if (!root.activePlugin) return;
             
             // Store selection for context in subsequent search calls
             root.lastSelectedItem = itemId;
             
             // Record execution for frecency tracking
             // Skip special IDs and items in plugins with "plugin" frecency mode
             const skipRecordIds = ["__back__", "__empty__", "__form_cancel__", "__add__", "__plugin__"];
             const frecencyMode = root.activePlugin.manifest?.frecency ?? "item";
             if (frecencyMode === "item" && !skipRecordIds.includes(itemId) && !itemId.startsWith("__")) {
                 root.recordExecution(root.activePlugin.id, itemId);
             }
            
            // Track the step type for depth management
            // Navigation depth increases when:
            // - Default item click (no actionId) that returns a view - user is drilling down
            // - NOT for action button clicks (actionId set) - these modify current view
            // - NOT for special IDs that are known to not navigate
            const nonNavigatingIds = ["__back__", "__empty__", "__form_cancel__"];
            const isDefaultClick = !actionId;  // No action button clicked, just the item itself
            if (isDefaultClick && !nonNavigatingIds.includes(itemId)) {
                root.pendingNavigation = true;
            }
            
            const input = {
               step: "action",
               selected: { id: itemId },
               session: root.activePlugin.session
           };
           
           if (actionId) {
               input.action = actionId;
           }
           
           // Include context if set (handler needs it for navigation state)
           if (root.pluginContext) {
               input.context = root.pluginContext;
           }
           
           // Include current query so handler can return filtered results
           // (especially important for plugins using hybrid search)
           if (root._lastSearchQuery) {
               input.query = root._lastSearchQuery;
           }
           
           // Use daemon if running, otherwise spawn new process
           const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
           if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
               root.pluginBusy = true;
               root.writeToDaemonStdin(root.activePlugin.id, input);
           } else {
               sendToPlugin(input);
           }
       }
      
       // Track last recorded slider to avoid spam from rapid slider moves
        property string _lastRecordedSliderId: ""
        property var _sliderRecordResetTimer: Timer {
            interval: 2000
            onTriggered: root._lastRecordedSliderId = ""
        }
        
        // Send slider value change to plugin (for result item sliders)
        // pluginId is optional - if not provided, uses activePlugin
        function sliderValueChanged(itemId, value, pluginId) {
            // Determine target plugin
            let targetPluginId = pluginId ?? root.activePlugin?.id;
            if (!targetPluginId) return;
            
            const plugin = root.plugins.find(p => p.id === targetPluginId);
            if (!plugin) return;
            
            // Record execution for frecency (once per slider, reset after 2s idle)
            const sliderKey = `${targetPluginId}/${itemId}`;
            if (root._lastRecordedSliderId !== sliderKey) {
                root._lastRecordedSliderId = sliderKey;
                root.recordExecution(targetPluginId, itemId);
            }
            root._sliderRecordResetTimer.restart();
           
           const input = {
               step: "action",
               selected: { id: itemId },
               action: "slider",
               value: value,
               session: root.activePlugin?.session ?? "slider-update"
           };
           
           // Include context if set
           if (root.pluginContext) {
               input.context = root.pluginContext;
           }
           
           const isDaemonPlugin = plugin.manifest?.daemon?.enabled;
           if (isDaemonPlugin && root.runningDaemons[targetPluginId]) {
               root.writeToDaemonStdin(targetPluginId, input);
           } else if (isDaemonPlugin) {
               // Daemon not running, start it and send
               root.startDaemon(targetPluginId);
               // Small delay to let daemon initialize
               Qt.callLater(() => root.writeToDaemonStdin(targetPluginId, input));
           } else {
               sendToPlugin(input);
           }
       }
       
       // Send form slider value change to active plugin (for live form updates)
       function formSliderValueChanged(fieldId, value) {
           if (!root.activePlugin) return;
           
           const input = {
               step: "formSlider",
               fieldId: fieldId,
               value: value,
               session: root.activePlugin.session
           };
           
           // Include context if set
           if (root.pluginContext) {
               input.context = root.pluginContext;
           }
           
           const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
           if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
               root.writeToDaemonStdin(root.activePlugin.id, input);
           } else {
               sendToPlugin(input);
           }
       }
       
       // Send form switch value change to active plugin (for live form updates)
       function formSwitchValueChanged(fieldId, value) {
           if (!root.activePlugin) return;
           
           const input = {
               step: "formSwitch",
               fieldId: fieldId,
               value: value,
               session: root.activePlugin.session
           };
           
           // Include context if set
           if (root.pluginContext) {
               input.context = root.pluginContext;
           }
           
           const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
           if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
               root.writeToDaemonStdin(root.activePlugin.id, input);
           } else {
               sendToPlugin(input);
           }
       }
    
      // Submit form data to active plugin
      function submitForm(formData) {
          if (!root.activePlugin) return;
          
          const input = {
              step: "form",
              formId: root.pluginForm?.id ?? "",
              formData: formData,
              session: root.activePlugin.session
          };
          
          // Include context if set (handler may use it to identify form purpose)
          if (root.pluginContext) {
              input.context = root.pluginContext;
          }
          
          const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
          if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
              root.pluginBusy = true;
              root.writeToDaemonStdin(root.activePlugin.id, input);
          } else {
              sendToPlugin(input);
          }
      }
      
      // Cancel form and return to previous state
      function cancelForm() {
          if (!root.activePlugin) return;
          
          // Cancelling form is going back one level
          root.pendingBack = true;
          
          // Send cancel action to handler - it decides what to do
          const input = {
              step: "action",
              selected: { id: "__form_cancel__" },
              session: root.activePlugin.session
          };
          
          if (root.pluginContext) {
              input.context = root.pluginContext;
          }
          
          const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
          if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
              root.pluginBusy = true;
              root.writeToDaemonStdin(root.activePlugin.id, input);
          } else {
              sendToPlugin(input);
          }
      }
    
       // Close active plugin
       // If in replay mode, let the process finish (notification needs to be sent)
       // For non-background daemons, stop the daemon
       function closePlugin() {
           const pluginId = root.activePlugin?.id;
           
           // In replay mode, don't kill the process - let it complete for notification
           if (!root.replayMode) {
               pluginProcess.running = false;
           }
           
           // Stop daemon if it's not a background daemon
           if (pluginId) {
               const daemon = root.runningDaemons[pluginId];
               const isDaemonPlugin = root.activePlugin?.manifest?.daemon?.enabled;
               const isBackground = daemon?.isBackground ?? false;
               
               if (isDaemonPlugin && !isBackground) {
                   root.stopDaemon(pluginId);
               }
           }
           
            root.activePlugin = null;
            root.pluginResults = [];
            root.pluginCard = null;
            root.pluginForm = null;
            root.pluginPrompt = "";
            root.pluginPlaceholder = "";
            root.lastSelectedItem = null;
            root.pluginContext = "";
            root.pluginError = "";
            root.pluginBusy = false;
            root.inputMode = "realtime";
            root.pluginActions = [];
            root.navigationDepth = 0;
            root.pendingNavigation = false;
            root.pendingBack = false;
            root._pendingBuiltinResults = null;
            root._lastSearchQuery = "";
            root._lastHandlerPrependResults = [];
            root.pluginClosed();
        }
       
        // Patch individual items in pluginResults without replacing the array
        // This is used for incremental updates (e.g., slider adjustments)
        function patchPluginResults(patches) {
            if (!patches || !Array.isArray(patches)) return;
            
            // Create a map of patches by id for efficient lookup
            const patchMap = new Map();
            for (const patch of patches) {
                if (patch.id) {
                    patchMap.set(patch.id, patch);
                }
            }
            
            // Create a new array with patched items
            const updated = root.pluginResults.map(item => {
                const patch = patchMap.get(item.id);
                if (patch) {
                    // Return a new object with merged properties
                    return Object.assign({}, item, patch);
                }
                return item;
            });
            
            // Replace the array to trigger update
            root.pluginResults = updated;
            
            // Increment version counter for additional reactivity
            root.resultsVersion++;
        }
     
      // Go back one step in plugin navigation
      // If we're at the initial view (depth 0), close the plugin entirely
      // Go back one level within the plugin (does nothing at depth 0)
      function goBack() {
          if (!root.activePlugin) return;
          
          // At initial view, do nothing - use closePlugin() to exit
          if (root.navigationDepth <= 0) {
              return;
          }
          
          // Mark this as a back navigation (will decrement depth if results returned)
          root.pendingBack = true;
          
          // Send __back__ action to handler - let it decide how to handle navigation
          const input = {
              step: "action",
              selected: { id: "__back__" },
              session: root.activePlugin.session
          };
          
          // Include context if set (handler may need it to know where to go back to)
          if (root.pluginContext) {
              input.context = root.pluginContext;
          }
          
          const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
          if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
              root.pluginBusy = true;
              root.writeToDaemonStdin(root.activePlugin.id, input);
          } else {
              sendToPlugin(input);
          }
      }
     
     // Check if a plugin is active
     function isActive() {
         return root.activePlugin !== null;
     }
     
     // Get plugin by ID
     function getPlugin(id) {
         return root.plugins.find(w => w.id === id) ?? null;
     }
     
      // Execute a plugin-level action (from toolbar button)
      // These actions (filter, add mode, etc.) increase depth so user can "go back"
      // Set skipNavigation=true for confirmed actions (destructive actions that don't navigate)
      function executePluginAction(actionId, skipNavigation) {
          if (!root.activePlugin) return;
          
          // Plugin actions increase depth (user can press Escape to go back)
          // Unless skipNavigation is true (for confirmed destructive actions)
          if (!skipNavigation) {
              root.pendingNavigation = true;
          }
          
          const input = {
              step: "action",
              selected: { id: "__plugin__" },  // Special marker for plugin-level actions
              action: actionId,
              session: root.activePlugin.session
          };
          
          // Include context if set
          if (root.pluginContext) {
              input.context = root.pluginContext;
          }
          
          const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
          if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
              root.pluginBusy = true;
              root.writeToDaemonStdin(root.activePlugin.id, input);
          } else {
              sendToPlugin(input);
          }
      }
    
      // Execute an action via plugin handler
      // All item executions go through this - handler does the actual work
      // keepOpen: true = launcher stays open (interactive), false = closes after execution
      // Returns true if action was initiated, false if plugin not found
      function executeAction(pluginId, entryPoint, keepOpen = false) {
           const plugin = root.plugins.find(w => w.id === pluginId);
           if (!plugin || !plugin.manifest || !entryPoint) {
               return false;
           }
           
           const session = generateSessionId();
          
          root.activePlugin = {
              id: plugin.id,
              path: plugin.path,
              manifest: plugin.manifest,
              session: session
          };
          root.pluginResults = [];
          root.pluginCard = null;
          root.pluginForm = null;
          root.pluginPrompt = "";
          root.pluginPlaceholder = "";
          root.pluginError = "";
          root.inputMode = "realtime";
          root.replayMode = !keepOpen;
          root.replayPluginInfo = {
              id: plugin.id,
              name: plugin.manifest.name,
              icon: plugin.manifest.icon
          };
          
          if (keepOpen) {
              root.navigationDepth = 1;  // Entering at depth 1 (not initial view)
          }
          
          // Build input from entryPoint
          const input = {
              step: entryPoint.step ?? "action",
              session: session
          };
          
          if (!keepOpen) {
              input.replay = true;  // Signal to handler this is a one-shot execution
          }
          
          if (entryPoint.selected) {
              input.selected = entryPoint.selected;
              root.lastSelectedItem = entryPoint.selected.id ?? null;
          }
          if (entryPoint.action) {
              input.action = entryPoint.action;
          }
          if (entryPoint.query) {
              input.query = entryPoint.query;
          }
          
          // For daemon plugins with keepOpen, use daemon; otherwise use request-response
          const isDaemonPlugin = plugin.manifest?.daemon?.enabled;
          if (isDaemonPlugin && keepOpen) {
              root.startDaemon(pluginId);
              root.writeToDaemonStdin(pluginId, input);
          } else {
              sendToPlugin(input);
          }
          
          return true;
      }
      

    
    
    // ==================== INTERNAL ====================
    
    function generateSessionId() {
        return Date.now().toString(36) + Math.random().toString(36).substr(2, 9);
    }
    
     function sendToPlugin(input) {
         if (!root.activePlugin) return;
         
         root.pluginBusy = true;
         root.pluginError = "";
         
         const handlerPath = root.activePlugin.manifest._handlerPath 
             ?? (root.activePlugin.path + "/handler.py");
         
         const inputJson = JSON.stringify(input);
         
        // Use bash to pipe input to handler - language-agnostic (relies on shebang)
        pluginProcess.running = false;
        pluginProcess.workingDirectory = root.activePlugin.path;
        const escapedInput = inputJson.replace(/'/g, "'\\''");
        pluginProcess.command = ["bash", "-c", `echo '${escapedInput}' | "${handlerPath}"`];
        pluginProcess.running = true;
     }
    
     function handlePluginResponse(response, wasReplayMode = false) {
         root.pluginBusy = false;
         
         if (!response || !response.type) {
             root.pluginError = "Invalid response from plugin";
             root.pendingNavigation = false;
             root.pendingBack = false;
             return;
         }
         
         // Update session if provided
         if (response.session && root.activePlugin) {
             root.activePlugin.session = response.session;
         }
         
         // Navigation depth management
         // Plugin explicitly controls depth via response fields:
         // - navigationDepth: number → set absolute depth (for jumping multiple levels)
         // - navigateForward: true   → increment depth by 1 (drilling down)
         // - navigateBack: true      → decrement depth by 1 (going up)
         // - neither                 → no depth change (same view, modified data)
         // Also uses pendingNavigation/pendingBack flags set before request was sent
         const isViewResponse = ["results", "card", "form"].includes(response.type);
         if (isViewResponse) {
             const hasNavDepth = response.navigationDepth !== undefined && response.navigationDepth !== null;
             // Check if plugin explicitly set navigation flags (can override pending flags)
             const hasExplicitForward = response.navigateForward !== undefined;
             const hasExplicitBack = response.navigateBack !== undefined;
             
              if (hasNavDepth) {
                 // Explicit absolute depth (for jumping multiple levels)
                 root.navigationDepth = Math.max(0, parseInt(response.navigationDepth, 10));
             } else if (response.navigateBack === true || (!hasExplicitBack && root.pendingBack)) {
                 // Back navigation - decrement depth
                 // Plugin's explicit navigateBack overrides pendingBack
                 root.navigationDepth = Math.max(0, root.navigationDepth - 1);
              } else if (response.navigateForward === true || (!hasExplicitForward && root.pendingNavigation)) {
                  // Forward navigation - increment depth
                  // Plugin's explicit navigateForward overrides pendingNavigation
                  root.navigationDepth++;
              }
             // No flag = no depth change (action modified view, didn't navigate)
         }
         root.pendingNavigation = false;
         root.pendingBack = false;
         
          switch (response.type) {
               case "update":
                    // Patch individual items in pluginResults without replacing the array
                    if (response.items && Array.isArray(response.items)) {
                        root.patchPluginResults(response.items);
                    }
                    break;
                    
               case "results":
                     // Hybrid search: if we have pending builtin results, merge them
                     // Handler results are prepended, builtin results appended
                     let finalResults = response.results ?? [];
                     const pluginId = root.activePlugin?.id;
                     
                     if (root._pendingBuiltinResults !== null) {
                         // Search step response - merge handler + builtin results
                         const handlerResults = finalResults;
                         const builtinResults = root._pendingBuiltinResults;
                         
                         // Store handler results for re-filtering after actions
                         root._lastHandlerPrependResults = handlerResults;
                         
                         // Deduplicate: handler results take precedence
                         const handlerIds = new Set(handlerResults.map(r => r.id));
                         const dedupedBuiltin = builtinResults.filter(r => !handlerIds.has(r.id));
                         
                         finalResults = handlerResults.concat(dedupedBuiltin);
                         root._pendingBuiltinResults = null;
                     } else if (root._lastSearchQuery && pluginId && root.hasIndexedItems(pluginId)) {
                         // Action step response with active query - filter with builtin search
                         // This happens when user performs action (toggle, delete) while search is active
                         // Handler returns full list, but we need filtered results
                         
                         // Process other response fields first
                         if (response.placeholder !== undefined) {
                             root.pluginPlaceholder = response.placeholder ?? "";
                         }
                         if (response.context !== undefined) {
                             root.pluginContext = response.context ?? "";
                         }
                         root.inputMode = response.inputMode ?? "realtime";
                         if (response.pluginActions !== undefined) {
                             root.pluginActions = response.pluginActions ?? [];
                         }
                         if (response.status && pluginId) {
                             root.updatePluginStatus(pluginId, response.status);
                         }
                         
                         if (response.clearInput) {
                             root.clearInputRequested();
                             root._lastSearchQuery = "";
                             root._lastHandlerPrependResults = [];
                             // Show unfiltered results since query is cleared
                             root.pluginResults = finalResults;
                             root.resultsVersion++;
                         } else {
                             // Apply builtin search immediately to avoid flicker
                             // Use stored handler prepend results from last search
                             root.doBuiltinSearchOnly(pluginId, root._lastSearchQuery, root._lastHandlerPrependResults);
                         }
                         break;
                     }
                     
                     root.pluginResults = finalResults;
                     root.resultsVersion++;
                     root.pluginCard = null;
                     root.pluginForm = null;
                     if (response.placeholder !== undefined) {
                         root.pluginPlaceholder = response.placeholder ?? "";
                     }
                     if (response.context !== undefined) {
                         root.pluginContext = response.context ?? "";
                     }
                     root.inputMode = response.inputMode ?? "realtime";
                     if (response.pluginActions !== undefined) {
                         root.pluginActions = response.pluginActions ?? [];
                     }
                     if (response.clearInput) {
                        root.clearInputRequested();
                    }
                    // Update plugin status if provided
                    if (response.status && root.activePlugin?.id) {
                        root.updatePluginStatus(root.activePlugin.id, response.status);
                    }
                    root.resultsReady(root.pluginResults);
                    break;
                 
             case "card":
                 root.pluginCard = response.card ?? null;
                 root.pluginForm = null;
                 if (response.placeholder !== undefined) {
                     root.pluginPlaceholder = response.placeholder ?? "";
                 }
                 // Set input mode from response (defaults to realtime)
                 root.inputMode = response.inputMode ?? "realtime";
                 if (response.clearInput) {
                     root.clearInputRequested();
                 }
                 root.cardReady(root.pluginCard);
                 break;
                 
             case "form":
                 root.pluginForm = response.form ?? null;
                 root.pluginCard = null;
                 root.pluginResults = [];
                 // Allow handler to set context for form submission handling
                 if (response.context !== undefined) {
                     root.pluginContext = response.context ?? "";
                 }
                 root.formReady(root.pluginForm);
                 break;
                
             case "execute":
                 {
                      // Properties are directly on response (new format)
                      
                      // In replay mode, activePlugin may be cleared - use replayPluginInfo
                      const pluginName = root.activePlugin?.manifest?.name 
                          ?? root.replayPluginInfo?.name 
                          ?? "Plugin";
                      const pluginIcon = root.activePlugin?.manifest?.icon 
                          ?? root.replayPluginInfo?.icon 
                          ?? "play_arrow";
                      const pluginId = root.activePlugin?.id 
                          ?? root.replayPluginInfo?.id 
                          ?? "";
                      
                      // Process safe API actions
                      root.processExecuteAction(response, pluginId);
                      
                     // If handler provides name, emit for history tracking
                     if (response.name) {
                         root.actionExecuted({
                             name: response.name,
                             entryPoint: response.entryPoint ?? null,
                             icon: response.icon ?? pluginIcon,
                             iconType: response.iconType ?? "material",
                             thumbnail: response.thumbnail ?? "",
                             workflowId: pluginId,
                             workflowName: pluginName
                         });
                     }
                     
                     // Clear replay info after use
                     if (wasReplayMode) {
                         root.replayPluginInfo = null;
                     }
                 }
                 break;
                
             case "prompt":
                 if (response.prompt) {
                     root.pluginPrompt = response.prompt.text ?? "";
                     // preserve_input handled by caller
                 }
                 // Card might also be sent with prompt (for LLM responses)
                 if (response.card) {
                     root.pluginCard = response.card;
                     root.cardReady(root.pluginCard);
                 }
                 break;
                 
             case "imageBrowser":
                 if (response.imageBrowser) {
                     const isInitial = root.navigationDepth === 0;
                     const config = {
                         directory: response.imageBrowser.directory ?? "",
                         title: response.imageBrowser.title ?? root.activePlugin?.manifest?.name ?? "Select Image",
                         extensions: response.imageBrowser.extensions ?? null,
                         actions: response.imageBrowser.actions ?? [],
                         workflowId: root.activePlugin?.id ?? "",
                         enableOcr: response.imageBrowser.enableOcr ?? false,
                         isInitialView: isInitial
                     };
                     // Only increment depth if not the initial view
                     if (!isInitial) {
                         root.navigationDepth++;
                     }
                     GlobalStates.openImageBrowserForPlugin(config);
                 }
                 break;
                 
             case "gridBrowser":
                 if (response.gridBrowser) {
                     const isInitial = root.navigationDepth === 0;
                     const config = {
                         title: response.gridBrowser.title ?? root.activePlugin?.manifest?.name ?? "Select Item",
                         items: response.gridBrowser.items ?? [],
                         columns: response.gridBrowser.columns ?? 8,
                         cellAspectRatio: response.gridBrowser.cellAspectRatio ?? 1.0,
                         actions: response.gridBrowser.actions ?? [],
                         workflowId: root.activePlugin?.id ?? "",
                         isInitialView: isInitial
                     };
                     if (!isInitial) {
                         root.navigationDepth++;
                     }
                     GlobalStates.openGridBrowserForPlugin(config);
                 }
                 break;
                 
             case "error":
                 root.pluginError = response.message ?? "Unknown error";
                 console.warn(`[PluginRunner] Error: ${root.pluginError}`);
                 break;
             
             case "noop":
                 // No operation - used for actions that don't need UI refresh (e.g., slider adjustments)
                 break;
                 
             case "startPlugin":
                 // Start another plugin (used by plugins browser)
                 if (response.pluginId) {
                     root.stop();  // Stop current plugin first
                     root.startPlugin(response.pluginId);
                 }
                 break;
                 
             default:
                 console.warn(`[PluginRunner] Unknown response type: ${response.type}`);
         }
     }
    
     // Handle image browser selection - send back to plugin
     Connections {
         target: GlobalStates
         function onImageBrowserSelected(filePath, actionId) {
             if (!root.activePlugin) return;
             
             const input = {
                 step: "action",
                 selected: {
                     id: "imageBrowser",
                     path: filePath,
                     action: actionId
                 },
                 session: root.activePlugin.session
             };
             
             const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
             if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
                 root.pluginBusy = true;
                 root.writeToDaemonStdin(root.activePlugin.id, input);
             } else {
                 sendToPlugin(input);
             }
         }
         
         function onImageBrowserCancelled() {
             if (root.navigationDepth > 0) {
                 root.navigationDepth--;
                 if (root.activePlugin) root.goBack();
             } else {
                 // At initial view, close the plugin entirely
                 root.closePlugin();
             }
         }
         
         function onGridBrowserSelected(itemId, actionId) {
             if (!root.activePlugin) return;
             
             // Record execution for frecency tracking
             const frecencyMode = root.activePlugin.manifest?.frecency ?? "item";
             if (frecencyMode === "item" && itemId && !itemId.startsWith("__")) {
                 root.recordExecution(root.activePlugin.id, itemId);
             }
             
             const input = {
                 step: "action",
                 selected: {
                     id: "gridBrowser",
                     itemId: itemId,
                     action: actionId
                 },
                 session: root.activePlugin.session
             };
             
             const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
             if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
                 root.pluginBusy = true;
                 root.writeToDaemonStdin(root.activePlugin.id, input);
             } else {
                 sendToPlugin(input);
             }
         }
         
         function onGridBrowserCancelled() {
             if (root.navigationDepth > 0) {
                 root.navigationDepth--;
                 if (root.activePlugin) root.goBack();
             } else {
                 root.closePlugin();
             }
         }
     }
     
     // Process for running plugin handler
     Process {
         id: pluginProcess
         
         stdout: StdioCollector {
             id: pluginStdout
             onStreamFinished: {
                 root.pluginBusy = false;
                 const wasReplayMode = root.replayMode;
                 root.replayMode = false;  // Reset replay mode after process completes
                 
                 const output = pluginStdout.text.trim();
                 if (!output) {
                     root.pluginError = "No output from plugin";
                     return;
                 }
                 
                 // Handler may emit multiple JSON lines (e.g., index + execute response)
                 // Process each line, but only handle the last relevant response for one-shot
                 const lines = output.split('\n').filter(l => l.trim());
                 let lastResponse = null;
                 
                 for (const line of lines) {
                     try {
                         const response = JSON.parse(line);
                         // For one-shot execution, skip index responses - we only care about execute/results/etc
                         if (wasReplayMode && response.type === "index") {
                             continue;
                         }
                         lastResponse = response;
                     } catch (e) {
                         console.warn(`[PluginRunner] Parse error for line: ${e}`);
                     }
                 }
                 
                 if (lastResponse) {
                     root.handlePluginResponse(lastResponse, wasReplayMode);
                 } else if (lines.length > 0) {
                     // All lines were index responses (or parse errors)
                     root.pluginError = "No actionable response from plugin";
                 }
             }
         }
         
         stderr: SplitParser {
             onRead: data => console.warn(`[PluginRunner] stderr: ${data}`)
         }
         
         onExited: (exitCode, exitStatus) => {
             root.replayMode = false;
             root.replayPluginInfo = null;
             if (exitCode !== 0) {
                 root.pluginBusy = false;
                 root.pluginError = `Plugin exited with code ${exitCode}`;
             }
         }
     }
     
     // Watch for launcher close to execute pending typeText
     Connections {
         target: GlobalStates
         function onLauncherOpenChanged() {
             if (!GlobalStates.launcherOpen && root.pendingTypeText) {
                 // Small delay to ensure focus has transferred to target window
                 typeTextTimer.start();
             }
         }
     }
     
     Timer {
         id: typeTextTimer
         interval: 150
         repeat: false
         onTriggered: {
             if (root.pendingTypeText) {
                 console.log("[PluginRunner] Typing text via ydotool");
                 Quickshell.execDetached(["ydotool", "type", "--clearmodifiers", "--", root.pendingTypeText]);
                 root.pendingTypeText = "";
             }
         }
     }
     
     // Prepared plugins for fuzzy search
     // Exclude index-only plugins (staticIndex or indexOnly) - they provide indexed items but can't be opened
     property var preppedPlugins: plugins
         .filter(w => w.manifest && !w.manifest.staticIndex && !w.manifest.indexOnly)
         .map(w => ({
             name: Fuzzy.prepare(w.id),
             plugin: w
         }))
     
     // Fuzzy search plugins by name
     // Excludes index-only plugins (staticIndex or indexOnly) - they can't be opened
     function fuzzyQueryPlugins(query) {
         if (!query || query.trim() === "") {
             return root.plugins.filter(w => w.manifest && !w.manifest.staticIndex && !w.manifest.indexOnly);
         }
         return Fuzzy.go(query, root.preppedPlugins, { key: "name", limit: 10 })
             .map(r => r.obj.plugin);
     }
     
     // ==================== MATCH PATTERNS ====================
     // Plugins can define regex patterns in manifest.json that auto-trigger
     // the plugin when the user's query matches any pattern.
     //
     // Manifest format:
     //   "match": {
     //     "patterns": ["^=", "^\\d+\\s*[+\\-*/]", ...],
     //     "priority": 100  // Higher = checked first (optional, default 0)
     //   }
     //
     // The plugin with highest priority that matches is selected.
     // If multiple plugins match with same priority, first match wins.
     // ===========================================================
     
     // Compiled regex cache: { pluginId: [RegExp, ...] }
     property var matchPatternCache: ({})
     
     // Build regex cache for a plugin (called when plugins load)
     function buildMatchPatternCache(plugin) {
         if (!plugin?.manifest?.match?.patterns) return;
         
         const patterns = plugin.manifest.match.patterns;
         const compiled = [];
         
         for (const pattern of patterns) {
             try {
                 compiled.push(new RegExp(pattern, "i"));
             } catch (e) {
                 console.warn(`[PluginRunner] Invalid match pattern for ${plugin.id}: ${pattern}`);
             }
         }
         
         if (compiled.length > 0) {
             root.matchPatternCache[plugin.id] = compiled;
         }
     }
     
     // Check if query matches any plugin's patterns
     // Returns: { pluginId, priority } or null if no match
     // Uses pluginsByPriority (sorted highest-first) for early exit on first match
     function findMatchingPlugin(query) {
         if (!query || query.trim() === "") return null;
         
         // Iterate in priority order - first match wins
         for (const plugin of root.pluginsByPriority) {
             if (!plugin?.manifest?.match?.patterns) continue;
             
             const patterns = root.matchPatternCache[plugin.id];
             if (!patterns) continue;
             
             for (const regex of patterns) {
                 if (regex.test(query)) {
                     return { 
                         pluginId: plugin.id, 
                         priority: plugin.manifest.match.priority ?? 0 
                     };
                 }
             }
         }
         
         return null;
     }
     
     // Check if a plugin has match patterns defined
     function hasMatchPatterns(pluginId) {
         return root.matchPatternCache[pluginId] !== undefined;
     }
     
     // ==================== IPC HANDLERS ====================
     
     IpcHandler {
         target: "pluginRunner"
         
         // Update plugin status (badges, description)
         // Usage: hamr status <pluginId> '<json>'
         // Example: hamr status todo '{"badges": [{"text": "5"}]}'
         function updateStatus(pluginId: string, statusJson: string): void {
             try {
                 const status = JSON.parse(statusJson);
                 root.updatePluginStatus(pluginId, status);
             } catch (e) {
                 console.warn(`[PluginRunner] Failed to parse status JSON: ${e}`);
             }
         }
     }
 }
