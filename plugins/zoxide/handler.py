#!/usr/bin/env python3
"""
Zoxide plugin - index frequently used directories from zoxide.
Socket-based daemon version for hamr-rs.
"""

import asyncio
import os
import shutil
import subprocess
import sys
from pathlib import Path

# Add parent directory to path to import SDK
sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

IS_NIRI = bool(os.environ.get("NIRI_SOCKET"))

MAX_ITEMS = 50
ZOXIDE_DB = Path.home() / ".local/share/zoxide/db.zo"


def get_zoxide_dirs() -> list[dict]:
    """Get directories from zoxide database with scores."""
    if not shutil.which("zoxide"):
        return []

    try:
        result = subprocess.run(
            ["zoxide", "query", "-l", "-s"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        if result.returncode != 0:
            return []

        dirs = []
        for line in result.stdout.strip().split("\n"):
            if not line.strip():
                continue
            parts = line.split(maxsplit=1)
            if len(parts) != 2:
                continue

            score_str, path = parts
            try:
                score = float(score_str)
            except ValueError:
                continue

            path_obj = Path(path)
            if path_obj.exists() and path_obj.is_dir():
                dirs.append({"path": path, "score": score})

        dirs.sort(key=lambda x: -x["score"])
        return dirs[:MAX_ITEMS]

    except (subprocess.TimeoutExpired, Exception):
        return []


def make_terminal_cmd(path: str) -> list[str]:
    """Build command to open terminal at directory.

    Uses terminal's native --working-directory flag.
    For ghostty with gtk-single-instance, we disable it for this invocation
    to ensure the working directory is respected.
    """
    terminal = os.environ.get("TERMINAL", "ghostty")
    terminal_name = os.path.basename(terminal).lower()

    if terminal_name in ("ghostty",):
        cmd_parts = [
            terminal,
            "--gtk-single-instance=false",
            f"--working-directory={path}",
        ]
    elif terminal_name in ("kitty",):
        cmd_parts = [terminal, "-d", path]
    elif terminal_name in ("alacritty",):
        cmd_parts = [terminal, "--working-directory", path]
    elif terminal_name in ("wezterm", "wezterm-gui"):
        cmd_parts = [terminal, "start", "--cwd", path]
    elif terminal_name in ("konsole",):
        cmd_parts = [terminal, "--workdir", path]
    elif terminal_name in ("foot",):
        cmd_parts = [terminal, "-D", path]
    else:
        cmd_parts = [terminal, f"--working-directory={path}"]

    if IS_NIRI:
        return ["niri", "msg", "action", "spawn", "--"] + cmd_parts
    return ["hyprctl", "dispatch", "exec", "--", *cmd_parts]


def get_directory_preview(path: str) -> str:
    """Get a preview of directory contents (first 20 items)."""
    try:
        path_obj = Path(path)
        if not path_obj.exists() or not path_obj.is_dir():
            return ""

        items = []
        for item in sorted(path_obj.iterdir())[:20]:
            suffix = "/" if item.is_dir() else ""
            items.append(f"{item.name}{suffix}")

        if len(list(path_obj.iterdir())) > 20:
            items.append("...")

        return "\n".join(items) if items else "(empty directory)"
    except (PermissionError, OSError):
        return "(permission denied)"


def dir_to_index_item(dir_info: dict) -> dict:
    """Convert directory info to indexable item format."""
    path = dir_info["path"]
    path_obj = Path(path)
    name = path_obj.name or path

    home = str(Path.home())
    if path.startswith(home):
        display_path = "~" + path[len(home) :]
    else:
        display_path = path

    path_parts = [p for p in path.lower().split("/") if p]

    preview_content = get_directory_preview(path)

    item_id = f"zoxide:{path}"
    return {
        "id": item_id,
        "name": name,
        "description": display_path,
        "icon": "folder_special",
        "keywords": path_parts,
        "verb": "Open",
        "preview": {
            "type": "text",
            "content": preview_content,
            "title": name,
            "metadata": [
                {"label": "Path", "value": display_path},
            ],
        },
        "actions": [
            {
                "id": "files",
                "name": "Open in Files",
                "icon": "folder_open",
            },
            {
                "id": "copy",
                "name": "Copy Path",
                "icon": "content_copy",
            },
        ],
    }


# Create plugin instance
plugin = HamrPlugin(
    id="zoxide",
    name="Zoxide",
    description="Jump to frequently used directories",
    icon="folder_special",
)


@plugin.on_action
async def handle_action(item_id: str, action=None, context=None):
    """Handle action request."""
    if not item_id.startswith("zoxide:"):
        return {"error": "Invalid item ID"}

    path = item_id[7:]

    if not path:
        return {"error": "Missing path"}

    if action == "files":
        try:
            subprocess.Popen(["xdg-open", path])
            return HamrPlugin.close()
        except Exception as e:
            return HamrPlugin.error(str(e))

    if action == "copy":
        try:
            subprocess.run(
                ["wl-copy"],
                input=path.encode(),
                timeout=5,
            )
            return HamrPlugin.close()
        except Exception as e:
            return HamrPlugin.error(str(e))

    # Default action: open terminal at directory
    try:
        cmd = make_terminal_cmd(path)
        subprocess.Popen(cmd)
        return HamrPlugin.close()
    except Exception as e:
        return HamrPlugin.error(str(e))


@plugin.add_background_task
async def emit_index(p: HamrPlugin):
    """Background task to emit full index on startup and watch for changes."""
    last_mtime = ZOXIDE_DB.stat().st_mtime if ZOXIDE_DB.exists() else 0
    last_dirs = None

    # Emit initial full index
    dirs = get_zoxide_dirs()
    items = [dir_to_index_item(d) for d in dirs]
    await p.send_index(items)
    last_dirs = dirs

    # Watch for changes - check mtime first (cheap), then compare actual data
    while True:
        await asyncio.sleep(5)

        if not ZOXIDE_DB.exists():
            continue

        current_mtime = ZOXIDE_DB.stat().st_mtime
        if current_mtime == last_mtime:
            continue  # No file change, skip expensive query

        last_mtime = current_mtime
        dirs = get_zoxide_dirs()

        # Only send if directory list actually changed
        if dirs != last_dirs:
            items = [dir_to_index_item(d) for d in dirs]
            await p.send_index(items)
            last_dirs = dirs


if __name__ == "__main__":
    plugin.run()
