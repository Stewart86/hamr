pragma Singleton
pragma ComponentBehavior: Bound

import QtQuick
import Qt.labs.folderlistmodel
import Quickshell
import Quickshell.Io
import qs.modules.common

/**
 * PluginWatcher - Manages file, directory, and Hyprland event watchers for plugins.
 * 
 * Triggers plugin reindexing when watched resources change.
 * Supports:
 *   - watchFiles: Reindex when specific files change
 *   - watchDirs: Reindex when directory contents change
 *   - watchCompositorEvents: Reindex on compositor IPC events (Hyprland/Niri)
 *   - reindex: Periodic reindexing (e.g., "30s", "5m", "1h")
 */
Singleton {
    id: root

    signal reindexRequested(string pluginId, string mode)

    // Reindex timers per plugin
    property var reindexTimers: ({})
    
    // File watchers per plugin: { pluginId: [FileView, ...] }
    property var fileWatchers: ({})
    
    // Directory watchers per plugin: { pluginId: [FolderListModel, ...] }
    property var dirWatchers: ({})
    
    // Debounce timers for file/dir change events
    property var debounceTimers: ({})
    
    // Compositor event watchers: { pluginId: { events: [...], debounce: ms } }
    property var compositorWatchers: ({})

    // Parse reindex interval from manifest string (e.g., "30s", "5m", "1h", "never")
    function parseReindexInterval(intervalStr: string): int {
        if (!intervalStr || intervalStr === "never") return 0;
        
        const match = intervalStr.match(/^(\d+)(s|m|h)$/);
        if (!match) return 0;
        
        const value = parseInt(match[1], 10);
        const unit = match[2];
        
        switch (unit) {
            case "s": return value * 1000;
            case "m": return value * 60 * 1000;
            case "h": return value * 60 * 60 * 1000;
            default: return 0;
        }
    }
    
    // Expand ~ to home directory
    function expandPath(path: string): string {
        if (path.startsWith("~/")) {
            return Directories.home + path.substring(1);
        }
        return path;
    }

    // Setup all watchers for a plugin based on its manifest
    function setupWatchers(pluginId: string, manifest: var): void {
        if (!manifest?.index) return;
        
        setupReindexTimer(pluginId, manifest);
        setupFileWatchers(pluginId, manifest);
        setupDirWatchers(pluginId, manifest);
        setupCompositorWatcher(pluginId, manifest);
    }

    // Setup periodic reindex timer
    function setupReindexTimer(pluginId: string, manifest: var): void {
        const intervalMs = parseReindexInterval(manifest.index.reindex);
        if (intervalMs <= 0) return;
        
        if (root.reindexTimers[pluginId]) {
            root.reindexTimers[pluginId].destroy();
        }
        
        const timer = Qt.createQmlObject(
            `import QtQuick; Timer { 
                interval: ${intervalMs}; 
                repeat: true; 
                running: true;
                property string targetPluginId: "${pluginId}"
            }`,
            root,
            "reindexTimer_" + pluginId
        );
        
        timer.triggered.connect(() => {
            root.reindexRequested(pluginId, "incremental");
        });
        
        root.reindexTimers[pluginId] = timer;
    }

    // Setup file watchers
    function setupFileWatchers(pluginId: string, manifest: var): void {
        const watchFiles = manifest.index.watchFiles;
        if (!Array.isArray(watchFiles) || watchFiles.length === 0) return;
        if (root.fileWatchers[pluginId]?.length > 0) return;
        
        const watchers = [];
        
        for (const filePath of watchFiles) {
            const expandedPath = root.expandPath(filePath);
            
            const watcher = Qt.createQmlObject(
                `import Quickshell.Io;
                FileView {
                    property string targetPluginId: "${pluginId}"
                    property string watchPath: "${expandedPath}"
                    path: watchPath
                    watchChanges: true
                    onFileChanged: {
                        root.onFileChanged(targetPluginId);
                    }
                    onLoadFailed: error => {
                        if (error === FileViewError.FileNotFound) {
                            console.log("[PluginWatcher] Watched file not found (ignoring): " + watchPath);
                        }
                    }
                }`,
                root,
                "fileWatcher_" + pluginId + "_" + filePath
            );
            
            watchers.push(watcher);
        }
        
        root.fileWatchers[pluginId] = watchers;
        console.log(`[PluginWatcher] Setup ${watchers.length} file watcher(s) for ${pluginId}`);
    }

    // Setup directory watchers
    function setupDirWatchers(pluginId: string, manifest: var): void {
        const watchDirs = manifest.index.watchDirs;
        if (!Array.isArray(watchDirs) || watchDirs.length === 0) return;
        if (root.dirWatchers[pluginId]?.length > 0) return;
        
        const watchers = [];
        
        for (const dirPath of watchDirs) {
            const expandedPath = root.expandPath(dirPath);
            
            const watcher = Qt.createQmlObject(
                `import Qt.labs.folderlistmodel;
                FolderListModel {
                    property string targetPluginId: "${pluginId}"
                    property string watchPath: "${expandedPath}"
                    folder: "file://${expandedPath}"
                    showFiles: true
                    showDirs: false
                    onStatusChanged: {
                        if (status === FolderListModel.Null) {
                            console.log("[PluginWatcher] Watched directory not found (ignoring): " + watchPath);
                        }
                    }
                    onCountChanged: {
                        if (status === FolderListModel.Ready) {
                            root.onDirChanged(targetPluginId);
                        }
                    }
                }`,
                root,
                "dirWatcher_" + pluginId + "_" + dirPath
            );
            
            watchers.push(watcher);
        }
        
        root.dirWatchers[pluginId] = watchers;
        console.log(`[PluginWatcher] Setup ${watchers.length} dir watcher(s) for ${pluginId}`);
    }

    function setupCompositorWatcher(pluginId: string, manifest: var): void {
        // Support both old and new property names
        const events = manifest.index.watchCompositorEvents ?? manifest.index.watchHyprlandEvents;
        if (!Array.isArray(events) || events.length === 0) return;
        if (root.compositorWatchers[pluginId]) return;

        const debounce = manifest.index.debounce ?? 200;
        root.compositorWatchers[pluginId] = {
            events: events,
            debounce: debounce
        };

        console.log(`[PluginWatcher] Setup compositor watcher for ${pluginId}: events=[${events.join(", ")}], debounce=${debounce}ms`);
    }

    // Handle file change - debounced
    function onFileChanged(pluginId: string): void {
        debounceReindex(pluginId, 500, "incremental");
    }

    // Handle directory change - debounced
    function onDirChanged(pluginId: string): void {
        debounceReindex(pluginId, 1000, "incremental");
    }

    function onCompositorEvent(eventName: string): void {
        for (const [pluginId, config] of Object.entries(root.compositorWatchers)) {
            if (!config.events.includes(eventName)) continue;
            debounceReindex(pluginId, config.debounce, "full");
        }
    }

    // Debounced reindex request
    function debounceReindex(pluginId: string, delayMs: int, mode: string): void {
        if (root.debounceTimers[pluginId]) {
            root.debounceTimers[pluginId].restart();
            return;
        }
        
        const timer = Qt.createQmlObject(
            `import QtQuick; Timer {
                interval: ${delayMs};
                repeat: false;
                running: true;
                property string targetPluginId: "${pluginId}"
                property string targetMode: "${mode}"
            }`,
            root,
            "debounce_" + pluginId
        );
        
        timer.triggered.connect(() => {
            root.reindexRequested(pluginId, mode);
            timer.destroy();
            delete root.debounceTimers[pluginId];
        });
        
        root.debounceTimers[pluginId] = timer;
    }

    Connections {
        target: CompositorService
        function onCompositorEvent(eventName, eventData) {
            root.onCompositorEvent(eventName);
        }
    }
}
