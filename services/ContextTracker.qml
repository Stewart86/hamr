pragma Singleton

import qs.modules.common
import QtQuick
import Quickshell
import Quickshell.Hyprland

Singleton {
    id: root

    // Current Hyprland context
    readonly property string currentWorkspace: Hyprland.focusedMonitor?.activeWorkspace?.name ?? ""
    readonly property int currentWorkspaceId: Hyprland.focusedMonitor?.activeWorkspace?.id ?? -1
    readonly property string currentMonitor: Hyprland.focusedMonitor?.name ?? ""
    
    // Session tracking
    property string hyprlandInstanceSignature: Quickshell.env("HYPRLAND_INSTANCE_SIGNATURE") ?? ""
    property string previousInstanceSignature: ""
    property bool isNewSession: false
    property real sessionStartTime: 0
    
    // Last launched app tracking (for sequence detection)
    property string lastLaunchedApp: ""
    property real lastLaunchTime: 0
    readonly property int sequenceWindowMs: 10 * 60 * 1000  // 10 minutes
    
    // Running apps context (for co-occurrence suggestions)
    readonly property var runningAppIds: {
        const apps = new Set();
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
        return Array.from(apps);
    }
    
    // Check if we're within the sequence window of the last launch
    function isWithinSequenceWindow() {
        if (!lastLaunchedApp || lastLaunchTime === 0) return false;
        return (Date.now() - lastLaunchTime) < sequenceWindowMs;
    }
    
    // Record an app launch for sequence tracking
    function recordLaunch(appId) {
        lastLaunchedApp = appId;
        lastLaunchTime = Date.now();
    }
    
    // Check if this is the first launch of the session
    function isSessionStart() {
        if (!isNewSession) return false;
        const timeSinceSessionStart = Date.now() - sessionStartTime;
        return timeSinceSessionStart < 5 * 60 * 1000;  // Within 5 minutes of session start
    }
    
    // Get context object for suggestion calculation
    function getContext() {
        const now = new Date();
        return {
            currentHour: now.getHours(),
            currentDay: now.getDay() === 0 ? 6 : now.getDay() - 1,  // Monday=0, Sunday=6
            workspace: currentWorkspace,
            workspaceId: currentWorkspaceId,
            monitor: currentMonitor,
            lastApp: isWithinSequenceWindow() ? lastLaunchedApp : "",
            isSessionStart: isSessionStart(),
            runningApps: runningAppIds
        };
    }
    
    // Initialize session tracking
    Component.onCompleted: {
        previousInstanceSignature = Persistent.states.context?.previousHyprlandInstance ?? "";
        
        if (hyprlandInstanceSignature !== previousInstanceSignature) {
            isNewSession = true;
            sessionStartTime = Date.now();
            Persistent.states.context = {
                previousHyprlandInstance: hyprlandInstanceSignature
            };
        }
    }
    
    // Listen for Hyprland events for additional context
    Connections {
        target: Hyprland
        function onRawEvent(event) {
            const eventName = event.name;
            
            if (eventName === "activewindow" || eventName === "activewindowv2") {
                // Could track active window changes for more context
            }
        }
    }
}
