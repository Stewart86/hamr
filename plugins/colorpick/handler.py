#!/usr/bin/env python3
"""
Color Picker plugin for hamr - Pick color from screen.

Socket-based plugin using hyprpicker to select and copy color values.
"""

import asyncio
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

ACTIONS = [
    {
        "id": "pick",
        "name": "Pick Color",
        "description": "Pick a color from screen and copy hex value",
        "icon": "colorize",
        "command": ["hyprpicker", "-a"],
    },
]


def action_to_result(action: dict) -> dict:
    """Convert action to result format."""
    return {
        "id": action["id"],
        "name": action["name"],
        "description": action["description"],
        "icon": action["icon"],
        "verb": "Pick",
    }


plugin = HamrPlugin(
    id="colorpick",
    name="Color Picker",
    description="Pick a color from screen and copy to clipboard",
    icon="colorize",
)


@plugin.on_initial
def handle_initial(params=None):
    """Handle initial request."""
    results = [action_to_result(a) for a in ACTIONS]
    return HamrPlugin.results(results, placeholder="Pick a color...")


@plugin.on_search
def handle_search(query: str, context: str | None):
    """Handle search request."""
    results = [action_to_result(a) for a in ACTIONS]
    return HamrPlugin.results(results)


@plugin.on_action
async def handle_action(item_id: str, action: str | None, context: str | None):
    """Handle action request."""
    selected_action = next((a for a in ACTIONS if a["id"] == item_id), None)
    if not selected_action:
        return HamrPlugin.error(f"Unknown action: {item_id}")

    await asyncio.to_thread(
        subprocess.Popen,
        selected_action["command"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )

    return HamrPlugin.execute(close=True)


if __name__ == "__main__":
    plugin.run()
