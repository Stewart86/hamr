import QtQuick
import Quickshell

QtObject {
    enum IconType { Material, Text, System, None }
    enum FontType { Normal, Monospace }
    enum ResultType { Standard, PluginEntry, PluginResult, Card }

    // Unique key for ScriptModel identity (prevents flicker on updates)
    property string key: id || name || ""
    
    // General stuff
    property string type: ""
    property var fontType: LauncherSearchResult.FontType.Normal
    property string name: ""
    property string rawValue: ""
    property string iconName: ""
    property var iconType: LauncherSearchResult.IconType.None
    property string verb: ""
    property bool blurImage: false
    property var execute: () => {
        print("Not implemented");
    }
    property var actions: []
    
    // Tab completion support
    property bool acceptsArguments: false  // True for quicklinks, actions that take args
    property string completionText: ""     // Text to complete to (e.g., "github " for quicklink)
    
    // Stuff needed for DesktopEntry 
    property string id: ""
    property bool shown: true
    property string comment: ""
    property bool runInTerminal: false
    property string genericName: ""
    property list<string> keywords: []

    // Extra stuff to allow for more flexibility
    property string category: type
    
     // ==================== PLUGIN SUPPORT ====================
     // Result type for different rendering modes
     property var resultType: LauncherSearchResult.ResultType.Standard
     
     // Plugin identification (for plugin results)
     property string pluginId: ""      // ID of the plugin this result belongs to
     property string pluginItemId: ""  // ID of the item within plugin results
     
     // Card display (for ResultType.Card)
     property string cardTitle: ""
     property string cardContent: ""
     property bool cardMarkdown: false
     
     // Plugin actions (from plugin result's actions array)
     // Each action: { id, name, icon }
     property var pluginActions: []
    
     // Thumbnail image path (for workflow results with images)
     property string thumbnail: ""
     
     // Keep launcher open after execution (for indexed items that show UI)
     property bool keepOpen: false
     
     // ==================== SMART SUGGESTIONS ====================
     // Whether this item is a smart suggestion
     property bool isSuggestion: false
     
     // Reason for the suggestion (e.g., "Often used at 9am")
     property string suggestionReason: ""
     
     // ==================== RUNNING WINDOW SUPPORT ====================
     // Number of open windows for this app (0 = not running)
     property int windowCount: 0
     
     // List of Toplevel objects for this app's open windows
     property list<var> windows: []
     
     // Whether this app has running windows
     property bool isRunning: windowCount > 0
     
     // ==================== PREVIEW PANEL SUPPORT ====================
     // Preview data object for side panel display
     // Structure: { type: "image"|"markdown"|"text"|"metadata", 
     //              content: string, title: string, 
     //              metadata: [{label, value}], actions: [{id, name, icon}],
     //              detachable: bool }
     property var preview: undefined
     
     // ==================== SLIDER ITEM SUPPORT ====================
     // For resultType === "slider" items
     property real value: 0
     property real min: 0
     property real max: 100
     property real step: 1
     property string displayValue: ""
     
     // ==================== VISUAL ENHANCEMENTS ====================
     // Badges: array of { label, value, icon, image }
     property var badges: []
     // Graph: { data: number[], min?, max? }
     property var graph: null
     // Gauge: { value, max, label? }
     property var gauge: null
     // Progress: { value, max, label?, color? }
     property var progress: null
}
