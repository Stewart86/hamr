#!/usr/bin/env python3
"""
Wallpaper workflow handler - browse and set wallpapers using the image browser.
Demonstrates the imageBrowser response type for rich image selection UI.
"""

import json
import os
import sys
from pathlib import Path

# Default wallpaper directory
PICTURES_DIR = Path.home() / "Pictures"
WALLPAPERS_DIR = PICTURES_DIR / "Wallpapers"

# Script paths
QUICKSHELL_CONFIG = Path.home() / ".config" / "quickshell" / "ii"
SWITCHWALL_SCRIPT = QUICKSHELL_CONFIG / "scripts" / "colors" / "switchwall.sh"


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    selected = input_data.get("selected", {})

    # Initial or search: show the image browser
    # The image browser handles its own search, so we just open it
    if step in ("initial", "search"):
        # Open image browser with wallpaper-specific actions
        # inputMode doesn't apply to imageBrowser, but include for consistency
        print(
            json.dumps(
                {
                    "type": "imageBrowser",
                    "imageBrowser": {
                        "directory": str(WALLPAPERS_DIR)
                        if WALLPAPERS_DIR.exists()
                        else str(PICTURES_DIR),
                        "title": "Select Wallpaper",
                        "actions": [
                            {
                                "id": "set_dark",
                                "name": "Set (Dark Mode)",
                                "icon": "dark_mode",
                            },
                            {
                                "id": "set_light",
                                "name": "Set (Light Mode)",
                                "icon": "light_mode",
                            },
                        ],
                    },
                }
            )
        )
        return

    # Handle image browser selection
    if step == "action" and selected.get("id") == "imageBrowser":
        file_path = selected.get("path", "")
        action_id = selected.get("action", "set_dark")

        if not file_path:
            print(json.dumps({"type": "error", "message": "No file selected"}))
            return

        # Determine mode based on action
        mode = "dark" if action_id == "set_dark" else "light"

        # Build command to set wallpaper
        command = [str(SWITCHWALL_SCRIPT), "--image", file_path, "--mode", mode]

        filename = Path(file_path).name
        print(
            json.dumps(
                {
                    "type": "execute",
                    "execute": {
                        "command": command,
                        "name": f"Set wallpaper: {filename}",
                        "icon": "wallpaper",
                        "thumbnail": file_path,
                        "close": True,
                    },
                }
            )
        )
        return

    # Unknown step
    print(json.dumps({"type": "error", "message": f"Unknown step: {step}"}))


if __name__ == "__main__":
    main()
