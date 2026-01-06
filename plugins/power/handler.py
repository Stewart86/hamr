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

TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

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
            "systemd-run",
            "--user",
            "--no-block",
            "bash",
            "-c",
            "if systemctl --user is-active --quiet hamr.service; then systemctl --user restart hamr.service; else qs kill -c hamr; qs -c hamr -d; fi; for i in $(seq 1 20); do qs ipc -c hamr call hamr open 2>/dev/null && break; sleep 0.1; done; notify-send 'Hamr' 'Launcher restarted'",
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
                {
                    "type": "results",
                    "results": results,
                    "placeholder": "Search power actions...",
                    "inputMode": "realtime",
                }
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
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                }
            ),
            flush=True,
        )
        return

    if step == "action":
        selected_id = selected.get("id", "")

        if selected_id == "__empty__":
            print(json.dumps({"type": "execute", "close": True}), flush=True)
            return

        action = next((a for a in POWER_ACTIONS if a["id"] == selected_id), None)
        if not action:
            print(
                json.dumps(
                    {"type": "error", "message": f"Unknown action: {selected_id}"}
                ),
                flush=True,
            )
            return

        # Execute the command in the handler
        if not TEST_MODE:
            subprocess.Popen(
                action["command"],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                start_new_session=True,
            )

        # Return safe API response (just close, command already executed)
        print(
            json.dumps(
                {
                    "type": "execute",
                    "name": action["name"],
                    "icon": action["icon"],
                    "close": True,
                }
            ),
            flush=True,
        )
        return

    print(json.dumps({"type": "error", "message": f"Unknown step: {step}"}), flush=True)


def main():
    signal.signal(signal.SIGTERM, lambda s, f: sys.exit(0))
    signal.signal(signal.SIGINT, lambda s, f: sys.exit(0))

    # Emit full index on startup (skip in test mode - tests use explicit index step)
    if not TEST_MODE:
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
