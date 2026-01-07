import QtQuick
import QtQuick.Layouts
import Qt5Compat.GraphicalEffects
import Quickshell
import Quickshell.Wayland
import qs.modules.common
import qs.modules.common.widgets

Scope {
    id: root
    property bool failed
    property string errorString

    Connections {
        target: Quickshell

        function onReloadCompleted() {
            root.failed = false;
            root.errorString = "";
            popupLoader.loading = true;
        }

        function onReloadFailed(error: string) {
            popupLoader.active = false;
            root.failed = true;
            root.errorString = error;
            popupLoader.loading = true;
        }
    }

    LazyLoader {
        id: popupLoader

        PanelWindow {
            id: popup

            exclusiveZone: 0
            anchors.top: true
            margins.top: 10

            implicitWidth: content.implicitWidth + shadow.radius * 2
            implicitHeight: content.implicitHeight + shadow.radius * 2

            WlrLayershell.namespace: "quickshell:reloadPopup"
            WlrLayershell.layer: WlrLayer.Overlay

            color: "transparent"

            Rectangle {
                id: content
                anchors.centerIn: parent
                
                implicitWidth: Math.max(Math.min(layout.implicitWidth + 48, 600), 400)
                implicitHeight: layout.implicitHeight + 32
                radius: Appearance.rounding.small
                color: Appearance.colors.colSurfaceContainer
                border.width: 1
                border.color: root.failed ? Appearance.colors.colError : Appearance.m3colors.m3primary

                MouseArea {
                    id: mouseArea
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: popupLoader.active = false
                }

                ColumnLayout {
                    id: layout
                    anchors.centerIn: parent
                    anchors.margins: 24
                    spacing: 16
                    width: parent.width - 48

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 12

                        MaterialSymbol {
                            text: root.failed ? "error" : "check_circle"
                            iconSize: 24
                            color: root.failed ? Appearance.colors.colError : Appearance.m3colors.m3primary
                        }

                        Text {
                            text: root.failed ? "Reload Failed" : "Reloaded"
                            font.family: Appearance.font.family.main
                            font.pixelSize: Appearance.font.pixelSize.normal
                            font.weight: Font.Medium
                            color: root.failed ? Appearance.colors.colError : Appearance.m3colors.m3primary
                        }
                    }

                    Rectangle {
                        visible: root.errorString !== ""
                        Layout.fillWidth: true
                        Layout.preferredHeight: errorText.implicitHeight + 16
                        Layout.maximumWidth: 450
                        radius: Appearance.rounding.small
                        color: Appearance.colors.colSurfaceContainerHighest

                        Text {
                            id: errorText
                            anchors.fill: parent
                            anchors.margins: 8
                            text: root.errorString
                            font.family: Appearance.font.family.monospace
                            font.pixelSize: Appearance.font.pixelSize.smaller
                            color: Appearance.m3colors.m3onSurface
                            wrapMode: Text.WrapAnywhere
                        }
                    }

                    // Progress bar for auto-dismiss
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 4
                        radius: Appearance.rounding.full
                        color: Qt.rgba(
                            (root.failed ? Appearance.colors.colError : Appearance.m3colors.m3primary).r,
                            (root.failed ? Appearance.colors.colError : Appearance.m3colors.m3primary).g,
                            (root.failed ? Appearance.colors.colError : Appearance.m3colors.m3primary).b,
                            0.3
                        )

                        Rectangle {
                            id: progressBar
                            anchors.left: parent.left
                            anchors.top: parent.top
                            anchors.bottom: parent.bottom
                            radius: Appearance.rounding.full
                            color: root.failed ? Appearance.colors.colError : Appearance.m3colors.m3primary

                            PropertyAnimation {
                                id: dismissAnim
                                target: progressBar
                                property: "width"
                                from: progressBar.parent.width
                                to: 0
                                duration: root.failed ? 10000 : 2000
                                paused: mouseArea.containsMouse
                                onFinished: popupLoader.active = false
                            }
                        }
                    }
                }

                Component.onCompleted: dismissAnim.start()
            }

            DropShadow {
                id: shadow
                anchors.fill: content
                horizontalOffset: 0
                verticalOffset: 3
                radius: 8
                samples: radius * 2 + 1
                color: Appearance.colors.colShadow
                source: content
            }
        }
    }
}
