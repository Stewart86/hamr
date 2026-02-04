#!/usr/bin/env python3
"""
Power plugin handler - system power and session controls.

Provides shutdown, restart, suspend, logout, lock, and Hyprland reload.
"""

import json
import os
import select
import signal
import subprocess
import sys
from pathlib import Path

# Add parent directory to path to import SDK
sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

POWER_ACTIONS = [
    {
        "id": "shutdown",
        "name": "Shutdown",
        "description": "Power off the system",
        "icon": "power_settings_new",
        "command": ["systemctl", "poweroff"],
        "confirm": True,
    },
    {
        "id": "restart",
        "name": "Restart",
        "description": "Reboot the system",
        "icon": "restart_alt",
        "command": ["systemctl", "reboot"],
        "confirm": True,
    },
    {
        "id": "suspend",
        "name": "Suspend",
        "description": "Suspend to RAM",
        "icon": "bedtime",
        "command": ["systemctl", "suspend"],
    },
    {
        "id": "hibernate",
        "name": "Hibernate",
        "description": "Suspend to disk",
        "icon": "downloading",
        "command": ["systemctl", "hibernate"],
    },
    {
        "id": "lock",
        "name": "Lock Screen",
        "description": "Lock the session",
        "icon": "lock",
        "command": ["loginctl", "lock-session"],
    },
    {
        "id": "logout",
        "name": "Log Out",
        "description": "End the current session",
        "icon": "logout",
        "command": ["loginctl", "terminate-user", os.environ.get("USER", "")],
        "confirm": True,
    },
    {
        "id": "reload-hyprland",
        "name": "Reload Hyprland",
        "description": "Reload Hyprland configuration",
        "icon": "refresh",
        "command": [
            "bash",
            "-c",
            "hyprctl reload && notify-send 'Hyprland' 'Configuration reloaded'",
        ],
    },
    {
        "id": "reload-niri",
        "name": "Reload Niri",
        "description": "Reload Niri configuration",
        "icon": "refresh",
        "command": [
            "bash",
            "-c",
            "niri msg action load-config-file && notify-send 'Niri' 'Configuration reloaded'",
        ],
    },
    {
        "id": "reload-hamr",
        "name": "Reload Hamr",
        "description": "Restart Hamr launcher",
        "icon": "sync",
        "command": [
            "bash",
            "-c",
            "if command -v systemctl >/dev/null 2>&1 && systemctl --user is-enabled --quiet hamr-daemon 2>/dev/null; then systemctl --user restart hamr-daemon hamr-gtk; else pkill -x hamr-gtk 2>/dev/null || true; hamr shutdown 2>/dev/null || true; sleep 0.2; nohup hamr >/dev/null 2>&1 & fi; notify-send 'Hamr' 'Launcher restarted'",
        ],
    },
]


def action_to_index_item(action: dict) -> dict:
    return {
        "id": action["id"],  # Use simple id (matches result IDs for frecency)
        "name": action["name"],
        "description": action["description"],
        "icon": action["icon"],
        "verb": "Run",
        "keywords": [action["id"], action["name"].lower()],
        "entryPoint": {
            "step": "action",
            "selected": {"id": action["id"]},
        },
    }


def action_to_result(action: dict) -> dict:
    return {
        "id": action["id"],
        "name": action["name"],
        "description": action["description"],
        "icon": action["icon"],
        "verb": "Run",
    }


def get_index_items() -> list[dict]:
    return [action_to_index_item(a) for a in POWER_ACTIONS]


def handle_request(request: dict) -> None:
    step = request.get("step", "initial")
    query = request.get("query", "").strip().lower()
    selected = request.get("selected", {})

    if step == "index":
        items = get_index_items()
        print(
            json.dumps({"type": "index", "mode": "full", "items": items}),
            flush=True,
        )
        return

    if step == "initial":
        results = [action_to_result(a) for a in POWER_ACTIONS]
        print(
            json.dumps(
                HamrPlugin.results(
                    results,
                    placeholder="Search power actions...",
                    input_mode="realtime",
                )
            ),
            flush=True,
        )
        return

    if step == "search":
        filtered = [
            a
            for a in POWER_ACTIONS
            if query in a["id"]
            or query in a["name"].lower()
            or query in a["description"].lower()
        ]
        results = [action_to_result(a) for a in filtered]
        if not results:
            results = [
                {
                    "id": "__empty__",
                    "name": f"No actions matching '{query}'",
                    "icon": "search_off",
                }
            ]
        print(
            json.dumps(HamrPlugin.results(results, input_mode="realtime")), flush=True
        )
        return

    if step == "action":
        selected_id = selected.get("id", "")

        if selected_id == "__empty__":
            print(json.dumps(HamrPlugin.close()), flush=True)
            return

        action = next((a for a in POWER_ACTIONS if a["id"] == selected_id), None)
        if not action:
            print(
                json.dumps(HamrPlugin.error(f"Unknown action: {selected_id}")),
                flush=True,
            )
            return

        subprocess.Popen(
            action["command"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
        response = HamrPlugin.close()
        response["name"] = action["name"]
        response["icon"] = action["icon"]
        print(json.dumps(response), flush=True)
        return

    print(json.dumps(HamrPlugin.error(f"Unknown step: {step}")), flush=True)


def main():
    signal.signal(signal.SIGTERM, lambda s, f: sys.exit(0))
    signal.signal(signal.SIGINT, lambda s, f: sys.exit(0))

    items = get_index_items()
    print(
        json.dumps({"type": "index", "mode": "full", "items": items}),
        flush=True,
    )

    while True:
        readable, _, _ = select.select([sys.stdin], [], [], 1.0)
        if readable:
            line = sys.stdin.readline()
            if not line:
                break
            request = json.loads(line.strip())
            handle_request(request)


if __name__ == "__main__":
    main()
