/**
 * PluginRichCard - Displays a rich, block-based plugin card
 *
 * This is designed for long-running conversational plugins (AI chat), but is
 * generic enough for other "timeline" style outputs: separators, pills (dates),
 * messages, and expandable detail sections (tool calls, thinking, artifacts).
 *
 * Expected card shape (all optional unless noted):
 * {
 *   title: string,
 *   kind: "blocks",
 *   maxHeight: int,
 *   showDetails: bool,
 *   allowToggleDetails: bool,
 *   blocks: [
 *     { type: "pill", text: string },
 *     { type: "separator", text: string },
 *     {
 *       type: "message",
 *       role: "user"|"assistant"|"system",
 *       content: string,
 *       markdown: bool,
 *       timestamp: string,
 *       details: {
 *         thinking: string,
 *         toolCalls: string,
 *         artifacts: [ { title: string, content: string, markdown: bool } ],
 *         raw: string
 *       }
 *     }
 *   ]
 * }
 */
import qs
import qs.modules.common
import qs.modules.common.widgets
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell

Rectangle {
    id: root

    property var card: null

    // When true, show inline loading indicator (keeps transcript visible)
    property bool busy: false


    property string title: card?.title ?? ""
    property var blocks: card?.blocks ?? []

    property bool showDetails: card?.showDetails ?? false
    property bool allowToggleDetails: card?.allowToggleDetails ?? true

    property int maxHeight: card?.maxHeight
        ?? Math.min(900, Math.round((Screen.height ?? 1000) * 0.70))

    visible: card !== null
    Layout.fillWidth: true

    implicitHeight: Math.min(root.maxHeight, cardColumn.implicitHeight + 24)

    color: "transparent"

    function scrollToBottom() {
        if (!scrollView.contentItem) return
        scrollView.contentItem.contentY = Math.max(
            0,
            scrollView.contentItem.contentHeight - scrollView.contentItem.height
        )
    }

    onCardChanged: Qt.callLater(root.scrollToBottom)

    ColumnLayout {
        id: cardColumn
        anchors {
            fill: parent
            margins: 12
            leftMargin: 20
            rightMargin: 20
        }
        spacing: 10

        RowLayout {
            id: headerRow
            Layout.fillWidth: true
            spacing: 10

            StyledText {
                Layout.fillWidth: true
                visible: root.title !== ""
                text: root.title
                font.pixelSize: Appearance.font.pixelSize.normal
                font.weight: Font.DemiBold
                color: Appearance.m3colors.m3onSurface
                wrapMode: Text.Wrap
            }

            RippleButton {
                id: detailsToggle
                visible: root.allowToggleDetails
                implicitHeight: 28
                implicitWidth: toggleRow.implicitWidth + 16
                buttonRadius: Appearance.rounding.verysmall

                colBackground: Appearance.colors.colSecondaryContainer
                colBackgroundHover: Appearance.colors.colSecondaryContainerHover
                colRipple: Appearance.colors.colSecondaryContainerActive

                RowLayout {
                    id: toggleRow
                    anchors.centerIn: parent
                    spacing: 4

                    MaterialSymbol {
                        text: root.showDetails ? "visibility_off" : "visibility"
                        iconSize: Appearance.font.pixelSize.small
                        color: Appearance.m3colors.m3onSecondaryContainer
                    }

                    StyledText {
                        text: root.showDetails ? "Hide details" : "Show details"
                        font.pixelSize: Appearance.font.pixelSize.smaller
                        color: Appearance.m3colors.m3onSecondaryContainer
                    }
                }

                onClicked: root.showDetails = !root.showDetails
            }
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

        Rectangle {
            Layout.fillWidth: true
            visible: (root.title !== "" && root.blocks.length > 0) || root.busy
            height: 1
            color: Appearance.colors.colOutlineVariant
        }

        readonly property int scrollMaxHeight: Math.max(
            180,
            root.maxHeight - headerRow.implicitHeight - footerRow.implicitHeight - 80
        )

        ScrollView {
            id: scrollView
            Layout.fillWidth: true
            clip: true
            implicitHeight: Math.min(cardColumn.scrollMaxHeight, blockColumn.implicitHeight + 4)

            ScrollBar.vertical: StyledScrollBar {
                policy: ScrollBar.AsNeeded
            }
            ScrollBar.horizontal: ScrollBar { policy: ScrollBar.AlwaysOff }

            Column {
                id: blockColumn
                width: scrollView.availableWidth
                spacing: 10

                onHeightChanged: Qt.callLater(root.scrollToBottom)

                Repeater {
                    id: blockRepeater
                    model: root.blocks

                    delegate: Item {
                        // Do NOT name this `modelData` (it shadows the repeater context).
                        property var block: modelData

                        width: blockColumn.width
                        implicitHeight: (blockLoader.item?.implicitHeight ?? 0)
                        height: implicitHeight

                        Loader {
                            id: blockLoader
                            width: parent.width
                            property var blockData: block

                            sourceComponent: {
                                const t = block?.type ?? ""
                                if (t === "pill") return pillBlock
                                if (t === "separator") return separatorBlock
                                if (t === "message") return messageBlock
                                if (t === "note") return noteBlock
                                return unknownBlock
                            }

                            onLoaded: {
                                if (blockLoader.item && blockLoader.item.modelData !== undefined) {
                                    blockLoader.item.modelData = blockLoader.blockData
                                }
                            }
                        }
                    }
                }
            }
        }

        RowLayout {
            id: footerRow
            Layout.fillWidth: true
            Layout.topMargin: 2
            spacing: 8

            Item { Layout.fillWidth: true }

            RippleButton {
                id: copyTranscriptButton
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

                function buildTranscript() {
                    let out = ""
                    for (let i = 0; i < root.blocks.length; i++) {
                        const b = root.blocks[i]
                        if ((b?.type ?? "") !== "message") continue
                        const role = b?.role ?? ""
                        const content = b?.content ?? ""
                        out += `${role}: ${content}\n\n`
                    }
                    return out.trim()
                }

                onClicked: {
                    const transcript = root.card?.transcript ?? ""
                    Quickshell.clipboardText = transcript !== "" ? transcript : copyTranscriptButton.buildTranscript()
                }

                StyledToolTip { text: "Copy to clipboard" }
            }
        }

        Component {
            id: pillBlock
            Item {
                property var modelData: ({})
                width: parent?.width ?? 0
                implicitHeight: pillRect.implicitHeight
                height: implicitHeight

                Rectangle {
                    id: pillRect
                    anchors.horizontalCenter: parent.horizontalCenter
                    height: 24
                    implicitHeight: height
                    implicitWidth: pillRow.implicitWidth + 20
                    width: implicitWidth
                    radius: 999
                    color: Appearance.colors.colSurfaceContainerHigh
                    border.width: 1
                    border.color: Appearance.colors.colOutlineVariant

                    RowLayout {
                        id: pillRow
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 10
                        spacing: 6

                        MaterialSymbol {
                            visible: (modelData?.icon ?? "") !== ""
                            text: modelData?.icon ?? ""
                            iconSize: Appearance.font.pixelSize.small
                            color: Appearance.colors.colSubtext
                        }

                        StyledText {
                            text: modelData?.text ?? ""
                            font.pixelSize: Appearance.font.pixelSize.smaller
                            color: Appearance.colors.colSubtext
                        }
                    }
                }
            }
        }

        Component {
            id: separatorBlock
            Item {
                property var modelData: ({})
                width: parent?.width ?? 0
                implicitHeight: 20
                height: implicitHeight

                RowLayout {
                    anchors.fill: parent
                    spacing: 10

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.alignment: Qt.AlignVCenter
                        height: 1
                        color: Appearance.colors.colOutlineVariant
                    }

                    StyledText {
                        visible: (modelData?.text ?? "") !== ""
                        text: modelData?.text ?? ""
                        font.pixelSize: Appearance.font.pixelSize.smaller
                        color: Appearance.colors.colSubtext
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.alignment: Qt.AlignVCenter
                        height: 1
                        color: Appearance.colors.colOutlineVariant
                    }
                }
            }
        }

        Component {
            id: messageBlock
            Item {
                property var modelData: ({})

                width: parent?.width ?? 0
                implicitHeight: messageContainer.implicitHeight
                height: implicitHeight

                readonly property string role: modelData?.role ?? "assistant"
                readonly property string content: modelData?.content ?? ""
                readonly property bool markdown: modelData?.markdown ?? false
                readonly property string timestamp: modelData?.timestamp ?? ""
                readonly property var details: modelData?.details ?? null

                function roleLabel() {
                    switch(role) {
                        case "user": return "You"
                        case "assistant": return "AI"
                        case "system": return "System"
                        default: return role
                    }
                }

                function bubbleColor() {
                    switch(role) {
                        case "user": return Appearance.colors.colPrimaryContainer
                        case "assistant": return Appearance.colors.colSurfaceContainerHigh
                        case "system": return Appearance.colors.colSurfaceContainerHigh
                        default: return Appearance.colors.colSurfaceContainerHigh
                    }
                }

                function bubbleTextColor() {
                    switch(role) {
                        case "user": return Appearance.m3colors.m3onPrimaryContainer
                        default: return Appearance.m3colors.m3onSurface
                    }
                }

                Rectangle {
                    id: messageContainer
                    width: parent.width
                    implicitWidth: width
                    radius: Appearance.rounding.verysmall
                    color: bubbleColor()

                    implicitHeight: messageColumn.implicitHeight + 20

                    ColumnLayout {
                        id: messageColumn
                        anchors.fill: parent
                        anchors.margins: 10
                        spacing: 8

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            StyledText {
                                text: roleLabel()
                                font.pixelSize: Appearance.font.pixelSize.smaller
                                font.weight: Font.DemiBold
                                color: bubbleTextColor()
                            }

                            Item { Layout.fillWidth: true }

                            Rectangle {
                                visible: timestamp !== ""
                                height: 18
                                radius: 999
                                color: Qt.rgba(0, 0, 0, 0.08)

                                RowLayout {
                                    anchors.fill: parent
                                    anchors.leftMargin: 8
                                    anchors.rightMargin: 8

                                    StyledText {
                                        text: timestamp
                                        font.pixelSize: Appearance.font.pixelSize.smaller
                                        color: bubbleTextColor()
                                    }
                                }
                            }
                        }

                        TextArea {
                            id: contentText
                            Layout.fillWidth: true

                            // Ensure the TextArea contributes height to layouts
                            implicitHeight: Math.max(24, contentHeight)

                            text: content
                            textFormat: markdown ? TextEdit.MarkdownText : TextEdit.PlainText


                            readOnly: true
                            selectByMouse: true
                            wrapMode: TextEdit.Wrap

                            font.family: Appearance.font.family.reading
                            font.pixelSize: markdown ? Appearance.font.pixelSize.smaller : Appearance.font.pixelSize.small
                            font.underline: false
                            color: Appearance.m3colors.m3onSurface


                            background: null
                            padding: 0
                        }
                    }
                }
            }
        }

        Component {
            id: unknownBlock
            Item {
                property var modelData: ({})
                implicitHeight: 24
                height: implicitHeight

                StyledText {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    text: `[Unsupported block: ${modelData?.type ?? "unknown"}]`
                    font.pixelSize: Appearance.font.pixelSize.smaller
                    color: Appearance.colors.colSubtext
                    elide: Text.ElideRight
                }
            }
        }
    }

    component CollapsibleSection: Rectangle {
        id: section

        property string title: ""
        property string content: ""
        property bool markdown: false
        property bool monospace: false
        property bool expanded: false

        Layout.fillWidth: true
        radius: Appearance.rounding.verysmall
        color: Appearance.colors.colSurfaceContainer
        border.width: 1
        border.color: Appearance.colors.colOutlineVariant

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            RippleButton {
                id: header
                Layout.fillWidth: true
                implicitHeight: 32
                buttonRadius: section.radius

                colBackground: "transparent"
                colBackgroundHover: Appearance.colors.colSurfaceContainerHigh
                colRipple: Appearance.colors.colSurfaceContainerHigh

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 10
                    anchors.rightMargin: 10
                    spacing: 8

                    MaterialSymbol {
                        text: section.expanded ? "expand_more" : "chevron_right"
                        iconSize: Appearance.font.pixelSize.small
                        color: Appearance.colors.colSubtext
                    }

                    StyledText {
                        Layout.fillWidth: true
                        text: section.title
                        font.pixelSize: Appearance.font.pixelSize.smaller
                        color: Appearance.colors.colSubtext
                        elide: Text.ElideRight
                    }
                }

                onClicked: section.expanded = !section.expanded
            }

            Rectangle {
                Layout.fillWidth: true
                visible: section.expanded
                height: 1
                color: Appearance.colors.colOutlineVariant
            }

            TextArea {
                Layout.fillWidth: true
                visible: section.expanded

                // Ensure expanded content actually takes space
                implicitHeight: Math.max(40, contentHeight + 20)

                text: section.content
                textFormat: section.markdown ? TextEdit.MarkdownText : TextEdit.PlainText


                readOnly: true
                selectByMouse: true
                wrapMode: TextEdit.Wrap

                font.family: section.monospace ? Appearance.font.family.monospace : Appearance.font.family.reading
                font.pixelSize: Appearance.font.pixelSize.smaller
                font.underline: false
                color: Appearance.m3colors.m3onSurface

                background: null
                padding: 10
            }
        }
    }
}
