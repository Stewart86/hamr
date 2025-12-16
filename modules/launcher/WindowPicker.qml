import qs
import qs.services
import qs.modules.common
import qs.modules.common.widgets
import QtQuick
import Quickshell
import Quickshell.Wayland
import Quickshell.Hyprland

Scope {
    id: root

    readonly property bool isOpen: GlobalStates.windowPickerOpen
    readonly property string appId: GlobalStates.windowPickerAppId
    readonly property var windows: GlobalStates.windowPickerWindows

    Loader {
        id: windowPickerLoader
        active: root.isOpen

        sourceComponent: PanelWindow {
            id: panelWindow
            readonly property HyprlandMonitor monitor: Hyprland.monitorFor(panelWindow.screen)
            property bool monitorIsFocused: (Hyprland.focusedMonitor?.id == monitor?.id)

            exclusionMode: ExclusionMode.Ignore
            WlrLayershell.namespace: "quickshell:windowPicker"
            WlrLayershell.layer: WlrLayer.Overlay
            color: "transparent"

            anchors.top: true
            margins {
                // Position below search bar area
                top: Appearance.sizes.elevationMargin * 25
            }

            mask: Region {
                item: content
            }

            implicitHeight: content.implicitHeight + Appearance.sizes.elevationMargin * 2
            implicitWidth: content.implicitWidth + Appearance.sizes.elevationMargin * 2

            HyprlandFocusGrab {
                id: grab
                windows: [panelWindow]
                active: windowPickerLoader.active
                onCleared: () => {
                    if (!active) GlobalStates.closeWindowPicker();
                }
            }

            // Shadow
            StyledRectangularShadow {
                target: content
            }

            WindowPickerContent {
                id: content
                anchors.centerIn: parent
                windows: root.windows
                focus: true

                onWindowSelected: toplevel => {
                    // Record window focus in history
                    const entry = DesktopEntries.byId(root.appId) ?? DesktopEntries.heuristicLookup(root.appId);
                    const appName = entry?.name ?? root.appId;
                    const iconName = entry?.icon ?? IconResolver.guessIcon(root.appId);
                    LauncherSearch.recordWindowFocus(root.appId, appName, toplevel.title, iconName);
                    
                    WindowManager.focusWindow(toplevel);
                    GlobalStates.closeWindowPicker();
                    GlobalStates.launcherOpen = false;
                }

                onWindowClosed: toplevel => {
                    WindowManager.closeWindow(toplevel);
                    // Update windows list - need to refresh from GlobalStates
                    const remaining = root.windows.filter(w => w !== toplevel);
                    if (remaining.length === 1) {
                        // Auto-focus last window
                        WindowManager.focusWindow(remaining[0]);
                        GlobalStates.closeWindowPicker();
                        GlobalStates.launcherOpen = false;
                    } else if (remaining.length === 0) {
                        GlobalStates.closeWindowPicker();
                    } else {
                        // Update the list
                        GlobalStates.windowPickerWindows = remaining;
                    }
                }

                onCancelled: {
                    GlobalStates.closeWindowPicker();
                }

                onNewInstanceRequested: {
                    LauncherSearch.launchNewInstance(root.appId);
                    GlobalStates.closeWindowPicker();
                    GlobalStates.launcherOpen = false;
                }
            }
        }
    }

    // Close launcher when window picker closes (unless selecting a window)
    Connections {
        target: GlobalStates
        function onWindowPickerOpenChanged() {
            if (!GlobalStates.windowPickerOpen) {
                // Picker closed - return focus to launcher if still open
            }
        }
    }
}
