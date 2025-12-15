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
    property bool quicklinksLoaded: false
    
    // ==================== EXCLUSIVE MODE ====================
    // Exclusive mode is for prefix-based filtering (/, :, =) that doesn't use plugins
    // but should still allow Escape to exit back to normal search
    property string exclusiveMode: ""  // "", "action", "emoji", "math"
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

    // File search prefix - fallback if not in config
    property string filePrefix: Config.options.search.prefix.file ?? "~"
    
    function ensurePrefix(prefix) {
        if ([Config.options.search.prefix.action, Config.options.search.prefix.app, Config.options.search.prefix.emojis, Config.options.search.prefix.math, Config.options.search.prefix.shellCommand, Config.options.search.prefix.webSearch, root.filePrefix].some(i => root.query.startsWith(i))) {
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
        }
        return success;
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
                return resultComp.createObject(null, {
                    name: action.name,
                    iconName: action.icon ?? 'play_arrow',
                    iconType: LauncherSearchResult.IconType.Material,
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
            
            return resultComp.createObject(null, {
                id: itemId,  // Set id for stable key in ScriptModel
                name: item.name,
                comment: item.description ?? "",
                verb: item.verb ?? "Select",
                type: PluginRunner.activePlugin?.manifest?.name ?? "Plugin",
                iconName: iconName,
                iconType: isSystemIcon ? LauncherSearchResult.IconType.System : LauncherSearchResult.IconType.Material,
                resultType: LauncherSearchResult.ResultType.PluginResult,
                pluginId: PluginRunner.activePlugin?.id ?? "",
                pluginItemId: itemId,
                pluginActions: item.actions ?? [],
                thumbnail: item.thumbnail ?? "",
                actions: itemActions,
                execute: () => {
                    // Default action: select without specific action
                    PluginRunner.selectItem(itemId, "");
                }
            });
        });
    }
    
    // Prepared plugins for fuzzy search (from PluginRunner)
    property var preppedPlugins: PluginRunner.preppedPlugins

    // Load quicklinks from config file
    property var quicklinks: []
    
    FileView {
        id: quicklinksFileView
        path: Directories.quicklinksConfig
        watchChanges: true
        onFileChanged: quicklinksFileView.reload()
        onLoaded: {
            try {
                const data = JSON.parse(quicklinksFileView.text());
                root.quicklinks = data.quicklinks || [];
            } catch (e) {
                console.log("[Quicklinks] Failed to parse quicklinks.json:", e);
                root.quicklinks = [];
            }
            root.quicklinksLoaded = true;
        }
        onLoadFailed: error => {
            console.log("[Quicklinks] Failed to load quicklinks.json:", error);
            root.quicklinks = [];
            root.quicklinksLoaded = true;
        }
    }

    // Prepared quicklinks for fuzzy search (includes aliases)
    property var preppedQuicklinks: {
        const items = [];
        for (const link of root.quicklinks) {
            // Add main name
            items.push({
                name: Fuzzy.prepare(link.name),
                quicklink: link
            });
            // Add aliases
            if (link.aliases) {
                for (const alias of link.aliases) {
                    items.push({
                        name: Fuzzy.prepare(alias),
                        quicklink: link,
                        isAlias: true,
                        aliasName: alias
                    });
                }
            }
        }
        return items;
    }

    // Search history for frecency-based ranking
    property var searchHistoryData: []
    property int maxHistoryItems: 200
    
    // Prepared URL history items for fuzzy search
    // Strip protocol for better fuzzy matching (https://github.com -> github.com)
    function stripProtocol(url) {
        return url.replace(/^https?:\/\//, '');
    }
    
    property var preppedUrlHistory: {
        return searchHistoryData
            .filter(item => item.type === "url")
            .map(item => ({
                name: Fuzzy.prepare(root.stripProtocol(item.name)),
                url: item.name,
                historyItem: item
            }));
    }
    
    // Prepared app history items for fuzzy search against recent search terms
    // This allows finding apps by the search terms previously used to find them
    property var preppedAppHistoryTerms: {
        const items = [];
        for (const historyItem of searchHistoryData) {
            if (historyItem.type === "app" && historyItem.recentSearchTerms) {
                for (const term of historyItem.recentSearchTerms) {
                    items.push({
                        name: Fuzzy.prepare(term),
                        appId: historyItem.name,
                        searchTerm: term,
                        historyItem: historyItem
                    });
                }
            }
        }
        return items;
    }
    
    // Prepared plugin history terms for fuzzy search
    // Allows finding plugins by previously used search terms (e.g., "q" -> QuickLinks)
    property var preppedPluginHistoryTerms: {
        const items = [];
        for (const historyItem of searchHistoryData) {
            if (historyItem.type === "workflow" && historyItem.recentSearchTerms) {
                for (const term of historyItem.recentSearchTerms) {
                    items.push({
                        name: Fuzzy.prepare(term),
                        pluginId: historyItem.name,
                        searchTerm: term,
                        historyItem: historyItem
                    });
                }
            }
        }
        return items;
    }
    
    // Prepared action history terms for fuzzy search
    property var preppedActionHistoryTerms: {
        const items = [];
        for (const historyItem of searchHistoryData) {
            if (historyItem.type === "action" && historyItem.recentSearchTerms) {
                for (const term of historyItem.recentSearchTerms) {
                    items.push({
                        name: Fuzzy.prepare(term),
                        actionName: historyItem.name,
                        searchTerm: term,
                        historyItem: historyItem
                    });
                }
            }
        }
        return items;
    }
    
    // Prepared quicklink history terms for fuzzy search
    property var preppedQuicklinkHistoryTerms: {
        const items = [];
        for (const historyItem of searchHistoryData) {
            if (historyItem.type === "quicklink" && historyItem.recentSearchTerms) {
                for (const term of historyItem.recentSearchTerms) {
                    items.push({
                        name: Fuzzy.prepare(term),
                        quicklinkName: historyItem.name,
                        searchTerm: term,
                        historyItem: historyItem
                    });
                }
            }
        }
        return items;
    }
    
    // Prepared plugin execution history for fuzzy search
    // Indexes both the action name and plugin name for matching
    property var preppedPluginExecutions: {
        return searchHistoryData
            .filter(item => item.type === "workflowExecution")
            .map(item => ({
                name: Fuzzy.prepare(`${item.workflowName} ${item.name}`),
                historyItem: item
            }));
    }
    
    // Prepared plugin execution history terms for fuzzy search
    // Allows finding plugin executions by previously used search terms
    property var preppedPluginExecutionHistoryTerms: {
        const items = [];
        for (const historyItem of searchHistoryData) {
            if (historyItem.type === "workflowExecution" && historyItem.recentSearchTerms) {
                for (const term of historyItem.recentSearchTerms) {
                    items.push({
                        name: Fuzzy.prepare(term),
                        executionKey: historyItem.key,
                        searchTerm: term,
                        historyItem: historyItem
                    });
                }
            }
        }
        return items;
    }
    
    // Prepared URL history terms for fuzzy search
    // Allows finding URLs by previously used search terms (e.g., "gh" -> "https://github.com")
    property var preppedUrlHistoryTerms: {
        const items = [];
        for (const historyItem of searchHistoryData) {
            if (historyItem.type === "url" && historyItem.recentSearchTerms) {
                for (const term of historyItem.recentSearchTerms) {
                    items.push({
                        name: Fuzzy.prepare(term),
                        url: historyItem.name,
                        searchTerm: term,
                        historyItem: historyItem
                    });
                }
            }
        }
        return items;
    }

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
    // searchTerm is the actual search content (e.g., "hyprland" for quicklinks, empty for apps)
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
    // All search result types (apps, files, URLs, actions, quicklinks) use the same
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
    // Detect user intent from query pattern to prioritize result categories.
    //
    // Intent Types:
    //   COMMAND   - First word is a known binary (git, ls, docker)
    //   MATH      - Starts with number, operator, or math function
    //   URL       - Matches URL pattern (domain.com)
    //   FILE      - Starts with file prefix (~)
    //   CLIPBOARD - Starts with clipboard prefix
    //   EMOJI     - Starts with emoji prefix
    //   GENERAL   - Default (app search)
    // ===========================================================
    
    readonly property var intent: ({
        COMMAND: "command",
        MATH: "math",
        URL: "url",
        FILE: "file",
        EMOJI: "emoji",
        GENERAL: "general"
    })
    
    function detectIntent(query) {
        if (!query || query.trim() === "") return root.intent.GENERAL;
        
        const trimmed = query.trim();
        
        // Explicit prefixes take priority
        if (trimmed.startsWith(Config.options.search.prefix.emojis)) return root.intent.EMOJI;
        if (trimmed.startsWith(root.filePrefix)) return root.intent.FILE;
        if (trimmed.startsWith(Config.options.search.prefix.shellCommand)) return root.intent.COMMAND;
        if (trimmed.startsWith(Config.options.search.prefix.math)) return root.intent.MATH;
        
        // Auto-detect intent from query pattern
        const firstWord = trimmed.split(/\s/)[0].toLowerCase();
        
        // Command: first word is a known binary
        if (firstWord && root.knownBinaries.has(firstWord)) return root.intent.COMMAND;
        
        // Math: starts with number, operator, parenthesis, or math function
        if (root.isMathExpression(trimmed)) return root.intent.MATH;
        
        // URL: looks like a URL
        if (root.isUrl(trimmed)) return root.intent.URL;
        
        return root.intent.GENERAL;
    }
    
    // ==================== TIERED RANKING ====================
    // Instead of magic score numbers, use category priority tiers.
    // Results are ranked within their category, then merged by tier.
    //
    // Category Priority by Intent:
    //   COMMAND → Command > Apps > Actions > Quicklinks > Others > WebSearch
    //   MATH    → Math > Apps > Actions > Quicklinks > Others > WebSearch
    //   URL     → URL > URLHistory > Apps > Quicklinks > WebSearch
    //   GENERAL → Apps > Actions > Quicklinks > URLHistory > Others > WebSearch
    //
    // Within each category, use tie-breaking:
    //   1. Match Type: Exact > Prefix > Fuzzy
    //   2. Fuzzy Score: Higher wins
    //   3. Frecency: More recent/frequent wins
    // ===========================================================
    
    readonly property var category: ({
        APP: "app",
        ACTION: "action",
        QUICKLINK: "quicklink",
        PLUGIN: "plugin",
        PLUGIN_EXECUTION: "plugin_execution",
        URL_DIRECT: "url_direct",
        URL_HISTORY: "url_history",
        EMOJI: "emoji",
        COMMAND: "command",
        MATH: "math",
        WEB_SEARCH: "web_search"
    })
    
    // ==================== TIERED + COMPETITIVE RANKING ====================
    // Tier 1: Intent-specific (always first based on detected intent)
    // Tier 2: Primary results (Apps, Actions, Quicklinks) - compete by match quality
    // Tier 3: Secondary results (Emoji, URL History, Shell History) - compete by match quality
    // Tier 4: Fallback (Web search) - always last
    // ======================================================================
    // FRECENCY-BASED RANKING
    // All results compete in a single pool sorted by match quality + frecency.
    // Web search always shown last as fallback.
    // ======================================================================
    
    // Merge all results using pure frecency-based ranking
    function mergeByFrecency(categorized, limits, detectedIntent) {
        const merged = [];
        const seenKeys = new Set();
        
        // Helper to add result if not duplicate
        const addResult = (r) => {
            const key = r.result?.id || r.result?.name;
            if (!seenKeys.has(key)) {
                seenKeys.add(key);
                merged.push(r);
            }
        };
        
        // All categories compete in one pool (except web search)
        const competingCategories = [
            root.category.APP,
            root.category.ACTION,
            root.category.PLUGIN,
            root.category.QUICKLINK,
            root.category.PLUGIN_EXECUTION,
            root.category.URL_HISTORY,
            root.category.URL_DIRECT,
            root.category.EMOJI,
            root.category.COMMAND,
            root.category.MATH
        ];
        
        const allResults = [];
        for (const cat of competingCategories) {
            const results = categorized[cat] || [];
            const limit = limits[cat] ?? 5;
            results.slice(0, limit).forEach(r => allResults.push(r));
        }
        
        // Sort by match quality + frecency
        allResults.sort(root.compareResults);
        
        // Take top results
        const maxResults = 15;
        allResults.slice(0, maxResults).forEach(addResult);
        
        // Web search always last as fallback
        const webResults = categorized[root.category.WEB_SEARCH] || [];
        if (webResults.length > 0) {
            addResult(webResults[0]);
        }
        
        return merged;
    }
    
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
    
    // Create a scored result object for an app entry
    function createAppResult(entry, score, type) {
        // Get running window info from WindowManager
        const windows = WindowManager.getWindowsForApp(entry.id);
        const windowCount = windows.length;
        
        return {
            score: score,
            result: resultComp.createObject(null, {
                type: type,
                id: entry.id,
                name: entry.name,
                iconName: entry.icon,
                iconType: LauncherSearchResult.IconType.System,
                verb: windowCount > 0 ? "Focus" : "Open",
                // Inject window info
                windowCount: windowCount,
                windows: windows,
                execute: () => {
                    // Re-fetch windows at execution time (not creation time)
                    // This ensures correct behavior when clicking from history
                    const currentWindows = WindowManager.getWindowsForApp(entry.id);
                    const currentWindowCount = currentWindows.length;
                    
                    // Smart execute based on current window count
                    if (currentWindowCount === 0) {
                        // No windows - launch new instance, record as app
                        root.recordSearch("app", entry.id, root.query);
                        if (!entry.runInTerminal)
                            entry.execute();
                        else {
                            Quickshell.execDetached(["bash", '-c', `${Config.options.apps.terminal} -e '${StringUtils.shellSingleQuoteEscape(entry.command.join(' '))}'`]);
                        }
                    } else if (currentWindowCount === 1) {
                        // Single window - auto-focus it, record as windowFocus
                        root.recordWindowFocus(entry.id, entry.name, currentWindows[0].title, entry.icon);
                        WindowManager.focusWindow(currentWindows[0]);
                        GlobalStates.launcherOpen = false;
                    } else {
                        // Multiple windows - open WindowPicker panel
                        // Don't record here - WindowPicker will record when user selects
                        GlobalStates.openWindowPicker(entry.id, currentWindows);
                    }
                },
                comment: entry.comment,
                runInTerminal: entry.runInTerminal,
                genericName: entry.genericName,
                keywords: entry.keywords,
                actions: entry.actions.map(action => {
                    return resultComp.createObject(null, {
                        name: action.name,
                        iconName: action.icon,
                        iconType: LauncherSearchResult.IconType.System,
                        execute: () => {
                            if (!action.runInTerminal)
                                action.execute();
                            else {
                                Quickshell.execDetached(["bash", '-c', `${Config.options.apps.terminal} -e '${StringUtils.shellSingleQuoteEscape(action.command.join(' '))}'`]);
                            }
                        }
                    });
                })
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

    property string mathResult: ""
    
    // Known binaries from PATH for command detection
    property var knownBinaries: new Set()
    property bool binariesLoaded: false
    
    // Load binaries from common bin directories on startup (once only)
    Component.onCompleted: {
        if (!binariesLoaded) {
            binariesProc.running = true;
        }
    }
    
    Process {
        id: binariesProc
        command: ["sh", "-c", "ls /usr/bin /usr/local/bin ~/.local/bin 2>/dev/null | sort -u"]
        stdout: SplitParser {
            onRead: data => {
                if (data.trim()) {
                    root.knownBinaries.add(data.trim().toLowerCase());
                }
            }
        }
        onRunningChanged: {
            if (!running) {
                root.binariesLoaded = true;
            }
        }
    }
    
    // Track if we're intentionally clearing query for plugin start
    property bool pluginStarting: false
    // Track if we're clearing input after receiving a response (don't trigger search)
    property bool pluginClearing: false
    
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
            if (root.query === root.filePrefix) {
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
                // Enter exclusive emoji mode when : is typed
                root.enterExclusiveMode("emoji");
            } else if (root.query === Config.options.search.prefix.math) {
                // Enter exclusive math mode when = is typed
                root.enterExclusiveMode("math");
            }
        }
    }
    
    // Submit plugin query (called on Enter key in submit mode)
    function submitPluginQuery() {
        if (PluginRunner.isActive() && PluginRunner.inputMode === "submit") {
            PluginRunner.search(root.query);
        }
    }
    
    // Exit plugin mode - should be called on Escape key
    function exitPlugin() {
        if (PluginRunner.isActive()) {
            PluginRunner.closePlugin();
            root.query = "";
        }
    }
    
    // Debounce timer for plugin search
    Timer {
        id: pluginSearchTimer
        interval: 150
        onTriggered: {
            // Double-check inputMode in case it changed while timer was running
            if (PluginRunner.isActive() && PluginRunner.inputMode === "realtime") {
                PluginRunner.search(root.query);
            }
        }
    }
    
    // URL detection regex - matches common URL patterns
    // Matches: http://, https://, ftp://, or domain.tld patterns
    property var urlRegex: /^(https?:\/\/|ftp:\/\/)?([a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}(\/[^\s]*)?$/
    
    function isUrl(text) {
        const trimmed = text.trim();
        // Check for explicit protocol
        if (/^(https?|ftp):\/\//i.test(trimmed)) {
            return true;
        }
        // Check for domain-like patterns (e.g., google.com, sub.domain.org/path)
        return urlRegex.test(trimmed);
    }
    
    function normalizeUrl(text) {
        const trimmed = text.trim();
        // Add https:// if no protocol specified
        if (!/^(https?|ftp):\/\//i.test(trimmed)) {
            return "https://" + trimmed;
        }
        return trimmed;
    }
    
    // ==================== MATH EXPRESSION DETECTION ====================
    // Detect if a query looks like a math expression before sending to qalc.
    // This prevents random text from being interpreted as math.
    //
    // Valid math expressions:
    //   - Start with number: "123", "3.14", ".5"
    //   - Start with math prefix (=)
    //   - Start with operator for implicit ans: "+5", "-3", "*2", "/4"
    //   - Start with parenthesis: "(1+2)*3"
    //   - Start with math function: "sin(", "sqrt(", "log(", etc.
    //   - Contain operators between numbers: "1+2", "3*4"
    //
    // NOT math:
    //   - Plain text: "firefox", "hello world"
    //   - URLs: "github.com"
    //   - File paths: "~/documents"
    // ==================================================================
    
    property var mathFunctionPattern: /^(sin|cos|tan|asin|acos|atan|sinh|cosh|tanh|sqrt|cbrt|log|ln|exp|abs|ceil|floor|round|factorial|rand)\s*\(/i
    property var mathExpressionPattern: /^[\d\.\(\)\+\-\*\/\^\%\s]*([\+\-\*\/\^\%][\d\.\(\)\+\-\*\/\^\%\s]*)+$/
    
    function isMathExpression(query) {
        const trimmed = query.trim();
        if (!trimmed) return false;
        
        // Explicit math prefix always triggers math
        if (trimmed.startsWith(Config.options.search.prefix.math)) return true;
        
        // Starts with digit or decimal point
        if (/^[\d\.]/.test(trimmed)) return true;
        
        // Starts with operator (implies previous answer)
        if (/^[\+\-\*\/\^]/.test(trimmed)) return true;
        
        // Starts with opening parenthesis
        if (trimmed.startsWith('(')) return true;
        
        // Starts with math function
        if (mathFunctionPattern.test(trimmed)) return true;
        
        // Contains math operators between things that look like numbers
        // e.g., "2+2", "10*5", "100/4"
        if (mathExpressionPattern.test(trimmed)) return true;
        
        return false;
    }
    
    Timer {
        id: nonAppResultsTimer
        interval: Config.options.search.nonAppResultDelay
        onTriggered: {
            let expr = root.query;
            // Strip math prefix if present (for non-exclusive mode)
            if (expr.startsWith(Config.options.search.prefix.math)) {
                expr = expr.slice(Config.options.search.prefix.math.length);
            }
            
            // In exclusive math mode, always try to calculate (query is already the expression)
            // Otherwise, only calculate if it looks like math
            if (root.exclusiveMode === "math" || root.isMathExpression(expr)) {
                if (expr.trim()) {
                    mathProc.calculateExpression(expr);
                } else {
                    root.mathResult = "";
                }
            } else {
                root.mathResult = "";
            }
        }
    }

    Process {
        id: mathProc
        property list<string> baseCommand: ["qalc", "-t"]
        function calculateExpression(expression) {
            mathProc.running = false;
            mathProc.command = baseCommand.concat(expression);
            mathProc.running = true;
        }
        stdout: SplitParser {
            onRead: data => {
                root.mathResult = data;
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
        
        // Emojis in exclusive mode - show full emoji results
        if (root.exclusiveMode === "emoji") {
            const searchString = root.query;
            return Emojis.fuzzyQuery(searchString).map(entry => {
                const emoji = entry.match(/^\s*(\S+)/)?.[1] || "";
                return resultComp.createObject(null, {
                    rawValue: entry,
                    name: entry.replace(/^\s*\S+\s+/, ""),
                    iconName: emoji,
                    iconType: LauncherSearchResult.IconType.Text,
                    verb: "Copy",
                    type: "Emoji",
                    execute: () => {
                        Quickshell.clipboardText = entry.match(/^\s*(\S+)/)?.[1];
                    }
                });
            }).filter(Boolean);
        }
        
        // Math in exclusive mode - show only math result
        if (root.exclusiveMode === "math") {
            // Trigger math calculation
            nonAppResultsTimer.restart();
            
            // Use the query directly as math expression
            if (root.mathResult && root.mathResult !== root.query) {
                return [resultComp.createObject(null, {
                    name: root.mathResult,
                    verb: "Copy",
                    type: "Math result",
                    fontType: LauncherSearchResult.FontType.Monospace,
                    iconName: 'calculate',
                    iconType: LauncherSearchResult.IconType.Material,
                    execute: () => { Quickshell.clipboardText = root.mathResult; }
                })];
            }
            return [];
        }
        
        ////////////////// Empty query - show recent history //////////////////
        if (root.query == "") {
            // Loading guard: wait for all async data sources to load before showing results
            // This prevents flickering where list populates incrementally
            if (!root.historyLoaded || !root.quicklinksLoaded || !PluginRunner.pluginsLoaded) return [];
            
            // Force dependency on quicklinks and allActions for re-evaluation
            const _quicklinksLoaded = root.quicklinks.length;
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
                        const entry = AppSearch.list.find(app => app.id === item.name);
                        if (!entry) return null;
                        return resultComp.createObject(null, {
                            type: "Recent",
                            id: entry.id,
                            name: entry.name,
                            iconName: entry.icon,
                            iconType: LauncherSearchResult.IconType.System,
                            verb: "Open",
                            actions: [makeRemoveAction("app", item.name)],
                            execute: () => {
                                // Smart execute: check windows at execution time
                                const currentWindows = WindowManager.getWindowsForApp(entry.id);
                                if (currentWindows.length === 0) {
                                    root.recordSearch("app", entry.id, "");
                                    entry.execute();
                                } else if (currentWindows.length === 1) {
                                    root.recordWindowFocus(entry.id, entry.name, currentWindows[0].title, entry.icon);
                                    WindowManager.focusWindow(currentWindows[0]);
                                    GlobalStates.launcherOpen = false;
                                } else {
                                    // Don't record - WindowPicker will record when user selects
                                    GlobalStates.openWindowPicker(entry.id, currentWindows);
                                }
                            }
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
                    } else if (item.type === "quicklink") {
                        const link = root.quicklinks.find(q => q.name === item.name);
                        if (!link) return null;
                        return resultComp.createObject(null, {
                            type: "Recent",
                            name: link.name,
                            iconName: link.icon || 'link',
                            iconType: LauncherSearchResult.IconType.Material,
                            verb: "Open",
                            actions: [makeRemoveAction("quicklink", item.name)],
                            execute: () => {
                                root.recordSearch("quicklink", link.name, "");
                                Qt.openUrlExternally(link.url.replace("{query}", ""));
                            }
                        });
                    } else if (item.type === "url") {
                        return resultComp.createObject(null, {
                            type: "Recent",
                            name: item.name,
                            fontType: LauncherSearchResult.FontType.Monospace,
                            iconName: 'open_in_browser',
                            iconType: LauncherSearchResult.IconType.Material,
                            verb: "Open",
                            actions: [makeRemoveAction("url", item.name)],
                            execute: () => {
                                root.recordSearch("url", item.name, "");
                                Quickshell.execDetached(["xdg-open", item.name]);
                            }
                        });
                    } else if (item.type === "file") {
                        const fileName = item.name.split('/').pop();
                        const displayPath = item.name.replace(/^\/home\/[^\/]+/, '~');
                        return resultComp.createObject(null, {
                            type: "Recent",
                            name: fileName,
                            comment: displayPath,
                            fontType: LauncherSearchResult.FontType.Monospace,
                            iconName: 'description',
                            iconType: LauncherSearchResult.IconType.Material,
                            verb: "Open",
                            actions: [makeRemoveAction("file", item.name)],
                            execute: () => {
                                root.recordSearch("file", item.name, "");
                                Quickshell.execDetached(["xdg-open", item.name]);
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
                .slice(0, 20);  // Take top 20 valid items
            
            return recentItems;
        }

        ////////////////// Tiered Ranking System ///////////////////
        // Uses intent detection and category-based ranking instead of magic score numbers.
        
        nonAppResultsTimer.restart();
        
        // Detect user intent from query pattern
        const detectedIntent = root.detectIntent(root.query);
        
        // Category limits - how many results from each category
        const categoryLimits = {
            [root.category.APP]: 8,
            [root.category.ACTION]: 5,
            [root.category.PLUGIN]: 5,
            [root.category.QUICKLINK]: 5,
            [root.category.URL_DIRECT]: 1,
            [root.category.URL_HISTORY]: 3,
            [root.category.CLIPBOARD]: 3,
            [root.category.EMOJI]: 3,
            [root.category.COMMAND]: 1,
            [root.category.MATH]: 1,
            [root.category.WEB_SEARCH]: 1
        };
        
        // Collect results by category
        const categorized = {};
        
        // ========== APPS ==========
        const appResults = AppSearch.fuzzyQueryWithScores(StringUtils.cleanPrefix(root.query, Config.options.search.prefix.app)).map(item => {
            const entry = item.entry;
            const frecency = root.getHistoryBoost("app", entry.id);
            const historyItem = root.searchHistoryData.find(h => h.type === "app" && h.name === entry.id);
            const recentTerms = historyItem?.recentSearchTerms || [];
            const resultMatchType = root.getTermMatchBoost(recentTerms, root.query) > 0 
                ? root.matchType.EXACT 
                : root.getMatchType(root.query, entry.name);
            
            return {
                matchType: resultMatchType,
                fuzzyScore: item.score,
                frecency: frecency,
                result: root.createAppResult(entry, 0, "App").result
            };
        });
        
        // Add app history term matches
        const appHistoryTermResults = Fuzzy.go(root.query, root.preppedAppHistoryTerms, { key: "name", limit: 5 });
        const existingAppIds = new Set(appResults.map(a => a.result.id));
        
        appHistoryTermResults
            .filter(result => !existingAppIds.has(result.obj.appId))
            .forEach(result => {
                const entry = AppSearch.list.find(app => app.id === result.obj.appId);
                if (!entry) return;
                appResults.push({
                    matchType: root.matchType.EXACT, // Term match = treated as exact
                    fuzzyScore: result._score,
                    frecency: root.getFrecencyScore(result.obj.historyItem),
                    result: root.createAppResult(entry, 0, "App").result
                });
            });
        
        categorized[root.category.APP] = appResults.sort(root.compareResults);
        
        // ========== ACTIONS ==========
        const actionQuery = root.query.startsWith(Config.options.search.prefix.action) 
            ? root.query.slice(Config.options.search.prefix.action.length).split(" ")[0]
            : root.query.split(" ")[0];
        const actionArgs = root.query.startsWith(Config.options.search.prefix.action)
            ? root.query.split(" ").slice(1).join(" ")
            : "";
        
        const seenActions = new Set();
        const actionResults = Fuzzy.go(actionQuery, root.preppedActions, { key: "name", limit: 10 }).map(result => {
            const action = result.obj.action;
            seenActions.add(action.action);
            const frecency = root.getHistoryBoost("action", action.action);
            const historyItem = root.searchHistoryData.find(h => h.type === "action" && h.name === action.action);
            const recentTerms = historyItem?.recentSearchTerms || [];
            const resultMatchType = root.getTermMatchBoost(recentTerms, actionQuery) > 0 
                ? root.matchType.EXACT 
                : root.getMatchType(actionQuery, action.action);
            
            // Actions can accept arguments (passed to execute function)
            const hasArgs = actionArgs.length > 0;
            return {
                matchType: resultMatchType,
                fuzzyScore: result._score,
                frecency: frecency,
                result: resultComp.createObject(null, {
                    name: action.action + (hasArgs ? " " + actionArgs : ""),
                    verb: "Run",
                    type: "Action",
                    iconName: 'settings_suggest',
                    iconType: LauncherSearchResult.IconType.Material,
                    acceptsArguments: !hasArgs, // Can accept args if none provided yet
                    completionText: !hasArgs ? action.action + " " : "",
                    execute: () => {
                        root.recordSearch("action", action.action, root.query);
                        action.execute(actionArgs);
                    }
                })
            };
        });
        
        // Add action history term matches
        const actionHistoryTermResults = Fuzzy.go(actionQuery, root.preppedActionHistoryTerms, { key: "name", limit: 5 });
        
        actionHistoryTermResults
            .filter(result => !seenActions.has(result.obj.actionName))
            .forEach(result => {
                const action = root.allActions.find(a => a.action === result.obj.actionName);
                if (!action) return;
                seenActions.add(action.action);
                const hasArgs = actionArgs.length > 0;
                
                actionResults.push({
                    matchType: root.matchType.EXACT, // Term match = treated as exact
                    fuzzyScore: result._score,
                    frecency: root.getFrecencyScore(result.obj.historyItem),
                    result: resultComp.createObject(null, {
                        name: action.action + (hasArgs ? " " + actionArgs : ""),
                        verb: "Run",
                        type: "Action",
                        iconName: 'settings_suggest',
                        iconType: LauncherSearchResult.IconType.Material,
                        acceptsArguments: !hasArgs,
                        completionText: !hasArgs ? action.action + " " : "",
                        execute: () => {
                            root.recordSearch("action", action.action, root.query);
                            action.execute(actionArgs);
                        }
                    })
                });
            });
        
        categorized[root.category.ACTION] = actionResults.sort(root.compareResults);
        
        // ========== PLUGINS ==========
        // Multi-step action plugins from ~/.config/hamr/plugins/
        const seenPlugins = new Set();
        const pluginResults = Fuzzy.go(actionQuery, root.preppedPlugins, { key: "name", limit: 10 })
            .filter(result => {
                const id = result.obj.plugin.id;
                if (seenPlugins.has(id)) return false;
                seenPlugins.add(id);
                return true;
            })
            .map(result => {
                const plugin = result.obj.plugin;
                const manifest = plugin.manifest;
                const frecency = root.getHistoryBoost("workflow", plugin.id);
                const historyItem = root.searchHistoryData.find(h => h.type === "workflow" && h.name === plugin.id);
                const recentTerms = historyItem?.recentSearchTerms || [];
                const resultMatchType = root.getTermMatchBoost(recentTerms, actionQuery) > 0 
                    ? root.matchType.EXACT 
                    : root.getMatchType(actionQuery, plugin.id);
                
                return {
                    matchType: resultMatchType,
                    fuzzyScore: result._score,
                    frecency: frecency,
                    result: resultComp.createObject(null, {
                        name: manifest.name ?? plugin.id,
                        comment: manifest.description ?? "",
                        verb: "Start",
                        type: "Plugin",
                        iconName: manifest.icon ?? 'extension',
                        iconType: LauncherSearchResult.IconType.Material,
                        resultType: LauncherSearchResult.ResultType.PluginEntry,
                        pluginId: plugin.id,
                        acceptsArguments: true,
                        completionText: plugin.id + " ",
                        execute: () => {
                            root.recordSearch("workflow", plugin.id, root.query);
                            root.startPlugin(plugin.id);
                        }
                    })
                };
            });
        
        // Add plugin history term matches (e.g., "q" -> QuickLinks if user previously typed "q" to find it)
        const pluginHistoryTermResults = Fuzzy.go(actionQuery, root.preppedPluginHistoryTerms, { key: "name", limit: 5 });
        
        pluginHistoryTermResults.forEach(result => {
                if (seenPlugins.has(result.obj.pluginId)) return;
                const plugin = PluginRunner.getPlugin(result.obj.pluginId);
                if (!plugin) return;
                seenPlugins.add(result.obj.pluginId);
                const manifest = plugin.manifest;
                pluginResults.push({
                    matchType: root.matchType.EXACT, // Term match = treated as exact
                    fuzzyScore: result._score,
                    frecency: root.getFrecencyScore(result.obj.historyItem),
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
                        execute: () => {
                            root.recordSearch("workflow", plugin.id, root.query);
                            root.startPlugin(plugin.id);
                        }
                    })
                });
            });
        
        categorized[root.category.PLUGIN] = pluginResults.sort(root.compareResults).slice(0, 3);
        
        // ========== QUICKLINKS ==========
        const queryParts = root.query.split(" ");
        const quicklinkQuery = queryParts[0];
        const quicklinkSearchTerm = queryParts.slice(1).join(" ");
        const hasSearchTerm = quicklinkSearchTerm.length > 0;
        
        const quicklinkFuzzyResults = Fuzzy.go(quicklinkQuery, root.preppedQuicklinks, { key: "name", limit: 16 });
        const seenQuicklinks = new Set();
        const quicklinkResults = [];
        
        quicklinkFuzzyResults.filter(result => {
            const linkName = result.obj.quicklink.name;
            if (seenQuicklinks.has(linkName)) return false;
            seenQuicklinks.add(linkName);
            return true;
        }).slice(0, 8).forEach(result => {
            const link = result.obj.quicklink;
            const displayName = link.name;
            const matchedVia = result.obj.isAlias ? ` (${result.obj.aliasName})` : "";
            const frecency = root.getHistoryBoost("quicklink", link.name);
            const resultMatchType = root.getMatchType(quicklinkQuery, link.name);
            
            // Check if this quicklink accepts a query argument
            const acceptsQuery = link.url.includes("{query}");
            
            // Check term match boost for discovery (how user found this quicklink)
            const historyItem = searchHistoryData.find(h => h.type === "quicklink" && h.name === link.name);
            const recentTerms = historyItem?.recentSearchTerms || [];
            const termMatchBoost = root.getTermMatchBoost(recentTerms, quicklinkQuery);
            const boostedMatchType = termMatchBoost > 0 ? root.matchType.EXACT : resultMatchType;
            
            if (hasSearchTerm) {
                quicklinkResults.push({
                    matchType: boostedMatchType,
                    fuzzyScore: result._score,
                    frecency: frecency,
                    result: resultComp.createObject(null, {
                        name: `${displayName}: ${quicklinkSearchTerm}`,
                        verb: "Search",
                        type: "Quicklink" + matchedVia,
                        acceptsArguments: false, // Already has argument
                        completionText: "",
                        iconName: link.icon || 'link',
                        iconType: LauncherSearchResult.IconType.Material,
                        execute: ((capturedDiscoveryTerm) => () => {
                            // Record the discovery term (how user found quicklink)
                            root.recordSearch("quicklink", link.name, capturedDiscoveryTerm);
                            const url = link.url.replace("{query}", encodeURIComponent(quicklinkSearchTerm));
                            Qt.openUrlExternally(url);
                        })(quicklinkQuery)
                    })
                });
            } else {
                // Show recent search terms for this quicklink
                recentTerms.forEach((term, idx) => {
                    quicklinkResults.push({
                        matchType: boostedMatchType,
                        fuzzyScore: result._score - idx * 10, // Slight penalty for older terms
                        frecency: frecency,
                        result: resultComp.createObject(null, {
                            name: `${displayName}: ${term}`,
                            verb: "Search",
                            type: "Quicklink" + matchedVia + " - recent",
                            iconName: link.icon || 'link',
                            iconType: LauncherSearchResult.IconType.Material,
                            acceptsArguments: false, // Already has argument
                            completionText: "",
                            execute: ((capturedDiscoveryTerm, capturedSearchTerm) => () => {
                                // Record the discovery term (how user found quicklink)
                                root.recordSearch("quicklink", link.name, capturedDiscoveryTerm);
                                const url = link.url.replace("{query}", encodeURIComponent(capturedSearchTerm));
                                Qt.openUrlExternally(url);
                            })(quicklinkQuery, term)
                        })
                    });
                });
                
                quicklinkResults.push({
                    matchType: boostedMatchType,
                    fuzzyScore: result._score - 50, // Base quicklink below recent searches
                    frecency: frecency,
                    result: resultComp.createObject(null, {
                        name: displayName,
                        verb: acceptsQuery ? "Search" : "Open",
                        type: "Quicklink" + matchedVia,
                        iconName: link.icon || 'link',
                        iconType: LauncherSearchResult.IconType.Material,
                        acceptsArguments: acceptsQuery,
                        completionText: acceptsQuery ? displayName + " " : "",
                        execute: ((capturedDiscoveryTerm) => () => {
                            // Record the discovery term (how user found quicklink)
                            root.recordSearch("quicklink", link.name, capturedDiscoveryTerm);
                            const url = link.url.replace("{query}", "");
                            Qt.openUrlExternally(url);
                        })(quicklinkQuery)
                    })
                });
            }
        });
        
        // Add quicklink history term matches (find quicklinks by previously used search terms)
        // This is for finding the quicklink itself, not the search terms used with it
        const quicklinkHistoryTermResults = Fuzzy.go(quicklinkQuery, root.preppedQuicklinkHistoryTerms, { key: "name", limit: 5 });
        
        quicklinkHistoryTermResults
            .filter(result => !seenQuicklinks.has(result.obj.quicklinkName))
            .forEach(result => {
                const link = root.quicklinks.find(q => q.name === result.obj.quicklinkName);
                if (!link) return;
                seenQuicklinks.add(result.obj.quicklinkName);
                const acceptsQuery = link.url.includes("{query}");
                
                quicklinkResults.push({
                    matchType: root.matchType.EXACT, // Term match = treated as exact
                    fuzzyScore: result._score,
                    frecency: root.getFrecencyScore(result.obj.historyItem),
                    result: resultComp.createObject(null, {
                        name: link.name,
                        verb: acceptsQuery ? "Search" : "Open",
                        type: "Quicklink",
                        iconName: link.icon || 'link',
                        iconType: LauncherSearchResult.IconType.Material,
                        acceptsArguments: acceptsQuery,
                        completionText: acceptsQuery ? link.name + " " : "",
                        execute: ((capturedDiscoveryTerm) => () => {
                            root.recordSearch("quicklink", link.name, capturedDiscoveryTerm);
                            const url = link.url.replace("{query}", "");
                            Qt.openUrlExternally(url);
                        })(quicklinkQuery)
                    })
                });
            });
        
        categorized[root.category.QUICKLINK] = quicklinkResults.sort(root.compareResults);
        
        // ========== URL (Direct) ==========
        const isQueryUrl = root.isUrl(root.query);
        const normalizedUrl = isQueryUrl ? root.normalizeUrl(root.query) : "";
        
        if (isQueryUrl) {
            categorized[root.category.URL_DIRECT] = [{
                matchType: root.matchType.EXACT,
                fuzzyScore: 1000,
                frecency: root.getHistoryBoost("url", normalizedUrl),
                result: resultComp.createObject(null, {
                    name: normalizedUrl,
                    verb: "Open",
                    type: "URL",
                    fontType: LauncherSearchResult.FontType.Monospace,
                    iconName: 'open_in_browser',
                    iconType: LauncherSearchResult.IconType.Material,
                    execute: ((capturedQuery) => () => {
                        root.recordSearch("url", normalizedUrl, capturedQuery);
                        Quickshell.execDetached(["xdg-open", normalizedUrl]);
                    })(root.query)
                })
            }];
        } else {
            categorized[root.category.URL_DIRECT] = [];
        }
        
        // ========== URL History ==========
        const seenUrls = new Set([normalizedUrl]);
        const urlHistoryResults = Fuzzy.go(root.query, root.preppedUrlHistory, { key: "name", limit: 5 })
            .filter(result => result.obj.url !== normalizedUrl)
            .map(result => {
                const url = result.obj.url;
                seenUrls.add(url);
                const historyItem = result.obj.historyItem;
                const recentTerms = historyItem?.recentSearchTerms || [];
                const resultMatchType = root.getTermMatchBoost(recentTerms, root.query) > 0 
                    ? root.matchType.EXACT 
                    : root.getMatchType(root.query, root.stripProtocol(url));
                
                return {
                    matchType: resultMatchType,
                    fuzzyScore: result._score,
                    frecency: root.getFrecencyScore(historyItem),
                    result: resultComp.createObject(null, {
                        name: url,
                        verb: "Open",
                        type: "URL" + " - recent",
                        fontType: LauncherSearchResult.FontType.Monospace,
                        iconName: 'open_in_browser',
                        iconType: LauncherSearchResult.IconType.Material,
                        execute: ((capturedQuery) => () => {
                            root.recordSearch("url", url, capturedQuery);
                            Quickshell.execDetached(["xdg-open", url]);
                        })(root.query)
                    })
                };
            });
        
        // Add URL history term matches
        const urlHistoryTermResults = Fuzzy.go(root.query, root.preppedUrlHistoryTerms, { key: "name", limit: 5 });
        
        urlHistoryTermResults
            .filter(result => !seenUrls.has(result.obj.url))
            .forEach(result => {
                const url = result.obj.url;
                seenUrls.add(url);
                
                urlHistoryResults.push({
                    matchType: root.matchType.EXACT,
                    fuzzyScore: result._score,
                    frecency: root.getFrecencyScore(result.obj.historyItem),
                    result: resultComp.createObject(null, {
                        name: url,
                        verb: "Open",
                        type: "URL" + " - recent",
                        fontType: LauncherSearchResult.FontType.Monospace,
                        iconName: 'open_in_browser',
                        iconType: LauncherSearchResult.IconType.Material,
                        execute: ((capturedQuery) => () => {
                            root.recordSearch("url", url, capturedQuery);
                            Quickshell.execDetached(["xdg-open", url]);
                        })(root.query)
                    })
                });
            });
        
        categorized[root.category.URL_HISTORY] = urlHistoryResults.sort(root.compareResults);
        
        // ========== PLUGIN EXECUTIONS ==========
        const seenPluginExecutions = new Set();
        const pluginExecResults = Fuzzy.go(root.query, root.preppedPluginExecutions, { key: "name", limit: 5 })
            .map(result => {
                const item = result.obj.historyItem;
                seenPluginExecutions.add(item.key);
                const recentTerms = item.recentSearchTerms || [];
                const resultMatchType = root.getTermMatchBoost(recentTerms, root.query) > 0 
                    ? root.matchType.EXACT 
                    : root.matchType.FUZZY;
                
                // Determine icon type from stored value
                const pluginIconType = item.iconType === "system" 
                    ? LauncherSearchResult.IconType.System 
                    : LauncherSearchResult.IconType.Material;
                return {
                    matchType: resultMatchType,
                    fuzzyScore: result._score,
                    frecency: root.getFrecencyScore(item),
                    result: resultComp.createObject(null, {
                        type: item.workflowName || "Recent",
                        name: item.name,
                        iconName: item.icon || 'play_arrow',
                        iconType: pluginIconType,
                        thumbnail: item.thumbnail || "",
                        verb: "Run",
                        execute: ((capturedQuery) => () => {
                            root.recordWorkflowExecution({
                                name: item.name,
                                command: item.command,
                                entryPoint: item.entryPoint,
                                icon: item.icon,
                                iconType: item.iconType,
                                thumbnail: item.thumbnail,
                                workflowId: item.workflowId,
                                workflowName: item.workflowName
                            }, capturedQuery);
                            // Hybrid replay: prefer command, fallback to entryPoint
                            if (item.command && item.command.length > 0) {
                                // Direct command execution (fast path)
                                Quickshell.execDetached(item.command);
                            } else if (item.entryPoint && item.workflowId) {
                                // Plugin replay via entryPoint (complex actions)
                                PluginRunner.replayAction(item.workflowId, item.entryPoint);
                            }
                        })(root.query)
                    })
                };
            });
        
        // Add plugin execution history term matches
        const pluginExecHistoryTermResults = Fuzzy.go(root.query, root.preppedPluginExecutionHistoryTerms, { key: "name", limit: 5 });
        
        pluginExecHistoryTermResults
            .filter(result => !seenPluginExecutions.has(result.obj.executionKey))
            .forEach(result => {
                const item = result.obj.historyItem;
                seenPluginExecutions.add(item.key);
                
                // Determine icon type from stored value
                const termIconType = item.iconType === "system" 
                    ? LauncherSearchResult.IconType.System 
                    : LauncherSearchResult.IconType.Material;
                pluginExecResults.push({
                    matchType: root.matchType.EXACT, // Term match = treated as exact
                    fuzzyScore: result._score,
                    frecency: root.getFrecencyScore(item),
                    result: resultComp.createObject(null, {
                        type: item.workflowName || "Recent",
                        name: item.name,
                        iconName: item.icon || 'play_arrow',
                        iconType: termIconType,
                        thumbnail: item.thumbnail || "",
                        verb: "Run",
                        execute: ((capturedQuery) => () => {
                            root.recordWorkflowExecution({
                                name: item.name,
                                command: item.command,
                                entryPoint: item.entryPoint,
                                icon: item.icon,
                                iconType: item.iconType,
                                thumbnail: item.thumbnail,
                                workflowId: item.workflowId,
                                workflowName: item.workflowName
                            }, capturedQuery);
                            if (item.command && item.command.length > 0) {
                                Quickshell.execDetached(item.command);
                            } else if (item.entryPoint && item.workflowId) {
                                PluginRunner.replayAction(item.workflowId, item.entryPoint);
                            }
                        })(root.query)
                    })
                });
            });
        
        categorized[root.category.PLUGIN_EXECUTION] = pluginExecResults.sort(root.compareResults);
        
        // ========== EMOJI ==========
        const emojiResults = Fuzzy.go(root.query, Emojis.preparedEntries, { key: "name", limit: 3 }).map(result => {
            const entry = result.obj.entry;
            const emoji = entry.match(/^\s*(\S+)/)?.[1] || "";
            return {
                matchType: root.matchType.FUZZY,
                fuzzyScore: result._score,
                frecency: 0,
                result: resultComp.createObject(null, {
                    rawValue: entry,
                    name: entry.replace(/^\s*\S+\s+/, ""),
                    iconName: emoji,
                    iconType: LauncherSearchResult.IconType.Text,
                    verb: "Copy",
                    type: "Emoji",
                    execute: () => { Quickshell.clipboardText = emoji; }
                })
            };
        });
        categorized[root.category.EMOJI] = emojiResults.sort(root.compareResults);
        
        // ========== COMMAND ==========
        // Only include if intent is COMMAND
        if (detectedIntent === root.intent.COMMAND) {
            categorized[root.category.COMMAND] = [{
                matchType: root.matchType.EXACT,
                fuzzyScore: 1000,
                frecency: 0,
                result: resultComp.createObject(null, {
                    name: FileUtils.trimFileProtocol(StringUtils.cleanPrefix(root.query, Config.options.search.prefix.shellCommand)),
                    verb: "Run",
                    type: "Command",
                    fontType: LauncherSearchResult.FontType.Monospace,
                    iconName: 'terminal',
                    iconType: LauncherSearchResult.IconType.Material,
                    execute: () => {
                        let cleanedCommand = FileUtils.trimFileProtocol(root.query);
                        cleanedCommand = StringUtils.cleanPrefix(cleanedCommand, Config.options.search.prefix.shellCommand);
                        Quickshell.execDetached(["ghostty", "--class=floating.terminal", "-e", "zsh", "-ic", cleanedCommand]);
                    }
                })
            }];
        } else {
            categorized[root.category.COMMAND] = [];
        }
        
        
        // ========== MATH ==========
        const isMath = root.isMathExpression(root.query);
        if (isMath && root.mathResult && root.mathResult !== root.query) {
            categorized[root.category.MATH] = [{
                matchType: root.matchType.EXACT,
                fuzzyScore: 1000,
                frecency: 0,
                result: resultComp.createObject(null, {
                    name: root.mathResult,
                    verb: "Copy",
                    type: "Math result",
                    fontType: LauncherSearchResult.FontType.Monospace,
                    iconName: 'calculate',
                    iconType: LauncherSearchResult.IconType.Material,
                    execute: () => { Quickshell.clipboardText = root.mathResult; }
                })
            }];
        } else {
            categorized[root.category.MATH] = [];
        }
        
        // ========== WEB SEARCH (always last) ==========
        categorized[root.category.WEB_SEARCH] = [{
            matchType: root.matchType.NONE,
            fuzzyScore: 0,
            frecency: 0,
            result: resultComp.createObject(null, {
                name: StringUtils.cleanPrefix(root.query, Config.options.search.prefix.webSearch),
                verb: "Search",
                type: "Web search",
                iconName: 'travel_explore',
                iconType: LauncherSearchResult.IconType.Material,
                execute: () => {
                    let query = StringUtils.cleanPrefix(root.query, Config.options.search.prefix.webSearch);
                    let url = Config.options.search.engineBaseUrl + query;
                    for (let site of Config.options.search.excludedSites) {
                        url += ` -site:${site}`;
                    }
                    Qt.openUrlExternally(url);
                }
            })
        }];
        
        // Merge results using pure frecency-based ranking
        const merged = root.mergeByFrecency(categorized, categoryLimits, detectedIntent);
        
        return merged.map(item => item.result);
    }

    Component {
        id: resultComp
        LauncherSearchResult {}
    }
}
