// pragma NativeMethodBehavior: AcceptThisObject
import qs
import qs.services
import qs.modules.common
import qs.modules.common.models
import qs.modules.common.widgets
import qs.modules.common.functions
import QtQuick
import QtQuick.Layouts
import Qt5Compat.GraphicalEffects
import Quickshell
import Quickshell.Widgets
import Quickshell.Hyprland

RippleButton {
    id: root
    property var entry
    property string query
    property bool entryShown: entry?.shown ?? true
    property string itemType: entry?.type ?? "App"
    property string itemName: entry?.name ?? ""
    property var iconType: entry?.iconType
    property string iconName: entry?.iconName ?? ""
    property var itemExecute: entry?.execute
    property var fontType: switch(entry?.fontType) {
        case LauncherSearchResult.FontType.Monospace:
            return "monospace"
        case LauncherSearchResult.FontType.Normal:
            return "main"
        default:
            return "main"
    }
    property string itemClickActionName: entry?.verb ?? "Open"
    property string itemComment: entry?.comment ?? ""
    property string bigText: entry?.iconType === LauncherSearchResult.IconType.Text ? entry?.iconName ?? "" : ""
    property string materialSymbol: entry.iconType === LauncherSearchResult.IconType.Material ? entry?.iconName ?? "" : ""
    property string thumbnail: entry?.thumbnail ?? ""
    // Check running state dynamically from WindowManager for apps
    // This ensures correct state even for history items (type "Recent")
    property int windowCount: {
        const entryId = entry?.id ?? "";
        if (entryId && (itemType === "App" || itemType === "Recent")) {
            return WindowManager.getWindowsForApp(entryId).length;
        }
        return entry?.windowCount ?? 0;
    }
    property bool isRunning: windowCount > 0

    visible: root.entryShown
    property int horizontalMargin: 4
    property int buttonHorizontalPadding: 10
    property int buttonVerticalPadding: 10
    property bool keyboardDown: false
    
    // Action focus: -1 = main item focused, 0+ = action button index
    property int focusedActionIndex: -1
    
    // Notify parent ListView when action index changes (for poll update restoration)
    // Also update the floating action hint popup
    onFocusedActionIndexChanged: {
        if (ListView.view && typeof ListView.view.updateActionIndex === "function") {
            ListView.view.updateActionIndex(focusedActionIndex);
        }
        updateActionHint();
    }
    
    function cycleActionNext() {
        const actions = root.entry.actions ?? [];
        if (actions.length === 0) return;
        
        if (root.focusedActionIndex < actions.length - 1) {
            root.focusedActionIndex++;
        } else {
            root.focusedActionIndex = -1; // Wrap back to main item
        }
    }
    
    function cycleActionPrev() {
        const actions = root.entry.actions ?? [];
        if (actions.length === 0) return;
        
        if (root.focusedActionIndex > -1) {
            root.focusedActionIndex--;
        } else {
            root.focusedActionIndex = actions.length - 1; // Wrap to last action
        }
    }
    
    function executeCurrentAction() {
        const actions = root.entry.actions;
        if (actions && root.focusedActionIndex >= 0 && root.focusedActionIndex < actions.length) {
            const action = actions[root.focusedActionIndex];
            if (action && typeof action.execute === "function") {
                LauncherSearch.skipNextAutoFocus = true;
                const listView = root.ListView.view;
                if (listView) listView.currentIndex = -1;
                action.execute();
            } else {
                root.clicked();
            }
        } else {
            root.clicked();
        }
    }
    
    // Reset action focus when losing list selection
    // Note: We don't reset on entryChanged because during poll updates,
    // the entry data changes but we want to preserve action focus
    // Also update the floating action hint popup
    ListView.onIsCurrentItemChanged: {
        if (!ListView.isCurrentItem) {
            root.focusedActionIndex = -1;
        }
        updateActionHint();
    }

    implicitHeight: rowLayout.implicitHeight + root.buttonVerticalPadding * 2
    implicitWidth: rowLayout.implicitWidth + root.buttonHorizontalPadding * 2
    buttonRadius: Appearance.rounding.verysmall
    
    property bool isSelected: root.ListView.isCurrentItem
    colBackground: (root.down || root.keyboardDown) ? Appearance.colors.colPrimaryContainerActive : 
        (root.isSelected ? Appearance.colors.colSurfaceContainerLow :
        ((root.hovered || root.focus) ? Appearance.colors.colPrimaryContainer : 
        ColorUtils.transparentize(Appearance.colors.colPrimaryContainer, 1)))
    colBackgroundHover: root.isSelected ? Appearance.colors.colSurfaceContainerLow : Appearance.colors.colPrimaryContainer
    colRipple: Appearance.colors.colPrimaryContainerActive
    
    // Border for selected item
    Rectangle {
        anchors.fill: root.background
        radius: root.buttonRadius
        color: "transparent"
        border.width: root.isSelected ? 1 : 0
        border.color: Appearance.colors.colOutline
        visible: root.isSelected
    }

    property string highlightPrefix: `<u><font color="${Appearance.colors.colPrimary}">`
    property string highlightSuffix: `</font></u>`
    function highlightContent(content, query) {
        if (!query || query.length === 0 || content == query || fontType === "monospace")
            return StringUtils.escapeHtml(content);

        let contentLower = content.toLowerCase();
        let queryLower = query.toLowerCase();

        let result = "";
        let lastIndex = 0;
        let qIndex = 0;

        for (let i = 0; i < content.length && qIndex < query.length; i++) {
            if (contentLower[i] === queryLower[qIndex]) {
                // Add non-highlighted part (escaped)
                if (i > lastIndex)
                    result += StringUtils.escapeHtml(content.slice(lastIndex, i));
                // Add highlighted character (escaped)
                result += root.highlightPrefix + StringUtils.escapeHtml(content[i]) + root.highlightSuffix;
                lastIndex = i + 1;
                qIndex++;
            }
        }
        // Add the rest of the string (escaped)
        if (lastIndex < content.length)
            result += StringUtils.escapeHtml(content.slice(lastIndex));

        return result;
    }
    property string displayContent: highlightContent(root.itemName, root.query)

    property list<string> urls: {
        if (!root.itemName) return [];
        // Regular expression to match URLs
        const urlRegex = /https?:\/\/[^\s<>"{}|\\^`[\]]+/gi;
        const matches = root.itemName?.match(urlRegex)
            ?.filter(url => !url.includes("…")) // Elided = invalid
        return matches ? matches : [];
    }
    
    PointingHandInteraction {}

    background {
        anchors.fill: root
        anchors.leftMargin: root.horizontalMargin
        anchors.rightMargin: root.horizontalMargin
    }
    
    // Subtle left accent bar for running apps
    Rectangle {
        visible: root.isRunning
        anchors.left: root.left
        anchors.verticalCenter: root.verticalCenter
        anchors.leftMargin: root.horizontalMargin + 2
        height: 16
        width: 3
        radius: 1.5
        color: Appearance.colors.colPrimary
        opacity: 0.7
    }

    onClicked: {
        // For plugin results, don't auto-close - let the handler decide via execute.close
        // For plugin entry (starting a plugin), keep open
        // For everything else (apps, actions, scripts), close first to release HyprlandFocusGrab.
        // This fixes region selectors like `slurp` (screenshot) not receiving pointer input.
        const isPlugin = entry?.resultType === LauncherSearchResult.ResultType.PluginEntry ||
                          entry?.resultType === LauncherSearchResult.ResultType.PluginResult;

        if (isPlugin) {
            root.itemExecute()
            return
        }

        GlobalStates.launcherOpen = false
        Qt.callLater(() => root.itemExecute())
    }
    Keys.onPressed: (event) => {
        if (event.key === Qt.Key_Delete && event.modifiers === Qt.ShiftModifier) {
            // Look for Delete or Remove action (Remove is used for history items)
            const deleteAction = root.entry.actions.find(action => action.name === "Delete" || action.name === "Remove");

            if (deleteAction) {
                deleteAction.execute()
                event.accepted = true
            }
        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            root.keyboardDown = true
            root.clicked()
            event.accepted = true
        } else if (event.key >= Qt.Key_1 && event.key <= Qt.Key_4) {
            // Number keys 1-4 trigger action buttons
            const index = event.key - Qt.Key_1
            const actions = root.entry.actions ?? []
            if (index < actions.length) {
                LauncherSearch.skipNextAutoFocus = true
                const listView = root.ListView.view
                if (listView) listView.currentIndex = -1
                actions[index].execute()
                event.accepted = true
            }
        }
    }
    Keys.onReleased: (event) => {
        if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            root.keyboardDown = false
            event.accepted = true;
        }
    }

    RowLayout {
        id: rowLayout
        spacing: iconContainer.visible ? 10 : 0
        anchors.fill: parent
        anchors.leftMargin: root.horizontalMargin + root.buttonHorizontalPadding
        anchors.rightMargin: root.horizontalMargin + root.buttonHorizontalPadding

        // Unified icon container - fixed size for visual consistency
        Item {
            id: iconContainer
            Layout.alignment: Qt.AlignVCenter
            visible: root.thumbnail !== "" || root.iconType !== LauncherSearchResult.IconType.None
            
            // Fixed container size - all icon types fit within this (from Config)
            property int containerSize: Appearance.sizes.resultIconSize
            implicitWidth: containerSize
            implicitHeight: containerSize
            
            // Thumbnail (takes priority)
            Rectangle {
                id: thumbnailRect
                visible: root.thumbnail !== ""
                anchors.fill: parent
                radius: 4
                color: Appearance.colors.colSurfaceContainerHigh
                
                Image {
                    anchors.fill: parent
                    source: root.thumbnail ? Qt.resolvedUrl(root.thumbnail) : ""
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    sourceSize.width: 80
                    sourceSize.height: 80
                    layer.enabled: true
                    layer.effect: OpacityMask {
                        maskSource: Rectangle {
                            width: thumbnailRect.width
                            height: thumbnailRect.height
                            radius: thumbnailRect.radius
                        }
                    }
                }
            }
            
            // System icon (app icons)
            IconImage {
                visible: !thumbnailRect.visible && root.iconType === LauncherSearchResult.IconType.System
                anchors.centerIn: parent
                source: root.iconName ? Quickshell.iconPath(root.iconName, "image-missing") : ""
                width: 32
                height: 32
            }
            
            // Material symbol
            MaterialSymbol {
                visible: !thumbnailRect.visible && root.iconType === LauncherSearchResult.IconType.Material
                anchors.centerIn: parent
                text: root.materialSymbol
                iconSize: 26
                color: Appearance.m3colors.m3onSurface
            }
            
            // Text/emoji icon
            StyledText {
                visible: !thumbnailRect.visible && root.iconType === LauncherSearchResult.IconType.Text
                anchors.centerIn: parent
                text: root.bigText
                font.pixelSize: Appearance.font.pixelSize.larger
                color: Appearance.m3colors.m3onSurface
            }
        }

        // Main text
        ColumnLayout {
            id: contentColumn
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignVCenter
            spacing: 0
            StyledText {
                font.pixelSize: Appearance.font.pixelSize.smaller
                color: Appearance.colors.colSubtext
                visible: root.itemType && root.itemType != "App"
                text: root.itemType
            }
            RowLayout {
                Repeater { // Favicons for links (limit to 3 to avoid process explosion)
                    model: (root.query == root.itemName ? [] : root.urls).slice(0, 3)
                    Favicon {
                        required property var modelData
                        size: parent.height
                        url: modelData
                    }
                }
                StyledText { // Item name/content
                    Layout.fillWidth: true
                    id: nameText
                    textFormat: Text.StyledText // RichText also works, but StyledText ensures elide work
                    font.pixelSize: Appearance.font.pixelSize.small
                    font.family: Appearance.font.family[root.fontType]
                    color: Appearance.m3colors.m3onSurface
                    horizontalAlignment: Text.AlignLeft
                    elide: Text.ElideRight
                    text: root.displayContent
                }
            }
            StyledText { // Comment/description (e.g., file path)
                Layout.fillWidth: true
                visible: root.itemComment !== ""
                font.pixelSize: Appearance.font.pixelSize.smaller
                font.family: Appearance.font.family.monospace
                color: Appearance.colors.colSubtext
                horizontalAlignment: Text.AlignLeft
                elide: Text.ElideMiddle
                text: root.itemComment
            }
        }

        RowLayout {
            Layout.alignment: Qt.AlignVCenter
            Layout.fillHeight: false
            spacing: 4
            
            // Primary action hint - shows on selection, aligned left of action buttons
            RowLayout {
                id: primaryActionHint
                Layout.rightMargin: 10
                spacing: 4
                opacity: root.isSelected ? 1 : 0
                visible: opacity > 0
                
                Behavior on opacity {
                    NumberAnimation { duration: 150; easing.type: Easing.OutCubic }
                }
                
                Kbd {
                    keys: "Enter"
                }
                
                Text {
                    text: root.itemClickActionName
                    font.pixelSize: Appearance.font.pixelSize.smallest
                    color: Appearance.m3colors.m3outline
                }
            }
            
            Repeater {
                model: (root.entry.actions ?? []).slice(0, 4)
                delegate: Item {
                    id: actionButton
                    required property var modelData
                    required property int index
                    property var iconType: modelData.iconType
                    property string iconName: modelData.iconName ?? ""
                    property string keyHint: (Config.options.search.actionKeys[index] ?? (index + 1).toString()).toUpperCase()
                    property bool isFocused: root.focusedActionIndex === index && root.ListView.isCurrentItem
                    implicitHeight: 28
                    implicitWidth: 28

                    Rectangle {
                        id: actionBg
                        anchors.fill: parent
                        radius: Appearance.rounding.verysmall
                        color: actionButton.isFocused ? Appearance.colors.colPrimary :
                               actionMouse.containsMouse ? Appearance.colors.colSecondaryContainerHover : "transparent"
                        Behavior on color {
                            ColorAnimation { duration: 100 }
                        }
                    }

                    Loader {
                        anchors.centerIn: parent
                        active: actionButton.iconType === LauncherSearchResult.IconType.Material || actionButton.iconName === ""
                        sourceComponent: MaterialSymbol {
                            text: actionButton.iconName || "video_settings"
                            font.pixelSize: 20
                            color: actionButton.isFocused ? Appearance.m3colors.m3onPrimary : Appearance.colors.colSubtext
                            opacity: actionButton.isFocused ? 1.0 : 0.8
                        }
                    }
                    Loader {
                        anchors.centerIn: parent
                        active: actionButton.iconType === LauncherSearchResult.IconType.System && actionButton.iconName !== ""
                        sourceComponent: IconImage {
                            source: Quickshell.iconPath(actionButton.iconName)
                            implicitSize: 20
                        }
                    }

                    MouseArea {
                        id: actionMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: (event) => {
                            event.accepted = true
                            LauncherSearch.skipNextAutoFocus = true
                            // Keep selection - don't clear currentIndex
                            actionButton.modelData.execute()
                        }
                        onPressed: (event) => { event.accepted = true }
                        onReleased: (event) => { event.accepted = true }
                    }

                }
            }
        }

    }

    // Calculate and report focused action position to GlobalStates for floating hint popup
    function updateActionHint() {
        const actions = root.entry.actions ?? [];
        const isCurrent = root.ListView.isCurrentItem;
        
        // Only show floating hint for action buttons, not primary action
        if (root.focusedActionIndex >= 0 && root.focusedActionIndex < actions.length && isCurrent) {
            const action = actions[root.focusedActionIndex];
            const buttonWidth = 28;
            const buttonSpacing = 4;
            // Only up to 4 actions are displayed (see Repeater model slicing)
            const actionsCount = Math.min(actions.length, 4);
            const actionsRowWidth = actionsCount * buttonWidth + (actionsCount - 1) * buttonSpacing;
            const buttonOffset = root.focusedActionIndex * (buttonWidth + buttonSpacing) + buttonWidth / 2;
            const localX = root.width - root.horizontalMargin - root.buttonHorizontalPadding - actionsRowWidth + buttonOffset;
            const localY = (root.height + buttonWidth) / 2 + 2;
            
            const globalPos = root.mapToGlobal(localX, localY);
            const keyHint = "^" + (Config.options.search.actionKeys[root.focusedActionIndex] ?? (root.focusedActionIndex + 1).toString()).toUpperCase();
            GlobalStates.showActionHint(keyHint, action.name, globalPos.x, globalPos.y);
        } else {
            GlobalStates.hideActionHint();
        }
    }
}
