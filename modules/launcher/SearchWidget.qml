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
    property bool showCard: PluginRunner.pluginCard !== null
    property bool showForm: PluginRunner.pluginForm !== null
    // Include the full visual height (content + offset)
    implicitWidth: searchWidgetContent.implicitWidth + Appearance.sizes.elevationMargin * 2
    implicitHeight: searchWidgetContent.implicitHeight + searchWidgetContent.anchors.topMargin + Appearance.sizes.elevationMargin * 2

    // Expose the content rectangle for input masking
    property alias contentItem: searchWidgetContent

    // Forward drag signals from SearchBar
    signal dragStarted(real mouseX, real mouseY)
    signal dragMoved(real mouseX, real mouseY)
    signal dragEnded

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
        radius: Appearance.rounding.normal
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

            clip: true
            // layer.enabled: true
            // layer.effect: OpacityMask {
            //     maskSource: Rectangle {
            //         width: searchWidgetContent.width
            //         height: searchWidgetContent.height
            //         radius: searchWidgetContent.radius
            //     }
            // }

            // Elevated search bar container
            Rectangle {
                id: searchBarContainer
                implicitWidth: searchBar.implicitWidth + 12
                implicitHeight: searchBar.implicitHeight + 12
                Layout.margins: 6
                radius: Appearance.rounding.small
                color: Appearance.colors.colSurfaceContainerHigh
                border.width: 1
                border.color: Appearance.colors.colSurfaceContainerHighest

                SearchBar {
                    id: searchBar
                    anchors.centerIn: parent
                    Synchronizer on searchingText {
                        property alias source: root.searchingText
                    }

                    // Forward drag signals
                    onDragStarted: (mouseX, mouseY) => root.dragStarted(mouseX, mouseY)
                    onDragMoved: (mouseX, mouseY) => root.dragMoved(mouseX, mouseY)
                    onDragEnded: root.dragEnded()

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
                                        LauncherSearch.skipNextAutoFocus = true;
                                        actions[index].execute();
                                        // Keep selection - don't clear currentIndex
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
                                    pluginActionBar.pendingConfirmAction = action;
                                } else {
                                    PluginRunner.executePluginAction(action.id);
                                }
                            }
                        }
                    }
                }
            }

            // Hint bar - shows prefix shortcuts on main view, back button in prefix modes
            // Hidden when plugin is active
            Item {
                id: hintBar
                visible: !PluginRunner.isActive()
                Layout.fillWidth: true
                Layout.leftMargin: 12
                Layout.rightMargin: 12
                Layout.bottomMargin: 8
                Layout.preferredHeight: 34
                
                // Check if we're in any prefix mode (by checking if query starts with a known prefix)
                readonly property bool inPrefixMode: {
                    const q = root.searchingText;
                    return q.startsWith(Config.options.search.prefix.file) ||
                           q.startsWith(Config.options.search.prefix.clipboard) ||
                           q.startsWith(Config.options.search.prefix.action) ||
                           q.startsWith(Config.options.search.prefix.shellHistory) ||
                           q.startsWith(Config.options.search.prefix.math) ||
                           q.startsWith(Config.options.search.prefix.emojis) ||
                           LauncherSearch.isInExclusiveMode();
                }
                
                // Check if in clipboard mode specifically (for wipe action)
                readonly property bool inClipboardMode: root.searchingText.startsWith(Config.options.search.prefix.clipboard)
                
                // Main view: prefix buttons
                RowLayout {
                    anchors.fill: parent
                    spacing: 8
                    visible: !hintBar.inPrefixMode

                    // Prefix buttons (clickable to enter prefix mode)
                    Repeater {
                        model: [
                            { key: Config.options.search.prefix.file, label: "files" },
                            { key: Config.options.search.prefix.clipboard, label: "clipboard" },
                            { key: Config.options.search.prefix.action, label: "plugins" },
                            { key: Config.options.search.prefix.shellHistory, label: "shell" },
                            { key: Config.options.search.prefix.math, label: "math" },
                            { key: Config.options.search.prefix.emojis, label: "emoji" },
                        ]

                        RippleButton {
                            id: prefixBtn
                            required property var modelData
                            required property int index
                            
                            Layout.fillHeight: true
                            implicitWidth: prefixContent.implicitWidth + 16
                            
                            buttonRadius: 4
                            colBackground: "transparent"
                            colBackgroundHover: Appearance.colors.colSurfaceContainerHighest
                            colRipple: Appearance.colors.colSurfaceContainerHighest
                            
                            onClicked: {
                                searchBar.searchInput.text = modelData.key;
                                LauncherSearch.query = modelData.key;
                                root.focusSearchInput();
                            }
                            
                            // Border outline
                            Rectangle {
                                anchors.fill: parent
                                radius: 4
                                color: "transparent"
                                border.width: 1
                                border.color: Appearance.colors.colOutlineVariant
                            }
                            
                            contentItem: RowLayout {
                                id: prefixContent
                                spacing: 8
                                
                                Kbd {
                                    Layout.alignment: Qt.AlignVCenter
                                    keys: prefixBtn.modelData.key
                                }
                                
                                Text {
                                    Layout.alignment: Qt.AlignVCenter
                                    text: prefixBtn.modelData.label
                                    font.pixelSize: Appearance.font.pixelSize.smaller
                                    color: Appearance.m3colors.m3onSurfaceVariant
                                }
                            }
                        }
                    }

                    // Spacer
                    Item {
                        Layout.fillWidth: true
                    }
                }
                
                // Prefix mode: back button (and wipe for clipboard)
                RowLayout {
                    anchors.fill: parent
                    spacing: 8
                    visible: hintBar.inPrefixMode
                    
                    // Back button
                    RippleButton {
                        id: backBtn
                        Layout.fillHeight: true
                        implicitWidth: backContent.implicitWidth + 16
                        
                        buttonRadius: 4
                        colBackground: "transparent"
                        colBackgroundHover: Appearance.colors.colSurfaceContainerHighest
                        colRipple: Appearance.colors.colSurfaceContainerHighest
                        
                        onClicked: {
                            root.cancelSearch();
                        }
                        
                        // Border outline
                        Rectangle {
                            anchors.fill: parent
                            radius: 4
                            color: "transparent"
                            border.width: 1
                            border.color: Appearance.colors.colOutlineVariant
                        }
                        
                        contentItem: RowLayout {
                            id: backContent
                            spacing: 8
                            
                            MaterialSymbol {
                                Layout.alignment: Qt.AlignVCenter
                                text: "arrow_back"
                                iconSize: 18
                                color: Appearance.m3colors.m3onSurfaceVariant
                            }
                            
                            Text {
                                Layout.alignment: Qt.AlignVCenter
                                text: "Back"
                                font.pixelSize: Appearance.font.pixelSize.smaller
                                color: Appearance.m3colors.m3onSurfaceVariant
                            }
                            
                            Kbd {
                                Layout.alignment: Qt.AlignVCenter
                                keys: "Esc"
                            }
                        }
                    }
                    
                    // Wipe button (clipboard mode only)
                    RippleButton {
                        id: wipeBtn
                        visible: hintBar.inClipboardMode
                        Layout.fillHeight: true
                        implicitWidth: wipeContent.implicitWidth + 16
                        
                        buttonRadius: 4
                        colBackground: "transparent"
                        colBackgroundHover: Appearance.colors.colSurfaceContainerHighest
                        colRipple: Appearance.colors.colSurfaceContainerHighest
                        
                        onClicked: {
                            Quickshell.exec(["cliphist", "wipe"]);
                            // Refresh the clipboard list
                            LauncherSearch.query = Config.options.search.prefix.clipboard;
                        }
                        
                        // Border outline
                        Rectangle {
                            anchors.fill: parent
                            radius: 4
                            color: "transparent"
                            border.width: 1
                            border.color: Appearance.colors.colOutlineVariant
                        }
                        
                        contentItem: RowLayout {
                            id: wipeContent
                            spacing: 8
                            
                            MaterialSymbol {
                                Layout.alignment: Qt.AlignVCenter
                                text: "delete_sweep"
                                iconSize: 18
                                color: Appearance.m3colors.m3onSurfaceVariant
                            }
                            
                            Text {
                                Layout.alignment: Qt.AlignVCenter
                                text: "Wipe All"
                                font.pixelSize: Appearance.font.pixelSize.smaller
                                color: Appearance.m3colors.m3onSurfaceVariant
                            }
                        }
                    }
                    
                    // Spacer
                    Item {
                        Layout.fillWidth: true
                    }
                }
            }
            
            // Plugin action bar - shows when plugin is active
            PluginActionBar {
                id: pluginActionBar
                visible: PluginRunner.isActive()
                Layout.fillWidth: true
                Layout.leftMargin: 12
                Layout.rightMargin: 12
                Layout.bottomMargin: 8
                Layout.preferredHeight: 34
                
                actions: PluginRunner.pluginActions
                navigationDepth: PluginRunner.navigationDepth
                
                onActionClicked: actionId => {
                    PluginRunner.executePluginAction(actionId);
                }
                
                onBackClicked: {
                    // Use goBack() to navigate back one step in the plugin
                    // If at initial view, this will close the plugin
                    LauncherSearch.exitPlugin();
                }
            }

            Rectangle {
                // Separator
                visible: root.showResults || root.showCard || root.showForm || PluginRunner.pluginBusy
                Layout.fillWidth: true
                // Both hint bar and plugin action bar have same height/margin, so no adjustment needed
                height: 1
                color: Appearance.colors.colOutlineVariant
            }

            // Loading indicator when plugin is processing and no card is shown.
            // Only show in submit mode - realtime mode should not show loading on every keystroke.
            // For card-based plugins (chat), the card itself shows its busy state.
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

            // Plugin card display (shown instead of results when card is present)
            // Use PluginRichCard when handler returns block-based cards (chat/timeline).
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

            // Plugin form display (shown when form response received)
            PluginForm {
                id: pluginFormView
                visible: root.showForm
                Layout.fillWidth: true
                Layout.leftMargin: 6
                Layout.rightMargin: 6
                Layout.bottomMargin: 6

                form: PluginRunner.pluginForm

                onSubmitted: formData => {
                    PluginRunner.submitForm(formData);
                    // Return focus to search bar after form submission
                    root.focusSearchInput();
                }

                onCancelled: {
                    PluginRunner.cancelForm();
                    // Return focus to search bar after form cancel
                    root.focusSearchInput();
                }
            }

            // Results container with recessed background
            // In realtime mode, keep results visible during processing to avoid flickering.
            // In submit mode, hide results while loading indicator is shown.
            Rectangle {
                id: resultsContainer
                visible: root.showResults && !root.showCard && !root.showForm && !(PluginRunner.pluginBusy && PluginRunner.inputMode === "submit")
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

                ListView { // App results
                    id: appResults
                    anchors.fill: parent
                    implicitHeight: Math.min(600, appResults.contentHeight + topMargin + bottomMargin)
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

                    // Track selected item key and action index for poll update restoration
                    property string selectedItemKey: ""
                    property int selectedActionIndex: -1

                    // Update selected key when selection changes
                    onCurrentIndexChanged: {
                        if (currentIndex >= 0 && currentIndex < LauncherSearch.results.length) {
                            selectedItemKey = LauncherSearch.results[currentIndex]?.key ?? "";
                        } else {
                            selectedItemKey = "";
                        }
                    }

                    // Track action index changes from current item
                    function updateActionIndex(index) {
                        selectedActionIndex = index;
                    }

                    model: ScriptModel {
                        id: model
                        objectProp: "key"
                        values: LauncherSearch.results
                        onValuesChanged: {
                            // Only preserve selection during poll updates (not user search/actions)
                            if (PluginRunner.isPollUpdate) {
                                PluginRunner.isPollUpdate = false;  // Clear flag after processing

                                const savedKey = appResults.selectedItemKey;
                                const savedActionIndex = appResults.selectedActionIndex;

                                if (savedKey) {
                                    const newIndex = LauncherSearch.results.findIndex(r => r.key === savedKey);
                                    if (newIndex >= 0) {
                                        appResults.currentIndex = newIndex;
                                        // Restore action focus after delegate is ready
                                        if (savedActionIndex >= 0) {
                                            Qt.callLater(() => {
                                                const currentItem = appResults.itemAtIndex(appResults.currentIndex);
                                                if (currentItem) {
                                                    currentItem.focusedActionIndex = savedActionIndex;
                                                }
                                            });
                                        }
                                        return;  // Selection preserved
                                    }
                                }
                            }

                            // Skip auto focus - keep current selection
                            if (LauncherSearch.skipNextAutoFocus) {
                                LauncherSearch.skipNextAutoFocus = false;
                                // Try to preserve selection by key if results changed
                                const savedKey = appResults.selectedItemKey;
                                if (savedKey) {
                                    const newIndex = LauncherSearch.results.findIndex(r => r.key === savedKey);
                                    if (newIndex >= 0) {
                                        appResults.currentIndex = newIndex;
                                        return;
                                    }
                                }
                                // Item was removed, select next available
                                if (appResults.currentIndex >= appResults.count) {
                                    appResults.currentIndex = Math.max(0, appResults.count - 1);
                                }
                                return;
                            }

                            // Normal update - reset to first
                            appResults.currentIndex = -1;
                            appResults.selectedActionIndex = -1;
                            root.focusFirstItem();
                        }
                    }

                    delegate: SearchItem {
                        // The selectable item for each search result
                        anchors.left: parent?.left
                        anchors.right: parent?.right
                        entry: modelData
                        query: StringUtils.cleanOnePrefix(root.searchingText, [Config.options.search.prefix.action, Config.options.search.prefix.app, Config.options.search.prefix.clipboard, Config.options.search.prefix.emojis, Config.options.search.prefix.math, Config.options.search.prefix.shellCommand, Config.options.search.prefix.webSearch])
                    }
                }
            }
        }
    }
}
