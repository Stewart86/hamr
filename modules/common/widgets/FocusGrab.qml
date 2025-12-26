import QtQuick
import Quickshell.Hyprland
import qs.services

Item {
    id: root

    required property var window
    property bool active: false
    property Item focusTarget: null
    property bool closeOnCleared: false

    signal closeRequested()

    property bool clearedByExternal: false

    readonly property alias grab: hyprlandGrab

    function regrabFocus() {
        if (!root.active) return;

        if (CompositorService.isHyprland) {
            hyprlandGrab.active = false;
            hyprlandGrab.active = true;
        }

        clearedByExternal = false;

        if (focusTarget) {
            focusTarget.forceActiveFocus();
        }
    }

    function activate() {
        if (CompositorService.isHyprland) {
            hyprlandGrab.active = true;
        }
        clearedByExternal = false;
        if (focusTarget) {
            focusTarget.forceActiveFocus();
        }
    }

    function deactivate() {
        if (CompositorService.isHyprland) {
            hyprlandGrab.active = false;
        }
    }

    HyprlandFocusGrab {
        id: hyprlandGrab
        windows: [root.window]
        active: root.active && CompositorService.isHyprland

        onCleared: {
            root.clearedByExternal = true;
            if (root.closeOnCleared && !active) {
                root.closeRequested();
            }
        }

        onActiveChanged: {
            if (active) {
                root.clearedByExternal = false;
            }
        }
    }

    onActiveChanged: {
        if (!CompositorService.isHyprland && active && focusTarget) {
            focusTarget.forceActiveFocus();
        }
    }
}
