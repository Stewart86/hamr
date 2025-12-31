import qs
import qs.services
import qs.modules.common
import qs.modules.common.widgets
import Qt.labs.synchronizer
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Shapes
import Qt5Compat.GraphicalEffects
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Hyprland

Scope {
    id: launcherScope
    property bool dontAutoCancelSearch: false

    // Track if state is pending cleanup (soft close occurred, waiting for timeout or reopen)
    property bool statePendingCleanup: false

    Connections {
        target: GlobalStates
        function onLauncherMinimizedChanged() {
            if (GlobalStates.launcherMinimized) {
                Persistent.states.launcher.hasUsedMinimize = true;
                const restoreWindowMs = Config.options.behavior.stateRestoreWindowMs;
                if (restoreWindowMs > 0) {
                    launcherScope.statePendingCleanup = true;
                    deferredCleanupTimer.restart();
                }
            } else {
                if (launcherScope.statePendingCleanup) {
                    deferredCleanupTimer.stop();
                    launcherScope.statePendingCleanup = false;
                }
            }
        }
    }

    // Write launch timestamp for plugins that need to know when hamr was opened
    // (e.g., screenrecord plugin uses this to trim the end of recordings)
    Process {
        id: launchTimestampProc
        command: ["bash", "-c", "mkdir -p ~/.cache/hamr && date +%s%3N > ~/.cache/hamr/launch_timestamp"]
    }

    // Deferred cleanup timer for soft close (click-outside) and minimize
    // When timer fires, actually clean up state
    Timer {
        id: deferredCleanupTimer
        interval: Config.options.behavior.stateRestoreWindowMs
        repeat: false
        onTriggered: {
            // Guard: only bail if launcher is fully open (not minimized)
            // If minimized, we still want to clean up state after timeout
            if (GlobalStates.launcherOpen && !GlobalStates.launcherMinimized) {
                launcherScope.statePendingCleanup = false;
                return;
            }

            // Timer expired - perform actual cleanup
            if (PluginRunner.isActive()) {
                LauncherSearch.closePlugin();
            }
            if (LauncherSearch.isInExclusiveMode()) {
                LauncherSearch.exclusiveMode = "";
            }
            LauncherSearch.query = "";
            launcherScope.statePendingCleanup = false;
        }
    }

    // Perform immediate cleanup (hard close)
    function performImmediateCleanup() {
        deferredCleanupTimer.stop();
        if (PluginRunner.isActive()) {
            LauncherSearch.closePlugin();
        }
        if (LauncherSearch.isInExclusiveMode()) {
            LauncherSearch.exclusiveMode = "";
        }
        LauncherSearch.query = "";
        launcherScope.statePendingCleanup = false;
    }

    Connections {
        target: GlobalStates
        function onLauncherOpenChanged() {
            if (GlobalStates.launcherOpen) {
                launchTimestampProc.running = true;

                // === OPENING (single handler, not per-screen) ===
                if (launcherScope.statePendingCleanup) {
                    // Reopening within restore window - cancel cleanup, preserve state
                    deferredCleanupTimer.stop();
                    launcherScope.statePendingCleanup = false;
                } else if (!launcherScope.dontAutoCancelSearch) {
                    // Normal open - start fresh
                    launcherScope.cancelSearchOnAllScreens();
                }
            } else {
                // === CLOSING (single handler, not per-screen) ===
                const restoreWindowMs = Config.options.behavior.stateRestoreWindowMs;

                if (GlobalStates.softClose && restoreWindowMs > 0) {
                    // Soft close (click-outside): defer cleanup, allow state restore
                    launcherScope.statePendingCleanup = true;
                    deferredCleanupTimer.restart();
                } else {
                    // Hard close (Escape, execute-with-close, etc.): immediate cleanup
                    launcherScope.performImmediateCleanup();
                }

                // Reset softClose flag for next close
                GlobalStates.softClose = false;

                // Reset dontAutoCancelSearch
                launcherScope.dontAutoCancelSearch = false;

                // Hide action tooltip
                GlobalStates.hideActionToolTip();
            }
        }
    }

    // Helper to cancel search - needs to call into the correct screen's searchWidget
    function cancelSearchOnAllScreens() {
        for (let i = 0; i < launcherVariants.instances.length; i++) {
            let panelWindow = launcherVariants.instances[i];
            if (panelWindow.monitorIsFocused) {
                panelWindow.searchWidget.cancelSearch();
                break;
            }
        }
    }

    Variants {
        id: launcherVariants
        model: Quickshell.screens
        PanelWindow {
            id: root
            required property var modelData
            property string searchingText: ""
            readonly property var hyprlandMonitor: Hyprland.monitorFor(root.screen)
            property bool monitorIsFocused: false
            property alias searchWidget: searchWidget
            screen: modelData
            visible: GlobalStates.launcherOpen && monitorIsFocused && !GlobalStates.launcherMinimized
            
            function updateMonitorFocus() {
                if (CompositorService.isHyprland) {
                    monitorIsFocused = Hyprland.focusedMonitor?.id == hyprlandMonitor?.id;
                } else if (CompositorService.isNiri) {
                    monitorIsFocused = modelData.name === NiriService.currentOutput;
                } else {
                    monitorIsFocused = true;
                }
            }
            
            Connections {
                target: GlobalStates
                function onLauncherOpenChanged() {
                    if (GlobalStates.launcherOpen || GlobalStates.launcherMinimized) {
                        root.updateMonitorFocus();
                    }
                }
                function onLauncherMinimizedChanged() {
                    if (GlobalStates.launcherOpen || GlobalStates.launcherMinimized) {
                        root.updateMonitorFocus();
                    }
                }
            }
            
            Connections {
                target: NiriService
                enabled: CompositorService.isNiri && (GlobalStates.launcherOpen || GlobalStates.launcherMinimized)
                function onCurrentOutputChanged() {
                    root.updateMonitorFocus();
                }
            }
            
            Connections {
                target: CompositorService.isHyprland ? Hyprland : null
                enabled: CompositorService.isHyprland && (GlobalStates.launcherOpen || GlobalStates.launcherMinimized)
                function onFocusedMonitorChanged() {
                    root.updateMonitorFocus();
                }
            }
            
            Component.onCompleted: updateMonitorFocus()

            WlrLayershell.namespace: "quickshell:hamr"
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.OnDemand
            exclusionMode: ExclusionMode.Ignore
            color: "transparent"

            mask: Region {
                item: GlobalStates.launcherOpen ? fullScreenBackground : null
            }

            anchors {
                top: true
                bottom: true
                left: true
                right: true
            }

            FocusGrab {
                id: grab
                window: root
                active: false
                focusTarget: searchWidget.searchInput

                property bool canBeActive: root.monitorIsFocused
            }

            // Re-grab focus when user clicks anywhere on the launcher content
            function regrabFocus() {
                if (!GlobalStates.launcherOpen)
                    return;
                if (!grab.canBeActive)
                    return;

                grab.regrabFocus();
            }

            Connections {
                target: GlobalStates
                function onLauncherOpenChanged() {
                    if (!GlobalStates.launcherOpen) {
                        // Per-screen UI cleanup on close
                        searchWidget.disableExpandAnimation();
                        grab.deactivate();
                    } else {
                        // Per-screen UI setup on open
                        delayedGrabTimer.start();
                    }
                }
            }

            // Handle plugin execute with close
            Connections {
                target: PluginRunner
                function onExecuteCommand(command) {
                    if (command.close) {
                        // Hard close - task completed, don't preserve state
                        LauncherSearch.closePlugin();
                        GlobalStates.softClose = false;
                        GlobalStates.launcherOpen = false;
                    }
                }
            }

            Timer {
                id: delayedGrabTimer
                interval: 20 // Race condition delay
                repeat: false
                onTriggered: {
                    if (!grab.canBeActive)
                        return;
                    if (GlobalStates.launcherOpen) {
                        grab.activate();
                        searchWidget.focusSearchInput();
                    }
                }
            }

            implicitWidth: columnLayout.implicitWidth
            implicitHeight: columnLayout.implicitHeight

            function setSearchingText(text) {
                searchWidget.setSearchingText(text);
                searchWidget.focusFirstItem();
            }

            // Full screen background Item for mask and click detection
            Item {
                id: fullScreenBackground
                anchors.fill: parent

                MouseArea {
                    anchors.fill: parent
                    visible: GlobalStates.launcherOpen
                    onClicked: mouse => {
                        const content = searchWidget.contentItem;
                        const contentMapped = content.mapToItem(fullScreenBackground, 0, 0);

                        if (mouse.x < contentMapped.x || mouse.x > contentMapped.x + content.width || mouse.y < contentMapped.y || mouse.y > contentMapped.y + content.height) {
                            const action = Config.options.behavior?.clickOutsideAction ?? "intuitive";
                            const shouldMinimize = action === "minimize" || (action === "intuitive" && Persistent.states.launcher.hasUsedMinimize);

                            if (shouldMinimize) {
                                GlobalStates.launcherMinimized = true;
                            } else {
                                GlobalStates.softClose = true;
                                GlobalStates.launcherOpen = false;
                            }
                        }
                    }
                }
            }

            // Floating action tooltip popup - above all launcher content
            Rectangle {
                id: actionToolTipPopup
                visible: opacity > 0
                opacity: GlobalStates.actionToolTipVisible ? 1 : 0
                scale: GlobalStates.actionToolTipVisible ? 1 : 0.9
                z: 1000

                // Convert global position to local
                x: {
                    const localPos = fullScreenBackground.mapFromGlobal(GlobalStates.actionToolTipPosition.x, GlobalStates.actionToolTipPosition.y);
                    return localPos.x - width / 2;
                }
                y: {
                    const localPos = fullScreenBackground.mapFromGlobal(GlobalStates.actionToolTipPosition.x, GlobalStates.actionToolTipPosition.y);
                    return localPos.y;
                }

                implicitWidth: toolTipContent.implicitWidth + 12
                implicitHeight: toolTipContent.implicitHeight + 6
                radius: 4
                color: Appearance.colors.colSurfaceContainerHigh
                border.width: 1
                border.color: Appearance.colors.colOutlineVariant

                Behavior on opacity {
                    NumberAnimation {
                        duration: 150
                        easing.type: Easing.OutCubic
                    }
                }
                Behavior on scale {
                    NumberAnimation {
                        duration: 150
                        easing.type: Easing.OutCubic
                    }
                }

                RowLayout {
                    id: toolTipContent
                    anchors.centerIn: parent
                    spacing: 6

                    Kbd {
                        keys: GlobalStates.actionToolTipKey
                    }

                    Text {
                        text: GlobalStates.actionToolTipName
                        font.pixelSize: Appearance.font.pixelSize.smallest
                        color: Appearance.m3colors.m3onSurfaceVariant
                    }
                }
            }

            // Preview Panel - drawer style, slides out from the side of the launcher
            // Defined BEFORE columnLayout so it renders underneath
            Item {
                id: previewPanelContainer
                visible: GlobalStates.launcherOpen && !GlobalStates.launcherMinimized
                
                property var previewItem: GlobalStates.previewItem
                property string currentItemId: previewItem?.id ?? ""
                property bool hasPreview: GlobalStates.previewPanelVisible
                
                property real launcherRight: columnLayout.x + columnLayout.width
                property real launcherLeft: columnLayout.x
                property real launcherY: columnLayout.y
                property real screenW: fullScreenBackground.width
                property real screenH: fullScreenBackground.height
                
                readonly property real panelWidth: Appearance.sizes.searchWidth * 0.75
                readonly property real overlapRight: 32
                readonly property real overlapLeft: 12
                
                readonly property bool showOnRight: {
                    const rightSpace = screenW - launcherRight;
                    const leftSpace = launcherLeft;
                    return rightSpace >= panelWidth - overlapRight || rightSpace >= leftSpace;
                }
                
                // Open position (overlapping with launcher)
                readonly property real openX: showOnRight 
                    ? launcherRight - overlapRight
                    : launcherLeft - panelWidth + overlapLeft
                
                // Closed position (hidden behind launcher)
                readonly property real closedX: showOnRight
                    ? launcherRight - panelWidth + overlapRight
                    : launcherLeft - overlapLeft
                
                // Drawer state: closed -> opening -> open -> closing -> closed
                property bool drawerOpen: false
                property var pendingItem: null
                
                onCurrentItemIdChanged: {
                    if (hasPreview) {
                        if (drawerOpen) {
                            // New item selected while open - slide out then back in
                            pendingItem = previewItem;
                            drawerOpen = false;
                        } else {
                            // First item - just open
                            drawerOpen = true;
                        }
                    }
                }
                
                onHasPreviewChanged: {
                    if (!hasPreview) {
                        drawerOpen = false;
                        pendingItem = null;
                    } else if (!drawerOpen) {
                        drawerOpen = true;
                    }
                }
                
                x: drawerOpen ? openX : closedX
                y: launcherY + Appearance.sizes.elevationMargin * 20
                
                opacity: drawerOpen ? 1 : 0
                
                implicitWidth: previewPanelLoader.item?.implicitWidth ?? 0 + Appearance.sizes.elevationMargin * 2
                implicitHeight: previewPanelLoader.item?.implicitHeight ?? 0 + Appearance.sizes.elevationMargin * 2
                
                // Drawer slide animation
                Behavior on x {
                    enabled: !columnLayout.isDragging
                    NumberAnimation {
                        id: slideAnim
                        duration: 180
                        easing.type: Easing.OutCubic
                        
                        onRunningChanged: {
                            if (!running && !previewPanelContainer.drawerOpen && previewPanelContainer.pendingItem) {
                                // Finished closing, now open with new item
                                previewPanelContainer.drawerOpen = true;
                                previewPanelContainer.pendingItem = null;
                            }
                        }
                    }
                }
                
                Behavior on opacity {
                    NumberAnimation {
                        duration: 150
                        easing.type: Easing.OutCubic
                    }
                }
                
                Loader {
                    id: previewPanelLoader
                    active: previewPanelContainer.drawerOpen
                    anchors.centerIn: parent
                    
                    sourceComponent: Item {
                        implicitWidth: previewPanelInner.implicitWidth
                        implicitHeight: previewPanelInner.implicitHeight
                        
                        StyledRectangularShadow {
                            target: previewPanelInner
                        }
                        
                        PreviewPanel {
                            id: previewPanelInner
                            item: previewPanelContainer.previewItem
                            
                            onDetachRequested: (globalX, globalY) => {
                                GlobalStates.detachCurrentPreview(globalX, globalY);
                            }
                        }
                    }
                }
            }

            Item {
                id: columnLayout
                visible: GlobalStates.launcherOpen && !GlobalStates.launcherMinimized

                // Track if we're dragging to avoid binding loop
                property bool isDragging: false

                // Calculate position from stored ratios (only when not dragging)
                x: isDragging ? x : Persistent.states.launcher.xRatio * fullScreenBackground.width - width / 2
                y: isDragging ? y : Persistent.states.launcher.yRatio * fullScreenBackground.height

                implicitWidth: searchWidget.implicitWidth
                implicitHeight: searchWidget.implicitHeight

                Keys.onPressed: event => {
                    if (event.key === Qt.Key_Escape) {
                        // Escape: minimize if user has used minimize before, otherwise soft close
                        if (Persistent.states.launcher.hasUsedMinimize) {
                            GlobalStates.launcherMinimized = true;
                        } else {
                            GlobalStates.softClose = true;
                            GlobalStates.launcherOpen = false;
                        }
                        event.accepted = true;
                    } else if (event.key === Qt.Key_M && (event.modifiers & Qt.ControlModifier)) {
                        GlobalStates.launcherMinimized = true;
                        event.accepted = true;
                    }
                }

                SearchWidget {
                    id: searchWidget
                    anchors.horizontalCenter: parent.horizontalCenter
                    Synchronizer on searchingText {
                        property alias source: root.searchingText
                    }

                    // Re-grab focus when user clicks on the launcher widget
                    onUserInteracted: root.regrabFocus()

                    // Drag offset from mouse to widget origin
                    property real dragOffsetX: 0
                    property real dragOffsetY: 0

                    onDragStarted: (mouseX, mouseY) => {
                        columnLayout.isDragging = true;
                        // Calculate offset from global mouse position to columnLayout origin
                        const widgetGlobalPos = columnLayout.mapToGlobal(0, 0);
                        dragOffsetX = mouseX - widgetGlobalPos.x;
                        dragOffsetY = mouseY - widgetGlobalPos.y;
                    }

                    onDragMoved: (mouseX, mouseY) => {
                        // Convert global mouse position to fullScreenBackground coordinates
                        const bgGlobalPos = fullScreenBackground.mapToGlobal(0, 0);
                        let newX = mouseX - bgGlobalPos.x - dragOffsetX;
                        let newY = mouseY - bgGlobalPos.y - dragOffsetY;

                        // Clamp to keep widget visible on screen
                        const screenW = fullScreenBackground.width;
                        const screenH = fullScreenBackground.height;
                        const topMargin = Appearance.sizes.elevationMargin * 20;
                        newX = Math.max(0, Math.min(newX, screenW - columnLayout.width));
                        newY = Math.max(-topMargin, Math.min(newY, screenH - columnLayout.height));

                        columnLayout.x = newX;
                        columnLayout.y = newY;
                    }

                    onDragEnded: {
                        // Save position as ratio for resolution independence
                        const screenW = fullScreenBackground.width;
                        const screenH = fullScreenBackground.height;
                        let xRatio = (columnLayout.x + columnLayout.width / 2) / screenW;
                        let yRatio = columnLayout.y / screenH;

                        // Clamp ratios to keep widget on screen (0.0-1.0)
                        xRatio = Math.max(0.0, Math.min(1.0, xRatio));
                        yRatio = Math.max(0.0, Math.min(1.0, yRatio));

                        Persistent.states.launcher.xRatio = xRatio;
                        Persistent.states.launcher.yRatio = yRatio;

                        columnLayout.isDragging = false;
                    }
                }
            }
        }
    }

    Variants {
        id: fabVariants
        model: Quickshell.screens
        PanelWindow {
            id: fabWindow
            required property var modelData
            readonly property var hyprlandMonitor: Hyprland.monitorFor(fabWindow.screen)
            property bool monitorIsFocused: false
            screen: modelData
            visible: GlobalStates.launcherMinimized && monitorIsFocused
            
            function updateMonitorFocus() {
                if (CompositorService.isHyprland) {
                    monitorIsFocused = Hyprland.focusedMonitor?.id == hyprlandMonitor?.id;
                } else if (CompositorService.isNiri) {
                    monitorIsFocused = modelData.name === NiriService.currentOutput;
                } else {
                    monitorIsFocused = true;
                }
            }
            
            Connections {
                target: GlobalStates
                enabled: !CompositorService.isHyprland
                function onLauncherMinimizedChanged() {
                    if (GlobalStates.launcherMinimized) {
                        fabWindow.updateMonitorFocus();
                    }
                }
            }
            
            Connections {
                target: NiriService
                enabled: CompositorService.isNiri && GlobalStates.launcherMinimized
                function onCurrentOutputChanged() {
                    fabWindow.updateMonitorFocus();
                }
            }
            
            Connections {
                target: CompositorService.isHyprland ? Hyprland : null
                enabled: CompositorService.isHyprland && GlobalStates.launcherMinimized
                function onFocusedMonitorChanged() {
                    fabWindow.updateMonitorFocus();
                }
            }
            
            Component.onCompleted: updateMonitorFocus()

            WlrLayershell.namespace: "quickshell:hamr-fab"
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
            exclusionMode: ExclusionMode.Ignore
            color: "transparent"

            mask: Region {
                item: fabContainer
            }

            anchors {
                top: true
                bottom: true
                left: true
                right: true
            }

            Item {
                id: fabContainer
                property bool isDragging: false

                x: isDragging ? x : Persistent.states.launcher.minXRatio * fabWindow.width - width / 2
                y: isDragging ? y : Persistent.states.launcher.minYRatio * fabWindow.height - Appearance.sizes.elevationMargin

                implicitWidth: fabContent.implicitWidth + Appearance.sizes.elevationMargin * 2
                implicitHeight: fabContent.implicitHeight + Appearance.sizes.elevationMargin * 2

                StyledRectangularShadow {
                    target: fabContent
                }

                // Glowing animated border that wraps around FAB
                Item {
                    id: glowBorderContainer
                    anchors.centerIn: parent
                    width: fabContent.width
                    height: fabContent.height
                    opacity: glowOpacity
                    visible: opacity > 0

                    property real dashOffset: 0
                    property real glowOpacity: 0

                    SequentialAnimation {
                        id: glowAnimation
                        running: false

                        PropertyAction {
                            target: glowBorderContainer
                            property: "dashOffset"
                            value: 0
                        }
                        PropertyAction {
                            target: glowBorderContainer
                            property: "glowOpacity"
                            value: 1
                        }

                        NumberAnimation {
                            target: glowBorderContainer
                            property: "dashOffset"
                            from: 0
                            to: 1
                            duration: 900
                            easing.type: Easing.Linear
                        }

                        NumberAnimation {
                            target: glowBorderContainer
                            property: "glowOpacity"
                            from: 1
                            to: 0
                            duration: 300
                            easing.type: Easing.OutCubic
                        }
                    }

                    Connections {
                        target: fabWindow
                        function onVisibleChanged() {
                            if (fabWindow.visible) {
                                glowAnimation.restart();
                            }
                        }
                    }

                    Shape {
                        id: glowBorderShape
                        anchors.fill: parent
                        layer.enabled: true
                        layer.effect: Glow {
                            radius: 4
                            samples: 9
                            spread: 0.3
                            color: Appearance.colors.colPrimary
                            transparentBorder: true
                        }

                        ShapePath {
                            id: glowPath
                            strokeWidth: 2
                            strokeColor: Appearance.colors.colPrimary
                            fillColor: "transparent"
                            capStyle: ShapePath.RoundCap
                            joinStyle: ShapePath.RoundJoin
                            strokeStyle: ShapePath.DashLine

                            readonly property real w: glowBorderShape.width
                            readonly property real h: glowBorderShape.height
                            readonly property real r: Math.min(w, h) / 2
                            readonly property real perimeter: 2 * (w - 2 * r) + 2 * (h - 2 * r) + 2 * Math.PI * r

                            dashOffset: -glowBorderContainer.dashOffset * perimeter
                            dashPattern: [perimeter * 0.15 / strokeWidth, perimeter * 0.85 / strokeWidth]

                            startX: r
                            startY: 0

                            PathLine {
                                x: glowPath.w - glowPath.r
                                y: 0
                            }
                            PathArc {
                                x: glowPath.w
                                y: glowPath.r
                                radiusX: glowPath.r
                                radiusY: glowPath.r
                            }
                            PathLine {
                                x: glowPath.w
                                y: glowPath.h - glowPath.r
                            }
                            PathArc {
                                x: glowPath.w - glowPath.r
                                y: glowPath.h
                                radiusX: glowPath.r
                                radiusY: glowPath.r
                            }
                            PathLine {
                                x: glowPath.r
                                y: glowPath.h
                            }
                            PathArc {
                                x: 0
                                y: glowPath.h - glowPath.r
                                radiusX: glowPath.r
                                radiusY: glowPath.r
                            }
                            PathLine {
                                x: 0
                                y: glowPath.r
                            }
                            PathArc {
                                x: glowPath.r
                                y: 0
                                radiusX: glowPath.r
                                radiusY: glowPath.r
                            }
                        }
                    }
                }

                Rectangle {
                    id: fabContent
                    anchors.centerIn: parent
                    implicitWidth: fabRow.implicitWidth + 16
                    implicitHeight: fabRow.implicitHeight + 12
                    radius: Appearance.rounding.full
                    color: Appearance.colors.colBackgroundSurfaceContainer
                    border.width: 1
                    border.color: Appearance.colors.colOutlineVariant

                    property bool rawHover: fabDragArea.containsMouse || fabCloseArea.containsMouse
                    property bool showCloseButton: rawHover || closeButtonHideDelay.running

                    onRawHoverChanged: {
                        if (rawHover) {
                            closeButtonHideDelay.stop();
                        } else {
                            closeButtonHideDelay.start();
                        }
                    }

                    Timer {
                        id: closeButtonHideDelay
                        interval: 300
                        repeat: false
                    }

                    RowLayout {
                        id: fabRow
                        anchors.centerIn: parent
                        spacing: 8

                        MaterialSymbol {
                            text: "gavel"
                            iconSize: Appearance.font.pixelSize.large
                            color: Appearance.colors.colPrimary
                        }

                        StyledText {
                            id: fabHamrText
                            text: "hamr"
                            font.pixelSize: Appearance.font.pixelSize.small
                            font.weight: Font.Medium
                            color: Appearance.m3colors.m3onSurface
                            Layout.preferredWidth: fabContent.showCloseButton ? 0 : implicitWidth
                            Layout.rightMargin: fabContent.showCloseButton ? -fabRow.spacing : 0
                            opacity: fabContent.showCloseButton ? 0 : 1
                            clip: true

                            Behavior on Layout.preferredWidth {
                                NumberAnimation {
                                    duration: 150
                                    easing.type: Easing.OutCubic
                                }
                            }
                            Behavior on Layout.rightMargin {
                                NumberAnimation {
                                    duration: 150
                                    easing.type: Easing.OutCubic
                                }
                            }
                            Behavior on opacity {
                                NumberAnimation {
                                    duration: 150
                                    easing.type: Easing.OutCubic
                                }
                            }
                        }

                        Item {
                            implicitWidth: 1
                            implicitHeight: parent.height

                            Rectangle {
                                anchors.centerIn: parent
                                width: 1
                                height: 16
                                color: Appearance.colors.colOutlineVariant
                            }
                        }

                        Item {
                            id: fabCloseButton
                            Layout.alignment: Qt.AlignVCenter
                            implicitWidth: fabContent.showCloseButton ? fabHamrText.implicitWidth : 0
                            implicitHeight: 24
                            clip: true

                            Behavior on implicitWidth {
                                NumberAnimation {
                                    duration: 150
                                    easing.type: Easing.OutCubic
                                }
                            }

                            Rectangle {
                                width: 24
                                height: 24
                                anchors.centerIn: parent
                                radius: Appearance.rounding.full
                                color: fabCloseArea.containsMouse ? Appearance.colors.colSurfaceContainerHighest : "transparent"
                                opacity: fabContent.showCloseButton ? 1 : 0

                                Behavior on opacity {
                                    NumberAnimation {
                                        duration: 150
                                        easing.type: Easing.OutCubic
                                    }
                                }

                                MaterialSymbol {
                                    anchors.centerIn: parent
                                    iconSize: Appearance.font.pixelSize.normal
                                    text: "close"
                                    color: fabCloseArea.containsMouse ? Appearance.colors.colOnSurface : Appearance.m3colors.m3outline
                                }
                            }

                            MouseArea {
                                id: fabCloseArea
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    Persistent.states.launcher.hasUsedMinimize = false;
                                    GlobalStates.launcherMinimized = false;
                                    GlobalStates.launcherOpen = false;
                                }
                            }

                            StyledToolTip {
                                parent: fabCloseArea
                                text: "Close"
                                visible: fabCloseArea.containsMouse && fabCloseButton.implicitWidth > 0
                            }
                        }

                        MaterialSymbol {
                            text: "drag_indicator"
                            iconSize: Appearance.font.pixelSize.normal
                            color: fabDragArea.containsMouse || fabDragArea.pressed ? Appearance.colors.colOnSurface : Appearance.m3colors.m3outline
                            Layout.leftMargin: fabContent.showCloseButton ? 0 : -fabRow.spacing

                            Behavior on Layout.leftMargin {
                                NumberAnimation {
                                    duration: 150
                                    easing.type: Easing.OutCubic
                                }
                            }
                        }
                    }

                    MouseArea {
                        id: fabClickArea
                        anchors.fill: parent
                        anchors.rightMargin: fabContent.showCloseButton ? 60 : 36
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            GlobalStates.launcherMinimized = false;
                            GlobalStates.launcherOpen = true;
                        }
                    }

                    MouseArea {
                        id: fabDragArea
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        width: 36
                        hoverEnabled: true
                        cursorShape: pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor

                        property real dragOffsetX: 0
                        property real dragOffsetY: 0

                        onPressed: mouse => {
                            fabContainer.isDragging = true;
                            const containerPos = mapToItem(fabContainer.parent, mouse.x, mouse.y);
                            dragOffsetX = containerPos.x - fabContainer.x;
                            dragOffsetY = containerPos.y - fabContainer.y;
                        }

                        onPositionChanged: mouse => {
                            if (pressed) {
                                const containerPos = mapToItem(fabContainer.parent, mouse.x, mouse.y);
                                let newX = containerPos.x - dragOffsetX;
                                let newY = containerPos.y - dragOffsetY;

                                const screenW = fabWindow.width;
                                const screenH = fabWindow.height;
                                const margin = Appearance.sizes.elevationMargin;
                                newX = Math.max(-margin, Math.min(newX, screenW - fabContainer.width + margin));
                                newY = Math.max(-margin, Math.min(newY, screenH - fabContainer.height + margin));

                                fabContainer.x = newX;
                                fabContainer.y = newY;
                            }
                        }

                        onReleased: {
                            const screenW = fabWindow.width;
                            const screenH = fabWindow.height;
                            const margin = Appearance.sizes.elevationMargin;
                            let xRatio = (fabContainer.x + fabContainer.width / 2) / screenW;
                            let yRatio = (fabContainer.y + margin) / screenH;

                            xRatio = Math.max(0.0, Math.min(1.0, xRatio));
                            yRatio = Math.max(0.0, Math.min(1.0, yRatio));

                            Persistent.states.launcher.minXRatio = xRatio;
                            Persistent.states.launcher.minYRatio = yRatio;

                            fabContainer.isDragging = false;
                        }
                    }

                    StyledToolTip {
                        visible: fabClickArea.containsMouse
                        text: "Expand"
                    }
                }
            }
        }
    }

    function toggleClipboard() {
        if (GlobalStates.launcherOpen && launcherScope.dontAutoCancelSearch) {
            // Toggle off - soft close (preserves state for restore window)
            GlobalStates.softClose = true;
            GlobalStates.launcherOpen = false;
            return;
        }
        const hints = Config.actionBarHints ?? [];
        const clipboardHint = hints.find(h => h.plugin === "clipboard");
        if (!clipboardHint) return;

        for (let i = 0; i < launcherVariants.instances.length; i++) {
            let panelWindow = launcherVariants.instances[i];
            if (panelWindow.monitorIsFocused) {
                launcherScope.dontAutoCancelSearch = true;
                panelWindow.setSearchingText(clipboardHint.prefix);
                GlobalStates.launcherOpen = true;
                return;
            }
        }
    }

    function toggleEmojis() {
        if (GlobalStates.launcherOpen && launcherScope.dontAutoCancelSearch) {
            // Toggle off - soft close (preserves state for restore window)
            GlobalStates.softClose = true;
            GlobalStates.launcherOpen = false;
            return;
        }
        
        const hints = Config.actionBarHints ?? [];
        const emojiHint = hints.find(h => h.plugin === "emoji");
        if (!emojiHint) return;

        for (let i = 0; i < launcherVariants.instances.length; i++) {
            let panelWindow = launcherVariants.instances[i];
            if (panelWindow.monitorIsFocused) {
                launcherScope.dontAutoCancelSearch = true;
                panelWindow.setSearchingText(emojiHint.prefix);
                GlobalStates.launcherOpen = true;
                return;
            }
        }
    }

    // Detached preview panels - persist independently of the launcher
    Variants {
        id: detachedPreviewVariants
        model: GlobalStates.detachedPreviews
        
        DetachedPreviewPanel {
            required property var modelData
            previewData: modelData
            initialX: modelData.x
            initialY: modelData.y
            visible: true
        }
    }
}
