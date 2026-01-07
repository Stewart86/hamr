#!/usr/bin/env python3
"""
Accent Color plugin handler - set system accent color.

Executes color switching script with selected color.
"""

import json
import os
import select
import signal
import subprocess
import sys
from pathlib import Path

COLORS = [
    {"id": "#FF5252", "name": "Red"},
    {"id": "#FF4081", "name": "Pink"},
    {"id": "#E040FB", "name": "Purple"},
    {"id": "#7C4DFF", "name": "Deep Purple"},
    {"id": "#536DFE", "name": "Indigo"},
    {"id": "#448AFF", "name": "Blue"},
    {"id": "#40C4FF", "name": "Light Blue"},
    {"id": "#18FFFF", "name": "Cyan"},
    {"id": "#64FFDA", "name": "Teal"},
    {"id": "#69F0AE", "name": "Green"},
    {"id": "#B2FF59", "name": "Light Green"},
    {"id": "#EEFF41", "name": "Lime"},
    {"id": "#FFFF00", "name": "Yellow"},
    {"id": "#FFD740", "name": "Amber"},
    {"id": "#FFAB40", "name": "Orange"},
    {"id": "#FF6E40", "name": "Deep Orange"},
]

SCRIPT_PATHS = [
    Path.home() / ".config/hamr/scripts/colors/switchwall.sh",
    Path.home() / ".config/quickshell/scripts/colors/switchwall.sh",
]


def find_script() -> str | None:
    for path in SCRIPT_PATHS:
        if path.is_file() and os.access(path, os.X_OK):
            return str(path)
    return None


def color_to_index_item(color: dict) -> dict:
    return {
        "id": color["id"],
        "name": f"Accent: {color['name']}",
        "description": f"Set accent color to {color['id']}",
        "icon": "palette",
        "verb": "Set",
        "keywords": ["accent", "color", color["name"].lower(), color["id"].lower()],
        "entryPoint": {
            "step": "action",
            "selected": {"id": color["id"]},
        },
    }


def color_to_result(color: dict) -> dict:
    return {
        "id": color["id"],
        "name": f"Accent: {color['name']}",
        "description": f"Set accent color to {color['id']}",
        "icon": "palette",
        "verb": "Set",
    }


def get_index_items() -> list[dict]:
    return [color_to_index_item(c) for c in COLORS]


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
        results = [color_to_result(c) for c in COLORS]
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "placeholder": "Search colors...",
                    "inputMode": "realtime",
                }
            ),
            flush=True,
        )
        return

    if step == "search":
        filtered = [
            c
            for c in COLORS
            if query in c["id"].lower()
            or query in c["name"].lower()
            or query in "accent"
            or query in "color"
        ]
        results = [color_to_result(c) for c in filtered]
        if not results:
            results = [
                {
                    "id": "__empty__",
                    "name": f"No colors matching '{query}'",
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

        color = next((c for c in COLORS if c["id"] == selected_id), None)
        if not color:
            print(
                json.dumps(
                    {"type": "error", "message": f"Unknown color: {selected_id}"}
                ),
                flush=True,
            )
            return

        script = find_script()
        if not script:
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "notify": "Color script not found. Install switchwall.sh to ~/.config/hamr/scripts/colors/",
                        "close": True,
                    }
                ),
                flush=True,
            )
            return

        subprocess.Popen(
            [script, "--noswitch", "--color", color["id"]],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )

        print(
            json.dumps(
                {
                    "type": "execute",
                    "name": f"Set {color['name']}",
                    "icon": "palette",
                    "notify": f"Accent color set to {color['name']}",
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
