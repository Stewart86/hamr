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

    // ==================== LAUNCHER STATE ====================
    property bool launcherOpen: false
    property bool superReleaseMightTrigger: true
    
     // ==================== IMAGE BROWSER ====================
     // Unified image browser that can be opened in standalone or plugin mode
     property bool imageBrowserOpen: false
     property var imageBrowserConfig: null  // { directory, title, extensions, actions, workflowId }
     property bool imageBrowserClosedBySelection: false  // Track if close was due to selection
     
     // Signal emitted when user selects an image in plugin mode
     signal imageBrowserSelected(string filePath, string actionId)
     
     // Open image browser for a plugin
     function openImageBrowserForPlugin(config) {
         imageBrowserConfig = config;
         imageBrowserClosedBySelection = false;
         imageBrowserOpen = true;
     }
    
    // Close image browser (manual close via Escape or click-outside)
    function closeImageBrowser() {
        imageBrowserOpen = false;
        imageBrowserConfig = null;
    }
    
    // Called by ImageBrowserContent when user selects an image
    function imageBrowserSelection(filePath, actionId) {
        imageBrowserSelected(filePath, actionId);
        // Close after selection - mark that it was a selection close
        if (imageBrowserConfig?.workflowId) {
            imageBrowserClosedBySelection = true;
            closeImageBrowser();
        }
    }

    // ==================== WINDOW PICKER ====================
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
