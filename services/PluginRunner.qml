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
    
    
    // Version counter - increments on any result item update to trigger UI re-evaluation
    // SearchItem depends on this to re-evaluate visual properties (gauge, progress, badges, etc.)
    property int resultsVersion: 0
    
     // Replay mode: when true, plugin is running a replay action (no UI needed)
     // Process should complete even if launcher closes
     property bool replayMode: false
     
    // Store plugin info for replay mode (activePlugin may be cleared before response)
    property var replayPluginInfo: null

    // ==================== DAEMON SUPPORT ====================
    // Track running daemon processes: { pluginId: { process, restartCount, isBackground, inputWriter } }
    property var runningDaemons: ({})
    
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

    // ==================== PLUGIN STATUS ====================
    // Plugins can provide dynamic status (badges, description override) that
    // appears on their entry in the main launcher list.
    // Updated via: response.status field, or IPC call "hamr status <plugin> <json>"
    
    // Status per plugin: { pluginId: { badges: [...], description: "..." } }
    // Using a non-reactive internal object to avoid triggering full results rebuild
    property var _pluginStatusesInternal: ({})
    
    // Version counter - increments on any status update to trigger badge refresh
    // SearchItem depends on this to re-evaluate badges without rebuilding results list
    property int statusVersion: 0
    
    function getPluginStatus(pluginId) {
        return root._pluginStatusesInternal[pluginId] ?? null;
    }
    
    function updatePluginStatus(pluginId, status) {
        if (!status || typeof status !== "object") return;
        
        root._pluginStatusesInternal[pluginId] = {
            badges: status.badges ?? [],
            chips: status.chips ?? [],
            description: status.description ?? null
        };
        root.statusVersion++;
    }

    // ==================== PLUGIN INDEXING ====================
    // Plugins provide searchable items in two ways:
    // 1. staticIndex in manifest.json - items loaded directly, no handler needed
    // 2. Daemon plugins emit {"type": "index"} messages autonomously
    
    // Indexed items per plugin: { pluginId: { items: [...], lastIndexed: timestamp } }
    property var pluginIndexes: ({})
    
    // Handle index response from daemon plugin
    function handleIndexResponse(pluginId, response) {
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
        
        // Update plugin status if provided in index response
        if (response.status) {
            root.updatePluginStatus(pluginId, response.status);
        }
        
        // Notify listeners (LauncherSearch) that index changed
        root.pluginIndexChanged(pluginId);
        
        // Save cache to disk (debounced)
        root.saveIndexCache();
    }
    
    // Load static index items from plugin manifests (no handler needed)
    function loadStaticIndexes() {
        for (const plugin of root.plugins) {
            const staticIndex = plugin.manifest?.staticIndex;
            if (!staticIndex || !Array.isArray(staticIndex) || staticIndex.length === 0) {
                continue;
            }
            
            // Enrich items with plugin metadata
            const items = staticIndex.map(item => Object.assign({}, item, {
                _pluginId: plugin.id,
                _pluginName: plugin.manifest?.name ?? plugin.id
            }));
            
            root.pluginIndexes[plugin.id] = {
                items: items,
                lastIndexed: Date.now()
            };
            
            console.log(`[PluginRunner] Loaded ${items.length} static index items from ${plugin.id}`);
            root.pluginIndexChanged(plugin.id);
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

     // ==================== DAEMON LIFECYCLE ====================
     
     // Start a daemon for a plugin (spawn once, keep running)
     function startDaemon(pluginId) {
         const plugin = root.plugins.find(p => p.id === pluginId);
         if (!plugin || !plugin.manifest || !plugin.manifest.daemon?.enabled) {
             return false;
         }
         
         // Already running
         if (root.runningDaemons[pluginId]) {
             return true;
         }
         
         const daemonConfig = plugin.manifest.daemon;
         const handlerPath = plugin.manifest._handlerPath ?? (plugin.path + "/handler.py");
         
         // Create persistent daemon process
         const process = Qt.createQmlObject(`
             import Quickshell.Io
             Process {
                 running: true
                 stdinEnabled: true
                 command: ["${handlerPath}"]
                 workingDirectory: "${plugin.path}"
                 
                 stdout: SplitParser {
                     splitMarker: "\\n"
                     onRead: data => root.handleDaemonStdout("${pluginId}", data)
                 }
                 
                 stderr: SplitParser {
                     onRead: data => console.warn("[Daemon ${pluginId}] stderr:", data)
                 }
                 
                 onExited: (code, status) => root.onDaemonExit("${pluginId}", code, status)
             }
         `, root, "daemon_" + pluginId);
         
         root.runningDaemons[pluginId] = {
             process: process,
             restartCount: 0,
             isBackground: daemonConfig.background ?? false,
             config: daemonConfig,
             plugin: plugin,
             handlerPath: handlerPath,
             session: generateSessionId()
         };
         
         console.log(`[PluginRunner] Started persistent daemon for ${pluginId} (mode: ${daemonConfig.background ? "background" : "foreground"})`);
         return true;
     }
    
     // Stop a daemon process
     function stopDaemon(pluginId) {
         const daemon = root.runningDaemons[pluginId];
         if (!daemon) return false;
         
         if (daemon.process) {
             daemon.process.running = false;
             daemon.process.destroy();
         }
         
         delete root.runningDaemons[pluginId];
         console.log(`[PluginRunner] Stopped daemon for ${pluginId}`);
         return true;
     }
    
    // Start all background daemons (call on hamr startup)
    function startBackgroundDaemons() {
        if (!root.pluginsLoaded) return;
        
        for (const plugin of root.plugins) {
            const daemonConfig = plugin.manifest?.daemon;
            if (daemonConfig?.enabled && daemonConfig?.background) {
                root.startDaemon(plugin.id);
            }
        }
    }
    
    // Stop all running daemons (call on hamr shutdown)
    function stopAllDaemons() {
        const pluginIds = Object.keys(root.runningDaemons);
        for (const pluginId of pluginIds) {
            root.stopDaemon(pluginId);
        }
    }
    
     // Write command to daemon stdin
     function writeToDaemonStdin(pluginId, command) {
         const daemon = root.runningDaemons[pluginId];
         if (!daemon || !daemon.process) {
             console.warn(`[PluginRunner] Cannot write to daemon ${pluginId}: not registered or process missing`);
             return false;
         }
         
         // Preserve session from daemon
         if (daemon.session) {
             command.session = daemon.session;
         }
         
         const json = JSON.stringify(command) + "\n";
         daemon.process.write(json);
         console.log(`[PluginRunner] Sent to daemon ${pluginId}: ${command.step}`);
         return true;
     }
    
     // Parse daemon stdout line and emit response
     function handleDaemonStdout(pluginId, data) {
         if (!data || data.trim() === "") return;
         
         try {
             const response = JSON.parse(data.trim());
             root.handleDaemonOutput(pluginId, response);
         } catch (e) {
             console.warn(`[PluginRunner] Failed to parse daemon output from ${pluginId}: ${e}`);
         }
     }
     
     // Handle daemon output/response
      function handleDaemonOutput(pluginId, response) {
         // Only process if this plugin is currently active
         const isActive = root.activePlugin?.id === pluginId;
         
         if (!response || !response.type) {
             console.warn(`[PluginRunner] Invalid daemon output from ${pluginId}`);
             return;
         }
         
         console.log(`[PluginRunner] Daemon output from ${pluginId}: ${response.type}`);
         
           switch (response.type) {
               case "results":
               case "card":
               case "form":
               case "prompt":
               case "error":
               case "imageBrowser":
               case "gridBrowser":
               case "update":
                   // Only process UI responses if plugin is active
                       if (isActive) {
                           // Update status if provided (before handlePluginResponse)
                           if (response.status) {
                               root.updatePluginStatus(pluginId, response.status);
                           }
                           root.handlePluginResponse(response);
                       }
                   break;
              
              case "status":
                  // Status updates always processed
                  root.updatePluginStatus(pluginId, response.status);
                  break;
              
              case "index":
                  // Index updates always processed
                  root.handleIndexResponse(pluginId, response);
                  break;
              
              case "execute":
                  // Execute responses always processed
                  if (isActive) {
                      root.handlePluginResponse(response);
                  }
                  break;
              
              default:
                  console.warn(`[PluginRunner] Unknown daemon response type: ${response.type}`);
          }
     }
    
    // Handle daemon crash/exit (for future use when full daemon communication is implemented)
    function onDaemonExit(pluginId, exitCode, exitStatus) {
        const daemon = root.runningDaemons[pluginId];
        if (!daemon) return;
        
        const config = daemon.config;
        const shouldRestart = config.restartOnCrash && 
                            daemon.restartCount < (config.maxRestarts ?? 3);
        
        if (shouldRestart) {
            daemon.restartCount++;
            console.log(`[PluginRunner] Restarting daemon for ${pluginId} (attempt ${daemon.restartCount}/${config.maxRestarts})`);
            
            // Reset restart count after successful run (when daemon runs for a while)
            daemon.process = null;
            
            // Restart after a short delay
            Qt.callLater(() => {
                root.startDaemon(pluginId);
            }, 1000);
        } else {
            console.log(`[PluginRunner] Daemon for ${pluginId} won't restart (maxRestarts reached or disabled)`);
            delete root.runningDaemons[pluginId];
            
            // Notify user if plugin was active
            if (root.activePlugin?.id === pluginId) {
                root.pluginError = "Plugin daemon crashed and won't restart";
            }
        }
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
            
            // Start background daemons after plugins are loaded
            root.startBackgroundDaemons();
            
            // Start pending plugin if one was requested before loading finished
            if (root.pendingPluginStart !== "") {
                const pluginId = root.pendingPluginStart;
                root.pendingPluginStart = "";
                root.startPlugin(pluginId);
            }
            // Load static index items from manifests (no handler needed)
            root.loadStaticIndexes();
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
          
          // For daemon plugins, start daemon if not already running
          const daemonConfig = plugin.manifest.daemon;
          if (daemonConfig?.enabled) {
              root.startDaemon(pluginId);
              // Send initial step through daemon
              root.writeToDaemonStdin(pluginId, { step: "initial", session: session });
          } else {
              // Use request-response model for non-daemon plugins
              sendToPlugin({ step: "initial", session: session });
          }
          
          return true;
      }
    
      // Send search query to active plugin
      function search(query) {
          if (!root.activePlugin) {
              console.log("[PluginRunner] sendToPlugin: No active plugin");
              return;
          }
          
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
          
          // Use daemon if running, otherwise spawn new process
          const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
          if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
              root.pluginBusy = true;
              root.writeToDaemonStdin(root.activePlugin.id, input);
          } else {
              sendToPlugin(input);
          }
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
           
           // Use daemon if running, otherwise spawn new process
           const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
           if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
               root.pluginBusy = true;
               root.writeToDaemonStdin(root.activePlugin.id, input);
           } else {
               sendToPlugin(input);
           }
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
           
           const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
           if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
               root.writeToDaemonStdin(root.activePlugin.id, input);
           } else {
               sendToPlugin(input);
           }
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
           
           const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
           if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
               root.writeToDaemonStdin(root.activePlugin.id, input);
           } else {
               sendToPlugin(input);
           }
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
           
           const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
           if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
               root.writeToDaemonStdin(root.activePlugin.id, input);
           } else {
               sendToPlugin(input);
           }
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
          
          const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
          if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
              root.pluginBusy = true;
              root.writeToDaemonStdin(root.activePlugin.id, input);
          } else {
              sendToPlugin(input);
          }
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
          
          const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
          if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
              root.pluginBusy = true;
              root.writeToDaemonStdin(root.activePlugin.id, input);
          } else {
              sendToPlugin(input);
          }
      }
    
       // Close active plugin
       // If in replay mode, let the process finish (notification needs to be sent)
       // For non-background daemons, stop the daemon
       function closePlugin() {
           const pluginId = root.activePlugin?.id;
           
           // In replay mode, don't kill the process - let it complete for notification
           if (!root.replayMode) {
               pluginProcess.running = false;
           }
           
           // Stop daemon if it's not a background daemon
           if (pluginId) {
               const daemon = root.runningDaemons[pluginId];
               const isDaemonPlugin = root.activePlugin?.manifest?.daemon?.enabled;
               const isBackground = daemon?.isBackground ?? false;
               
               if (isDaemonPlugin && !isBackground) {
                   root.stopDaemon(pluginId);
               }
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
           root.pluginActions = [];
           root.navigationDepth = 0;
           root.pendingNavigation = false;
           root.pendingBack = false;
           root.pluginClosed();
       }
       
        // Patch individual items in pluginResults without replacing the array
        // This is used for incremental updates (e.g., slider adjustments)
        function patchPluginResults(patches) {
            if (!patches || !Array.isArray(patches)) return;
            
            // Create a map of patches by id for efficient lookup
            const patchMap = new Map();
            for (const patch of patches) {
                if (patch.id) {
                    patchMap.set(patch.id, patch);
                }
            }
            
            // Create a new array with patched items
            const updated = root.pluginResults.map(item => {
                const patch = patchMap.get(item.id);
                if (patch) {
                    // Return a new object with merged properties
                    return Object.assign({}, item, patch);
                }
                return item;
            });
            
            // Replace the array to trigger update
            root.pluginResults = updated;
            
            // Increment version counter for additional reactivity
            root.resultsVersion++;
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
          
          const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
          if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
              root.pluginBusy = true;
              root.writeToDaemonStdin(root.activePlugin.id, input);
          } else {
              sendToPlugin(input);
          }
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
          
          const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
          if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
              root.pluginBusy = true;
              root.writeToDaemonStdin(root.activePlugin.id, input);
          } else {
              sendToPlugin(input);
          }
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
          
          // For daemon plugins in replay mode, still use request-response
          // (replay shouldn't leave daemon running)
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
          
          // For daemon plugins, start daemon and use daemon
          const isDaemonPlugin = plugin.manifest?.daemon?.enabled;
          if (isDaemonPlugin) {
              root.startDaemon(pluginId);
              root.writeToDaemonStdin(pluginId, input);
          } else {
              sendToPlugin(input);
          }
          
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
               case "update":
                    // Patch individual items in pluginResults without replacing the array
                    if (response.items && Array.isArray(response.items)) {
                        root.patchPluginResults(response.items);
                    }
                    break;
                    
               case "results":
                    root.pluginResults = response.results ?? [];
                    root.resultsVersion++;
                    root.pluginCard = null;
                    root.pluginForm = null;
                    if (response.placeholder !== undefined) {
                        root.pluginPlaceholder = response.placeholder ?? "";
                    }
                    if (response.context !== undefined) {
                        root.pluginContext = response.context ?? "";
                    }
                    root.inputMode = response.inputMode ?? "realtime";
                    if (response.pluginActions !== undefined) {
                        root.pluginActions = response.pluginActions ?? [];
                    }
                    if (response.clearInput) {
                        root.clearInputRequested();
                    }
                    // Update plugin status if provided
                    if (response.status && root.activePlugin?.id) {
                        root.updatePluginStatus(root.activePlugin.id, response.status);
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
                      if (exec.sound) {
                          AudioService.playSound(exec.sound);
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
             
             const input = {
                 step: "action",
                 selected: {
                     id: "imageBrowser",
                     path: filePath,
                     action: actionId
                 },
                 session: root.activePlugin.session
             };
             
             const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
             if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
                 root.pluginBusy = true;
                 root.writeToDaemonStdin(root.activePlugin.id, input);
             } else {
                 sendToPlugin(input);
             }
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
             
             const input = {
                 step: "action",
                 selected: {
                     id: "gridBrowser",
                     itemId: itemId,
                     action: actionId
                 },
                 session: root.activePlugin.session
             };
             
             const isDaemonPlugin = root.activePlugin.manifest?.daemon?.enabled;
             if (isDaemonPlugin && root.runningDaemons[root.activePlugin.id]) {
                 root.pluginBusy = true;
                 root.writeToDaemonStdin(root.activePlugin.id, input);
             } else {
                 sendToPlugin(input);
             }
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
     // Exclude index-only plugins (staticIndex or indexOnly) - they provide indexed items but can't be opened
     property var preppedPlugins: plugins
         .filter(w => w.manifest && !w.manifest.staticIndex && !w.manifest.indexOnly)
         .map(w => ({
             name: Fuzzy.prepare(w.id),
             plugin: w
         }))
     
     // Fuzzy search plugins by name
     // Excludes index-only plugins (staticIndex or indexOnly) - they can't be opened
     function fuzzyQueryPlugins(query) {
         if (!query || query.trim() === "") {
             return root.plugins.filter(w => w.manifest && !w.manifest.staticIndex && !w.manifest.indexOnly);
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
         
         // Update plugin status (badges, description)
         // Usage: hamr status <pluginId> '<json>'
         // Example: hamr status todo '{"badges": [{"text": "5"}]}'
         function updateStatus(pluginId: string, statusJson: string): void {
             try {
                 const status = JSON.parse(statusJson);
                 root.updatePluginStatus(pluginId, status);
             } catch (e) {
                 console.warn(`[PluginRunner] Failed to parse status JSON: ${e}`);
             }
         }
     }
 }
