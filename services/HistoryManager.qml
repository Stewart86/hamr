pragma Singleton

import qs.modules.common
import qs.modules.common.functions
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root

    property var searchHistoryData: []
    property bool historyLoaded: false
    property int maxHistoryItems: Config.options.search.maxHistoryItems
    property int maxRecentSearchTerms: 10
    property int maxTotalScore: 10000
    property int maxAgeDays: 90
    property int maxSequenceItems: 5
    property bool _pendingSave: false

    Timer {
        id: saveDebounceTimer
        interval: 500
        repeat: false
        onTriggered: {
            if (root._pendingSave) {
                searchHistoryFileView.setText(JSON.stringify({ history: root.searchHistoryData }, null, 2));
                root._pendingSave = false;
            }
        }
    }

    function deferredSave() {
        root._pendingSave = true;
        saveDebounceTimer.restart();
    }

    FileView {
        id: searchHistoryFileView
        path: Directories.searchHistory
        watchChanges: true
        onFileChanged: searchHistoryFileView.reload()
        onLoaded: {
            try {
                const data = JSON.parse(searchHistoryFileView.text());
                const history = data.history || [];
                root.searchHistoryData = migrateWindowFocusEntries(history);
            } catch (e) {
                console.error("[SearchHistory] Failed to parse:", e);
                root.searchHistoryData = [];
            }
            root.historyLoaded = true;
        }
        onLoadFailed: error => {
            if (error == FileViewError.FileNotFound) {
                searchHistoryFileView.setText(JSON.stringify({ history: [] }));
            }
            root.searchHistoryData = [];
            root.historyLoaded = true;
        }
    }

    function migrateWindowFocusEntries(history) {
        const windowFocusMap = new Map();
        const otherEntries = [];

        for (const item of history) {
            if (item.type === "windowFocus") {
                const newKey = "windowFocus:" + item.appId;
                const existing = windowFocusMap.get(newKey);

                if (existing) {
                    existing.count += item.count;
                    if (item.lastUsed > existing.lastUsed) {
                        existing.lastUsed = item.lastUsed;
                        existing.windowTitle = item.windowTitle;
                    }
                } else {
                    windowFocusMap.set(newKey, {
                        type: "windowFocus",
                        key: newKey,
                        appId: item.appId,
                        appName: item.appName,
                        windowTitle: item.windowTitle,
                        iconName: item.iconName,
                        count: item.count,
                        lastUsed: item.lastUsed
                    });
                }
            } else {
                otherEntries.push(item);
            }
        }

        const migratedHistory = [...otherEntries, ...windowFocusMap.values()];

        if (windowFocusMap.size > 0 && migratedHistory.length !== history.length) {
            searchHistoryFileView.setText(JSON.stringify({ history: migratedHistory }, null, 2));
        }

        return migratedHistory;
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

    function createEmptySmartFields() {
        return {
            hourSlotCounts: new Array(24).fill(0),
            dayOfWeekCounts: new Array(7).fill(0),
            workspaceCounts: {},
            monitorCounts: {},
            launchedAfter: {},
            sessionStartCount: 0,
            resumeFromIdleCount: 0,
            launchFromEmptyCount: 0,
            consecutiveDays: 0,
            lastConsecutiveDate: ""
        };
    }

    function updateSmartFields(existing, context) {
        const now = Date.now();
        const hour = StatisticalUtils.getHourSlot(now);
        const day = StatisticalUtils.getDayOfWeek(now);

        const hourSlotCounts = existing.hourSlotCounts ? existing.hourSlotCounts.slice() : new Array(24).fill(0);
        const dayOfWeekCounts = existing.dayOfWeekCounts ? existing.dayOfWeekCounts.slice() : new Array(7).fill(0);
        const workspaceCounts = existing.workspaceCounts ? Object.assign({}, existing.workspaceCounts) : {};
        const monitorCounts = existing.monitorCounts ? Object.assign({}, existing.monitorCounts) : {};
        const launchedAfter = existing.launchedAfter ? Object.assign({}, existing.launchedAfter) : {};

        hourSlotCounts[hour] = (hourSlotCounts[hour] || 0) + 1;
        dayOfWeekCounts[day] = (dayOfWeekCounts[day] || 0) + 1;

        if (context && context.workspace) {
            workspaceCounts[context.workspace] = (workspaceCounts[context.workspace] || 0) + 1;
        }
        if (context && context.monitor) {
            monitorCounts[context.monitor] = (monitorCounts[context.monitor] || 0) + 1;
        }

        if (context && context.lastApp) {
            launchedAfter[context.lastApp] = (launchedAfter[context.lastApp] || 0) + 1;
            const sorted = Object.entries(launchedAfter).sort((a, b) => b[1] - a[1]);
            if (sorted.length > root.maxSequenceItems) {
                const keys = Object.keys(launchedAfter);
                for (const k of keys) {
                    delete launchedAfter[k];
                }
                for (let i = 0; i < root.maxSequenceItems; i++) {
                    launchedAfter[sorted[i][0]] = sorted[i][1];
                }
            }
        }

        let sessionStartCount = existing.sessionStartCount || 0;
        if (context && context.isSessionStart) {
            sessionStartCount++;
        }

        let resumeFromIdleCount = existing.resumeFromIdleCount || 0;
        if (context && context.isResumeFromIdle) {
            resumeFromIdleCount++;
        }

        let launchFromEmptyCount = existing.launchFromEmptyCount || 0;
        if (context && context.launchFromEmpty) {
            launchFromEmptyCount++;
        }

        const today = new Date().toISOString().split('T')[0];
        const yesterday = new Date(Date.now() - 86400000).toISOString().split('T')[0];
        let consecutiveDays = existing.consecutiveDays || 0;
        let lastConsecutiveDate = existing.lastConsecutiveDate || "";

        if (lastConsecutiveDate === today) {
            // Already used today, no change
        } else if (lastConsecutiveDate === yesterday) {
            consecutiveDays++;
            lastConsecutiveDate = today;
        } else {
            consecutiveDays = 1;
            lastConsecutiveDate = today;
        }

        return {
            hourSlotCounts: hourSlotCounts,
            dayOfWeekCounts: dayOfWeekCounts,
            workspaceCounts: workspaceCounts,
            monitorCounts: monitorCounts,
            launchedAfter: launchedAfter,
            sessionStartCount: sessionStartCount,
            resumeFromIdleCount: resumeFromIdleCount,
            launchFromEmptyCount: launchFromEmptyCount,
            consecutiveDays: consecutiveDays,
            lastConsecutiveDate: lastConsecutiveDate
        };
    }

    function mergeWithSmartFields(baseObj, smartFields) {
        baseObj.hourSlotCounts = smartFields.hourSlotCounts;
        baseObj.dayOfWeekCounts = smartFields.dayOfWeekCounts;
        baseObj.workspaceCounts = smartFields.workspaceCounts;
        baseObj.monitorCounts = smartFields.monitorCounts;
        baseObj.launchedAfter = smartFields.launchedAfter;
        baseObj.sessionStartCount = smartFields.sessionStartCount;
        baseObj.resumeFromIdleCount = smartFields.resumeFromIdleCount;
        baseObj.launchFromEmptyCount = smartFields.launchFromEmptyCount;
        baseObj.consecutiveDays = smartFields.consecutiveDays;
        baseObj.lastConsecutiveDate = smartFields.lastConsecutiveDate;
        return baseObj;
    }

    function recordSearch(searchType, searchName, searchTerm, context) {
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
                recentTerms = recentTerms.slice(0, root.maxRecentSearchTerms);
            }

            const smartFields = updateSmartFields(existing, context);

            newHistory[existingIndex] = mergeWithSmartFields({
                type: existing.type,
                name: existing.name,
                count: existing.count + 1,
                lastUsed: now,
                recentSearchTerms: recentTerms
            }, smartFields);
        } else {
            const smartFields = updateSmartFields({}, context);

            newHistory.unshift(mergeWithSmartFields({
                type: searchType,
                name: searchName,
                count: 1,
                lastUsed: now,
                recentSearchTerms: searchTerm ? [searchTerm] : []
            }, smartFields));
        }

        newHistory = ageAndPruneHistory(newHistory, now);

        if (newHistory.length > root.maxHistoryItems) {
            newHistory = newHistory.slice(0, root.maxHistoryItems);
        }

        searchHistoryData = newHistory;
        searchHistoryFileView.setText(JSON.stringify({ history: newHistory }, null, 2));
    }

    function recordWorkflowExecution(actionInfo, searchTerm, context) {
        const now = Date.now();
        const key = actionInfo.workflowId + ":" + actionInfo.name;
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
                recentTerms = recentTerms.slice(0, root.maxRecentSearchTerms);
            }

            const smartFields = updateSmartFields(existing, context);

            newHistory[existingIndex] = mergeWithSmartFields({
                type: existing.type,
                key: existing.key,
                name: existing.name,
                workflowId: existing.workflowId,
                workflowName: existing.workflowName,
                command: actionInfo.command,
                entryPoint: actionInfo.entryPoint || null,
                icon: actionInfo.icon,
                iconType: actionInfo.iconType || existing.iconType || "material",
                thumbnail: actionInfo.thumbnail,
                keepOpen: actionInfo.keepOpen || existing.keepOpen || false,
                count: existing.count + 1,
                lastUsed: now,
                recentSearchTerms: recentTerms
            }, smartFields);
        } else {
            const smartFields = updateSmartFields({}, context);

            newHistory.unshift(mergeWithSmartFields({
                type: "workflowExecution",
                key: key,
                name: actionInfo.name,
                workflowId: actionInfo.workflowId,
                workflowName: actionInfo.workflowName,
                command: actionInfo.command,
                entryPoint: actionInfo.entryPoint || null,
                icon: actionInfo.icon,
                iconType: actionInfo.iconType || "material",
                thumbnail: actionInfo.thumbnail,
                keepOpen: actionInfo.keepOpen || false,
                count: 1,
                lastUsed: now,
                recentSearchTerms: searchTerm ? [searchTerm] : []
            }, smartFields));
        }

        newHistory = ageAndPruneHistory(newHistory, now);

        if (newHistory.length > root.maxHistoryItems) {
            newHistory = newHistory.slice(0, root.maxHistoryItems);
        }

        searchHistoryData = newHistory;
        searchHistoryFileView.setText(JSON.stringify({ history: newHistory }, null, 2));
    }

    function recordWindowFocus(appId, appName, windowTitle, iconName, searchTerm, context) {
        const now = Date.now();
        let newHistory = searchHistoryData.slice();

        if (searchTerm) {
            const appIndex = newHistory.findIndex(h => h.type === "app" && h.name === appId);
            if (appIndex >= 0) {
                const existing = newHistory[appIndex];
                let recentTerms = existing.recentSearchTerms || [];
                recentTerms = recentTerms.filter(t => t !== searchTerm);
                recentTerms.unshift(searchTerm);
                recentTerms = recentTerms.slice(0, root.maxRecentSearchTerms);

                const smartFields = updateSmartFields(existing, context);

                newHistory[appIndex] = mergeWithSmartFields({
                    type: existing.type,
                    name: existing.name,
                    count: existing.count + 1,
                    lastUsed: now,
                    recentSearchTerms: recentTerms
                }, smartFields);
            } else {
                const smartFields = updateSmartFields({}, context);

                newHistory.unshift(mergeWithSmartFields({
                    type: "app",
                    name: appId,
                    count: 1,
                    lastUsed: now,
                    recentSearchTerms: [searchTerm]
                }, smartFields));
            }
        }

        const key = "windowFocus:" + appId;
        const existingIndex = newHistory.findIndex(
            h => h.type === "windowFocus" && h.key === key
        );

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

        if (newHistory.length > root.maxHistoryItems) {
            newHistory = newHistory.slice(0, root.maxHistoryItems);
        }

        searchHistoryData = newHistory;
        deferredSave();
    }

    function ageAndPruneHistory(history, now) {
        let totalCount = history.reduce((sum, item) => sum + item.count, 0);

        if (totalCount > root.maxTotalScore) {
            const scaleFactor = (root.maxTotalScore * 0.9) / totalCount;
            history = history.map(item => {
                const scaled = Object.assign({}, item);
                scaled.count = item.count * scaleFactor;

                if (scaled.hourSlotCounts) {
                    scaled.hourSlotCounts = scaled.hourSlotCounts.map(c => c * scaleFactor);
                }
                if (scaled.dayOfWeekCounts) {
                    scaled.dayOfWeekCounts = scaled.dayOfWeekCounts.map(c => c * scaleFactor);
                }
                if (scaled.workspaceCounts) {
                    const ws = {};
                    const entries = Object.entries(scaled.workspaceCounts);
                    for (let i = 0; i < entries.length; i++) {
                        ws[entries[i][0]] = entries[i][1] * scaleFactor;
                    }
                    scaled.workspaceCounts = ws;
                }
                if (scaled.monitorCounts) {
                    const mc = {};
                    const entries = Object.entries(scaled.monitorCounts);
                    for (let i = 0; i < entries.length; i++) {
                        mc[entries[i][0]] = entries[i][1] * scaleFactor;
                    }
                    scaled.monitorCounts = mc;
                }
                if (scaled.launchedAfter) {
                    const la = {};
                    const entries = Object.entries(scaled.launchedAfter);
                    for (let i = 0; i < entries.length; i++) {
                        la[entries[i][0]] = entries[i][1] * scaleFactor;
                    }
                    scaled.launchedAfter = la;
                }
                if (scaled.sessionStartCount) {
                    scaled.sessionStartCount = scaled.sessionStartCount * scaleFactor;
                }
                if (scaled.resumeFromIdleCount) {
                    scaled.resumeFromIdleCount = scaled.resumeFromIdleCount * scaleFactor;
                }
                if (scaled.launchFromEmptyCount) {
                    scaled.launchFromEmptyCount = scaled.launchFromEmptyCount * scaleFactor;
                }

                return scaled;
            });
        }

        const maxAgeMs = root.maxAgeDays * 24 * 60 * 60 * 1000;
        history = history.filter(item => {
            const age = now - item.lastUsed;
            const isOld = age > maxAgeMs;
            const hasLowScore = item.count < 1;
            return !(isOld && hasLowScore);
        });

        return history;
    }

    function getAppLaunchCount(appId) {
        const item = searchHistoryData.find(h => h.type === "app" && h.name === appId);
        return item ? item.count : 0;
    }

    function getAppHistoryItems() {
        return searchHistoryData.filter(h => h.type === "app");
    }

    function getHistoryItem(type, name) {
        return searchHistoryData.find(h => h.type === type && h.name === name);
    }
}
