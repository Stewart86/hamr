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
    
    signal dragStarted(real mouseX, real mouseY)
    signal dragMoved(real mouseX, real mouseY)
    signal dragEnded()
    
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
    
    Item {
        id: searchIconContainer
        Layout.alignment: Qt.AlignVCenter
        visible: !PluginRunner.isActive()
        implicitWidth: searchIcon.implicitWidth
        implicitHeight: searchIcon.implicitHeight
        property bool hovered: searchIconHover.containsMouse
        
        MaterialShapeWrappedMaterialSymbol {
            id: searchIcon
            anchors.centerIn: parent
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
                case SearchBar.SearchPrefixType.DefaultSearch: return "gavel";
                default: return "gavel";
            }
        }
        
        MouseArea {
            id: searchIconHover
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: {
                Persistent.states.launcher.actionBarHidden = !Persistent.states.launcher.actionBarHidden;
            }
        }
        
        StyledToolTip {
            text: Persistent.states.launcher.actionBarHidden ? "Show action bar" : "Hide action bar"
        }
    }

    Item {
        id: pluginIconContainer
        Layout.alignment: Qt.AlignVCenter
        visible: PluginRunner.isActive()
        implicitWidth: pluginIcon.implicitWidth
        implicitHeight: pluginIcon.implicitHeight
        property bool hovered: pluginIconHover.containsMouse
        
        onVisibleChanged: {
            if (visible) {
                pluginIcon.scale = 0;
                pluginIconBorder.scale = 0;
                scaleIn.start();
            }
        }
        
        MaterialShape {
            id: pluginIconBorder
            anchors.centerIn: parent
            shape: MaterialShape.Shape.Squircle
            implicitSize: pluginIcon.implicitSize + 4
            color: Appearance.colors.colPrimary
            scale: 0
        }
        
        MaterialShapeWrappedMaterialSymbol {
            id: pluginIcon
            anchors.centerIn: parent
            iconSize: Appearance.font.pixelSize.huge
            shape: MaterialShape.Shape.Squircle
            color: Appearance.colors.colPrimaryContainer
            colSymbol: Appearance.colors.colOnPrimaryContainer
            text: PluginRunner.activePlugin?.manifest?.icon ?? "extension"
            
            SequentialAnimation {
                id: scaleIn
                ParallelAnimation {
                    NumberAnimation {
                        target: pluginIconBorder
                        property: "scale"
                        from: 0
                        to: 1.15
                        duration: 200
                        easing.type: Easing.OutBack
                    }
                    NumberAnimation {
                        target: pluginIcon
                        property: "scale"
                        from: 0
                        to: 1.15
                        duration: 200
                        easing.type: Easing.OutBack
                    }
                }
                ParallelAnimation {
                    NumberAnimation {
                        target: pluginIconBorder
                        property: "scale"
                        to: 1.0
                        duration: 150
                        easing.type: Easing.InOutQuad
                    }
                    NumberAnimation {
                        target: pluginIcon
                        property: "scale"
                        to: 1.0
                        duration: 150
                        easing.type: Easing.InOutQuad
                    }
                }
            }
        }
        
        MouseArea {
            id: pluginIconHover
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.NoButton
        }
        
        StyledToolTip {
            text: PluginRunner.activePlugin?.manifest?.name ?? ""
        }
    }

    signal navigateDown()
    signal navigateUp()
    signal selectCurrent()

    ToolbarTextField {
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
         
         Timer {
            id: searchDebounce
            interval: Config.options?.search?.debounceMs ?? 150
            onTriggered: LauncherSearch.query = searchInput.text
        }
        
         Connections {
            target: LauncherSearch
            function onQueryChanged() {
                if (searchInput.text !== LauncherSearch.query) {
                    searchInput.text = LauncherSearch.query;
                }
            }
        }

         signal cycleActionNext()
         signal cycleActionPrev()
         signal executeActionByIndex(int index)
         signal executePluginAction(int index)

         Keys.onPressed: event => {
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
                 const pluginActionIndex = event.key - Qt.Key_1;
                if (pluginActionIndex >= 0 && pluginActionIndex <= 5) {
                    searchInput.executePluginAction(pluginActionIndex);
                    event.accepted = true;
                    return;
                }
                
                 const actionKeys = Config.options.search.actionKeys;
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
