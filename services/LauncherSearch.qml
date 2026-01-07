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
        if (!query || query.length < 2)
            return null;

        const colonIndex = query.indexOf(":");
        if (colonIndex < 1)
            return null;

        const prefix = query.substring(0, colonIndex).toLowerCase();
        const searchQuery = query.substring(colonIndex + 1);

        const indexedPlugins = PluginRunner.getIndexedPluginIds();
        if (indexedPlugins.includes(prefix)) {
            return {
                pluginId: prefix,
                searchQuery: searchQuery
            };
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
            PluginRunner.recordExecution("apps", appId, root.query, root.query === "");
            entry.execute();
        }
    }

    function ensurePrefix(prefix) {
        if ([Config.options.search.prefix.plugins, Config.options.search.prefix.app, Config.options.search.prefix.emojis, Config.options.search.prefix.math, Config.options.search.prefix.shellCommand, Config.options.search.prefix.webSearch].some(i => root.query.startsWith(i))) {
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
            if (actionInfo.workflowId && actionInfo.itemId) {
                PluginRunner.recordExecution(actionInfo.workflowId, actionInfo.itemId);
            }
        }
        function onClearInputRequested() {
            root.pluginClearing = true;
            root.query = "";
            root.pluginClearing = false;
        }
        function onPluginClosed() {
            root.clearPluginResultCache();
        }
    }

    // Use a non-reactive cache to avoid binding loops
    // This is a plain JS object, not a QML property
    readonly property var _pluginResultCacheHolder: ({
            cache: {}
        })

    // Converted plugin results - updated imperatively to avoid binding loops
    property list<var> _convertedPluginResults: []

    // Update converted results when plugin results change
    Connections {
        target: PluginRunner
        function onPluginResultsChanged() {
            if (PluginRunner.activePlugin !== null) {
                root._convertedPluginResults = root.pluginResultsToSearchResults(PluginRunner.pluginResults);
            }
        }
        function onActivePluginChanged() {
            if (PluginRunner.activePlugin === null) {
                root._convertedPluginResults = [];
                root.clearPluginResultCache();
            }
        }
    }

    function clearPluginResultCache() {
        const cache = root._pluginResultCacheHolder.cache;
        for (const id of Object.keys(cache)) {
            const cached = cache[id];
            if (cached?.result) {
                cached.result.destroy();
            }
            if (cached?.actions) {
                for (const action of cached.actions) {
                    action.destroy();
                }
            }
        }
        root._pluginResultCacheHolder.cache = {};
    }

    function pluginResultsToSearchResults(pluginResults: var): var {
        const existingCache = root._pluginResultCacheHolder.cache;
        const newCache = {};
        const pluginId = PluginRunner.activePlugin?.id ?? "";
        const pluginName = PluginRunner.activePlugin?.manifest?.name ?? "Plugin";
        const pluginIcon = PluginRunner.activePlugin?.manifest?.icon ?? "extension";

        if (!pluginResults || pluginResults.length === 0) {
            if (!PluginRunner.pluginBusy) {
                return [resultComp.createObject(null, {
                        id: "__empty__",
                        name: "No items",
                        comment: "No results found",
                        type: pluginName,
                        iconName: pluginIcon,
                        iconType: LauncherSearchResult.IconType.Material,
                        verb: "",
                        execute: () => {}
                    })];
            }
            return [];
        }

        const results = pluginResults.map(item => {
            const itemId = item.id;
            const itemKey = item.key ?? itemId;
            const cached = existingCache[itemKey];

            const iconName = item.icon ?? PluginRunner.activePlugin?.manifest?.icon ?? 'extension';
            let isSystemIcon;
            if (item.iconType === "system") {
                isSystemIcon = true;
            } else if (item.iconType === "material") {
                isSystemIcon = false;
            } else {
                isSystemIcon = iconName.includes('.') || iconName.includes('-');
            }

            if (cached?.result) {
                const result = cached.result;
                const idChanged = result.id !== itemId;
                result.id = itemId;
                result.pluginItemId = itemId;
                result.name = item.name;
                result.comment = item.description ?? "";
                result.verb = item.verb ?? "Select";
                result.iconName = iconName;
                result.iconType = isSystemIcon ? LauncherSearchResult.IconType.System : LauncherSearchResult.IconType.Material;
                result.thumbnail = item.thumbnail ?? "";
                result.preview = item.preview ?? undefined;
                result.value = item.value ?? 0;
                result.min = item.min ?? 0;
                result.max = item.max ?? 100;
                result.step = item.step ?? 1;
                result.displayValue = item.displayValue ?? "";
                result.badges = item.badges ?? [];
                result.chips = item.chips ?? [];
                result.graph = item.graph ?? null;
                result.gauge = item.gauge ?? null;
                result.progress = item.progress ?? null;
                result._pluginId = pluginId;
                result._pluginName = pluginName;

                if (idChanged) {
                    result.execute = ((capturedItemId, capturedPluginId) => () => {
                                PluginRunner.selectItem(capturedItemId, "");
                            })(itemId, pluginId);
                }

                const newActions = item.actions ?? [];
                const cachedActionIds = (cached.actions ?? []).map(a => a.name).join(',');
                const newActionIds = newActions.map(a => a.name).join(',');

                if (cachedActionIds !== newActionIds || idChanged) {
                    for (const action of (cached.actions ?? [])) {
                        action.destroy();
                    }
                    const itemActions = newActions.map(action => {
                        const actionId = action.id;
                        const actionIconType = action.iconType === "system" ? LauncherSearchResult.IconType.System : LauncherSearchResult.IconType.Material;
                        return resultComp.createObject(null, {
                            name: action.name,
                            iconName: action.icon ?? 'play_arrow',
                            iconType: actionIconType,
                            execute: () => {
                                PluginRunner.selectItem(itemId, actionId);
                            }
                        });
                    });
                    result.actions = itemActions;
                    cached.actions = itemActions;
                }
                result.pluginActions = newActions;

                newCache[itemKey] = cached;
                return result;
            }

            const itemActions = (item.actions ?? []).map(action => {
                const actionId = action.id;
                const actionIconType = action.iconType === "system" ? LauncherSearchResult.IconType.System : LauncherSearchResult.IconType.Material;
                return resultComp.createObject(null, {
                    name: action.name,
                    iconName: action.icon ?? 'play_arrow',
                    iconType: actionIconType,
                    execute: () => {
                        PluginRunner.selectItem(itemId, actionId);
                    }
                });
            });

            const result = resultComp.createObject(null, {
                id: itemId,
                key: itemKey,
                name: item.name,
                comment: item.description ?? "",
                verb: item.verb ?? "Select",
                type: (item.type === "slider" || item.type === "switch") ? item.type : pluginName,
                resultType: (item.type === "slider" || item.type === "switch") ? item.type : LauncherSearchResult.ResultType.PluginResult,
                iconName: iconName,
                iconType: isSystemIcon ? LauncherSearchResult.IconType.System : LauncherSearchResult.IconType.Material,
                pluginId: pluginId,
                pluginItemId: itemId,
                pluginActions: item.actions ?? [],
                thumbnail: item.thumbnail ?? "",
                preview: item.preview ?? undefined,
                actions: itemActions,
                value: item.value ?? 0,
                min: item.min ?? 0,
                max: item.max ?? 100,
                step: item.step ?? 1,
                displayValue: item.displayValue ?? "",
                badges: item.badges ?? [],
                chips: item.chips ?? [],
                graph: item.graph ?? null,
                gauge: item.gauge ?? null,
                progress: item.progress ?? null,
                _pluginId: pluginId,
                _pluginName: pluginName,
                execute: (capturedItemId => () => {
                            PluginRunner.selectItem(capturedItemId, "");
                        })(itemId)
            });

            newCache[itemKey] = {
                result,
                actions: itemActions
            };
            return result;
        });

        // Defer cache cleanup and update to avoid binding loop
        // (modifying state during binding evaluation causes loops)
        Qt.callLater(() => {
            for (const id of Object.keys(existingCache)) {
                if (!newCache[id]) {
                    const cached = existingCache[id];
                    if (cached?.result) {
                        cached.result.destroy();
                    }
                    if (cached?.actions) {
                        for (const action of cached.actions) {
                            action.destroy();
                        }
                    }
                }
            }
            root._pluginResultCacheHolder.cache = newCache;
        });

        return results;
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

        // Add plugins as searchables (frecency via __plugin__ entries)
        const plugins = root.preppedPlugins ?? [];
        for (const preppedPlugin of plugins) {
            const plugin = preppedPlugin.plugin;
            items.push({
                name: preppedPlugin.name,
                sourceType: "plugin",
                id: plugin.id,
                pluginId: plugin.id,
                data: {
                    plugin
                },
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
                pluginId: item._pluginId ?? "",
                data: {
                    item
                },
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
        // Note: pluginStatusChanged is intentionally not handled here.
        // Status is read dynamically via getPluginStatus() when results are created,
        // so it will be picked up on the next search/keystroke without needing a full rebuild.
    }

    property var preppedHistorySearchables: []

    // Build history searchables from index items that have frecency data
    function rebuildHistorySearchables() {
        const items = [];
        const pluginMap = new Map(PluginRunner.plugins.map(p => [p.id, p]));

        // Get all items with frecency from index
        const itemsWithFrecency = PluginRunner.getItemsWithFrecency();

        for (const {
            pluginId,
            item
        } of itemsWithFrecency) {
            const plugin = pluginMap.get(pluginId);
            const pluginName = plugin?.manifest?.name ?? pluginId;

            // Enrich item with plugin metadata
            const enrichedItem = Object.assign({}, item, {
                _pluginId: pluginId,
                _pluginName: pluginName
            });

            // Add searchable for recent search terms (history terms)
            const recentTerms = item._recentSearchTerms ?? [];
            for (const term of recentTerms) {
                items.push({
                    name: Fuzzy.prepare(term),
                    sourceType: ResultFactory.sourceType.INDEXED_ITEM,
                    id: item.id,
                    pluginId: pluginId,
                    data: {
                        item: enrichedItem
                    },
                    isHistoryTerm: true,
                    matchedTerm: term
                });
            }
        }

        root.preppedHistorySearchables = items;
    }

    Timer {
        id: historyRebuildTimer
        interval: 250
        onTriggered: root.rebuildHistorySearchables()
    }

    // Rebuild when index changes (frecency updates, new items, etc.)
    Connections {
        target: PluginRunner
        function onPluginIndexChanged(pluginId) {
            historyRebuildTimer.restart();
        }
    }

    property var preppedSearchables: preppedStaticSearchables.concat(preppedHistorySearchables)

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
        } else if (root.isInExclusiveMode()) {} else if (!root.exclusiveModeStarting) {
            const matchedHint = root.findMatchingHint(root.query);
            if (matchedHint) {
                root.startPlugin(matchedHint.plugin);
            } else if (root.query.length >= 2) {
                matchPatternCheckTimer.restart();
            }
        }
    }

    Timer {
        id: matchPatternCheckTimer
        interval: 50
        onTriggered: {
            if (PluginRunner.isActive() || root.isInExclusiveMode())
                return;

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
        if (!PluginRunner.isActive())
            return;
        PluginRunner.closePlugin();
        root.query = "";
    }

    function executePreviewAction(item, actionId) {
        if (!item || !actionId)
            return;

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
            startPlugin: root.startPlugin,
            resultComponent: resultComp,
            launcherSearchResult: LauncherSearchResult,
            config: Config,
            stringUtils: StringUtils
        })

    // Helper to get frecency for a searchable item (uses index-based frecency)
    function getFrecencyForSearchable(item) {
        const data = item.data;

        // Indexed items have frecency data via PluginRunner
        if (data.item?._pluginId && data.item?.id) {
            return PluginRunner.getItemFrecency(data.item._pluginId, data.item.id);
        }

        // Plugins use __plugin__ frecency entry
        if (data.plugin?.id) {
            return PluginRunner.getItemFrecency(data.plugin.id, "__plugin__");
        }

        return 0;
    }

    function unifiedFuzzySearch(query, limit) {
        if (!query || query.trim() === "")
            return [];

        // Capture query string for use in scoreFn closure
        const searchQuery = query.toLowerCase();

        // Use multi-field search: name (primary) + keywords (secondary)
        // scoreFn integrates field weights + frecency into ranking
        const fuzzyResults = Fuzzy.go(query, root.preppedSearchables, {
            keys: ["name", "keywords"],
            limit: limit * 2,
            threshold: 0.25  // Reject poor matches early
            ,
            scoreFn: result => {
                const item = result.obj;

                // Multi-field scoring: name matches weighted higher than keywords
                const nameScore = result[0]?.score ?? 0;
                const keywordsScore = result[1]?.score ?? 0;
                const baseScore = nameScore * 1.0 + keywordsScore * 0.3;

                // Exact name match bonus: if query exactly equals the name, boost significantly
                // This ensures "fire" ranks above "firefighter" when searching "fire"
                // Note: item.name is a fuzzysort prepared object, get original name from data
                const originalName = item.data?.item?.name ?? item.data?.plugin?.name ?? "";
                const nameLower = originalName.toLowerCase();
                const exactMatchBonus = (searchQuery === nameLower) ? 0.5 : 0;

                // Get frecency boost
                const frecency = root.getFrecencyForSearchable(item);
                const frecencyBoost = Math.min(frecency * 0.02, 0.3);  // Cap at 0.3

                // History term matches get a significant boost
                const historyBoost = item.isHistoryTerm ? 0.2 : 0;

                // Combined score
                return baseScore + exactMatchBonus + frecencyBoost + historyBoost;
            }
        });

        const seen = new Map();
        for (const match of fuzzyResults) {
            const item = match.obj;
            const key = `${item.sourceType}:${item.id}`;
            const existing = seen.get(key);

            if (!existing || match.score > existing.score) {
                seen.set(key, {
                    score: match.score  // Use normalized score (includes frecency)
                    ,
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

        const resultObj = ResultFactory.createResultFromSearchable(item, query, fuzzyScore, root.resultFactoryDependencies, frecency, resultMatchType);

        // Add composite score and pluginId for sorting and diversity
        if (resultObj) {
            resultObj.compositeScore = FrecencyScorer.getCompositeScore(resultMatchType, fuzzyScore, frecency);
            resultObj._pluginId = item.pluginId ?? item.data?.item?._pluginId ?? item.data?.plugin?.id ?? "";
        }

        return resultObj;
    }

    // Create suggestion results from SmartSuggestions
    function createSuggestionResults(allIndexed, appIdMap) {
        const suggestions = SmartSuggestions.getSuggestions();

        return suggestions.map(suggestion => {
            const historyItem = suggestion.item;
            const appItem = appIdMap.get(historyItem.name);
            if (!appItem)
                return null;

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
                execute: ((capturedAppItem, capturedAppId, capturedItemId) => () => {
                            PluginRunner.recordExecution("apps", capturedItemId, "", true);
                            const currentWindows = WindowManager.getWindowsForApp(capturedAppId);
                            if (currentWindows.length === 0) {
                                const entryPoint = capturedAppItem.entryPoint ?? {
                                    step: "action",
                                    selected: {
                                        id: capturedItemId
                                    }
                                };
                                PluginRunner.executeAction("apps", entryPoint, false);
                            } else if (currentWindows.length === 1) {
                                WindowManager.focusWindow(currentWindows[0]);
                                GlobalStates.launcherOpen = false;
                            } else {
                                GlobalStates.openWindowPicker(capturedAppId, currentWindows, capturedItemId, true);
                            }
                        })(appItem, appId, appItem.id)
            });
        }).filter(Boolean);
    }

    property list<var> results: {
        const _pluginActive = PluginRunner.activePlugin !== null;
        if (_pluginActive) {
            // Use pre-converted results (updated via Connections to avoid binding loop)
            return root._convertedPluginResults;
        }

        const isolationMatch = root.parseIndexIsolationPrefix(root.query);
        if (isolationMatch) {
            const {
                pluginId,
                searchQuery
            } = isolationMatch;
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
                matches = preppedItems.slice(0, 50).map(p => ({
                            obj: p
                        }));
            } else {
                matches = Fuzzy.go(searchQuery, preppedItems, {
                    key: "name",
                    limit: 50
                });
            }

            return matches.map(match => {
                const item = match.obj.item;
                const resultObj = ResultFactory.createIndexedItemResultFromData({
                    item
                }, searchQuery, 0, 0, FrecencyScorer.matchType.FUZZY, root.resultFactoryDependencies);
                return resultObj?.result;
            }).filter(Boolean);
        }

        if (root.query == "") {
            if (!PluginRunner.pluginsLoaded || !PluginRunner.indexCacheLoaded)
                return [];

            // Note: Don't depend on indexVersion here - it causes full list rebuild
            // Live updates happen via SearchItem's liveData property
            const allIndexed = PluginRunner.getAllIndexedItems();
            const pluginMap = new Map(PluginRunner.plugins.map(p => [p.id, p]));
            const appIdMap = new Map(allIndexed.filter(i => i.appId).map(i => [i.appId, i]));

            // Get smart suggestions first
            const suggestions = root.createSuggestionResults(allIndexed, appIdMap);
            const suggestionIds = new Set(suggestions.map(s => s.id || s.name));

            // Get recent items from index (frecency-based)
            const recentItems = PluginRunner.getItemsWithFrecency().filter(({
                    item
                }) => !suggestionIds.has(item.id) && !suggestionIds.has(item.appId)).slice(0, Config.options.search?.maxRecentItems ?? 20).map(({
                    pluginId,
                    item
                }) => {
                const plugin = pluginMap.get(pluginId);
                const pluginName = plugin?.manifest?.name ?? pluginId;

                // Handle plugin-level frecency entries
                const isPluginEntry = item._isPluginEntry === true;
                if (isPluginEntry) {
                    return resultComp.createObject(null, {
                        type: "Recent",
                        id: pluginId,
                        name: pluginName,
                        iconName: plugin?.manifest?.icon ?? 'extension',
                        iconType: LauncherSearchResult.IconType.Material,
                        verb: "Open",
                        comment: plugin?.manifest?.description ?? "",
                        resultType: LauncherSearchResult.ResultType.PluginEntry,
                        pluginId: pluginId,
                        execute: (capturedPluginId => () => {
                                    PluginRunner.recordExecution(capturedPluginId, "__plugin__");
                                    root.startPlugin(capturedPluginId);
                                })(pluginId)
                    });
                }

                let iconType = LauncherSearchResult.IconType.Material;
                if (item.iconType === "system") {
                    iconType = LauncherSearchResult.IconType.System;
                } else if (item.iconType === "text") {
                    iconType = LauncherSearchResult.IconType.Text;
                }

                const isAppItem = item.appId !== undefined;
                const windows = isAppItem ? WindowManager.getWindowsForApp(item.appId) : [];

                const props = {
                    type: "Recent",
                    id: item.appId ?? item.id,
                    name: item.name,
                    iconName: item.icon ?? 'extension',
                    iconType: iconType,
                    verb: isAppItem ? (windows.length > 0 ? "Focus" : "Open") : (item.verb ?? "Run"),
                    _pluginId: pluginId,
                    _pluginName: pluginName,
                    comment: isAppItem ? "" : pluginName,
                    windowCount: windows.length,
                    windows: windows
                };

                // Add slider properties if it's a slider
                if (item.type === "slider") {
                    props.resultType = "slider";
                    props.value = item.value;
                    props.min = item.min;
                    props.max = item.max;
                    props.step = item.step;
                    props.gauge = item.gauge;
                }
                
                // Add switch properties if it's a switch
                if (item.type === "switch") {
                    props.resultType = "switch";
                    props.value = item.value;
                }

                // Add graph/gauge/progress properties
                if (item.graph)
                    props.graph = item.graph;
                if (item.gauge)
                    props.gauge = item.gauge;
                if (item.progress)
                    props.progress = item.progress;

                if (item.badges?.length > 0)
                    props.badges = item.badges;
                if (item.thumbnail)
                    props.thumbnail = item.thumbnail;
                if (item.description)
                    props.comment = item.description;

                // Build actions from indexed item
                const itemActions = (item.actions ?? []).map(action => {
                    const actionIconType = action.iconType === "system" ? LauncherSearchResult.IconType.System : LauncherSearchResult.IconType.Material;
                    return resultComp.createObject(null, {
                        name: action.name,
                        iconName: action.icon ?? 'play_arrow',
                        iconType: actionIconType,
                        execute: ((capturedAction, capturedItem, capturedPluginId) => () => {
                                    const entryPoint = capturedAction.entryPoint ?? {
                                        step: "action",
                                        selected: {
                                            id: capturedItem.id
                                        },
                                        action: capturedAction.id
                                    };
                                    PluginRunner.executeAction(capturedPluginId, entryPoint, capturedAction.keepOpen ?? false);
                                })(action, item, pluginId)
                    });
                });
                if (itemActions.length > 0) {
                    props.actions = itemActions;
                }

                props.execute = ((capturedItem, capturedPluginId, capturedIsApp, capturedKeepOpen) => () => {
                            // Record execution (launchFromEmpty=true since this is from Recent/empty query)
                            PluginRunner.recordExecution(capturedPluginId, capturedItem.id, "", true);

                            if (capturedIsApp) {
                                const currentWindows = WindowManager.getWindowsForApp(capturedItem.appId);
                                if (currentWindows.length === 0) {
                                    const entryPoint = capturedItem.entryPoint ?? {
                                        step: "action",
                                        selected: {
                                            id: capturedItem.id
                                        }
                                    };
                                    PluginRunner.executeAction(capturedPluginId, entryPoint, false);
                                } else if (currentWindows.length === 1) {
                                    WindowManager.focusWindow(currentWindows[0]);
                                    GlobalStates.launcherOpen = false;
                                } else {
                                    GlobalStates.openWindowPicker(capturedItem.appId, currentWindows, capturedItem.id, true);
                                }
                            } else {
                                const entryPoint = capturedItem.entryPoint ?? {
                                    step: "action",
                                    selected: {
                                        id: capturedItem.id
                                    }
                                };
                                PluginRunner.executeAction(capturedPluginId, entryPoint, capturedKeepOpen);
                            }
                        })(item, pluginId, isAppItem, item.keepOpen ?? false);

                return resultComp.createObject(null, props);
            }).filter(Boolean);

            return suggestions.concat(recentItems);
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

        // Apply diversity to prevent single plugin from dominating results
        const maxPerPlugin = Config.options.search?.maxResultsPerPlugin ?? 0;
        const diversityOptions = {
            decayFactor: Config.options.search?.diversityDecay ?? 0.7,
            maxPerSource: maxPerPlugin > 0 ? maxPerPlugin : Infinity
        };
        const diverseResults = FrecencyScorer.applyDiversity(allResults, diversityOptions);

        const webSearchQuery = StringUtils.cleanPrefix(root.query, Config.options.search.prefix.webSearch);
        diverseResults.push({
            matchType: FrecencyScorer.matchType.NONE,
            fuzzyScore: 0,
            frecency: 0,
            _pluginId: "__websearch__",
            result: resultComp.createObject(null, {
                name: webSearchQuery,
                verb: "Search",
                type: "Web search",
                iconName: 'travel_explore',
                iconType: LauncherSearchResult.IconType.Material,
                execute: (capturedQuery => () => {
                            let url = Config.options.search.engineBaseUrl + capturedQuery;
                            for (let site of Config.options.search.excludedSites) {
                                url += ` -site:${site}`;
                            }
                            Qt.openUrlExternally(url);
                        })(webSearchQuery)
            })
        });

        const maxResults = Config.options.search?.maxDisplayedResults ?? 16;
        return diverseResults.slice(0, maxResults).map(item => item.result);
    }

    Component {
        id: resultComp
        LauncherSearchResult {}
    }
}
