import qs
import qs.services
import qs.modules.common
import qs.modules.common.widgets
import qs.modules.common.functions
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell

Rectangle {
    id: root
    
    required property var item
    
    signal detachRequested(real globalX, real globalY)
    
    property var preview: item?.preview ?? null
    property string previewType: preview?.type ?? ""
    property string previewContent: preview?.content ?? ""
    property string previewTitle: preview?.title ?? item?.name ?? ""
    property var previewMetadata: preview?.metadata ?? []
    property var previewActions: preview?.actions ?? []
    property bool detachable: preview?.detachable ?? true
    
    readonly property real panelWidth: Appearance.sizes.searchWidth * 0.75
    
    visible: preview !== null
    opacity: visible ? 1 : 0
    
    implicitWidth: panelWidth
    implicitHeight: contentColumn.implicitHeight + 24
    
    radius: Appearance.rounding.normal
    color: Appearance.colors.colBackgroundSurfaceContainer
    border.width: 1
    border.color: Appearance.colors.colOutlineVariant
    
    Behavior on opacity {
        NumberAnimation {
            duration: 200
            easing.type: Easing.OutCubic
        }
    }
    
    ColumnLayout {
        id: contentColumn
        anchors {
            fill: parent
            topMargin: 12
            bottomMargin: 12
            leftMargin: 20
            rightMargin: 12
        }
        spacing: 8
        
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            
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
                visible: root.detachable
                implicitWidth: 24
                implicitHeight: 24
                
                Rectangle {
                    anchors.fill: parent
                    radius: Appearance.rounding.verysmall
                    color: detachButton.containsMouse || detachButton.pressed 
                        ? Appearance.colors.colSurfaceContainerHighest 
                        : "transparent"
                    
                    MaterialSymbol {
                        anchors.centerIn: parent
                        text: "push_pin"
                        iconSize: Appearance.font.pixelSize.small
                        color: detachButton.containsMouse 
                            ? Appearance.m3colors.m3onSurface 
                            : Appearance.m3colors.m3outline
                    }
                }
                
                MouseArea {
                    id: detachButton
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    
                    onClicked: mouse => {
                        const globalPos = root.mapToGlobal(root.width / 2, 0);
                        root.detachRequested(globalPos.x, globalPos.y);
                    }
                }
                
                StyledToolTip {
                    visible: detachButton.containsMouse && !detachButton.pressed
                    text: "Pin to screen"
                }
            }
        }
        
        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: Appearance.colors.colOutlineVariant
            visible: root.previewTitle !== ""
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
        
        RowLayout {
            Layout.fillWidth: true
            Layout.topMargin: 8
            spacing: 8
            visible: root.previewActions.length > 0
            
            Item { Layout.fillWidth: true }
            
            Repeater {
                model: root.previewActions
                
                RippleButton {
                    implicitHeight: 28
                    implicitWidth: actionRow.implicitWidth + 16
                    buttonRadius: Appearance.rounding.verysmall
                    
                    colBackground: index === 0 
                        ? Appearance.colors.colPrimaryContainer 
                        : Appearance.colors.colSecondaryContainer
                    colBackgroundHover: index === 0 
                        ? Appearance.colors.colPrimaryContainerHover 
                        : Appearance.colors.colSecondaryContainerHover
                    colRipple: index === 0 
                        ? Appearance.colors.colPrimaryContainerActive 
                        : Appearance.colors.colSecondaryContainerActive
                    
                    RowLayout {
                        id: actionRow
                        anchors.centerIn: parent
                        spacing: 4
                        
                        MaterialSymbol {
                            visible: modelData.icon
                            text: modelData.icon ?? ""
                            iconSize: Appearance.font.pixelSize.small
                            color: index === 0 
                                ? Appearance.m3colors.m3onPrimaryContainer 
                                : Appearance.m3colors.m3onSecondaryContainer
                        }
                        
                        StyledText {
                            text: modelData.name ?? ""
                            font.pixelSize: Appearance.font.pixelSize.smaller
                            color: index === 0 
                                ? Appearance.m3colors.m3onPrimaryContainer 
                                : Appearance.m3colors.m3onSecondaryContainer
                        }
                    }
                    
                    onClicked: {
                        if (root.item && modelData.id) {
                            LauncherSearch.executePreviewAction(root.item, modelData.id);
                        }
                    }
                }
            }
        }
    }
}
