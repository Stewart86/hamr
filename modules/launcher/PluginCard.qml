/**
 * PluginCard - Displays a card response from a plugin handler
 * 
 * Used for plugins that return rich content (e.g., dictionary definitions,
 * AI responses, documentation lookups) rather than a list of selectable results.
 */
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
    
     // Card data from PluginRunner.pluginCard
     // { title: string, content: string, markdown: bool }
    property var card: null

    // When true, show inline loading indicator (keeps content visible)
    property bool busy: false
    
    property string title: card?.title ?? ""
    property string content: card?.content ?? ""
    property bool markdown: card?.markdown ?? false

    
    visible: card !== null
    
    Layout.fillWidth: true
    implicitHeight: Math.min(400, cardColumn.implicitHeight + 24)
    
    color: "transparent"
    
    ColumnLayout {
        id: cardColumn
        anchors {
            fill: parent
            margins: 12
            leftMargin: 20
            rightMargin: 20
        }
        spacing: 8
        
        // Title
        StyledText {
            id: titleText
            Layout.fillWidth: true
            visible: root.title !== ""
            text: root.title
            font.pixelSize: Appearance.font.pixelSize.normal
            font.weight: Font.DemiBold
            color: Appearance.m3colors.m3onSurface
            wrapMode: Text.Wrap
        }
        
         // Inline plugin busy indicator (keeps card visible)
         RowLayout {
             visible: root.busy
             Layout.fillWidth: true
             spacing: 10

             StyledIndeterminateProgressBar {
                 Layout.fillWidth: true
             }

             StyledText {
                 text: "Processing..."
                 font.pixelSize: Appearance.font.pixelSize.small
                 color: Appearance.colors.colSubtext
             }
         }

        // Separator
        Rectangle {
            Layout.fillWidth: true
            visible: (root.title !== "" && root.content !== "") || root.busy
            height: 1
            color: Appearance.colors.colOutlineVariant
        }
        
        // Content sheet - visual separation for markdown content
        Rectangle {
            id: contentSheet
            Layout.fillWidth: true
            visible: root.content !== ""
            implicitHeight: scrollView.implicitHeight + 24
            radius: Appearance.rounding.verysmall
            color: Appearance.colors.colSurfaceContainerLow
            border.width: 1
            border.color: Appearance.m3colors.m3surfaceContainerLowest
            
            ScrollView {
                id: scrollView
                anchors {
                    fill: parent
                    margins: 12
                }
                
                clip: true
                
                ScrollBar.vertical: StyledScrollBar {
                    policy: ScrollBar.AsNeeded
                }
                ScrollBar.horizontal: ScrollBar {
                    policy: ScrollBar.AlwaysOff
                }
                
                // Content - supports markdown
                TextArea {
                    id: contentText
                    width: scrollView.availableWidth
                    
                    text: root.content
                    textFormat: root.markdown ? TextEdit.MarkdownText : TextEdit.PlainText

                    
                    readOnly: true
                    selectByMouse: true
                    wrapMode: TextEdit.Wrap
                    
                    font.family: Appearance.font.family.reading
                    font.pixelSize: root.markdown ? Appearance.font.pixelSize.smaller : Appearance.font.pixelSize.small
                    font.underline: false
                    color: Appearance.m3colors.m3onSurface
                    selectedTextColor: Appearance.m3colors.m3onSecondaryContainer
                    selectionColor: Appearance.colors.colSecondaryContainer
                    
                    background: null
                    padding: 0
                    leftPadding: 0
                    rightPadding: 0
                    topPadding: 0
                    bottomPadding: 0
                    
                    onLinkActivated: (link) => {
                        Qt.openUrlExternally(link)
                    }
                    
                    MouseArea {
                        anchors.fill: parent
                        acceptedButtons: Qt.NoButton
                        hoverEnabled: true
                        cursorShape: parent.hoveredLink !== "" ? Qt.PointingHandCursor : Qt.IBeamCursor
                    }
                }
            }
        }
        
        // Copy button row
        RowLayout {
            Layout.fillWidth: true
            Layout.topMargin: 4
            spacing: 8
            
            Item { Layout.fillWidth: true }
            
            RippleButton {
                id: copyButton
                implicitHeight: 28
                implicitWidth: copyRow.implicitWidth + 16
                buttonRadius: Appearance.rounding.verysmall
                
                colBackground: Appearance.colors.colSecondaryContainer
                colBackgroundHover: Appearance.colors.colSecondaryContainerHover
                colRipple: Appearance.colors.colSecondaryContainerActive
                
                RowLayout {
                    id: copyRow
                    anchors.centerIn: parent
                    spacing: 4
                    
                    MaterialSymbol {
                        text: "content_copy"
                        iconSize: Appearance.font.pixelSize.small
                        color: Appearance.m3colors.m3onSecondaryContainer
                    }
                    
                    StyledText {
                        text: "Copy"
                        font.pixelSize: Appearance.font.pixelSize.smaller
                        color: Appearance.m3colors.m3onSecondaryContainer
                    }
                }
                
                onClicked: {
                    // Copy raw content (not rendered markdown)
                    Quickshell.clipboardText = root.content
                }
                
                StyledToolTip {
                    text: "Copy to clipboard"
                }
            }
        }
    }
}
