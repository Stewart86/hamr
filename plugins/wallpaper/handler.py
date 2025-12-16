#!/usr/bin/env python3
"""
Wallpaper workflow handler - browse and set wallpapers using the image browser.

Supports multiple wallpaper backends with automatic detection:
1. swww (recommended for Hyprland)
2. hyprctl hyprpaper
3. swaybg
4. feh (X11 fallback)

For theme integration (dark/light mode), place a custom script at:
  ~/.config/hamr/scripts/switchwall.sh

The script will be called with: switchwall.sh --image <path> --mode <dark|light>
"""

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

# Test mode for development
TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

# Default wallpaper directory
PICTURES_DIR = Path.home() / "Pictures"
WALLPAPERS_DIR = PICTURES_DIR / "Wallpapers"

# Switchwall script paths (popular dotfiles first, then hamr fallback)
# Add more dotfile paths here as needed
XDG_CONFIG = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
SWITCHWALL_PATHS = [
    XDG_CONFIG
    / "quickshell"
    / "ii"
    / "scripts"
    / "colors"
    / "switchwall.sh",  # end-4 illogical-impulse
    XDG_CONFIG / "hamr" / "scripts" / "switchwall.sh",  # hamr standalone
]


def find_switchwall_script() -> Path | None:
    """Find switchwall script (popular dotfiles first, then hamr fallback)."""
    for script in SWITCHWALL_PATHS:
        if script.exists() and os.access(script, os.X_OK):
            return script
    return None


def detect_wallpaper_backend() -> str | None:
    """Detect available wallpaper backend."""
    # Check for swww daemon
    if shutil.which("swww"):
        try:
            result = subprocess.run(["swww", "query"], capture_output=True, timeout=2)
            if result.returncode == 0:
                return "swww"
        except (subprocess.TimeoutExpired, FileNotFoundError):
            pass

    # Check for hyprpaper via hyprctl
    if shutil.which("hyprctl"):
        try:
            result = subprocess.run(
                ["hyprctl", "hyprpaper", "listloaded"], capture_output=True, timeout=2
            )
            if result.returncode == 0:
                return "hyprpaper"
        except (subprocess.TimeoutExpired, FileNotFoundError):
            pass

    # Check for swaybg
    if shutil.which("swaybg"):
        return "swaybg"

    # Check for feh (X11)
    if shutil.which("feh"):
        return "feh"

    return None


def build_wallpaper_command(image_path: str, mode: str) -> list[str]:
    """Build command to set wallpaper based on available backend."""
    # First check for switchwall script (user override or bundled)
    custom_script = find_switchwall_script()
    if custom_script:
        return [str(custom_script), "--image", image_path, "--mode", mode]

    # Detect backend
    backend = detect_wallpaper_backend()

    if backend == "swww":
        return [
            "swww",
            "img",
            image_path,
            "--transition-type",
            "fade",
            "--transition-duration",
            "1",
        ]

    if backend == "hyprpaper":
        # hyprpaper requires preload then set
        # We use hyprctl to communicate with hyprpaper
        return [
            "bash",
            "-c",
            f'hyprctl hyprpaper preload "{image_path}" && '
            f'hyprctl hyprpaper wallpaper ",{image_path}"',
        ]

    if backend == "swaybg":
        return ["swaybg", "-i", image_path, "-m", "fill"]

    if backend == "feh":
        return ["feh", "--bg-fill", image_path]

    # No backend found - return notify-send as fallback
    return [
        "notify-send",
        "Wallpaper",
        f"No wallpaper backend found. Install swww, hyprpaper, swaybg, or feh.\n\nSelected: {image_path}",
    ]


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    selected = input_data.get("selected", {})

    # Initial or search: show the image browser
    if step in ("initial", "search"):
        # Determine initial directory
        initial_dir = (
            str(WALLPAPERS_DIR) if WALLPAPERS_DIR.exists() else str(PICTURES_DIR)
        )

        has_custom_script = find_switchwall_script() is not None

        # Build actions - only show dark/light mode if custom script supports it
        if has_custom_script:
            actions = [
                {"id": "set_dark", "name": "Set (Dark Mode)", "icon": "dark_mode"},
                {"id": "set_light", "name": "Set (Light Mode)", "icon": "light_mode"},
            ]
        else:
            # Simple set action when no theming script
            actions = [
                {"id": "set", "name": "Set Wallpaper", "icon": "wallpaper"},
            ]

        print(
            json.dumps(
                {
                    "type": "imageBrowser",
                    "imageBrowser": {
                        "directory": initial_dir,
                        "title": "Select Wallpaper",
                        "actions": actions,
                    },
                }
            )
        )
        return

    if step == "action" and selected.get("id") == "imageBrowser":
        file_path = selected.get("path", "")
        action_id = selected.get("action", "set")

        if not file_path:
            print(json.dumps({"type": "error", "message": "No file selected"}))
            return

        # Determine mode based on action
        if action_id == "set_light":
            mode = "light"
        else:
            mode = "dark"  # default

        # Build command to set wallpaper
        command = build_wallpaper_command(file_path, mode)
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
