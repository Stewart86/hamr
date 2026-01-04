pragma ComponentBehavior: Bound
import QtQuick
import Qt5Compat.GraphicalEffects
import qs.modules.common
import qs.modules.common.widgets

Rectangle {
    id: root

    // Text (1-3 chars, like initials "JD", "A", "OK")
    property string text: ""
    // Image source (avatar, icon image)
    property string image: ""
    // Material icon name
    property string icon: ""
    // Icon/text color (background is always theme default)
    property color textColor: Appearance.m3colors.m3onSurface

    readonly property int badgeSize: 20
    readonly property bool hasImage: root.image !== "" && avatarImage.status === Image.Ready
    readonly property bool hasIcon: root.icon !== "" && root.image === ""
    readonly property bool hasText: root.text !== "" && root.image === "" && root.icon === ""

    implicitWidth: badgeSize
    implicitHeight: badgeSize
    width: badgeSize
    height: badgeSize

    radius: badgeSize / 2
    color: root.hasImage ? "transparent" : Appearance.colors.colSurfaceContainerHighest
    border.width: 1
    border.color: Appearance.colors.colOutline

    Image {
        id: avatarImage
        anchors.fill: parent
        anchors.margins: 1
        visible: root.image !== ""
        source: {
            if (!root.image) return "";
            if (root.image.startsWith("file://")) return root.image;
            if (root.image.startsWith("/")) return "file://" + root.image;
            return root.image;
        }
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        
        layer.enabled: status === Image.Ready
        layer.effect: OpacityMask {
            maskSource: Rectangle {
                width: avatarImage.width
                height: avatarImage.height
                radius: width / 2
            }
        }
    }

    MaterialSymbol {
        visible: root.hasIcon
        anchors.centerIn: parent
        anchors.verticalCenterOffset: 0.5
        text: root.icon
        iconSize: 12
        color: root.textColor
    }

    Text {
        visible: root.hasText
        anchors.fill: parent
        anchors.margins: 2
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        text: root.text.substring(0, 3).toUpperCase()
        color: root.textColor
        font {
            family: Appearance.font.family.monospace
            pixelSize: root.text.length > 2 ? 7 : (root.text.length > 1 ? 8 : 9)
            weight: Font.Bold
        }
    }
}
