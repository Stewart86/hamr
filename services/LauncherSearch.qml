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
    property bool skipNextAutoFocus: false
    property bool historyLoaded: false
    
    property string exclusiveMode: ""
    property bool exclusiveModeStarting: false
    
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
    
    property string indexIsolationPlugin: ""
    
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

    readonly property var excludedActionExtensions: [".md", ".txt", ".json", ".yaml", ".yml", ".toml", ".ini", ".cfg", ".conf", ".log", ".csv", ".sh"]
    readonly property var excludedActionPrefixes: ["test-", "hamr-test"]
    
    function extractScriptsFromFolder(folderModel: FolderListModel): list<var> {
        const actions = [];
        for (let i = 0; i < folderModel.count; i++) {
             const fileName = folderModel.get(i, "fileName");
             const filePath = folderModel.get(i, "filePath");
             if (fileName && filePath) {
                 const lowerName = fileName.toLowerCase();
                 if (root.excludedActionExtensions.some(ext => lowerName.endsWith(ext))) {
                     continue;
                 }
                 if (root.excludedActionPrefixes.some(prefix => lowerName.startsWith(prefix))) {
                     continue;
                 }
                 
                 const actionName = fileName.replace(/\.[^/.]+$/, "");
                const scriptPath = FileUtils.trimFileProtocol(filePath);
                actions.push({
                     action: actionName,
                     execute: ((path) => (args) => {
                         Quickshell.execDetached(["bash", path, ...(args ? args.split(" ") : [])]);
                     })(scriptPath)
                });
            }
        }
        return actions;
    }
    
    property var userActionScripts: extractScriptsFromFolder(userActionsFolder)
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
    
    property bool pluginActive: PluginRunner.activePlugin !== null
    property string activePluginId: PluginRunner.activePlugin?.id ?? ""
    
    function startPlugin(pluginId) {
         const success = PluginRunner.startPlugin(pluginId);
         if (success) {
             root.exclusiveMode = "";
             root.pluginStarting = true;
             root.query = "";
             root.pluginStarting = false;
             root.lastEscapeTime = 0;
         }
        return success;
    }
    
    function startPluginWithQuery(pluginId, initialQuery) {
         const success = PluginRunner.startPlugin(pluginId);
         if (success) {
             root.exclusiveMode = "";
             root.lastEscapeTime = 0;
             matchPatternSearchTimer.query = initialQuery;
             matchPatternSearchTimer.restart();
         }
         return success;
     }
     
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
     
    function closePlugin() {
        PluginRunner.closePlugin();
    }
    
    function checkPluginExit() {
         if (PluginRunner.isActive() && root.query === "") {
             PluginRunner.closePlugin();
         }
     }
     
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
    
    function pluginResultsToSearchResults(pluginResults: var): var {
         return pluginResults.map(item => {
             const itemId = item.id;
             
             const itemActions = (item.actions ?? []).map(action => {
                 const actionId = action.id;
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
             
             const iconName = item.icon ?? PluginRunner.activePlugin?.manifest?.icon ?? 'extension';
             let isSystemIcon;
             if (item.iconType === "system") {
                 isSystemIcon = true;
             } else if (item.iconType === "material") {
                 isSystemIcon = false;
             } else {
                 isSystemIcon = iconName.includes('.') || iconName.includes('-');
             }
             
             const executeCommand = item.execute?.command ?? null;
             const executeNotify = item.execute?.notify ?? null;
             const executeName = item.execute?.name ?? null;
             const pluginId = PluginRunner.activePlugin?.id ?? "";
             const pluginName = PluginRunner.activePlugin?.manifest?.name ?? "Plugin";
             
             return resultComp.createObject(null, {
                 id: itemId,
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
                     if (capturedExecuteCommand) {
                         Quickshell.execDetached(capturedExecuteCommand);
                         if (capturedExecuteNotify) {
                             Quickshell.execDetached(["notify-send", capturedPluginName, capturedExecuteNotify, "-a", "Shell"]);
                         }
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
                     PluginRunner.selectItem(capturedItemId, "");
                 })(itemId, executeCommand, executeNotify, executeName, pluginId, pluginName, iconName)
             });
         });
     }
    
    property var preppedPlugins: PluginRunner.preppedPlugins
    
    readonly property var sourceType: ({
        PLUGIN: "plugin",
        PLUGIN_EXECUTION: "pluginExecution",
        WEB_SEARCH: "webSearch",
        INDEXED_ITEM: "indexedItem"
    })
    
    property var preppedStaticSearchables: []
    
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
         
         const indexedItems = PluginRunner.getAllIndexedItems();
         for (const item of indexedItems) {
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
    
    Connections {
        target: Quickshell
        function onReloadCompleted() {
            root.rebuildStaticSearchables();
        }
    }
    
    Connections {
        target: PluginRunner
        function onPluginsChanged() {
            root.rebuildStaticSearchables();
        }
        function onPluginIndexChanged(pluginId) {
            root.rebuildStaticSearchables();
        }
    }
    
    onAllActionsChanged: {
        root.rebuildStaticSearchables();
    }
    
    property var preppedHistorySearchables: []
    
     function rebuildHistorySearchables() {
         const items = [];
         
         const indexedItems = PluginRunner.getAllIndexedItems();
         for (const historyItem of searchHistoryData.filter(h => h.type === "app" && h.recentSearchTerms?.length > 0)) {
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
         
         for (const historyItem of searchHistoryData.filter(h => h.type === "workflowExecution")) {
             items.push({
                 name: Fuzzy.prepare(`${historyItem.workflowName} ${historyItem.name}`),
                 sourceType: root.sourceType.PLUGIN_EXECUTION,
                 id: historyItem.key,
                 data: { historyItem },
                 isHistoryTerm: false
             });
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
    
    onSearchHistoryDataChanged: {
        root.rebuildHistorySearchables();
    }
    
    property var preppedSearchables: [...preppedStaticSearchables, ...preppedHistorySearchables]

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
                 console.error("[SearchHistory] Failed to parse:", e);
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

    property int maxRecentSearchTerms: 5
    
    function recordSearch(searchType, searchName, searchTerm) {
        const now = Date.now();
        const existingIndex = searchHistoryData.findIndex(
            h => h.type === searchType && h.name === searchName
        );
        
        let newHistory = searchHistoryData.slice();
        
         if (existingIndex >= 0) {
             const existing = newHistory[existingIndex];
             let recentTerms = existing.recentSearchTerms || [];
             
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
             newHistory.unshift({
                 type: searchType,
                 name: searchName,
                 count: 1,
                 lastUsed: now,
                 recentSearchTerms: searchTerm ? [searchTerm] : []
             });
         }
         
         newHistory = ageAndPruneHistory(newHistory, now);
        
        // Trim to max items
        if (newHistory.length > maxHistoryItems) {
            newHistory = newHistory.slice(0, maxHistoryItems);
        }
        
        searchHistoryData = newHistory;
        searchHistoryFileView.setText(JSON.stringify({ history: newHistory }, null, 2));
    }
    
    function recordWorkflowExecution(actionInfo, searchTerm) {
        const now = Date.now();
        // Use name + workflowId as unique key
        const key = `${actionInfo.workflowId}:${actionInfo.name}`;
        const existingIndex = searchHistoryData.findIndex(
            h => h.type === "workflowExecution" && h.key === key
        );
        
        let newHistory = searchHistoryData.slice();
        
         if (existingIndex >= 0) {
             const existing = newHistory[existingIndex];
             let recentTerms = existing.recentSearchTerms || [];
             
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
    
    function recordWindowFocus(appId, appName, windowTitle, iconName) {
        const now = Date.now();
        // Use appId + windowTitle as unique key
        const key = `windowFocus:${appId}:${windowTitle}`;
        const existingIndex = searchHistoryData.findIndex(
            h => h.type === "windowFocus" && h.key === key
        );
        
        let newHistory = searchHistoryData.slice();
        
         if (existingIndex >= 0) {
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
     
     function ageAndPruneHistory(history, now) {
         let totalCount = history.reduce((sum, item) => sum + item.count, 0);
         
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
         
         const maxAgeMs = maxAgeDays * 24 * 60 * 60 * 1000;
         history = history.filter(item => {
             const age = now - item.lastUsed;
             const isOld = age > maxAgeMs;
             const hasLowScore = item.count < 1;
             return !(isOld && hasLowScore);
         });
         
         return history;
     }

    property int maxTotalScore: 10000
    property int maxAgeDays: 90
    
    function getFrecencyScore(historyItem) {
        if (!historyItem) return 0;
         const now = Date.now();
         const hoursSinceUse = (now - historyItem.lastUsed) / (1000 * 60 * 60);
         
         let recencyMultiplier;
         if (hoursSinceUse < 1) recencyMultiplier = 4;
         else if (hoursSinceUse < 24) recencyMultiplier = 2;
         else if (hoursSinceUse < 168) recencyMultiplier = 1;
         else recencyMultiplier = 0.5;
         
         return historyItem.count * recencyMultiplier;
     }

    function getHistoryBoost(searchType, searchName) {
        const historyItem = searchHistoryData.find(
            h => h.type === searchType && h.name === searchName
        );
        return getFrecencyScore(historyItem);
    }
     
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
    
    function compareResults(a, b) {
         const aIsExact = a.matchType === root.matchType.EXACT;
         const bIsExact = b.matchType === root.matchType.EXACT;
         
         if (aIsExact !== bIsExact) {
             return aIsExact ? -1 : 1;
         }
         
         if (aIsExact && bIsExact) {
             if (Math.abs(a.frecency - b.frecency) > 1) {
                 return b.frecency - a.frecency;
             }
             return b.fuzzyScore - a.fuzzyScore;
         }
         
         if (a.fuzzyScore !== b.fuzzyScore) {
             return b.fuzzyScore - a.fuzzyScore;
         }
         return b.frecency - a.frecency;
     }
    
    property real frecencyBoostFactor: 50
    property real maxFrecencyBoost: 500
    
    function getCombinedScore(fuzzyScore, frecencyBoost) {
         const boost = Math.min(frecencyBoost * frecencyBoostFactor, maxFrecencyBoost);
         return fuzzyScore + boost;
     }
     
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
     
    function unifiedFuzzySearch(query, limit) {
        if (!query || query.trim() === "") return [];
        
         const fuzzyResults = Fuzzy.go(query, root.preppedSearchables, {
             key: "name",
             limit: limit * 3
         });
         
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
     
    function createResultFromSearchable(item, query, fuzzyScore) {
         const st = root.sourceType;
         const data = item.data;
         const resultMatchType = item.isHistoryTerm ? root.matchType.EXACT : root.matchType.FUZZY;
         
         let frecency = 0;
         if (data.historyItem) {
             frecency = root.getFrecencyScore(data.historyItem);
         } else {
             switch (item.sourceType) {
                 case st.PLUGIN:
                     if (data.isAction) {
                         frecency = root.getHistoryBoost("action", data.action.action);
                     } else {
                         frecency = root.getHistoryBoost("workflow", data.plugin.id);
                     }
                     break;
                 case st.INDEXED_ITEM:
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
    
    function createPluginResultFromData(data, itemId, query, fuzzyScore, frecency, resultMatchType) {
         if (data.isAction) {
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
     
    function createIndexedItemResultFromData(data, query, fuzzyScore, frecency, resultMatchType) {
         const item = data.item;
         
         let iconType;
         if (item.iconType === "text") {
             iconType = LauncherSearchResult.IconType.Text;
         } else if (item.iconType === "system") {
             iconType = LauncherSearchResult.IconType.System;
         } else {
             iconType = LauncherSearchResult.IconType.Material;
         }
         
         const isAppItem = item.appId !== undefined;
         const appId = item.appId ?? "";
         
         const windows = isAppItem ? WindowManager.getWindowsForApp(appId) : [];
         const windowCount = windows.length;
         
         const itemActions = (item.actions ?? []).map(action => {
             const actionIconType = action.iconType === "system" 
                 ? LauncherSearchResult.IconType.System 
                 : LauncherSearchResult.IconType.Material;
             return resultComp.createObject(null, {
                 name: action.name,
                 iconName: action.icon ?? 'play_arrow',
                 iconType: actionIconType,
                 execute: ((capturedAction, capturedItem) => () => {
                     if (capturedAction.entryPoint) {
                         if (capturedAction.keepOpen) {
                             PluginRunner.executeEntryPoint(capturedItem._pluginId, capturedAction.entryPoint);
                         } else {
                             PluginRunner.replayAction(capturedItem._pluginId, capturedAction.entryPoint);
                             GlobalStates.launcherOpen = false;
                         }
                         return;
                     }
                     if (capturedAction.command) {
                         Quickshell.execDetached(capturedAction.command);
                         GlobalStates.launcherOpen = false;
                     }
                     if (capturedItem.appId) {
                         root.recordSearch("app", capturedItem.appId, query);
                     }
                 })(action, item)
             });
         });
         
         let verb = item.verb ?? (item.execute?.notify ? "Copy" : "Run");
         if (item.entryPoint) {
             verb = item.verb ?? "Copy";
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
                         const currentWindows = WindowManager.getWindowsForApp(capturedAppId);
                         const currentWindowCount = currentWindows.length;
                         
                         if (currentWindowCount === 0) {
                             root.recordSearch("app", capturedAppId, capturedQuery);
                             if (capturedItem.execute?.command) {
                                 Quickshell.execDetached(capturedItem.execute.command);
                             }
                         } else if (currentWindowCount === 1) {
                             root.recordWindowFocus(capturedAppId, capturedItem.name, currentWindows[0].title, capturedItem.icon);
                             WindowManager.focusWindow(currentWindows[0]);
                             GlobalStates.launcherOpen = false;
                         } else {
                             GlobalStates.openWindowPicker(capturedAppId, currentWindows);
                         }
                     } else {
                         if (capturedItem.entryPoint) {
                             if (capturedItem.keepOpen) {
                                 PluginRunner.executeEntryPoint(capturedItem._pluginId, capturedItem.entryPoint);
                             } else {
                                 PluginRunner.replayAction(capturedItem._pluginId, capturedItem.entryPoint);
                                 GlobalStates.launcherOpen = false;
                             }
                             return;
                         }
                         
                         if (capturedItem.execute?.command) {
                             Quickshell.execDetached(capturedItem.execute.command);
                         }
                         if (capturedItem.execute?.notify) {
                             Quickshell.execDetached(["notify-send", capturedItem._pluginName ?? "Plugin", capturedItem.execute.notify, "-a", "Shell"]);
                         }
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

    property var allActions: {
         const combined = [...searchActions, ...builtinActionScripts];
         for (const userScript of userActionScripts) {
             const existingIdx = combined.findIndex(a => a.action === userScript.action);
             if (existingIdx >= 0) {
                 combined[existingIdx] = userScript;
             } else {
                 combined.push(userScript);
             }
         }
         return combined;
     }

    property var preppedActions: allActions.map(a => ({
         name: Fuzzy.prepare(a.action),
         action: a
     }))

    Component.onCompleted: {
         Qt.callLater(root.rebuildStaticSearchables);
     }
     
    property bool pluginStarting: false
    property bool pluginClearing: false
    property string matchPatternQuery: ""
    
    onQueryChanged: {
         if (PluginRunner.isActive()) {
             if (!root.pluginStarting && !root.pluginClearing) {
                 if (PluginRunner.inputMode === "realtime") {
                     pluginSearchTimer.restart();
                 }
             }
         } else if (root.isInExclusiveMode()) {
         } else if (!root.exclusiveModeStarting) {
             if (root.query === Config.options.search.prefix.file) {
                 root.startPlugin("files");
             } else if (root.query === Config.options.search.prefix.clipboard) {
                 root.startPlugin("clipboard");
             } else if (root.query === Config.options.search.prefix.shellHistory) {
                 root.startPlugin("shell");
             } else if (root.query === Config.options.search.prefix.action) {
                 root.enterExclusiveMode("action");
             } else if (root.query === Config.options.search.prefix.emojis) {
                 root.startPlugin("emoji");
             } else if (root.query === Config.options.search.prefix.math) {
                 root.startPlugin("calculate");
             } else if (root.query.length >= 2) {
                 matchPatternCheckTimer.restart();
             }
         }
     }
     
     Timer {
         id: matchPatternCheckTimer
         interval: 50
         onTriggered: {
             if (PluginRunner.isActive() || root.isInExclusiveMode()) return;
             
             const match = PluginRunner.findMatchingPlugin(root.query);
             if (match) {
                 root.matchPatternQuery = root.query;
                 root.startPluginWithQuery(match.pluginId, root.query);
             }
         }
     }
     
    function submitPluginQuery() {
         if (PluginRunner.isActive() && PluginRunner.inputMode === "submit") {
             PluginRunner.search(root.query);
         }
     }
     
    property real lastEscapeTime: 0
    readonly property int doubleEscapeThreshold: 300
    
    function exitPlugin() {
        if (!PluginRunner.isActive()) return;
        
         const now = Date.now();
         const timeSinceLastEscape = now - root.lastEscapeTime;
         root.lastEscapeTime = now;
         
         if (timeSinceLastEscape < root.doubleEscapeThreshold && PluginRunner.navigationDepth > 0) {
             PluginRunner.closePlugin();
             root.query = "";
             return;
         }
         
         const wasAtInitial = PluginRunner.navigationDepth <= 0;
         PluginRunner.goBack();
         if (wasAtInitial) {
             root.query = "";
         }
     }
     
    Timer {
         id: pluginSearchTimer
         interval: Config.options.search?.pluginDebounceMs ?? 150
         onTriggered: {
             if (PluginRunner.isActive() && PluginRunner.inputMode === "realtime") {
                 PluginRunner.search(root.query);
             }
         }
     }
    
    property list<var> results: {
         const _pluginActive = PluginRunner.activePlugin !== null;
         const _pluginResults = PluginRunner.pluginResults;
         if (_pluginActive) {
             return root.pluginResultsToSearchResults(_pluginResults);
         }
         
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
            
             const preppedItems = pluginItems.map(item => ({
                 name: Fuzzy.prepare(item.keywords?.length > 0 ? `${item.name} ${item.keywords.join(" ")}` : item.name),
                 item: item
             }));
             
             let matches;
             if (searchQuery.trim() === "") {
                 matches = preppedItems.slice(0, 50).map(p => ({ obj: p }));
             } else {
                 matches = Fuzzy.go(searchQuery, preppedItems, { key: "name", limit: 50 });
             }
             
             const plugin = PluginRunner.plugins.find(p => p.id === pluginId);
             const pluginName = plugin?.manifest?.name ?? pluginId;
             
             return matches.map(match => {
                 const item = match.obj.item;
                 return root.createIndexedItemResultFromData(item, searchQuery);
             }).filter(Boolean);
         }
         
         if (root.query == "") {
             if (!root.historyLoaded || !PluginRunner.pluginsLoaded) return [];
             
             const _actionsLoaded = root.allActions.length;
             const _historyLoaded = searchHistoryData.length;
             
             if (_historyLoaded === 0) return [];
             
             const recentItems = searchHistoryData
                 .slice()
                 .sort((a, b) => (b.lastUsed || 0) - (a.lastUsed || 0))
                 .map(item => {
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
                                 if (item.command && item.command.length > 0) {
                                     Quickshell.execDetached(item.command);
                                 } else if (item.entryPoint && item.workflowId) {
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

         const unifiedResults = root.unifiedFuzzySearch(root.query, 50);
         
         const allResults = [];
         for (const match of unifiedResults) {
             const resultObj = root.createResultFromSearchable(match.item, root.query, match.score);
             if (resultObj) {
                 allResults.push(resultObj);
             }
         }
         
         allResults.sort(root.compareResults);
         
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
         
         const maxResults = Config.options.search?.maxDisplayedResults ?? 16;
         return allResults.slice(0, maxResults).map(item => item.result);
     }

    Component {
        id: resultComp
        LauncherSearchResult {}
    }
}
