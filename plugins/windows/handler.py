#!/usr/bin/env python3
"""
Windows plugin handler - switch between open Hyprland windows.

Uses hyprctl to query and focus windows.
"""

import json
import os
import subprocess
import sys

TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

MOCK_WINDOWS = [
    {
        "address": "0x55587961e9a0",
        "class": "com.mitchellh.ghostty",
        "title": "Terminal",
        "workspace": {"id": 1, "name": "1"},
        "pid": 12345,
        "focusHistoryID": 0,
    },
    {
        "address": "0x55587961e9b0",
        "class": "firefox",
        "title": "GitHub - Mozilla Firefox",
        "workspace": {"id": 2, "name": "2"},
        "pid": 12346,
        "focusHistoryID": 1,
    },
    {
        "address": "0x55587961e9c0",
        "class": "code",
        "title": "handler.py - hamr - Visual Studio Code",
        "workspace": {"id": 1, "name": "1"},
        "pid": 12347,
        "focusHistoryID": 2,
    },
]

MOCK_WORKSPACES = [
    {"id": 1, "name": "1"},
    {"id": 2, "name": "2"},
    {"id": 3, "name": "3"},
]


def get_windows() -> list[dict]:
    """Get all open windows from Hyprland"""
    if TEST_MODE:
        return MOCK_WINDOWS

    try:
        result = subprocess.run(
            ["hyprctl", "clients", "-j"],
            capture_output=True,
            text=True,
            check=True,
        )
        windows = json.loads(result.stdout)
        # Sort by focusHistoryID (most recently focused first)
        windows.sort(key=lambda w: w.get("focusHistoryID", 999))
        return windows
    except (subprocess.CalledProcessError, FileNotFoundError, json.JSONDecodeError):
        return []


def get_workspaces() -> list[dict]:
    """Get all workspaces from Hyprland"""
    if TEST_MODE:
        return MOCK_WORKSPACES

    try:
        result = subprocess.run(
            ["hyprctl", "workspaces", "-j"],
            capture_output=True,
            text=True,
            check=True,
        )
        workspaces = json.loads(result.stdout)
        workspaces.sort(key=lambda w: w.get("id", 0))
        return workspaces
    except (subprocess.CalledProcessError, FileNotFoundError, json.JSONDecodeError):
        return []


def window_to_index_item(window: dict) -> dict:
    """Convert window to index item format"""
    address = window.get("address", "")
    title = window.get("title", "")
    window_class = window.get("class", "")
    workspace = window.get("workspace", {})
    workspace_name = workspace.get("name", str(workspace.get("id", "")))

    # Use class as description, add workspace info
    description = window_class
    if workspace_name:
        description = f"{window_class} (workspace {workspace_name})"

    return {
        "id": f"window:{address}",
        "name": title or window_class,
        "description": description,
        "icon": window_class,
        "iconType": "system",
        "verb": "Focus",
        "execute": {
            "command": ["hyprctl", "dispatch", "focuswindow", f"address:{address}"],
        },
    }


def window_to_result(window: dict, workspaces: list[dict] | None = None) -> dict:
    """Convert window to result format (for workflow mode)"""
    address = window.get("address", "")
    title = window.get("title", "")
    window_class = window.get("class", "")
    workspace = window.get("workspace", {})
    workspace_id = workspace.get("id", 0)
    workspace_name = workspace.get("name", str(workspace_id))

    description = window_class
    if workspace_name:
        description = f"{window_class} (workspace {workspace_name})"

    actions = [
        {"id": "close", "name": "Close Window", "icon": "close"},
    ]

    if workspaces:
        number_icons = {
            1: "looks_one",
            2: "looks_two",
            3: "looks_3",
            4: "looks_4",
            5: "looks_5",
            6: "looks_6",
        }
        for ws in workspaces:
            ws_id = ws.get("id", 0)
            ws_name = ws.get("name", str(ws_id))
            if ws_id != workspace_id and ws_id > 0:
                icon = number_icons.get(ws_id, "drive_file_move")
                actions.append(
                    {
                        "id": f"move:{ws_id}",
                        "name": f"Move to Workspace {ws_name}",
                        "icon": icon,
                    }
                )

    return {
        "id": f"window:{address}",
        "name": title or window_class,
        "description": description,
        "icon": window_class,
        "iconType": "system",
        "verb": "Focus",
        "actions": actions,
    }


def focus_window(address: str) -> tuple[bool, str]:
    """Focus a window by address"""
    if TEST_MODE:
        return True, f"Focused window {address}"

    try:
        subprocess.run(
            ["hyprctl", "dispatch", "focuswindow", f"address:{address}"],
            check=True,
            capture_output=True,
        )
        return True, "Window focused"
    except subprocess.CalledProcessError:
        return False, f"Failed to focus window {address}"


def close_window(address: str) -> tuple[bool, str]:
    """Close a window by address"""
    if TEST_MODE:
        return True, f"Closed window {address}"

    try:
        subprocess.run(
            ["hyprctl", "dispatch", "closewindow", f"address:{address}"],
            check=True,
            capture_output=True,
        )
        return True, "Window closed"
    except subprocess.CalledProcessError:
        return False, f"Failed to close window {address}"


def move_window_to_workspace(address: str, workspace_id: int) -> tuple[bool, str]:
    """Move a window to a workspace (silently, without switching to it)"""
    if TEST_MODE:
        return True, f"Moved window {address} to workspace {workspace_id}"

    try:
        subprocess.run(
            [
                "hyprctl",
                "dispatch",
                "movetoworkspacesilent",
                f"{workspace_id},address:{address}",
            ],
            check=True,
            capture_output=True,
        )
        return True, f"Moved to workspace {workspace_id}"
    except subprocess.CalledProcessError:
        return False, f"Failed to move window to workspace {workspace_id}"


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    windows = get_windows()
    workspaces = get_workspaces()

    if step == "index":
        items = [window_to_index_item(w) for w in windows]
        print(json.dumps({"type": "index", "items": items}))
        return

    if step == "initial":
        results = [window_to_result(w, workspaces) for w in windows]
        if not results:
            results = [
                {
                    "id": "__empty__",
                    "name": "No windows open",
                    "icon": "info",
                    "description": "Open an application to see windows here",
                }
            ]
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "placeholder": "Filter windows...",
                    "inputMode": "realtime",
                }
            )
        )
        return

    if step == "search":
        query_lower = query.lower()
        filtered = [
            w
            for w in windows
            if query_lower in w.get("title", "").lower()
            or query_lower in w.get("class", "").lower()
        ]
        results = [window_to_result(w, workspaces) for w in filtered]
        if not results:
            results = [
                {
                    "id": "__empty__",
                    "name": f"No windows matching '{query}'",
                    "icon": "search_off",
                }
            ]
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                }
            )
        )
        return

    if step == "action":
        item_id = selected.get("id", "")

        if item_id == "__empty__":
            print(json.dumps({"type": "execute", "execute": {"close": True}}))
            return

        if item_id.startswith("window:"):
            address = item_id.replace("window:", "")

            if action == "close":
                success, message = close_window(address)
                windows = get_windows()
                workspaces = get_workspaces()
                results = [window_to_result(w, workspaces) for w in windows]
                if not results:
                    results = [
                        {
                            "id": "__empty__",
                            "name": "No windows open",
                            "icon": "info",
                        }
                    ]
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": results,
                            "notify": message if success else None,
                        }
                    )
                )
                return

            if action.startswith("move:"):
                workspace_id = int(action.replace("move:", ""))
                success, message = move_window_to_workspace(address, workspace_id)
                windows = get_windows()
                workspaces = get_workspaces()
                results = [window_to_result(w, workspaces) for w in windows]
                if not results:
                    results = [
                        {
                            "id": "__empty__",
                            "name": "No windows open",
                            "icon": "info",
                        }
                    ]
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": results,
                            "notify": message if success else None,
                        }
                    )
                )
                return

            # Default action: focus and close launcher
            success, message = focus_window(address)
            if success:
                print(json.dumps({"type": "execute", "execute": {"close": True}}))
            else:
                print(json.dumps({"type": "error", "message": message}))
            return

    print(json.dumps({"type": "error", "message": f"Unknown step: {step}"}))


if __name__ == "__main__":
    main()
