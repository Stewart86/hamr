#!/usr/bin/env python3
"""
Accent Color plugin for hamr - Set system accent color.

Socket-based plugin for switching between predefined accent colors.
"""

import asyncio
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

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
    """Find the color switching script."""
    for path in SCRIPT_PATHS:
        if path.is_file() and path.stat().st_mode & 0o111:
            return str(path)
    return None


def color_to_result(color: dict) -> dict:
    """Convert color to result format."""
    return {
        "id": color["id"],
        "name": f"Accent: {color['name']}",
        "description": f"Set accent color to {color['id']}",
        "icon": "palette",
        "verb": "Set",
    }


plugin = HamrPlugin(
    id="accentcolor",
    name="Accent Color",
    description="Set system accent color",
    icon="palette",
)


@plugin.on_initial
def handle_initial(params=None):
    """Handle initial request."""
    results = [color_to_result(c) for c in COLORS]
    return HamrPlugin.results(results, placeholder="Search colors...")


@plugin.on_search
def handle_search(query: str, context: str | None):
    """Handle search request."""
    query_lower = query.lower()
    filtered = [
        c
        for c in COLORS
        if query_lower in c["id"].lower()
        or query_lower in c["name"].lower()
        or query_lower in "accent"
        or query_lower in "color"
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

    return HamrPlugin.results(results)


@plugin.on_action
async def handle_action(item_id: str, action: str | None, context: str | None):
    """Handle action request."""
    if item_id == "__empty__":
        return HamrPlugin.close()

    color = next((c for c in COLORS if c["id"] == item_id), None)
    if not color:
        return HamrPlugin.error(f"Unknown color: {item_id}")

    script = find_script()
    if not script:
        return HamrPlugin.execute(close=True)

    await asyncio.to_thread(
        subprocess.Popen,
        [script, "--noswitch", "--color", color["id"]],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )

    return HamrPlugin.execute(close=True)


if __name__ == "__main__":
    plugin.run()
