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

Item {
    id: root
    readonly property string xdgConfigHome: Directories.config
    property string searchingText: LauncherSearch.query
    property bool showResults: searchingText != "" || LauncherSearch.results.length > 0
    property bool showCard: PluginRunner.pluginCard !== null
    property bool showForm: PluginRunner.pluginForm !== null
    readonly property bool showImageBrowser: GlobalStates.imageBrowserOpen
    readonly property bool showGridBrowser: GlobalStates.gridBrowserOpen
    
    implicitWidth: searchWidgetContent.implicitWidth + Appearance.sizes.elevationMargin * 2
    implicitHeight: searchWidgetContent.implicitHeight + searchWidgetContent.anchors.topMargin + Appearance.sizes.elevationMargin * 2

    property alias contentItem: searchWidgetContent
    readonly property bool searchInputHasFocus: searchBar.searchInput.activeFocus
    readonly property Item searchInput: searchBar.searchInput

    signal dragStarted(real mouseX, real mouseY)
    signal dragMoved(real mouseX, real mouseY)
    signal dragEnded
    signal userInteracted  // Emitted when user clicks/interacts - used to re-grab focus

    // Helper functions to access grid components (loaders defined later in tree)
    function getGridBrowserGrid() {
        return gridBrowserLoader.item?.gridComponent ?? null;
    }
    
    function getImageBrowserGrid() {
        return imageBrowserLoader.item?.gridComponent ?? null;
    }

    function focusFirstItem() {
        if (appResults.count > 0) {
            appResults.currentIndex = 0;
        }
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
        // Also exit exclusive mode when cancelling search
        if (LauncherSearch.isInExclusiveMode()) {
            LauncherSearch.exclusiveMode = "";
        }
        // Close active plugin if any
        if (PluginRunner.isActive()) {
            PluginRunner.closePlugin();
        }
        searchBar.animateWidth = true;
    }

    function setSearchingText(text) {
        searchBar.searchInput.text = text;
        LauncherSearch.query = text;
    }

    function navigateBack() {
        // Navigate back one level: gridBrowser > imageBrowser > plugin > exclusive mode > clear search
        if (GlobalStates.gridBrowserOpen) {
            const isInitialView = GlobalStates.gridBrowserConfig?.isInitialView ?? false;
            if (!isInitialView) {
                GlobalStates.cancelGridBrowser();
            } else {
                GlobalStates.closeGridBrowser();
                LauncherSearch.exitPlugin();
            }
        } else if (GlobalStates.imageBrowserOpen) {
            const isInitialView = GlobalStates.imageBrowserConfig?.isInitialView ?? false;
            if (!isInitialView) {
                GlobalStates.cancelImageBrowser();
            } else {
                GlobalStates.closeImageBrowser();
                LauncherSearch.exitPlugin();
            }
        } else if (PluginRunner.isActive()) {
            if (PluginRunner.navigationDepth > 0) {
                PluginRunner.goBack();
            } else {
                LauncherSearch.exitPlugin();
            }
        } else if (LauncherSearch.isInExclusiveMode()) {
            LauncherSearch.exitExclusiveMode();
        } else if (root.searchingText !== "") {
            root.cancelSearch();
        }
    }

    Keys.onPressed: event => {
         if (event.key === Qt.Key_Escape)
             return;

         if (event.text && event.text.length === 1 && event.key !== Qt.Key_Enter && event.key !== Qt.Key_Return && event.key !== Qt.Key_Delete && event.text.charCodeAt(0) >= 0x20) {
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
     Rectangle {
         id: searchWidgetContent
        anchors {
            top: parent.top
            horizontalCenter: parent.horizontalCenter
            topMargin: Appearance.sizes.elevationMargin * 20
        }
        clip: true
        implicitWidth: (root.showImageBrowser || root.showGridBrowser)
            ? Appearance.sizes.imageBrowserGridWidth + 12  // grid width + margins
            : columnLayout.implicitWidth
        implicitHeight: columnLayout.implicitHeight
        radius: Appearance.rounding.normal
        color: Appearance.colors.colBackgroundSurfaceContainer

        Behavior on implicitHeight {
            id: searchHeightBehavior
            enabled: GlobalStates.launcherOpen && (root.showResults || root.showCard || root.showImageBrowser || root.showGridBrowser)
            animation: Appearance.animation.elementMove.numberAnimation.createObject(this)
        }

        Behavior on implicitWidth {
            enabled: GlobalStates.launcherOpen
            animation: Appearance.animation.elementMove.numberAnimation.createObject(this)
        }
        
        // Invisible overlay to detect clicks and re-grab focus if lost
        MouseArea {
            anchors.fill: parent
            z: 1000
            propagateComposedEvents: true
            onPressed: mouse => {
                root.userInteracted();
                mouse.accepted = false;
            }
        }

        ColumnLayout {
            id: columnLayout
            anchors {
                top: parent.top
                left: parent.left
                right: parent.right
            }
            spacing: 0

             clip: true

             Rectangle {
                id: searchBarContainer
                implicitWidth: searchBar.fixedWidth + 12
                implicitHeight: searchBar.implicitHeight + 12
                Layout.margins: 6
                Layout.fillWidth: true
                radius: Appearance.rounding.small
                color: Appearance.colors.colSurfaceContainerHigh
                border.width: 1
                border.color: Appearance.colors.colSurfaceContainerHighest

                SearchBar {
                    id: searchBar
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: 6
                    anchors.rightMargin: 6
                    anchors.verticalCenter: parent.verticalCenter
                    expandSearchInput: root.showImageBrowser || root.showGridBrowser
                     Synchronizer on searchingText {
                         property alias source: root.searchingText
                     }

                     onDragStarted: (mouseX, mouseY) => root.dragStarted(mouseX, mouseY)
                    onDragMoved: (mouseX, mouseY) => root.dragMoved(mouseX, mouseY)
                    onDragEnded: root.dragEnded()
                    
                    onNavigateBack: root.navigateBack()
                    onExitPluginImmediate: {
                        if (GlobalStates.gridBrowserOpen) {
                            GlobalStates.closeGridBrowser();
                            LauncherSearch.exitPlugin();
                        } else if (GlobalStates.imageBrowserOpen) {
                            GlobalStates.closeImageBrowser();
                            LauncherSearch.exitPlugin();
                        } else if (PluginRunner.isActive()) {
                            LauncherSearch.exitPlugin();
                        } else if (LauncherSearch.isInExclusiveMode()) {
                            LauncherSearch.exitExclusiveMode();
                        }
                    }

                    onNavigateDown: {
                        if (root.showImageBrowser) {
                            const grid = root.getImageBrowserGrid();
                            if (grid) grid.moveSelection(grid.columns);
                            return;
                        }
                        if (root.showGridBrowser) {
                            const grid = root.getGridBrowserGrid();
                            if (grid) grid.moveSelection(grid.columns);
                            return;
                        }
                        if (appResults.count === 0) return;
                        if (appResults.currentIndex < appResults.count - 1) {
                            appResults.currentIndex++;
                        } else {
                            appResults.currentIndex = 0;
                        }
                    }
                    onNavigateUp: {
                        if (root.showImageBrowser) {
                            const grid = root.getImageBrowserGrid();
                            if (grid) grid.moveSelection(-grid.columns);
                            return;
                        }
                        if (root.showGridBrowser) {
                            const grid = root.getGridBrowserGrid();
                            if (grid) grid.moveSelection(-grid.columns);
                            return;
                        }
                        if (appResults.count === 0) return;
                        if (appResults.currentIndex > 0) {
                            appResults.currentIndex--;
                        } else {
                            appResults.currentIndex = appResults.count - 1;
                        }
                    }
                    onNavigateLeft: {
                        if (root.showImageBrowser) {
                            const grid = root.getImageBrowserGrid();
                            if (grid) grid.moveSelection(-1);
                        }
                        if (root.showGridBrowser) {
                            const grid = root.getGridBrowserGrid();
                            if (grid) grid.moveSelection(-1);
                        }
                    }
                    onNavigateRight: {
                        if (root.showImageBrowser) {
                            const grid = root.getImageBrowserGrid();
                            if (grid) grid.moveSelection(1);
                        } else if (root.showGridBrowser) {
                            const grid = root.getGridBrowserGrid();
                            if (grid) grid.moveSelection(1);
                        } else {
                            // Original Ctrl+L behavior: select current item
                            if (appResults.count > 0 && appResults.currentIndex >= 0) {
                                let currentItem = appResults.itemAtIndex(appResults.currentIndex);
                                if (currentItem?.clicked) {
                                    currentItem.clicked();
                                }
                            }
                        }
                    }
                    onSelectCurrent: {
                        // If imageBrowser is open, activate current image
                        if (root.showImageBrowser) {
                            const grid = root.getImageBrowserGrid();
                            if (grid) grid.activateCurrent();
                            return;
                        }
                        // If gridBrowser is open, activate current item
                        if (root.showGridBrowser) {
                            const grid = root.getGridBrowserGrid();
                            if (grid) grid.activateCurrent();
                            return;
                        }

                        // If there's a pending confirmation dialog, Enter confirms it
                        if (actionBar.pendingConfirmAction !== null) {
                            const actionId = actionBar.pendingConfirmAction?.id ?? "";
                            actionBar.pendingConfirmAction = null;
                            if (actionId) {
                                // Confirmed actions are typically destructive (wipe, clear)
                                // They modify the view but don't navigate, so skip depth change
                                PluginRunner.executePluginAction(actionId, true);
                            }
                            return;
                        }
                        
                        // If plugin is active in submit mode with user input:
                        // - Default: Enter submits the query
                        // - Exception: if user navigated to a non-first result, Enter selects it
                        if (PluginRunner.isActive() && PluginRunner.inputMode === "submit" && root.searchingText.trim() !== "") {
                            if (appResults.count > 0 && appResults.currentIndex > 0) {
                                // Fall through to select current item
                            } else {
                                LauncherSearch.submitPluginQuery();
                                return;
                            }
                        }

                        if (appResults.count > 0 && appResults.currentIndex >= 0) {
                            let currentItem = appResults.itemAtIndex(appResults.currentIndex);
                            if (currentItem) {
                                // Check if an action is focused via Tab
                                if (currentItem.focusedActionIndex >= 0) {
                                    currentItem.executeCurrentAction();
                                } else if (currentItem.clicked) {
                                    currentItem.clicked();
                                }
                            }
                        }
                    }

                    Connections {
                        target: searchBar.searchInput
                        function onCycleActionNext() {
                            if (appResults.count > 0 && appResults.currentIndex >= 0) {
                                let currentItem = appResults.itemAtIndex(appResults.currentIndex);
                                if (currentItem) {
                                    currentItem.cycleActionNext();
                                }
                            }
                        }
                        function onCycleActionPrev() {
                            if (appResults.count > 0 && appResults.currentIndex >= 0) {
                                let currentItem = appResults.itemAtIndex(appResults.currentIndex);
                                if (currentItem) {
                                    currentItem.cycleActionPrev();
                                }
                            }
                        }
                        function onExecuteActionByIndex(index) {
                            if (appResults.count > 0 && appResults.currentIndex >= 0) {
                                let currentItem = appResults.itemAtIndex(appResults.currentIndex);
                                if (currentItem) {
                                    const actions = currentItem.entry.actions ?? [];
                                    // Max 4 actions supported
                                    if (index < actions.length && index < 4) {
                                        // Capture selection before action executes
                                        appResults.captureSelection();
                                        LauncherSearch.skipNextAutoFocus = true;
                                        actions[index].execute();
                                    }
                                }
                            }
                        }
                        function onExecutePluginAction(index) {
                            // Execute plugin action by index (Ctrl+1 through Ctrl+6)
                            if (!PluginRunner.isActive()) return;
                            const actions = PluginRunner.pluginActions;
                            if (index >= 0 && index < actions.length) {
                                const action = actions[index];
                                if (action.confirm) {
                                    // Show confirmation in action bar
                                    actionBar.pendingConfirmAction = action;
                                } else {
                                    PluginRunner.executePluginAction(action.id);
                                }
                            }
                        }
                    }
                }
            }

            ActionBar {
                id: actionBar
                visible: !Persistent.states.launcher.actionBarHidden
                Layout.fillWidth: true
                Layout.leftMargin: 12
                Layout.rightMargin: 12
                Layout.bottomMargin: 8
                Layout.preferredHeight: 34
                
                showSeparator: root.showResults || root.showCard || root.showForm || PluginRunner.pluginBusy || root.showImageBrowser || root.showGridBrowser
                
                readonly property bool inSearchMode: {
                    const q = root.searchingText;
                    const prefixes = LauncherSearch.getConfiguredPrefixes();
                    return prefixes.some(p => q.startsWith(p)) || LauncherSearch.isInExclusiveMode();
                }
                
                mode: (root.showImageBrowser || root.showGridBrowser) ? "plugin" : (PluginRunner.isActive() ? "plugin" : (inSearchMode ? "search" : "hints"))
                
                actions: {
                    if (root.showImageBrowser) {
                        const config = GlobalStates.imageBrowserConfig;
                        const customActions = config?.actions ?? [];
                        return customActions.map((action, idx) => ({
                            id: action.id,
                            icon: action.icon ?? "play_arrow",
                            name: action.name ?? action.id,
                            shortcut: `Ctrl+${idx + 1}`
                        }));
                    } else if (root.showGridBrowser) {
                        const config = GlobalStates.gridBrowserConfig;
                        const customActions = config?.actions ?? [];
                        return customActions.map((action, idx) => ({
                            id: action.id,
                            icon: action.icon ?? "play_arrow",
                            name: action.name ?? action.id,
                            shortcut: `Ctrl+${idx + 1}`
                        }));
                    } else if (PluginRunner.isActive()) {
                        return PluginRunner.pluginActions;
                    } else if (inSearchMode) {
                        return [];
                    } else {
                        const hints = Config.actionBarHints ?? [];
                        return hints.map(hint => ({
                            key: hint.prefix,
                            icon: hint.icon,
                            label: hint.label
                        }));
                    }
                }
                
                navigationDepth: PluginRunner.navigationDepth
                
                showBrowserKeys: root.showImageBrowser || root.showGridBrowser
                
                onActionClicked: (actionId, wasConfirmed) => {
                    if (root.showImageBrowser) {
                        imageBrowserLoader.executeActionOnCurrent(actionId);
                    } else if (root.showGridBrowser) {
                        gridBrowserLoader.executeActionOnCurrent(actionId);
                    } else if (PluginRunner.isActive()) {
                        PluginRunner.executePluginAction(actionId, wasConfirmed);
                    } else if (!actionBar.inSearchMode) {
                        searchBar.searchInput.text = actionId;
                        LauncherSearch.query = actionId;
                        root.focusSearchInput();
                    }
                }
                
                onBackClicked: {
                    if (root.showImageBrowser) {
                        if (GlobalStates.imageBrowserConfig?.workflowId) {
                            GlobalStates.cancelImageBrowser();
                        } else {
                            GlobalStates.closeImageBrowser();
                        }
                        return;
                    }
                    if (root.showGridBrowser) {
                        if (GlobalStates.gridBrowserConfig?.workflowId) {
                            GlobalStates.cancelGridBrowser();
                        } else {
                            GlobalStates.closeGridBrowser();
                        }
                        return;
                    }
                    if (PluginRunner.isActive()) {
                        if (root.showForm) {
                            PluginRunner.cancelForm();
                            root.focusSearchInput();
                            return;
                        }
                        PluginRunner.goBack();
                    } else {
                        root.cancelSearch();
                    }
                }
                
                onHomeClicked: {
                    if (root.showImageBrowser) {
                        GlobalStates.closeImageBrowser();
                    }
                    if (root.showGridBrowser) {
                        GlobalStates.closeGridBrowser();
                    }
                    LauncherSearch.exitPlugin();
                }
                
                onKeybindingHelpRequested: {
                    keybindingMapPopup.visible = !keybindingMapPopup.visible;
                }
            }

             RowLayout {
                visible: PluginRunner.pluginBusy && !root.showCard && PluginRunner.inputMode === "submit"
                Layout.fillWidth: true
                Layout.margins: 20
                spacing: 12

                StyledIndeterminateProgressBar {
                    Layout.fillWidth: true
                }

                StyledText {
                    text: "Processing..."
                    font.pixelSize: Appearance.font.pixelSize.small
                    color: Appearance.colors.colSubtext
                }
            }

             Loader {
                id: pluginCardLoader
                visible: root.showCard
                Layout.fillWidth: true
                Layout.leftMargin: 6
                Layout.rightMargin: 6
                Layout.bottomMargin: 6
                Layout.topMargin: 6

                property var currentCard: PluginRunner.pluginCard

                sourceComponent: {
                    const c = pluginCardLoader.currentCard;
                    if (c === null || c === undefined)
                        return null;
                    if ((c.kind ?? "") === "blocks" || c.blocks !== undefined)
                        return richCardComponent;
                    return simpleCardComponent;
                }
            }

            Component {
                id: richCardComponent
                PluginRichCard {
                    card: pluginCardLoader.currentCard
                    busy: PluginRunner.pluginBusy
                }
            }

            Component {
                id: simpleCardComponent
                PluginCard {
                    card: pluginCardLoader.currentCard
                    busy: PluginRunner.pluginBusy
                }
            }

            Loader {
                id: pluginFormLoader
                active: root.showForm
                visible: active
                Layout.fillWidth: true
                Layout.leftMargin: 6
                Layout.rightMargin: 6
                Layout.bottomMargin: 6
                
                sourceComponent: PluginForm {
                    form: PluginRunner.pluginForm

                    onSubmitted: formData => {
                        PluginRunner.submitForm(formData);
                        root.focusSearchInput();
                    }

                    onCancelled: {
                        PluginRunner.cancelForm();
                        root.focusSearchInput();
                    }
                }
            }

             Rectangle {
                id: resultsContainer
                visible: root.showResults && !root.showCard && !root.showForm && !(PluginRunner.pluginBusy && PluginRunner.inputMode === "submit") && !root.showImageBrowser && !root.showGridBrowser
                Layout.fillWidth: true
                Layout.leftMargin: 6
                Layout.rightMargin: 6
                Layout.bottomMargin: 6
                Layout.topMargin: 4
                implicitHeight: appResults.implicitHeight
                radius: Appearance.rounding.small
                color: Appearance.colors.colSurfaceContainerLow
                border.width: 1
                border.color: Appearance.colors.colOutlineVariant

                 ListView {
                     id: appResults
                    anchors.fill: parent
                    implicitHeight: Math.min(Appearance.sizes.maxResultsHeight, appResults.contentHeight + topMargin + bottomMargin)
                    clip: true
                    cacheBuffer: 500  // Keep more delegates cached to reduce flicker
                    reuseItems: true  // Enable delegate reuse
                    currentIndex: -1  // Start with no selection to prevent out-of-range errors
                    topMargin: 6
                    bottomMargin: 6
                    spacing: 2
                    KeyNavigation.up: searchBar
                    highlightMoveDuration: 100

                    onFocusChanged: {
                        if (focus && appResults.count > 1)
                            appResults.currentIndex = 1;
                    }

                    Connections {
                        target: root
                        function onSearchingTextChanged() {
                            if (appResults.count > 0)
                                appResults.currentIndex = 0;
                        }
                    }

                    property string selectedItemKey: ""
                     property int selectedActionIndex: -1
                     
                     property string pendingItemKey: ""
                     property int pendingActionIndex: -1
                     property int pendingCurrentIndex: -1

                     onCurrentIndexChanged: {
                        if (currentIndex >= 0 && currentIndex < LauncherSearch.results.length) {
                            selectedItemKey = LauncherSearch.results[currentIndex]?.key ?? "";
                            // Update preview panel with selected item
                            GlobalStates.setPreviewItem(LauncherSearch.results[currentIndex]);
                        } else {
                            selectedItemKey = "";
                            GlobalStates.clearPreviewItem();
                        }
                    }

                     function updateActionIndex(index) {
                        selectedActionIndex = index;
                    }
                    
                     function captureSelection() {
                        pendingItemKey = selectedItemKey;
                        pendingActionIndex = selectedActionIndex;
                        pendingCurrentIndex = currentIndex;
                    }
                    
                     function clearPendingSelection() {
                        pendingItemKey = "";
                        pendingActionIndex = -1;
                        pendingCurrentIndex = -1;
                    }

                    model: ScriptModel {
                        id: model
                        objectProp: "key"
                        values: LauncherSearch.results
                        onValuesChanged: {
                            const hasPendingRestore = appResults.pendingItemKey !== "" || appResults.pendingCurrentIndex >= 0;
                            const isPoll = PluginRunner.isPollUpdate;
                            
                            if (isPoll) {
                                 PluginRunner.isPollUpdate = false;
                             }
                             
                             const shouldTryRestore = LauncherSearch.skipNextAutoFocus || hasPendingRestore || isPoll;
                            
                            if (shouldTryRestore && appResults.count > 0) {
                                // Clear the one-shot flag if it was set
                                if (LauncherSearch.skipNextAutoFocus) {
                                    LauncherSearch.skipNextAutoFocus = false;
                                }
                                
                                 const savedKey = hasPendingRestore ? appResults.pendingItemKey : appResults.selectedItemKey;
                                 const savedActionIndex = hasPendingRestore ? appResults.pendingActionIndex : appResults.selectedActionIndex;
                                 const savedIndex = appResults.pendingCurrentIndex;
                                 
                                 if (savedKey) {
                                     const newIndex = LauncherSearch.results.findIndex(r => r.key === savedKey);
                                     if (newIndex >= 0) {
                                         appResults.currentIndex = newIndex;
                                         if (hasPendingRestore) {
                                             appResults.clearPendingSelection();
                                         }
                                         if (savedActionIndex >= 0) {
                                            Qt.callLater(() => {
                                                const currentItem = appResults.itemAtIndex(appResults.currentIndex);
                                                if (currentItem) {
                                                    currentItem.focusedActionIndex = savedActionIndex;
                                                }
                                            });
                                        }
                                        return;
                                    }
                                }
                                
                                 if (savedIndex >= 0) {
                                    const clampedIndex = Math.min(savedIndex, appResults.count - 1);
                                    appResults.currentIndex = Math.max(0, clampedIndex);
                                    if (hasPendingRestore) {
                                        appResults.clearPendingSelection();
                                    }
                                    return;
                                }
                                
                                 if (appResults.currentIndex >= 0 && appResults.currentIndex < appResults.count) {
                                    return;
                                }
                            }
                            
                            if (appResults.count === 0 && hasPendingRestore) {
                                return;
                            }

                            appResults.clearPendingSelection();
                             appResults.currentIndex = -1;
                             appResults.selectedActionIndex = -1;
                             root.focusFirstItem();
                        }
                    }

                    delegate: SearchItem {
                         anchors.left: parent?.left
                        anchors.right: parent?.right
                        entry: modelData
                        query: StringUtils.cleanOnePrefix(root.searchingText, [Config.options.search.prefix.action, Config.options.search.prefix.app, Config.options.search.prefix.clipboard, Config.options.search.prefix.emojis, Config.options.search.prefix.math, Config.options.search.prefix.shellCommand, Config.options.search.prefix.webSearch])
                    }
                }

                Rectangle {
                    id: depthGradientOverlay
                    visible: !PluginRunner.isActive()
                    anchors.fill: parent
                    radius: resultsContainer.radius
                    gradient: Gradient {
                        GradientStop { position: 0.0; color: "transparent" }
                        GradientStop { position: 0.60; color: Qt.rgba(0, 0, 0, 0.03) }
                        GradientStop { position: 0.80; color: Qt.rgba(0, 0, 0, 0.08) }
                        GradientStop { position: 0.95; color: Qt.rgba(0, 0, 0, 0.15) }
                        GradientStop { position: 1.0; color: Qt.rgba(0, 0, 0, 0.20) }
                    }
                }
            }

            Loader {
                id: imageBrowserLoader
                active: root.showImageBrowser
                visible: active
                Layout.preferredWidth: Appearance.sizes.imageBrowserGridWidth
                Layout.leftMargin: 6
                Layout.rightMargin: 6
                Layout.bottomMargin: 6
                Layout.topMargin: 4

                function executeActionOnCurrent(actionId) {
                    if (item && item.gridComponent) {
                        item.gridComponent.executeActionOnCurrent(actionId);
                    }
                }

                sourceComponent: Rectangle {
                    id: imageBrowserContainer
                    property alias gridComponent: imageBrowserGrid
                    
                    radius: Appearance.rounding.small
                    color: Appearance.colors.colSurfaceContainerLow
                    border.width: 1
                    border.color: Appearance.colors.colOutlineVariant
                    implicitWidth: imageBrowserGrid.implicitWidth
                    implicitHeight: imageBrowserGrid.implicitHeight

                    ImageBrowserGrid {
                        id: imageBrowserGrid
                        anchors.fill: parent
                        anchors.margins: 6
                        focus: true

                        onImageSelected: (filePath, actionId) => {
                            GlobalStates.imageBrowserSelection(filePath, actionId);
                        }

                        onCancelled: {
                            if (GlobalStates.imageBrowserConfig?.workflowId) {
                                GlobalStates.cancelImageBrowser();
                            } else {
                                GlobalStates.closeImageBrowser();
                            }
                        }
                    }
                }
            }

            Loader {
                id: gridBrowserLoader
                active: root.showGridBrowser
                visible: active
                Layout.preferredWidth: Appearance.sizes.imageBrowserGridWidth
                Layout.leftMargin: 6
                Layout.rightMargin: 6
                Layout.bottomMargin: 6
                Layout.topMargin: 4

                function executeActionOnCurrent(actionId) {
                    if (item && item.gridComponent) {
                        item.gridComponent.executeActionOnCurrent(actionId);
                    }
                }

                sourceComponent: Rectangle {
                    id: gridBrowserContainer
                    property alias gridComponent: genericGrid
                    
                    radius: Appearance.rounding.small
                    color: Appearance.colors.colSurfaceContainerLow
                    border.width: 1
                    border.color: Appearance.colors.colOutlineVariant
                    implicitWidth: genericGrid.implicitWidth
                    implicitHeight: genericGrid.implicitHeight

                    GenericGrid {
                        id: genericGrid
                        anchors.fill: parent
                        anchors.margins: 6
                        focus: true

                        onItemSelected: (itemId, actionId) => {
                            GlobalStates.gridBrowserSelection(itemId, actionId);
                        }

                        onCancelled: {
                            if (GlobalStates.gridBrowserConfig?.workflowId) {
                                GlobalStates.cancelGridBrowser();
                            } else {
                                GlobalStates.closeGridBrowser();
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Keybinding map popup - centered on the widget
    KeybindingMap {
        id: keybindingMapPopup
        visible: false
        showBrowserKeys: root.showImageBrowser || root.showGridBrowser
        
        anchors.centerIn: parent
        z: 100
        
        // Close on any key press or mouse click outside
        Connections {
            target: GlobalStates
            function onLauncherOpenChanged() {
                if (!GlobalStates.launcherOpen) {
                    keybindingMapPopup.visible = false;
                }
            }
        }
    }
    
    // Click outside to close keybinding map
    MouseArea {
        anchors.fill: parent
        visible: keybindingMapPopup.visible
        z: 99
        onClicked: keybindingMapPopup.visible = false
    }
}
