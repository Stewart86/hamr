import QtQuick
import Quickshell.Hyprland
import qs.modules.common
import qs

Item {
    GlobalShortcut {
        name: "hamrToggle"
        description: "Toggle Hamr launcher"
        onPressed: {
            if (GlobalStates.launcherOpen && !GlobalStates.launcherMinimized) {
                if (Persistent.states.launcher.hasUsedMinimize ?? false) {
                    GlobalStates.launcherMinimized = true
                } else {
                    GlobalStates.softClose = true
                    GlobalStates.launcherOpen = false
                }
            } else {
                GlobalStates.launcherMinimized = false
                GlobalStates.launcherOpen = true
            }
        }
    }

    GlobalShortcut {
        name: "hamrToggleRelease"
        description: "Toggle Hamr on key release"
        onReleased: {
            if (GlobalStates.launcherOpen && !GlobalStates.launcherMinimized) {
                if (Persistent.states.launcher.hasUsedMinimize ?? false) {
                    GlobalStates.launcherMinimized = true
                } else {
                    GlobalStates.softClose = true
                    GlobalStates.launcherOpen = false
                }
            } else {
                GlobalStates.launcherMinimized = false
                GlobalStates.launcherOpen = true
            }
        }
    }
}
