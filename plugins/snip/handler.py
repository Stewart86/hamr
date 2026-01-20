#!/usr/bin/env python3
"""
Screenshot Snip plugin for hamr - Take screenshots with region selection.

Socket-based plugin using grim, slurp, and satty for screenshot annotation.
"""

import asyncio
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

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


def action_to_result(action: dict) -> dict:
    """Convert action to result format."""
    return {
        "id": action["id"],
        "name": action["name"],
        "description": action["description"],
        "icon": action["icon"],
        "verb": "Take",
    }


plugin = HamrPlugin(
    id="snip",
    name="Screenshot Snip",
    description="Take screenshot with region selection and annotation",
    icon="screenshot",
)


@plugin.on_initial
def handle_initial(params=None):
    """Handle initial request."""
    results = [action_to_result(a) for a in ACTIONS]
    return HamrPlugin.results(results, placeholder="Search screenshot actions...")


@plugin.on_search
def handle_search(query: str, context: str | None):
    """Handle search request."""
    query_lower = query.lower()
    filtered = [
        a
        for a in ACTIONS
        if query_lower in a["id"]
        or query_lower in a["name"].lower()
        or query_lower in a["description"].lower()
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

    return HamrPlugin.results(results)


@plugin.on_action
async def handle_action(item_id: str, action: str | None, context: str | None):
    """Handle action request."""
    if item_id == "__empty__":
        return HamrPlugin.close()

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
