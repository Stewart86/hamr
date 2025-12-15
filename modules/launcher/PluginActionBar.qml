/**
 * PluginActionBar - Toolbar for plugin-level actions
 * 
 * Displays action buttons for the active plugin. These are global actions
 * that apply to the plugin itself (e.g., "Add", "Wipe", "Refresh") rather
 * than to specific items in the result list.
 * 
 * Features:
 * - Up to 6 action buttons with icons and labels
 * - Keyboard shortcuts (Ctrl+1 through Ctrl+6)
 * - Confirmation dialog for dangerous actions (when action has `confirm` field)
 */
import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import qs.modules.common
import qs.modules.common.widgets
import qs.services

Item {
    id: root
    
    // Actions to display (max 6)
    // Each action: { id, name, icon, confirm?: string, shortcut?: string }
    property var actions: []
    
    // Signal when an action is clicked (after confirmation if needed)
    signal actionClicked(string actionId)
    
    // Currently showing confirmation for this action
    property var pendingConfirmAction: null
    
    // Signal when back button is clicked
    signal backClicked()
    
    // Action buttons row
    RowLayout {
        id: actionsRow
        anchors.fill: parent
        spacing: 8
        visible: root.pendingConfirmAction === null
        
        // Back button (always shown)
        RippleButton {
            id: backBtn
            Layout.fillHeight: true
            implicitWidth: backContent.implicitWidth + 16
            
            buttonRadius: 4
            colBackground: "transparent"
            colBackgroundHover: Appearance.colors.colSurfaceContainerHighest
            colRipple: Appearance.colors.colSurfaceContainerHighest
            
            onClicked: root.backClicked()
            
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
        
        Repeater {
            model: root.actions.slice(0, 5)  // Reduced to 5 since back button takes one slot
            
            delegate: RippleButton {
                id: actionBtn
                required property var modelData
                required property int index
                
                property string actionId: modelData.id ?? ""
                property string actionName: modelData.name ?? ""
                property string actionIcon: modelData.icon ?? "play_arrow"
                property string confirmMessage: modelData.confirm ?? ""
                property string shortcutKey: modelData.shortcut ?? `Ctrl+${index + 1}`
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
                        root.actionClicked(actionBtn.actionId);
                    }
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
                    id: btnContent
                    spacing: 8
                    
                    MaterialSymbol {
                        Layout.alignment: Qt.AlignVCenter
                        text: actionBtn.actionIcon
                        iconSize: 18
                        color: Appearance.m3colors.m3onSurfaceVariant
                    }
                    
                    Text {
                        Layout.alignment: Qt.AlignVCenter
                        text: actionBtn.actionName
                        font.pixelSize: Appearance.font.pixelSize.smaller
                        color: Appearance.m3colors.m3onSurfaceVariant
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
    }
    
    // Confirmation dialog overlay
    Rectangle {
        id: confirmDialog
        visible: root.pendingConfirmAction !== null
        anchors.fill: parent
        color: Appearance.m3colors.m3surfaceContainer
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
            
            // Cancel button
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
                        keys: "Esc"
                    }
                }
            }
            
            // Confirm button
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
                        root.actionClicked(actionId);
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
    
    // Handle keyboard shortcuts
    Keys.onPressed: event => {
        // Escape cancels confirmation
        if (event.key === Qt.Key_Escape && root.pendingConfirmAction !== null) {
            root.pendingConfirmAction = null;
            event.accepted = true;
            return;
        }
        
        // Enter confirms pending action
        if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && root.pendingConfirmAction !== null) {
            const actionId = root.pendingConfirmAction?.id ?? "";
            root.pendingConfirmAction = null;
            if (actionId) {
                root.actionClicked(actionId);
            }
            event.accepted = true;
            return;
        }
        
        // Ctrl+1 through Ctrl+6 for action shortcuts (only when no confirmation pending)
        if (root.pendingConfirmAction === null && (event.modifiers & Qt.ControlModifier)) {
            const keyIndex = event.key - Qt.Key_1;
            if (keyIndex >= 0 && keyIndex < root.actions.length && keyIndex < 6) {
                const action = root.actions[keyIndex];
                if (action.confirm) {
                    root.pendingConfirmAction = action;
                } else {
                    root.actionClicked(action.id);
                }
                event.accepted = true;
            }
        }
    }
}
