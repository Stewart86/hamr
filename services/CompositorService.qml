pragma Singleton
pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import Quickshell.Hyprland
import qs.modules.common

Singleton {
    id: root

    property bool isHyprland: false
    property bool isNiri: false
    property string compositor: "unknown"

    readonly property string hyprlandSignature: Quickshell.env("HYPRLAND_INSTANCE_SIGNATURE")
    readonly property string niriSocket: Quickshell.env("NIRI_SOCKET")

    Component.onCompleted: detectCompositor()

    Timer {
        id: compositorInitTimer
        interval: 100
        running: true
        repeat: false
        onTriggered: detectCompositor()
    }

    function detectCompositor() {
        if (hyprlandSignature && hyprlandSignature.length > 0 && !niriSocket) {
            isHyprland = true;
            isNiri = false;
            compositor = "hyprland";
            console.info("CompositorService: Detected Hyprland");
            return;
        }

        if (niriSocket && niriSocket.length > 0) {
            isNiri = true;
            isHyprland = false;
            compositor = "niri";
            console.info("CompositorService: Detected Niri with socket:", niriSocket);
            return;
        }

        isHyprland = false;
        isNiri = false;
        compositor = "unknown";
        console.warn("CompositorService: No compositor detected");
    }

    function getFocusedScreen() {
        if (isHyprland && Hyprland.focusedMonitor) {
            const monitorName = Hyprland.focusedMonitor.name;
            for (let i = 0; i < Quickshell.screens.length; i++) {
                if (Quickshell.screens[i].name === monitorName) {
                    return Quickshell.screens[i];
                }
            }
        }

        if (isNiri && NiriService.currentOutput) {
            const outputName = NiriService.currentOutput;
            for (let i = 0; i < Quickshell.screens.length; i++) {
                if (Quickshell.screens[i].name === outputName) {
                    return Quickshell.screens[i];
                }
            }
        }

        return Quickshell.screens.length > 0 ? Quickshell.screens[0] : null;
    }

    readonly property string focusedScreenName: {
        if (isHyprland) {
            return Hyprland.focusedMonitor?.name ?? "";
        }
        if (isNiri) {
            return NiriService.currentOutput ?? "";
        }
        return Quickshell.screens.length > 0 ? Quickshell.screens[0].name : "";
    }

    function isScreenFocused(screen) {
        if (!screen) return false;
        if (Quickshell.screens.length === 1) return true;
        return screen.name === focusedScreenName;
    }

    function getScreenScale(screen) {
        if (!screen) return 1;

        if (isHyprland) {
            const hyprMonitor = Hyprland.monitors?.values?.find(m => m.name === screen.name);
            if (hyprMonitor?.scale !== undefined) {
                return hyprMonitor.scale;
            }
        }

        if (isNiri) {
            const niriScale = NiriService.displayScales[screen.name];
            if (niriScale !== undefined) {
                return niriScale;
            }
        }

        return screen?.devicePixelRatio ?? 1;
    }

    readonly property string currentWorkspace: {
        if (isHyprland) {
            return Hyprland.focusedMonitor?.activeWorkspace?.name ?? "";
        }
        if (isNiri) {
            const ws = NiriService.workspaces[NiriService.focusedWorkspaceId];
            return ws?.name ?? String(ws?.idx + 1) ?? "";
        }
        return "";
    }

    readonly property int currentWorkspaceId: {
        if (isHyprland) {
            return Hyprland.focusedMonitor?.activeWorkspace?.id ?? -1;
        }
        if (isNiri) {
            const wsId = NiriService.focusedWorkspaceId;
            if (wsId === "" || wsId === undefined || wsId === null) return -1;
            const parsed = parseInt(wsId, 10);
            return isNaN(parsed) ? -1 : parsed;
        }
        return -1;
    }

    readonly property string currentMonitor: {
        if (isHyprland) {
            return Hyprland.focusedMonitor?.name ?? "";
        }
        if (isNiri) {
            return NiriService.currentOutput ?? "";
        }
        return "";
    }

    readonly property var runningAppIds: {
        const apps = new Set();

        if (isHyprland) {
            const workspaces = Hyprland.workspaces?.values ?? [];
            for (const ws of workspaces) {
                const toplevels = ws.toplevels?.values ?? [];
                for (const toplevel of toplevels) {
                    const appClass = toplevel.lastIpcObject?.class ?? "";
                    if (appClass) {
                        apps.add(appClass.toLowerCase());
                    }
                }
            }
        }

        if (isNiri) {
            for (const w of NiriService.windows) {
                if (w.app_id) {
                    apps.add(w.app_id.toLowerCase());
                }
            }
        }

        return Array.from(apps);
    }

    signal compositorEvent(string eventName, var eventData)

    Connections {
        target: isHyprland ? Hyprland : null
        enabled: isHyprland

        function onRawEvent(event) {
            root.compositorEvent(event.name, event.data);
        }
    }

    Connections {
        target: isNiri ? NiriService : null
        enabled: isNiri

        function onWindowListChanged() {
            root.compositorEvent("windowschanged", null);
        }
    }
}
