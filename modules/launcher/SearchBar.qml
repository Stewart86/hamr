import qs
import qs.services
import qs.modules.common
import qs.modules.common.widgets
import qs.modules.common.functions
import Qt5Compat.GraphicalEffects
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland

RowLayout {
    id: root
    spacing: 6
    property bool animateWidth: false
    property alias searchInput: searchInput
    property string searchingText
    
    // Custom placeholder from active plugin or exclusive mode
    readonly property string pluginPlaceholder: PluginRunner.isActive() ? PluginRunner.pluginPlaceholder : ""
    readonly property string exclusiveModePlaceholder: {
        switch (LauncherSearch.exclusiveMode) {
            case "action": return "Search actions...";
            case "emoji": return "Search emoji...";
            case "math": return "Calculate...";
            default: return "";
        }
    }

    function forceFocus() {
        searchInput.forceActiveFocus();
    }

    enum SearchPrefixType { Action, App, Clipboard, Emojis, Math, ShellCommand, WebSearch, DefaultSearch }

    property var searchPrefixType: {
        // Check exclusive mode first
        if (LauncherSearch.exclusiveMode === "action") return SearchBar.SearchPrefixType.Action;
        if (LauncherSearch.exclusiveMode === "emoji") return SearchBar.SearchPrefixType.Emojis;
        if (LauncherSearch.exclusiveMode === "math") return SearchBar.SearchPrefixType.Math;
        // Fall back to prefix detection for non-exclusive modes
        if (root.searchingText.startsWith(Config.options.search.prefix.action)) return SearchBar.SearchPrefixType.Action;
        if (root.searchingText.startsWith(Config.options.search.prefix.app)) return SearchBar.SearchPrefixType.App;
        if (root.searchingText.startsWith(Config.options.search.prefix.clipboard)) return SearchBar.SearchPrefixType.Clipboard;
        if (root.searchingText.startsWith(Config.options.search.prefix.emojis)) return SearchBar.SearchPrefixType.Emojis;
        if (root.searchingText.startsWith(Config.options.search.prefix.math)) return SearchBar.SearchPrefixType.Math;
        if (root.searchingText.startsWith(Config.options.search.prefix.shellCommand)) return SearchBar.SearchPrefixType.ShellCommand;
        if (root.searchingText.startsWith(Config.options.search.prefix.webSearch)) return SearchBar.SearchPrefixType.WebSearch;
        return SearchBar.SearchPrefixType.DefaultSearch;
    }
    
    MaterialShapeWrappedMaterialSymbol {
        id: searchIcon
        Layout.alignment: Qt.AlignVCenter
        iconSize: Appearance.font.pixelSize.huge
        shape: switch(root.searchPrefixType) {
            case SearchBar.SearchPrefixType.Action: return MaterialShape.Shape.Pill;
            case SearchBar.SearchPrefixType.App: return MaterialShape.Shape.Clover4Leaf;
            case SearchBar.SearchPrefixType.Clipboard: return MaterialShape.Shape.Gem;
            case SearchBar.SearchPrefixType.Emojis: return MaterialShape.Shape.Sunny;
            case SearchBar.SearchPrefixType.Math: return MaterialShape.Shape.PuffyDiamond;
            case SearchBar.SearchPrefixType.ShellCommand: return MaterialShape.Shape.PixelCircle;
            case SearchBar.SearchPrefixType.WebSearch: return MaterialShape.Shape.SoftBurst;
            default: return MaterialShape.Shape.Circle;
        }
        text: switch (root.searchPrefixType) {
            case SearchBar.SearchPrefixType.Action: return "settings_suggest";
            case SearchBar.SearchPrefixType.App: return "apps";
            case SearchBar.SearchPrefixType.Clipboard: return "content_paste_search";
            case SearchBar.SearchPrefixType.Emojis: return "add_reaction";
            case SearchBar.SearchPrefixType.Math: return "calculate";
            case SearchBar.SearchPrefixType.ShellCommand: return "terminal";
            case SearchBar.SearchPrefixType.WebSearch: return "travel_explore";
            case SearchBar.SearchPrefixType.DefaultSearch: return "search";
            default: return "search";
        }
    }
    // Signals for vim-style navigation (handled by parent SearchWidget)
    signal navigateDown()
    signal navigateUp()
    signal selectCurrent()

    ToolbarTextField { // Search box
        id: searchInput
        Layout.topMargin: 4
        Layout.bottomMargin: 4
        implicitHeight: 40
        focus: GlobalStates.launcherOpen
        font.pixelSize: Appearance.font.pixelSize.small
        placeholderText: root.pluginPlaceholder !== "" ? root.pluginPlaceholder : 
                         root.exclusiveModePlaceholder !== "" ? root.exclusiveModePlaceholder : "It's hamr time!"
        implicitWidth: Appearance.sizes.searchWidth

        Behavior on implicitWidth {
            id: searchWidthBehavior
            enabled: root.animateWidth
            NumberAnimation {
                duration: 300
                easing.type: Appearance.animation.elementMove.type
                easing.bezierCurve: Appearance.animation.elementMove.bezierCurve
            }
        }

        onTextChanged: LauncherSearch.query = text
        
        // Sync text when LauncherSearch.query changes externally (e.g., workflow start clears it)
        Connections {
            target: LauncherSearch
            function onQueryChanged() {
                if (searchInput.text !== LauncherSearch.query) {
                    searchInput.text = LauncherSearch.query;
                }
            }
        }

        // Signals for action navigation
        signal cycleActionNext()
        signal cycleActionPrev()

        // Vim-style navigation (Ctrl+J/K/L) and Tab for action cycling
        Keys.onPressed: event => {
            // Tab cycles through actions on current item
            if (event.key === Qt.Key_Tab && !(event.modifiers & Qt.ControlModifier)) {
                if (event.modifiers & Qt.ShiftModifier) {
                    searchInput.cycleActionPrev();
                } else {
                    searchInput.cycleActionNext();
                }
                event.accepted = true;
                return;
            }
            
            if (event.modifiers & Qt.ControlModifier) {
                if (event.key === Qt.Key_J) {
                    root.navigateDown();
                    event.accepted = true;
                    return;
                }
                if (event.key === Qt.Key_K) {
                    root.navigateUp();
                    event.accepted = true;
                    return;
                }
                if (event.key === Qt.Key_L) {
                    root.selectCurrent();
                    event.accepted = true;
                    return;
                }
            }
        }

        onAccepted: root.selectCurrent()
    }

}
