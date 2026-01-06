// pragma NativeMethodBehavior: AcceptThisObject
import qs
import qs.services
import qs.modules.common
import qs.modules.common.models
import qs.modules.common.widgets
import qs.modules.common.functions
import QtQuick
import QtQuick.Controls
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
    // Display properties - initialized from entry, updated imperatively by Connections
    // for switch items (ScriptModel doesn't notify delegates when item properties change)
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
    property string bigText: entry?.iconType === LauncherSearchResult.IconType.Text ? iconName : ""
    property string materialSymbol: entry?.iconType === LauncherSearchResult.IconType.Material ? iconName : ""
    property string thumbnail: entry?.thumbnail ?? ""
    property bool isPluginEntry: entry?.resultType === LauncherSearchResult.ResultType.PluginEntry
    property string entryPluginId: entry?.pluginId ?? ""
     // For plugin entries, get badges/chips from live status; otherwise use entry values
     // Depends on statusVersion to trigger re-evaluation when status updates (plugin entries)
     // Depends on resultsVersion for non-plugin entries (item updates)
     property var badges: {
         const _statusVersion = PluginRunner.statusVersion; // Reactive for plugin entries
         const _resultsVersion = PluginRunner.resultsVersion; // Reactive for item updates
         if (isPluginEntry && entryPluginId) {
             const status = PluginRunner.getPluginStatus(entryPluginId);
             if (status?.badges) return status.badges;
         }
         return entry?.badges ?? [];
     }
     property var chips: {
         const _statusVersion = PluginRunner.statusVersion; // Reactive for plugin entries
         const _resultsVersion = PluginRunner.resultsVersion; // Reactive for item updates
         if (isPluginEntry && entryPluginId) {
             const status = PluginRunner.getPluginStatus(entryPluginId);
             if (status?.chips) return status.chips;
         }
         return entry?.chips ?? [];
     }
     // Live data lookup for indexed items - allows real-time updates from daemon
     property var liveData: {
         const _indexVersion = PluginRunner.indexVersion;
         const pluginId = entry?._pluginId;
         const itemId = entry?.id;
         if (pluginId && itemId) {
             return PluginRunner.getIndexedItem(pluginId, itemId);
         }
         return null;
     }
     // Helper to get value with live fallback
     function getLiveValue(key, fallback) {
         return liveData?.[key] ?? entry?.[key] ?? fallback;
     }
     
     property var graphData: {
         const _version = PluginRunner.resultsVersion;
         const _indexVersion = PluginRunner.indexVersion;
         return getLiveValue("graph", null);
     }
     property var gaugeData: {
         const _version = PluginRunner.resultsVersion;
         const _indexVersion = PluginRunner.indexVersion;
         return getLiveValue("gauge", null);
     }
     // Progress bar properties
     property var progressData: {
         const _version = PluginRunner.resultsVersion;
         const _indexVersion = PluginRunner.indexVersion;
         return getLiveValue("progress", null);
     }
     property bool hasProgress: progressData !== null
     // Slider item properties
     property bool isSliderItem: entry?.resultType === "slider" || entry?.type === "slider"
     property real sliderValue: {
         const _version = PluginRunner.resultsVersion;
         return getLiveValue("value", 0);
     }
     property real sliderMin: getLiveValue("min", 0)
     property real sliderMax: getLiveValue("max", 100)
     property real sliderStep: getLiveValue("step", 1)
     property string sliderDisplayValue: getLiveValue("displayValue", "")
     property string sliderUnit: getLiveValue("unit", "")
     
     // Switch item properties
     property bool isSwitchItem: entry?.resultType === "switch" || entry?.type === "switch"
     property bool switchValue: liveData?.value ?? entry?.value ?? false
     
     // Listen for version changes to update display properties
     // This is needed because ScriptModel doesn't notify delegates when item properties change
     // - resultsVersion: for plugin view items (inside a plugin)
     // - indexVersion: for indexed items (main search view)
     Connections {
         target: PluginRunner
         function onResultsVersionChanged() {
             if (root.isSwitchItem && root.entry) {
                 root.itemName = root.entry.name ?? ""
                 root.iconName = root.entry.iconName ?? ""
                 root.itemComment = root.entry.comment ?? ""
             }
         }
         function onIndexVersionChanged() {
             if (root.isSwitchItem && root.liveData) {
                 // For indexed items, read from liveData (updated index)
                 root.itemName = root.liveData.name ?? root.itemName
                 root.iconName = root.liveData.icon ?? root.iconName
                 root.itemComment = root.liveData.description ?? root.itemComment
             }
         }
     }
    
    function adjustSlider(direction) {
        if (!isSliderItem) return
        const delta = direction * sliderStep
        const newValue = Math.max(sliderMin, Math.min(sliderMax, itemSlider.value + delta))
        itemSlider.value = newValue
        PluginRunner.sliderValueChanged(entry?.id ?? "", newValue, entry?._pluginId)
    }
    
    function toggleSwitch() {
        if (!isSwitchItem) return
        const newValue = !itemSwitch.checked
        itemSwitch.checked = newValue
        PluginRunner.switchValueChanged(entry?.id ?? "", newValue, entry?._pluginId)
    }
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

    onPressed: {
        // Immediately update selection to clicked item to prevent scroll jump
        // Skip for items that navigate to a new view (plugin entry/result)
        const isPluginEntry = entry?.resultType === LauncherSearchResult.ResultType.PluginEntry;
        const isPluginResult = entry?.resultType === LauncherSearchResult.ResultType.PluginResult;
        if (isPluginEntry || isPluginResult) return;
        
        const listView = root.ListView.view;
        if (listView && entry?.key) {
            const idx = LauncherSearch.results.findIndex(r => r.key === entry.key);
            if (idx >= 0) {
                listView.currentIndex = idx;
            }
        }
    }

    onClicked: {
        const isPluginEntry = entry?.resultType === LauncherSearchResult.ResultType.PluginEntry;
        const isPluginResult = entry?.resultType === LauncherSearchResult.ResultType.PluginResult;
        const shouldKeepOpen = entry?.keepOpen === true;

        if (isPluginEntry || isPluginResult) {
            // Navigating to new view - just execute, selection will reset
            root.itemExecute()
            return
        }

        if (shouldKeepOpen) {
            // Action that keeps current view - capture selection for restoration
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
         // Slider keyboard controls (arrow keys)
         if (root.isSliderItem) {
             if (event.key === Qt.Key_Left) {
                 root.adjustSlider(-1)
                 event.accepted = true
                 return
             }
             if (event.key === Qt.Key_Right) {
                 root.adjustSlider(1)
                 event.accepted = true
                 return
             }
         }
         
         // Switch keyboard controls (arrow keys or space to toggle)
         if (root.isSwitchItem) {
             if (event.key === Qt.Key_Left || event.key === Qt.Key_Right || event.key === Qt.Key_Space) {
                 root.toggleSwitch()
                 event.accepted = true
                 return
             }
         }
         
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
            visible: root.thumbnail !== "" || root.iconType !== LauncherSearchResult.IconType.None || root.graphData !== null || root.gaugeData !== null
             
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
            
            LineGraph {
                visible: !thumbnailRect.visible && root.graphData !== null
                anchors.centerIn: parent
                data: root.graphData?.data ?? []
                size: Appearance.sizes.resultIconSize
            }
            
            Gauge {
                visible: !thumbnailRect.visible && root.gaugeData !== null && root.graphData === null
                anchors.centerIn: parent
                value: root.gaugeData?.value ?? 0
                max: root.gaugeData?.max ?? 100
                label: root.gaugeData?.label ?? ""
                size: Appearance.sizes.resultIconSize
            }
            
            IconImage {
                visible: !thumbnailRect.visible && root.iconType === LauncherSearchResult.IconType.System && root.graphData === null && root.gaugeData === null
                anchors.centerIn: parent
                source: {
                    if (!root.iconName) return "";
                    const resolved = IconResolver.guessIcon(root.iconName);
                    return resolved.startsWith("/") ? "file://" + resolved : Quickshell.iconPath(resolved, "image-missing");
                }
                width: 32
                height: 32
            }
            
            MaterialSymbol {
                visible: !thumbnailRect.visible && root.iconType === LauncherSearchResult.IconType.Material && root.graphData === null && root.gaugeData === null
                anchors.centerIn: parent
                text: root.materialSymbol
                iconSize: 26
                color: Appearance.m3colors.m3onSurface
            }
            
            StyledText {
                visible: !thumbnailRect.visible && root.iconType === LauncherSearchResult.IconType.Text && root.graphData === null && root.gaugeData === null
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
                spacing: 4
                
                Repeater {
                    model: (root.query == root.itemName ? [] : root.urls).slice(0, 3)
                    Favicon {
                        required property var modelData
                        size: parent.height
                        url: modelData
                    }
                }
                
                StyledText {
                    id: nameText
                    textFormat: Text.StyledText
                    font.pixelSize: Appearance.font.pixelSize.small
                    font.family: Appearance.font.family[root.fontType]
                    color: Appearance.m3colors.m3onSurface
                    horizontalAlignment: Text.AlignLeft
                    elide: Text.ElideRight
                    text: root.displayContent
                    Layout.fillWidth: root.badges.length === 0

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
                
                Item { Layout.fillWidth: true }
            }
             StyledText {
                 Layout.fillWidth: true
                visible: root.itemComment !== "" && !root.hasProgress
                font.pixelSize: Appearance.font.pixelSize.smallest
                font.family: Appearance.font.family.monospace
                color: Appearance.colors.colSubtext
                horizontalAlignment: Text.AlignLeft
                elide: Text.ElideMiddle
                text: root.itemComment
            }
            
            // Progress bar
            RowLayout {
                visible: root.hasProgress
                Layout.fillWidth: true
                spacing: 8
                
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 4
                    radius: 2
                    color: Appearance.colors.colSurfaceContainerHigh
                    
                    Rectangle {
                        width: parent.width * ((root.progressData?.value ?? 0) / (root.progressData?.max ?? 100))
                        height: parent.height
                        radius: parent.radius
                        color: root.progressData?.color ? Qt.color(root.progressData.color) : Appearance.colors.colPrimary
                        
                        Behavior on width {
                            NumberAnimation { duration: 200; easing.type: Easing.OutCubic }
                        }
                    }
                }
                
                StyledText {
                    visible: root.progressData?.label ?? false
                    font.pixelSize: Appearance.font.pixelSize.smallest
                    font.family: Appearance.font.family.monospace
                    color: Appearance.colors.colSubtext
                    text: root.progressData?.label ?? ""
                }
            }
        }

        // Slider for slider items
        Slider {
            id: itemSlider
            visible: root.isSliderItem
            Layout.alignment: Qt.AlignVCenter
            Layout.preferredWidth: 200
            min: root.sliderMin
            max: root.sliderMax
            step: root.sliderStep
            displayValue: root.sliderDisplayValue
            unit: root.sliderUnit
            
            // Initialize value from entry, but don't bind (allows local updates)
            Component.onCompleted: value = root.sliderValue
            
            // Update only when entry changes (new item selected)
            Connections {
                target: root
                function onEntryChanged() {
                    if (root.isSliderItem) {
                        itemSlider.value = root.sliderValue
                    }
                }
            }
            
            // Update when live data changes from daemon (indexVersion increments)
            Connections {
                target: PluginRunner
                function onIndexVersionChanged() {
                    if (root.isSliderItem && root.liveData) {
                        itemSlider.value = root.liveData.value ?? itemSlider.value
                    }
                }
            }
            
            onValueCommitted: (newValue) => {
                PluginRunner.sliderValueChanged(root.entry?.id ?? "", newValue, root.entry?._pluginId)
            }
        }
        
        // Badges for slider items
        RowLayout {
            visible: root.isSliderItem && root.badges.length > 0
            Layout.alignment: Qt.AlignVCenter
            spacing: 8
            
            Repeater {
                model: root.badges.slice(0, 5)
                
                delegate: Badge {
                    required property var modelData
                    text: modelData.text ?? ""
                    image: modelData.image ?? ""
                    icon: modelData.icon ?? ""
                    textColor: modelData.color ? Qt.color(modelData.color) : Appearance.m3colors.m3onSurface
                }
            }
        }
        
        // Switch for switch items
        Item {
            visible: root.isSwitchItem
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: 44
            implicitHeight: 24
            
            Rectangle {
                id: switchTrack
                anchors.fill: parent
                radius: 12
                color: itemSwitch.checked ? Appearance.colors.colPrimary : Appearance.colors.colSurfaceContainerHigh
                border.width: itemSwitch.checked ? 0 : 1
                border.color: Appearance.colors.colOutlineVariant
                
                Behavior on color {
                    ColorAnimation { duration: 200 }
                }
                
                Rectangle {
                    id: switchThumb
                    width: 20
                    height: 20
                    radius: 10
                    color: itemSwitch.checked ? Appearance.m3colors.m3onPrimary : Appearance.m3colors.m3outline
                    anchors.verticalCenter: parent.verticalCenter
                    x: itemSwitch.checked ? parent.width - width - 2 : 2
                    
                    Behavior on x {
                        NumberAnimation { duration: 200 }
                    }
                    
                    Behavior on color {
                        ColorAnimation { duration: 200 }
                    }
                }
                
                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.toggleSwitch()
                }
            }
            
            CheckBox {
                id: itemSwitch
                visible: false
                
                Component.onCompleted: checked = root.switchValue
                
                Connections {
                    target: root
                    function onEntryChanged() {
                        if (root.isSwitchItem) {
                            itemSwitch.checked = root.switchValue
                        }
                    }
                }
                
                Connections {
                    target: PluginRunner
                    function onIndexVersionChanged() {
                        if (root.isSwitchItem && root.liveData) {
                            itemSwitch.checked = root.liveData.value ?? itemSwitch.checked
                        }
                    }
                    function onResultsVersionChanged() {
                        if (root.isSwitchItem && root.entry) {
                            const newValue = root.liveData?.value ?? root.entry.value ?? false
                            itemSwitch.checked = newValue
                        }
                    }
                }
            }
        }
        
        // Badges for switch items
        RowLayout {
            visible: root.isSwitchItem && root.badges.length > 0
            Layout.alignment: Qt.AlignVCenter
            spacing: 8
            
            Repeater {
                model: root.badges.slice(0, 5)
                
                delegate: Badge {
                    required property var modelData
                    text: modelData.text ?? ""
                    image: modelData.image ?? ""
                    icon: modelData.icon ?? ""
                    textColor: modelData.color ? Qt.color(modelData.color) : Appearance.m3colors.m3onSurface
                }
            }
        }

        RowLayout {
            visible: !root.isSliderItem && !root.isSwitchItem
            Layout.alignment: Qt.AlignVCenter
            Layout.fillHeight: false
            spacing: 4
            
            // Primary action hint (shown when selected)
            RowLayout {
                id: primaryActionHint
                Layout.rightMargin: 6
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
            
            // Badges (always shown)
            Repeater {
                model: root.badges.slice(0, 5)
                
                delegate: Badge {
                    required property var modelData
                    Layout.rightMargin: 4
                    text: modelData.text ?? ""
                    image: modelData.image ?? ""
                    icon: modelData.icon ?? ""
                    textColor: modelData.color ? Qt.color(modelData.color) : Appearance.m3colors.m3onSurface
                }
            }
            
            // Chips (pill-shaped tags for longer text)
            Repeater {
                model: root.chips.slice(0, 3)
                
                delegate: Chip {
                    required property var modelData
                    Layout.rightMargin: 4
                    text: modelData.text ?? ""
                    icon: modelData.icon ?? ""
                    backgroundColor: modelData.background ? Qt.color(modelData.background) : Appearance.colors.colSurfaceContainerHighest
                    textColor: modelData.color ? Qt.color(modelData.color) : Appearance.m3colors.m3onSurface
                }
            }
            
            Repeater {
                model: (root.entry?.actions ?? []).slice(0, 4)
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
                            source: actionButton.iconName.startsWith("/") ? "file://" + actionButton.iconName : Quickshell.iconPath(actionButton.iconName)
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
                            const listView = root.ListView.view
                            // Update currentIndex to clicked item so restoration uses this position
                            if (listView && root.entry?.key) {
                                const idx = LauncherSearch.results.findIndex(r => r.key === root.entry.key)
                                if (idx >= 0) {
                                    listView.currentIndex = idx
                                }
                            }
                            // Capture selection before action executes (for restoration after results update)
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
        if (!root.entry) return;
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
