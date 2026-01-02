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

    // Delegate to HistoryManager service
    readonly property bool historyLoaded: HistoryManager.historyLoaded
    readonly property var searchHistoryData: HistoryManager.searchHistoryData

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
        if (colonIndex < 1) return null;

        const prefix = query.substring(0, colonIndex).toLowerCase();
        const searchQuery = query.substring(colonIndex + 1);

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

    function findMatchingHint(query) {
        const hints = Config.actionBarHints ?? [];
        for (const hint of hints) {
            if (query === hint.prefix) {
                return hint;
            }
        }
        return null;
    }

    function getConfiguredPrefixes() {
        const hints = Config.actionBarHints ?? [];
        return hints.map(h => h.prefix);
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

    // Delegate history functions to HistoryManager service (with context for smart suggestions)
    function recordSearch(searchType, searchName, searchTerm) {
        const context = ContextTracker.getContext();
        context.launchFromEmpty = root.query === "";
        HistoryManager.recordSearch(searchType, searchName, searchTerm, context);

        // Record app launch for sequence tracking
        if (searchType === "app") {
            ContextTracker.recordLaunch(searchName);
        }
    }

    function recordWorkflowExecution(actionInfo, searchTerm) {
        const context = ContextTracker.getContext();
        context.launchFromEmpty = root.query === "";
        HistoryManager.recordWorkflowExecution(actionInfo, searchTerm, context);
    }

    function recordWindowFocus(appId, appName, windowTitle, iconName, searchTerm) {
        const context = ContextTracker.getContext();
        context.launchFromEmpty = root.query === "";
        const term = searchTerm ?? root.query;
        
        Qt.callLater(() => {
            HistoryManager.recordWindowFocus(appId, appName, windowTitle, iconName, term, context);
            ContextTracker.recordLaunch(appId);
        });
    }

    function removeHistoryItem(historyType, identifier) {
        HistoryManager.removeHistoryItem(historyType, identifier);
    }

    // Delegate frecency functions to FrecencyScorer
    function getFrecencyScore(historyItem) {
        return FrecencyScorer.getFrecencyScore(historyItem);
    }

    function getHistoryBoost(searchType, searchName) {
        const historyItem = searchHistoryData.find(
            h => h.type === searchType && h.name === searchName
        );
        return FrecencyScorer.getFrecencyScore(historyItem);
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
         }
        return success;
    }

    function startPluginWithQuery(pluginId, initialQuery) {
         const success = PluginRunner.startPlugin(pluginId);
         if (success) {
             root.exclusiveMode = "";
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
                 preview: item.preview ?? undefined,
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
                 sourceType: ResultFactory.sourceType.PLUGIN,
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
                 sourceType: ResultFactory.sourceType.PLUGIN,
                 id: `workflow:${plugin.id}`,
                 data: { plugin, isAction: false },
                 isHistoryTerm: false
             });
         }

         const indexedItems = PluginRunner.getAllIndexedItems();
         for (const item of indexedItems) {
             items.push({
                 name: Fuzzy.prepare(item.name),
                 keywords: item.keywords?.length > 0 ? Fuzzy.prepare(item.keywords.join(" ")) : null,
                 sourceType: ResultFactory.sourceType.INDEXED_ITEM,
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
                     sourceType: ResultFactory.sourceType.INDEXED_ITEM,
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
                     sourceType: ResultFactory.sourceType.PLUGIN,
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
                     sourceType: ResultFactory.sourceType.PLUGIN,
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
                 sourceType: ResultFactory.sourceType.PLUGIN_EXECUTION,
                 id: historyItem.key,
                 data: { historyItem },
                 isHistoryTerm: false
             });
             if (historyItem.recentSearchTerms) {
                 for (const term of historyItem.recentSearchTerms) {
                     items.push({
                         name: Fuzzy.prepare(term),
                         sourceType: ResultFactory.sourceType.PLUGIN_EXECUTION,
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
                 sourceType: ResultFactory.sourceType.WEB_SEARCH,
                 id: `webSearch:${historyItem.name}`,
                 data: { query: historyItem.name, historyItem },
                 isHistoryTerm: false
             });
         }

         root.preppedHistorySearchables = items;
     }

    Timer {
        id: historyRebuildTimer
        interval: 250  // Debounce history rebuilds (avoids redundant work on rapid changes)
        onTriggered: root.rebuildHistorySearchables()
    }

    onSearchHistoryDataChanged: {
        historyRebuildTimer.restart();
    }

    property var preppedSearchables: [...preppedStaticSearchables, ...preppedHistorySearchables]

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
             // Don't send queries to plugin when imageBrowser is open - filter locally instead
             if (GlobalStates.imageBrowserOpen) {
                 return;
             }
             if (!root.pluginStarting && !root.pluginClearing) {
                 if (PluginRunner.inputMode === "realtime") {
                     pluginSearchTimer.restart();
                 }
             }
         } else if (root.isInExclusiveMode()) {
         } else if (!root.exclusiveModeStarting) {
             const matchedHint = root.findMatchingHint(root.query);
             if (matchedHint) {
                 if (matchedHint.plugin === "action") {
                     root.enterExclusiveMode("action");
                 } else {
                     root.startPlugin(matchedHint.plugin);
                 }
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

    function exitPlugin() {
        if (!PluginRunner.isActive()) return;
        PluginRunner.closePlugin();
        root.query = "";
    }

    function executePreviewAction(item, actionId) {
        if (!item || !actionId) return;
        
        // Execute the action through the plugin system
        if (item.pluginItemId && PluginRunner.isActive()) {
            PluginRunner.selectItem(item.pluginItemId, actionId);
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

    // Dependencies object for ResultFactory
    readonly property var resultFactoryDependencies: ({
        recordSearch: root.recordSearch,
        recordWorkflowExecution: root.recordWorkflowExecution,
        recordWindowFocus: root.recordWindowFocus,
        startPlugin: root.startPlugin,
        resultComponent: resultComp,
        launcherSearchResult: LauncherSearchResult,
        config: Config,
        stringUtils: StringUtils
    })

    // Helper to get frecency for a searchable item (used by scoreFn)
    function getFrecencyForSearchable(item) {
        const data = item.data;
        if (data.historyItem) {
            return FrecencyScorer.getFrecencyScore(data.historyItem);
        }
        switch (item.sourceType) {
            case ResultFactory.sourceType.PLUGIN:
                if (data.isAction) {
                    return root.getHistoryBoost("action", data.action.action);
                }
                return root.getHistoryBoost("workflow", data.plugin.id);
            case ResultFactory.sourceType.INDEXED_ITEM:
                if (data.item?.appId) {
                    return root.getHistoryBoost("app", data.item.appId);
                }
                return 0;
            default:
                return 0;
        }
    }

    function unifiedFuzzySearch(query, limit) {
        if (!query || query.trim() === "") return [];

        // Use multi-field search: name (primary) + keywords (secondary)
        // scoreFn integrates field weights + frecency into ranking
        const fuzzyResults = Fuzzy.go(query, root.preppedSearchables, {
            keys: ["name", "keywords"],
            limit: limit * 2,
            threshold: 0.25,  // Reject poor matches early
            scoreFn: (result) => {
                const item = result.obj;

                // Multi-field scoring: name matches weighted higher than keywords
                const nameScore = result[0]?.score ?? 0;
                const keywordsScore = result[1]?.score ?? 0;
                const baseScore = nameScore * 1.0 + keywordsScore * 0.3;

                // Get frecency boost
                const frecency = root.getFrecencyForSearchable(item);
                const frecencyBoost = Math.min(frecency * 0.02, 0.3);  // Cap at 0.3

                // History term matches get a significant boost
                const historyBoost = item.isHistoryTerm ? 0.2 : 0;

                // Combined score
                return baseScore + frecencyBoost + historyBoost;
            }
        });

        const seen = new Map();
        for (const match of fuzzyResults) {
            const item = match.obj;
            const key = `${item.sourceType}:${item.id}`;
            const existing = seen.get(key);

            if (!existing || match.score > existing.score) {
                seen.set(key, {
                    score: match.score,  // Use normalized score (includes frecency)
                    item: item,
                    isHistoryTerm: item.isHistoryTerm
                });
            }
        }

        return Array.from(seen.values());
    }

    function createResultFromSearchable(item, query, fuzzyScore) {
        const resultMatchType = item.isHistoryTerm ? FrecencyScorer.matchType.EXACT : FrecencyScorer.matchType.FUZZY;

        // Frecency is already factored into fuzzyScore via scoreFn,
        // but we still need it for display/sorting consistency
        const frecency = root.getFrecencyForSearchable(item);

        const resultObj = ResultFactory.createResultFromSearchable(
            item, query, fuzzyScore,
            root.resultFactoryDependencies,
            frecency, resultMatchType
        );

        // Add composite score for efficient sorting
        if (resultObj) {
            resultObj.compositeScore = FrecencyScorer.getCompositeScore(
                resultMatchType, fuzzyScore, frecency
            );
        }

        return resultObj;
    }

    // Create suggestion results from SmartSuggestions
    function createSuggestionResults(allIndexed) {
        const suggestions = SmartSuggestions.getSuggestions();

        return suggestions.map(suggestion => {
            const historyItem = suggestion.item;
            const appItem = allIndexed.find(idx => idx.appId === historyItem.name);
            if (!appItem) return null;

            const appId = appItem.appId;
            const reason = SmartSuggestions.getPrimaryReason(suggestion);

            return resultComp.createObject(null, {
                type: "Suggested",
                id: appId,
                name: appItem.name,
                comment: reason,
                iconName: appItem.icon,
                iconType: LauncherSearchResult.IconType.System,
                verb: "Open",
                isSuggestion: true,
                suggestionReason: reason,
                execute: ((capturedAppItem, capturedAppId) => () => {
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
        }).filter(Boolean);
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

             return matches.map(match => {
                 const item = match.obj.item;
                 const resultObj = ResultFactory.createIndexedItemResultFromData(
                     { item }, searchQuery, 0, 0, FrecencyScorer.matchType.FUZZY,
                     root.resultFactoryDependencies
                 );
                 return resultObj?.result;
             }).filter(Boolean);
         }

         if (root.query == "") {
             if (!root.historyLoaded || !PluginRunner.pluginsLoaded) return [];

             const _actionsLoaded = root.allActions.length;
             const _historyLoaded = searchHistoryData.length;

             const allIndexed = PluginRunner.getAllIndexedItems();

             // Get smart suggestions first
             const suggestions = root.createSuggestionResults(allIndexed);
             const suggestionAppIds = new Set(suggestions.map(s => s.id));

             if (_historyLoaded === 0) return suggestions;
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
                                const windows = WindowManager.getWindowsForApp(item.appId);
                                const targetWindow = windows.find(w => w.title === item.windowTitle);

                                if (targetWindow) {
                                    root.recordWindowFocus(item.appId, item.appName, item.windowTitle, item.iconName);
                                    WindowManager.focusWindow(targetWindow);
                                    GlobalStates.launcherOpen = false;
                                } else if (windows.length === 1) {
                                    root.recordWindowFocus(item.appId, item.appName, windows[0].title, item.iconName);
                                    WindowManager.focusWindow(windows[0]);
                                    GlobalStates.launcherOpen = false;
                                } else if (windows.length > 1) {
                                    GlobalStates.openWindowPicker(item.appId, windows);
                                } else {
                                    const entry = DesktopEntries.byId(item.appId);
                                    if (entry) entry.execute();
                                }
                            }
                        });
                    }
                    return null;
                })
                .filter(Boolean)
                // Filter out items that are already in suggestions to avoid duplicates
                .filter(item => !suggestionAppIds.has(item.id))
                .slice(0, Config.options.search?.maxRecentItems ?? 100);

             // Prepend suggestions to recent items
             return [...suggestions, ...recentItems];
         }

         const unifiedResults = root.unifiedFuzzySearch(root.query, 50);

         const allResults = [];
         for (const match of unifiedResults) {
             const resultObj = root.createResultFromSearchable(match.item, root.query, match.score);
             if (resultObj) {
                 allResults.push(resultObj);
             }
         }

         // Use composite score for faster sorting (single numeric comparison)
         allResults.sort(FrecencyScorer.compareByCompositeScore);

         const webSearchQuery = StringUtils.cleanPrefix(root.query, Config.options.search.prefix.webSearch);
         allResults.push({
             matchType: FrecencyScorer.matchType.NONE,
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
