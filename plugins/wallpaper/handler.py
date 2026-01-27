#!/usr/bin/env python3
"""
Wallpaper workflow handler - browse and set wallpapers.

Supports multiple wallpaper backends with automatic detection:
1. awww (swww renamed, recommended for Wayland)
2. swww (legacy name)
3. hyprctl hyprpaper
4. swaybg
5. feh (X11 fallback)
"""

import json
import os
import random
import shutil
import subprocess
import sys
from pathlib import Path

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


def scan_wallpaper_dir(directory: Path) -> list[dict]:
    """Scan wallpaper directory and return results."""
    if not directory.exists() or not directory.is_dir():
        return []

    results = []
    for f in directory.iterdir():
        if f.is_file() and f.suffix.lower() in IMAGE_EXTENSIONS:
            file_path = str(f)
            results.append(
                {
                    "id": file_path,
                    "name": f.stem,
                    "icon": "image",
                    "thumbnail": file_path,
                    "verb": "Set",
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
                }
            )

    return results


PLUGIN_DIR = Path(__file__).parent
SWITCHWALL_PATHS = [
    PLUGIN_DIR / "switchwall.sh",  # bundled with plugin
    Path.home() / ".config" / "hamr" / "scripts" / "colors" / "switchwall.sh",
]


def find_switchwall_script() -> Path | None:
    """Find switchwall script for setting wallpaper and updating colors."""
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
    # First check for switchwall script (handles wallpaper + color extraction)
    switchwall = find_switchwall_script()
    if switchwall:
        return [str(switchwall), "--image", image_path, "--mode", mode]

    # Fall back to direct backend
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


def set_wallpaper(file_path: str, mode: str = "dark") -> None:
    """Set wallpaper and save to history."""
    command = build_wallpaper_command(file_path, mode)
    save_wallpaper_to_history(file_path)
    # Detach subprocess so it continues running after this script exits
    subprocess.Popen(
        command,
        start_new_session=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    if step == "initial":
        wallpaper_dir = get_wallpaper_dir()
        results = scan_wallpaper_dir(wallpaper_dir)

        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "displayHint": "large_grid",
                    "pluginActions": get_plugin_actions(),
                }
            )
        )
        return

    if step == "search":
        wallpaper_dir = get_wallpaper_dir()
        results = scan_wallpaper_dir(wallpaper_dir)

        if query:
            query_lower = query.lower()
            results = [r for r in results if query_lower in r["name"].lower()]

        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "displayHint": "large_grid",
                    "pluginActions": get_plugin_actions(),
                }
            )
        )
        return

    if step == "action":
        item_id = selected.get("id", "")
        wallpaper_dir = get_wallpaper_dir()

        # Handle plugin actions
        if item_id == "__plugin__":
            if action == "random":
                random_path = get_random_wallpaper(wallpaper_dir)
                if random_path:
                    set_wallpaper(random_path, "dark")
                    print(json.dumps({"type": "execute", "close": True}))
                else:
                    print(
                        json.dumps({"type": "error", "message": "No wallpapers found"})
                    )
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
                        results.append(
                            {
                                "id": f"history:{path}",
                                "name": Path(path).name,
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
        if item_id.startswith("history:"):
            file_path = item_id[8:]  # Remove "history:" prefix
            if not Path(file_path).exists():
                print(json.dumps({"type": "error", "message": "File no longer exists"}))
                return

            set_wallpaper(file_path, "dark")
            print(json.dumps({"type": "execute", "close": True}))
            return

        # Handle image selection (item_id is the file path)
        file_path = item_id
        if not Path(file_path).exists():
            print(
                json.dumps({"type": "error", "message": f"File not found: {file_path}"})
            )
            return

        # Determine mode based on action
        mode = "light" if action == "set_light" else "dark"
        set_wallpaper(file_path, mode)
        print(json.dumps({"type": "execute", "close": True}))


if __name__ == "__main__":
    main()
