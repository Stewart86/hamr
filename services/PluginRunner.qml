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
    
    // Polling: interval in ms (0 = disabled)
    // Can be set via manifest.json "poll" field or response "pollInterval" field
    property int pollInterval: 0
    property string lastPollQuery: ""  // Last query sent to plugin (for poll context)
    
    // Flag to indicate the next result update is from a poll (not user action)
    // This is set before sending poll request and cleared after results are processed
    property bool isPollUpdate: false
    
    
     // Replay mode: when true, plugin is running a replay action (no UI needed)
     // Process should complete even if launcher closes
     property bool replayMode: false
     
     // Store plugin info for replay mode (activePlugin may be cleared before response)
     property var replayPluginInfo: null

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

    // ==================== PLUGIN INDEXING ====================
    // Plugins can provide searchable items via step: "index"
    // These items appear in main search without entering the plugin
    
    // Indexed items per plugin: { pluginId: { items: [...], lastIndexed: timestamp } }
    property var pluginIndexes: ({})
    
    // Track which plugins are currently being indexed (to avoid concurrent requests)
    property var indexingPlugins: ({})
    
    // Queue of plugins pending indexing (since we can only run one at a time)
    property var indexQueue: []
    
    // Request index from a plugin
    // mode: "full" (replace all items) or "incremental" (merge/remove)
    function indexPlugin(pluginId, mode = "full") {
        const plugin = root.plugins.find(p => p.id === pluginId);
        if (!plugin || !plugin.manifest) {
            console.warn(`[PluginRunner] indexPlugin: Plugin not found: ${pluginId}`);
            return false;
        }
        
        // Check if plugin supports indexing
        const indexConfig = plugin.manifest.index;
        if (!indexConfig || !indexConfig.enabled) {
            return false;
        }
        
        // Don't queue if already queued or indexing
        if (root.indexingPlugins[pluginId]) {
            return false;
        }
        if (root.indexQueue.some(item => item.pluginId === pluginId)) {
            return false;
        }
        
        // Add to queue
        root.indexQueue = [...root.indexQueue, { pluginId, mode }];
        
        // Start processing queue if not already running
        processIndexQueue();
        
        return true;
    }
    
    // Process the next item in the index queue
    function processIndexQueue() {
        // Don't start if already indexing or queue is empty
        if (indexProcess.running || root.indexQueue.length === 0) {
            return;
        }
        
        // Get next item from queue
        const next = root.indexQueue[0];
        root.indexQueue = root.indexQueue.slice(1);
        
        const plugin = root.plugins.find(p => p.id === next.pluginId);
        if (!plugin || !plugin.manifest) {
            // Skip invalid, process next
            processIndexQueue();
            return;
        }
        
        root.indexingPlugins[next.pluginId] = true;
        
        const handlerPath = plugin.manifest._handlerPath ?? (plugin.path + "/handler.py");
        const input = {
            step: "index",
            mode: next.mode
        };
        
        // For incremental, include timestamp and current indexed IDs
        if (next.mode === "incremental" && root.pluginIndexes[next.pluginId]) {
            input.since = root.pluginIndexes[next.pluginId].lastIndexed;
            input.indexedIds = (root.pluginIndexes[next.pluginId].items ?? []).map(item => item.id);
        }
        
        const inputJson = JSON.stringify(input);
        const escapedInput = inputJson.replace(/'/g, "'\\''");
        
        // Start indexing process
        // Execute handler directly (language-agnostic) - relies on shebang or executable
        indexProcess.pluginId = next.pluginId;
        indexProcess.workingDirectory = plugin.path;
        indexProcess.command = ["bash", "-c", `echo '${escapedInput}' | "${handlerPath}"`];
        indexProcess.running = true;
    }
    
    // Index all plugins that support indexing (called on startup)
    // Skip indexing for plugins with cached data AND (watchers OR reindex: "never")
    function indexAllPlugins() {
        const indexablePlugins = root.plugins.filter(p => p.manifest?.index?.enabled);
        console.log(`[PluginRunner] Starting indexing for ${indexablePlugins.length} plugins`);
        
        for (const plugin of indexablePlugins) {
            const hasCachedData = root.pluginIndexes[plugin.id]?.items?.length > 0;
            const hasWatchers = plugin.manifest.index.watchFiles?.length > 0 || 
                               plugin.manifest.index.watchDirs?.length > 0;
            const neverReindex = plugin.manifest.index.reindex === "never";
            
            // Skip indexing if we have cache AND (watchers OR never reindex)
            if (hasCachedData && (hasWatchers || neverReindex)) {
                const reason = hasWatchers ? "has watchers" : "reindex: never";
                console.log(`[PluginRunner] Skipping ${plugin.id}: using cached ${root.pluginIndexes[plugin.id].items.length} items (${reason})`);
                // Still setup watchers for this plugin
                PluginWatcher.setupWatchers(plugin.id, plugin.manifest);
                continue;
            }
            
            const mode = hasCachedData ? "incremental" : "full";
            console.log(`[PluginRunner] Queueing ${plugin.id} for ${mode} index (cached: ${hasCachedData ? root.pluginIndexes[plugin.id].items.length + " items" : "none"})`);
            root.indexPlugin(plugin.id, mode);
        }
    }
    
    // Handle index response from plugin
    function handleIndexResponse(pluginId, response) {
        root.indexingPlugins[pluginId] = false;
        
        if (!response || response.type !== "index") {
            console.warn(`[PluginRunner] Invalid index response from ${pluginId}`);
            return;
        }
        
        const isIncremental = response.mode === "incremental";
        const itemCount = response.items?.length ?? 0;
        const now = Date.now();
        
        if (isIncremental && root.pluginIndexes[pluginId]) {
            // Incremental: merge new items, remove deleted
            const existing = root.pluginIndexes[pluginId].items ?? [];
            const newItems = response.items ?? [];
            const removeIds = new Set(response.remove ?? []);
            
            // Remove deleted items
            let merged = existing.filter(item => !removeIds.has(item.id));
            
            // Update or add new items
            const existingIds = new Set(merged.map(item => item.id));
            for (const item of newItems) {
                if (existingIds.has(item.id)) {
                    // Update existing
                    merged = merged.map(i => i.id === item.id ? item : i);
                } else {
                    // Add new
                    merged.push(item);
                }
            }
            
            root.pluginIndexes[pluginId] = {
                items: merged,
                lastIndexed: now
            };
            console.log(`[PluginRunner] Indexed ${pluginId}: ${itemCount} items (incremental, merged to ${merged.length})`);
        } else {
            // Full: replace all items
            root.pluginIndexes[pluginId] = {
                items: response.items ?? [],
                lastIndexed: now
            };
            console.log(`[PluginRunner] Indexed ${pluginId}: ${itemCount} items (full)`);
        }
        
        // Notify listeners (LauncherSearch) that index changed
        root.pluginIndexChanged(pluginId);
        
        // Save cache to disk (debounced)
        root.saveIndexCache();
        
        // Setup watchers (file, dir, Hyprland events, periodic reindex)
        const plugin = root.plugins.find(p => p.id === pluginId);
        if (plugin?.manifest) {
            PluginWatcher.setupWatchers(pluginId, plugin.manifest);
        }
    }
    
    // Connect to PluginWatcher reindex requests
    Connections {
        target: PluginWatcher
        function onReindexRequested(pluginId, mode) {
            root.indexPlugin(pluginId, mode);
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
    
    // Process for indexing plugins (separate from main plugin process)
    Process {
        id: indexProcess
        property string pluginId: ""
        
        stdout: StdioCollector {
            id: indexStdout
            onStreamFinished: {
                const output = indexStdout.text.trim();
                if (!output) {
                    root.indexingPlugins[indexProcess.pluginId] = false;
                    // Process next in queue
                    root.processIndexQueue();
                    return;
                }
                
                try {
                    const response = JSON.parse(output);
                    root.handleIndexResponse(indexProcess.pluginId, response);
                } catch (e) {
                    console.warn(`[PluginRunner] Failed to parse index response from ${indexProcess.pluginId}: ${e}`);
                    root.indexingPlugins[indexProcess.pluginId] = false;
                }
                // Process next in queue
                root.processIndexQueue();
            }
        }
        
        stderr: SplitParser {
            onRead: data => console.warn(`[PluginRunner] index stderr (${indexProcess.pluginId}): ${data}`)
        }
        
        onExited: (exitCode, exitStatus) => {
            if (exitCode !== 0) {
                console.warn(`[PluginRunner] Index process for ${indexProcess.pluginId} exited with code ${exitCode}`);
                root.indexingPlugins[indexProcess.pluginId] = false;
                // Process next in queue even on failure
                root.processIndexQueue();
            }
        }
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
                    
                    // Log cache loading stats
                    const pluginIds = Object.keys(data.indexes);
                    const totalItems = pluginIds.reduce((sum, id) => sum + (data.indexes[id]?.items?.length ?? 0), 0);
                    console.log(`[PluginRunner] Loaded index cache: ${pluginIds.length} plugins, ${totalItems} total items`);
                    for (const pluginId of pluginIds) {
                        const itemCount = data.indexes[pluginId]?.items?.length ?? 0;
                        console.log(`[PluginRunner]   - ${pluginId}: ${itemCount} items`);
                        root.pluginIndexChanged(pluginId);
                    }
                }
            } catch (e) {
                console.log("[PluginRunner] Failed to parse index cache:", e);
            }
            root.indexCacheLoaded = true;
        }
        
        onLoadFailed: error => {
            if (error !== FileViewError.FileNotFound) {
                console.log("[PluginRunner] Failed to load index cache:", error);
            } else {
                console.log("[PluginRunner] No index cache found, will perform full index");
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
        const pluginIds = Object.keys(root.pluginIndexes);
        const totalItems = pluginIds.reduce((sum, id) => sum + (root.pluginIndexes[id]?.items?.length ?? 0), 0);
        console.log(`[PluginRunner] Saving index cache: ${pluginIds.length} plugins, ${totalItems} total items`);
        
        const data = {
            version: 1,
            savedAt: Date.now(),
            indexes: root.pluginIndexes
        };
        const json = JSON.stringify(data);
        // Use FileView.setText to write
        indexCacheFile.setText(json);
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
            // Start pending plugin if one was requested before loading finished
            if (root.pendingPluginStart !== "") {
                const pluginId = root.pendingPluginStart;
                root.pendingPluginStart = "";
                root.startPlugin(pluginId);
            }
            // Index all plugins that support indexing
            root.indexAllPlugins();
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
         root.pollInterval = plugin.manifest.poll ?? 0;  // Poll interval from manifest
         root.lastPollQuery = "";
         
         sendToPlugin({ step: "initial", session: session });
         return true;
     }
    
     // Send search query to active plugin
     function search(query) {
         if (!root.activePlugin) {
             console.log("[PluginRunner] sendToPlugin: No active plugin");
             return;
         }
         
         // Track last query for poll context
         root.lastPollQuery = query;
         
         // Don't clear card here - it should persist until new response arrives
         
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
         
         sendToPlugin(input);
     }
    
      // Select an item and optionally execute an action
      function selectItem(itemId, actionId) {
          if (!root.activePlugin) return;
          
          // Store selection for context in subsequent search calls
          root.lastSelectedItem = itemId;
          
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
          
          sendToPlugin(input);
      }
      
      // Send slider value change to active plugin (for result item sliders)
      function sliderValueChanged(itemId, value) {
          if (!root.activePlugin) return;
          
          const input = {
              step: "action",
              selected: { id: itemId },
              action: "slider",
              value: value,
              session: root.activePlugin.session
          };
          
          // Include context if set
          if (root.pluginContext) {
              input.context = root.pluginContext;
          }
          
          sendToPlugin(input);
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
          
          sendToPlugin(input);
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
          
          sendToPlugin(input);
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
         
         sendToPlugin(input);
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
         
         sendToPlugin(input);
     }
    
     // Close active plugin
     // If in replay mode, let the process finish (notification needs to be sent)
     function closePlugin() {
         // In replay mode, don't kill the process - let it complete for notification
         if (!root.replayMode) {
             pluginProcess.running = false;
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
         root.pollInterval = 0;
         root.lastPollQuery = "";
         root.pluginActions = [];
         root.navigationDepth = 0;
         root.pendingNavigation = false;
         root.pendingBack = false;
         root.pluginClosed();
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
         
         sendToPlugin(input);
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
         
         sendToPlugin(input);
     }
    
     // Replay a saved action using entryPoint
     // Used for history items that need plugin logic instead of direct command
     // Returns true if replay was initiated, false if plugin not found
     function replayAction(pluginId, entryPoint) {
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
         root.replayMode = true;  // Don't kill process when launcher closes
         root.replayPluginInfo = {
             id: plugin.id,
             name: plugin.manifest.name,
             icon: plugin.manifest.icon
         };
         
         // Build replay input from entryPoint
         const input = {
             step: entryPoint.step ?? "action",
             session: session,
             replay: true  // Signal to handler this is a replay
         };
         
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
         
         sendToPlugin(input);
         return true;
     }
     
     // Execute an entryPoint action with UI visible (not replay mode)
     // Used for indexed items that open a view (e.g., viewing a note)
     // Returns true if action was initiated, false if plugin not found
     function executeEntryPoint(pluginId, entryPoint) {
         console.log(`[PluginRunner] executeEntryPoint: ${pluginId}, ${JSON.stringify(entryPoint)}`);
         const plugin = root.plugins.find(w => w.id === pluginId);
         if (!plugin || !plugin.manifest || !entryPoint) {
             console.log(`[PluginRunner] executeEntryPoint failed: plugin=${!!plugin}, manifest=${!!plugin?.manifest}, entryPoint=${!!entryPoint}`);
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
         root.replayMode = false;  // Keep UI visible
         root.navigationDepth = 1;  // We're entering at depth 1 (not initial view)
         
         // Build input from entryPoint
         const input = {
             step: entryPoint.step ?? "action",
             session: session
         };
         
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
         
         sendToPlugin(input);
         return true;
     }
    
    // Execute a detached preview action (has its own command)
    function executeDetachedPreviewAction(action) {
        if (!action) return;
        
        if (action.command) {
            Quickshell.execDetached(action.command);
        }
        if (action.notify) {
            Quickshell.execDetached(["notify-send", "Preview", action.notify, "-a", "Shell"]);
        }
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
         
         console.log("[PluginRunner] handlePluginResponse type:", response?.type);
         
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
              case "results":
                  root.pluginResults = response.results ?? [];
                  root.pluginCard = null;
                  root.pluginForm = null;
                  if (response.placeholder !== undefined) {
                      root.pluginPlaceholder = response.placeholder ?? "";
                  }
                  // Allow handler to set the context for subsequent search calls
                  if (response.context !== undefined) {
                      root.pluginContext = response.context ?? "";
                  }
                  // Set input mode from response (defaults to realtime)
                  root.inputMode = response.inputMode ?? "realtime";
                  // Allow handler to override poll interval dynamically
                  if (response.pollInterval !== undefined) {
                      root.pollInterval = response.pollInterval ?? 0;
                  }
                  // Plugin-level actions (toolbar buttons)
                  if (response.pluginActions !== undefined) {
                      root.pluginActions = response.pluginActions ?? [];
                  }
                  if (response.clearInput) {
                      root.clearInputRequested();
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
                 if (response.execute) {
                     const exec = response.execute;
                     
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
                     
                     if (exec.command) {
                         Quickshell.execDetached(exec.command);
                     }
                     if (exec.notify) {
                         Quickshell.execDetached(["notify-send", pluginName, exec.notify, "-a", "Shell"]);
                     }
                     // If handler provides name, emit for history tracking
                     // Include entryPoint for complex actions that need plugin replay
                     if (exec.name) {
                         root.actionExecuted({
                             name: exec.name,
                             command: exec.command ?? [],
                             entryPoint: exec.entryPoint ?? null,  // For plugin replay
                             icon: exec.icon ?? pluginIcon,
                             iconType: exec.iconType ?? "material",  // "system" for app icons
                             thumbnail: exec.thumbnail ?? "",
                             workflowId: pluginId,
                             workflowName: pluginName
                         });
                     }
                     if (exec.close) {
                         root.executeCommand(exec);
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
                 
             default:
                 console.warn(`[PluginRunner] Unknown response type: ${response.type}`);
         }
     }
    
     // Handle image browser selection - send back to plugin
     Connections {
         target: GlobalStates
         function onImageBrowserSelected(filePath, actionId) {
             if (!root.activePlugin) return;
             
             // Send selection back to plugin handler
             sendToPlugin({
                 step: "action",
                 selected: {
                     id: "imageBrowser",
                     path: filePath,
                     action: actionId
                 },
                 session: root.activePlugin.session
             });
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
             
             // Send selection back to plugin handler
             sendToPlugin({
                 step: "action",
                 selected: {
                     id: "gridBrowser",
                     itemId: itemId,
                     action: actionId
                 },
                 session: root.activePlugin.session
             });
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
     
     // Polling timer - periodically refreshes plugin results
     Timer {
         id: pollTimer
         interval: root.pollInterval
         running: root.activePlugin !== null && root.pollInterval > 0 && !root.pluginBusy
         repeat: true
         onTriggered: {
             if (root.activePlugin && !root.pluginBusy) {
                 root.isPollUpdate = true;
                 const input = {
                     step: "poll",
                     query: root.lastPollQuery,
                     session: root.activePlugin.session
                 };
                 if (root.lastSelectedItem) {
                     input.selected = { id: root.lastSelectedItem };
                 }
                 if (root.pluginContext) {
                     input.context = root.pluginContext;
                 }
                 sendToPlugin(input);
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
                 
                 try {
                     const response = JSON.parse(output);
                     root.handlePluginResponse(response, wasReplayMode);
                 } catch (e) {
                     root.pluginError = `Failed to parse plugin output: ${e}`;
                     console.warn(`[PluginRunner] Parse error: ${e}, output: ${output}`);
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
     
     // Prepared plugins for fuzzy search
     property var preppedPlugins: plugins
         .filter(w => w.manifest)
         .map(w => ({
             name: Fuzzy.prepare(w.id),
             plugin: w
         }))
     
     // Fuzzy search plugins by name
     function fuzzyQueryPlugins(query) {
         if (!query || query.trim() === "") {
             return root.plugins.filter(w => w.manifest);
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
         
         // Reindex a specific plugin
         // Usage: qs -c hamr ipc call pluginRunner reindex <pluginId>
         function reindex(pluginId: string): void {
             root.indexPlugin(pluginId, "full");
         }
         
         // Reindex all plugins
         // Usage: qs -c hamr ipc call pluginRunner reindexAll
         function reindexAll(): void {
             root.indexAllPlugins();
         }
     }
 }
