import qs
import qs.services
import qs.modules.common
import qs.modules.common.widgets
import qs.modules.common.functions
import QtQuick
import QtQuick.Controls
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Hyprland

Scope {
    id: root

    // ImageBrowser is now workflow-only (triggered by WorkflowRunner with imageBrowser response type)
    readonly property bool isOpen: GlobalStates.imageBrowserOpen
    readonly property var config: GlobalStates.imageBrowserConfig

    Loader {
        id: imageBrowserLoader
        active: root.isOpen

        sourceComponent: PanelWindow {
            id: panelWindow
            readonly property HyprlandMonitor monitor: Hyprland.monitorFor(panelWindow.screen)
            property bool monitorIsFocused: (Hyprland.focusedMonitor?.id == monitor?.id)

            exclusionMode: ExclusionMode.Ignore
            WlrLayershell.namespace: "quickshell:imageBrowser"
            WlrLayershell.layer: WlrLayer.Overlay
            color: "transparent"

            anchors.top: true
            margins {
                // Position down from top like SearchWidget (elevationMargin * 20)
                top: Appearance.sizes.elevationMargin * 20
            }

            mask: Region {
                item: content
            }

            implicitHeight: Appearance.sizes.imageBrowserHeight
            implicitWidth: Appearance.sizes.imageBrowserWidth

            HyprlandFocusGrab {
                id: grab
                windows: [ panelWindow ]
                active: imageBrowserLoader.active
                onCleared: () => {
                    if (!active) GlobalStates.closeImageBrowser();
                }
            }

            ImageBrowserContent {
                id: content
                anchors.fill: parent
                config: root.config
            }
        }
    }

    // Close overview when image browser closes via manual close (Escape/click-outside)
    // If closed by selection, workflow handler decides via executeCommand with close: true
    Connections {
        target: GlobalStates
        function onImageBrowserOpenChanged() {
            if (!GlobalStates.imageBrowserOpen && !GlobalStates.imageBrowserClosedBySelection) {
                // Manual close - close the launcher too
                GlobalStates.launcherOpen = false;
            }
        }
    }

}
