import QtQuick
import QtQuick.Layouts
import Qt5Compat.GraphicalEffects
import Quickshell
import Quickshell.Wayland
import qs.modules.common

Scope {
	id: root
	property bool failed;
	property string errorString;

	// Connect to the Quickshell global to listen for the reload signals.
	Connections {
		target: Quickshell

		function onReloadCompleted() {
			root.failed = false;
			popupLoader.loading = true;
		}

		function onReloadFailed(error: string) {
			// Close any existing popup before making a new one.
			popupLoader.active = false;

			root.failed = true;
			root.errorString = error;
			popupLoader.loading = true;
		}
	}

	// Keep the popup in a loader because it isn't needed most of the time
	LazyLoader {
		id: popupLoader

		PanelWindow {
			id: popup

			exclusiveZone: 0
			anchors.top: true
			margins.top: 10

			implicitWidth: rect.width + shadow.radius * 2
			implicitHeight: rect.height + shadow.radius * 2

			WlrLayershell.namespace: "quickshell:reloadPopup"

			// color blending is a bit odd as detailed in the type reference.
			color: "transparent"

			Rectangle {
				id: rect
				anchors.centerIn: parent
				color: root.failed ? Appearance.m3colors.m3errorContainer : Appearance.m3colors.m3successContainer

				implicitHeight: layout.implicitHeight + 24
				implicitWidth: layout.implicitWidth + 40
				radius: Appearance.rounding.full

				// Fills the whole area of the rectangle, making any clicks go to it,
				// which dismiss the popup.
				MouseArea {
					id: mouseArea
					anchors.fill: parent
					onPressed: {
						popupLoader.active = false
					}

					// makes the mouse area track mouse hovering, so the hide animation
					// can be paused when hovering.
					hoverEnabled: true
				}

				ColumnLayout {
					id: layout
					spacing: 8
					anchors {
						top: parent.top
						topMargin: 12
						horizontalCenter: parent.horizontalCenter
					}

					Text {
						renderType: Text.QtRendering
						font.family: Appearance.font.family.main
						font.pixelSize: Appearance.font.pixelSize.normal
						text: root.failed ? "hamr got hammered" : "hamr time"
						color: root.failed ? Appearance.m3colors.m3onErrorContainer : Appearance.m3colors.m3onSuccessContainer
						Layout.alignment: Qt.AlignHCenter
					}

					Text {
						renderType: Text.QtRendering
						font.family: Appearance.font.family.monospace
						font.pixelSize: Appearance.font.pixelSize.smaller
						text: root.errorString
						color: root.failed ? Appearance.m3colors.m3onErrorContainer : Appearance.m3colors.m3onSuccessContainer
						// When visible is false, it also takes up no space.
						visible: root.errorString != ""
						Layout.alignment: Qt.AlignHCenter
					}
				}

				// A progress bar on the bottom of the screen, showing how long until the
				// popup is removed.
				Rectangle {
					z: 2
					id: bar
					color: root.failed ? Appearance.m3colors.m3error : Appearance.m3colors.m3success
					anchors.bottom: parent.bottom
					anchors.horizontalCenter: parent.horizontalCenter
					anchors.margins: 8
					height: 4
					radius: Appearance.rounding.full

					PropertyAnimation {
						id: anim
						target: bar
						property: "width"
						from: rect.width - 16
						to: 0
						duration: root.failed ? 10000 : 1500
						onFinished: popupLoader.active = false

						// Pause the animation when the mouse is hovering over the popup,
						// so it stays onscreen while reading. This updates reactively
						// when the mouse moves on and off the popup.
						paused: mouseArea.containsMouse
					}
				}
				// Its bg
				Rectangle {
					z: 1
					id: bar_bg
					color: root.failed ? Qt.rgba(Appearance.m3colors.m3error.r, Appearance.m3colors.m3error.g, Appearance.m3colors.m3error.b, 0.3) : Qt.rgba(Appearance.m3colors.m3success.r, Appearance.m3colors.m3success.g, Appearance.m3colors.m3success.b, 0.3)
					anchors.bottom: parent.bottom
					anchors.horizontalCenter: parent.horizontalCenter
					anchors.margins: 8
					height: 4
					radius: Appearance.rounding.full
					width: rect.width - 16
				}

				// We could set `running: true` inside the animation, but the width of the
				// rectangle might not be calculated yet, due to the layout.
				// In the `Component.onCompleted` event handler, all of the component's
				// properties and children have been initialized.
				Component.onCompleted: anim.start()
			}

			DropShadow {
				id: shadow
                anchors.fill: rect
                horizontalOffset: 0
                verticalOffset: 3
                radius: 8
                samples: radius * 2 + 1
                color: Appearance.colors.colShadow
                source: rect
            }
		}
	}
}
