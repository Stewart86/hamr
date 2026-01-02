pragma ComponentBehavior: Bound
import qs.modules.common
import QtQuick
import QtQuick.Shapes

Item {
    id: root

    property real value: 0
    property real min: 0
    property real max: 100
    property string label: ""
    property int size: 40

    implicitWidth: size
    implicitHeight: size

    readonly property real ratio: Math.max(0, Math.min(1, (value - min) / (max - min)))
    readonly property real arcStrokeWidth: 4
    readonly property real centerX: width / 2
    readonly property real centerY: height / 2
    readonly property real radius: (width - arcStrokeWidth) / 2
    readonly property real startAngle: 135
    readonly property real sweepAngle: 270

    Shape {
        anchors.fill: parent

        ShapePath {
            id: backgroundPath
            fillColor: "transparent"
            strokeColor: Appearance.colors.colSurfaceContainerHigh
            strokeWidth: arcStrokeWidth
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            startX: centerX + radius * Math.cos(startAngle * Math.PI / 180)
            startY: centerY + radius * Math.sin(startAngle * Math.PI / 180)

            PathArc {
                x: centerX + radius * Math.cos((startAngle + sweepAngle) * Math.PI / 180)
                y: centerY + radius * Math.sin((startAngle + sweepAngle) * Math.PI / 180)
                radiusX: radius
                radiusY: radius
                xAxisRotation: 0
                useLargeArc: true
                direction: PathArc.Clockwise
            }
        }

        ShapePath {
            id: foregroundPath
            fillColor: "transparent"
            strokeColor: root.ratio > 0 ? Appearance.colors.colPrimary : "transparent"
            strokeWidth: arcStrokeWidth
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            startX: centerX + radius * Math.cos(startAngle * Math.PI / 180)
            startY: centerY + radius * Math.sin(startAngle * Math.PI / 180)

            PathArc {
                x: centerX + radius * Math.cos((startAngle + sweepAngle * Math.max(0.001, root.ratio)) * Math.PI / 180)
                y: centerY + radius * Math.sin((startAngle + sweepAngle * Math.max(0.001, root.ratio)) * Math.PI / 180)
                radiusX: radius
                radiusY: radius
                xAxisRotation: 0
                useLargeArc: sweepAngle * root.ratio > 180
                direction: PathArc.Clockwise
            }
        }
    }

    Text {
        visible: root.label !== ""
        anchors.centerIn: parent
        text: root.label
        color: Appearance.m3colors.m3onSurface
        font {
            family: Appearance.font.family.main
            pixelSize: Appearance.font.pixelSize.smaller
        }
    }
}
