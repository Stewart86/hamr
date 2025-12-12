import qs
import qs.services
import qs.modules.common
import qs.modules.common.widgets
import qs.modules.common.functions
import Qt.labs.synchronizer
import Qt5Compat.GraphicalEffects
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

Item { // Wrapper
    id: root
    readonly property string xdgConfigHome: Directories.config
    property string searchingText: LauncherSearch.query
    property bool showResults: searchingText != "" || LauncherSearch.results.length > 0
    property bool showCard: WorkflowRunner.workflowCard !== null
    // Include the full visual height (content + offset)
    implicitWidth: searchWidgetContent.implicitWidth + Appearance.sizes.elevationMargin * 2
    implicitHeight: searchWidgetContent.implicitHeight + searchWidgetContent.anchors.topMargin + Appearance.sizes.elevationMargin * 2
    
    // Expose the content rectangle for input masking
    property alias contentItem: searchWidgetContent

    function focusFirstItem() {
        appResults.currentIndex = 0;
    }

    function focusSearchInput() {
        searchBar.forceFocus();
    }

    function disableExpandAnimation() {
        searchBar.animateWidth = false;
    }

    function cancelSearch() {
        searchBar.searchInput.selectAll();
        LauncherSearch.query = "";
        searchBar.animateWidth = true;
    }

    function setSearchingText(text) {
        searchBar.searchInput.text = text;
        LauncherSearch.query = text;
    }

    Keys.onPressed: event => {
        // Prevent Esc and Backspace from registering
        if (event.key === Qt.Key_Escape)
            return;

        // Handle Backspace: focus and delete character if not focused
        if (event.key === Qt.Key_Backspace) {
            if (!searchBar.searchInput.activeFocus) {
                root.focusSearchInput();
                if (event.modifiers & Qt.ControlModifier) {
                    // Delete word before cursor
                    let text = searchBar.searchInput.text;
                    let pos = searchBar.searchInput.cursorPosition;
                    if (pos > 0) {
                        // Find the start of the previous word
                        let left = text.slice(0, pos);
                        let match = left.match(/(\s*\S+)\s*$/);
                        let deleteLen = match ? match[0].length : 1;
                        searchBar.searchInput.text = text.slice(0, pos - deleteLen) + text.slice(pos);
                        searchBar.searchInput.cursorPosition = pos - deleteLen;
                    }
                } else {
                    // Delete character before cursor if any
                    if (searchBar.searchInput.cursorPosition > 0) {
                        searchBar.searchInput.text = searchBar.searchInput.text.slice(0, searchBar.searchInput.cursorPosition - 1) + searchBar.searchInput.text.slice(searchBar.searchInput.cursorPosition);
                        searchBar.searchInput.cursorPosition -= 1;
                    }
                }
                // Always move cursor to end after programmatic edit
                searchBar.searchInput.cursorPosition = searchBar.searchInput.text.length;
                event.accepted = true;
            }
            // If already focused, let TextField handle it
            return;
        }

        // Only handle visible printable characters (ignore control chars, arrows, etc.)
        if (event.text && event.text.length === 1 && event.key !== Qt.Key_Enter && event.key !== Qt.Key_Return && event.key !== Qt.Key_Delete && event.text.charCodeAt(0) >= 0x20) // ignore control chars like Backspace, Tab, etc.
        {
            if (!searchBar.searchInput.activeFocus) {
                root.focusSearchInput();
                // Insert the character at the cursor position
                searchBar.searchInput.text = searchBar.searchInput.text.slice(0, searchBar.searchInput.cursorPosition) + event.text + searchBar.searchInput.text.slice(searchBar.searchInput.cursorPosition);
                searchBar.searchInput.cursorPosition += 1;
                event.accepted = true;
                root.focusFirstItem();
            }
        }
    }

    StyledRectangularShadow {
        target: searchWidgetContent
    }
    Rectangle { // Background
        id: searchWidgetContent
        anchors {
            top: parent.top
            horizontalCenter: parent.horizontalCenter
            topMargin: Appearance.sizes.elevationMargin * 20
        }
        clip: true
        implicitWidth: columnLayout.implicitWidth
        implicitHeight: columnLayout.implicitHeight
        radius: searchBar.height / 2 + searchBar.padding
        color: Appearance.colors.colBackgroundSurfaceContainer

        Behavior on implicitHeight {
            id: searchHeightBehavior
            enabled: GlobalStates.launcherOpen && (root.showResults || root.showCard)
            animation: Appearance.animation.elementMove.numberAnimation.createObject(this)
        }

        ColumnLayout {
            id: columnLayout
            anchors {
                top: parent.top
                horizontalCenter: parent.horizontalCenter
            }
            spacing: 0

            // clip: true
            layer.enabled: true
            layer.effect: OpacityMask {
                maskSource: Rectangle {
                    width: searchWidgetContent.width
                    height: searchWidgetContent.height
                    radius: searchWidgetContent.radius
                }
            }

            SearchBar {
                id: searchBar
                property real padding: 4
                Layout.fillWidth: true
                Layout.leftMargin: padding
                Layout.rightMargin: padding
                Layout.topMargin: padding
                Layout.bottomMargin: padding
                Synchronizer on searchingText {
                    property alias source: root.searchingText
                }
                
                onNavigateDown: {
                    if (appResults.currentIndex < appResults.count - 1) {
                        appResults.currentIndex++;
                    }
                }
                onNavigateUp: {
                    if (appResults.currentIndex > 0) {
                        appResults.currentIndex--;
                    }
                }
                onSelectCurrent: {
                    if (appResults.count > 0) {
                        let currentItem = appResults.itemAtIndex(appResults.currentIndex);
                        if (currentItem && currentItem.clicked) {
                            currentItem.clicked();
                        }
                    }
                }
            }

            // Hint bar - shows prefix shortcuts when query is empty, navigation hints when results shown
            RowLayout {
                id: hintBar
                visible: !WorkflowRunner.isActive()
                Layout.fillWidth: true
                Layout.leftMargin: 16
                Layout.rightMargin: 16
                Layout.bottomMargin: 8
                spacing: 16
                
                // Prefix hints (always shown when no workflow active)
                Repeater {
                    model: [
                        { key: "~", label: "files" },
                        { key: ";", label: "clipboard" },
                        { key: "/", label: "actions" },
                        { key: "!", label: "shell" },
                        { key: "=", label: "math" },
                        { key: ":", label: "emoji" },
                    ]
                    
                    Text {
                        required property var modelData
                        text: `<font color="${Appearance.colors.colPrimary}">${modelData.key}</font> ${modelData.label}`
                        textFormat: Text.RichText
                        font.pixelSize: Appearance.font.pixelSize.smaller
                        color: Appearance.m3colors.m3outline
                    }
                }
                
                // Separator
                Text {
                    visible: root.showResults
                    text: "|"
                    font.pixelSize: Appearance.font.pixelSize.smaller
                    color: Appearance.colors.colOutlineVariant
                }
                
                // Navigation hints (shown when results are displayed)
                Repeater {
                    model: root.showResults ? [
                        { key: "^J", label: "down" },
                        { key: "^K", label: "up" },
                        { key: "^L", label: "select" },
                        { key: "1-4", label: "actions" },
                    ] : []
                    
                    Text {
                        required property var modelData
                        text: `<font color="${Appearance.colors.colPrimary}">${modelData.key}</font> ${modelData.label}`
                        textFormat: Text.RichText
                        font.pixelSize: Appearance.font.pixelSize.smaller
                        color: Appearance.m3colors.m3outline
                    }
                }
            }

            Rectangle {
                // Separator
                visible: root.showResults || root.showCard
                Layout.fillWidth: true
                height: 1
                color: Appearance.colors.colOutlineVariant
            }

            // Workflow card display (shown instead of results when card is present)
            WorkflowCard {
                id: workflowCard
                visible: root.showCard
                Layout.fillWidth: true
                Layout.maximumHeight: 400
                card: WorkflowRunner.workflowCard
            }

            ListView { // App results
                id: appResults
                visible: root.showResults && !root.showCard
                Layout.fillWidth: true
                implicitHeight: Math.min(600, appResults.contentHeight + topMargin + bottomMargin)
                clip: true
                topMargin: 10
                bottomMargin: 10
                spacing: 2
                KeyNavigation.up: searchBar
                highlightMoveDuration: 100

                onFocusChanged: {
                    if (focus)
                        appResults.currentIndex = 1;
                }

                Connections {
                    target: root
                    function onSearchingTextChanged() {
                        if (appResults.count > 0)
                            appResults.currentIndex = 0;
                    }
                }

                model: ScriptModel {
                    id: model
                    objectProp: "key"
                    values: LauncherSearch.results
                    onValuesChanged: {
                        if (LauncherSearch.skipNextAutoFocus) {
                            LauncherSearch.skipNextAutoFocus = false;
                            appResults.currentIndex = -1;
                            return;
                        }
                        root.focusFirstItem();
                    }
                }

                delegate: SearchItem {
                    // The selectable item for each search result
                    required property var modelData
                    anchors.left: parent?.left
                    anchors.right: parent?.right
                    entry: modelData
                    query: StringUtils.cleanOnePrefix(root.searchingText, [
                        Config.options.search.prefix.action,
                        Config.options.search.prefix.app,
                        Config.options.search.prefix.clipboard,
                        Config.options.search.prefix.emojis,
                        Config.options.search.prefix.math,
                        Config.options.search.prefix.shellCommand,
                        Config.options.search.prefix.webSearch
                    ])
                }
            }
        }
    }
}
