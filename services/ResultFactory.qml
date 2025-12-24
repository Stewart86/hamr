pragma Singleton

import qs
import qs.modules.common
import qs.modules.common.models
import QtQuick
import Quickshell

Singleton {
    id: root

    readonly property var sourceType: ({
        PLUGIN: "plugin",
        PLUGIN_EXECUTION: "pluginExecution",
        WEB_SEARCH: "webSearch",
        INDEXED_ITEM: "indexedItem"
    })

    readonly property var matchType: ({
        EXACT: 3,
        PREFIX: 2,
        FUZZY: 1,
        NONE: 0
    })

    /**
     * Creates a result object from a searchable item.
     * Routes to the appropriate specific creator based on item.sourceType.
     *
     * @param {Object} item - The searchable item with sourceType, id, data, etc.
     * @param {string} query - The search query string
     * @param {number} fuzzyScore - The fuzzy match score
     * @param {Object} dependencies - Callback functions and services:
     *   - recordSearch(type, name, query)
     *   - recordWorkflowExecution(actionInfo, searchTerm)
     *   - recordWindowFocus(appId, appName, windowTitle, iconName, searchTerm)
     *   - startPlugin(pluginId)
     *   - resultComponent - The LauncherSearchResult component to instantiate
     *   - matchTypeEnum - The match type enum (EXACT, PREFIX, FUZZY, NONE)
     *   - launcherSearchResult - LauncherSearchResult for IconType access
     *   - config - Config object for options
     *   - stringUtils - StringUtils for cleaning prefixes
     * @param {number} frecency - The frecency score for this item
     * @param {number} resultMatchType - The match type (EXACT, PREFIX, FUZZY, NONE)
     * @returns {Object} Result object with matchType, fuzzyScore, frecency, and result properties
     */
    function createResultFromSearchable(item, query, fuzzyScore, dependencies, frecency, resultMatchType) {
        const st = root.sourceType;

        switch (item.sourceType) {
            case st.PLUGIN:
                return createPluginResultFromData(
                    item.data,
                    item.id,
                    query,
                    fuzzyScore,
                    frecency,
                    resultMatchType,
                    dependencies
                );
            case st.PLUGIN_EXECUTION:
                return createPluginExecResultFromData(
                    item.data,
                    query,
                    fuzzyScore,
                    frecency,
                    resultMatchType,
                    dependencies
                );
            case st.WEB_SEARCH:
                return createWebSearchHistoryResultFromData(
                    item.data,
                    query,
                    fuzzyScore,
                    frecency,
                    resultMatchType,
                    dependencies
                );
            case st.INDEXED_ITEM:
                return createIndexedItemResultFromData(
                    item.data,
                    query,
                    fuzzyScore,
                    frecency,
                    resultMatchType,
                    dependencies
                );
            default:
                return null;
        }
    }

    /**
     * Creates a result from a plugin or action.
     *
     * @param {Object} data - Contains either { action, isAction: true } or { plugin, isAction: false }
     * @param {string} itemId - The item ID (action:name or workflow:id)
     * @param {string} query - The search query
     * @param {number} fuzzyScore - The fuzzy match score
     * @param {number} frecency - The frecency score
     * @param {number} resultMatchType - The match type enum value
     * @param {Object} dependencies - Dependency callback functions
     * @returns {Object} Result object with matchType, fuzzyScore, frecency, result
     */
    function createPluginResultFromData(
        data,
        itemId,
        query,
        fuzzyScore,
        frecency,
        resultMatchType,
        dependencies
    ) {
        if (data.isAction) {
            const action = data.action;
            const actionArgs = query.includes(" ") ? query.split(" ").slice(1).join(" ") : "";
            const hasArgs = actionArgs.length > 0;

            return {
                matchType: resultMatchType,
                fuzzyScore: fuzzyScore,
                frecency: frecency,
                result: dependencies.resultComponent.createObject(null, {
                    name: action.action + (hasArgs ? " " + actionArgs : ""),
                    verb: "Run",
                    type: "Action",
                    iconName: 'settings_suggest',
                    iconType: dependencies.launcherSearchResult.IconType.Material,
                    acceptsArguments: !hasArgs,
                    completionText: !hasArgs ? action.action + " " : "",
                    execute: ((capturedAction, capturedArgs, capturedQuery) => () => {
                        dependencies.recordSearch("action", capturedAction.action, capturedQuery);
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
                result: dependencies.resultComponent.createObject(null, {
                    name: manifest?.name ?? plugin.id,
                    comment: manifest?.description ?? "",
                    verb: "Start",
                    type: "Plugin",
                    iconName: manifest?.icon ?? 'extension',
                    iconType: dependencies.launcherSearchResult.IconType.Material,
                    resultType: dependencies.launcherSearchResult.ResultType.PluginEntry,
                    pluginId: plugin.id,
                    acceptsArguments: true,
                    completionText: plugin.id + " ",
                    execute: ((capturedPlugin, capturedQuery) => () => {
                        dependencies.recordSearch("workflow", capturedPlugin.id, capturedQuery);
                        dependencies.startPlugin(capturedPlugin.id);
                    })(plugin, query)
                })
            };
        }
    }

    /**
     * Creates a result from a workflow execution history item.
     *
     * @param {Object} data - Contains { historyItem }
     * @param {string} query - The search query
     * @param {number} fuzzyScore - The fuzzy match score
     * @param {number} frecency - The frecency score
     * @param {number} resultMatchType - The match type enum value
     * @param {Object} dependencies - Dependency callback functions
     * @returns {Object} Result object with matchType, fuzzyScore, frecency, result
     */
    function createPluginExecResultFromData(
        data,
        query,
        fuzzyScore,
        frecency,
        resultMatchType,
        dependencies
    ) {
        const item = data.historyItem;
        const iconType = item.iconType === "system"
            ? dependencies.launcherSearchResult.IconType.System
            : dependencies.launcherSearchResult.IconType.Material;

        const itemKeepOpen = item.keepOpen === true;
        
        return {
            matchType: resultMatchType,
            fuzzyScore: fuzzyScore,
            frecency: frecency,
            result: dependencies.resultComponent.createObject(null, {
                type: item.workflowName || "Recent",
                name: item.name,
                iconName: item.icon || 'play_arrow',
                iconType: iconType,
                thumbnail: item.thumbnail || "",
                verb: "Run",
                keepOpen: itemKeepOpen,
                execute: ((capturedItem, capturedQuery, capturedKeepOpen) => () => {
                    dependencies.recordWorkflowExecution({
                        name: capturedItem.name,
                        command: capturedItem.command,
                        entryPoint: capturedItem.entryPoint,
                        icon: capturedItem.icon,
                        iconType: capturedItem.iconType,
                        thumbnail: capturedItem.thumbnail,
                        workflowId: capturedItem.workflowId,
                        workflowName: capturedItem.workflowName,
                        keepOpen: capturedKeepOpen
                    }, capturedQuery);
                    if (capturedItem.command && capturedItem.command.length > 0) {
                        Quickshell.execDetached(capturedItem.command);
                    } else if (capturedItem.entryPoint && capturedItem.workflowId) {
                        if (capturedKeepOpen) {
                            PluginRunner.executeEntryPoint(capturedItem.workflowId, capturedItem.entryPoint);
                        } else {
                            PluginRunner.replayAction(capturedItem.workflowId, capturedItem.entryPoint);
                        }
                    }
                })(item, query, itemKeepOpen)
            })
        };
    }

    /**
     * Creates a result from a web search history item.
     *
     * @param {Object} data - Contains { query, historyItem }
     * @param {string} query - The search query
     * @param {number} fuzzyScore - The fuzzy match score
     * @param {number} frecency - The frecency score
     * @param {number} resultMatchType - The match type enum value
     * @param {Object} dependencies - Dependency callback functions
     * @returns {Object} Result object with matchType, fuzzyScore, frecency, result
     */
    function createWebSearchHistoryResultFromData(
        data,
        query,
        fuzzyScore,
        frecency,
        resultMatchType,
        dependencies
    ) {
        const searchQuery = data.query;

        return {
            matchType: resultMatchType,
            fuzzyScore: fuzzyScore,
            frecency: frecency,
            result: dependencies.resultComponent.createObject(null, {
                name: searchQuery,
                verb: "Search",
                type: "Web search - recent",
                iconName: 'travel_explore',
                iconType: dependencies.launcherSearchResult.IconType.Material,
                execute: ((capturedQuery) => () => {
                    dependencies.recordSearch("webSearch", capturedQuery, capturedQuery);
                    let url = dependencies.config.options.search.engineBaseUrl + capturedQuery;
                    for (let site of dependencies.config.options.search.excludedSites) {
                        url += ` -site:${site}`;
                    }
                    Qt.openUrlExternally(url);
                })(searchQuery)
            })
        };
    }

    /**
     * Creates a result from an indexed item (app, emoji, clipboard, etc.).
     *
     * @param {Object} data - Contains { item }
     * @param {string} query - The search query
     * @param {number} fuzzyScore - The fuzzy match score
     * @param {number} frecency - The frecency score
     * @param {number} resultMatchType - The match type enum value
     * @param {Object} dependencies - Dependency callback functions
     * @returns {Object} Result object with matchType, fuzzyScore, frecency, result
     */
    function createIndexedItemResultFromData(
        data,
        query,
        fuzzyScore,
        frecency,
        resultMatchType,
        dependencies
    ) {
        const item = data.item;

        let iconType;
        if (item.iconType === "text") {
            iconType = dependencies.launcherSearchResult.IconType.Text;
        } else if (item.iconType === "system") {
            iconType = dependencies.launcherSearchResult.IconType.System;
        } else {
            iconType = dependencies.launcherSearchResult.IconType.Material;
        }

        const isAppItem = item.appId !== undefined;
        const appId = item.appId ?? "";

        const windows = isAppItem ? WindowManager.getWindowsForApp(appId) : [];
        const windowCount = windows.length;

        const itemActions = (item.actions ?? []).map(action => {
            const actionIconType = action.iconType === "system"
                ? dependencies.launcherSearchResult.IconType.System
                : dependencies.launcherSearchResult.IconType.Material;
            return dependencies.resultComponent.createObject(null, {
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
                        dependencies.recordSearch("app", capturedItem.appId, query);
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

        const itemKeepOpen = item.keepOpen === true;

        const resultObj = dependencies.resultComponent.createObject(null, {
            type: isAppItem ? "App" : (item._pluginName ?? "Plugin"),
            id: appId,  // For window tracking
            name: item.name,
            comment: item.description ?? "",
            iconName: item.icon ?? 'extension',
            iconType: iconType,
            thumbnail: item.thumbnail ?? "",
            verb: verb,
            keepOpen: itemKeepOpen,
            windowCount: windowCount,
            windows: windows,
            actions: itemActions,
            execute: ((capturedItem, capturedQuery, capturedAppId, capturedIsApp) => () => {
                    if (capturedIsApp) {
                        const currentWindows = WindowManager.getWindowsForApp(capturedAppId);
                        const currentWindowCount = currentWindows.length;

                        if (currentWindowCount === 0) {
                            dependencies.recordSearch("app", capturedAppId, capturedQuery);
                            if (capturedItem.execute?.command) {
                                Quickshell.execDetached(capturedItem.execute.command);
                            }
                        } else if (currentWindowCount === 1) {
                            dependencies.recordWindowFocus(
                                capturedAppId,
                                capturedItem.name,
                                currentWindows[0].title,
                                capturedItem.icon,
                                capturedQuery
                            );
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
                            Quickshell.execDetached([
                                "notify-send",
                                capturedItem._pluginName ?? "Plugin",
                                capturedItem.execute.notify,
                                "-a",
                                "Shell"
                            ]);
                        }
                        if (capturedItem.execute?.name) {
                            dependencies.recordWorkflowExecution({
                                name: capturedItem.execute.name,
                                command: capturedItem.execute?.command ?? [],
                                entryPoint: capturedItem.entryPoint ?? null,
                                icon: capturedItem.icon ?? 'play_arrow',
                                iconType: capturedItem.iconType ?? "material",
                                thumbnail: capturedItem.thumbnail ?? "",
                                workflowId: capturedItem._pluginId,
                                workflowName: capturedItem._pluginName,
                                keepOpen: capturedItem.keepOpen === true
                            }, capturedQuery);
                        }
                    }
                })(item, query, appId, isAppItem)
        });
        
        return {
            matchType: resultMatchType,
            fuzzyScore: fuzzyScore,
            frecency: frecency,
            result: resultObj
        };
    }
}
