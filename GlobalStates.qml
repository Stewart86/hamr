import qs.modules.common
import qs.services
import QtQuick
import Quickshell
import Quickshell.Hyprland
import Quickshell.Io
pragma Singleton
pragma ComponentBehavior: Bound

Singleton {
    id: root

    property bool launcherOpen: false
    property bool launcherMinimized: false
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

    // Floating action hint that appears above all launcher content
    property bool actionHintVisible: false
    property string actionHintKey: ""
    property string actionHintName: ""
    property point actionHintPosition: Qt.point(0, 0)  // Global screen position

    function showActionHint(key, name, globalX, globalY) {
        actionHintKey = key;
        actionHintName = name;
        actionHintPosition = Qt.point(globalX, globalY);
        actionHintVisible = true;
    }

    function hideActionHint() {
        actionHintVisible = false;
    }

    // Window picker for switching between multiple windows of an app
    property bool windowPickerOpen: false
    property string windowPickerAppId: ""
    property var windowPickerWindows: []

    // Signal emitted when user selects a window
    signal windowPickerSelected(var toplevel)

    // Open window picker for an app with multiple windows
    function openWindowPicker(appId, windows) {
        windowPickerAppId = appId;
        windowPickerWindows = windows;
        windowPickerOpen = true;
    }

    // Close window picker
    function closeWindowPicker() {
        windowPickerOpen = false;
        windowPickerAppId = "";
        windowPickerWindows = [];
    }

    // Called when user selects a window
    function windowPickerSelection(toplevel) {
        windowPickerSelected(toplevel);
        closeWindowPicker();
    }
}
