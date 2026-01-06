import QtQuick
import QtQuick.Layouts
import qs.modules.common
import qs.modules.common.widgets
import qs.services

Rectangle {
    id: root
    
    property bool showBrowserKeys: false
    
    // Read action bar hints from config
    readonly property var actionBarHints: Config.actionBarHints ?? []
    
    implicitWidth: content.implicitWidth + 32
    implicitHeight: content.implicitHeight + 24
    radius: Appearance.rounding.small
    color: Appearance.colors.colSurfaceContainer
    border.width: 1
    border.color: Appearance.colors.colOutlineVariant
    
    ColumnLayout {
        id: content
        anchors.centerIn: parent
        spacing: 16
        
        // Navigation - spatial layout
        ColumnLayout {
            spacing: 8
            Layout.alignment: Qt.AlignHCenter
            
            Text {
                text: root.showBrowserKeys ? "Grid Navigation" : "List Navigation"
                font.pixelSize: Appearance.font.pixelSize.smaller
                font.weight: Font.Medium
                color: Appearance.m3colors.m3primary
            }
            
            // Spatial key layout with labels
            Item {
                id: navContainer
                Layout.alignment: Qt.AlignHCenter
                implicitWidth: leftLabel.implicitWidth + 12 + navGrid.implicitWidth + 12 + rightLabel.implicitWidth
                implicitHeight: upLabel.implicitHeight + 8 + navGrid.implicitHeight + 8 + downLabel.implicitHeight
                
                Text {
                    id: upLabel
                    anchors.horizontalCenter: navGrid.horizontalCenter
                    anchors.top: parent.top
                    text: "up"
                    font.pixelSize: Appearance.font.pixelSize.smallest
                    color: Appearance.m3colors.m3outline
                }
                
                Text {
                    id: leftLabel
                    anchors.right: navGrid.left
                    anchors.rightMargin: 12
                    anchors.verticalCenter: navGrid.verticalCenter
                    anchors.verticalCenterOffset: (kbdHeight + 4) / 2
                    text: root.showBrowserKeys ? "left" : "back"
                    font.pixelSize: Appearance.font.pixelSize.smallest
                    color: Appearance.m3colors.m3outline
                }
                
                GridLayout {
                    id: navGrid
                    anchors.centerIn: parent
                    columns: 3
                    rowSpacing: 4
                    columnSpacing: 4
                    
                    // Row 1: Up
                    Item { implicitWidth: kbdWidth; implicitHeight: kbdHeight }
                    Kbd { 
                        id: upKey
                        keys: root.showBrowserKeys ? "K" : "^K"
                        Layout.alignment: Qt.AlignHCenter
                    }
                    Item { implicitWidth: kbdWidth; implicitHeight: kbdHeight }
                    
                    // Row 2: Left/Back, Down, Right/Select
                    Kbd { 
                        keys: root.showBrowserKeys ? "H" : "^H"
                        Layout.alignment: Qt.AlignHCenter
                    }
                    Kbd { 
                        keys: root.showBrowserKeys ? "J" : "^J"
                        Layout.alignment: Qt.AlignHCenter
                    }
                    Kbd { 
                        keys: root.showBrowserKeys ? "L" : "^L"
                        Layout.alignment: Qt.AlignHCenter
                    }
                }
                
                Text {
                    id: rightLabel
                    anchors.left: navGrid.right
                    anchors.leftMargin: 12
                    anchors.verticalCenter: navGrid.verticalCenter
                    anchors.verticalCenterOffset: (kbdHeight + 4) / 2
                    text: root.showBrowserKeys ? "right" : "select"
                    font.pixelSize: Appearance.font.pixelSize.smallest
                    color: Appearance.m3colors.m3outline
                }
                
                Text {
                    id: downLabel
                    anchors.horizontalCenter: navGrid.horizontalCenter
                    anchors.bottom: parent.bottom
                    text: "down"
                    font.pixelSize: Appearance.font.pixelSize.smallest
                    color: Appearance.m3colors.m3outline
                }
            }
        }
        
        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: Appearance.colors.colOutlineVariant
        }
        
        // Actions
        ColumnLayout {
            spacing: 8
            
            Text {
                text: "Actions"
                font.pixelSize: Appearance.font.pixelSize.smaller
                font.weight: Font.Medium
                color: Appearance.m3colors.m3primary
            }
            
            GridLayout {
                columns: 2
                rowSpacing: 6
                columnSpacing: 20
                
                KeybindingRow { keys: "Enter"; label: "confirm" }
                KeybindingRow { keys: "Tab"; label: "cycle actions" }
                KeybindingRow { keys: "Bksp"; label: "go back" }
                KeybindingRow { keys: "^Bksp"; label: "exit plugin" }
                KeybindingRow { keys: "^UIOP"; label: "item actions 1-4" }
                KeybindingRow { keys: "^1-6"; label: "plugin actions" }
                KeybindingRow { keys: "^⇧HL"; label: "slider -/+" }
                KeybindingRow { keys: "^⇧T"; label: "toggle switch" }
            }
        }
        
        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: Appearance.colors.colOutlineVariant
        }
        
        // Prefixes - dynamically loaded from config
        ColumnLayout {
            spacing: 8
            visible: root.actionBarHints.length > 0
            
            Text {
                text: "Quick Prefixes"
                font.pixelSize: Appearance.font.pixelSize.smaller
                font.weight: Font.Medium
                color: Appearance.m3colors.m3primary
            }
            
            GridLayout {
                columns: 2
                rowSpacing: 6
                columnSpacing: 20
                
                Repeater {
                    model: root.actionBarHints
                    
                    delegate: KeybindingRow {
                        required property var modelData
                        keys: modelData.prefix ?? ""
                        label: modelData.label ?? ""
                    }
                }
            }
        }
        
        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: Appearance.colors.colOutlineVariant
        }
        
        // Window
        ColumnLayout {
            spacing: 8
            
            Text {
                text: "Window"
                font.pixelSize: Appearance.font.pixelSize.smaller
                font.weight: Font.Medium
                color: Appearance.m3colors.m3primary
            }
            
            GridLayout {
                columns: 2
                rowSpacing: 6
                columnSpacing: 20
                
                KeybindingRow { keys: "Esc"; label: "close" }
                KeybindingRow { keys: "^M"; label: "minimize" }
            }
        }
    }
    
    // Standard kbd dimensions for layout calculations
    readonly property real kbdWidth: 32
    readonly property real kbdHeight: 22
    
    component KeybindingRow: RowLayout {
        property alias keys: kbd.keys
        property alias label: labelText.text
        spacing: 8
        
        Kbd {
            id: kbd
            Layout.preferredWidth: root.kbdWidth
        }
        
        Text {
            id: labelText
            font.pixelSize: Appearance.font.pixelSize.smaller
            color: Appearance.m3colors.m3onSurfaceVariant
        }
    }
}
