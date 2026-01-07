#!/usr/bin/env python3
"""
Hyprland plugin handler - window management and dispatcher commands.

Uses hyprctl to query windows and execute Hyprland dispatchers.
Supports natural language commands like "move to workspace 2" or "toggle floating".

Runs as a daemon, watching Hyprland's IPC socket for window events.
"""

import json
import os
import re
import select
import socket
import subprocess
import sys
from pathlib import Path

TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

# Hyprland events that trigger reindex
WATCH_EVENTS = {"openwindow", "closewindow", "movewindow", "windowtitle"}


def get_hyprland_socket_path() -> Path | None:
    """Get path to Hyprland's event socket (.socket2.sock)."""
    runtime_dir = os.environ.get("XDG_RUNTIME_DIR")
    instance_sig = os.environ.get("HYPRLAND_INSTANCE_SIGNATURE")
    if not runtime_dir or not instance_sig:
        return None
    socket_path = Path(runtime_dir) / "hypr" / instance_sig / ".socket2.sock"
    return socket_path if socket_path.exists() else None


def connect_hyprland_socket() -> socket.socket | None:
    """Connect to Hyprland's event socket. Returns socket or None."""
    socket_path = get_hyprland_socket_path()
    if not socket_path:
        return None
    try:
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.connect(str(socket_path))
        sock.setblocking(False)
        return sock
    except (OSError, socket.error):
        return None


def read_hyprland_events(sock: socket.socket) -> list[str]:
    """Read pending events from Hyprland socket. Returns list of event names."""
    events = []
    try:
        data = sock.recv(4096)
        if data:
            for line in data.decode(errors="ignore").split("\n"):
                if ">>" in line:
                    event_name = line.split(">>")[0]
                    events.append(event_name)
    except (OSError, socket.error, BlockingIOError):
        pass
    return events


HYPR_DISPATCHERS = [
    # Workspace Navigation
    {
        "id": "goto-workspace",
        "name": "Go to Workspace",
        "description": "Switch to a specific workspace",
        "icon": "space_dashboard",
        "patterns": [
            r"(?:go\s*to|switch\s*to|workspace)\s*(\d+)",
            r"ws\s*(\d+)",
        ],
        "dispatcher": "workspace",
        "param_type": "workspace",
    },
    {
        "id": "move-to-workspace",
        "name": "Move to Workspace",
        "description": "Move active window to workspace (and switch)",
        "icon": "drive_file_move",
        "patterns": [
            r"move\s*(?:to\s*)?(?:workspace\s*)?(\d+)",
            r"send\s*(?:to\s*)?(?:workspace\s*)?(\d+)",
        ],
        "dispatcher": "movetoworkspace",
        "param_type": "workspace",
    },
    {
        "id": "move-to-workspace-silent",
        "name": "Move to Workspace (Silent)",
        "description": "Move active window to workspace (stay on current)",
        "icon": "drive_file_move_outline",
        "patterns": [
            r"move\s*(?:to\s*)?(?:workspace\s*)?(\d+)\s*silent",
            r"send\s*(?:to\s*)?(?:workspace\s*)?(\d+)\s*silent",
        ],
        "dispatcher": "movetoworkspacesilent",
        "param_type": "workspace",
    },
    {
        "id": "toggle-special",
        "name": "Toggle Scratchpad",
        "description": "Toggle the special workspace (scratchpad)",
        "icon": "visibility",
        "patterns": [
            r"(?:toggle\s*)?scratchpad",
            r"(?:toggle\s*)?special",
        ],
        "dispatcher": "togglespecialworkspace",
        "param_type": "none",
    },
    {
        "id": "next-workspace",
        "name": "Next Workspace",
        "description": "Go to next workspace",
        "icon": "arrow_forward",
        "patterns": [r"next\s*(?:workspace|ws)"],
        "dispatcher": "workspace",
        "param": "+1",
    },
    {
        "id": "prev-workspace",
        "name": "Previous Workspace",
        "description": "Go to previous workspace",
        "icon": "arrow_back",
        "patterns": [r"(?:prev(?:ious)?|back)\s*(?:workspace|ws)"],
        "dispatcher": "workspace",
        "param": "-1",
    },
    # Window State
    {
        "id": "toggle-floating",
        "name": "Toggle Floating",
        "description": "Toggle floating state of active window",
        "icon": "picture_in_picture",
        "patterns": [
            r"(?:toggle\s*)?float(?:ing)?",
            r"tile|tiled",
        ],
        "dispatcher": "togglefloating",
        "param_type": "none",
    },
    {
        "id": "fullscreen",
        "name": "Toggle Fullscreen",
        "description": "Toggle fullscreen (takes entire screen)",
        "icon": "fullscreen",
        "patterns": [r"fullscreen"],
        "dispatcher": "fullscreen",
        "param": "0",
    },
    {
        "id": "maximize",
        "name": "Toggle Maximize",
        "description": "Toggle maximize (keeps gaps and bar)",
        "icon": "crop_square",
        "patterns": [r"maximize"],
        "dispatcher": "fullscreen",
        "param": "1",
    },
    {
        "id": "pin",
        "name": "Pin Window",
        "description": "Pin window (show on all workspaces)",
        "icon": "push_pin",
        "patterns": [r"pin"],
        "dispatcher": "pin",
        "param_type": "none",
    },
    {
        "id": "center-window",
        "name": "Center Window",
        "description": "Center floating window on screen",
        "icon": "center_focus_strong",
        "patterns": [r"center(?:\s*window)?"],
        "dispatcher": "centerwindow",
        "param_type": "none",
    },
    {
        "id": "close-window",
        "name": "Close Window",
        "description": "Close the active window",
        "icon": "close",
        "patterns": [r"close(?:\s*window)?", r"kill(?:\s*active)?"],
        "dispatcher": "killactive",
        "param_type": "none",
    },
    # Focus Movement
    {
        "id": "focus-left",
        "name": "Focus Left",
        "description": "Move focus to the left",
        "icon": "west",
        "patterns": [r"focus\s*left"],
        "dispatcher": "movefocus",
        "param": "l",
    },
    {
        "id": "focus-right",
        "name": "Focus Right",
        "description": "Move focus to the right",
        "icon": "east",
        "patterns": [r"focus\s*right"],
        "dispatcher": "movefocus",
        "param": "r",
    },
    {
        "id": "focus-up",
        "name": "Focus Up",
        "description": "Move focus up",
        "icon": "north",
        "patterns": [r"focus\s*up"],
        "dispatcher": "movefocus",
        "param": "u",
    },
    {
        "id": "focus-down",
        "name": "Focus Down",
        "description": "Move focus down",
        "icon": "south",
        "patterns": [r"focus\s*down"],
        "dispatcher": "movefocus",
        "param": "d",
    },
    # Window Movement
    {
        "id": "move-left",
        "name": "Move Window Left",
        "description": "Move active window left",
        "icon": "arrow_back",
        "patterns": [r"move\s*(?:window\s*)?left"],
        "dispatcher": "movewindow",
        "param": "l",
    },
    {
        "id": "move-right",
        "name": "Move Window Right",
        "description": "Move active window right",
        "icon": "arrow_forward",
        "patterns": [r"move\s*(?:window\s*)?right"],
        "dispatcher": "movewindow",
        "param": "r",
    },
    {
        "id": "move-up",
        "name": "Move Window Up",
        "description": "Move active window up",
        "icon": "arrow_upward",
        "patterns": [r"move\s*(?:window\s*)?up"],
        "dispatcher": "movewindow",
        "param": "u",
    },
    {
        "id": "move-down",
        "name": "Move Window Down",
        "description": "Move active window down",
        "icon": "arrow_downward",
        "patterns": [r"move\s*(?:window\s*)?down"],
        "dispatcher": "movewindow",
        "param": "d",
    },
    # Swap
    {
        "id": "swap-left",
        "name": "Swap Left",
        "description": "Swap with window on the left",
        "icon": "swap_horiz",
        "patterns": [r"swap\s*(?:window\s*)?left"],
        "dispatcher": "swapwindow",
        "param": "l",
    },
    {
        "id": "swap-right",
        "name": "Swap Right",
        "description": "Swap with window on the right",
        "icon": "swap_horiz",
        "patterns": [r"swap\s*(?:window\s*)?right"],
        "dispatcher": "swapwindow",
        "param": "r",
    },
    {
        "id": "swap-up",
        "name": "Swap Up",
        "description": "Swap with window above",
        "icon": "swap_vert",
        "patterns": [r"swap\s*(?:window\s*)?up"],
        "dispatcher": "swapwindow",
        "param": "u",
    },
    {
        "id": "swap-down",
        "name": "Swap Down",
        "description": "Swap with window below",
        "icon": "swap_vert",
        "patterns": [r"swap\s*(?:window\s*)?down"],
        "dispatcher": "swapwindow",
        "param": "d",
    },
    # Cycle
    {
        "id": "cycle-next",
        "name": "Cycle Next Window",
        "description": "Focus next window on workspace",
        "icon": "keyboard_tab",
        "patterns": [r"cycle\s*next", r"next\s*window"],
        "dispatcher": "cyclenext",
        "param_type": "none",
    },
    {
        "id": "cycle-prev",
        "name": "Cycle Previous Window",
        "description": "Focus previous window on workspace",
        "icon": "keyboard_tab",
        "patterns": [r"cycle\s*prev(?:ious)?", r"prev(?:ious)?\s*window"],
        "dispatcher": "cyclenext",
        "param": "prev",
    },
    {
        "id": "focus-last",
        "name": "Focus Last Window",
        "description": "Switch between current and last focused window",
        "icon": "history",
        "patterns": [r"(?:focus\s*)?last(?:\s*window)?", r"alt[\s-]*tab"],
        "dispatcher": "focuscurrentorlast",
        "param_type": "none",
    },
    {
        "id": "focus-urgent",
        "name": "Focus Urgent Window",
        "description": "Focus urgent window or last focused",
        "icon": "priority_high",
        "patterns": [r"(?:focus\s*)?urgent"],
        "dispatcher": "focusurgentorlast",
        "param_type": "none",
    },
    # Monitor
    {
        "id": "focus-next-monitor",
        "name": "Focus Next Monitor",
        "description": "Focus the next monitor",
        "icon": "desktop_windows",
        "patterns": [r"(?:focus\s*)?next\s*monitor", r"monitor\s*right"],
        "dispatcher": "focusmonitor",
        "param": "+1",
    },
    {
        "id": "focus-prev-monitor",
        "name": "Focus Previous Monitor",
        "description": "Focus the previous monitor",
        "icon": "desktop_windows",
        "patterns": [r"(?:focus\s*)?prev(?:ious)?\s*monitor", r"monitor\s*left"],
        "dispatcher": "focusmonitor",
        "param": "-1",
    },
    {
        "id": "move-workspace-to-monitor",
        "name": "Move Workspace to Monitor",
        "description": "Move current workspace to next monitor",
        "icon": "monitor",
        "patterns": [
            r"move\s*workspace\s*(?:to\s*)?(?:next\s*)?monitor",
            r"workspace\s*to\s*monitor",
        ],
        "dispatcher": "movecurrentworkspacetomonitor",
        "param": "+1",
    },
    {
        "id": "swap-workspaces",
        "name": "Swap Workspaces Between Monitors",
        "description": "Swap active workspaces between two monitors",
        "icon": "swap_horizontal_circle",
        "patterns": [r"swap\s*workspaces?(?:\s*monitors?)?"],
        "dispatcher": "swapactiveworkspaces",
        "param": "0 1",
    },
    # Layout
    {
        "id": "increase-split",
        "name": "Increase Split Ratio",
        "description": "Increase the split ratio",
        "icon": "expand",
        "patterns": [r"(?:increase|grow|expand)\s*(?:split|ratio)"],
        "dispatcher": "splitratio",
        "param": "+0.1",
    },
    {
        "id": "decrease-split",
        "name": "Decrease Split Ratio",
        "description": "Decrease the split ratio",
        "icon": "compress",
        "patterns": [r"(?:decrease|shrink|contract)\s*(?:split|ratio)"],
        "dispatcher": "splitratio",
        "param": "-0.1",
    },
    # Groups
    {
        "id": "toggle-group",
        "name": "Create/Dissolve Group",
        "description": "Make current window a group, or dissolve if already grouped",
        "icon": "tab",
        "patterns": [
            r"(?:toggle\s*)?group",
            r"tab(?:bed)?",
            r"create\s*group",
            r"dissolve\s*group",
        ],
        "dispatcher": "togglegroup",
        "param_type": "none",
    },
    {
        "id": "group-next",
        "name": "Next in Group",
        "description": "Focus next window in group",
        "icon": "tab",
        "patterns": [r"(?:group\s*)?next\s*tab", r"next\s*(?:in\s*)?group"],
        "dispatcher": "changegroupactive",
        "param": "f",
    },
    {
        "id": "group-prev",
        "name": "Previous in Group",
        "description": "Focus previous window in group",
        "icon": "tab",
        "patterns": [
            r"(?:group\s*)?prev(?:ious)?\s*tab",
            r"prev(?:ious)?\s*(?:in\s*)?group",
        ],
        "dispatcher": "changegroupactive",
        "param": "b",
    },
    {
        "id": "lock-groups",
        "name": "Toggle Lock Groups",
        "description": "Lock/unlock all groups",
        "icon": "lock",
        "patterns": [r"lock\s*groups?"],
        "dispatcher": "lockgroups",
        "param": "toggle",
    },
    {
        "id": "move-into-group-left",
        "name": "Join Group Left",
        "description": "Add this window to the group on the left",
        "icon": "tab_move",
        "patterns": [r"(?:add|move|join)\s*(?:to|into)?\s*group\s*left"],
        "dispatcher": "moveintogroup",
        "param": "l",
    },
    {
        "id": "move-into-group-right",
        "name": "Join Group Right",
        "description": "Add this window to the group on the right",
        "icon": "tab_move",
        "patterns": [r"(?:add|move|join)\s*(?:to|into)?\s*group\s*right"],
        "dispatcher": "moveintogroup",
        "param": "r",
    },
    {
        "id": "move-into-group-up",
        "name": "Join Group Up",
        "description": "Add this window to the group above",
        "icon": "tab_move",
        "patterns": [r"(?:add|move|join)\s*(?:to|into)?\s*group\s*up"],
        "dispatcher": "moveintogroup",
        "param": "u",
    },
    {
        "id": "move-into-group-down",
        "name": "Join Group Down",
        "description": "Add this window to the group below",
        "icon": "tab_move",
        "patterns": [r"(?:add|move|join)\s*(?:to|into)?\s*group\s*down"],
        "dispatcher": "moveintogroup",
        "param": "d",
    },
    {
        "id": "move-out-of-group",
        "name": "Remove from Group",
        "description": "Move window out of its group",
        "icon": "tab_close",
        "patterns": [
            r"(?:move\s*)?out\s*(?:of\s*)?group",
            r"remove\s*(?:from\s*)?group",
            r"ungroup",
        ],
        "dispatcher": "moveoutofgroup",
        "param_type": "none",
    },
    # Misc
    {
        "id": "cursor-corner",
        "name": "Move Cursor to Corner",
        "description": "Move cursor to corner of active window",
        "icon": "open_in_full",
        "patterns": [r"cursor\s*(?:to\s*)?corner", r"move\s*cursor"],
        "dispatcher": "movecursortocorner",
        "param": "2",
    },
    {
        "id": "reload-renderer",
        "name": "Reload Renderer",
        "description": "Force reload all Hyprland resources",
        "icon": "refresh",
        "patterns": [r"(?:force\s*)?reload\s*(?:renderer|resources)"],
        "dispatcher": "forcerendererreload",
        "param_type": "none",
    },
]

MOCK_GLOBAL_SHORTCUTS = [
    {"id": "quickshell:overlayToggle", "description": "Toggles overlay on press"},
    {"id": "com.example.app:toggle", "description": "Toggle feature"},
]


def get_global_shortcuts() -> list[dict]:
    """Get registered global shortcuts from Hyprland"""
    if TEST_MODE:
        return MOCK_GLOBAL_SHORTCUTS

    try:
        result = subprocess.run(
            ["hyprctl", "globalshortcuts"],
            capture_output=True,
            text=True,
            check=True,
        )
        shortcuts = []
        for line in result.stdout.strip().split("\n"):
            if " -> " in line:
                shortcut_id, description = line.split(" -> ", 1)
                shortcuts.append(
                    {
                        "id": shortcut_id.strip(),
                        "description": description.strip(),
                    }
                )
        return shortcuts
    except (subprocess.CalledProcessError, FileNotFoundError):
        return []


def shortcut_to_result(shortcut: dict) -> dict:
    """Convert global shortcut to result format"""
    shortcut_id = shortcut["id"]
    description = shortcut["description"]

    # Extract app name from id (before the colon)
    app_name = shortcut_id.split(":")[0] if ":" in shortcut_id else shortcut_id

    return {
        "id": f"shortcut:{shortcut_id}",
        "name": description,
        "description": f"{app_name} shortcut",
        "icon": "keyboard",
        "verb": "Run",
    }


def shortcut_to_index_item(shortcut: dict) -> dict:
    """Convert global shortcut to index item format"""
    shortcut_id = shortcut["id"]
    description = shortcut["description"]
    app_name = shortcut_id.split(":")[0] if ":" in shortcut_id else shortcut_id
    item_id = f"shortcut:{shortcut_id}"

    return {
        "id": item_id,
        "name": description,
        "description": f"{app_name} shortcut",
        "icon": "keyboard",
        "verb": "Run",
        "keywords": [
            shortcut_id.lower(),
            description.lower(),
            app_name.lower(),
            "shortcut",
            "global",
        ],
        "entryPoint": {
            "step": "action",
            "selected": {"id": item_id},
        },
    }


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

    item_id = f"window:{address}"
    return {
        "id": item_id,
        "name": title or window_class,
        "description": description,
        "icon": window_class,
        "iconType": "system",
        "verb": "Focus",
        "entryPoint": {
            "step": "action",
            "selected": {"id": item_id},
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


def match_dispatcher(query: str) -> tuple[dict | None, str | None]:
    """Match query against dispatcher patterns, return (dispatcher, extracted_param)"""
    query_lower = query.lower().strip()
    for dispatcher in HYPR_DISPATCHERS:
        for pattern in dispatcher.get("patterns", []):
            match = re.search(pattern, query_lower, re.IGNORECASE)
            if match:
                extracted_param = match.group(1) if match.lastindex else None
                return dispatcher, extracted_param
    return None, None


def filter_dispatchers(query: str) -> list[dict]:
    """Filter dispatchers by name/description matching"""
    query_lower = query.lower().strip()
    results = []
    for dispatcher in HYPR_DISPATCHERS:
        name = dispatcher.get("name", "").lower()
        desc = dispatcher.get("description", "").lower()
        dispatcher_id = dispatcher.get("id", "").lower()
        if query_lower in name or query_lower in desc or query_lower in dispatcher_id:
            results.append(dispatcher)
    return results


def dispatcher_to_result(dispatcher: dict, extracted_param: str | None = None) -> dict:
    """Convert dispatcher to result format"""
    name = dispatcher.get("name", "")
    description = dispatcher.get("description", "")
    dispatcher_id = dispatcher.get("id", "")

    if extracted_param and dispatcher.get("param_type") == "workspace":
        name = f"{name} {extracted_param}"
        description = f"{description} (workspace {extracted_param})"
        dispatcher_id = f"{dispatcher_id}:{extracted_param}"

    return {
        "id": f"dispatch:{dispatcher_id}",
        "name": name,
        "description": description,
        "icon": dispatcher.get("icon", "terminal"),
        "verb": "Run",
    }


def dispatcher_to_index_item(dispatcher: dict) -> dict:
    """Convert dispatcher to index item format"""
    name = dispatcher.get("name", "")
    icon = dispatcher.get("icon", "terminal")
    item_id = f"dispatch:{dispatcher['id']}"

    item = {
        "id": item_id,
        "name": name,
        "description": dispatcher.get("description", ""),
        "icon": icon,
        "verb": "Run",
        "keywords": [
            dispatcher.get("id", ""),
            dispatcher.get("dispatcher", ""),
            name.lower(),
        ],
    }

    # Add entryPoint for dispatchers with static params (can be executed directly)
    if dispatcher.get("param_type") != "workspace":
        item["entryPoint"] = {
            "step": "action",
            "selected": {"id": item_id},
        }

    return item


def get_common_commands() -> list[dict]:
    """Get commonly used commands for initial view"""
    commands = []

    # Common dispatchers to show in initial view
    common_ids = [
        "toggle-floating",
        "fullscreen",
        "maximize",
        "center-window",
        "close-window",
        "pin",
        "toggle-group",
        "move-into-group-left",
        "move-into-group-right",
        "move-out-of-group",
        "group-next",
        "group-prev",
        "focus-last",
    ]

    for dispatcher in HYPR_DISPATCHERS:
        if dispatcher["id"] in common_ids:
            commands.append(
                {
                    "id": f"dispatch:{dispatcher['id']}",
                    "name": dispatcher["name"],
                    "description": dispatcher["description"],
                    "icon": dispatcher["icon"],
                    "verb": "Run",
                }
            )

    # Add workspace navigation
    commands.extend(
        [
            {
                "id": "dispatch:workspace-next",
                "name": "Next Workspace",
                "description": "Go to next workspace",
                "icon": "arrow_forward",
                "verb": "Run",
            },
            {
                "id": "dispatch:workspace-prev",
                "name": "Previous Workspace",
                "description": "Go to previous workspace",
                "icon": "arrow_back",
                "verb": "Run",
            },
            {
                "id": "dispatch:goto-special",
                "name": "Toggle Scratchpad",
                "description": "Toggle the special workspace",
                "icon": "visibility",
                "verb": "Run",
            },
        ]
    )

    return commands


def generate_workspace_index_items() -> list[dict]:
    """Generate index items for workspace 1-10 shortcuts plus special workspaces"""
    items = []
    number_icons = {
        1: "looks_one",
        2: "looks_two",
        3: "looks_3",
        4: "looks_4",
        5: "looks_5",
        6: "looks_6",
    }

    for ws in range(1, 11):
        icon = number_icons.get(ws, "space_dashboard")
        goto_id = f"dispatch:goto-workspace:{ws}"
        move_id = f"dispatch:move-to-workspace:{ws}"
        goto_name = f"Go to Workspace {ws}"
        move_name = f"Move to Workspace {ws}"
        items.append(
            {
                "id": goto_id,
                "name": goto_name,
                "description": f"Switch to workspace {ws}",
                "icon": icon,
                "verb": "Run",
                "keywords": [
                    f"workspace {ws}",
                    f"ws {ws}",
                    f"go to {ws}",
                    f"switch to {ws}",
                ],
                "entryPoint": {
                    "step": "action",
                    "selected": {"id": goto_id},
                },
            }
        )
        items.append(
            {
                "id": move_id,
                "name": move_name,
                "description": f"Move active window to workspace {ws}",
                "icon": icon,
                "verb": "Run",
                "keywords": [f"move to {ws}", f"send to {ws}", f"move workspace {ws}"],
                "entryPoint": {
                    "step": "action",
                    "selected": {"id": move_id},
                },
            }
        )

    # Special workspace (scratchpad)
    items.append(
        {
            "id": "dispatch:goto-special",
            "name": "Toggle Scratchpad",
            "description": "Toggle the special workspace (scratchpad)",
            "icon": "visibility",
            "verb": "Run",
            "keywords": ["scratchpad", "special", "toggle special", "scratch"],
            "entryPoint": {
                "step": "action",
                "selected": {"id": "dispatch:goto-special"},
            },
        }
    )
    items.append(
        {
            "id": "dispatch:move-to-special",
            "name": "Move to Scratchpad",
            "description": "Move active window to special workspace",
            "icon": "visibility_off",
            "verb": "Run",
            "keywords": ["move to scratchpad", "send to special", "move special"],
            "entryPoint": {
                "step": "action",
                "selected": {"id": "dispatch:move-to-special"},
            },
        }
    )

    # Relative workspace navigation
    items.append(
        {
            "id": "dispatch:workspace-next",
            "name": "Next Workspace",
            "description": "Go to next workspace",
            "icon": "arrow_forward",
            "verb": "Run",
            "keywords": ["next workspace", "workspace next", "ws next"],
            "entryPoint": {
                "step": "action",
                "selected": {"id": "dispatch:workspace-next"},
            },
        }
    )
    items.append(
        {
            "id": "dispatch:workspace-prev",
            "name": "Previous Workspace",
            "description": "Go to previous workspace",
            "icon": "arrow_back",
            "verb": "Run",
            "keywords": [
                "previous workspace",
                "prev workspace",
                "workspace prev",
                "ws prev",
                "back workspace",
            ],
            "entryPoint": {
                "step": "action",
                "selected": {"id": "dispatch:workspace-prev"},
            },
        }
    )
    items.append(
        {
            "id": "dispatch:workspace-empty",
            "name": "Go to Empty Workspace",
            "description": "Switch to first empty workspace",
            "icon": "add_box",
            "verb": "Run",
            "keywords": ["empty workspace", "new workspace", "blank workspace"],
            "entryPoint": {
                "step": "action",
                "selected": {"id": "dispatch:workspace-empty"},
            },
        }
    )

    return items


def execute_dispatcher(dispatcher: dict, param: str | None = None) -> tuple[bool, str]:
    """Execute a Hyprland dispatcher"""
    dispatcher_name = dispatcher.get("dispatcher", "")
    dispatcher_param = param or dispatcher.get("param", "")

    if TEST_MODE:
        return True, f"Would run: hyprctl dispatch {dispatcher_name} {dispatcher_param}"

    try:
        cmd = ["hyprctl", "dispatch", dispatcher_name]
        if dispatcher_param:
            cmd.append(dispatcher_param)
        subprocess.run(cmd, check=True, capture_output=True)
        return True, f"{dispatcher.get('name', dispatcher_name)} executed"
    except subprocess.CalledProcessError as e:
        return False, f"Failed to execute {dispatcher_name}: {e}"


def get_index_items() -> list[dict]:
    """Generate full index items (windows + dispatchers + shortcuts)."""
    windows = get_windows()
    items = [window_to_index_item(w) for w in windows]
    # Only index dispatchers that have static params (can be executed directly)
    for d in HYPR_DISPATCHERS:
        if d.get("param_type") != "workspace":
            items.append(dispatcher_to_index_item(d))
    items.extend(generate_workspace_index_items())
    # Add global shortcuts
    shortcuts = get_global_shortcuts()
    items.extend([shortcut_to_index_item(s) for s in shortcuts])
    return items


def handle_request(input_data: dict):
    """Handle a single request from stdin."""
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    windows = get_windows()
    workspaces = get_workspaces()

    if step == "index":
        items = get_index_items()
        print(json.dumps({"type": "index", "items": items}))
        return

    if step == "initial":
        results = [window_to_result(w, workspaces) for w in windows]
        results.extend(get_common_commands())
        shortcuts = get_global_shortcuts()
        results.extend([shortcut_to_result(s) for s in shortcuts])
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "placeholder": "Filter windows or type a command...",
                    "inputMode": "realtime",
                }
            )
        )
        return

    if step == "search":
        query_lower = query.lower()
        results = []

        matched_dispatcher, extracted_param = match_dispatcher(query)
        if matched_dispatcher:
            results.append(dispatcher_to_result(matched_dispatcher, extracted_param))

        filtered_dispatchers = filter_dispatchers(query)
        for d in filtered_dispatchers:
            if not matched_dispatcher or d["id"] != matched_dispatcher["id"]:
                results.append(dispatcher_to_result(d))

        filtered_windows = [
            w
            for w in windows
            if query_lower in w.get("title", "").lower()
            or query_lower in w.get("class", "").lower()
        ]

        # Filter global shortcuts
        shortcuts = get_global_shortcuts()
        filtered_shortcuts = [
            s
            for s in shortcuts
            if query_lower in s["id"].lower() or query_lower in s["description"].lower()
        ]
        results.extend([shortcut_to_result(s) for s in filtered_shortcuts])
        results.extend([window_to_result(w, workspaces) for w in filtered_windows])

        if not results:
            results = [
                {
                    "id": "__empty__",
                    "name": f"No matches for '{query}'",
                    "icon": "search_off",
                    "description": "Try 'move to 2', 'toggle floating', 'fullscreen'...",
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
            print(json.dumps({"type": "execute", "close": True}))
            return

        if item_id.startswith("shortcut:"):
            shortcut_id = item_id.replace("shortcut:", "")
            cmd = ["hyprctl", "dispatch", "global", shortcut_id]

            # Find the shortcut description for the notification
            shortcuts = get_global_shortcuts()
            shortcut = next((s for s in shortcuts if s["id"] == shortcut_id), None)
            name = shortcut["description"] if shortcut else shortcut_id

            if TEST_MODE:
                print(
                    json.dumps(
                        {
                            "type": "execute",
                            "close": True,
                            "notify": f"Would run: {' '.join(cmd)}",
                        }
                    )
                )
                return
            try:
                subprocess.run(cmd, check=True, capture_output=True)
                print(
                    json.dumps(
                        {
                            "type": "execute",
                            "close": True,
                            "notify": f"{name} executed",
                        }
                    )
                )
            except subprocess.CalledProcessError as e:
                print(json.dumps({"type": "error", "message": f"Failed: {e}"}))
            return

        if item_id.startswith("dispatch:"):
            dispatcher_id_full = item_id.replace("dispatch:", "")

            # Handle generated workspace commands (workspace-next, goto-special, etc.)
            static_commands = {
                "workspace-next": (
                    ["hyprctl", "dispatch", "workspace", "+1"],
                    "Next Workspace",
                ),
                "workspace-prev": (
                    ["hyprctl", "dispatch", "workspace", "-1"],
                    "Previous Workspace",
                ),
                "workspace-empty": (
                    ["hyprctl", "dispatch", "workspace", "empty"],
                    "Go to Empty Workspace",
                ),
                "goto-special": (
                    ["hyprctl", "dispatch", "togglespecialworkspace"],
                    "Toggle Scratchpad",
                ),
                "move-to-special": (
                    ["hyprctl", "dispatch", "movetoworkspacesilent", "special"],
                    "Move to Scratchpad",
                ),
            }

            if dispatcher_id_full in static_commands:
                cmd, name = static_commands[dispatcher_id_full]
                if TEST_MODE:
                    print(
                        json.dumps(
                            {
                                "type": "execute",
                                "close": True,
                                "notify": f"Would run: {' '.join(cmd)}",
                            }
                        )
                    )
                    return
                try:
                    subprocess.run(cmd, check=True, capture_output=True)
                    print(
                        json.dumps(
                            {
                                "type": "execute",
                                "close": True,
                                "notify": f"{name} executed",
                            }
                        )
                    )
                except subprocess.CalledProcessError as e:
                    print(json.dumps({"type": "error", "message": f"Failed: {e}"}))
                return

            # Handle goto-workspace:N and move-to-workspace:N
            if dispatcher_id_full.startswith("goto-workspace:"):
                ws = dispatcher_id_full.split(":")[1]
                cmd = ["hyprctl", "dispatch", "workspace", ws]
                name = f"Go to Workspace {ws}"
                if TEST_MODE:
                    print(
                        json.dumps(
                            {
                                "type": "execute",
                                "close": True,
                                "notify": f"Would run: {' '.join(cmd)}",
                            }
                        )
                    )
                    return
                try:
                    subprocess.run(cmd, check=True, capture_output=True)
                    print(
                        json.dumps(
                            {
                                "type": "execute",
                                "close": True,
                                "notify": f"{name} executed",
                            }
                        )
                    )
                except subprocess.CalledProcessError as e:
                    print(json.dumps({"type": "error", "message": f"Failed: {e}"}))
                return

            if dispatcher_id_full.startswith("move-to-workspace:"):
                ws = dispatcher_id_full.split(":")[1]
                cmd = ["hyprctl", "dispatch", "movetoworkspace", ws]
                name = f"Move to Workspace {ws}"
                if TEST_MODE:
                    print(
                        json.dumps(
                            {
                                "type": "execute",
                                "close": True,
                                "notify": f"Would run: {' '.join(cmd)}",
                            }
                        )
                    )
                    return
                try:
                    subprocess.run(cmd, check=True, capture_output=True)
                    print(
                        json.dumps(
                            {
                                "type": "execute",
                                "close": True,
                                "notify": f"{name} executed",
                            }
                        )
                    )
                except subprocess.CalledProcessError as e:
                    print(json.dumps({"type": "error", "message": f"Failed: {e}"}))
                return

            # Handle HYPR_DISPATCHERS
            extracted_param = None
            if ":" in dispatcher_id_full:
                dispatcher_id, extracted_param = dispatcher_id_full.split(":", 1)
            else:
                dispatcher_id = dispatcher_id_full

            dispatcher = next(
                (d for d in HYPR_DISPATCHERS if d["id"] == dispatcher_id), None
            )
            if not dispatcher:
                print(
                    json.dumps(
                        {
                            "type": "error",
                            "message": f"Unknown dispatcher: {dispatcher_id}",
                        }
                    )
                )
                return

            param = extracted_param or dispatcher.get("param", "")

            success, message = execute_dispatcher(dispatcher, param)
            if success:
                print(
                    json.dumps(
                        {
                            "type": "execute",
                            "close": True,
                            "notify": message,
                        }
                    )
                )
            else:
                print(json.dumps({"type": "error", "message": message}))
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

            success, message = focus_window(address)
            if success:
                print(json.dumps({"type": "execute", "close": True}))
            else:
                print(json.dumps({"type": "error", "message": message}))
            return

    print(json.dumps({"type": "error", "message": f"Unknown step: {step}"}))


def main():
    """Daemon main loop with Hyprland IPC socket watching."""
    # In test mode, handle single request and exit
    if TEST_MODE:
        input_data = json.load(sys.stdin)
        handle_request(input_data)
        return

    # Emit full index on startup
    items = get_index_items()
    print(json.dumps({"type": "index", "mode": "full", "items": items}), flush=True)

    # Try to connect to Hyprland's event socket
    hypr_socket = connect_hyprland_socket()

    if hypr_socket is not None:
        # Daemon mode with Hyprland IPC socket
        while True:
            try:
                readable, _, _ = select.select([sys.stdin, hypr_socket], [], [], 1.0)
            except (ValueError, OSError):
                # Socket closed, try to reconnect
                hypr_socket = connect_hyprland_socket()
                if hypr_socket is None:
                    break
                continue

            for r in readable:
                if r == sys.stdin:
                    try:
                        line = sys.stdin.readline()
                        if not line:
                            return
                        input_data = json.loads(line)
                        handle_request(input_data)
                        sys.stdout.flush()
                    except json.JSONDecodeError:
                        continue

                elif r == hypr_socket:
                    events = read_hyprland_events(hypr_socket)
                    if any(ev in WATCH_EVENTS for ev in events):
                        print(json.dumps({"type": "index"}))
                        sys.stdout.flush()
    else:
        # Fallback: no socket, just handle stdin requests (polling mode)
        while True:
            try:
                readable, _, _ = select.select([sys.stdin], [], [], 2.0)
            except (ValueError, OSError):
                break

            if readable:
                try:
                    line = sys.stdin.readline()
                    if not line:
                        return
                    input_data = json.loads(line)
                    handle_request(input_data)
                    sys.stdout.flush()
                except json.JSONDecodeError:
                    continue


if __name__ == "__main__":
    main()
