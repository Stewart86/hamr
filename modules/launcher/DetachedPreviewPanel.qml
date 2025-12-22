import qs
import qs.services
import qs.modules.common
import qs.modules.common.widgets
import qs.modules.common.functions
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: root
    
    required property var previewData
    required property real initialX
    required property real initialY
    
    property var preview: previewData?.preview ?? null
    property string previewType: preview?.type ?? ""
    property string previewContent: preview?.content ?? ""
    property string previewTitle: preview?.title ?? previewData?.name ?? ""
    property var previewMetadata: preview?.metadata ?? []
    
    readonly property real panelWidth: Appearance.sizes.searchWidth * 0.75
    readonly property real panelHeight: Math.min(500, contentColumn.implicitHeight + 48)
    
    WlrLayershell.namespace: "quickshell:hamr-preview"
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"
    
    mask: Region {
        item: panelContainer
    }
    
    anchors {
        top: true
        bottom: true
        left: true
        right: true
    }
    
    Item {
        id: panelContainer
        
        property bool isDragging: false
        
        // Saved position (updated on drag end, used in binding)
        property real posX: root.initialX - root.panelWidth / 2
        property real posY: root.initialY - 20
        
        // Use same pattern as FAB: gate binding with isDragging
        x: isDragging ? x : posX
        y: isDragging ? y : posY
        
        implicitWidth: panelContent.implicitWidth + Appearance.sizes.elevationMargin * 2
        implicitHeight: panelContent.implicitHeight + Appearance.sizes.elevationMargin * 2
        
        StyledRectangularShadow {
            target: panelContent
        }
        
        Rectangle {
            id: panelContent
            anchors.centerIn: parent
            
            implicitWidth: root.panelWidth
            implicitHeight: root.panelHeight
            
            radius: Appearance.rounding.normal
            color: Appearance.colors.colBackgroundSurfaceContainer
            border.width: 1
            border.color: Appearance.colors.colOutlineVariant
            
            ColumnLayout {
                id: contentColumn
                anchors {
                    fill: parent
                    margins: 12
                }
                spacing: 8
                
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    
                    Item {
                        implicitWidth: 24
                        implicitHeight: 24
                        
                        Rectangle {
                            anchors.fill: parent
                            radius: Appearance.rounding.verysmall
                            color: closeButton.containsMouse 
                                ? Appearance.colors.colErrorContainer 
                                : "transparent"
                            
                            MaterialSymbol {
                                anchors.centerIn: parent
                                text: "close"
                                iconSize: Appearance.font.pixelSize.small
                                color: closeButton.containsMouse 
                                    ? Appearance.m3colors.m3onErrorContainer 
                                    : Appearance.m3colors.m3outline
                            }
                        }
                        
                        MouseArea {
                            id: closeButton
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            
                            onClicked: {
                                GlobalStates.closeDetachedPreview(root.previewData.id);
                            }
                        }
                        
                        StyledToolTip {
                            visible: closeButton.containsMouse
                            text: "Close"
                        }
                    }
                    
                    StyledText {
                        Layout.fillWidth: true
                        text: root.previewTitle
                        font.pixelSize: Appearance.font.pixelSize.normal
                        font.weight: Font.DemiBold
                        color: Appearance.m3colors.m3onSurface
                        elide: Text.ElideRight
                        maximumLineCount: 1
                    }
                    
                    Item {
                        implicitWidth: 24
                        implicitHeight: 24
                        
                        Rectangle {
                            anchors.fill: parent
                            radius: Appearance.rounding.verysmall
                            color: headerDragArea.containsMouse || headerDragArea.pressed 
                                ? Appearance.colors.colSurfaceContainerHighest 
                                : "transparent"
                            
                            MaterialSymbol {
                                anchors.centerIn: parent
                                text: "drag_indicator"
                                iconSize: Appearance.font.pixelSize.small
                                color: headerDragArea.containsMouse 
                                    ? Appearance.m3colors.m3onSurface 
                                    : Appearance.m3colors.m3outline
                            }
                        }
                        
                        MouseArea {
                            id: headerDragArea
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor
                            
                            property real dragOffsetX: 0
                            property real dragOffsetY: 0
                            
                            onPressed: mouse => {
                                panelContainer.isDragging = true;
                                const containerPos = mapToItem(panelContainer.parent, mouse.x, mouse.y);
                                dragOffsetX = containerPos.x - panelContainer.x;
                                dragOffsetY = containerPos.y - panelContainer.y;
                            }
                            
                            onPositionChanged: mouse => {
                                if (pressed) {
                                    const containerPos = mapToItem(panelContainer.parent, mouse.x, mouse.y);
                                    let newX = containerPos.x - dragOffsetX;
                                    let newY = containerPos.y - dragOffsetY;
                                    
                                    const screenW = root.width;
                                    const screenH = root.height;
                                    const margin = Appearance.sizes.elevationMargin;
                                    newX = Math.max(-margin, Math.min(newX, screenW - panelContainer.width + margin));
                                    newY = Math.max(-margin, Math.min(newY, screenH - panelContainer.height + margin));
                                    
                                    // Directly update position during drag
                                    panelContainer.x = newX;
                                    panelContainer.y = newY;
                                }
                            }
                            
                            onReleased: {
                                // Save final position so binding uses it when isDragging becomes false
                                panelContainer.posX = panelContainer.x;
                                panelContainer.posY = panelContainer.y;
                                panelContainer.isDragging = false;
                            }
                        }
                    }
                }
                
                Rectangle {
                    Layout.fillWidth: true
                    height: 1
                    color: Appearance.colors.colOutlineVariant
                }
                
                Loader {
                    id: contentLoader
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    
                    sourceComponent: {
                        switch (root.previewType) {
                            case "image": return imagePreview;
                            case "markdown": return markdownPreview;
                            case "text": return textPreview;
                            case "metadata": return metadataPreview;
                            default: return null;
                        }
                    }
                }
                
                Component {
                    id: imagePreview
                    
                    ColumnLayout {
                        spacing: 8
                        
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: {
                                if (imageItem.status !== Image.Ready) return 200;
                                const aspectRatio = imageItem.sourceSize.height / Math.max(1, imageItem.sourceSize.width);
                                const availableWidth = root.panelWidth - 24 - 8;
                                const calculatedHeight = availableWidth * aspectRatio;
                                return Math.max(100, Math.min(350, calculatedHeight));
                            }
                            radius: Appearance.rounding.verysmall
                            color: Appearance.colors.colSurfaceContainerLow
                            clip: true
                            
                            Image {
                                id: imageItem
                                anchors.fill: parent
                                anchors.margins: 4
                                source: root.previewContent ? "file://" + root.previewContent : ""
                                fillMode: Image.PreserveAspectFit
                                asynchronous: true
                                
                                BusyIndicator {
                                    anchors.centerIn: parent
                                    running: imageItem.status === Image.Loading
                                    visible: running
                                }
                            }
                        }
                        
                        ColumnLayout {
                            id: metadataCol
                            Layout.fillWidth: true
                            spacing: 4
                            visible: root.previewMetadata.length > 0
                            
                            Repeater {
                                model: root.previewMetadata
                                
                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 8
                                    
                                    StyledText {
                                        text: modelData.label + ":"
                                        font.pixelSize: Appearance.font.pixelSize.smaller
                                        color: Appearance.m3colors.m3outline
                                    }
                                    
                                    StyledText {
                                        Layout.fillWidth: true
                                        text: modelData.value
                                        font.pixelSize: Appearance.font.pixelSize.smaller
                                        color: Appearance.m3colors.m3onSurfaceVariant
                                        elide: Text.ElideRight
                                    }
                                }
                            }
                        }
                    }
                }
                
                Component {
                    id: markdownPreview
                    
                    ScrollView {
                        clip: true
                        
                        ScrollBar.vertical: StyledScrollBar {
                            policy: ScrollBar.AsNeeded
                        }
                        ScrollBar.horizontal: ScrollBar {
                            policy: ScrollBar.AlwaysOff
                        }
                        
                        TextArea {
                            width: parent.availableWidth
                            text: root.previewContent
                            textFormat: TextEdit.MarkdownText
                            readOnly: true
                            selectByMouse: true
                            wrapMode: TextEdit.Wrap
                            
                            font.family: Appearance.font.family.reading
                            font.pixelSize: Appearance.font.pixelSize.smaller
                            color: Appearance.m3colors.m3onSurface
                            selectedTextColor: Appearance.m3colors.m3onSecondaryContainer
                            selectionColor: Appearance.colors.colSecondaryContainer
                            
                            background: null
                            padding: 0
                            
                            onLinkActivated: link => Qt.openUrlExternally(link)
                            
                            MouseArea {
                                anchors.fill: parent
                                acceptedButtons: Qt.NoButton
                                hoverEnabled: true
                                cursorShape: parent.hoveredLink !== "" ? Qt.PointingHandCursor : Qt.IBeamCursor
                            }
                        }
                    }
                }
                
                Component {
                    id: textPreview
                    
                    ScrollView {
                        clip: true
                        
                        ScrollBar.vertical: StyledScrollBar {
                            policy: ScrollBar.AsNeeded
                        }
                        ScrollBar.horizontal: ScrollBar {
                            policy: ScrollBar.AlwaysOff
                        }
                        
                        TextArea {
                            width: parent.availableWidth
                            text: root.previewContent
                            textFormat: TextEdit.PlainText
                            readOnly: true
                            selectByMouse: true
                            wrapMode: TextEdit.Wrap
                            
                            font.family: Appearance.font.family.monospace
                            font.pixelSize: Appearance.font.pixelSize.smaller
                            color: Appearance.m3colors.m3onSurface
                            selectedTextColor: Appearance.m3colors.m3onSecondaryContainer
                            selectionColor: Appearance.colors.colSecondaryContainer
                            
                            background: null
                            padding: 0
                        }
                    }
                }
                
                Component {
                    id: metadataPreview
                    
                    ScrollView {
                        clip: true
                        
                        ScrollBar.vertical: StyledScrollBar {
                            policy: ScrollBar.AsNeeded
                        }
                        
                        ColumnLayout {
                            width: parent.availableWidth
                            spacing: 6
                            
                            Repeater {
                                model: root.previewMetadata
                                
                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 12
                                    
                                    StyledText {
                                        Layout.preferredWidth: 100
                                        text: modelData.label
                                        font.pixelSize: Appearance.font.pixelSize.small
                                        font.weight: Font.Medium
                                        color: Appearance.m3colors.m3outline
                                        horizontalAlignment: Text.AlignRight
                                    }
                                    
                                    StyledText {
                                        Layout.fillWidth: true
                                        text: modelData.value
                                        font.pixelSize: Appearance.font.pixelSize.small
                                        color: Appearance.m3colors.m3onSurface
                                        wrapMode: Text.Wrap
                                    }
                                }
                            }
                        }
                    }
                }
                
            }
        }
    }
}
