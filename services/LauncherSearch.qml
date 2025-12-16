pragma Singleton

import qs
import qs.modules.common
import qs.modules.common.models
import qs.modules.common.functions
import qs.services
import QtQuick
import Qt.labs.folderlistmodel
import Quickshell
import Quickshell.Io

Singleton {
    id: root

    property string query: ""
    
    // Flag to skip auto-focus on next results update (set by action buttons)
    property bool skipNextAutoFocus: false
    
    // Loading guards: prevent flickering by waiting for async data to load
    property bool historyLoaded: false
    
    // ==================== EXCLUSIVE MODE ====================
    // Exclusive mode is for prefix-based filtering (/, :, =) that doesn't use plugins
    // but should still allow Escape to exit back to normal search
    property string exclusiveMode: ""  // "", "action"
    property bool exclusiveModeStarting: false  // Flag to prevent re-triggering on query clear
    
    function enterExclusiveMode(mode) {
        root.exclusiveModeStarting = true;
        root.exclusiveMode = mode;
        root.query = "";
        root.exclusiveModeStarting = false;
    }
    
    function exitExclusiveMode() {
        if (root.exclusiveMode !== "") {
            root.exclusiveMode = "";
            root.query = "";
        }
    }
    
    function isInExclusiveMode() {
        return root.exclusiveMode !== "";
    }
    
    // ==================== INDEX ISOLATION ====================
    // Search within a specific plugin's index using prefix (e.g., "emoji:", "apps:")
    // This allows focused search without starting the full plugin.
    // ==========================================================
    
    property string indexIsolationPlugin: ""  // Plugin ID to isolate search to, "" for normal search
    
    // Check if query matches index isolation pattern (pluginId:query)
    // Returns { pluginId, searchQuery } or null if not a match
    function parseIndexIsolationPrefix(query) {
        if (!query || query.length < 2) return null;
        
        const colonIndex = query.indexOf(":");
        if (colonIndex < 1) return null;  // Need at least 1 char before colon
        
        const prefix = query.substring(0, colonIndex).toLowerCase();
        const searchQuery = query.substring(colonIndex + 1);
        
        // Check if prefix matches an indexed plugin
        const indexedPlugins = PluginRunner.getIndexedPluginIds();
        if (indexedPlugins.includes(prefix)) {
            return { pluginId: prefix, searchQuery: searchQuery };
        }
        
        return null;
    }
    
    function enterIndexIsolation(pluginId) {
        root.indexIsolationPlugin = pluginId;
    }
    
    function exitIndexIsolation() {
        root.indexIsolationPlugin = "";
    }
    
    function isInIndexIsolation() {
        return root.indexIsolationPlugin !== "";
    }

    // ==================== WINDOW PICKER SUPPORT ====================
    // Window picker state is managed in GlobalStates

    // Launch new instance of the app currently in window picker
    function launchNewInstance(appId) {
        const entry = DesktopEntries.byId(appId);
        if (entry) {
            root.recordSearch("app", appId, root.query);
            entry.execute();
        }
    }

    function ensurePrefix(prefix) {
        if ([Config.options.search.prefix.action, Config.options.search.prefix.app, Config.options.search.prefix.emojis, Config.options.search.prefix.math, Config.options.search.prefix.shellCommand, Config.options.search.prefix.webSearch].some(i => root.query.startsWith(i))) {
            root.query = prefix + root.query.slice(1);
        } else {
            root.query = prefix + root.query;
        }
    }

    // https://specifications.freedesktop.org/menu/latest/category-registry.html
    property list<string> mainRegisteredCategories: ["AudioVideo", "Development", "Education", "Game", "Graphics", "Network", "Office", "Science", "Settings", "System", "Utility"]
    property list<string> appCategories: DesktopEntries.applications.values.reduce((acc, entry) => {
        for (const category of entry.categories) {
            if (!acc.includes(category) && mainRegisteredCategories.includes(category)) {
                acc.push(category);
            }
        }
        return acc;
    }, []).sort()

    // Load action scripts from plugins folders
    // Uses FolderListModel to auto-reload when scripts are added/removed
    // Note: Plugin folders (containing manifest.json) are handled by PluginRunner
    // Excludes text/config files like .md, .txt, .json, .yaml, etc.
    // Excludes test-* prefixed files (test utilities)
    readonly property var excludedActionExtensions: [".md", ".txt", ".json", ".yaml", ".yml", ".toml", ".ini", ".cfg", ".conf", ".log", ".csv", ".sh"]
    readonly property var excludedActionPrefixes: ["test-", "hamr-test"]
    
    // Helper to extract scripts from a FolderListModel
    function extractScriptsFromFolder(folderModel: FolderListModel): list<var> {
        const actions = [];
        for (let i = 0; i < folderModel.count; i++) {
            const fileName = folderModel.get(i, "fileName");
            const filePath = folderModel.get(i, "filePath");
            if (fileName && filePath) {
                // Skip text/config files
                const lowerName = fileName.toLowerCase();
                if (root.excludedActionExtensions.some(ext => lowerName.endsWith(ext))) {
                    continue;
                }
                // Skip test utilities (test-*, hamr-test)
                if (root.excludedActionPrefixes.some(prefix => lowerName.startsWith(prefix))) {
                    continue;
                }
                
                const actionName = fileName.replace(/\.[^/.]+$/, ""); // strip extension
                const scriptPath = FileUtils.trimFileProtocol(filePath);
                actions.push({
                    action: actionName,
                    execute: ((path) => (args) => {
                        // Run through bash to ensure proper shell script execution
                        Quickshell.execDetached(["bash", path, ...(args ? args.split(" ") : [])]);
                    })(scriptPath)
                });
            }
        }
        return actions;
    }
    
    // User scripts from ~/.config/hamr/plugins/
    property var userActionScripts: extractScriptsFromFolder(userActionsFolder)
    
    // Built-in scripts from repo plugins/ folder
    property var builtinActionScripts: extractScriptsFromFolder(builtinActionsFolder)

    FolderListModel {
        id: userActionsFolder
        folder: Qt.resolvedUrl(Directories.userPlugins)
        showDirs: false
        showHidden: false
        sortField: FolderListModel.Name
    }
    
    FolderListModel {
        id: builtinActionsFolder
        folder: Qt.resolvedUrl(Directories.builtinPlugins)
        showDirs: false
        showHidden: false
        sortField: FolderListModel.Name
    }
    
    // ==================== PLUGIN INTEGRATION ====================
    // Active plugin state - when a plugin is active, results come from PluginRunner
    property bool pluginActive: PluginRunner.activePlugin !== null
    property string activePluginId: PluginRunner.activePlugin?.id ?? ""
    
    // Start a plugin by ID
    function startPlugin(pluginId) {
        const success = PluginRunner.startPlugin(pluginId);
        if (success) {
            // Clear exclusive mode when starting a plugin
            root.exclusiveMode = "";
            // Clear query for fresh plugin input
            root.pluginStarting = true;
            root.query = "";
            root.pluginStarting = false;
            // Reset double-escape timer
            root.lastEscapeTime = 0;
        }
        return success;
    }
    
    // Start a plugin and immediately send a search query
    // Used for match pattern triggers where the query should be passed to the plugin
    function startPluginWithQuery(pluginId, initialQuery) {
        const success = PluginRunner.startPlugin(pluginId);
        if (success) {
            // Clear exclusive mode when starting a plugin
            root.exclusiveMode = "";
            // Keep the query (don't clear it) so user can continue typing
            // The plugin will receive the query via search step
            root.lastEscapeTime = 0;
            
            // Send search with initial query after plugin starts
            // Use a small delay to ensure plugin has processed initial step
            matchPatternSearchTimer.query = initialQuery;
            matchPatternSearchTimer.restart();
        }
        return success;
    }
    
    // Timer to send initial query to match pattern plugin
    Timer {
        id: matchPatternSearchTimer
        interval: 50
        property string query: ""
        onTriggered: {
            if (PluginRunner.isActive() && query) {
                PluginRunner.search(query);
            }
        }
    }
    
    // Close active plugin
    function closePlugin() {
        PluginRunner.closePlugin();
    }
    
    // Check if we should exit plugin mode (called when query becomes empty)
    function checkPluginExit() {
        if (PluginRunner.isActive() && root.query === "") {
            PluginRunner.closePlugin();
        }
    }
    
    // Listen for plugin executions to record in history
    Connections {
        target: PluginRunner
        function onActionExecuted(actionInfo) {
            root.recordWorkflowExecution(actionInfo);
        }
        function onClearInputRequested() {
            root.pluginClearing = true;
            root.query = "";
            root.pluginClearing = false;
        }
    }
    
    // Convert plugin results to LauncherSearchResult objects
    function pluginResultsToSearchResults(pluginResults: var): var {
        return pluginResults.map(item => {
            // Store item.id in local const to ensure closure captures value
            const itemId = item.id;
            
            // Convert plugin actions to LauncherSearchResult action objects
            const itemActions = (item.actions ?? []).map(action => {
                // Store action.id in local const to ensure closure captures value
                const actionId = action.id;
                // Determine icon type: respect explicit iconType, otherwise default to Material
                const actionIconType = action.iconType === "system" 
                    ? LauncherSearchResult.IconType.System 
                    : LauncherSearchResult.IconType.Material;
                return resultComp.createObject(null, {
                    name: action.name,
                    iconName: action.icon ?? 'play_arrow',
                    iconType: actionIconType,
                    execute: () => {
                        PluginRunner.selectItem(itemId, actionId);
                    }
                });
            });
            
            // Detect icon type based on iconType field from plugin, or auto-detect
            // If plugin explicitly sets iconType: "system" or "material", use that
            // Otherwise, default to Material icons (most plugin icons are material symbols)
            const iconName = item.icon ?? PluginRunner.activePlugin?.manifest?.icon ?? 'extension';
            let isSystemIcon;
            if (item.iconType === "system") {
                isSystemIcon = true;
            } else if (item.iconType === "material") {
                isSystemIcon = false;
            } else {
                // Auto-detect: System icons typically have dots or dashes (e.g., "org.gnome.Calculator", "google-chrome")
                // Default to Material for simple names (e.g., "apps", "folder", "play_circle")
                isSystemIcon = iconName.includes('.') || iconName.includes('-');
            }
            
            // Capture execute command if provided (for direct execution without action handler)
            const executeCommand = item.execute?.command ?? null;
            const executeNotify = item.execute?.notify ?? null;
            const executeName = item.execute?.name ?? null;
            const pluginId = PluginRunner.activePlugin?.id ?? "";
            const pluginName = PluginRunner.activePlugin?.manifest?.name ?? "Plugin";
            
            return resultComp.createObject(null, {
                id: itemId,  // Set id for stable key in ScriptModel
                name: item.name,
                comment: item.description ?? "",
                verb: item.verb ?? "Select",
                type: pluginName,
                iconName: iconName,
                iconType: isSystemIcon ? LauncherSearchResult.IconType.System : LauncherSearchResult.IconType.Material,
                resultType: LauncherSearchResult.ResultType.PluginResult,
                pluginId: pluginId,
                pluginItemId: itemId,
                pluginActions: item.actions ?? [],
                thumbnail: item.thumbnail ?? "",
                actions: itemActions,
                execute: ((capturedItemId, capturedExecuteCommand, capturedExecuteNotify, capturedExecuteName, capturedPluginId, capturedPluginName, capturedIconName) => () => {
                    // If item has execute.command, run it directly (e.g., calculator copy)
                    if (capturedExecuteCommand) {
                        Quickshell.execDetached(capturedExecuteCommand);
                        if (capturedExecuteNotify) {
                            Quickshell.execDetached(["notify-send", capturedPluginName, capturedExecuteNotify, "-a", "Shell"]);
                        }
                        // Record to history if execute.name is set
                        if (capturedExecuteName) {
                            root.recordWorkflowExecution({
                                name: capturedExecuteName,
                                command: capturedExecuteCommand,
                                entryPoint: null,
                                icon: capturedIconName,
                                iconType: "material",
                                thumbnail: "",
                                workflowId: capturedPluginId,
                                workflowName: capturedPluginName
                            }, root.query);
                        }
                        GlobalStates.launcherOpen = false;
                        return;
                    }
                    // Default action: select without specific action (calls handler)
                    PluginRunner.selectItem(capturedItemId, "");
                })(itemId, executeCommand, executeNotify, executeName, pluginId, pluginName, iconName)
            });
        });
    }
    
    // Prepared plugins for fuzzy search (from PluginRunner)
    property var preppedPlugins: PluginRunner.preppedPlugins


    
    // ==================== UNIFIED SEARCHABLES ====================
    // Single prepared array combining all searchable sources.
    // Each item has: { name, sourceType, id, data, isHistoryTerm }
    //
    // Source types:
    //   - plugin: Actions (scripts) and plugins (workflows)
    //   - pluginExecution: Past workflow actions (replayable)
    //   - webSearch: Past web searches
    //   - indexedItem: Plugin-indexed items (emoji, apps, quicklinks, etc.)
    // ==============================================================
    
    readonly property var sourceType: ({
        PLUGIN: "plugin",
        PLUGIN_EXECUTION: "pluginExecution",
        WEB_SEARCH: "webSearch",
        INDEXED_ITEM: "indexedItem"
    })
    
    // ==================== STATIC SEARCHABLES ====================
    // Rebuilt only on reload (plugins, actions, indexed items)
    // These change rarely - only when apps installed, plugins added, etc.
    // ==============================================================
    
    property var preppedStaticSearchables: []
    
    // Debounce timer for static searchables rebuild
    // Prevents excessive rebuilds during startup when multiple sources load
    Timer {
        id: staticRebuildTimer
        interval: 100
        onTriggered: root.doRebuildStaticSearchables()
    }
    
    function rebuildStaticSearchables() {
        staticRebuildTimer.restart();
    }
    
    function doRebuildStaticSearchables() {
        const items = [];
        
        // NOTE: Apps now come from plugin index (plugins/apps)
        
        // ========== ACTIONS ==========
        const actions = root.preppedActions ?? [];
        for (const preppedAction of actions) {
            const action = preppedAction.action;
            items.push({
                name: preppedAction.name,
                sourceType: root.sourceType.PLUGIN,
                id: `action:${action.action}`,
                data: { action, isAction: true },
                isHistoryTerm: false
            });
        }
        
        // ========== WORKFLOWS ==========
        const plugins = root.preppedPlugins ?? [];
        for (const preppedPlugin of plugins) {
            const plugin = preppedPlugin.plugin;
            items.push({
                name: preppedPlugin.name,
                sourceType: root.sourceType.PLUGIN,
                id: `workflow:${plugin.id}`,
                data: { plugin, isAction: false },
                isHistoryTerm: false
            });
        }
        
        // ========== PLUGIN INDEXED ITEMS ==========
        // Items from plugins that support indexing (emoji, apps, etc.)
        const indexedItems = PluginRunner.getAllIndexedItems();
        for (const item of indexedItems) {
            // Build searchable string from name + keywords
            const searchableText = item.keywords?.length > 0
                ? `${item.name} ${item.keywords.join(" ")}`
                : item.name;
            items.push({
                name: Fuzzy.prepare(searchableText),
                sourceType: root.sourceType.INDEXED_ITEM,
                id: item.id,
                data: { item },
                isHistoryTerm: false
            });
        }
        
        root.preppedStaticSearchables = items;
    }
    
    // Rebuild static searchables on reload
    Connections {
        target: Quickshell
        function onReloadCompleted() {
            root.rebuildStaticSearchables();
        }
    }
    
    // Rebuild static searchables when plugins change (new plugin added/removed)
    Connections {
        target: PluginRunner
        function onPluginsChanged() {
            root.rebuildStaticSearchables();
        }
        // Rebuild when plugin indexes are updated (emoji, apps, etc.)
        function onPluginIndexChanged(pluginId) {
            root.rebuildStaticSearchables();
        }
    }
    
    // Rebuild static searchables when actions change (new script added/removed)
    onAllActionsChanged: {
        root.rebuildStaticSearchables();
    }
    
    // NOTE: Initial rebuild is done in the main Component.onCompleted below
    
    // ==================== DYNAMIC SEARCHABLES ====================
    // History-based items that change frequently.
    // Rebuilt when history changes, but history changes are infrequent
    // (only after user actions, not during search).
    // ==============================================================
    
    property var preppedHistorySearchables: []
    
    function rebuildHistorySearchables() {
        const items = [];
        
        // ========== LEARNED SHORTCUTS (history terms for static items) ==========
        // These let users find apps/plugins by previously used search terms
        
        // App history terms (e.g., "ff" -> Firefox)
        // Look up apps from plugin index
        const indexedItems = PluginRunner.getAllIndexedItems();
        for (const historyItem of searchHistoryData.filter(h => h.type === "app" && h.recentSearchTerms?.length > 0)) {
            // Find app in indexed items by appId
            const appItem = indexedItems.find(item => item.appId === historyItem.name);
            if (!appItem) continue;
            for (const term of historyItem.recentSearchTerms) {
                items.push({
                    name: Fuzzy.prepare(term),
                    sourceType: root.sourceType.INDEXED_ITEM,
                    id: appItem.id,
                    data: { item: appItem, historyItem },
                    isHistoryTerm: true,
                    matchedTerm: term
                });
            }
        }
        
        // Action history terms
        for (const historyItem of searchHistoryData.filter(h => h.type === "action" && h.recentSearchTerms?.length > 0)) {
            const action = root.allActions.find(a => a.action === historyItem.name);
            if (!action) continue;
            for (const term of historyItem.recentSearchTerms) {
                items.push({
                    name: Fuzzy.prepare(term),
                    sourceType: root.sourceType.PLUGIN,
                    id: `action:${action.action}`,
                    data: { action, historyItem, isAction: true },
                    isHistoryTerm: true,
                    matchedTerm: term
                });
            }
        }
        
        // Workflow history terms
        for (const historyItem of searchHistoryData.filter(h => h.type === "workflow" && h.recentSearchTerms?.length > 0)) {
            const plugin = PluginRunner.getPlugin(historyItem.name);
            if (!plugin) continue;
            for (const term of historyItem.recentSearchTerms) {
                items.push({
                    name: Fuzzy.prepare(term),
                    sourceType: root.sourceType.PLUGIN,
                    id: `workflow:${plugin.id}`,
                    data: { plugin, historyItem, isAction: false },
                    isHistoryTerm: true,
                    matchedTerm: term
                });
            }
        }
        
        // ========== HISTORY-ONLY ITEMS ==========
        // These are items that only exist in history (plugin executions, web searches)
        
        // Plugin executions
        for (const historyItem of searchHistoryData.filter(h => h.type === "workflowExecution")) {
            items.push({
                name: Fuzzy.prepare(`${historyItem.workflowName} ${historyItem.name}`),
                sourceType: root.sourceType.PLUGIN_EXECUTION,
                id: historyItem.key,
                data: { historyItem },
                isHistoryTerm: false
            });
            // Plugin execution history terms
            if (historyItem.recentSearchTerms) {
                for (const term of historyItem.recentSearchTerms) {
                    items.push({
                        name: Fuzzy.prepare(term),
                        sourceType: root.sourceType.PLUGIN_EXECUTION,
                        id: historyItem.key,
                        data: { historyItem },
                        isHistoryTerm: true,
                        matchedTerm: term
                    });
                }
            }
        }
        
        // Web search history
        for (const historyItem of searchHistoryData.filter(h => h.type === "webSearch")) {
            items.push({
                name: Fuzzy.prepare(historyItem.name),
                sourceType: root.sourceType.WEB_SEARCH,
                id: `webSearch:${historyItem.name}`,
                data: { query: historyItem.name, historyItem },
                isHistoryTerm: false
            });
        }
        
        root.preppedHistorySearchables = items;
    }
    
    // Rebuild history searchables when history data changes
    onSearchHistoryDataChanged: {
        root.rebuildHistorySearchables();
    }
    
    // ==================== COMBINED SEARCHABLES ====================
    // Simple concat of static + history (no rebuild, just reference)
    // ==============================================================
    
    property var preppedSearchables: [...preppedStaticSearchables, ...preppedHistorySearchables]

    // Search history for frecency-based ranking
    property var searchHistoryData: []
    property int maxHistoryItems: Config.options.search.maxHistoryItems

    FileView {
        id: searchHistoryFileView
        path: Directories.searchHistory
        watchChanges: true
        onFileChanged: searchHistoryFileView.reload()
        onLoaded: {
            try {
                const data = JSON.parse(searchHistoryFileView.text());
                root.searchHistoryData = data.history || [];
            } catch (e) {
                console.log("[SearchHistory] Failed to parse:", e);
                root.searchHistoryData = [];
            }
            root.historyLoaded = true;
        }
        onLoadFailed: error => {
            if (error == FileViewError.FileNotFound) {
                // Create empty history file
                searchHistoryFileView.setText(JSON.stringify({ history: [] }));
            }
            root.searchHistoryData = [];
            root.historyLoaded = true;
        }
    }

    // Remove a history item by type and identifier
    // For workflowExecution: uses key (workflowId:name)
    // For windowFocus: uses key (windowFocus:appId:windowTitle)
    // For others: uses type + name
    function removeHistoryItem(historyType, identifier) {
        let newHistory;
        if (historyType === "workflowExecution" || historyType === "windowFocus") {
            newHistory = searchHistoryData.filter(h => !(h.type === historyType && h.key === identifier));
        } else {
            newHistory = searchHistoryData.filter(h => !(h.type === historyType && h.name === identifier));
        }
        
        if (newHistory.length !== searchHistoryData.length) {
            searchHistoryData = newHistory;
            searchHistoryFileView.setText(JSON.stringify({ history: newHistory }, null, 2));
        }
    }

    // Record a search execution
    // searchTerm is the actual search content (e.g., "hyprland" for web search, empty for apps)
    property int maxRecentSearchTerms: 5
    
    function recordSearch(searchType, searchName, searchTerm) {
        const now = Date.now();
        const existingIndex = searchHistoryData.findIndex(
            h => h.type === searchType && h.name === searchName
        );
        
        let newHistory = searchHistoryData.slice();
        
        if (existingIndex >= 0) {
            // Update existing entry
            const existing = newHistory[existingIndex];
            let recentTerms = existing.recentSearchTerms || [];
            
            // Add new search term to front, remove duplicates, limit size
            if (searchTerm) {
                recentTerms = recentTerms.filter(t => t !== searchTerm);
                recentTerms.unshift(searchTerm);
                recentTerms = recentTerms.slice(0, maxRecentSearchTerms);
            }
            
            newHistory[existingIndex] = {
                type: existing.type,
                name: existing.name,
                count: existing.count + 1,
                lastUsed: now,
                recentSearchTerms: recentTerms
            };
        } else {
            // Add new entry
            newHistory.unshift({
                type: searchType,
                name: searchName,
                count: 1,
                lastUsed: now,
                recentSearchTerms: searchTerm ? [searchTerm] : []
            });
        }
        
        // Apply aging and pruning (zoxide-inspired)
        newHistory = ageAndPruneHistory(newHistory, now);
        
        // Trim to max items
        if (newHistory.length > maxHistoryItems) {
            newHistory = newHistory.slice(0, maxHistoryItems);
        }
        
        searchHistoryData = newHistory;
        searchHistoryFileView.setText(JSON.stringify({ history: newHistory }, null, 2));
    }
    
    // Record a workflow execution (stores command and/or entryPoint for replay)
    // Hybrid approach:
    //   - command: Direct shell command for simple replay (fast, no workflow needed)
    //   - entryPoint: Workflow step for complex actions (invokes handler logic)
    // On replay: prefers command if available, falls back to entryPoint
    // searchTerm: optional search term used to find this execution (for learned shortcuts)
    function recordWorkflowExecution(actionInfo, searchTerm) {
        const now = Date.now();
        // Use name + workflowId as unique key
        const key = `${actionInfo.workflowId}:${actionInfo.name}`;
        const existingIndex = searchHistoryData.findIndex(
            h => h.type === "workflowExecution" && h.key === key
        );
        
        let newHistory = searchHistoryData.slice();
        
        if (existingIndex >= 0) {
            // Update existing entry
            const existing = newHistory[existingIndex];
            let recentTerms = existing.recentSearchTerms || [];
            
            // Add new search term to front, remove duplicates, limit size
            if (searchTerm) {
                recentTerms = recentTerms.filter(t => t !== searchTerm);
                recentTerms.unshift(searchTerm);
                recentTerms = recentTerms.slice(0, maxRecentSearchTerms);
            }
            
            newHistory[existingIndex] = {
                type: existing.type,
                key: existing.key,
                name: existing.name,
                workflowId: existing.workflowId,
                workflowName: existing.workflowName,
                command: actionInfo.command,
                entryPoint: actionInfo.entryPoint ?? null,
                icon: actionInfo.icon,
                iconType: actionInfo.iconType ?? existing.iconType ?? "material",
                thumbnail: actionInfo.thumbnail,
                count: existing.count + 1,
                lastUsed: now,
                recentSearchTerms: recentTerms
            };
        } else {
            // Add new entry
            newHistory.unshift({
                type: "workflowExecution",
                key: key,
                name: actionInfo.name,
                workflowId: actionInfo.workflowId,
                workflowName: actionInfo.workflowName,
                command: actionInfo.command,
                entryPoint: actionInfo.entryPoint ?? null,
                icon: actionInfo.icon,
                iconType: actionInfo.iconType ?? "material",
                thumbnail: actionInfo.thumbnail,
                count: 1,
                lastUsed: now,
                recentSearchTerms: searchTerm ? [searchTerm] : []
            });
        }
        
        newHistory = ageAndPruneHistory(newHistory, now);
        
        if (newHistory.length > maxHistoryItems) {
            newHistory = newHistory.slice(0, maxHistoryItems);
        }
        
        searchHistoryData = newHistory;
        searchHistoryFileView.setText(JSON.stringify({ history: newHistory }, null, 2));
    }
    
    // Record a window focus action (for switching to specific windows)
    // Stores: app ID, app name, window title for replay
    function recordWindowFocus(appId, appName, windowTitle, iconName) {
        const now = Date.now();
        // Use appId + windowTitle as unique key
        const key = `windowFocus:${appId}:${windowTitle}`;
        const existingIndex = searchHistoryData.findIndex(
            h => h.type === "windowFocus" && h.key === key
        );
        
        let newHistory = searchHistoryData.slice();
        
        if (existingIndex >= 0) {
            // Update existing entry
            const existing = newHistory[existingIndex];
            newHistory[existingIndex] = {
                type: existing.type,
                key: existing.key,
                appId: appId,
                appName: appName,
                windowTitle: windowTitle,
                iconName: iconName,
                count: existing.count + 1,
                lastUsed: now
            };
        } else {
            // Add new entry
            newHistory.unshift({
                type: "windowFocus",
                key: key,
                appId: appId,
                appName: appName,
                windowTitle: windowTitle,
                iconName: iconName,
                count: 1,
                lastUsed: now
            });
        }
        
        newHistory = ageAndPruneHistory(newHistory, now);
        
        if (newHistory.length > maxHistoryItems) {
            newHistory = newHistory.slice(0, maxHistoryItems);
        }
        
        searchHistoryData = newHistory;
        searchHistoryFileView.setText(JSON.stringify({ history: newHistory }, null, 2));
    }
    
    // ==================== AGING & PRUNING ====================
    // Inspired by zoxide's aging algorithm.
    //
    // Aging: When total score exceeds maxTotalScore, scale all counts down
    // so total becomes ~90% of maxTotalScore. This prevents score inflation.
    //
    // Pruning: Remove entries that are:
    //   - Older than maxAgeDays AND have count < 1 after aging
    // ===========================================================
    
    function ageAndPruneHistory(history, now) {
        // Calculate total score (just counts, not frecency)
        let totalCount = history.reduce((sum, item) => sum + item.count, 0);
        
        // Aging: if total exceeds max, scale down all counts
        if (totalCount > maxTotalScore) {
            const scaleFactor = (maxTotalScore * 0.9) / totalCount;
            history = history.map(item => ({
                type: item.type,
                name: item.name,
                count: item.count * scaleFactor,
                lastUsed: item.lastUsed,
                recentSearchTerms: item.recentSearchTerms
            }));
        }
        
        // Pruning: remove old entries with very low scores
        const maxAgeMs = maxAgeDays * 24 * 60 * 60 * 1000;
        history = history.filter(item => {
            const age = now - item.lastUsed;
            const isOld = age > maxAgeMs;
            const hasLowScore = item.count < 1;
            // Keep if: not old, or has reasonable score
            return !(isOld && hasLowScore);
        });
        
        return history;
    }

    // ==================== FRECENCY SCORING SYSTEM ====================
    // Inspired by zoxide's algorithm: https://github.com/ajeetdsouza/zoxide/wiki/Algorithm
    //
    // OVERVIEW:
    // All search result types (apps, files, URLs, actions, indexed items) use the same
    // unified scoring system for consistent, predictable ranking.
    //
    // FRECENCY FORMULA:
    //   frecency = count * recency_multiplier
    //
    // RECENCY MULTIPLIERS (4 simple brackets, zoxide-style):
    //   - Within 1 hour:  count * 4
    //   - Within 1 day:   count * 2  
    //   - Within 1 week:  count * 1
    //   - Older:          count * 0.5
    //
    // COMBINED SCORE FORMULA:
    //   finalScore = fuzzyScore + (frecency * scaleFactor * matchQuality)
    //   - scaleFactor (100): Amplifies frecency to compete with fuzzy scores
    //   - matchQuality (0.3-1.0): Poor matches get less frecency boost
    //
    // TERM MATCH BOOST:
    //   When user searches with a previously-used term, add:
    //   - Exact match: +5000
    //   - Prefix match: +3000
    //
    // AGING (prevents score inflation):
    //   When total count exceeds maxTotalScore (10000), scale all counts
    //   down so total becomes 90% of max.
    //
    // PRUNING:
    //   Remove entries older than maxAgeDays (90) with count < 1
    // ===================================================================
    
    property int maxTotalScore: 10000  // Triggers aging when exceeded
    property int maxAgeDays: 90        // Entries older than this with score < 1 are pruned
    
    // Calculate frecency score (combines frequency + recency)
    // Uses zoxide-style simple recency brackets
    function getFrecencyScore(historyItem) {
        if (!historyItem) return 0;
        const now = Date.now();
        const hoursSinceUse = (now - historyItem.lastUsed) / (1000 * 60 * 60);
        
        // Simple 4-bracket recency multiplier (zoxide-inspired)
        let recencyMultiplier;
        if (hoursSinceUse < 1) recencyMultiplier = 4;        // Within 1 hour
        else if (hoursSinceUse < 24) recencyMultiplier = 2;  // Within 1 day
        else if (hoursSinceUse < 168) recencyMultiplier = 1; // Within 1 week
        else recencyMultiplier = 0.5;                        // Older
        
        return historyItem.count * recencyMultiplier;
    }

    // Get frecency score for a search result by type and name
    function getHistoryBoost(searchType, searchName) {
        const historyItem = searchHistoryData.find(
            h => h.type === searchType && h.name === searchName
        );
        return getFrecencyScore(historyItem);
    }
    
    // ==================== INTENT DETECTION ====================
    // Determines the type of search based on query pattern:
    //   URL       - Matches URL pattern (domain.com)
    //   FILE      - Starts with file prefix (~)
    //   GENERAL   - Default (app search)
    //
    // Note: Math intent removed - now handled by calculate plugin via match patterns
    // ==================== MATCH TYPE ====================
    // Match type for tie-breaking (higher = better)
    readonly property var matchType: ({
        EXACT: 3,
        PREFIX: 2,
        FUZZY: 1,
        NONE: 0
    })
    
    function getMatchType(query, target) {
        if (!query || !target) return root.matchType.NONE;
        const q = query.toLowerCase();
        const t = target.toLowerCase();
        if (t === q) return root.matchType.EXACT;
        if (t.startsWith(q)) return root.matchType.PREFIX;
        return root.matchType.FUZZY;
    }
    
    // Compare two results for sorting
    // Returns negative if a should come before b
    // 
    // Ranking strategy:
    // - EXACT match (learned shortcut) + frecency = highest priority
    // - For non-EXACT matches, fuzzy score matters more than frecency
    //   (prevents high-frecency items from appearing for unrelated queries)
    function compareResults(a, b) {
        const aIsExact = a.matchType === root.matchType.EXACT;
        const bIsExact = b.matchType === root.matchType.EXACT;
        
        // 1. EXACT matches (learned shortcuts) always beat non-EXACT
        if (aIsExact !== bIsExact) {
            return aIsExact ? -1 : 1;
        }
        
        // 2. Among EXACT matches, frecency decides (learned shortcuts compete by usage)
        if (aIsExact && bIsExact) {
            if (Math.abs(a.frecency - b.frecency) > 1) {
                return b.frecency - a.frecency;
            }
            // If frecency is similar, use fuzzy score
            return b.fuzzyScore - a.fuzzyScore;
        }
        
        // 3. Among non-EXACT matches, fuzzy score first, then frecency as tiebreaker
        if (a.fuzzyScore !== b.fuzzyScore) {
            return b.fuzzyScore - a.fuzzyScore;
        }
        return b.frecency - a.frecency;
    }
    
    // ==================== COMBINED SCORING ====================
    // Based on research from Firefox frecency, Sublime Text fuzzy matching,
    // and fzf scoring algorithms.
    //
    // KEY INSIGHT: Use MULTIPLICATIVE combination, not additive.
    // This ensures frecency can boost good matches but never make
    // a bad match beat a good one.
    //
    // FORMULA:
    //   finalScore = fuzzyScore * (1 + frecencyBoost * boostFactor)
    //
    // - boostFactor: Controls max frecency influence (capped at maxFrecencyBoost)
    // - frecencyBoost: From getFrecencyScore() (typically 1-40)
    //
    // EXAMPLES:
    //   Good match (fuzzy=3000) + high frecency (20):
    //     3000 * (1 + min(20 * 0.05, 1.0)) = 3000 * 2.0 = 6000
    //   
    //   Poor match (fuzzy=500) + high frecency (20):
    //     500 * (1 + min(20 * 0.05, 1.0)) = 500 * 2.0 = 1000
    //     Still loses to good match without frecency (3000)
    //
    //   Good match (fuzzy=3000) + no frecency (0):
    //     3000 * (1 + 0) = 3000
    //     Still beats poor match with high frecency (1000)
    // ===========================================================
    
    property real frecencyBoostFactor: 50    // Points added per frecency unit
    property real maxFrecencyBoost: 500     // Cap on total frecency bonus
    
    function getCombinedScore(fuzzyScore, frecencyBoost) {
        // Fuzzy libraries may return negative scores (penalty-based, like Sublime)
        // or positive scores. Either way, higher = better match.
        // 
        // For negative scores: -30 is better than -1000
        // For positive scores: 1000 is better than 30
        //
        // We ADD frecency bonus (capped) to boost recently used items
        const boost = Math.min(frecencyBoost * frecencyBoostFactor, maxFrecencyBoost);
        return fuzzyScore + boost;
    }
    
    // ==================== TERM MATCH BOOST ====================
    // When user searches with a term they previously used to find an item,
    // give it a significant boost. This is separate from frecency.
    //
    // Values chosen to ensure term matches rank high but don't completely
    // override good fuzzy matches:
    //   - Exact match: 5000 (above most fuzzy scores)
    //   - Prefix match: 3000 (competitive with good fuzzy scores)
    // ===========================================================
    
    property int termMatchExactBoost: 5000
    property int termMatchPrefixBoost: 3000
    
    function getTermMatchBoost(recentTerms, query) {
        const queryLower = query.toLowerCase();
        let boost = 0;
        for (const term of recentTerms) {
            const termLower = term.toLowerCase();
            if (termLower === queryLower) {
                return termMatchExactBoost;
            } else if (termLower.startsWith(queryLower)) {
                boost = Math.max(boost, termMatchPrefixBoost);
            }
        }
        return boost;
    }
    
    // ==================== UNIFIED SEARCH ====================
    // Single Fuzzy.go() call with deduplication by (sourceType, id)
    // ===========================================================
    
    // Perform unified fuzzy search and return deduplicated results
    function unifiedFuzzySearch(query, limit) {
        if (!query || query.trim() === "") return [];
        
        const fuzzyResults = Fuzzy.go(query, root.preppedSearchables, {
            key: "name",
            limit: limit * 3  // Fetch extra to account for deduplication
        });
        
        // Dedupe by (sourceType, id), keeping highest score
        const seen = new Map();
        for (const match of fuzzyResults) {
            const item = match.obj;
            const key = `${item.sourceType}:${item.id}`;
            const existing = seen.get(key);
            
            if (!existing || match._score > existing.score) {
                seen.set(key, {
                    score: match._score,
                    item: item,
                    isHistoryTerm: item.isHistoryTerm
                });
            }
        }
        
        return Array.from(seen.values());
    }
    
    // Create LauncherSearchResult from unified searchable item
    function createResultFromSearchable(item, query, fuzzyScore) {
        const st = root.sourceType;
        const data = item.data;
        const resultMatchType = item.isHistoryTerm ? root.matchType.EXACT : root.matchType.FUZZY;
        
        // Look up frecency from history at search time (for static items that don't store historyItem)
        let frecency = 0;
        if (data.historyItem) {
            frecency = root.getFrecencyScore(data.historyItem);
        } else {
            // Look up history for static items
            switch (item.sourceType) {
                case st.PLUGIN:
                    if (data.isAction) {
                        frecency = root.getHistoryBoost("action", data.action.action);
                    } else {
                        frecency = root.getHistoryBoost("workflow", data.plugin.id);
                    }
                    break;
                case st.INDEXED_ITEM:
                    // For apps from plugin index, look up by appId
                    if (data.item?.appId) {
                        frecency = root.getHistoryBoost("app", data.item.appId);
                    }
                    break;
            }
        }
        
        switch (item.sourceType) {
            case st.PLUGIN:
                return createPluginResultFromData(data, item.id, query, fuzzyScore, frecency, resultMatchType);
            case st.PLUGIN_EXECUTION:
                return createPluginExecResultFromData(data, query, fuzzyScore, frecency, resultMatchType);
            case st.WEB_SEARCH:
                return createWebSearchHistoryResultFromData(data, query, fuzzyScore, frecency, resultMatchType);
            case st.INDEXED_ITEM:
                return createIndexedItemResultFromData(data, query, fuzzyScore, frecency, resultMatchType);
            default:
                return null;
        }
    }
    
    // Factory: Plugin result (actions + workflows)
    function createPluginResultFromData(data, itemId, query, fuzzyScore, frecency, resultMatchType) {
        if (data.isAction) {
            // Action (simple script)
            const action = data.action;
            const actionArgs = query.includes(" ") ? query.split(" ").slice(1).join(" ") : "";
            const hasArgs = actionArgs.length > 0;
            
            return {
                matchType: resultMatchType,
                fuzzyScore: fuzzyScore,
                frecency: frecency,
                result: resultComp.createObject(null, {
                    name: action.action + (hasArgs ? " " + actionArgs : ""),
                    verb: "Run",
                    type: "Action",
                    iconName: 'settings_suggest',
                    iconType: LauncherSearchResult.IconType.Material,
                    acceptsArguments: !hasArgs,
                    completionText: !hasArgs ? action.action + " " : "",
                    execute: ((capturedAction, capturedArgs, capturedQuery) => () => {
                        root.recordSearch("action", capturedAction.action, capturedQuery);
                        capturedAction.execute(capturedArgs);
                    })(action, actionArgs, query)
                })
            };
        } else {
            // Workflow (multi-step plugin)
            const plugin = data.plugin;
            const manifest = plugin.manifest;
            
            return {
                matchType: resultMatchType,
                fuzzyScore: fuzzyScore,
                frecency: frecency,
                result: resultComp.createObject(null, {
                    name: manifest?.name ?? plugin.id,
                    comment: manifest?.description ?? "",
                    verb: "Start",
                    type: "Plugin",
                    iconName: manifest?.icon ?? 'extension',
                    iconType: LauncherSearchResult.IconType.Material,
                    resultType: LauncherSearchResult.ResultType.PluginEntry,
                    pluginId: plugin.id,
                    acceptsArguments: true,
                    completionText: plugin.id + " ",
                    execute: ((capturedPlugin, capturedQuery) => () => {
                        root.recordSearch("workflow", capturedPlugin.id, capturedQuery);
                        root.startPlugin(capturedPlugin.id);
                    })(plugin, query)
                })
            };
        }
    }
    
    // Factory: Plugin execution history result
    function createPluginExecResultFromData(data, query, fuzzyScore, frecency, resultMatchType) {
        const item = data.historyItem;
        const iconType = item.iconType === "system" 
            ? LauncherSearchResult.IconType.System 
            : LauncherSearchResult.IconType.Material;
        
        return {
            matchType: resultMatchType,
            fuzzyScore: fuzzyScore,
            frecency: frecency,
            result: resultComp.createObject(null, {
                type: item.workflowName || "Recent",
                name: item.name,
                iconName: item.icon || 'play_arrow',
                iconType: iconType,
                thumbnail: item.thumbnail || "",
                verb: "Run",
                execute: ((capturedItem, capturedQuery) => () => {
                    root.recordWorkflowExecution({
                        name: capturedItem.name,
                        command: capturedItem.command,
                        entryPoint: capturedItem.entryPoint,
                        icon: capturedItem.icon,
                        iconType: capturedItem.iconType,
                        thumbnail: capturedItem.thumbnail,
                        workflowId: capturedItem.workflowId,
                        workflowName: capturedItem.workflowName
                    }, capturedQuery);
                    if (capturedItem.command && capturedItem.command.length > 0) {
                        Quickshell.execDetached(capturedItem.command);
                    } else if (capturedItem.entryPoint && capturedItem.workflowId) {
                        PluginRunner.replayAction(capturedItem.workflowId, capturedItem.entryPoint);
                    }
                })(item, query)
            })
        };
    }
    
    // Factory: Web search history result
    function createWebSearchHistoryResultFromData(data, query, fuzzyScore, frecency, resultMatchType) {
        const searchQuery = data.query;
        
        return {
            matchType: resultMatchType,
            fuzzyScore: fuzzyScore,
            frecency: frecency,
            result: resultComp.createObject(null, {
                name: searchQuery,
                verb: "Search",
                type: "Web search - recent",
                iconName: 'travel_explore',
                iconType: LauncherSearchResult.IconType.Material,
                execute: ((capturedQuery) => () => {
                    root.recordSearch("webSearch", capturedQuery, capturedQuery);
                    let url = Config.options.search.engineBaseUrl + capturedQuery;
                    for (let site of Config.options.search.excludedSites) {
                        url += ` -site:${site}`;
                    }
                    Qt.openUrlExternally(url);
                })(searchQuery)
            })
        };
    }
    
    // Factory: Plugin indexed item result (emoji, apps from plugins, etc.)
    function createIndexedItemResultFromData(data, query, fuzzyScore, frecency, resultMatchType) {
        const item = data.item;
        
        // Determine icon type from item
        let iconType;
        if (item.iconType === "text") {
            iconType = LauncherSearchResult.IconType.Text;
        } else if (item.iconType === "system") {
            iconType = LauncherSearchResult.IconType.System;
        } else {
            iconType = LauncherSearchResult.IconType.Material;
        }
        
        // Check if this is an app item (has appId field) for window integration
        const isAppItem = item.appId !== undefined;
        const appId = item.appId ?? "";
        
        // Get window info for apps at search time (for display)
        const windows = isAppItem ? WindowManager.getWindowsForApp(appId) : [];
        const windowCount = windows.length;
        
        // Convert item actions to LauncherSearchResult actions
        const itemActions = (item.actions ?? []).map(action => {
            const actionIconType = action.iconType === "system" 
                ? LauncherSearchResult.IconType.System 
                : LauncherSearchResult.IconType.Material;
            return resultComp.createObject(null, {
                name: action.name,
                iconName: action.icon ?? 'play_arrow',
                iconType: actionIconType,
                execute: ((capturedAction, capturedItem) => () => {
                    // Handle entryPoint (plugin handler execution)
                    if (capturedAction.entryPoint) {
                        if (capturedAction.keepOpen) {
                            // Keep UI open - use executeEntryPoint (e.g., edit snippet)
                            PluginRunner.executeEntryPoint(capturedItem._pluginId, capturedAction.entryPoint);
                        } else {
                            // Background execution - use replayAction
                            PluginRunner.replayAction(capturedItem._pluginId, capturedAction.entryPoint);
                            GlobalStates.launcherOpen = false;
                        }
                        return;
                    }
                    // Execute action command if provided
                    if (capturedAction.command) {
                        Quickshell.execDetached(capturedAction.command);
                        GlobalStates.launcherOpen = false;
                    }
                    // Record to history for apps
                    if (capturedItem.appId) {
                        root.recordSearch("app", capturedItem.appId, query);
                    }
                })(action, item)
            });
        });
        
        // Determine verb based on item type and state
        let verb = item.verb ?? (item.execute?.notify ? "Copy" : "Run");
        if (item.entryPoint) {
            verb = item.verb ?? "Copy";  // Default for entryPoint items (like bitwarden)
        }
        if (isAppItem) {
            verb = windowCount > 0 ? "Focus" : "Open";
        }
        
        return {
            matchType: resultMatchType,
            fuzzyScore: fuzzyScore,
            frecency: frecency,
            result: resultComp.createObject(null, {
                type: isAppItem ? "App" : (item._pluginName ?? "Plugin"),
                id: appId,  // For window tracking
                name: item.name,
                comment: item.description ?? "",
                iconName: item.icon ?? 'extension',
                iconType: iconType,
                thumbnail: item.thumbnail ?? "",
                verb: verb,
                keepOpen: item.keepOpen ?? false,
                windowCount: windowCount,
                windows: windows,
                actions: itemActions,
                execute: ((capturedItem, capturedQuery, capturedAppId, capturedIsApp) => () => {
                    if (capturedIsApp) {
                        // App execution with window integration
                        const currentWindows = WindowManager.getWindowsForApp(capturedAppId);
                        const currentWindowCount = currentWindows.length;
                        
                        if (currentWindowCount === 0) {
                            // No windows - launch app
                            root.recordSearch("app", capturedAppId, capturedQuery);
                            if (capturedItem.execute?.command) {
                                Quickshell.execDetached(capturedItem.execute.command);
                            }
                        } else if (currentWindowCount === 1) {
                            // One window - focus it
                            root.recordWindowFocus(capturedAppId, capturedItem.name, currentWindows[0].title, capturedItem.icon);
                            WindowManager.focusWindow(currentWindows[0]);
                            GlobalStates.launcherOpen = false;
                        } else {
                            // Multiple windows - show picker
                            GlobalStates.openWindowPicker(capturedAppId, currentWindows);
                        }
                    } else {
                        // Non-app item execution (emoji, bitwarden, notes, etc.)
                        console.log(`[LauncherSearch] Indexed item execute: entryPoint=${!!capturedItem.entryPoint}, keepOpen=${capturedItem.keepOpen}, pluginId=${capturedItem._pluginId}`);
                        
                        // Handle entryPoint (plugin handler execution)
                        if (capturedItem.entryPoint) {
                            if (capturedItem.keepOpen) {
                                // Keep UI open - use executeEntryPoint (e.g., view note)
                                console.log(`[LauncherSearch] Using executeEntryPoint for ${capturedItem._pluginId}`);
                                PluginRunner.executeEntryPoint(capturedItem._pluginId, capturedItem.entryPoint);
                            } else {
                                // Background execution - use replayAction (e.g., copy password)
                                console.log(`[LauncherSearch] Using replayAction for ${capturedItem._pluginId}`);
                                PluginRunner.replayAction(capturedItem._pluginId, capturedItem.entryPoint);
                                GlobalStates.launcherOpen = false;
                            }
                            return;
                        }
                        
                        // Execute command if provided
                        if (capturedItem.execute?.command) {
                            Quickshell.execDetached(capturedItem.execute.command);
                        }
                        // Show notification if provided
                        if (capturedItem.execute?.notify) {
                            Quickshell.execDetached(["notify-send", capturedItem._pluginName ?? "Plugin", capturedItem.execute.notify, "-a", "Shell"]);
                        }
                        // Record to history (only if execute.name is explicitly set)
                        if (capturedItem.execute?.name) {
                            root.recordWorkflowExecution({
                                name: capturedItem.execute.name,
                                command: capturedItem.execute?.command ?? [],
                                entryPoint: capturedItem.entryPoint ?? null,
                                icon: capturedItem.icon ?? 'play_arrow',
                                iconType: capturedItem.iconType ?? "material",
                                thumbnail: capturedItem.thumbnail ?? "",
                                workflowId: capturedItem._pluginId,
                                workflowName: capturedItem._pluginName
                            }, capturedQuery);
                        }
                    }
                })(item, query, appId, isAppItem)
            })
        };
    }

    property var searchActions: []

    // Combined built-in and user actions (user scripts override built-in with same name)
    property var allActions: {
        const combined = [...searchActions, ...builtinActionScripts];
        // Add user scripts, allowing them to override built-in scripts with same name
        for (const userScript of userActionScripts) {
            const existingIdx = combined.findIndex(a => a.action === userScript.action);
            if (existingIdx >= 0) {
                combined[existingIdx] = userScript; // User script overrides
            } else {
                combined.push(userScript);
            }
        }
        return combined;
    }

    // Prepared actions for fuzzy search
    property var preppedActions: allActions.map(a => ({
        name: Fuzzy.prepare(a.action),
        action: a
    }))

    // Rebuild static searchables on initial load
    Component.onCompleted: {
        // Delay slightly to ensure all dependencies are loaded
        Qt.callLater(root.rebuildStaticSearchables);
    }
    
    // Track if we're intentionally clearing query for plugin start
    property bool pluginStarting: false
    // Track if we're clearing input after receiving a response (don't trigger search)
    property bool pluginClearing: false
    // Track if we're starting a match pattern plugin (to pass initial query)
    property string matchPatternQuery: ""
    
    // Trigger plugin search when query changes
    onQueryChanged: {
        if (PluginRunner.isActive()) {
            // Don't exit plugin on empty query - let user use Escape to exit
            // Skip if we're programmatically clearing input after a response
            if (!root.pluginStarting && !root.pluginClearing) {
                // Only send search if inputMode is "realtime"
                // For "submit" mode, wait for Enter key
                if (PluginRunner.inputMode === "realtime") {
                    pluginSearchTimer.restart();
                }
            }
        } else if (root.isInExclusiveMode()) {
            // Already in exclusive mode, just let the query filter results
            // (no special handling needed)
        } else if (!root.exclusiveModeStarting) {
            // Check for prefix triggers (not in plugin or exclusive mode)
            if (root.query === Config.options.search.prefix.file) {
                // Start files plugin when ~ is typed
                root.startPlugin("files");
            } else if (root.query === Config.options.search.prefix.clipboard) {
                // Start clipboard plugin when ; is typed
                root.startPlugin("clipboard");
            } else if (root.query === Config.options.search.prefix.shellHistory) {
                // Start shell plugin when ! is typed
                root.startPlugin("shell");
            } else if (root.query === Config.options.search.prefix.action) {
                // Enter exclusive action mode when / is typed
                root.enterExclusiveMode("action");
            } else if (root.query === Config.options.search.prefix.emojis) {
                // Start emoji plugin when : is typed
                root.startPlugin("emoji");
            } else if (root.query === Config.options.search.prefix.math) {
                // Start calculate plugin when = is typed
                root.startPlugin("calculate");
            } else if (root.query.length >= 2) {
                // Check for match pattern triggers (need at least 2 chars to avoid false positives)
                matchPatternCheckTimer.restart();
            }
        }
    }
    
    // Debounce timer for match pattern checking (avoid checking on every keystroke)
    Timer {
        id: matchPatternCheckTimer
        interval: 50
        onTriggered: {
            if (PluginRunner.isActive() || root.isInExclusiveMode()) return;
            
            const match = PluginRunner.findMatchingPlugin(root.query);
            if (match) {
                // Store query to pass to plugin after it starts
                root.matchPatternQuery = root.query;
                root.startPluginWithQuery(match.pluginId, root.query);
            }
        }
    }
    
    // Submit plugin query (called on Enter key in submit mode)
    function submitPluginQuery() {
        if (PluginRunner.isActive() && PluginRunner.inputMode === "submit") {
            PluginRunner.search(root.query);
        }
    }
    
    // ==================== DOUBLE-ESCAPE DETECTION ====================
    // Double-tap Escape within 300ms to close plugin entirely (skip navigation)
    property real lastEscapeTime: 0
    readonly property int doubleEscapeThreshold: 300  // ms
    
    // Exit plugin mode - should be called on Escape key
    // Single Escape: go back one step (or close if at initial view)
    // Double Escape: close plugin entirely regardless of depth
    function exitPlugin() {
        if (!PluginRunner.isActive()) return;
        
        const now = Date.now();
        const timeSinceLastEscape = now - root.lastEscapeTime;
        root.lastEscapeTime = now;
        
        // Double-escape: close plugin entirely
        if (timeSinceLastEscape < root.doubleEscapeThreshold && PluginRunner.navigationDepth > 0) {
            PluginRunner.closePlugin();
            root.query = "";
            return;
        }
        
        // Single escape: go back one step (or close if at depth 0)
        const wasAtInitial = PluginRunner.navigationDepth <= 0;
        PluginRunner.goBack();
        // Only clear query if we're actually closing the plugin
        if (wasAtInitial) {
            root.query = "";
        }
    }
    
    // Debounce timer for plugin search
    Timer {
        id: pluginSearchTimer
        interval: Config.options.search?.pluginDebounceMs ?? 150
        onTriggered: {
            // Double-check inputMode in case it changed while timer was running
            if (PluginRunner.isActive() && PluginRunner.inputMode === "realtime") {
                PluginRunner.search(root.query);
            }
        }
    }
    
    property list<var> results: {
        // Search results are handled here
        
        ////////////////// Plugin mode - show plugin results //////////////////
        // Use property access (not function call) to ensure proper QML binding
        const _pluginActive = PluginRunner.activePlugin !== null;
        const _pluginResults = PluginRunner.pluginResults;
        if (_pluginActive) {
            // Convert plugin results to LauncherSearchResult objects
            return root.pluginResultsToSearchResults(_pluginResults);
        }
        
        ////////////////// Exclusive mode - check BEFORE empty query //////////////////
        // This ensures exclusive modes (/, :, =) are handled even when query is cleared
        
        // Actions/Plugins in exclusive mode - show only actions and plugins
        if (root.exclusiveMode === "action") {
            const searchString = root.query.split(" ")[0];
            const actionArgs = root.query.split(" ").slice(1).join(" ");
            
            // Get actions
            const actionMatches = searchString === "" 
                ? root.allActions.slice(0, 20)
                : Fuzzy.go(searchString, root.preppedActions, { key: "name", limit: 20 }).map(r => r.obj.action);
            
            const actionItems = actionMatches.map(action => {
                const hasArgs = actionArgs.length > 0;
                return resultComp.createObject(null, {
                    name: action.action + (hasArgs ? " " + actionArgs : ""),
                    verb: "Run",
                    type: "Action",
                    iconName: 'settings_suggest',
                    iconType: LauncherSearchResult.IconType.Material,
                    execute: () => {
                        root.recordSearch("action", action.action, root.query);
                        action.execute(actionArgs);
                    }
                });
            });
            
            // Get plugins
            const pluginMatches = searchString === ""
                ? PluginRunner.plugins.slice(0, 20)
                : Fuzzy.go(searchString, root.preppedPlugins, { key: "name", limit: 20 }).map(r => r.obj.plugin);
            
            const pluginItems = pluginMatches.map(plugin => {
                return resultComp.createObject(null, {
                    name: plugin.manifest?.name || plugin.id,
                    comment: plugin.manifest?.description || "",
                    verb: "Open",
                    type: "Plugin",
                    iconName: plugin.manifest?.icon || 'extension',
                    iconType: LauncherSearchResult.IconType.Material,
                    resultType: LauncherSearchResult.ResultType.PluginEntry,
                    pluginId: plugin.id,
                    execute: () => {
                        root.recordSearch("workflow", plugin.id, root.query);
                        root.startPlugin(plugin.id);
                    }
                });
            });
            
            return [...pluginItems, ...actionItems].filter(Boolean);
        }
        
        ////////////////// Index Isolation Mode - search within specific plugin's index //////////////////
        // Check if query matches "pluginId:searchterm" pattern
        const isolationMatch = root.parseIndexIsolationPrefix(root.query);
        if (isolationMatch) {
            const { pluginId, searchQuery } = isolationMatch;
            const pluginItems = PluginRunner.getIndexedItemsForPlugin(pluginId);
            
            if (pluginItems.length === 0) {
                return [resultComp.createObject(null, {
                    name: `No indexed items for "${pluginId}"`,
                    type: "Info",
                    iconName: 'info',
                    iconType: LauncherSearchResult.IconType.Material
                })];
            }
            
            // Build searchables for this plugin only
            const preppedItems = pluginItems.map(item => ({
                name: Fuzzy.prepare(item.keywords?.length > 0 ? `${item.name} ${item.keywords.join(" ")}` : item.name),
                item: item
            }));
            
            // Fuzzy search within plugin items
            let matches;
            if (searchQuery.trim() === "") {
                // Empty search query - show all items from plugin
                matches = preppedItems.slice(0, 50).map(p => ({ obj: p }));
            } else {
                matches = Fuzzy.go(searchQuery, preppedItems, { key: "name", limit: 50 });
            }
            
            // Convert to results
            const plugin = PluginRunner.plugins.find(p => p.id === pluginId);
            const pluginName = plugin?.manifest?.name ?? pluginId;
            
            return matches.map(match => {
                const item = match.obj.item;
                return root.createIndexedItemResultFromData(item, searchQuery);
            }).filter(Boolean);
        }
        
        ////////////////// Empty query - show recent history //////////////////
        if (root.query == "") {
            // Loading guard: wait for all async data sources to load before showing results
            // This prevents flickering where list populates incrementally
            if (!root.historyLoaded || !PluginRunner.pluginsLoaded) return [];
            
            // Force dependency on allActions for re-evaluation
            const _actionsLoaded = root.allActions.length;
            const _historyLoaded = searchHistoryData.length;
            
            if (_historyLoaded === 0) return [];
            
            // Sort history by frecency and show recent items
            // Map items first, filter nulls, then take top 20
            // For initial list, prioritize recency over frequency
            const recentItems = searchHistoryData
                .slice()
                .sort((a, b) => (b.lastUsed || 0) - (a.lastUsed || 0))
                .map(item => {
                    // Helper to create remove action for history items
                    const makeRemoveAction = (historyType, identifier) => ({
                        name: "Remove",
                        iconName: "delete",
                        iconType: LauncherSearchResult.IconType.Material,
                        execute: () => root.removeHistoryItem(historyType, identifier)
                    });
                    
                    if (item.type === "app") {
                        // Look up app from plugin index
                        const allIndexed = PluginRunner.getAllIndexedItems();
                        const appItem = allIndexed.find(idx => idx.appId === item.name);
                        if (!appItem) return null;
                        const appId = appItem.appId;
                        return resultComp.createObject(null, {
                            type: "Recent",
                            id: appId,
                            name: appItem.name,
                            iconName: appItem.icon,
                            iconType: LauncherSearchResult.IconType.System,
                            verb: "Open",
                            actions: [makeRemoveAction("app", item.name)],
                            execute: ((capturedAppItem, capturedAppId) => () => {
                                // Smart execute: check windows at execution time
                                const currentWindows = WindowManager.getWindowsForApp(capturedAppId);
                                if (currentWindows.length === 0) {
                                    root.recordSearch("app", capturedAppId, "");
                                    if (capturedAppItem.execute?.command) {
                                        Quickshell.execDetached(capturedAppItem.execute.command);
                                    }
                                } else if (currentWindows.length === 1) {
                                    root.recordWindowFocus(capturedAppId, capturedAppItem.name, currentWindows[0].title, capturedAppItem.icon);
                                    WindowManager.focusWindow(currentWindows[0]);
                                    GlobalStates.launcherOpen = false;
                                } else {
                                    // Don't record - WindowPicker will record when user selects
                                    GlobalStates.openWindowPicker(capturedAppId, currentWindows);
                                }
                            })(appItem, appId)
                        });
                    } else if (item.type === "action") {
                        const action = root.allActions.find(a => a.action === item.name);
                        if (!action) return null;
                        return resultComp.createObject(null, {
                            type: "Recent",
                            name: action.action,
                            iconName: 'settings_suggest',
                            iconType: LauncherSearchResult.IconType.Material,
                            verb: "Run",
                            actions: [makeRemoveAction("action", item.name)],
                            execute: () => {
                                root.recordSearch("action", action.action, "");
                                action.execute("");
                            }
                        });
                    } else if (item.type === "workflow") {
                        const plugin = PluginRunner.getPlugin(item.name);
                        if (!plugin) return null;
                        return resultComp.createObject(null, {
                            type: "Recent",
                            name: plugin.manifest?.name || item.name,
                            iconName: plugin.manifest?.icon || 'extension',
                            iconType: LauncherSearchResult.IconType.Material,
                            resultType: LauncherSearchResult.ResultType.PluginEntry,
                            verb: "Open",
                            actions: [makeRemoveAction("workflow", item.name)],
                            execute: () => {
                                root.recordSearch("workflow", item.name, "");
                                root.startPlugin(item.name);
                            }
                        });
                    } else if (item.type === "workflowExecution") {
                        // Determine icon type from stored value
                        const iconType = item.iconType === "system" 
                            ? LauncherSearchResult.IconType.System 
                            : LauncherSearchResult.IconType.Material;
                        return resultComp.createObject(null, {
                            type: item.workflowName || "Recent",
                            name: item.name,
                            iconName: item.icon || 'play_arrow',
                            iconType: iconType,
                            thumbnail: item.thumbnail || "",
                            verb: "Run",
                            actions: [makeRemoveAction("workflowExecution", item.key)],
                            execute: () => {
                                // Re-record to update frecency (no search term for Recent items)
                                root.recordWorkflowExecution({
                                    name: item.name,
                                    command: item.command,
                                    entryPoint: item.entryPoint,
                                    icon: item.icon,
                                    iconType: item.iconType,
                                    thumbnail: item.thumbnail,
                                    workflowId: item.workflowId,
                                    workflowName: item.workflowName
                                }, "");
                                // Hybrid replay: prefer command, fallback to entryPoint
                                if (item.command && item.command.length > 0) {
                                    // Direct command execution (fast path)
                                    Quickshell.execDetached(item.command);
                                } else if (item.entryPoint && item.workflowId) {
                                    // Plugin replay via entryPoint (complex actions)
                                    PluginRunner.replayAction(item.workflowId, item.entryPoint);
                                }
                            }
                        });
                    } else if (item.type === "windowFocus") {
                        return resultComp.createObject(null, {
                            type: "Recent",
                            id: item.appId,
                            name: item.appName,
                            comment: item.windowTitle,
                            iconName: item.iconName,
                            iconType: LauncherSearchResult.IconType.System,
                            verb: "Focus",
                            actions: [makeRemoveAction("windowFocus", item.key)],
                            execute: () => {
                                // Find window by title
                                const windows = WindowManager.getWindowsForApp(item.appId);
                                const targetWindow = windows.find(w => w.title === item.windowTitle);
                                
                                if (targetWindow) {
                                    // Found the exact window - focus it
                                    root.recordWindowFocus(item.appId, item.appName, item.windowTitle, item.iconName);
                                    WindowManager.focusWindow(targetWindow);
                                    GlobalStates.launcherOpen = false;
                                } else if (windows.length === 1) {
                                    // Only one window - focus it (title may have changed)
                                    root.recordWindowFocus(item.appId, item.appName, windows[0].title, item.iconName);
                                    WindowManager.focusWindow(windows[0]);
                                    GlobalStates.launcherOpen = false;
                                } else if (windows.length > 1) {
                                    // Multiple windows but can't find exact match - show picker
                                    GlobalStates.openWindowPicker(item.appId, windows);
                                } else {
                                    // No windows - launch new instance
                                    const entry = DesktopEntries.byId(item.appId);
                                    if (entry) entry.execute();
                                }
                            }
                        });
                    }
                    return null;
                })
                .filter(Boolean)
                .slice(0, Config.options.search?.maxRecentItems ?? 20);  // Take top N valid items
            
            return recentItems;
        }

        ////////////////// Unified Search System ///////////////////
        // Single Fuzzy.go() call with deduplication for all sources.
        
        // Perform unified fuzzy search
        const unifiedResults = root.unifiedFuzzySearch(root.query, 50);
        
        // Convert unified results to scored result objects
        const allResults = [];
        for (const match of unifiedResults) {
            const resultObj = root.createResultFromSearchable(match.item, root.query, match.score);
            if (resultObj) {
                allResults.push(resultObj);
            }
        }
        
        // Sort by match quality + frecency
        allResults.sort(root.compareResults);
        
        // ========== WEB SEARCH (always last) ==========
        const webSearchQuery = StringUtils.cleanPrefix(root.query, Config.options.search.prefix.webSearch);
        allResults.push({
            matchType: root.matchType.NONE,
            fuzzyScore: 0,
            frecency: 0,
            result: resultComp.createObject(null, {
                name: webSearchQuery,
                verb: "Search",
                type: "Web search",
                iconName: 'travel_explore',
                iconType: LauncherSearchResult.IconType.Material,
                execute: ((capturedQuery) => () => {
                    root.recordSearch("webSearch", capturedQuery, capturedQuery);
                    let url = Config.options.search.engineBaseUrl + capturedQuery;
                    for (let site of Config.options.search.excludedSites) {
                        url += ` -site:${site}`;
                    }
                    Qt.openUrlExternally(url);
                })(webSearchQuery)
            })
        });
        
        // Take top results (configurable via search.maxDisplayedResults)
        const maxResults = Config.options.search?.maxDisplayedResults ?? 16;
        return allResults.slice(0, maxResults).map(item => item.result);
    }

    Component {
        id: resultComp
        LauncherSearchResult {}
    }
}
