pragma Singleton
pragma ComponentBehavior: Bound

import qs
import QtQuick
import Qt.labs.folderlistmodel
import Quickshell
import Quickshell.Io
import qs.modules.common
import qs.modules.common.functions

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

    // ==================== PLUGIN DISCOVERY ====================
    
    // Loaded plugins from both built-in and user plugins directories
    // Each plugin: { id, path, manifest: { name, description, icon, ... }, isBuiltin: bool }
    // User plugins override built-in plugins with the same id
    property var plugins: []
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
                    manifest._handlerPath = manifestLoader.pluginPath + "/handler.py";
                    
                    // Skip if plugin already exists (prevents duplicates from race conditions)
                    if (!root.plugins.some(p => p.id === manifestLoader.pluginId)) {
                        const updated = root.plugins.slice();
                        updated.push({
                            id: manifestLoader.pluginId,
                            path: manifestLoader.pluginPath,
                            manifest: manifest,
                            isBuiltin: manifestLoader.isBuiltin
                        });
                        root.plugins = updated;
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
         
         const input = {
             step: "action",
             selected: { id: itemId },
             session: root.activePlugin.session
         };
         
         if (actionId) {
             input.action = actionId;
         }
         
         sendToPlugin(input);
     }
    
     // Submit form data to active plugin
     function submitForm(formData) {
         if (!root.activePlugin) return;
         
         const input = {
             step: "form",
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
         root.pluginClosed();
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
     function executePluginAction(actionId) {
         if (!root.activePlugin) return;
         
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
         
         // Use bash to pipe input to python - avoids stdinEnabled issues
         pluginProcess.running = false;
         pluginProcess.workingDirectory = root.activePlugin.path;
         const escapedInput = inputJson.replace(/'/g, "'\\''");
         pluginProcess.command = ["bash", "-c", `echo '${escapedInput}' | python3 "${handlerPath}"`];
         pluginProcess.running = true;
     }
    
     function handlePluginResponse(response, wasReplayMode = false) {
         root.pluginBusy = false;
         
         if (!response || !response.type) {
             root.pluginError = "Invalid response from plugin";
             return;
         }
         
         // Update session if provided
         if (response.session && root.activePlugin) {
             root.activePlugin.session = response.session;
         }
         
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
                 // Open image browser with plugin configuration
                 if (response.imageBrowser) {
                     const config = {
                         directory: response.imageBrowser.directory ?? "",
                         title: response.imageBrowser.title ?? root.activePlugin?.manifest?.name ?? "Select Image",
                         extensions: response.imageBrowser.extensions ?? null,
                         actions: response.imageBrowser.actions ?? [],
                         workflowId: root.activePlugin?.id ?? "",
                         enableOcr: response.imageBrowser.enableOcr ?? false
                     };
                     GlobalStates.openImageBrowserForPlugin(config);
                 }
                 break;
                 
             case "error":
                 root.pluginError = response.message ?? "Unknown error";
                 console.warn(`[PluginRunner] Error: ${root.pluginError}`);
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
 }
