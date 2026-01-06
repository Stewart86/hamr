import qs.modules.common
import qs.services
import QtQuick
import Quickshell
import Quickshell.Io
pragma Singleton
pragma ComponentBehavior: Bound

Singleton {
    id: root

    property bool launcherOpen: false
    onLauncherOpenChanged: {
        if (!launcherOpen) {
            // Trigger GC when launcher closes to free temporary objects
            gcTimer.restart();
        }
    }
    
    Timer {
        id: gcTimer
        interval: 500
        onTriggered: gc()
    }
    
    property bool launcherMinimized: Persistent.states.launcher.minimized
    onLauncherMinimizedChanged: Persistent.states.launcher.minimized = launcherMinimized
    property bool superReleaseMightTrigger: true
    
    // Soft close: click-outside (preserves state for restore window)
    // Hard close: Escape, execute-with-close, IPC close (immediate cleanup)
    property bool softClose: false
    
    property bool imageBrowserOpen: false
    property var imageBrowserConfig: null
    property bool imageBrowserClosedBySelection: false
    property bool imageBrowserClosedByCancel: false
    
    signal imageBrowserSelected(string filePath, string actionId)
    signal imageBrowserCancelled()
    
    function openImageBrowserForPlugin(config) {
        imageBrowserConfig = config;
        imageBrowserClosedBySelection = false;
        imageBrowserClosedByCancel = false;
        imageBrowserOpen = true;
    }
    
    function closeImageBrowser() {
        imageBrowserOpen = false;
        imageBrowserConfig = null;
    }
    
    function cancelImageBrowser() {
        imageBrowserClosedByCancel = true;
        imageBrowserCancelled();
        imageBrowserOpen = false;
        imageBrowserConfig = null;
    }
    
    function imageBrowserSelection(filePath, actionId) {
        imageBrowserSelected(filePath, actionId);
        if (imageBrowserConfig?.workflowId) {
            imageBrowserClosedBySelection = true;
            closeImageBrowser();
        }
    }

    property bool gridBrowserOpen: false
    property var gridBrowserConfig: null
    property bool gridBrowserClosedBySelection: false
    property bool gridBrowserClosedByCancel: false
    
    signal gridBrowserSelected(string itemId, string actionId)
    signal gridBrowserCancelled()
    
    function openGridBrowserForPlugin(config) {
        gridBrowserConfig = config;
        gridBrowserClosedBySelection = false;
        gridBrowserClosedByCancel = false;
        gridBrowserOpen = true;
    }
    
    function closeGridBrowser() {
        gridBrowserOpen = false;
        gridBrowserConfig = null;
    }
    
    function cancelGridBrowser() {
        gridBrowserClosedByCancel = true;
        gridBrowserCancelled();
        gridBrowserOpen = false;
        gridBrowserConfig = null;
    }
    
    function gridBrowserSelection(itemId, actionId) {
        gridBrowserSelected(itemId, actionId);
        if (gridBrowserConfig?.workflowId) {
            gridBrowserClosedBySelection = true;
            closeGridBrowser();
        }
    }

    // Floating action tooltip that appears above all launcher content
    property bool actionToolTipVisible: false
    property string actionToolTipKey: ""
    property string actionToolTipName: ""
    property point actionToolTipPosition: Qt.point(0, 0)  // Global screen position

    function showActionToolTip(key, name, globalX, globalY) {
        actionToolTipKey = key;
        actionToolTipName = name;
        actionToolTipPosition = Qt.point(globalX, globalY);
        actionToolTipVisible = true;
    }

    function hideActionToolTip() {
        actionToolTipVisible = false;
    }

    // Window picker for switching between multiple windows of an app
    property bool windowPickerOpen: false
    property string windowPickerAppId: ""
    property string windowPickerItemId: ""  // Full item ID for frecency recording
    property var windowPickerWindows: []

    // Signal emitted when user selects a window
    signal windowPickerSelected(var toplevel)

    // Open window picker for an app with multiple windows
    function openWindowPicker(appId, windows, itemId) {
        windowPickerAppId = appId;
        windowPickerItemId = itemId ?? "";
        windowPickerWindows = windows;
        windowPickerOpen = true;
    }

    // Close window picker
    function closeWindowPicker() {
        windowPickerOpen = false;
        windowPickerAppId = "";
        windowPickerItemId = "";
        windowPickerWindows = [];
    }

    // Called when user selects a window
    function windowPickerSelection(toplevel) {
        windowPickerSelected(toplevel);
        closeWindowPicker();
    }

    // Preview panel state
    property var previewItem: null
    property bool previewPanelVisible: previewItem !== null && previewItem.preview !== undefined
    
    // Detached preview panels (list of preview data objects that are pinned)
    property var detachedPreviews: []
    property bool _detachedPreviewsLoaded: false
    onDetachedPreviewsChanged: {
        if (_detachedPreviewsLoaded && Persistent.ready) {
            Persistent.states.launcher.detachedPreviews = detachedPreviews;
        }
    }
    
    Connections {
        target: Persistent
        function onReadyChanged() {
            if (Persistent.ready && !root._detachedPreviewsLoaded) {
                root.detachedPreviews = Persistent.states.launcher.detachedPreviews ?? [];
                root._detachedPreviewsLoaded = true;
            }
        }
    }
    
    // Signal when a preview is detached
    signal previewDetached(var previewData, real x, real y)
    
    function setPreviewItem(item) {
        previewItem = item;
    }
    
    function clearPreviewItem() {
        previewItem = null;
    }
    
    function detachCurrentPreview(screenX, screenY) {
        if (previewItem && previewItem.preview) {
            const detachedData = {
                id: Date.now().toString(),
                preview: previewItem.preview,
                name: previewItem.name ?? "",
                x: screenX,
                y: screenY
            };
            const newList = detachedPreviews.slice();
            newList.push(detachedData);
            detachedPreviews = newList;
            previewDetached(detachedData, screenX, screenY);
            return detachedData;
        }
        return null;
    }
    
    function closeDetachedPreview(id) {
        detachedPreviews = detachedPreviews.filter(p => p.id !== id);
    }
    
    function clearAllDetachedPreviews() {
        detachedPreviews = [];
    }
    
    function updateDetachedPreviewPosition(id, newX, newY) {
        detachedPreviews = detachedPreviews.map(p => {
            if (p.id === id) {
                return {
                    id: p.id,
                    preview: p.preview,
                    name: p.name,
                    x: newX,
                    y: newY
                };
            }
            return p;
        });
    }

    // FAB override from plugins
    // Structure: { pluginId: string, chips: [], badges: [], priority: number }
    // Higher priority wins when multiple plugins compete
    property var fabOverride: null
    
    // Internal storage for all plugin FAB overrides: { pluginId: { chips, badges, priority } }
    property var _fabOverrides: ({})
    
    function updateFabOverride(pluginId, fabData) {
        if (!fabData || typeof fabData !== "object") {
            // Clear this plugin's override
            if (_fabOverrides[pluginId]) {
                delete _fabOverrides[pluginId];
                _fabOverrides = Object.assign({}, _fabOverrides);
                _resolveFabOverride();
            }
            return;
        }
        
        _fabOverrides[pluginId] = {
            pluginId: pluginId,
            chips: fabData.chips ?? [],
            badges: fabData.badges ?? [],
            priority: fabData.priority ?? 0
        };
        _fabOverrides = Object.assign({}, _fabOverrides);
        _resolveFabOverride();
        
        // If showFab is true and launcher is closed, show the FAB
        if (fabData.showFab && !root.launcherOpen && !root.launcherMinimized) {
            root.launcherMinimized = true;
        }
    }
    
    function clearFabOverride(pluginId) {
        if (_fabOverrides[pluginId]) {
            delete _fabOverrides[pluginId];
            _fabOverrides = Object.assign({}, _fabOverrides);
            _resolveFabOverride();
        }
    }
    
    function _resolveFabOverride() {
        const overrides = Object.values(_fabOverrides);
        if (overrides.length === 0) {
            fabOverride = null;
            return;
        }
        
        // Find highest priority override
        let best = overrides[0];
        for (const o of overrides) {
            if (o.priority > best.priority) {
                best = o;
            }
        }
        fabOverride = best;
    }
}
