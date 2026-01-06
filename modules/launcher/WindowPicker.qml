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
    readonly property string itemId: GlobalStates.windowPickerItemId
    readonly property var windows: GlobalStates.windowPickerWindows

    Loader {
        id: windowPickerLoader
        active: root.isOpen

        sourceComponent: PanelWindow {
            id: panelWindow
            readonly property var hyprlandMonitor: Hyprland.monitorFor(panelWindow.screen)
            readonly property bool monitorIsFocused: CompositorService.isHyprland
                ? (Hyprland.focusedMonitor?.id === hyprlandMonitor?.id)
                : (panelWindow.screen.name === CompositorService.focusedScreenName)

            exclusionMode: ExclusionMode.Ignore
            WlrLayershell.namespace: "quickshell:windowPicker"
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.OnDemand
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

            FocusGrab {
                id: grab
                window: panelWindow
                active: windowPickerLoader.active
                closeOnCleared: true
                onCloseRequested: GlobalStates.closeWindowPicker()
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
                    // Record execution for frecency (use itemId which is the full path)
                    if (root.itemId) {
                        PluginRunner.recordExecution("apps", root.itemId);
                    }
                    ContextTracker.recordLaunch(root.appId);
                    
                    WindowManager.focusWindow(toplevel);
                    GlobalStates.closeWindowPicker();
                    GlobalStates.launcherOpen = false;
                }

                onWindowClosed: toplevel => {
                     WindowManager.closeWindow(toplevel);
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

}
