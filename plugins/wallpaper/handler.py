#!/usr/bin/env python3
"""
Wallpaper workflow handler - browse and set wallpapers using the image browser.

Supports multiple wallpaper backends with automatic detection:
1. awww (swww renamed, recommended for Wayland)
2. swww (legacy name)
3. hyprctl hyprpaper
4. swaybg
5. feh (X11 fallback)

For theme integration (dark/light mode), place a custom script at:
  ~/.config/hamr/scripts/switchwall.sh

The script will be called with: switchwall.sh --image <path> --mode <dark|light>
"""

import json
import os
import random
import shutil
import subprocess
import sys
from pathlib import Path

# Test mode for development
TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

# Config and default paths
XDG_CONFIG = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
XDG_CACHE = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache"))
HAMR_CONFIG_PATH = XDG_CONFIG / "hamr" / "config.json"
WALLPAPER_HISTORY_FILE = XDG_CACHE / "hamr" / "wallpaper-history.json"
PICTURES_DIR = Path.home() / "Pictures"
DEFAULT_WALLPAPERS_DIR = PICTURES_DIR / "Wallpapers"
MAX_HISTORY_ITEMS = 10
IMAGE_EXTENSIONS = {".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp"}


def load_wallpaper_history() -> list[str]:
    """Load wallpaper history from cache (most recent first)"""
    if not WALLPAPER_HISTORY_FILE.exists():
        return []
    try:
        return json.loads(WALLPAPER_HISTORY_FILE.read_text())
    except (json.JSONDecodeError, OSError):
        return []


def save_wallpaper_to_history(path: str) -> None:
    """Save wallpaper path to history"""
    if TEST_MODE:
        return
    history = load_wallpaper_history()
    if path in history:
        history.remove(path)
    history.insert(0, path)
    history = history[:MAX_HISTORY_ITEMS]
    try:
        WALLPAPER_HISTORY_FILE.parent.mkdir(parents=True, exist_ok=True)
        WALLPAPER_HISTORY_FILE.write_text(json.dumps(history))
    except OSError:
        pass


def get_random_wallpaper(directory: Path) -> str | None:
    """Get a random wallpaper from the directory"""
    if not directory.exists():
        return None

    wallpapers = [
        f
        for f in directory.iterdir()
        if f.is_file() and f.suffix.lower() in IMAGE_EXTENSIONS
    ]

    if not wallpapers:
        return None

    return str(random.choice(wallpapers))


def get_wallpaper_dir() -> Path:
    """Get wallpaper directory from config or use default."""
    if HAMR_CONFIG_PATH.exists():
        try:
            with open(HAMR_CONFIG_PATH) as f:
                config = json.load(f)
                wallpaper_dir = config.get("paths", {}).get("wallpaperDir", "")
                if wallpaper_dir:
                    expanded = Path(wallpaper_dir).expanduser()
                    if expanded.exists() and expanded.is_dir():
                        return expanded
        except (json.JSONDecodeError, OSError):
            pass

    if DEFAULT_WALLPAPERS_DIR.exists():
        return DEFAULT_WALLPAPERS_DIR
    return PICTURES_DIR


# Switchwall script paths (in order of preference)
SCRIPT_DIR = Path(__file__).parent
HAMR_DIR = SCRIPT_DIR.parent.parent

SWITCHWALL_PATHS = [
    HAMR_DIR / "scripts" / "colors" / "switchwall.sh",  # bundled with hamr
    XDG_CONFIG / "hamr" / "scripts" / "switchwall.sh",  # user override
]


def find_switchwall_script() -> Path | None:
    """Find switchwall script, preferring bundled then user override."""
    for path in SWITCHWALL_PATHS:
        if path.exists() and os.access(path, os.X_OK):
            return path
    return None


def detect_wallpaper_backend() -> str | None:
    """Detect available wallpaper backend."""
    # Check for awww (swww renamed to awww)
    if shutil.which("awww"):
        try:
            result = subprocess.run(["awww", "query"], capture_output=True, timeout=2)
            if result.returncode == 0:
                return "awww"
        except (subprocess.TimeoutExpired, FileNotFoundError):
            pass

    # Check for swww daemon (legacy name)
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

    if backend == "awww":
        return [
            "awww",
            "img",
            image_path,
            "--transition-type",
            "fade",
            "--transition-duration",
            "1",
        ]

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


def get_plugin_actions() -> list[dict]:
    """Get plugin-level actions for the action bar"""
    return [
        {
            "id": "random",
            "name": "Random",
            "icon": "shuffle",
            "shortcut": "Ctrl+1",
        },
        {
            "id": "history",
            "name": "History",
            "icon": "history",
            "shortcut": "Ctrl+2",
        },
    ]


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")
    context = input_data.get("context", "")

    # Handle plugin actions
    if step == "action" and selected.get("id") == "__plugin__":
        wallpaper_dir = get_wallpaper_dir()

        if action == "random":
            random_path = get_random_wallpaper(wallpaper_dir)
            if random_path:
                mode = "dark"
                command = build_wallpaper_command(random_path, mode)
                filename = Path(random_path).name
                save_wallpaper_to_history(random_path)
                print(
                    json.dumps(
                        {
                            "type": "execute",
                            "execute": {
                                "command": command,
                                "name": f"Random wallpaper: {filename}",
                                "icon": "shuffle",
                                "thumbnail": random_path,
                                "close": True,
                            },
                        }
                    )
                )
            else:
                print(json.dumps({"type": "error", "message": "No wallpapers found"}))
            return

        if action == "history":
            history = load_wallpaper_history()
            if not history:
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": [
                                {
                                    "id": "__empty__",
                                    "name": "No wallpaper history",
                                    "icon": "info",
                                    "description": "Set a wallpaper to see it here",
                                }
                            ],
                            "pluginActions": get_plugin_actions(),
                            "context": "history",
                        }
                    )
                )
                return

            results = []
            for path in history:
                if Path(path).exists():
                    filename = Path(path).name
                    results.append(
                        {
                            "id": f"history:{path}",
                            "name": filename,
                            "description": path,
                            "icon": "image",
                            "thumbnail": path,
                            "verb": "Set",
                        }
                    )

            if not results:
                results.append(
                    {
                        "id": "__empty__",
                        "name": "No wallpaper history",
                        "icon": "info",
                        "description": "Previous wallpapers no longer exist",
                    }
                )

            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": results,
                        "pluginActions": get_plugin_actions(),
                        "context": "history",
                    }
                )
            )
            return

    # Handle history item selection
    if step == "action" and selected.get("id", "").startswith("history:"):
        file_path = selected.get("id")[8:]  # Remove "history:" prefix
        if not Path(file_path).exists():
            print(json.dumps({"type": "error", "message": "File no longer exists"}))
            return

        mode = "dark"
        command = build_wallpaper_command(file_path, mode)
        filename = Path(file_path).name
        save_wallpaper_to_history(file_path)

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

    # Initial or search: show the image browser
    if step in ("initial", "search"):
        # Determine initial directory
        initial_dir = str(get_wallpaper_dir())

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
                    "pluginActions": get_plugin_actions(),
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

        # Save to history
        save_wallpaper_to_history(file_path)

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
