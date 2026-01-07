#!/usr/bin/env python3
"""
Screenshot Snip plugin handler - take screenshots with region selection.
"""

import json
import os
import select
import signal
import subprocess
import sys
from datetime import datetime
from pathlib import Path

ACTIONS = [
    {
        "id": "snip",
        "name": "Screenshot Snip",
        "description": "Select region and annotate with satty",
        "icon": "screenshot",
        "command": ["bash", "-c", 'grim -g "$(slurp)" - | satty -f -'],
    },
    {
        "id": "snip-copy",
        "name": "Screenshot to Clipboard",
        "description": "Select region and copy to clipboard",
        "icon": "content_copy",
        "command": ["bash", "-c", 'grim -g "$(slurp)" - | wl-copy'],
        "notify": "Screenshot copied to clipboard",
    },
    {
        "id": "snip-save",
        "name": "Screenshot to File",
        "description": "Select region and save to Screenshots folder",
        "icon": "save",
        "command": [
            "bash",
            "-c",
            'mkdir -p ~/Pictures/Screenshots && grim -g "$(slurp)" ~/Pictures/Screenshots/screenshot_$(date +%Y-%m-%d_%H.%M.%S).png',
        ],
        "notify": "Screenshot saved to Pictures/Screenshots",
    },
]


def action_to_index_item(action: dict) -> dict:
    return {
        "id": action["id"],
        "name": action["name"],
        "description": action["description"],
        "icon": action["icon"],
        "verb": "Take",
        "keywords": ["screenshot", "snip", "capture"] + action["id"].split("-"),
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
        "verb": "Take",
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
                    "placeholder": "Search screenshot actions...",
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
                    "name": f"No actions matching '{query}'",
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

        subprocess.Popen(
            action["command"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )

        response = {
            "type": "execute",
            "name": action["name"],
            "icon": action["icon"],
            "close": True,
        }
        if action.get("notify"):
            response["notify"] = action["notify"]

        print(json.dumps(response), flush=True)
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
