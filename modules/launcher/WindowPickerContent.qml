import QtQuick
import QtQuick.Layouts
import Qt5Compat.GraphicalEffects
import Quickshell
import Quickshell.Wayland
import Quickshell.Widgets
import qs.modules.common
import qs.modules.common.widgets
import qs.modules.common.functions
import qs.services

/**
 * A horizontal popup showing live window previews with keyboard navigation.
 * 
 * Features:
 * - Live window previews using ScreencopyView
 * - Keyboard navigation (h/l/arrows/vim keys)
 * - Mouse-free workflow support
 * - Visual indication of selected window
 * 
 * Usage:
 *   WindowPicker {
 *       windows: MyWindowList.toplevels
 *       onWindowSelected: window => MyApp.activate(window)
 *       onWindowClosed: window => window.close()
 *   }
 */
Rectangle {
    id: root
    
    // ==================== PROPERTIES ====================
    
    /**
     * List of Toplevel objects to display as window previews
     */
    property list<var> windows: []
    
    /**
     * Currently selected window index (0-based)
     */
    property int selectedIndex: 0
    
    /**
     * Maximum preview width in pixels (from config)
     */
    property real maxPreviewWidth: Appearance.sizes.windowPickerMaxWidth
    
    /**
     * Maximum preview height in pixels (from config)
     */
    property real maxPreviewHeight: Appearance.sizes.windowPickerMaxHeight
    
    // ==================== SIGNALS ====================
    
    /**
     * Emitted when user selects a window (Enter/Space/click)
     */
    signal windowSelected(var toplevel)
    
    /**
     * Emitted when user closes a window (x button/Delete)
     */
    signal windowClosed(var toplevel)
    
    /**
     * Emitted when user presses Escape
     */
    signal cancelled()
    
    /**
     * Emitted when user presses 'n' for new instance
     */
    signal newInstanceRequested()
    
    // ==================== STYLING ====================
    
    color: Appearance.colors.colSurfaceContainer
    radius: Appearance.rounding.normal
    
    implicitWidth: windowLayout.implicitWidth + windowLayout.anchors.margins * 2
    implicitHeight: windowLayout.implicitHeight + windowLayout.anchors.margins * 2
    
    // ==================== KEYBOARD HANDLING ====================
    
    Keys.enabled: visible
    Keys.onPressed: event => {
        const key = event.key;
        const modifiers = event.modifiers;
        
        // Navigation: Previous window
        if ((key === Qt.Key_H) || 
            (key === Qt.Key_Left) || 
            (key === Qt.Key_K && modifiers & Qt.ControlModifier) ||
            (key === Qt.Key_H && modifiers & Qt.ControlModifier)) {
            selectedIndex = (selectedIndex - 1 + windows.length) % windows.length;
            event.accepted = true;
            return;
        }
        
        // Navigation: Next window
        if ((key === Qt.Key_L) || 
            (key === Qt.Key_Right) || 
            (key === Qt.Key_J && modifiers & Qt.ControlModifier) ||
            (key === Qt.Key_L && modifiers & Qt.ControlModifier)) {
            selectedIndex = (selectedIndex + 1) % windows.length;
            event.accepted = true;
            return;
        }
        
        // Vim-style navigation (without modifiers)
        if (key === Qt.Key_K) {
            selectedIndex = (selectedIndex - 1 + windows.length) % windows.length;
            event.accepted = true;
            return;
        }
        
        if (key === Qt.Key_J) {
            selectedIndex = (selectedIndex + 1) % windows.length;
            event.accepted = true;
            return;
        }
        
        // Select/Activate window
        if ((key === Qt.Key_Return) || (key === Qt.Key_Enter) || (key === Qt.Key_Space)) {
            if (windows.length > 0 && selectedIndex < windows.length) {
                root.windowSelected(windows[selectedIndex]);
            }
            event.accepted = true;
            return;
        }
        
        // Close selected window
        if ((key === Qt.Key_X) || (key === Qt.Key_Delete)) {
            if (windows.length > 0 && selectedIndex < windows.length) {
                root.windowClosed(windows[selectedIndex]);
            }
            event.accepted = true;
            return;
        }
        
        // New instance
        if (key === Qt.Key_N) {
            root.newInstanceRequested();
            event.accepted = true;
            return;
        }
        
        // Cancel/Close picker
        if (key === Qt.Key_Escape) {
            root.cancelled();
            event.accepted = true;
            return;
        }
    }
    
    // ==================== WINDOW PREVIEWS ====================
    
    RowLayout {
        id: windowLayout
        anchors.centerIn: parent
        anchors.margins: 12
        spacing: 12
        
        Repeater {
            model: root.windows.length
            delegate: windowPreviewDelegate
        }
    }
    
    // ==================== WINDOW PREVIEW DELEGATE ====================
    
    Component {
        id: windowPreviewDelegate
        
        RippleButton {
            id: windowButton
            
            required property int index
            
            Layout.preferredWidth: previewContainer.implicitWidth
            Layout.preferredHeight: previewContainer.implicitHeight
            Layout.fillHeight: false
            Layout.fillWidth: false
            
            padding: 0
            
            // Highlight selected window with border
            property bool isSelected: index === root.selectedIndex
            property color borderColor: isSelected ? Appearance.colors.colPrimary : "transparent"
            
            background: Rectangle {
                color: "transparent"
                border.color: windowButton.borderColor
                border.width: isSelected ? 2 : 0
                radius: Appearance.rounding.small
                
                Behavior on border.color {
                    ColorAnimation {
                        duration: 150
                        easing.type: Easing.OutCubic
                    }
                }
                
                Behavior on border.width {
                    NumberAnimation {
                        duration: 150
                        easing.type: Easing.OutCubic
                    }
                }
            }
            
            // Click to select
            onClicked: {
                root.selectedIndex = index;
                root.windowSelected(root.windows[index]);
            }
            
            // Middle click to close
            middleClickAction: () => {
                root.windowClosed(root.windows[index]);
            }
            
            contentItem: ColumnLayout {
                id: previewContainer
                // Size based on screencopy view like the dock does
                implicitWidth: screencopyView.implicitWidth
                implicitHeight: screencopyView.implicitHeight + titleRow.implicitHeight + spacing
                spacing: 4
                
                // Title bar with close button
                RowLayout {
                    id: titleRow
                    Layout.preferredWidth: screencopyView.implicitWidth
                    spacing: 4
                    
                    StyledText {
                        text: root.windows[index]?.title ?? "Unknown"
                        Layout.fillWidth: true
                        Layout.preferredWidth: screencopyView.implicitWidth - closeButton.width - parent.spacing
                        elide: Text.ElideRight
                        color: Appearance.colors.colOnSurface
                        font.pixelSize: Appearance.font.pixelSize.small
                    }
                    
                    // Close button
                    RippleButton {
                        id: closeButton
                        Layout.preferredWidth: 24
                        Layout.preferredHeight: 24
                        padding: 0
                        
                        contentItem: MaterialSymbol {
                            text: "close"
                            font.pixelSize: 18
                            color: Appearance.colors.colOnSurface
                        }
                        
                        onClicked: {
                            root.windowClosed(root.windows[index]);
                        }
                    }
                }
                
                // Live preview (constraintSize maintains aspect ratio)
                ScreencopyView {
                    id: screencopyView
                    captureSource: root.windows[index] ?? null
                    live: true
                    paintCursor: true
                    constraintSize: Qt.size(root.maxPreviewWidth, root.maxPreviewHeight)
                    
                    layer.enabled: true
                    layer.effect: OpacityMask {
                        maskSource: Rectangle {
                            width: screencopyView.width
                            height: screencopyView.height
                            radius: Appearance.rounding.small
                        }
                    }
                }
            }
        }
    }
}
