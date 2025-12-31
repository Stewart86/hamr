import qs
import qs.services
import qs.modules.common
import qs.modules.common.widgets
import qs.modules.common.functions
import qs.modules.imageBrowser
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Qt5Compat.GraphicalEffects

FocusScope {
    id: root

    property int columns: Config.options.imageBrowser?.columns ?? 4
    property real cellAspectRatio: Config.options.imageBrowser?.cellAspectRatio ?? (4 / 3)
    
    property var config: GlobalStates.imageBrowserConfig
    readonly property string title: config?.title ?? "Browse Images"
    readonly property var customActions: config?.actions ?? []
    readonly property string initialDirectory: config?.directory ?? ""
    readonly property bool enableOcr: config?.enableOcr ?? false

    signal imageSelected(string filePath, string actionId)
    signal cancelled()

    Component.onCompleted: {
        if (initialDirectory) {
            const expandedPath = initialDirectory.replace(/^~/, FileUtils.trimFileProtocol(Directories.home));
            FolderBrowser.setDirectory(expandedPath);
        }
        FolderBrowser.ocrEnabled = enableOcr;
        FolderBrowser.searchQuery = "";
        updateThumbnails();
        startOcrIndexing();
    }

    Component.onDestruction: {
        FolderBrowser.searchQuery = "";
    }

    function updateThumbnails() {
        const totalImageMargin = (Appearance.sizes.imageBrowserItemMargins + Appearance.sizes.imageBrowserItemPadding) * 2
        const thumbnailSizeName = Images.thumbnailSizeNameForDimensions(grid.cellWidth - totalImageMargin, grid.cellHeight - totalImageMargin)
        FolderBrowser.generateThumbnail(thumbnailSizeName)
    }
    
    function startOcrIndexing() {
        if (root.enableOcr && !FolderBrowser.ocrIndexingRunning) {
            FolderBrowser.generateOcrIndex();
        }
    }

    Connections {
        target: FolderBrowser
        function onDirectoryChanged() {
            root.updateThumbnails()
            root.startOcrIndexing()
        }
    }

    function selectFilePath(filePath) {
        if (!filePath || filePath.length === 0) return;
        const defaultAction = customActions.length > 0 ? customActions[0].id : "";
        root.imageSelected(filePath, defaultAction);
    }
    
    function selectFileWithAction(filePath, actionId) {
        if (!filePath || filePath.length === 0) return;
        root.imageSelected(filePath, actionId);
    }

    function executeActionOnCurrent(actionId) {
        const item = FolderBrowser.filteredModel.get(grid.currentIndex);
        if (item?.filePath && !item.fileIsDir) {
            root.selectFileWithAction(item.filePath, actionId);
        }
    }

    function moveSelection(delta) {
        grid.currentIndex = Math.max(0, Math.min(FolderBrowser.filteredModel.count - 1, grid.currentIndex + delta));
        grid.positionViewAtIndex(grid.currentIndex, GridView.Contain);
    }

    function activateCurrent() {
        const item = FolderBrowser.filteredModel.get(grid.currentIndex);
        if (!item) return;
        if (item.fileIsDir) {
            FolderBrowser.setDirectory(item.filePath);
        } else {
            root.selectFilePath(item.filePath);
        }
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

            StyledIndeterminateProgressBar {
                id: indeterminateProgressBar
                visible: FolderBrowser.thumbnailGenerationRunning && FolderBrowser.thumbnailGenerationProgress == 0
                anchors {
                    top: parent.top
                    left: parent.left
                    right: parent.right
                }
            }

            StyledProgressBar {
                visible: FolderBrowser.thumbnailGenerationRunning && FolderBrowser.thumbnailGenerationProgress > 0
                value: FolderBrowser.thumbnailGenerationProgress
                anchors.fill: indeterminateProgressBar
            }

            StyledProgressBar {
                visible: root.enableOcr && FolderBrowser.ocrIndexingRunning
                value: FolderBrowser.ocrIndexingProgress
                anchors {
                    top: indeterminateProgressBar.bottom
                    left: parent.left
                    right: parent.right
                    topMargin: 2
                }
                height: 2
            }

            GridView {
                id: grid
                visible: FolderBrowser.filteredModel.count > 0
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

                model: FolderBrowser.filteredModel
                onModelChanged: currentIndex = 0
                delegate: ImageBrowserItem {
                    required property var modelData
                    required property int index
                    fileModelData: modelData
                    width: grid.cellWidth
                    height: grid.cellHeight
                    colBackground: (index === grid.currentIndex || containsMouse) ? Appearance.colors.colPrimary : ColorUtils.transparentize(Appearance.colors.colPrimaryContainer)
                    colText: (index === grid.currentIndex || containsMouse) ? Appearance.colors.colOnPrimary : Appearance.colors.colOnLayer0

                    onEntered: {
                        grid.currentIndex = index;
                    }
                    
                    onActivated: {
                        if (fileModelData.fileIsDir) {
                            FolderBrowser.setDirectory(fileModelData.filePath);
                        } else {
                            root.selectFilePath(fileModelData.filePath);
                        }
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
                visible: FolderBrowser.filteredModel.count === 0
                anchors.centerIn: parent
                text: FolderBrowser.searchQuery ? "No matching images" : "No images in this folder"
                color: Appearance.colors.colSubtext
                font.pixelSize: Appearance.font.pixelSize.normal
            }
        }
    }

    Connections {
        target: LauncherSearch
        function onQueryChanged() {
            if (GlobalStates.imageBrowserOpen) {
                FolderBrowser.searchQuery = LauncherSearch.query;
            }
        }
    }
}
