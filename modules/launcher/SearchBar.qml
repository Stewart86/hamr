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
    
    // Drag handle signals
    signal dragStarted(real mouseX, real mouseY)
    signal dragMoved(real mouseX, real mouseY)
    signal dragEnded()
    
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
        implicitHeight: Appearance.sizes.searchInputHeight
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

        onTextChanged: searchDebounce.restart()
        
        // Debounce timer - batches rapid keystrokes to reduce search overhead
        // Default 50ms feels instant but prevents multiple searches during fast typing
        Timer {
            id: searchDebounce
            interval: Config.options?.search?.debounceMs ?? 150
            onTriggered: LauncherSearch.query = searchInput.text
        }
        
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
        signal executeActionByIndex(int index)
        signal executePluginAction(int index)

        // Vim-style navigation (Ctrl+J/K/L) and Tab for action cycling
        Keys.onPressed: event => {
            // Tab cycles through actions on current item
            // Shift+Tab generates Key_Backtab on some systems
            if (event.key === Qt.Key_Backtab || (event.key === Qt.Key_Tab && (event.modifiers & Qt.ShiftModifier))) {
                searchInput.cycleActionPrev();
                event.accepted = true;
                return;
            }
            if (event.key === Qt.Key_Tab && !(event.modifiers & Qt.ControlModifier)) {
                searchInput.cycleActionNext();
                event.accepted = true;
                return;
            }
            
            if (event.modifiers & Qt.ControlModifier) {
                // Ctrl+1 through Ctrl+6 execute plugin actions (when plugin is active)
                const pluginActionIndex = event.key - Qt.Key_1;
                if (pluginActionIndex >= 0 && pluginActionIndex <= 5) {
                    searchInput.executePluginAction(pluginActionIndex);
                    event.accepted = true;
                    return;
                }
                
                // Ctrl+actionKeys execute action buttons directly (default: u,i,o,p)
                // Check this first so users can override navigation keys if desired
                const actionKeys = Config.options.search.actionKeys;
                // event.text is empty when Ctrl is held, so convert key code to char
                const keyChar = event.key >= Qt.Key_A && event.key <= Qt.Key_Z 
                    ? String.fromCharCode(event.key - Qt.Key_A + 97)  // 97 = 'a'
                    : "";
                const actionIndex = actionKeys.indexOf(keyChar);
                if (actionIndex >= 0 && actionIndex < 4) {
                    searchInput.executeActionByIndex(actionIndex);
                    event.accepted = true;
                    return;
                }
                
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

    // Drag handle icon
    Item {
        Layout.alignment: Qt.AlignVCenter
        implicitWidth: 24
        implicitHeight: 24
        
        MaterialSymbol {
            anchors.centerIn: parent
            iconSize: Appearance.font.pixelSize.normal
            text: "drag_indicator"
            color: dragHandleArea.containsMouse || dragHandleArea.pressed 
                ? Appearance.colors.colOnSurface 
                : Appearance.m3colors.m3outline
        }
        
        MouseArea {
            id: dragHandleArea
            anchors.fill: parent
            anchors.margins: -8 // Extend hit area
            hoverEnabled: true
            cursorShape: pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor
            
            onPressed: mouse => {
                const globalPos = mapToGlobal(mouse.x, mouse.y);
                root.dragStarted(globalPos.x, globalPos.y);
            }
            
            onPositionChanged: mouse => {
                if (pressed) {
                    const globalPos = mapToGlobal(mouse.x, mouse.y);
                    root.dragMoved(globalPos.x, globalPos.y);
                }
            }
            
            onReleased: root.dragEnded()
        }
    }
}
