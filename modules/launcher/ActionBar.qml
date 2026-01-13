/**
 * ActionBar - Unified toolbar for search hints and plugin actions
 * 
 * Renders contextually based on the current mode:
 * - "hints": Shows search prefix shortcuts (files, clipboard, plugins, etc.)
 *            When ambient items exist, collapses prefixes and shows ambient items
 * - "search": Shows back button and search-specific actions (e.g., Wipe All for clipboard)
 * - "plugin": Shows home/back buttons, navigation depth, and plugin actions
 * 
 * Features:
 * - Icon + Kbd buttons with tooltips
 * - Home button (double Esc) - returns to main view (plugin mode only)
 * - Back button (single Esc) - goes back one level
 * - Up to 6 action buttons with keyboard shortcuts
 * - Confirmation dialog for dangerous actions
 * - Ambient items display with collapsed prefix indicator
 */
import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import qs.modules.common
import qs.modules.common.widgets
import qs.services

Item {
    id: root
    
    // Mode: "hints" | "search" | "plugin"
    property string mode: "hints"
    
    // Actions to display (format depends on mode)
    // hints mode: [{ icon, key, label }]
    // search mode: [{ id, icon, name, confirm?, shortcut? }]
    // plugin mode: [{ id, icon, name, confirm?, shortcut? }]
    property var actions: []
    
    // Navigation depth (plugin mode only)
    property int navigationDepth: 0
    
    // Whether to show browser-style keys (hjkl for grid navigation)
    property bool showBrowserKeys: false
    
    // Currently showing confirmation for this action
    property var pendingConfirmAction: null
    
    // Ambient items from PluginRunner
    readonly property var ambientItems: {
        const _version = PluginRunner.ambientVersion;
        return PluginRunner.getAmbientItems();
    }
    readonly property bool hasAmbientItems: ambientItems.length > 0
    readonly property bool showAmbientMode: mode === "hints" && hasAmbientItems
    
    // Signals
    signal actionClicked(string actionId, bool wasConfirmed)
    signal backClicked()
    signal homeClicked()
    signal keybindingHelpRequested()
    
    // Main content row
    RowLayout {
        id: actionsRow
        anchors.fill: parent
        spacing: 8
        visible: root.pendingConfirmAction === null
        
        // Ambient items row - shown when ambient items exist in hints mode
        Item {
            id: ambientContainer
            visible: root.showAmbientMode
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            
            Row {
                id: ambientRow
                anchors.verticalCenter: parent.verticalCenter
                spacing: 6
                
                // Animate position for marquee effect when overflowing
                x: ambientMarquee.running ? ambientMarquee.xPos : 0
                
                Repeater {
                    model: root.ambientItems
                    
                    delegate: AmbientItem {
                        id: ambientItemDelegate
                        required property var modelData
                        required property int index
                        
                        item: modelData
                        pluginId: modelData.pluginId ?? ""
                        
                        onDismissed: {
                            PluginRunner.handleAmbientAction(pluginId, item.id, "__dismiss__");
                            PluginRunner.removeAmbientItem(pluginId, item.id);
                        }
                        
                        onActionClicked: (actionId) => {
                            PluginRunner.handleAmbientAction(pluginId, item.id, actionId);
                        }
                    }
                }
            }
            
            // Marquee animation for overflow - continuous scroll with wrap
            Timer {
                id: ambientMarquee
                property real xPos: 0
                property real maxScroll: Math.max(0, ambientRow.width - ambientContainer.width + 50)
                
                interval: 50
                repeat: true
                running: ambientRow.width > ambientContainer.width && ambientContainer.visible
                
                onTriggered: {
                    xPos -= 1;
                    if (xPos <= -maxScroll) {
                        xPos = 50;
                    }
                }
                
                onRunningChanged: {
                    if (!running) {
                        xPos = 0;
                    }
                }
            }
        }
        
        // Home button - only in plugin mode
        RippleButton {
            id: homeBtn
            visible: root.mode === "plugin"
            Layout.fillHeight: true
            implicitWidth: homeContent.implicitWidth + 16
            
            buttonRadius: 4
            colBackground: "transparent"
            colBackgroundHover: Appearance.colors.colSurfaceContainerHighest
            colRipple: Appearance.colors.colSurfaceContainerHighest
            
            onClicked: root.homeClicked()
            
            StyledToolTip {
                text: "Home"
            }
            
            Rectangle {
                anchors.fill: parent
                radius: 4
                color: "transparent"
                border.width: 1
                border.color: Appearance.colors.colOutlineVariant
            }
            
            contentItem: RowLayout {
                id: homeContent
                spacing: 8
                
                MaterialSymbol {
                    Layout.alignment: Qt.AlignVCenter
                    text: "home"
                    iconSize: 18
                    color: Appearance.m3colors.m3onSurfaceVariant
                }
                
                Item {
                    Layout.alignment: Qt.AlignVCenter
                    implicitWidth: stackedKbd.width + 3
                    implicitHeight: stackedKbd.height + 3
                    
                    Kbd {
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        keys: "Shift+Bksp"
                        opacity: 0.5
                    }
                    
                    Kbd {
                        id: stackedKbd
                        keys: "Shift+Bksp"
                    }
                }
            }
        }
        
        // Back button - in prefix and plugin modes
        RippleButton {
            id: backBtn
            visible: root.mode === "search" || root.mode === "plugin"
            Layout.fillHeight: true
            implicitWidth: backContent.implicitWidth + 16
            enabled: root.mode === "search" || root.navigationDepth > 0
            opacity: enabled ? 1.0 : 0.4
            
            buttonRadius: 4
            colBackground: "transparent"
            colBackgroundHover: Appearance.colors.colSurfaceContainerHighest
            colRipple: Appearance.colors.colSurfaceContainerHighest
            
            onClicked: root.backClicked()
            
            StyledToolTip {
                text: "Back"
                extraVisibleCondition: backBtn.enabled
            }
            
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
                
                Kbd {
                    Layout.alignment: Qt.AlignVCenter
                    keys: "Bksp"
                }
            }
        }
        
        // Navigation depth indicator - only in plugin mode
        Row {
            id: depthIndicator
            Layout.alignment: Qt.AlignVCenter
            spacing: 4
            visible: root.mode === "plugin" && root.navigationDepth > 0
            
            Repeater {
                model: root.navigationDepth
                
                delegate: Rectangle {
                    id: depthDot
                    required property int index
                    
                    width: 6
                    height: 6
                    radius: 3
                    color: Appearance.m3colors.m3primary
                    opacity: 0.7
                    
                    scale: 0
                    Component.onCompleted: scaleIn.start()
                    
                    NumberAnimation {
                        id: scaleIn
                        target: depthDot
                        property: "scale"
                        from: 0
                        to: 1
                        duration: 150
                        easing.type: Easing.OutBack
                    }
                }
            }
        }
        
        // Action buttons - hidden in hints mode (shortcuts are in popup)
        Repeater {
            model: root.mode === "hints" ? [] : root.actions.slice(0, 6)
            
            delegate: RippleButton {
                id: actionBtn
                required property var modelData
                required property int index
                
                // Support both formats: { key, label, icon } and { id, name, icon, shortcut }
                property string actionId: modelData.id ?? modelData.key ?? ""
                property string actionName: modelData.name ?? modelData.label ?? ""
                property string actionIcon: modelData.icon ?? ""
                property string confirmMessage: modelData.confirm ?? ""
                property string shortcutKey: {
                    if (modelData.shortcut !== undefined) return modelData.shortcut;
                    if (modelData.key !== undefined) return modelData.key;
                    if (root.mode === "plugin") return `Ctrl+${index + 1}`;
                    return "";
                }
                property bool hasConfirm: confirmMessage !== ""
                
                Layout.fillHeight: true
                implicitWidth: btnContent.implicitWidth + 16
                
                buttonRadius: 4
                colBackground: "transparent"
                colBackgroundHover: Appearance.colors.colSurfaceContainerHighest
                colRipple: Appearance.colors.colSurfaceContainerHighest
                
                onClicked: {
                    if (actionBtn.hasConfirm) {
                        root.pendingConfirmAction = actionBtn.modelData;
                    } else {
                        root.actionClicked(actionBtn.actionId, false);
                    }
                }
                
                StyledToolTip {
                    text: actionBtn.actionName
                }
                
                Rectangle {
                    anchors.fill: parent
                    radius: 4
                    color: "transparent"
                    border.width: 1
                    border.color: Appearance.colors.colOutlineVariant
                }
                
                contentItem: RowLayout {
                    id: btnContent
                    spacing: 8
                    
                    MaterialSymbol {
                        Layout.alignment: Qt.AlignVCenter
                        text: actionBtn.actionIcon
                        iconSize: 18
                        color: Appearance.m3colors.m3onSurfaceVariant
                        visible: actionBtn.actionIcon !== ""
                    }
                    
                    Kbd {
                        Layout.alignment: Qt.AlignVCenter
                        keys: actionBtn.shortcutKey
                        visible: actionBtn.shortcutKey !== ""
                    }
                }
            }
        }
        
        // Spacer - hidden when ambient mode is active (ambient container fills width)
        Item {
            visible: !root.showAmbientMode
            Layout.fillWidth: true
        }
        
        // Shortcuts & help button - minimal icon that shows popup (hidden when ambient items shown)
        Item {
            id: shortcutsBtn
            visible: root.mode === "hints" && !root.hasAmbientItems
            Layout.alignment: Qt.AlignTop
            Layout.topMargin: -12
            implicitWidth: 20
            implicitHeight: 20
            
            MaterialSymbol {
                id: shortcutsIcon
                anchors.centerIn: parent
                text: "more_horiz"
                iconSize: 16
                color: shortcutsMouse.containsMouse ? Appearance.m3colors.m3onSurfaceVariant : Appearance.m3colors.m3outline
                opacity: shortcutsMouse.containsMouse ? 1.0 : 0.6
                
                Behavior on color { ColorAnimation { duration: 100 } }
                Behavior on opacity { NumberAnimation { duration: 100 } }
            }
            
            MouseArea {
                id: shortcutsMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: shortcutsPopup.open()
            }
            
            StyledToolTip {
                visible: shortcutsMouse.containsMouse
                text: "Shortcuts & help"
            }
            
            Popup {
                id: shortcutsPopup
                x: parent.width - width
                y: parent.height + 4
                padding: 8
                
                background: Rectangle {
                    color: Appearance.colors.colSurfaceContainer
                    radius: Appearance.rounding.small
                    border.width: 1
                    border.color: Appearance.colors.colOutlineVariant
                }
                
                contentItem: Column {
                    spacing: 8
                    
                    // Prefix shortcuts
                    Row {
                        spacing: 4
                        
                        Repeater {
                            model: root.actions.slice(0, 6)
                            
                            delegate: RippleButton {
                                id: popupActionBtn
                                required property var modelData
                                required property int index
                                
                                property string actionId: modelData.id ?? modelData.key ?? ""
                                property string actionName: modelData.name ?? modelData.label ?? ""
                                property string actionIcon: modelData.icon ?? ""
                                property string shortcutKey: modelData.shortcut ?? modelData.key ?? ""
                                
                                implicitWidth: popupBtnContent.implicitWidth + 12
                                implicitHeight: 28
                                
                                buttonRadius: 4
                                colBackground: "transparent"
                                colBackgroundHover: Appearance.colors.colSurfaceContainerHighest
                                colRipple: Appearance.colors.colSurfaceContainerHighest
                                
                                onClicked: {
                                    shortcutsPopup.close();
                                    root.actionClicked(popupActionBtn.actionId, false);
                                }
                                
                                StyledToolTip {
                                    text: popupActionBtn.actionName
                                }
                                
                                Rectangle {
                                    anchors.fill: parent
                                    radius: 4
                                    color: "transparent"
                                    border.width: 1
                                    border.color: Appearance.colors.colOutlineVariant
                                }
                                
                                contentItem: RowLayout {
                                    id: popupBtnContent
                                    spacing: 4
                                    
                                    MaterialSymbol {
                                        Layout.alignment: Qt.AlignVCenter
                                        text: popupActionBtn.actionIcon
                                        iconSize: 14
                                        color: Appearance.m3colors.m3onSurfaceVariant
                                        visible: popupActionBtn.actionIcon !== ""
                                    }
                                    
                                    Kbd {
                                        Layout.alignment: Qt.AlignVCenter
                                        keys: popupActionBtn.shortcutKey
                                        visible: popupActionBtn.shortcutKey !== ""
                                    }
                                }
                            }
                        }
                    }
                    
                    // Separator
                    Rectangle {
                        width: parent.width
                        height: 1
                        color: Appearance.colors.colOutlineVariant
                    }
                    
                    // Keymap help button
                    RippleButton {
                        implicitWidth: keymapContent.implicitWidth + 12
                        implicitHeight: 28
                        
                        buttonRadius: 4
                        colBackground: "transparent"
                        colBackgroundHover: Appearance.colors.colSurfaceContainerHighest
                        colRipple: Appearance.colors.colSurfaceContainerHighest
                        
                        onClicked: {
                            shortcutsPopup.close();
                            root.keybindingHelpRequested();
                        }
                        
                        contentItem: RowLayout {
                            id: keymapContent
                            spacing: 6
                            
                            MaterialSymbol {
                                Layout.alignment: Qt.AlignVCenter
                                text: "keyboard"
                                iconSize: 14
                                color: Appearance.m3colors.m3onSurfaceVariant
                            }
                            
                            Text {
                                text: "Keyboard shortcuts"
                                font.pixelSize: Appearance.font.pixelSize.smaller
                                color: Appearance.m3colors.m3onSurfaceVariant
                            }
                            
                            Kbd {
                                Layout.alignment: Qt.AlignVCenter
                                keys: "?"
                            }
                        }
                    }
                }
            }
        }
        
        // Keybinding help button - only in search/plugin modes
        RippleButton {
            id: keybindingBtn
            visible: root.mode !== "hints"
            Layout.fillHeight: true
            implicitWidth: keybindingContent.implicitWidth + 12
            
            buttonRadius: 4
            colBackground: "transparent"
            colBackgroundHover: Appearance.colors.colSurfaceContainerHighest
            colRipple: Appearance.colors.colSurfaceContainerHighest
            
            onClicked: root.keybindingHelpRequested()
            
            StyledToolTip {
                text: "Keyboard shortcuts"
            }
            
            contentItem: RowLayout {
                id: keybindingContent
                spacing: 4
                
                MaterialSymbol {
                    Layout.alignment: Qt.AlignVCenter
                    text: "keyboard"
                    iconSize: 16
                    color: Appearance.m3colors.m3outline
                }
                
                Text {
                    text: "?"
                    font.pixelSize: Appearance.font.pixelSize.smaller
                    font.weight: Font.Medium
                    color: Appearance.m3colors.m3outline
                }
            }
        }
    }
    
    // Confirmation dialog overlay
    Rectangle {
        id: confirmDialog
        visible: root.pendingConfirmAction !== null
        anchors.fill: parent
        color: Appearance.colors.colSurfaceContainer
        radius: 4
        border.width: 1
        border.color: Appearance.colors.colOutlineVariant
        
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 12
            anchors.rightMargin: 12
            anchors.topMargin: 4
            anchors.bottomMargin: 4
            spacing: 12
            
            MaterialSymbol {
                text: "warning"
                iconSize: 20
                color: Appearance.colors.colError
            }
            
            Text {
                Layout.fillWidth: true
                text: root.pendingConfirmAction?.confirm ?? "Are you sure?"
                font.pixelSize: Appearance.font.pixelSize.smaller
                color: Appearance.m3colors.m3onSurface
                elide: Text.ElideRight
            }
            
            RippleButton {
                Layout.fillHeight: true
                implicitWidth: cancelContent.implicitWidth + 16
                buttonRadius: 4
                colBackground: "transparent"
                colBackgroundHover: Appearance.colors.colSurfaceContainerHighest
                
                onClicked: root.pendingConfirmAction = null
                
                contentItem: RowLayout {
                    id: cancelContent
                    spacing: 8
                    
                    Text {
                        text: "Cancel"
                        font.pixelSize: Appearance.font.pixelSize.smaller
                        color: Appearance.m3colors.m3onSurfaceVariant
                    }
                    
                    Kbd {
                        keys: "Bksp"
                    }
                }
            }
            
            RippleButton {
                Layout.fillHeight: true
                implicitWidth: confirmContent.implicitWidth + 16
                buttonRadius: 4
                colBackground: Qt.darker(Appearance.colors.colErrorContainer, 1.3)
                colBackgroundHover: Qt.darker(Appearance.colors.colErrorContainer, 1.15)
                colRipple: Qt.darker(Appearance.colors.colError, 1.2)
                
                onClicked: {
                    const actionId = root.pendingConfirmAction?.id ?? "";
                    root.pendingConfirmAction = null;
                    if (actionId) {
                        root.actionClicked(actionId, true);
                    }
                }
                
                contentItem: RowLayout {
                    id: confirmContent
                    spacing: 8
                    
                    Text {
                        text: "Confirm"
                        font.pixelSize: Appearance.font.pixelSize.smaller
                        font.weight: Font.Medium
                        color: Appearance.colors.colOnErrorContainer ?? Appearance.colors.colError
                    }
                    
                    Kbd {
                        keys: "Enter"
                        textColor: Appearance.colors.colOnErrorContainer ?? Appearance.colors.colError
                    }
                }
            }
        }
    }
    
    Keys.onPressed: event => {
        if (event.key === Qt.Key_Backspace && root.pendingConfirmAction !== null) {
            root.pendingConfirmAction = null;
            event.accepted = true;
            return;
        }
        
        if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && root.pendingConfirmAction !== null) {
            const actionId = root.pendingConfirmAction?.id ?? "";
            root.pendingConfirmAction = null;
            if (actionId) {
                root.actionClicked(actionId, true);
            }
            event.accepted = true;
            return;
        }
        
        // Ctrl+1 through Ctrl+6 for plugin action shortcuts
        if (root.mode === "plugin" && root.pendingConfirmAction === null && (event.modifiers & Qt.ControlModifier)) {
            const keyIndex = event.key - Qt.Key_1;
            if (keyIndex >= 0 && keyIndex < root.actions.length && keyIndex < 6) {
                const action = root.actions[keyIndex];
                if (action.confirm) {
                    root.pendingConfirmAction = action;
                } else {
                    root.actionClicked(action.id, false);
                }
                event.accepted = true;
            }
        }
    }
}
