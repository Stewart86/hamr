#!/usr/bin/env python3
"""
Niri plugin handler - window management and compositor actions.

Uses niri msg to query windows and execute Niri actions.
Runs as a daemon emitting full index on startup.
"""

import json
import os
import select
import signal
import subprocess
import sys
import time
from pathlib import Path

# Add parent directory to path to import SDK
sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

INDEX_DEBOUNCE_INTERVAL = 2.0

NIRI_ACTIONS = [
    # Window State
    {
        "id": "close-window",
        "name": "Close Window",
        "description": "Close the focused window",
        "icon": "close",
        "action": "close-window",
    },
    {
        "id": "fullscreen-window",
        "name": "Toggle Fullscreen",
        "description": "Toggle fullscreen on the focused window",
        "icon": "fullscreen",
        "action": "fullscreen-window",
    },
    {
        "id": "toggle-windowed-fullscreen",
        "name": "Toggle Windowed Fullscreen",
        "description": "Toggle windowed (fake) fullscreen",
        "icon": "fullscreen_exit",
        "action": "toggle-windowed-fullscreen",
    },
    {
        "id": "maximize-column",
        "name": "Maximize Column",
        "description": "Toggle the maximized state of the focused column",
        "icon": "crop_square",
        "action": "maximize-column",
    },
    {
        "id": "center-column",
        "name": "Center Column",
        "description": "Center the focused column on the screen",
        "icon": "center_focus_strong",
        "action": "center-column",
    },
    {
        "id": "center-window",
        "name": "Center Window",
        "description": "Center the focused window on the screen",
        "icon": "filter_center_focus",
        "action": "center-window",
    },
    # Floating
    {
        "id": "toggle-floating",
        "name": "Toggle Floating",
        "description": "Move window between floating and tiling layout",
        "icon": "picture_in_picture",
        "action": "toggle-window-floating",
    },
    {
        "id": "focus-floating",
        "name": "Focus Floating",
        "description": "Switch focus to the floating layout",
        "icon": "layers",
        "action": "focus-floating",
    },
    {
        "id": "focus-tiling",
        "name": "Focus Tiling",
        "description": "Switch focus to the tiling layout",
        "icon": "grid_view",
        "action": "focus-tiling",
    },
    {
        "id": "switch-floating-tiling",
        "name": "Switch Floating/Tiling Focus",
        "description": "Toggle focus between floating and tiling layout",
        "icon": "swap_horiz",
        "action": "switch-focus-between-floating-and-tiling",
    },
    # Column Management
    {
        "id": "toggle-tabbed",
        "name": "Toggle Tabbed Display",
        "description": "Toggle column between normal and tabbed display",
        "icon": "tab",
        "action": "toggle-column-tabbed-display",
    },
    {
        "id": "consume-window",
        "name": "Consume Window Into Column",
        "description": "Consume the window to the right into the focused column",
        "icon": "vertical_align_center",
        "action": "consume-window-into-column",
    },
    {
        "id": "expel-window",
        "name": "Expel Window From Column",
        "description": "Expel the focused window from the column",
        "icon": "open_in_new",
        "action": "expel-window-from-column",
    },
    {
        "id": "expand-column",
        "name": "Expand Column to Available Width",
        "description": "Expand the focused column to available space",
        "icon": "expand",
        "action": "expand-column-to-available-width",
    },
    # Focus Navigation - Columns
    {
        "id": "focus-column-left",
        "name": "Focus Column Left",
        "description": "Focus the column to the left",
        "icon": "west",
        "action": "focus-column-left",
    },
    {
        "id": "focus-column-right",
        "name": "Focus Column Right",
        "description": "Focus the column to the right",
        "icon": "east",
        "action": "focus-column-right",
    },
    {
        "id": "focus-column-first",
        "name": "Focus First Column",
        "description": "Focus the first column",
        "icon": "first_page",
        "action": "focus-column-first",
    },
    {
        "id": "focus-column-last",
        "name": "Focus Last Column",
        "description": "Focus the last column",
        "icon": "last_page",
        "action": "focus-column-last",
    },
    # Focus Navigation - Windows
    {
        "id": "focus-window-up",
        "name": "Focus Window Up",
        "description": "Focus the window above",
        "icon": "north",
        "action": "focus-window-up",
    },
    {
        "id": "focus-window-down",
        "name": "Focus Window Down",
        "description": "Focus the window below",
        "icon": "south",
        "action": "focus-window-down",
    },
    {
        "id": "focus-window-previous",
        "name": "Focus Previous Window",
        "description": "Focus the previously focused window",
        "icon": "history",
        "action": "focus-window-previous",
    },
    # Move Columns
    {
        "id": "move-column-left",
        "name": "Move Column Left",
        "description": "Move the focused column to the left",
        "icon": "arrow_back",
        "action": "move-column-left",
    },
    {
        "id": "move-column-right",
        "name": "Move Column Right",
        "description": "Move the focused column to the right",
        "icon": "arrow_forward",
        "action": "move-column-right",
    },
    {
        "id": "move-column-first",
        "name": "Move Column to First",
        "description": "Move the focused column to the start of the workspace",
        "icon": "first_page",
        "action": "move-column-to-first",
    },
    {
        "id": "move-column-last",
        "name": "Move Column to Last",
        "description": "Move the focused column to the end of the workspace",
        "icon": "last_page",
        "action": "move-column-to-last",
    },
    # Move Windows
    {
        "id": "move-window-up",
        "name": "Move Window Up",
        "description": "Move the focused window up in a column",
        "icon": "arrow_upward",
        "action": "move-window-up",
    },
    {
        "id": "move-window-down",
        "name": "Move Window Down",
        "description": "Move the focused window down in a column",
        "icon": "arrow_downward",
        "action": "move-window-down",
    },
    # Swap
    {
        "id": "swap-window-left",
        "name": "Swap Window Left",
        "description": "Swap focused window with one to the left",
        "icon": "swap_horiz",
        "action": "swap-window-left",
    },
    {
        "id": "swap-window-right",
        "name": "Swap Window Right",
        "description": "Swap focused window with one to the right",
        "icon": "swap_horiz",
        "action": "swap-window-right",
    },
    # Workspace Navigation
    {
        "id": "focus-workspace-up",
        "name": "Focus Workspace Up",
        "description": "Focus the workspace above",
        "icon": "arrow_upward",
        "action": "focus-workspace-up",
    },
    {
        "id": "focus-workspace-down",
        "name": "Focus Workspace Down",
        "description": "Focus the workspace below",
        "icon": "arrow_downward",
        "action": "focus-workspace-down",
    },
    {
        "id": "focus-workspace-previous",
        "name": "Focus Previous Workspace",
        "description": "Focus the previous workspace",
        "icon": "history",
        "action": "focus-workspace-previous",
    },
    # Move to Workspace
    {
        "id": "move-window-workspace-up",
        "name": "Move Window to Workspace Up",
        "description": "Move the focused window to the workspace above",
        "icon": "upload",
        "action": "move-window-to-workspace-up",
    },
    {
        "id": "move-window-workspace-down",
        "name": "Move Window to Workspace Down",
        "description": "Move the focused window to the workspace below",
        "icon": "download",
        "action": "move-window-to-workspace-down",
    },
    {
        "id": "move-column-workspace-up",
        "name": "Move Column to Workspace Up",
        "description": "Move the focused column to the workspace above",
        "icon": "upload",
        "action": "move-column-to-workspace-up",
    },
    {
        "id": "move-column-workspace-down",
        "name": "Move Column to Workspace Down",
        "description": "Move the focused column to the workspace below",
        "icon": "download",
        "action": "move-column-to-workspace-down",
    },
    # Move Workspace
    {
        "id": "move-workspace-up",
        "name": "Move Workspace Up",
        "description": "Move the focused workspace up",
        "icon": "move_up",
        "action": "move-workspace-up",
    },
    {
        "id": "move-workspace-down",
        "name": "Move Workspace Down",
        "description": "Move the focused workspace down",
        "icon": "move_down",
        "action": "move-workspace-down",
    },
    # Monitor Navigation
    {
        "id": "focus-monitor-left",
        "name": "Focus Monitor Left",
        "description": "Focus the monitor to the left",
        "icon": "desktop_windows",
        "action": "focus-monitor-left",
    },
    {
        "id": "focus-monitor-right",
        "name": "Focus Monitor Right",
        "description": "Focus the monitor to the right",
        "icon": "desktop_windows",
        "action": "focus-monitor-right",
    },
    {
        "id": "focus-monitor-up",
        "name": "Focus Monitor Up",
        "description": "Focus the monitor above",
        "icon": "desktop_windows",
        "action": "focus-monitor-up",
    },
    {
        "id": "focus-monitor-down",
        "name": "Focus Monitor Down",
        "description": "Focus the monitor below",
        "icon": "desktop_windows",
        "action": "focus-monitor-down",
    },
    {
        "id": "focus-monitor-next",
        "name": "Focus Next Monitor",
        "description": "Focus the next monitor",
        "icon": "arrow_forward",
        "action": "focus-monitor-next",
    },
    {
        "id": "focus-monitor-previous",
        "name": "Focus Previous Monitor",
        "description": "Focus the previous monitor",
        "icon": "arrow_back",
        "action": "focus-monitor-previous",
    },
    # Move to Monitor
    {
        "id": "move-window-monitor-left",
        "name": "Move Window to Monitor Left",
        "description": "Move the focused window to the monitor to the left",
        "icon": "drive_file_move",
        "action": "move-window-to-monitor-left",
    },
    {
        "id": "move-window-monitor-right",
        "name": "Move Window to Monitor Right",
        "description": "Move the focused window to the monitor to the right",
        "icon": "drive_file_move",
        "action": "move-window-to-monitor-right",
    },
    {
        "id": "move-column-monitor-left",
        "name": "Move Column to Monitor Left",
        "description": "Move the focused column to the monitor to the left",
        "icon": "drive_file_move",
        "action": "move-column-to-monitor-left",
    },
    {
        "id": "move-column-monitor-right",
        "name": "Move Column to Monitor Right",
        "description": "Move the focused column to the monitor to the right",
        "icon": "drive_file_move",
        "action": "move-column-to-monitor-right",
    },
    # Width/Height
    {
        "id": "switch-preset-column-width",
        "name": "Switch Preset Column Width",
        "description": "Switch between preset column widths",
        "icon": "width",
        "action": "switch-preset-column-width",
    },
    {
        "id": "switch-preset-window-height",
        "name": "Switch Preset Window Height",
        "description": "Switch between preset window heights",
        "icon": "height",
        "action": "switch-preset-window-height",
    },
    {
        "id": "reset-window-height",
        "name": "Reset Window Height",
        "description": "Reset the height of the focused window back to automatic",
        "icon": "restart_alt",
        "action": "reset-window-height",
    },
    # Misc
    {
        "id": "toggle-overview",
        "name": "Toggle Overview",
        "description": "Toggle (open/close) the Overview",
        "icon": "view_comfy_alt",
        "action": "toggle-overview",
    },
    {
        "id": "screenshot",
        "name": "Screenshot",
        "description": "Open the screenshot UI",
        "icon": "screenshot",
        "action": "screenshot",
    },
    {
        "id": "screenshot-screen",
        "name": "Screenshot Screen",
        "description": "Screenshot the focused screen",
        "icon": "screenshot_monitor",
        "action": "screenshot-screen",
    },
    {
        "id": "screenshot-window",
        "name": "Screenshot Window",
        "description": "Screenshot the focused window",
        "icon": "screenshot",
        "action": "screenshot-window",
    },
    {
        "id": "show-hotkey-overlay",
        "name": "Show Hotkey Overlay",
        "description": "Show the hotkey overlay",
        "icon": "keyboard",
        "action": "show-hotkey-overlay",
    },
    {
        "id": "switch-layout",
        "name": "Switch Keyboard Layout",
        "description": "Switch between keyboard layouts",
        "icon": "language",
        "action": "switch-layout",
    },
    {
        "id": "power-off-monitors",
        "name": "Power Off Monitors",
        "description": "Power off all monitors via DPMS",
        "icon": "power_settings_new",
        "action": "power-off-monitors",
    },
    {
        "id": "power-on-monitors",
        "name": "Power On Monitors",
        "description": "Power on all monitors via DPMS",
        "icon": "power",
        "action": "power-on-monitors",
    },
]


def get_windows() -> list[dict]:
    try:
        result = subprocess.run(
            ["niri", "msg", "--json", "windows"],
            capture_output=True,
            text=True,
            check=True,
        )
        windows = json.loads(result.stdout)
        windows.sort(
            key=lambda w: (
                0 if w.get("is_focused") else 1,
                -(w.get("focus_timestamp") or {}).get("secs", 0),
            )
        )
        return windows
    except (subprocess.CalledProcessError, FileNotFoundError, json.JSONDecodeError):
        return []


def get_workspaces() -> list[dict]:
    try:
        result = subprocess.run(
            ["niri", "msg", "--json", "workspaces"],
            capture_output=True,
            text=True,
            check=True,
        )
        workspaces = json.loads(result.stdout)
        workspaces.sort(key=lambda w: (w.get("output", ""), w.get("idx", 0)))
        return workspaces
    except (subprocess.CalledProcessError, FileNotFoundError, json.JSONDecodeError):
        return []


def window_to_result(window: dict, workspaces: list[dict] | None = None) -> dict:
    window_id = window.get("id", 0)
    title = window.get("title", "")
    app_id = window.get("app_id", "")
    workspace_id = window.get("workspace_id", 0)
    is_floating = window.get("is_floating", False)

    description = app_id
    if workspace_id:
        description = f"{app_id} (workspace {workspace_id})"
    if is_floating:
        description += " [floating]"

    actions = [
        {"id": "close", "name": "Close Window", "icon": "close"},
    ]

    if workspaces:
        for ws in workspaces:
            ws_id = ws.get("id", 0)
            ws_idx = ws.get("idx", 0)
            ws_output = ws.get("output", "")
            if ws_id != workspace_id and ws_id > 0:
                actions.append(
                    {
                        "id": f"move:{ws_idx}",
                        "name": f"Move to Workspace {ws_idx} ({ws_output})",
                        "icon": "drive_file_move",
                    }
                )

    return {
        "id": f"window:{window_id}",
        "name": title or app_id,
        "description": description,
        "icon": app_id,
        "iconType": "system",
        "verb": "Focus",
        "actions": actions,
    }


def action_to_result(action: dict) -> dict:
    return {
        "id": f"action:{action['id']}",
        "name": action["name"],
        "description": action["description"],
        "icon": action["icon"],
        "verb": "Run",
    }


def focus_window(window_id: int) -> tuple[bool, str]:
    try:
        subprocess.run(
            ["niri", "msg", "action", "focus-window", "--id", str(window_id)],
            check=True,
            capture_output=True,
        )
        return True, "Window focused"
    except subprocess.CalledProcessError:
        return False, f"Failed to focus window {window_id}"


def close_window(window_id: int) -> tuple[bool, str]:
    try:
        subprocess.run(
            ["niri", "msg", "action", "focus-window", "--id", str(window_id)],
            check=True,
            capture_output=True,
        )
        subprocess.run(
            ["niri", "msg", "action", "close-window"],
            check=True,
            capture_output=True,
        )
        return True, "Window closed"
    except subprocess.CalledProcessError:
        return False, f"Failed to close window {window_id}"


def move_window_to_workspace(window_id: int, workspace_idx: int) -> tuple[bool, str]:
    try:
        subprocess.run(
            ["niri", "msg", "action", "focus-window", "--id", str(window_id)],
            check=True,
            capture_output=True,
        )
        subprocess.run(
            ["niri", "msg", "action", "move-window-to-workspace", str(workspace_idx)],
            check=True,
            capture_output=True,
        )
        return True, f"Moved to workspace {workspace_idx}"
    except subprocess.CalledProcessError:
        return False, f"Failed to move window to workspace {workspace_idx}"


def execute_action(action: dict) -> tuple[bool, str]:
    action_name = action.get("action", "")

    try:
        subprocess.run(
            ["niri", "msg", "action", action_name],
            check=True,
            capture_output=True,
        )
        return True, f"{action.get('name', action_name)} executed"
    except subprocess.CalledProcessError as e:
        return False, f"Failed to execute {action_name}: {e}"


def get_common_commands() -> list[dict]:
    common_ids = [
        "close-window",
        "fullscreen-window",
        "maximize-column",
        "toggle-floating",
        "center-column",
        "toggle-tabbed",
        "consume-window",
        "expel-window",
        "focus-window-previous",
        "toggle-overview",
        "screenshot",
    ]

    commands = []
    for action in NIRI_ACTIONS:
        if action["id"] in common_ids:
            commands.append(action_to_result(action))
    return commands


# Create plugin instance
plugin = HamrPlugin(
    id="niri",
    name="Niri",
    description="Window management and Niri actions",
    icon="desktop_windows",
)

# Plugin state
state = {
    "current_query": "",
}


@plugin.on_initial
async def handle_initial(params=None):
    """Handle initial request when plugin is opened."""
    windows = get_windows()
    workspaces = get_workspaces()
    results = [window_to_result(w, workspaces) for w in windows]
    results.extend(get_common_commands())
    return HamrPlugin.results(
        results,
        placeholder="Filter windows or type a command...",
        input_mode="realtime",
    )


@plugin.on_search
async def handle_search(query: str, context=None):
    """Handle search request."""
    state["current_query"] = query
    query_lower = query.lower()
    results = []

    filtered_actions = [
        a
        for a in NIRI_ACTIONS
        if query_lower in a["name"].lower()
        or query_lower in a["description"].lower()
        or query_lower in a["id"].lower()
    ]
    results.extend([action_to_result(a) for a in filtered_actions])

    windows = get_windows()
    filtered_windows = [
        w
        for w in windows
        if query_lower in w.get("title", "").lower()
        or query_lower in w.get("app_id", "").lower()
    ]
    workspaces = get_workspaces()
    results.extend([window_to_result(w, workspaces) for w in filtered_windows])

    if not results:
        results = [
            {
                "id": "__empty__",
                "name": f"No matches for '{query}'",
                "icon": "search_off",
                "description": "Try 'fullscreen', 'floating', 'center'...",
            }
        ]
    return HamrPlugin.results(
        results,
        input_mode="realtime",
    )


@plugin.on_action
async def handle_action(item_id: str, action: str = None, context: str = None):
    """Handle action request."""
    if item_id == "__empty__":
        return {"type": "execute", "close": True}

    if item_id.startswith("action:"):
        action_id_full = item_id.replace("action:", "")

        if action_id_full.startswith("goto-workspace:"):
            ws_idx = action_id_full.split(":")[1]
            cmd = ["niri", "msg", "action", "focus-workspace", ws_idx]
            name = f"Go to Workspace {ws_idx}"
            try:
                subprocess.run(cmd, check=True, capture_output=True)
                return {"type": "execute", "close": True, "notify": f"{name} executed"}
            except subprocess.CalledProcessError as e:
                return {"type": "error", "message": f"Failed: {e}"}

        if action_id_full.startswith("move-to-workspace:"):
            ws_idx = action_id_full.split(":")[1]
            cmd = ["niri", "msg", "action", "move-window-to-workspace", ws_idx]
            name = f"Move to Workspace {ws_idx}"
            try:
                subprocess.run(cmd, check=True, capture_output=True)
                return {"type": "execute", "close": True, "notify": f"{name} executed"}
            except subprocess.CalledProcessError as e:
                return {"type": "error", "message": f"Failed: {e}"}

        niri_action = next((a for a in NIRI_ACTIONS if a["id"] == action_id_full), None)
        if not niri_action:
            return {
                "type": "error",
                "message": f"Unknown action: {action_id_full}",
            }

        success, message = execute_action(niri_action)
        if success:
            return {"type": "execute", "close": True, "notify": message}
        else:
            return {"type": "error", "message": message}

    if item_id.startswith("window:"):
        window_id = int(item_id.replace("window:", ""))

        if action == "close":
            success, message = close_window(window_id)
            windows = get_windows()
            workspaces = get_workspaces()
            results = [window_to_result(w, workspaces) for w in windows]
            if not results:
                results = [
                    {"id": "__empty__", "name": "No windows open", "icon": "info"}
                ]
            response = HamrPlugin.results(results)
            if success:
                response["status"] = {"notify": message}
            return response

        if action and action.startswith("move:"):
            workspace_idx = int(action.replace("move:", ""))
            success, message = move_window_to_workspace(window_id, workspace_idx)
            windows = get_windows()
            workspaces = get_workspaces()
            results = [window_to_result(w, workspaces) for w in windows]
            if not results:
                results = [
                    {"id": "__empty__", "name": "No windows open", "icon": "info"}
                ]
            response = HamrPlugin.results(results)
            if success:
                response["status"] = {"notify": message}
            return response

        success, message = focus_window(window_id)
        if success:
            return {"type": "execute", "close": True}
        else:
            return {"type": "error", "message": message}

    return {"type": "error", "message": f"Unknown item: {item_id}"}


@plugin.add_background_task
async def watch_niri_events(p: HamrPlugin):
    """Background task that watches Niri events and sends updates."""
    import asyncio
    import fcntl

    def start_niri_event_stream() -> subprocess.Popen | None:
        """Start niri event-stream subprocess. Returns Popen or None."""
        try:
            proc = subprocess.Popen(
                ["niri", "msg", "-j", "event-stream"],
                stdout=subprocess.PIPE,
                stderr=subprocess.DEVNULL,
                text=True,
                bufsize=1,
            )
            # Set stdout to non-blocking
            if proc.stdout:
                fd = proc.stdout.fileno()
                fl = fcntl.fcntl(fd, fcntl.F_GETFL)
                fcntl.fcntl(fd, fcntl.F_SETFL, fl | os.O_NONBLOCK)
            return proc
        except (OSError, FileNotFoundError):
            return None

    def read_niri_event(proc: subprocess.Popen) -> str | None:
        """Read a single event from niri event stream (non-blocking). Returns event type or None."""
        try:
            if proc.stdout is None:
                return None

            line = proc.stdout.readline()
            if not line:
                return None
            data = json.loads(line)
            keys = list(data.keys())
            return keys[0] if keys else None
        except (json.JSONDecodeError, OSError, IOError, BlockingIOError):
            return None

    WATCH_EVENTS = {"WindowOpenedOrChanged", "WindowClosed"}

    event_proc = start_niri_event_stream()
    last_index_time = 0.0
    pending_index = False

    if event_proc is not None:
        while True:
            now = time.time()
            if pending_index and now - last_index_time >= INDEX_DEBOUNCE_INTERVAL:
                windows = get_windows()
                workspaces = get_workspaces()
                results = [window_to_result(w, workspaces) for w in windows]
                results.extend(get_common_commands())

                await p.send_results(
                    results,
                    placeholder="Filter windows or type a command...",
                    input_mode="realtime",
                )
                last_index_time = now
                pending_index = False

            # Non-blocking check for events
            event = read_niri_event(event_proc)
            if event in WATCH_EVENTS:
                pending_index = True

            # Check if process died
            if event_proc.poll() is not None:
                event_proc = start_niri_event_stream()
                if event_proc is None:
                    break

            # Yield control to other async tasks - this is critical!
            await asyncio.sleep(0.1)


if __name__ == "__main__":
    plugin.run()
