pragma ComponentBehavior: Bound
import qs.modules.common
import QtQuick

Item {
    id: root

    property real value: 0
    property real min: 0
    property real max: 100
    property real step: 1
    property string displayValue: ""
    property string unit: ""  // Unit of measurement (e.g., "%", "px", "ms")
    property bool showValue: displayValue !== "" || unit !== ""

    signal valueCommitted(real newValue)

    implicitWidth: 300
    implicitHeight: 52

    readonly property real clampedValue: Math.max(min, Math.min(max, value))
    
    // Current visual value (tracks drag position or committed value)
    readonly property real currentValue: {
        if (track.width > 0 && (dragArea.pressed || dragArea.containsMouse)) {
            const thumbCenter = thumb.x + thumb.width / 2
            const ratio = Math.max(0, Math.min(1, thumbCenter / track.width))
            const raw = min + ratio * (max - min)
            return Math.round(raw / step) * step
        }
        return clampedValue
    }
    
    // Live display during drag
    readonly property string liveDisplayValue: {
        if (dragArea.pressed) {
            return formatValue(currentValue)
        }
        return displayValue
    }
    
    function commitValue(newValue) {
        const clamped = Math.max(min, Math.min(max, newValue))
        const rounded = Math.round(clamped / step) * step
        root.value = rounded
        root.valueCommitted(rounded)
    }
    
    // Format a value for display based on step precision
    function formatValue(val) {
        // Determine decimal places from step
        const stepStr = String(step)
        const decimalIdx = stepStr.indexOf('.')
        const decimals = decimalIdx >= 0 ? stepStr.length - decimalIdx - 1 : 0
        return val.toFixed(decimals)
    }
    
    function valueToX(val) {
        if (track.width <= 0) return 0
        const range = max - min
        if (range <= 0) return 0
        const ratio = (Math.max(min, Math.min(max, val)) - min) / range
        return ratio * track.width - thumb.width / 2
    }

    Row {
        anchors.fill: parent
        spacing: 8

        Rectangle {
            id: decrementButton
            width: 24
            height: 24
            radius: 4
            color: decrementMouse.containsMouse ? Appearance.colors.colSurfaceContainerHighest : Appearance.colors.colSurfaceContainerHigh
            anchors.verticalCenter: parent.verticalCenter

            Text {
                anchors.centerIn: parent
                text: "−"
                color: Appearance.m3colors.m3onSurface
                font {
                    family: Appearance.font.family.main
                    pixelSize: 18
                }
            }

            MouseArea {
                id: decrementMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.commitValue(root.clampedValue - root.step)
            }
        }

        Item {
            id: trackContainer
            width: parent.width - decrementButton.width - incrementButton.width - parent.spacing * 2
            height: parent.height

            Rectangle {
                id: track
                width: parent.width
                height: 4
                radius: 2
                color: Appearance.colors.colSurfaceContainerHigh
                anchors.verticalCenter: parent.verticalCenter

                Rectangle {
                    id: filledTrack
                    height: parent.height
                    width: Math.max(0, thumb.x + thumb.width / 2)
                    radius: parent.radius
                    color: Appearance.colors.colPrimary
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: (event) => {
                        if (track.width <= 0) return
                        const ratio = Math.max(0, Math.min(1, event.x / track.width))
                        const newValue = root.min + ratio * (root.max - root.min)
                        root.commitValue(newValue)
                    }
                }
            }

            Rectangle {
                id: thumb
                width: 16
                height: 16
                radius: 8
                color: Appearance.colors.colPrimary
                y: track.y + track.height / 2 - height / 2
                
                // Only set x from value when not dragging
                x: root.valueToX(root.clampedValue)
                
                Behavior on x {
                    enabled: !dragArea.pressed
                    NumberAnimation { duration: 100; easing.type: Easing.OutCubic }
                }

                MouseArea {
                    id: dragArea
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    preventStealing: true
                    hoverEnabled: true
                    
                    property real dragStartThumbX: 0
                    property real dragStartMouseX: 0
                    
                    onPressed: (event) => {
                        const globalPos = mapToItem(track, event.x, event.y)
                        dragStartThumbX = thumb.x
                        dragStartMouseX = globalPos.x
                    }
                    
                    onPositionChanged: (event) => {
                        if (!pressed) return
                        const globalPos = mapToItem(track, event.x, event.y)
                        const delta = globalPos.x - dragStartMouseX
                        const newX = dragStartThumbX + delta
                        const clampedX = Math.max(-thumb.width / 2, Math.min(track.width - thumb.width / 2, newX))
                        thumb.x = clampedX
                    }
                    
                    onReleased: {
                        if (track.width <= 0) return
                        const thumbCenter = thumb.x + thumb.width / 2
                        const ratio = Math.max(0, Math.min(1, thumbCenter / track.width))
                        const newValue = root.min + ratio * (root.max - root.min)
                        root.commitValue(newValue)
                    }
                }
            }

            Text {
                visible: root.showValue || dragArea.pressed
                text: {
                    const val = dragArea.pressed ? root.formatValue(root.currentValue) : (root.displayValue || root.formatValue(root.clampedValue))
                    return root.unit ? val + root.unit : val
                }
                anchors {
                    top: thumb.bottom
                    topMargin: 4
                    horizontalCenter: thumb.horizontalCenter
                }
                color: Appearance.colors.colSubtext
                font {
                    family: Appearance.font.family.main
                    pixelSize: Appearance.font.pixelSize.smallest
                }
            }
        }

        Rectangle {
            id: incrementButton
            width: 24
            height: 24
            radius: 4
            color: incrementMouse.containsMouse ? Appearance.colors.colSurfaceContainerHighest : Appearance.colors.colSurfaceContainerHigh
            anchors.verticalCenter: parent.verticalCenter

            Text {
                anchors.centerIn: parent
                text: "+"
                color: Appearance.m3colors.m3onSurface
                font {
                    family: Appearance.font.family.main
                    pixelSize: 18
                }
            }

            MouseArea {
                id: incrementMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.commitValue(root.clampedValue + root.step)
            }
        }
    }
}
