pragma Singleton
pragma ComponentBehavior: Bound
import QtQuick
import Quickshell
import Quickshell.Io
import qs.modules.common.functions

Singleton {
    id: root
    property string filePath: Directories.shellConfigPath
    property alias options: configOptionsJsonAdapter
    property bool ready: false
    property int readWriteDelay: 50 // milliseconds
    property bool blockWrites: false

    // Parsed actionBarHints - kept outside JsonObject to avoid JsonAdapter serialization crash on some systems
    readonly property var actionBarHints: {
        try {
            return JSON.parse(options.search.actionBarHintsJson);
        } catch (e) {
            return [
                { "prefix": "~", "icon": "folder", "label": "Files", "plugin": "files" },
                { "prefix": ";", "icon": "content_paste", "label": "Clipboard", "plugin": "clipboard" },
                { "prefix": "/", "icon": "extension", "label": "Plugins", "plugin": "action" },
                { "prefix": "!", "icon": "terminal", "label": "Shell", "plugin": "shell" },
                { "prefix": "=", "icon": "calculate", "label": "Math", "plugin": "calculate" },
                { "prefix": ":", "icon": "emoji_emotions", "label": "Emoji", "plugin": "emoji" }
            ];
        }
    }

    function setNestedValue(nestedKey, value) {
        let keys = nestedKey.split(".");
        let obj = root.options;
        let parents = [obj];

        // Traverse and collect parent objects
        for (let i = 0; i < keys.length - 1; ++i) {
            if (!obj[keys[i]] || typeof obj[keys[i]] !== "object") {
                obj[keys[i]] = {};
            }
            obj = obj[keys[i]];
            parents.push(obj);
        }

        // Convert value to correct type using JSON.parse when safe
        let convertedValue = value;
        if (typeof value === "string") {
            let trimmed = value.trim();
            if (trimmed === "true" || trimmed === "false" || !isNaN(Number(trimmed))) {
                try {
                    convertedValue = JSON.parse(trimmed);
                } catch (e) {
                    convertedValue = value;
                }
            }
        }

        obj[keys[keys.length - 1]] = convertedValue;
    }

    Timer {
        id: fileReloadTimer
        interval: root.readWriteDelay
        repeat: false
        onTriggered: {
            configFileView.reload()
        }
    }

    Timer {
        id: fileWriteTimer
        interval: root.readWriteDelay
        repeat: false
        onTriggered: {
            configFileView.writeAdapter()
        }
    }

    FileView {
        id: configFileView
        path: root.filePath
        watchChanges: true
        blockWrites: root.blockWrites
        onFileChanged: fileReloadTimer.restart()
        onAdapterUpdated: fileWriteTimer.restart()
        onLoaded: root.ready = true
        onLoadFailed: error => {
            if (error == FileViewError.FileNotFound) {
                writeAdapter();
            }
        }

        JsonAdapter {
            id: configOptionsJsonAdapter

            // ==================== APPS ====================
            property JsonObject apps: JsonObject {
                property string terminal: "ghostty" // Terminal for shell actions
                property string terminalArgs: "--class=floating.terminal" // Terminal window class args
                property string shell: "zsh" // Shell for command execution (zsh, bash, fish)
            }

            // ==================== SEARCH ====================
            property JsonObject search: JsonObject {
                property int nonAppResultDelay: 30 // Prevents lagging when typing
                property int debounceMs: 50 // Debounce for search input (ms)
                property int pluginDebounceMs: 150 // Plugin search debounce (ms)
                property int maxHistoryItems: 500 // Max search history entries (affects memory & fuzzy search speed)
                property int maxDisplayedResults: 16 // Max results shown in launcher
                property int maxRecentItems: 20 // Max recent history items shown
                property int shellHistoryLimit: 50 // Shell history results limit
                property string engineBaseUrl: "https://www.google.com/search?q="
                property list<string> excludedSites: ["quora.com", "facebook.com"]
                property JsonObject prefix: JsonObject {
                    property string action: "/"
                    property string app: ">"
                    property string clipboard: ";"
                    property string emojis: ":"
                    property string file: "~"
                    property string math: "="
                    property string shellCommand: "$"
                    property string shellHistory: "!"
                    property string webSearch: "?"
                }
                property JsonObject shellHistory: JsonObject {
                    property bool enable: true
                    property string shell: "auto" // "auto", "zsh", "bash", "fish"
                    property string customHistoryPath: "" // Override auto-detection
                    property int maxEntries: 500
                }
                // Action button shortcuts (Ctrl + key)
                // Default: u, i, o, p for actions 1-4
                // Note: j/k are used for navigation, l for select, Ctrl+Shift+P for pin preview
                property list<string> actionKeys: ["u", "i", "o", "p"]
                // Action bar hints - customizable prefix shortcuts shown in the action bar
                // Stored as JSON string to work around Quickshell JsonAdapter limitation with arrays of objects
                // Each hint has: prefix, icon, label, plugin
                // Note: The old "prefix" object above is kept for backwards compatibility
                property string actionBarHintsJson: '[{"prefix":"~","icon":"folder","label":"Files","plugin":"files"},{"prefix":";","icon":"content_paste","label":"Clipboard","plugin":"clipboard"},{"prefix":"/","icon":"extension","label":"Plugins","plugin":"action"},{"prefix":"!","icon":"terminal","label":"Shell","plugin":"shell"},{"prefix":"=","icon":"calculate","label":"Math","plugin":"calculate"},{"prefix":":","icon":"emoji_emotions","label":"Emoji","plugin":"emoji"}]'
            }

            // ==================== IMAGE BROWSER ====================
            property JsonObject imageBrowser: JsonObject {
                property bool useSystemFileDialog: false
                property int columns: 4 // Grid columns
                property real cellAspectRatio: 1.333 // 4:3 aspect ratio
                property int sidebarWidth: 140 // Quick dirs sidebar width
            }

            // ==================== BEHAVIOR ====================
            property JsonObject behavior: JsonObject {
                // Time window (ms) to preserve state after soft close (click-outside)
                // Reopening within this window restores previous view
                // Set to 0 to disable (always start fresh)
                property int stateRestoreWindowMs: 30000  // 30 seconds
                // What happens when clicking outside the launcher:
                // "intuitive" - minimize if previously minimized, otherwise close
                // "close" - always close
                // "minimize" - always minimize to FAB
                property string clickOutsideAction: "intuitive"
            }

            // ==================== AUDIO ====================
            property JsonObject audio: JsonObject {
                property bool enabled: true // Enable/disable all sound effects
            }

            // ==================== APPEARANCE ====================
            property JsonObject appearance: JsonObject {
                // Transparency (0.0 = opaque, 1.0 = fully transparent)
                property real backgroundTransparency: 0.2
                property real contentTransparency: 0.2
                
                // Launcher position as ratio of screen (0.0-1.0)
                property real launcherXRatio: 0.5 // 0.5 = centered
                property real launcherYRatio: 0.1 // 0.1 = 10% from top
                
                // Font scale multiplier (0.75 = 75%, 1.0 = 100%, 1.5 = 150%)
                property real fontScale: 1.0
            }

            // ==================== SIZES ====================
            property JsonObject sizes: JsonObject {
                // Launcher dimensions
                property int searchWidth: 580
                property int searchInputHeight: 40
                property int maxResultsHeight: 600
                property int resultIconSize: 40
                
                // Image browser dimensions (legacy separate panel)
                property int imageBrowserWidth: 1200
                property int imageBrowserHeight: 690
                // Image browser grid (integrated into launcher)
                property int imageBrowserGridWidth: 900
                property int imageBrowserGridHeight: 600
                
                // Window picker preview
                property int windowPickerMaxWidth: 350
                property int windowPickerMaxHeight: 220
            }

            // ==================== FONTS ====================
            property JsonObject fonts: JsonObject {
                property string main: "Google Sans Flex"
                property string monospace: "JetBrains Mono NF"
                property string reading: "Readex Pro"
                property string icon: "Material Symbols Rounded"
            }

            // ==================== PATHS ====================
            property JsonObject paths: JsonObject {
                property string wallpaperDir: "" // Empty = default ~/Pictures/Wallpapers
                property string colorsJson: "" // Empty = default ~/.local/state/user/generated/colors.json
            }
        }
    }
}
