pragma Singleton

import qs.modules.common
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root
    
    property var searchHistoryData: []
    property bool historyLoaded: false
    property int maxHistoryItems: Config.options.search.maxHistoryItems
    property int maxRecentSearchTerms: 5
    property int maxTotalScore: 10000
    property int maxAgeDays: 90

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
                recentTerms = recentTerms.slice(0, root.maxRecentSearchTerms);
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
        if (newHistory.length > root.maxHistoryItems) {
            newHistory = newHistory.slice(0, root.maxHistoryItems);
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
                recentTerms = recentTerms.slice(0, root.maxRecentSearchTerms);
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
        
        if (newHistory.length > root.maxHistoryItems) {
            newHistory = newHistory.slice(0, root.maxHistoryItems);
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
        
        if (newHistory.length > root.maxHistoryItems) {
            newHistory = newHistory.slice(0, root.maxHistoryItems);
        }
        
        searchHistoryData = newHistory;
        searchHistoryFileView.setText(JSON.stringify({ history: newHistory }, null, 2));
    }
    
    function ageAndPruneHistory(history, now) {
        let totalCount = history.reduce((sum, item) => sum + item.count, 0);
        
        if (totalCount > root.maxTotalScore) {
            const scaleFactor = (root.maxTotalScore * 0.9) / totalCount;
            history = history.map(item => ({
                type: item.type,
                name: item.name,
                count: item.count * scaleFactor,
                lastUsed: item.lastUsed,
                recentSearchTerms: item.recentSearchTerms
            }));
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
}
