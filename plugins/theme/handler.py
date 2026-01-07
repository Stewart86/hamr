#!/usr/bin/env python3
"""
Theme plugin handler - switch between light and dark mode.
"""

import json
import os
import select
import signal
import subprocess
import sys
from pathlib import Path

SCRIPT_PATHS = [
    Path.home() / ".config/hamr/scripts/colors/switchwall.sh",
    Path.home() / ".config/quickshell/scripts/colors/switchwall.sh",
]

ACTIONS = [
    {
        "id": "light",
        "name": "Light Mode",
        "description": "Switch to light color scheme",
        "icon": "light_mode",
        "mode": "light",
        "notify": "Light mode activated",
    },
    {
        "id": "dark",
        "name": "Dark Mode",
        "description": "Switch to dark color scheme",
        "icon": "dark_mode",
        "mode": "dark",
        "notify": "Dark mode activated",
    },
]


def find_script() -> str | None:
    for path in SCRIPT_PATHS:
        if path.is_file() and os.access(path, os.X_OK):
            return str(path)
    return None


def action_to_index_item(action: dict) -> dict:
    return {
        "id": action["id"],
        "name": action["name"],
        "description": action["description"],
        "icon": action["icon"],
        "verb": "Switch",
        "keywords": [action["id"], "theme", "mode"],
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
        "verb": "Switch",
    }


def get_index_items() -> list[dict]:
    return [action_to_index_item(a) for a in ACTIONS]


def handle_request(request: dict) -> None:
    step = request.get("step", "initial")
    query = request.get("query", "").strip().lower()
    selected = request.get("selected", {})

    if step == "index":
        items = get_index_items()
        print(json.dumps({"type": "index", "mode": "full", "items": items}), flush=True)
        return

    if step == "initial":
        results = [action_to_result(a) for a in ACTIONS]
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "placeholder": "Search theme...",
                    "inputMode": "realtime",
                }
            ),
            flush=True,
        )
        return

    if step == "search":
        filtered = [
            a
            for a in ACTIONS
            if query in a["id"]
            or query in a["name"].lower()
            or query in a["description"].lower()
        ]
        results = [action_to_result(a) for a in filtered]
        if not results:
            results = [
                {
                    "id": "__empty__",
                    "name": f"No themes matching '{query}'",
                    "icon": "search_off",
                }
            ]
        print(
            json.dumps(
                {"type": "results", "results": results, "inputMode": "realtime"}
            ),
            flush=True,
        )
        return

    if step == "action":
        selected_id = selected.get("id", "")

        if selected_id == "__empty__":
            print(json.dumps({"type": "execute", "close": True}), flush=True)
            return

        action = next((a for a in ACTIONS if a["id"] == selected_id), None)
        if not action:
            print(
                json.dumps(
                    {"type": "error", "message": f"Unknown action: {selected_id}"}
                ),
                flush=True,
            )
            return

        script = find_script()
        if script:
            subprocess.Popen(
                [script, "--mode", action["mode"], "--noswitch"],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                start_new_session=True,
            )
        else:
            color_scheme = (
                "prefer-light" if action["mode"] == "light" else "prefer-dark"
            )
            subprocess.Popen(
                [
                    "gsettings",
                    "set",
                    "org.gnome.desktop.interface",
                    "color-scheme",
                    color_scheme,
                ],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                start_new_session=True,
            )

        print(
            json.dumps(
                {
                    "type": "execute",
                    "name": action["name"],
                    "icon": action["icon"],
                    "notify": action["notify"],
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

    items = get_index_items()
    print(json.dumps({"type": "index", "mode": "full", "items": items}), flush=True)

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
