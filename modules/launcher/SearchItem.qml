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
    property bool isSuggestion: entry?.isSuggestion ?? false
    property string suggestionReason: entry?.suggestionReason ?? ""
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
    
    property int focusedActionIndex: -1
    
    onFocusedActionIndexChanged: {
        if (ListView.view && typeof ListView.view.updateActionIndex === "function") {
            ListView.view.updateActionIndex(focusedActionIndex);
        }
        updateActionToolTip();
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
                // Capture selection before action executes (for restoration after results update)
                const listView = root.ListView.view;
                if (listView && typeof listView.captureSelection === "function") {
                    listView.captureSelection();
                }
                LauncherSearch.skipNextAutoFocus = true;
                action.execute();
            } else {
                root.clicked();
            }
        } else {
            root.clicked();
        }
    }
    
     ListView.onIsCurrentItemChanged: {
        if (!ListView.isCurrentItem) {
            root.focusedActionIndex = -1;
        }
        updateActionToolTip();
    }
    
    onHoveredChanged: {
        if (hovered && entry) {
            GlobalStates.setPreviewItem(entry);
        }
    }

    implicitHeight: rowLayout.implicitHeight + root.buttonVerticalPadding * 2
    implicitWidth: rowLayout.implicitWidth + root.buttonHorizontalPadding * 2
    buttonRadius: Appearance.rounding.verysmall
    
    property bool isSelected: root.ListView.isCurrentItem
    colBackground: (root.down || root.keyboardDown) ? Appearance.colors.colPrimaryContainerActive : 
        (root.isSelected ? Appearance.colors.colSurfaceContainerHigh :
        ((root.hovered || root.focus) ? Appearance.colors.colPrimaryContainer : 
        "transparent"))
    colBackgroundHover: root.isSelected ? Appearance.colors.colSurfaceContainerHighest : Appearance.colors.colPrimaryContainer
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
                 if (i > lastIndex)
                     result += StringUtils.escapeHtml(content.slice(lastIndex, i));
                 result += root.highlightPrefix + StringUtils.escapeHtml(content[i]) + root.highlightSuffix;
                 lastIndex = i + 1;
                 qIndex++;
             }
         }
         if (lastIndex < content.length)
             result += StringUtils.escapeHtml(content.slice(lastIndex));

        return result;
    }
    property string displayContent: highlightContent(root.itemName, root.query)

    property list<string> urls: {
         if (!root.itemName) return [];
         const urlRegex = /https?:\/\/[^\s<>"{}|\\^`[\]]+/gi;
         const matches = root.itemName?.match(urlRegex)
             ?.filter(url => !url.includes("…"))
         return matches ? matches : [];
     }
    
    PointingHandInteraction {}

    background {
        anchors.fill: root
        anchors.leftMargin: root.horizontalMargin
        anchors.rightMargin: root.horizontalMargin
    }
    
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
         const isPlugin = entry?.resultType === LauncherSearchResult.ResultType.PluginEntry ||
                           entry?.resultType === LauncherSearchResult.ResultType.PluginResult;
         const shouldKeepOpen = entry?.keepOpen === true;

        if (isPlugin || shouldKeepOpen) {
            // Capture selection before action executes (for restoration after results update)
            const listView = root.ListView.view;
            if (listView && typeof listView.captureSelection === "function") {
                listView.captureSelection();
            }
            LauncherSearch.skipNextAutoFocus = true;
            root.itemExecute()
            return
        }

        // Execute first, then close launcher (closing can be slow due to animations)
        root.itemExecute()
        Qt.callLater(() => { GlobalStates.launcherOpen = false })
    }
    Keys.onPressed: (event) => {
         if (event.key === Qt.Key_Delete && event.modifiers === Qt.ShiftModifier) {
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
             const index = event.key - Qt.Key_1
            const actions = root.entry.actions ?? []
            if (index < actions.length) {
                // Capture selection before action executes (for restoration after results update)
                const listView = root.ListView.view
                if (listView && typeof listView.captureSelection === "function") {
                    listView.captureSelection()
                }
                LauncherSearch.skipNextAutoFocus = true
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

        Item {
            id: iconContainer
            Layout.alignment: Qt.AlignVCenter
            visible: root.thumbnail !== "" || root.iconType !== LauncherSearchResult.IconType.None
             
             property int containerSize: Appearance.sizes.resultIconSize
            implicitWidth: containerSize
            implicitHeight: containerSize
            
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
            
            IconImage {
                visible: !thumbnailRect.visible && root.iconType === LauncherSearchResult.IconType.System
                anchors.centerIn: parent
                source: root.iconName ? Quickshell.iconPath(IconResolver.guessIcon(root.iconName), "image-missing") : ""
                width: 32
                height: 32
            }
            
            MaterialSymbol {
                visible: !thumbnailRect.visible && root.iconType === LauncherSearchResult.IconType.Material
                anchors.centerIn: parent
                text: root.materialSymbol
                iconSize: 26
                color: Appearance.m3colors.m3onSurface
            }
            
            StyledText {
                visible: !thumbnailRect.visible && root.iconType === LauncherSearchResult.IconType.Text
                anchors.centerIn: parent
                text: root.bigText
                font.pixelSize: Appearance.font.pixelSize.larger
                color: Appearance.m3colors.m3onSurface
            }
        }

        ColumnLayout {
            id: contentColumn
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignVCenter
            spacing: 0
            RowLayout {
                spacing: 6
                visible: root.itemType && root.itemType != "App"
                
                StyledText {
                    font.pixelSize: Appearance.font.pixelSize.smaller
                    color: root.isSuggestion ? Appearance.colors.colPrimary : Appearance.colors.colSubtext
                    text: root.isSuggestion ? "Suggested" : root.itemType
                }
                
                MaterialSymbol {
                    visible: root.isSuggestion
                    text: "auto_awesome"
                    iconSize: Appearance.font.pixelSize.smaller
                    color: Appearance.colors.colPrimary
                    opacity: 0.8
                }
            }
            RowLayout {
             Repeater {
                     model: (root.query == root.itemName ? [] : root.urls).slice(0, 3)
                    Favicon {
                        required property var modelData
                        size: parent.height
                        url: modelData
                    }
                }
                 StyledText {
                     Layout.fillWidth: true
                     id: nameText
                     textFormat: Text.StyledText
                    font.pixelSize: Appearance.font.pixelSize.small
                    font.family: Appearance.font.family[root.fontType]
                    color: Appearance.m3colors.m3onSurface
                    horizontalAlignment: Text.AlignLeft
                    elide: Text.ElideRight
                    text: root.displayContent

                    HoverHandler {
                        id: nameHover
                    }

                    StyledToolTipContent {
                        text: root.itemName
                        shown: nameHover.hovered && nameText.truncated
                        anchors.bottom: nameText.top
                        anchors.left: nameText.left
                    }
                }
            }
             StyledText {
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
                            // Capture selection before action executes (for restoration after results update)
                            const listView = root.ListView.view
                            if (listView && typeof listView.captureSelection === "function") {
                                listView.captureSelection()
                            }
                            LauncherSearch.skipNextAutoFocus = true
                            actionButton.modelData.execute()
                        }
                        onPressed: (event) => { event.accepted = true }
                        onReleased: (event) => { event.accepted = true }
                        onContainsMouseChanged: {
                            if (containsMouse) {
                                const globalPos = actionButton.mapToGlobal(actionButton.width / 2, actionButton.height + 2)
                                GlobalStates.showActionToolTip(actionButton.keyHint, actionButton.modelData.name, globalPos.x, globalPos.y)
                            } else {
                                GlobalStates.hideActionToolTip()
                            }
                        }
                    }

                }
            }
        }

    }

    function updateActionToolTip() {
        const actions = root.entry.actions ?? [];
        const isCurrent = root.ListView.isCurrentItem;
        
         if (root.focusedActionIndex >= 0 && root.focusedActionIndex < actions.length && isCurrent) {
             const action = actions[root.focusedActionIndex];
             const buttonWidth = 28;
             const buttonSpacing = 4;
             const actionsCount = Math.min(actions.length, 4);
            const actionsRowWidth = actionsCount * buttonWidth + (actionsCount - 1) * buttonSpacing;
            const buttonOffset = root.focusedActionIndex * (buttonWidth + buttonSpacing) + buttonWidth / 2;
            const localX = root.width - root.horizontalMargin - root.buttonHorizontalPadding - actionsRowWidth + buttonOffset;
            const localY = (root.height + buttonWidth) / 2 + 2;
            
            const globalPos = root.mapToGlobal(localX, localY);
            const keyHint = "^" + (Config.options.search.actionKeys[root.focusedActionIndex] ?? (root.focusedActionIndex + 1).toString()).toUpperCase();
            GlobalStates.showActionToolTip(keyHint, action.name, globalPos.x, globalPos.y);
        } else {
            GlobalStates.hideActionToolTip();
        }
    }
}
