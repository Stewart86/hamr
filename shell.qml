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

    // Initialize services on startup
    Component.onCompleted: {
        MaterialThemeLoader.reapplyTheme()
        ShellHistory.refresh()
        PluginRunner.loadPlugins()
    }

    // Main launcher components
    Launcher {}
    ImageBrowser {}
    WindowPicker {}

    // Reload popup for development
    ReloadPopup {}

    // IPC handler for hamr
    IpcHandler {
        target: "hamr"

        function toggle() {
            GlobalStates.launcherOpen = !GlobalStates.launcherOpen
        }

        function open() {
            GlobalStates.launcherOpen = true
        }

        function close() {
            GlobalStates.launcherOpen = false
        }

        function openWith(prefix: string) {
            // Open with a specific prefix (e.g., "~" for files, ";" for clipboard)
            GlobalStates.launcherOpen = true
            // TODO: Set the search prefix
        }

         function plugin(name: string) {
             // Start a specific plugin directly
             GlobalStates.launcherOpen = true
             LauncherSearch.startPlugin(name)
         }
    }

    // Global shortcuts for hamr
    // Hyprland bind format: bind = <modifiers>, <key>, global, quickshell:<name>
    
    GlobalShortcut {
        name: "hamrToggle"
        description: "Toggle Hamr launcher"
        onPressed: GlobalStates.launcherOpen = !GlobalStates.launcherOpen
    }

    GlobalShortcut {
        name: "hamrToggleRelease"
        description: "Toggle Hamr on key release"
        onReleased: {
            GlobalStates.launcherOpen = !GlobalStates.launcherOpen
        }
    }
}

