//@ pragma UseQApplication
//@ pragma Env QS_NO_RELOAD_POPUP=1
//@ pragma Env QT_QUICK_CONTROLS_STYLE=Basic
//@ pragma Env QT_QUICK_FLICKABLE_WHEEL_DECELERATION=10000

// Adjust this to make the shell smaller or larger
//@ pragma Env QT_SCALE_FACTOR=1

import qs.modules.common
import qs.modules.launcher
import qs.modules.imageBrowser

import QtQuick
import QtQuick.Window
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland
import qs.services

ShellRoot {
    id: root

    Component.onCompleted: {
        MaterialThemeLoader.reapplyTheme()
        ShellHistory.refresh()
        PluginRunner.loadPlugins()
    }

    Launcher {}
    ImageBrowser {}
    WindowPicker {}

    // Reload popup for development
    ReloadPopup {}

    IpcHandler {
        target: "hamr"

        function toggle() {
            if (GlobalStates.launcherOpen) {
                // Toggle off - soft close (preserves state for restore window)
                GlobalStates.softClose = true
            }
            GlobalStates.launcherOpen = !GlobalStates.launcherOpen
        }

        function open() {
            GlobalStates.launcherMinimized = false
            GlobalStates.launcherOpen = true
        }

        function close() {
            // Explicit close request - hard close
            GlobalStates.softClose = false
            GlobalStates.launcherOpen = false
        }

        function openWith(prefix: string) {
            GlobalStates.launcherMinimized = false
            GlobalStates.launcherOpen = true
        }

        function plugin(name: string) {
            GlobalStates.launcherMinimized = false
            GlobalStates.launcherOpen = true
            LauncherSearch.startPlugin(name)
        }
    }

    // Global shortcuts for hamr
    // Hyprland bind format: bind = <modifiers>, <key>, global, quickshell:<name>
    
    GlobalShortcut {
        name: "hamrToggle"
        description: "Toggle Hamr launcher"
        onPressed: {
            if (GlobalStates.launcherOpen && !GlobalStates.launcherMinimized) {
                GlobalStates.softClose = true
                GlobalStates.launcherOpen = false
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
                GlobalStates.softClose = true
                GlobalStates.launcherOpen = false
            } else {
                GlobalStates.launcherMinimized = false
                GlobalStates.launcherOpen = true
            }
        }
    }
}

