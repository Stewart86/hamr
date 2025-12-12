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

    // File search prefix - fallback if not in config
    property string filePrefix: Config.options.search.prefix.file ?? "~"
    
    function ensurePrefix(prefix) {
        if ([Config.options.search.prefix.action, Config.options.search.prefix.app, Config.options.search.prefix.emojis, Config.options.search.prefix.math, Config.options.search.prefix.shellCommand, Config.options.search.prefix.shellHistory, Config.options.search.prefix.webSearch, root.filePrefix].some(i => root.query.startsWith(i))) {
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

    // Load user action scripts from ~/.config/hamr/actions/
    // Uses FolderListModel to auto-reload when scripts are added/removed
    // Note: Workflow folders (containing manifest.json) are handled by WorkflowRunner
    // Excludes text/config files like .md, .txt, .json, .yaml, etc.
    readonly property var excludedActionExtensions: [".md", ".txt", ".json", ".yaml", ".yml", ".toml", ".ini", ".cfg", ".conf", ".log", ".csv"]
    
    property var userActionScripts: {
        const actions = [];
        for (let i = 0; i < userActionsFolder.count; i++) {
            const fileName = userActionsFolder.get(i, "fileName");
            const filePath = userActionsFolder.get(i, "filePath");
            if (fileName && filePath) {
                // Skip text/config files
                const lowerName = fileName.toLowerCase();
                if (root.excludedActionExtensions.some(ext => lowerName.endsWith(ext))) {
                    continue;
                }
                
                const actionName = fileName.replace(/\.[^/.]+$/, ""); // strip extension
                const scriptPath = filePath.toString().replace("file://", "");
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

    FolderListModel {
        id: userActionsFolder
        folder: `file://${Directories.userActions}`
        showDirs: false
        showHidden: false
        sortField: FolderListModel.Name
    }
    
    // ==================== WORKFLOW INTEGRATION ====================
    // Active workflow state - when a workflow is active, results come from WorkflowRunner
    property bool workflowActive: WorkflowRunner.isActive()
    property string activeWorkflowId: WorkflowRunner.activeWorkflow?.id ?? ""
    
    // Start a workflow by ID
    function startWorkflow(workflowId) {
        const success = WorkflowRunner.startWorkflow(workflowId);
        if (success) {
            // Clear query for fresh workflow input
            root.workflowStarting = true;
            root.query = "";
            root.workflowStarting = false;
        }
        return success;
    }
    
    // Close active workflow
    function closeWorkflow() {
        WorkflowRunner.closeWorkflow();
    }
    
    // Check if we should exit workflow mode (called when query becomes empty)
    function checkWorkflowExit() {
        if (WorkflowRunner.isActive() && root.query === "") {
            WorkflowRunner.closeWorkflow();
        }
    }
    
    // Listen for workflow executions to record in history
    Connections {
        target: WorkflowRunner
        function onActionExecuted(actionInfo) {
            root.recordWorkflowExecution(actionInfo);
        }
        function onClearInputRequested() {
            root.query = "";
        }
    }
    
    // Convert workflow results to LauncherSearchResult objects
    function workflowResultsToSearchResults(workflowResults: var): var {
        return workflowResults.map(item => {
            // Convert workflow actions to LauncherSearchResult action objects
            const itemActions = (item.actions ?? []).map(action => {
                return resultComp.createObject(null, {
                    name: action.name,
                    iconName: action.icon ?? 'play_arrow',
                    iconType: LauncherSearchResult.IconType.Material,
                    execute: () => {
                        WorkflowRunner.selectItem(item.id, action.id);
                    }
                });
            });
            
            return resultComp.createObject(null, {
                name: item.name,
                comment: item.description ?? "",
                verb: item.verb ?? "Select",
                type: WorkflowRunner.activeWorkflow?.manifest?.name ?? "Workflow",
                iconName: item.icon ?? WorkflowRunner.activeWorkflow?.manifest?.icon ?? 'extension',
                iconType: LauncherSearchResult.IconType.Material,
                resultType: LauncherSearchResult.ResultType.WorkflowResult,
                workflowId: WorkflowRunner.activeWorkflow?.id ?? "",
                workflowItemId: item.id,
                workflowActions: item.actions ?? [],
                thumbnail: item.thumbnail ?? "",
                actions: itemActions,
                execute: () => {
                    // Default action: select without specific action
                    WorkflowRunner.selectItem(item.id, "");
                }
            });
        });
    }
    
    // Prepared workflows for fuzzy search (from WorkflowRunner)
    property var preppedWorkflows: WorkflowRunner.preppedWorkflows

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
        }
        onLoadFailed: error => {
            console.log("[Quicklinks] Failed to load quicklinks.json:", error);
            root.quicklinks = [];
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
    
    // Prepared workflow execution history for fuzzy search
    // Indexes both the action name and workflow name for matching
    property var preppedWorkflowExecutions: {
        return searchHistoryData
            .filter(item => item.type === "workflowExecution")
            .map(item => ({
                name: Fuzzy.prepare(`${item.workflowName} ${item.name}`),
                historyItem: item
            }));
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
        }
        onLoadFailed: error => {
            if (error == FileViewError.FileNotFound) {
                // Create empty history file
                searchHistoryFileView.setText(JSON.stringify({ history: [] }));
            }
            root.searchHistoryData = [];
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
    
    // Record a workflow execution (stores command for replay)
    function recordWorkflowExecution(actionInfo) {
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
            newHistory[existingIndex] = {
                type: existing.type,
                key: existing.key,
                name: existing.name,
                workflowId: existing.workflowId,
                workflowName: existing.workflowName,
                command: actionInfo.command,
                icon: actionInfo.icon,
                thumbnail: actionInfo.thumbnail,
                count: existing.count + 1,
                lastUsed: now
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
                icon: actionInfo.icon,
                thumbnail: actionInfo.thumbnail,
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
        SHELL_HISTORY: "shell_history",
        GENERAL: "general"
    })
    
    function detectIntent(query) {
        if (!query || query.trim() === "") return root.intent.GENERAL;
        
        const trimmed = query.trim();
        
        // Explicit prefixes take priority
        if (trimmed.startsWith(Config.options.search.prefix.emojis)) return root.intent.EMOJI;
        if (trimmed.startsWith(root.filePrefix)) return root.intent.FILE;
        if (trimmed.startsWith(Config.options.search.prefix.shellCommand)) return root.intent.COMMAND;
        if (trimmed.startsWith(Config.options.search.prefix.shellHistory)) return root.intent.SHELL_HISTORY;
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
        WORKFLOW: "workflow",
        WORKFLOW_EXECUTION: "workflow_execution",
        URL_DIRECT: "url_direct",
        URL_HISTORY: "url_history",
        EMOJI: "emoji",
        COMMAND: "command",
        SHELL_HISTORY: "shell_history",
        MATH: "math",
        WEB_SEARCH: "web_search"
    })
    
    // ==================== TIERED + COMPETITIVE RANKING ====================
    // Tier 1: Intent-specific (always first based on detected intent)
    // Tier 2: Primary results (Apps, Actions, Quicklinks) - compete by match quality
    // Tier 3: Secondary results (Emoji, URL History, Shell History) - compete by match quality
    // Tier 4: Fallback (Web search) - always last
    // ======================================================================
    
    function getTierConfig(detectedIntent) {
        // Returns which categories go in which tier
        // Categories in the same tier compete by match quality
        let tier1 = [];
        
        switch (detectedIntent) {
            case root.intent.COMMAND:
                tier1 = [root.category.COMMAND];
                break;
            case root.intent.SHELL_HISTORY:
                // Only promote shell history when using ! prefix
                tier1 = [root.category.SHELL_HISTORY];
                break;
            case root.intent.MATH:
                tier1 = [root.category.MATH];
                break;
            case root.intent.URL:
                tier1 = [root.category.URL_DIRECT];
                break;
        }
        
        return {
            tier1: tier1,  // Intent-specific (shown first, in order)
            tier2: [root.category.APP, root.category.ACTION, root.category.WORKFLOW, root.category.QUICKLINK],  // Primary (compete)
            tier3: [root.category.WORKFLOW_EXECUTION, root.category.SHELL_HISTORY, root.category.URL_HISTORY, root.category.EMOJI],  // Secondary (compete)
            tier4: [root.category.WEB_SEARCH]  // Fallback (always last)
        };
    }
    
    // Merge results using tiered + competitive approach
    function mergeByTiers(categorized, tierConfig, limits) {
        const merged = [];
        
        // Tier 1: Intent-specific results (in order, not competing)
        for (const cat of tierConfig.tier1) {
            const results = categorized[cat] || [];
            const limit = limits[cat] ?? 1;
            for (let i = 0; i < Math.min(results.length, limit); i++) {
                merged.push(results[i]);
            }
        }
        
        // Tier 2: Primary results - collect all, sort by match quality, then limit
        const tier2Results = [];
        for (const cat of tierConfig.tier2) {
            const results = categorized[cat] || [];
            const limit = limits[cat] ?? 5;
            // Tag each result with its category for potential future use
            results.slice(0, limit).forEach(r => {
                tier2Results.push(r);
            });
        }
        tier2Results.sort(root.compareResults);
        // Take top results from tier 2 (limit total)
        const tier2Limit = 12;
        tier2Results.slice(0, tier2Limit).forEach(r => merged.push(r));
        
        // Tier 3: Secondary results - collect all, sort by match quality, then limit
        const tier3Results = [];
        for (const cat of tierConfig.tier3) {
            const results = categorized[cat] || [];
            const limit = limits[cat] ?? 3;
            results.slice(0, limit).forEach(r => {
                tier3Results.push(r);
            });
        }
        tier3Results.sort(root.compareResults);
        // Take top results from tier 3 (limit total)
        const tier3Limit = 6;
        tier3Results.slice(0, tier3Limit).forEach(r => merged.push(r));
        
        // Tier 4: Fallback (always last)
        for (const cat of tierConfig.tier4) {
            const results = categorized[cat] || [];
            for (let i = 0; i < Math.min(results.length, 1); i++) {
                merged.push(results[i]);
            }
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
    
    // Compare two results for sorting within a category
    // Returns negative if a should come before b
    function compareResults(a, b) {
        // 1. Match type (exact > prefix > fuzzy)
        if (a.matchType !== b.matchType) return b.matchType - a.matchType;
        // 2. Fuzzy score (higher is better, but scores can be negative)
        if (a.fuzzyScore !== b.fuzzyScore) return b.fuzzyScore - a.fuzzyScore;
        // 3. Frecency (higher is better)
        return b.frecency - a.frecency;
    }
    
    // Merge results from multiple categories based on priority order
    // Takes limited items from each category to create balanced results
    function mergeByPriority(categorizedResults, priorityOrder, limits) {
        const merged = [];
        const defaultLimit = 5;
        
        for (const category of priorityOrder) {
            const results = categorizedResults[category] || [];
            const limit = limits[category] ?? defaultLimit;
            // Results are already sorted within category
            for (let i = 0; i < Math.min(results.length, limit); i++) {
                merged.push(results[i]);
            }
        }
        
        return merged;
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
        return {
            score: score,
            result: resultComp.createObject(null, {
                type: type,
                id: entry.id,
                name: entry.name,
                iconName: entry.icon,
                iconType: LauncherSearchResult.IconType.System,
                verb: "Open",
                execute: () => {
                    root.recordSearch("app", entry.id, root.query);
                    if (!entry.runInTerminal)
                        entry.execute();
                    else {
                        Quickshell.execDetached(["bash", '-c', `${Config.options.apps.terminal} -e '${StringUtils.shellSingleQuoteEscape(entry.command.join(' '))}'`]);
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

    // Combined built-in and user actions
    property var allActions: searchActions.concat(userActionScripts)

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
    
    // Track if we're intentionally clearing query for workflow start
    property bool workflowStarting: false
    
    // Trigger workflow search when query changes
    onQueryChanged: {
        if (WorkflowRunner.isActive()) {
            // Don't exit workflow on empty query - let user use Escape to exit
            // Just send the query to workflow (debounced)
            if (!root.workflowStarting) {
                workflowSearchTimer.restart();
            }
        } else if (root.query === root.filePrefix) {
            // Start files workflow when ~ is typed
            root.startWorkflow("files");
        } else if (root.query === Config.options.search.prefix.clipboard) {
            // Start clipboard workflow when ; is typed
            root.startWorkflow("clipboard");
        }
    }
    
    // Exit workflow mode - should be called on Escape key
    function exitWorkflow() {
        if (WorkflowRunner.isActive()) {
            WorkflowRunner.closeWorkflow();
            root.query = "";
        }
    }
    
    // Debounce timer for workflow search
    Timer {
        id: workflowSearchTimer
        interval: 150
        onTriggered: {
            if (WorkflowRunner.isActive()) {
                WorkflowRunner.search(root.query);
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
            if (expr.startsWith(Config.options.search.prefix.math)) {
                expr = expr.slice(Config.options.search.prefix.math.length);
            }
            
            // Only calculate if it looks like math
            if (root.isMathExpression(expr)) {
                mathProc.calculateExpression(expr);
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
        
        ////////////////// Workflow mode - show workflow results //////////////////
        if (WorkflowRunner.isActive()) {
            // Convert workflow results to LauncherSearchResult objects
            return root.workflowResultsToSearchResults(WorkflowRunner.workflowResults);
        }
        
        ////////////////// Empty query - show recent history //////////////////
        if (root.query == "") {
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
                            execute: () => {
                                root.recordSearch("app", entry.id, "");
                                entry.execute();
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
                            execute: () => {
                                root.recordSearch("action", action.action, "");
                                action.execute("");
                            }
                        });
                    } else if (item.type === "workflow") {
                        const workflow = WorkflowRunner.getWorkflow(item.name);
                        if (!workflow) return null;
                        return resultComp.createObject(null, {
                            type: "Recent",
                            name: workflow.manifest?.name || item.name,
                            iconName: workflow.manifest?.icon || 'extension',
                            iconType: LauncherSearchResult.IconType.Material,
                            resultType: LauncherSearchResult.ResultType.WorkflowEntry,
                            verb: "Open",
                            execute: () => {
                                root.recordSearch("workflow", item.name, "");
                                root.startWorkflow(item.name);
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
                            execute: () => {
                                root.recordSearch("file", item.name, "");
                                Quickshell.execDetached(["xdg-open", item.name]);
                            }
                        });
                    } else if (item.type === "workflowExecution") {
                        return resultComp.createObject(null, {
                            type: item.workflowName || "Recent",
                            name: item.name,
                            iconName: item.icon || 'play_arrow',
                            iconType: LauncherSearchResult.IconType.Material,
                            thumbnail: item.thumbnail || "",
                            verb: "Run",
                            execute: () => {
                                // Re-record to update frecency
                                root.recordWorkflowExecution({
                                    name: item.name,
                                    command: item.command,
                                    icon: item.icon,
                                    thumbnail: item.thumbnail,
                                    workflowId: item.workflowId,
                                    workflowName: item.workflowName
                                });
                                // Execute stored command directly
                                if (item.command && item.command.length > 0) {
                                    Quickshell.execDetached(item.command);
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

        ///////////// Special cases (exclusive - return early) ///////////////
        
        // Actions/Workflows with prefix - show only actions and workflows
        if (root.query.startsWith(Config.options.search.prefix.action)) {
            const searchString = StringUtils.cleanPrefix(root.query, Config.options.search.prefix.action).split(" ")[0];
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
            
            // Get workflows
            const workflowMatches = searchString === ""
                ? WorkflowRunner.workflows.slice(0, 20)
                : Fuzzy.go(searchString, root.preppedWorkflows, { key: "name", limit: 20 }).map(r => r.obj.workflow);
            
            const workflowItems = workflowMatches.map(workflow => {
                return resultComp.createObject(null, {
                    name: workflow.manifest?.name || workflow.id,
                    comment: workflow.manifest?.description || "",
                    verb: "Open",
                    type: "Workflow",
                    iconName: workflow.manifest?.icon || 'extension',
                    iconType: LauncherSearchResult.IconType.Material,
                    execute: () => {
                        root.recordSearch("workflow", workflow.id, root.query);
                        root.startWorkflow(workflow.id);
                    }
                });
            });
            
            return [...workflowItems, ...actionItems].filter(Boolean);
        }
        
        // Emojis with prefix - show full emoji results
        if (root.query.startsWith(Config.options.search.prefix.emojis)) {
            const searchString = StringUtils.cleanPrefix(root.query, Config.options.search.prefix.emojis);
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
        
        // Shell history with prefix - show full shell history results
        if (root.query.startsWith(Config.options.search.prefix.shellHistory)) {
            const searchString = StringUtils.cleanPrefix(root.query, Config.options.search.prefix.shellHistory);
            return ShellHistory.fuzzyQuery(searchString).map(cmd => {
                return resultComp.createObject(null, {
                    name: cmd,
                    verb: "Run",
                    type: "Shell History",
                    fontType: LauncherSearchResult.FontType.Monospace,
                    iconName: 'terminal',
                    iconType: LauncherSearchResult.IconType.Material,
                    execute: () => {
                        // Run command in terminal with interactive shell
                        Quickshell.execDetached(["ghostty", "--class=floating.terminal", "-e", ShellHistory.detectedShell || "bash", "-ic", cmd]);
                    },
                    actions: [
                        resultComp.createObject(null, {
                            name: "Copy",
                            iconName: "content_copy",
                            iconType: LauncherSearchResult.IconType.Material,
                            execute: () => {
                                Quickshell.clipboardText = cmd;
                            }
                        }),
                        resultComp.createObject(null, {
                            name: "Run in terminal",
                            iconName: "terminal",
                            iconType: LauncherSearchResult.IconType.Material,
                            execute: () => {
                                Quickshell.execDetached([Config.options.apps.terminal, "-e", ShellHistory.detectedShell || "bash", "-ic", cmd]);
                            }
                        })
                    ]
                });
            }).filter(Boolean);
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
            [root.category.WORKFLOW]: 5,
            [root.category.QUICKLINK]: 5,
            [root.category.URL_DIRECT]: 1,
            [root.category.URL_HISTORY]: 3,
            [root.category.CLIPBOARD]: 3,
            [root.category.EMOJI]: 3,
            [root.category.COMMAND]: 1,
            [root.category.SHELL_HISTORY]: 5,
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
        
        const actionResults = Fuzzy.go(actionQuery, root.preppedActions, { key: "name", limit: 10 }).map(result => {
            const action = result.obj.action;
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
        categorized[root.category.ACTION] = actionResults.sort(root.compareResults);
        
        // ========== WORKFLOWS ==========
        // Multi-step action workflows from ~/.config/hamr/actions/
        const seenWorkflows = new Set();
        const workflowResults = Fuzzy.go(actionQuery, root.preppedWorkflows, { key: "name", limit: 10 })
            .filter(result => {
                const id = result.obj.workflow.id;
                if (seenWorkflows.has(id)) return false;
                seenWorkflows.add(id);
                return true;
            })
            .map(result => {
                const workflow = result.obj.workflow;
                const manifest = workflow.manifest;
                const frecency = root.getHistoryBoost("workflow", workflow.id);
                const resultMatchType = root.getMatchType(actionQuery, workflow.id);
                
                return {
                    matchType: resultMatchType,
                    fuzzyScore: result._score,
                    frecency: frecency,
                    result: resultComp.createObject(null, {
                        name: manifest.name ?? workflow.id,
                        comment: manifest.description ?? "",
                        verb: "Start",
                        type: "Workflow",
                        iconName: manifest.icon ?? 'extension',
                        iconType: LauncherSearchResult.IconType.Material,
                        resultType: LauncherSearchResult.ResultType.WorkflowEntry,
                        workflowId: workflow.id,
                        acceptsArguments: true,
                        completionText: workflow.id + " ",
                        execute: () => {
                            root.recordSearch("workflow", workflow.id, root.query);
                            root.startWorkflow(workflow.id);
                        }
                    })
                };
            });
        categorized[root.category.WORKFLOW] = workflowResults.sort(root.compareResults).slice(0, 3);
        
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
            
            if (hasSearchTerm) {
                quicklinkResults.push({
                    matchType: resultMatchType,
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
                        execute: () => {
                            root.recordSearch("quicklink", link.name, quicklinkSearchTerm);
                            const url = link.url.replace("{query}", encodeURIComponent(quicklinkSearchTerm));
                            Qt.openUrlExternally(url);
                        }
                    })
                });
            } else {
                // Get recent search terms
                const historyItem = searchHistoryData.find(h => h.type === "quicklink" && h.name === link.name);
                const recentTerms = historyItem?.recentSearchTerms || [];
                
                recentTerms.forEach((term, idx) => {
                    quicklinkResults.push({
                        matchType: resultMatchType,
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
                            execute: () => {
                                root.recordSearch("quicklink", link.name, term);
                                const url = link.url.replace("{query}", encodeURIComponent(term));
                                Qt.openUrlExternally(url);
                            }
                        })
                    });
                });
                
                quicklinkResults.push({
                    matchType: resultMatchType,
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
                        execute: () => {
                            root.recordSearch("quicklink", link.name, "");
                            const url = link.url.replace("{query}", "");
                            Qt.openUrlExternally(url);
                        }
                    })
                });
            }
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
                    execute: () => {
                        root.recordSearch("url", normalizedUrl, "");
                        Quickshell.execDetached(["xdg-open", normalizedUrl]);
                    }
                })
            }];
        } else {
            categorized[root.category.URL_DIRECT] = [];
        }
        
        // ========== URL History ==========
        const urlHistoryResults = Fuzzy.go(root.query, root.preppedUrlHistory, { key: "name", limit: 5 })
            .filter(result => result.obj.url !== normalizedUrl)
            .map(result => ({
                matchType: root.getMatchType(root.query, root.stripProtocol(result.obj.url)),
                fuzzyScore: result._score,
                frecency: root.getFrecencyScore(result.obj.historyItem),
                result: resultComp.createObject(null, {
                    name: result.obj.url,
                    verb: "Open",
                    type: "URL" + " - recent",
                    fontType: LauncherSearchResult.FontType.Monospace,
                    iconName: 'open_in_browser',
                    iconType: LauncherSearchResult.IconType.Material,
                    execute: () => {
                        root.recordSearch("url", result.obj.url, "");
                        Quickshell.execDetached(["xdg-open", result.obj.url]);
                    }
                })
            }));
        categorized[root.category.URL_HISTORY] = urlHistoryResults.sort(root.compareResults);
        
        // ========== WORKFLOW EXECUTIONS ==========
        const workflowExecResults = Fuzzy.go(root.query, root.preppedWorkflowExecutions, { key: "name", limit: 5 })
            .map(result => {
                const item = result.obj.historyItem;
                return {
                    matchType: root.matchType.FUZZY,
                    fuzzyScore: result._score,
                    frecency: root.getFrecencyScore(item),
                    result: resultComp.createObject(null, {
                        type: item.workflowName || "Recent",
                        name: item.name,
                        iconName: item.icon || 'play_arrow',
                        iconType: LauncherSearchResult.IconType.Material,
                        thumbnail: item.thumbnail || "",
                        verb: "Run",
                        execute: () => {
                            root.recordWorkflowExecution({
                                name: item.name,
                                command: item.command,
                                icon: item.icon,
                                thumbnail: item.thumbnail,
                                workflowId: item.workflowId,
                                workflowName: item.workflowName
                            });
                            if (item.command && item.command.length > 0) {
                                Quickshell.execDetached(item.command);
                            }
                        }
                    })
                };
            });
        categorized[root.category.WORKFLOW_EXECUTION] = workflowExecResults.sort(root.compareResults);
        
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
                    name: StringUtils.cleanPrefix(root.query, Config.options.search.prefix.shellCommand).replace("file://", ""),
                    verb: "Run",
                    type: "Command",
                    fontType: LauncherSearchResult.FontType.Monospace,
                    iconName: 'terminal',
                    iconType: LauncherSearchResult.IconType.Material,
                    execute: () => {
                        let cleanedCommand = root.query.replace("file://", "");
                        cleanedCommand = StringUtils.cleanPrefix(cleanedCommand, Config.options.search.prefix.shellCommand);
                        Quickshell.execDetached(["ghostty", "--class=floating.terminal", "-e", "zsh", "-ic", cleanedCommand]);
                    }
                })
            }];
        } else {
            categorized[root.category.COMMAND] = [];
        }
        
        // ========== SHELL HISTORY ==========
        // Include shell history commands that fuzzy match the query
        if (ShellHistory.enabled && ShellHistory.ready) {
            const shellHistoryResults = ShellHistory.fuzzyQueryWithScores(root.query).slice(0, 10).map(item => {
                const cmd = item.command;
                return {
                    matchType: root.getMatchType(root.query, cmd),
                    fuzzyScore: item.score,
                    frecency: 0, // Shell history doesn't use frecency from launcher
                    result: resultComp.createObject(null, {
                        name: cmd,
                        verb: "Run",
                        type: "Shell History",
                        fontType: LauncherSearchResult.FontType.Monospace,
                        iconName: 'terminal',
                        iconType: LauncherSearchResult.IconType.Material,
                        execute: () => {
                            Quickshell.execDetached(["ghostty", "--class=floating.terminal", "-e", ShellHistory.detectedShell || "bash", "-ic", cmd]);
                        },
                        actions: [
                            resultComp.createObject(null, {
                                name: "Copy",
                                iconName: "content_copy",
                                iconType: LauncherSearchResult.IconType.Material,
                                execute: () => {
                                    Quickshell.clipboardText = cmd;
                                }
                            }),
                            resultComp.createObject(null, {
                                name: "Run in terminal",
                                iconName: "terminal",
                                iconType: LauncherSearchResult.IconType.Material,
                                execute: () => {
                                    Quickshell.execDetached([Config.options.apps.terminal, "-e", ShellHistory.detectedShell || "bash", "-ic", cmd]);
                                }
                            })
                        ]
                    })
                };
            });
            categorized[root.category.SHELL_HISTORY] = shellHistoryResults.sort(root.compareResults);
        } else {
            categorized[root.category.SHELL_HISTORY] = [];
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
        
        // Merge results using tiered + competitive approach
        const tierConfig = root.getTierConfig(detectedIntent);
        const merged = root.mergeByTiers(categorized, tierConfig, categoryLimits);
        
        return merged.map(item => item.result);
    }

    Component {
        id: resultComp
        LauncherSearchResult {}
    }
}
