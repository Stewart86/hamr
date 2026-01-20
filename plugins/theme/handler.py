#!/usr/bin/env python3
"""
Theme plugin for hamr - Switch between light and dark mode.

Socket-based daemon plugin that provides theme switching capabilities.
"""

import asyncio
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

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
    """Find the color switching script."""
    for path in SCRIPT_PATHS:
        if path.is_file() and path.stat().st_mode & 0o111:
            return str(path)
    return None


def action_to_result(action: dict) -> dict:
    """Convert action to result format."""
    return {
        "id": action["id"],
        "name": action["name"],
        "description": action["description"],
        "icon": action["icon"],
        "verb": "Switch",
    }


plugin = HamrPlugin(
    id="theme",
    name="Theme",
    description="Switch between light and dark mode",
    icon="contrast",
)


@plugin.on_initial
def handle_initial(params=None):
    """Handle initial request."""
    results = [action_to_result(a) for a in ACTIONS]
    return HamrPlugin.results(results, placeholder="Search theme...")


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
                "name": f"No themes matching '{query}'",
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

    # Try to use script if available
    script = find_script()
    if script:
        await asyncio.to_thread(
            subprocess.Popen,
            [script, "--mode", selected_action["mode"], "--noswitch"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
    else:
        # Fallback to gsettings
        color_scheme = (
            "prefer-light" if selected_action["mode"] == "light" else "prefer-dark"
        )
        await asyncio.to_thread(
            subprocess.Popen,
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

    return HamrPlugin.execute(close=True)


if __name__ == "__main__":
    plugin.run()
