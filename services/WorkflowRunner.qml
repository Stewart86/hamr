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
 * WorkflowRunner - Multi-step action workflow execution service
 * 
 * Manages bidirectional JSON communication with workflow handler scripts.
 * Workflows are folders in ~/.config/hamr/actions/ containing:
 *   - manifest.json: Workflow metadata and configuration
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

    // ==================== ACTIVE WORKFLOW STATE ====================
    property var activeWorkflow: null  // { id, path, manifest, session }
    property var workflowResults: []   // Current results from workflow
    property var workflowCard: null    // Card to display (title, content, markdown)
    property string workflowPrompt: "" // Custom prompt text
    property string workflowPlaceholder: "" // Custom placeholder text for search bar
    property bool workflowBusy: false  // True while waiting for script response
    property string workflowError: ""  // Last error message
    property var lastSelectedItem: null // Last selected item (persisted across search calls)
    property string workflowContext: ""  // Custom context string for multi-step flows
    
    // Input mode: "realtime" (every keystroke) or "submit" (only on Enter)
    // Handler controls this via response - allows different modes per step
    property string inputMode: "realtime"
    
    
    // Replay mode: when true, workflow is running a replay action (no UI needed)
    // Process should complete even if launcher closes
    property bool replayMode: false
    
    // Store workflow info for replay mode (activeWorkflow may be cleared before response)
    property var replayWorkflowInfo: null

    // Signal when workflow produces results
    signal resultsReady(var results)
    signal cardReady(var card)
    signal executeCommand(var command)
    signal workflowClosed()
    signal clearInputRequested()  // Signal to clear the search input
    
    // Signal when a trackable action is executed (has name field)
    // Payload: { name, command, entryPoint, icon, thumbnail, workflowId, workflowName }
    // - command: Direct shell command for simple replay (optional)
    // - entryPoint: Workflow step to replay for complex actions (optional)
    signal actionExecuted(var actionInfo)

    // ==================== WORKFLOW DISCOVERY ====================
    
    // Loaded workflows from actions/ directory
    // Each workflow: { id, path, manifest: { name, description, icon, ... } }
    property var workflows: []
    property var pendingManifestLoads: []
    
    // Force refresh workflows - call this when launcher opens to detect new workflows
    // This works around FolderListModel not detecting changes in symlinked directories
    function refreshWorkflows() {
        // Touch the folder property to force FolderListModel to re-scan
        const currentFolder = workflowsFolder.folder;
        workflowsFolder.folder = "";
        workflowsFolder.folder = currentFolder;
    }
    
    // Load workflows when directory changes
    function loadWorkflows() {
        root.pendingManifestLoads = [];
        
        for (let i = 0; i < workflowsFolder.count; i++) {
            const fileName = workflowsFolder.get(i, "fileName");
            const filePath = workflowsFolder.get(i, "filePath");
            if (fileName && filePath) {
                const dirPath = filePath.toString().replace("file://", "");
                root.pendingManifestLoads.push({
                    id: fileName,
                    path: dirPath
                });
            }
        }
        
        root.workflows = [];
        
        if (root.pendingManifestLoads.length > 0) {
            loadNextManifest();
        }
    }
    
    function loadNextManifest() {
        if (root.pendingManifestLoads.length === 0) {
            return;
        }
        
        const workflow = root.pendingManifestLoads.shift();
        manifestLoader.workflowId = workflow.id;
        manifestLoader.workflowPath = workflow.path;
        manifestLoader.command = ["cat", workflow.path + "/manifest.json"];
        manifestLoader.running = true;
    }
    
    Process {
        id: manifestLoader
        property string workflowId: ""
        property string workflowPath: ""
        property string outputBuffer: ""
        
        stdout: SplitParser {
            splitMarker: ""
            onRead: data => {
                manifestLoader.outputBuffer += data;
            }
        }
        
        onExited: (exitCode, exitStatus) => {
            if (exitCode === 0 && manifestLoader.outputBuffer.trim()) {
                try {
                    const manifest = JSON.parse(manifestLoader.outputBuffer.trim());
                    manifest._handlerPath = manifestLoader.workflowPath + "/handler.py";
                    
                    const updated = root.workflows.slice();
                    updated.push({
                        id: manifestLoader.workflowId,
                        path: manifestLoader.workflowPath,
                        manifest: manifest
                    });
                    root.workflows = updated;
                } catch (e) {
                    console.warn(`[WorkflowRunner] Failed to parse manifest for ${manifestLoader.workflowId}:`, e);
                }
            }
            
            manifestLoader.outputBuffer = "";
            root.loadNextManifest();
        }
    }
    
    // Watch for workflow folders
    FolderListModel {
        id: workflowsFolder
        folder: `file://${Directories.userActions}`
        showDirs: true
        showFiles: false
        showHidden: false
        sortField: FolderListModel.Name
        onCountChanged: root.loadWorkflows()
        onStatusChanged: {
            if (status === FolderListModel.Ready) {
                root.loadWorkflows();
            }
        }
    }
    


    // ==================== WORKFLOW EXECUTION ====================
    
    // Start a workflow
    function startWorkflow(workflowId) {
        const workflow = root.workflows.find(w => w.id === workflowId);
        if (!workflow || !workflow.manifest) {
            return false;
        }
        
        const session = generateSessionId();
        
        root.activeWorkflow = {
            id: workflow.id,
            path: workflow.path,
            manifest: workflow.manifest,
            session: session
        };
        root.workflowResults = [];
        root.workflowCard = null;
        root.workflowPrompt = workflow.manifest.steps?.initial?.prompt ?? "";
        root.workflowPlaceholder = "";  // Reset placeholder on workflow start
        root.workflowError = "";
        root.inputMode = "realtime";  // Default to realtime, handler can change
        
        sendToWorkflow({ step: "initial", session: session });
        return true;
    }
    
    // Send search query to active workflow
    function search(query) {
        if (!root.activeWorkflow) {
            console.log("[WorkflowRunner] sendToWorkflow: No active workflow");
            return;
        }
        
        // Don't clear card here - it should persist until new response arrives
        
        const input = {
            step: "search",
            query: query,
            session: root.activeWorkflow.session
        };
        
        // Include last selected item for context (useful for multi-step workflows)
        if (root.lastSelectedItem) {
            input.selected = { id: root.lastSelectedItem };
        }
        
        // Include workflow context if set (for multi-step flows like search mode, edit mode)
        if (root.workflowContext) {
            input.context = root.workflowContext;
        }
        
        sendToWorkflow(input);
    }
    
    // Select an item and optionally execute an action
    function selectItem(itemId, actionId) {
        if (!root.activeWorkflow) return;
        
        // Store selection for context in subsequent search calls
        root.lastSelectedItem = itemId;
        
        const input = {
            step: "action",
            selected: { id: itemId },
            session: root.activeWorkflow.session
        };
        
        if (actionId) {
            input.action = actionId;
        }
        
        sendToWorkflow(input);
    }
    
    // Close active workflow
    // If in replay mode, let the process finish (notification needs to be sent)
    function closeWorkflow() {
        // In replay mode, don't kill the process - let it complete for notification
        if (!root.replayMode) {
            workflowProcess.running = false;
        }
        root.activeWorkflow = null;
        root.workflowResults = [];
        root.workflowCard = null;
        root.workflowPrompt = "";
        root.workflowPlaceholder = "";
        root.lastSelectedItem = null;
        root.workflowContext = "";
        root.workflowError = "";
        root.workflowBusy = false;
        root.inputMode = "realtime";
        root.workflowClosed();
    }
    
    // Check if a workflow is active
    function isActive() {
        return root.activeWorkflow !== null;
    }
    
    // Get workflow by ID
    function getWorkflow(id) {
        return root.workflows.find(w => w.id === id) ?? null;
    }
    
    // Replay a saved action using entryPoint
    // Used for history items that need workflow logic instead of direct command
    // Returns true if replay was initiated, false if workflow not found
    function replayAction(workflowId, entryPoint) {
        const workflow = root.workflows.find(w => w.id === workflowId);
        if (!workflow || !workflow.manifest || !entryPoint) {
            return false;
        }
        
        const session = generateSessionId();
        
        root.activeWorkflow = {
            id: workflow.id,
            path: workflow.path,
            manifest: workflow.manifest,
            session: session
        };
        root.workflowResults = [];
        root.workflowCard = null;
        root.workflowPrompt = "";
        root.workflowPlaceholder = "";
        root.workflowError = "";
        root.inputMode = "realtime";
        root.replayMode = true;  // Don't kill process when launcher closes
        root.replayWorkflowInfo = {
            id: workflow.id,
            name: workflow.manifest.name,
            icon: workflow.manifest.icon
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
        
        sendToWorkflow(input);
        return true;
    }
    
    // ==================== INTERNAL ====================
    
    function generateSessionId() {
        return Date.now().toString(36) + Math.random().toString(36).substr(2, 9);
    }
    
    function sendToWorkflow(input) {
        if (!root.activeWorkflow) return;
        
        root.workflowBusy = true;
        root.workflowError = "";
        
        const handlerPath = root.activeWorkflow.manifest._handlerPath 
            ?? (root.activeWorkflow.path + "/handler.py");
        
        const inputJson = JSON.stringify(input);
        
        // Use bash to pipe input to python - avoids stdinEnabled issues
        workflowProcess.running = false;
        workflowProcess.workingDirectory = root.activeWorkflow.path;
        const escapedInput = inputJson.replace(/'/g, "'\\''");
        workflowProcess.command = ["bash", "-c", `echo '${escapedInput}' | python3 "${handlerPath}"`];
        workflowProcess.running = true;
    }
    
    function handleWorkflowResponse(response, wasReplayMode = false) {
        root.workflowBusy = false;
        
        if (!response || !response.type) {
            root.workflowError = "Invalid response from workflow";
            return;
        }
        
        // Update session if provided
        if (response.session && root.activeWorkflow) {
            root.activeWorkflow.session = response.session;
        }
        
        switch (response.type) {
            case "results":
                root.workflowResults = response.results ?? [];
                root.workflowCard = null;
                if (response.placeholder !== undefined) {
                    root.workflowPlaceholder = response.placeholder ?? "";
                }
                // Allow handler to set the context for subsequent search calls
                if (response.context !== undefined) {
                    root.workflowContext = response.context ?? "";
                }
                // Set input mode from response (defaults to realtime)
                root.inputMode = response.inputMode ?? "realtime";
                if (response.clearInput) {
                    root.clearInputRequested();
                }
                root.resultsReady(root.workflowResults);
                break;
                
            case "card":
                root.workflowCard = response.card ?? null;
                if (response.placeholder !== undefined) {
                    root.workflowPlaceholder = response.placeholder ?? "";
                }
                // Set input mode from response (defaults to realtime)
                root.inputMode = response.inputMode ?? "realtime";
                if (response.clearInput) {
                    root.clearInputRequested();
                }
                root.cardReady(root.workflowCard);
                break;
                
            case "execute":
                if (response.execute) {
                    const exec = response.execute;
                    
                    // In replay mode, activeWorkflow may be cleared - use replayWorkflowInfo
                    const workflowName = root.activeWorkflow?.manifest?.name 
                        ?? root.replayWorkflowInfo?.name 
                        ?? "Workflow";
                    const workflowIcon = root.activeWorkflow?.manifest?.icon 
                        ?? root.replayWorkflowInfo?.icon 
                        ?? "play_arrow";
                    const workflowId = root.activeWorkflow?.id 
                        ?? root.replayWorkflowInfo?.id 
                        ?? "";
                    
                    if (exec.command) {
                        Quickshell.execDetached(exec.command);
                    }
                    if (exec.notify) {
                        Quickshell.execDetached(["notify-send", workflowName, exec.notify, "-a", "Shell"]);
                    }
                    // If handler provides name, emit for history tracking
                    // Include entryPoint for complex actions that need workflow replay
                    if (exec.name) {
                        root.actionExecuted({
                            name: exec.name,
                            command: exec.command ?? [],
                            entryPoint: exec.entryPoint ?? null,  // For workflow replay
                            icon: exec.icon ?? workflowIcon,
                            thumbnail: exec.thumbnail ?? "",
                            workflowId: workflowId,
                            workflowName: workflowName
                        });
                    }
                    if (exec.close) {
                        root.executeCommand(exec);
                    }
                    
                    // Clear replay info after use
                    if (wasReplayMode) {
                        root.replayWorkflowInfo = null;
                    }
                }
                break;
                
            case "prompt":
                if (response.prompt) {
                    root.workflowPrompt = response.prompt.text ?? "";
                    // preserve_input handled by caller
                }
                // Card might also be sent with prompt (for LLM responses)
                if (response.card) {
                    root.workflowCard = response.card;
                    root.cardReady(root.workflowCard);
                }
                break;
                
            case "imageBrowser":
                // Open image browser with workflow configuration
                if (response.imageBrowser) {
                    const config = {
                        directory: response.imageBrowser.directory ?? "",
                        title: response.imageBrowser.title ?? root.activeWorkflow?.manifest?.name ?? "Select Image",
                        extensions: response.imageBrowser.extensions ?? null,
                        actions: response.imageBrowser.actions ?? [],
                        workflowId: root.activeWorkflow?.id ?? ""
                    };
                    GlobalStates.openImageBrowserForWorkflow(config);
                }
                break;
                
            case "error":
                root.workflowError = response.message ?? "Unknown error";
                console.warn(`[WorkflowRunner] Error: ${root.workflowError}`);
                break;
                
            default:
                console.warn(`[WorkflowRunner] Unknown response type: ${response.type}`);
        }
    }
    
    // Handle image browser selection - send back to workflow
    Connections {
        target: GlobalStates
        function onImageBrowserSelected(filePath, actionId) {
            if (!root.activeWorkflow) return;
            
            // Send selection back to workflow handler
            sendToWorkflow({
                step: "action",
                selected: {
                    id: "imageBrowser",
                    path: filePath,
                    action: actionId
                },
                session: root.activeWorkflow.session
            });
        }
    }
    
    // Process for running workflow handler
    Process {
        id: workflowProcess
        
        stdout: StdioCollector {
            id: workflowStdout
            onStreamFinished: {
                root.workflowBusy = false;
                const wasReplayMode = root.replayMode;
                root.replayMode = false;  // Reset replay mode after process completes
                
                const output = workflowStdout.text.trim();
                if (!output) {
                    root.workflowError = "No output from workflow";
                    return;
                }
                
                try {
                    const response = JSON.parse(output);
                    root.handleWorkflowResponse(response, wasReplayMode);
                } catch (e) {
                    root.workflowError = `Failed to parse workflow output: ${e}`;
                    console.warn(`[WorkflowRunner] Parse error: ${e}, output: ${output}`);
                }
            }
        }
        
        stderr: SplitParser {
            onRead: data => console.warn(`[WorkflowRunner] stderr: ${data}`)
        }
        
        onExited: (exitCode, exitStatus) => {
            root.replayMode = false;
            root.replayWorkflowInfo = null;
            if (exitCode !== 0) {
                root.workflowBusy = false;
                root.workflowError = `Workflow exited with code ${exitCode}`;
            }
        }
    }
    
    // Prepared workflows for fuzzy search
    property var preppedWorkflows: workflows
        .filter(w => w.manifest)
        .map(w => ({
            name: Fuzzy.prepare(w.id),
            workflow: w
        }))
    
    // Fuzzy search workflows by name
    function fuzzyQueryWorkflows(query) {
        if (!query || query.trim() === "") {
            return root.workflows.filter(w => w.manifest);
        }
        return Fuzzy.go(query, root.preppedWorkflows, { key: "name", limit: 10 })
            .map(r => r.obj.workflow);
    }
}
