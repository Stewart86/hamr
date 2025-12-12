// pragma NativeMethodBehavior: AcceptThisObject
import qs
import qs.services
import qs.modules.common
import qs.modules.common.models
import qs.modules.common.widgets
import qs.modules.common.functions
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Widgets
import Quickshell.Hyprland

RippleButton {
    id: root
    property LauncherSearchResult entry
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

    visible: root.entryShown
    property int horizontalMargin: 10
    property int buttonHorizontalPadding: 10
    property int buttonVerticalPadding: 6
    property bool keyboardDown: false

    implicitHeight: rowLayout.implicitHeight + root.buttonVerticalPadding * 2
    implicitWidth: rowLayout.implicitWidth + root.buttonHorizontalPadding * 2
    buttonRadius: Appearance.rounding.normal
    colBackground: (root.down || root.keyboardDown) ? Appearance.colors.colPrimaryContainerActive : 
        ((root.hovered || root.focus) ? Appearance.colors.colPrimaryContainer : 
        ColorUtils.transparentize(Appearance.colors.colPrimaryContainer, 1))
    colBackgroundHover: Appearance.colors.colPrimaryContainer
    colRipple: Appearance.colors.colPrimaryContainerActive

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

    onClicked: {
        // For workflow results, don't auto-close - let the handler decide via execute.close
        // For workflow entry (starting a workflow), keep open
        // For everything else (apps, actions, etc.), close
        const isWorkflow = entry?.resultType === LauncherSearchResult.ResultType.WorkflowEntry ||
                          entry?.resultType === LauncherSearchResult.ResultType.WorkflowResult;

        root.itemExecute()

        if (!isWorkflow) {
            GlobalStates.launcherOpen = false
        }
    }
    Keys.onPressed: (event) => {
        if (event.key === Qt.Key_Delete && event.modifiers === Qt.ShiftModifier) {
            const deleteAction = root.entry.actions.find(action => action.name == "Delete");

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
        spacing: iconLoader.sourceComponent === null ? 0 : 10
        anchors.fill: parent
        anchors.leftMargin: root.horizontalMargin + root.buttonHorizontalPadding
        anchors.rightMargin: root.horizontalMargin + root.buttonHorizontalPadding

        // Icon or Thumbnail
        Loader {
            id: iconLoader
            Layout.alignment: Qt.AlignVCenter
            active: true
            sourceComponent: {
                // Thumbnail takes priority if present
                if (root.thumbnail !== "") return thumbnailComponent;
                
                switch(root.iconType) {
                    case LauncherSearchResult.IconType.Material:
                        return materialSymbolComponent
                    case LauncherSearchResult.IconType.Text:
                        return bigTextComponent
                    case LauncherSearchResult.IconType.System:
                        return iconImageComponent
                    case LauncherSearchResult.IconType.None:
                        return null
                    default:
                        return null
                }
            }
        }

        Component {
            id: thumbnailComponent
            Rectangle {
                width: 48
                height: 48
                radius: Appearance.rounding.small
                color: Appearance.colors.colSurfaceContainerHigh
                clip: true
                
                Image {
                    anchors.fill: parent
                    source: root.thumbnail ? `file://${root.thumbnail}` : ""
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    sourceSize.width: 96
                    sourceSize.height: 96
                }
            }
        }

        Component {
            id: iconImageComponent
            IconImage {
                source: Quickshell.iconPath(root.iconName, "image-missing")
                width: 35
                height: 35
            }
        }

        Component {
            id: materialSymbolComponent
            MaterialSymbol {
                text: root.materialSymbol
                iconSize: 30
                color: Appearance.m3colors.m3onSurface
            }
        }

        Component {
            id: bigTextComponent
            StyledText {
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
                    text: `${root.displayContent}`
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
            Repeater {
                model: (root.entry.actions ?? []).slice(0, 4)
                delegate: Item {
                    id: actionButton
                    required property var modelData
                    required property int index
                    property var iconType: modelData.iconType
                    property string iconName: modelData.iconName ?? ""
                    property string keyHint: (index + 1).toString()
                    implicitHeight: 34
                    implicitWidth: 34

                    Rectangle {
                        id: actionBg
                        anchors.fill: parent
                        radius: Appearance.rounding.small
                        color: actionMouse.containsMouse ? Appearance.colors.colSecondaryContainerHover : "transparent"
                        Behavior on color {
                            ColorAnimation { duration: 100 }
                        }
                    }

                    Loader {
                        anchors.centerIn: parent
                        active: actionButton.iconType === LauncherSearchResult.IconType.Material || actionButton.iconName === ""
                        sourceComponent: MaterialSymbol {
                            text: actionButton.iconName || "video_settings"
                            font.pixelSize: Appearance.font.pixelSize.hugeass
                            color: Appearance.m3colors.m3onSurface
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
                            // Clear selection immediately
                            const listView = root.ListView.view
                            if (listView) listView.currentIndex = -1
                            actionButton.modelData.execute()
                        }
                        onPressed: (event) => { event.accepted = true }
                        onReleased: (event) => { event.accepted = true }
                    }

                    StyledToolTip {
                        text: `${actionButton.modelData.name} (${actionButton.keyHint})`
                        extraVisibleCondition: actionMouse.containsMouse
                    }
                }
            }
        }

    }
}
