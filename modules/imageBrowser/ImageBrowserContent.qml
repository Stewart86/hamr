import qs
import qs.services
import qs.modules.common
import qs.modules.common.widgets
import qs.modules.common.functions
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Qt5Compat.GraphicalEffects
import Quickshell
import Quickshell.Io

MouseArea {
    id: root
    property int columns: 4
    property real previewCellAspectRatio: 4 / 3
    
    // Workflow configuration
    property var config: null  // { directory, title, extensions, actions, workflowId, enableOcr }
    
    // Computed properties based on config
    readonly property string title: config?.title ?? "Browse Images"
    readonly property var customActions: config?.actions ?? []
    readonly property string initialDirectory: config?.directory ?? ""
    readonly property bool enableOcr: config?.enableOcr ?? false

    Component.onCompleted: {
        if (initialDirectory) {
            // Set initial directory from workflow config
            const expandedPath = initialDirectory.replace(/^~/, FileUtils.trimFileProtocol(Directories.home));
            FolderBrowser.setDirectory(expandedPath);
        }
        // Enable OCR if configured
        FolderBrowser.ocrEnabled = enableOcr;
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
            // Start OCR indexing when directory changes (if enabled)
            root.startOcrIndexing()
        }
    }

    function handleFilePasting(event) {
        const clipboardText = Quickshell.clipboardText ?? "";
        // Check if clipboard contains a file path
        if (clipboardText.startsWith("file://") || clipboardText.startsWith("/")) {
            const path = clipboardText.startsWith("file://") 
                ? FileUtils.trimFileProtocol(decodeURIComponent(clipboardText))
                : clipboardText;
            FolderBrowser.setDirectory(path);
            event.accepted = true;
        } else {
            event.accepted = false;
        }
    }

    // Handle file selection - send selection back to workflow handler
    function selectFilePath(filePath) {
        if (!filePath || filePath.length === 0) return;
        
        // Default action is the first custom action, or empty string
        const defaultAction = customActions.length > 0 ? customActions[0].id : "";
        GlobalStates.imageBrowserSelection(filePath, defaultAction);
    }
    
    // Handle action button click
    function selectFileWithAction(filePath, actionId) {
        if (!filePath || filePath.length === 0) return;
        GlobalStates.imageBrowserSelection(filePath, actionId);
    }

    acceptedButtons: Qt.BackButton | Qt.ForwardButton
    onPressed: event => {
        if (event.button === Qt.BackButton) {
            FolderBrowser.navigateBack();
        } else if (event.button === Qt.ForwardButton) {
            FolderBrowser.navigateForward();
        }
    }

    Keys.onPressed: event => {
        if (event.key === Qt.Key_Escape) {
            GlobalStates.closeImageBrowser();
            event.accepted = true;
        } else if ((event.modifiers & Qt.ControlModifier) && event.key === Qt.Key_V) {
            root.handleFilePasting(event);
        } else if (event.modifiers & Qt.AltModifier && event.key === Qt.Key_Up) {
            FolderBrowser.navigateUp();
            event.accepted = true;
        } else if (event.modifiers & Qt.AltModifier && event.key === Qt.Key_Left) {
            FolderBrowser.navigateBack();
            event.accepted = true;
        } else if (event.modifiers & Qt.AltModifier && event.key === Qt.Key_Right) {
            FolderBrowser.navigateForward();
            event.accepted = true;
        // Vim navigation: h/j/k/l for grid movement, H for parent directory
        } else if (event.key === Qt.Key_H && (event.modifiers & Qt.ShiftModifier)) {
            FolderBrowser.navigateUp();
            event.accepted = true;
        } else if (event.key === Qt.Key_H) {
            grid.moveSelection(-1);
            event.accepted = true;
        } else if (event.key === Qt.Key_L) {
            grid.moveSelection(1);
            event.accepted = true;
        } else if (event.key === Qt.Key_K) {
            grid.moveSelection(-grid.columns);
            event.accepted = true;
        } else if (event.key === Qt.Key_J) {
            grid.moveSelection(grid.columns);
            event.accepted = true;
        } else if (event.key === Qt.Key_Left) {
            grid.moveSelection(-1);
            event.accepted = true;
        } else if (event.key === Qt.Key_Right) {
            grid.moveSelection(1);
            event.accepted = true;
        } else if (event.key === Qt.Key_Up) {
            grid.moveSelection(-grid.columns);
            event.accepted = true;
        } else if (event.key === Qt.Key_Down) {
            grid.moveSelection(grid.columns);
            event.accepted = true;
        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            grid.activateCurrent();
            event.accepted = true;
        } else if (event.key === Qt.Key_Backspace) {
            if (filterField.text.length > 0) {
                filterField.text = filterField.text.substring(0, filterField.text.length - 1);
            } else {
                FolderBrowser.navigateUp();
            }
            event.accepted = true;
        } else if (event.modifiers & Qt.ControlModifier && event.key === Qt.Key_L) {
            addressBar.focusBreadcrumb();
            event.accepted = true;
        } else if (event.key === Qt.Key_Slash) {
            filterField.forceActiveFocus();
            event.accepted = true;
        } else if (event.key === Qt.Key_Minus) {
            // Minus key to go up one directory (like vim-vinegar)
            FolderBrowser.navigateUp();
            event.accepted = true;
        } else {
            if (event.text.length > 0) {
                filterField.text += event.text;
                filterField.cursorPosition = filterField.text.length;
                filterField.forceActiveFocus();
            }
            event.accepted = true;
        }
    }

    implicitHeight: mainLayout.implicitHeight
    implicitWidth: mainLayout.implicitWidth

    StyledRectangularShadow {
        target: browserBackground
    }
    Rectangle {
        id: browserBackground
        anchors {
            fill: parent
            margins: Appearance.sizes.elevationMargin
        }
        focus: true
        border.width: 1
        border.color: Appearance.colors.colLayer0Border
        color: Appearance.colors.colLayer0
        radius: Appearance.rounding.normal

        property int calculatedRows: Math.ceil(grid.count / grid.columns)

        implicitWidth: gridColumnLayout.implicitWidth
        implicitHeight: gridColumnLayout.implicitHeight

        RowLayout {
            id: mainLayout
            anchors.fill: parent
            spacing: -4

            Rectangle {
                Layout.fillHeight: true
                Layout.margins: 4
                implicitWidth: quickDirColumnLayout.implicitWidth
                implicitHeight: quickDirColumnLayout.implicitHeight
                color: Appearance.colors.colLayer1
                radius: browserBackground.radius - Layout.margins

                ColumnLayout {
                    id: quickDirColumnLayout
                    anchors.fill: parent
                    spacing: 0

                    StyledText {
                        Layout.margins: 12
                        font {
                            pixelSize: Appearance.font.pixelSize.normal
                            weight: Font.Medium
                        }
                        text: root.title
                    }
                    ListView {
                        // Quick dirs
                        Layout.fillHeight: true
                        Layout.margins: 4
                        implicitWidth: 140
                        clip: true
                        model: [
                            { icon: "home", name: "Home", path: Directories.home }, 
                            { icon: "docs", name: "Documents", path: Directories.documents }, 
                            { icon: "download", name: "Downloads", path: Directories.downloads }, 
                            { icon: "image", name: "Pictures", path: Directories.pictures }, 
                            { icon: "movie", name: "Videos", path: Directories.videos }, 
                            { icon: "", name: "---", path: "INTENTIONALLY_INVALID_DIR" }, 
                            { icon: "wallpaper", name: "Wallpapers", path: `${Directories.pictures}/Wallpapers` }, 
                        ]
                        delegate: RippleButton {
                            id: quickDirButton
                            required property var modelData
                            anchors {
                                left: parent.left
                                right: parent.right
                            }
                            onClicked: FolderBrowser.setDirectory(quickDirButton.modelData.path)
                            enabled: modelData.icon.length > 0
                            toggled: FolderBrowser.directory === Qt.resolvedUrl(modelData.path)
                            colBackgroundToggled: Appearance.colors.colSecondaryContainer
                            colBackgroundToggledHover: Appearance.colors.colSecondaryContainerHover
                            colRippleToggled: Appearance.colors.colSecondaryContainerActive
                            buttonRadius: Appearance.rounding.verysmall
                            implicitHeight: 38

                            contentItem: RowLayout {
                                MaterialSymbol {
                                    color: quickDirButton.toggled ? Appearance.colors.colOnSecondaryContainer : Appearance.colors.colOnLayer1
                                    iconSize: Appearance.font.pixelSize.larger
                                    text: quickDirButton.modelData.icon
                                    fill: quickDirButton.toggled ? 1 : 0
                                }
                                StyledText {
                                    Layout.fillWidth: true
                                    horizontalAlignment: Text.AlignLeft
                                    color: quickDirButton.toggled ? Appearance.colors.colOnSecondaryContainer : Appearance.colors.colOnLayer1
                                    text: quickDirButton.modelData.name
                                }
                            }
                        }
                    }

                    // Key guide
                    ColumnLayout {
                        id: keyGuideColumn
                        Layout.fillWidth: true
                        Layout.margins: 8
                        Layout.topMargin: 0
                        spacing: 4

                        StyledText {
                            font {
                                pixelSize: Appearance.font.pixelSize.smaller
                                weight: Font.Medium
                            }
                            color: Appearance.colors.colOnLayer1
                            text: "Keys"
                        }

                        Repeater {
                            model: [
                                { keys: "h j k l", desc: "Navigate" },
                                { keys: "Enter", desc: "Open" },
                                { keys: "- or H", desc: "Parent dir" },
                                { keys: "/", desc: "Search" },
                                { keys: "Esc", desc: "Close" },
                            ]
                            delegate: RowLayout {
                                required property var modelData
                                Layout.fillWidth: true
                                spacing: 4

                                StyledText {
                                    font.pixelSize: Appearance.font.pixelSize.smaller
                                    color: Appearance.colors.colOnLayer1
                                    text: modelData.keys
                                    Layout.preferredWidth: 56
                                }
                                StyledText {
                                    font.pixelSize: Appearance.font.pixelSize.smaller
                                    color: Appearance.colors.colOnLayer1
                                    text: modelData.desc
                                    Layout.fillWidth: true
                                }
                            }
                        }
                    }
                }
            }

            ColumnLayout {
                id: gridColumnLayout
                Layout.fillWidth: true
                Layout.fillHeight: true

                AddressBar {
                    id: addressBar
                    Layout.margins: 4
                    Layout.fillWidth: true
                    Layout.fillHeight: false
                    directory: FolderBrowser.effectiveDirectory
                    onNavigateToDirectory: path => {
                        FolderBrowser.setDirectory(path.length == 0 ? "/" : path);
                    }
                    radius: browserBackground.radius - Layout.margins
                }

                Item {
                    id: gridDisplayRegion
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    // Progress bar for thumbnail generation
                    StyledIndeterminateProgressBar {
                        id: indeterminateProgressBar
                        visible: FolderBrowser.thumbnailGenerationRunning && value == 0
                        anchors {
                            bottom: parent.top
                            left: parent.left
                            right: parent.right
                            leftMargin: 4
                            rightMargin: 4
                        }
                    }

                    StyledProgressBar {
                        visible: FolderBrowser.thumbnailGenerationRunning && value > 0
                        value: FolderBrowser.thumbnailGenerationProgress
                        anchors.fill: indeterminateProgressBar
                    }
                    
                    // Secondary progress bar for OCR indexing
                    StyledProgressBar {
                        visible: root.enableOcr && FolderBrowser.ocrIndexingRunning
                        value: FolderBrowser.ocrIndexingProgress
                        anchors {
                            bottom: indeterminateProgressBar.top
                            left: parent.left
                            right: parent.right
                            leftMargin: 4
                            rightMargin: 4
                            bottomMargin: 2
                        }
                        height: 2
                    }

                    GridView {
                        id: grid
                        visible: FolderBrowser.filteredModel.count > 0

                        readonly property int columns: root.columns
                        readonly property int rows: Math.max(1, Math.ceil(count / columns))
                        property int currentIndex: 0

                        anchors.fill: parent
                        cellWidth: width / root.columns
                        cellHeight: cellWidth / root.previewCellAspectRatio
                        interactive: true
                        clip: true
                        keyNavigationWraps: true
                        boundsBehavior: Flickable.StopAtBounds
                        bottomMargin: extraOptions.implicitHeight
                        ScrollBar.vertical: StyledScrollBar {}

                        Component.onCompleted: {
                            root.updateThumbnails()
                            root.startOcrIndexing()
                        }

                        function moveSelection(delta) {
                            currentIndex = Math.max(0, Math.min(grid.model.count - 1, currentIndex + delta));
                            positionViewAtIndex(currentIndex, GridView.Contain);
                        }

                        function activateCurrent() {
                            const item = FolderBrowser.filteredModel.get(currentIndex);
                            if (!item) return;
                            if (item.fileIsDir) {
                                FolderBrowser.setDirectory(item.filePath);
                            } else {
                                root.selectFilePath(item.filePath);
                            }
                        }

                        model: FolderBrowser.filteredModel
                        onModelChanged: currentIndex = 0
                        delegate: ImageBrowserItem {
                            required property var modelData
                            required property int index
                            fileModelData: modelData
                            width: grid.cellWidth
                            height: grid.cellHeight
                            colBackground: (index === grid?.currentIndex || containsMouse) ? Appearance.colors.colPrimary : ColorUtils.transparentize(Appearance.colors.colPrimaryContainer)
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
                                radius: browserBackground.radius
                            }
                        }
                    }

                    Toolbar {
                        id: extraOptions
                        anchors {
                            bottom: parent.bottom
                            horizontalCenter: parent.horizontalCenter
                            bottomMargin: 8
                        }

                        // Custom action buttons from workflow config
                        Repeater {
                            model: root.customActions
                            delegate: IconToolbarButton {
                                required property var modelData
                                implicitWidth: height
                                text: modelData.icon ?? "play_arrow"
                                onClicked: {
                                    const item = FolderBrowser.filteredModel.get(grid.currentIndex);
                                    if (item?.filePath) {
                                        root.selectFileWithAction(item.filePath, modelData.id);
                                    }
                                }
                                StyledToolTip {
                                    text: modelData.name ?? ""
                                }
                            }
                        }

                        ToolbarTextField {
                            id: filterField
                            placeholderText: root.enableOcr 
                                ? (focus ? "Search images or text..." : "Hit \"/\" to search")
                                : (focus ? "Search images" : "Hit \"/\" to search")

                            // Style
                            clip: true
                            font.pixelSize: Appearance.font.pixelSize.small

                            // Search
                            onTextChanged: {
                                FolderBrowser.searchQuery = text;
                            }

                            Keys.onPressed: event => {
                                if ((event.modifiers & Qt.ControlModifier) && event.key === Qt.Key_V) {
                                    root.handleFilePasting(event);
                                    return;
                                }
                                else if (text.length !== 0) {
                                    if (event.key === Qt.Key_Down) {
                                        grid.moveSelection(grid.columns);
                                        event.accepted = true;
                                        return;
                                    }
                                    if (event.key === Qt.Key_Up) {
                                        grid.moveSelection(-grid.columns);
                                        event.accepted = true;
                                        return;
                                    }
                                }
                                event.accepted = false;
                            }
                        }

                        IconToolbarButton {
                            implicitWidth: height
                            onClicked: {
                                GlobalStates.closeImageBrowser();
                            }
                            text: "close"
                            StyledToolTip {
                                text: "Close"
                            }
                        }
                    }
                }
            }
        }
    }

    Connections {
        target: GlobalStates
        function onImageBrowserOpenChanged() {
            if (GlobalStates.imageBrowserOpen) {
                filterField.forceActiveFocus();
            }
        }
    }
}
