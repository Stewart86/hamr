#!/usr/bin/env python3
"""
Color Picker plugin handler - pick color from screen.
"""

import json
import select
import signal
import subprocess
import sys

ACTIONS = [
    {
        "id": "pick",
        "name": "Pick Color",
        "description": "Pick a color from screen and copy hex value",
        "icon": "colorize",
        "command": ["hyprpicker", "-a"],
    },
]


def action_to_index_item(action: dict) -> dict:
    return {
        "id": action["id"],
        "name": action["name"],
        "description": action["description"],
        "icon": action["icon"],
        "verb": "Pick",
        "keywords": ["color", "pick", "picker", "eyedropper", "hex", "rgb"],
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
        "verb": "Pick",
    }


def get_index_items() -> list[dict]:
    return [action_to_index_item(a) for a in ACTIONS]


def handle_request(request: dict) -> None:
    step = request.get("step", "initial")
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
                    "placeholder": "Pick a color...",
                    "inputMode": "realtime",
                }
            ),
            flush=True,
        )
        return

    if step == "search":
        results = [action_to_result(a) for a in ACTIONS]
        print(
            json.dumps(
                {"type": "results", "results": results, "inputMode": "realtime"}
            ),
            flush=True,
        )
        return

    if step == "action":
        selected_id = selected.get("id", "")

        action = next((a for a in ACTIONS if a["id"] == selected_id), None)
        if not action:
            print(
                json.dumps(
                    {"type": "error", "message": f"Unknown action: {selected_id}"}
                ),
                flush=True,
            )
            return

        subprocess.Popen(
            action["command"],
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
