pragma Singleton
pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import Quickshell.Io
import qs.modules.common
import qs.modules.common.functions

Singleton {
    id: root

    readonly property string socketPath: Quickshell.env("NIRI_SOCKET")

    property var workspaces: ({})
    property string focusedWorkspaceId: ""
    property string currentOutput: ""
    property string currentWorkspaceName: ""
    property var outputs: ({})
    property var windows: []
    property var displayScales: ({})
    property bool socketVerified: false

    signal windowListChanged

    function setWorkspaces(newMap) {
        root.workspaces = newMap;
    }

    Component.onCompleted: {
        if (socketPath && socketPath.length > 0) {
            verifySocket();
        }
    }

    function verifySocket() {
        Proc.runCommand("niri-socket-check", ["test", "-S", socketPath], (output, exitCode) => {
            if (exitCode === 0) {
                socketVerified = true;
                fetchOutputs();
                fetchWindows();
            } else {
                console.warn("NiriService: Socket not found:", socketPath);
            }
        });
    }

    function fetchWindows() {
        if (!root.isActive) return;
        Proc.runCommand("niri-fetch-windows", ["niri", "msg", "-j", "windows"], (output, exitCode) => {
            if (exitCode !== 0) {
                console.warn("NiriService: Failed to fetch windows, exit code:", exitCode);
                return;
            }
            try {
                const windowsData = JSON.parse(output);
                windows = windowsData;
                console.info("NiriService: Loaded", windowsData.length, "windows");
            } catch (e) {
                console.warn("NiriService: Failed to parse windows:", e);
            }
        });
    }

    property bool isActive: socketPath && socketPath.length > 0 && socketVerified

    Socket {
        id: eventStreamSocket
        path: root.socketPath
        connected: root.isActive

        onConnectionStateChanged: {
            if (connected) {
                write('"EventStream"\n');
                flush();
                fetchOutputs();
            }
        }

        parser: SplitParser {
            onRead: line => {
                try {
                    const event = JSON.parse(line);
                    handleNiriEvent(event);
                } catch (e) {
                    console.warn("NiriService: Failed to parse event:", line, e);
                }
            }
        }
    }

    Socket {
        id: requestSocket
        path: root.socketPath
        connected: root.isActive
    }

    function fetchOutputs() {
        if (!root.isActive) return;
        Proc.runCommand("niri-fetch-outputs", ["niri", "msg", "-j", "outputs"], (output, exitCode) => {
            if (exitCode !== 0) {
                console.warn("NiriService: Failed to fetch outputs, exit code:", exitCode);
                return;
            }
            try {
                const outputsData = JSON.parse(output);
                outputs = outputsData;
                console.info("NiriService: Loaded", Object.keys(outputsData).length, "outputs");
                updateDisplayScales();
            } catch (e) {
                console.warn("NiriService: Failed to parse outputs:", e);
            }
        });
    }

    function updateDisplayScales() {
        if (!outputs || Object.keys(outputs).length === 0) return;
        const scales = {};
        for (const outputName in outputs) {
            const output = outputs[outputName];
            if (output.logical && output.logical.scale !== undefined) {
                scales[outputName] = output.logical.scale;
            }
        }
        displayScales = scales;
    }

    function handleNiriEvent(event) {
        const eventType = Object.keys(event)[0];

        switch (eventType) {
        case 'WorkspacesChanged':
            handleWorkspacesChanged(event.WorkspacesChanged);
            break;
        case 'WorkspaceActivated':
            handleWorkspaceActivated(event.WorkspaceActivated);
            break;
        case 'WindowsChanged':
            handleWindowsChanged(event.WindowsChanged);
            break;
        case 'WindowClosed':
            handleWindowClosed(event.WindowClosed);
            break;
        case 'WindowOpenedOrChanged':
            handleWindowOpenedOrChanged(event.WindowOpenedOrChanged);
            break;
        case 'OutputsChanged':
            handleOutputsChanged(event.OutputsChanged);
            break;
        }
    }

    function handleWorkspacesChanged(data) {
        const newWorkspaces = {};
        for (const ws of data.workspaces) {
            newWorkspaces[ws.id] = ws;
        }
        setWorkspaces(newWorkspaces);

        const focusedWs = data.workspaces.find(w => w.is_focused);
        if (focusedWs) {
            focusedWorkspaceId = focusedWs.id;
            currentOutput = focusedWs.output ?? "";
            currentWorkspaceName = focusedWs.name ?? String(focusedWs.idx + 1);
        }
    }

    function handleWorkspaceActivated(data) {
        const ws = root.workspaces[data.id];
        if (!ws) return;

        if (data.focused) {
            const newOutput = ws.output ?? "";
            if (newOutput !== currentOutput) {
                currentOutput = newOutput;
            }
            if (data.id !== focusedWorkspaceId) {
                focusedWorkspaceId = data.id;
            }
            const newName = ws.name ?? String(ws.idx + 1);
            if (newName !== currentWorkspaceName) {
                currentWorkspaceName = newName;
            }
        }
    }

    function handleWindowsChanged(data) {
        windows = data.windows;
        windowListChanged();
    }

    function handleWindowClosed(data) {
        windows = windows.filter(w => w.id !== data.id);
        windowListChanged();
    }

    function handleWindowOpenedOrChanged(data) {
        if (!data.window) return;
        const window = data.window;
        const existingIndex = windows.findIndex(w => w.id === window.id);

        if (existingIndex >= 0) {
            const updatedWindows = [...windows];
            updatedWindows[existingIndex] = window;
            windows = updatedWindows;
        } else {
            windows = [...windows, window];
        }
        windowListChanged();
    }

    function handleOutputsChanged(data) {
        if (!data.outputs) return;
        outputs = data.outputs;
        updateDisplayScales();
    }

    function send(request) {
        if (!root.isActive || !requestSocket.connected) return false;
        const json = typeof request === "string" ? request : JSON.stringify(request);
        requestSocket.write(json + "\n");
        requestSocket.flush();
        return true;
    }

    function focusWindow(windowId) {
        return send({
            "Action": {
                "FocusWindow": {
                    "id": windowId
                }
            }
        });
    }

    function switchToWorkspace(workspaceIndex) {
        return send({
            "Action": {
                "FocusWorkspace": {
                    "reference": {
                        "Index": workspaceIndex
                    }
                }
            }
        });
    }

    function powerOffMonitors() {
        return send({
            "Action": {
                "PowerOffMonitors": {}
            }
        });
    }

    function powerOnMonitors() {
        return send({
            "Action": {
                "PowerOnMonitors": {}
            }
        });
    }
}
