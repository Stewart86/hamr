/**
 * ActionBar - Unified toolbar for search hints and plugin actions
 * 
 * Renders contextually based on the current mode:
 * - "hints": Shows search prefix shortcuts (files, clipboard, plugins, etc.)
 * - "search": Shows back button and search-specific actions (e.g., Wipe All for clipboard)
 * - "plugin": Shows home/back buttons, navigation depth, and plugin actions
 * 
 * Features:
 * - Icon + Kbd buttons with tooltips
 * - Home button (double Esc) - returns to main view (plugin mode only)
 * - Back button (single Esc) - goes back one level
 * - Up to 6 action buttons with keyboard shortcuts
 * - Confirmation dialog for dangerous actions
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
                        keys: "^Bksp"
                        opacity: 0.5
                    }
                    
                    Kbd {
                        id: stackedKbd
                        keys: "^Bksp"
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
        
        // Action buttons
        Repeater {
            model: root.actions.slice(0, 6)
            
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
        
        // Spacer
        Item {
            Layout.fillWidth: true
        }
        
        // Keybinding help button
        RippleButton {
            id: keybindingBtn
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
    
    // Whether to show separator (set by parent based on content below)
    property bool showSeparator: false
    
    // Separator line at the bottom
    Rectangle {
        visible: root.showSeparator
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.bottomMargin: -8
        height: 1
        color: Appearance.colors.colOutlineVariant
    }
    
    // Handle keyboard shortcuts
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
