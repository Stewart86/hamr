import qs
import qs.services
import qs.modules.common
import qs.modules.common.widgets
import qs.modules.common.functions
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Qt5Compat.GraphicalEffects

FocusScope {
    id: root

    property int columns: config?.columns ?? 8
    property real cellAspectRatio: config?.cellAspectRatio ?? 1.0
    
    property var config: GlobalStates.gridBrowserConfig
    readonly property string title: config?.title ?? "Select Item"
    readonly property var customActions: config?.actions ?? []
    readonly property var items: config?.items ?? []
    
    property string filterQuery: ""

    signal itemSelected(string itemId, string actionId)
    signal cancelled()

    readonly property var filteredItems: {
        if (!filterQuery || filterQuery.trim() === "") {
            return items;
        }
        const query = filterQuery.toLowerCase().trim();
        const queryWords = query.split(/\s+/);
        return items.filter(item => {
            const searchable = `${item.name ?? ""} ${(item.keywords ?? []).join(" ")}`.toLowerCase();
            return queryWords.every(word => searchable.includes(word));
        });
    }

    function selectItem(itemId) {
        if (!itemId || itemId.length === 0) return;
        const defaultAction = customActions.length > 0 ? customActions[0].id : "";
        root.itemSelected(itemId, defaultAction);
    }
    
    function selectItemWithAction(itemId, actionId) {
        if (!itemId || itemId.length === 0) return;
        root.itemSelected(itemId, actionId);
    }

    function executeActionOnCurrent(actionId) {
        const item = filteredItems[grid.currentIndex];
        if (item?.id) {
            root.selectItemWithAction(item.id, actionId);
        }
    }

    function moveSelection(delta) {
        grid.currentIndex = Math.max(0, Math.min(filteredItems.length - 1, grid.currentIndex + delta));
        grid.positionViewAtIndex(grid.currentIndex, GridView.Contain);
    }

    function activateCurrent() {
        const item = filteredItems[grid.currentIndex];
        if (!item) return;
        root.selectItem(item.id);
    }

    Keys.onPressed: event => {
        // Escape is handled by Launcher.qml
        if (event.key === Qt.Key_Escape) {
            return;
        } else if (event.key === Qt.Key_H && !(event.modifiers & Qt.ShiftModifier)) {
            // H or Ctrl+H: move left in grid
            root.moveSelection(-1);
            event.accepted = true;
        } else if (event.key === Qt.Key_L) {
            root.moveSelection(1);
            event.accepted = true;
        } else if (event.key === Qt.Key_K) {
            root.moveSelection(-root.columns);
            event.accepted = true;
        } else if (event.key === Qt.Key_J) {
            root.moveSelection(root.columns);
            event.accepted = true;
        } else if (event.key === Qt.Key_Left) {
            root.moveSelection(-1);
            event.accepted = true;
        } else if (event.key === Qt.Key_Right) {
            root.moveSelection(1);
            event.accepted = true;
        } else if (event.key === Qt.Key_Up) {
            root.moveSelection(-root.columns);
            event.accepted = true;
        } else if (event.key === Qt.Key_Down) {
            root.moveSelection(root.columns);
            event.accepted = true;
        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            root.activateCurrent();
            event.accepted = true;
        } else if ((event.modifiers & Qt.ControlModifier) && event.key >= Qt.Key_1 && event.key <= Qt.Key_6) {
            const actionIndex = event.key - Qt.Key_1;
            if (actionIndex < root.customActions.length) {
                root.executeActionOnCurrent(root.customActions[actionIndex].id);
            }
            event.accepted = true;
        }
    }

    implicitWidth: Appearance.sizes.imageBrowserGridWidth
    implicitHeight: gridContainer.implicitHeight

    ColumnLayout {
        id: gridContainer
        anchors.fill: parent
        spacing: 0

        Item {
            id: gridDisplayRegion
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(
                Appearance.sizes.imageBrowserGridHeight,
                grid.contentHeight + grid.topMargin + grid.bottomMargin + 16
            )

            GridView {
                id: grid
                visible: root.filteredItems.length > 0
                focus: true

                readonly property int columns: root.columns
                property int currentIndex: 0

                anchors {
                    fill: parent
                    topMargin: 8
                }
                cellWidth: width / root.columns
                cellHeight: cellWidth / root.cellAspectRatio
                interactive: true
                clip: true
                keyNavigationWraps: true
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: StyledScrollBar {}

                model: root.filteredItems
                onModelChanged: currentIndex = 0
                delegate: GridViewItem {
                    required property var modelData
                    required property int index
                    itemData: modelData
                    width: grid.cellWidth
                    height: grid.cellHeight
                    colBackground: (index === grid.currentIndex || containsMouse) ? Appearance.colors.colPrimary : ColorUtils.transparentize(Appearance.colors.colPrimaryContainer)
                    colText: (index === grid.currentIndex || containsMouse) ? Appearance.colors.colOnPrimary : Appearance.colors.colOnLayer0

                    onEntered: {
                        grid.currentIndex = index;
                    }
                    
                    onActivated: {
                        root.selectItem(itemData.id);
                    }
                }

                layer.enabled: true
                layer.effect: OpacityMask {
                    maskSource: Rectangle {
                        width: gridDisplayRegion.width
                        height: gridDisplayRegion.height
                        radius: Appearance.rounding.small
                    }
                }
            }

            StyledText {
                visible: root.filteredItems.length === 0
                anchors.centerIn: parent
                anchors.verticalCenterOffset: 0
                topPadding: 40
                bottomPadding: 40
                text: root.filterQuery ? "No matching items" : "No items"
                color: Appearance.colors.colSubtext
                font.pixelSize: Appearance.font.pixelSize.normal
            }
        }
    }

    Connections {
        target: LauncherSearch
        function onQueryChanged() {
            if (GlobalStates.gridBrowserOpen) {
                root.filterQuery = LauncherSearch.query;
            }
        }
    }
}
