#!/usr/bin/env python3
"""
Zoxide plugin handler - index frequently used directories from zoxide.
"""

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"
IS_NIRI = bool(os.environ.get("NIRI_SOCKET"))

MAX_ITEMS = 50


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


def get_mock_dirs() -> list[dict]:
    """Return mock data for testing."""
    return [
        {"path": "/home/user/Projects/hamr", "score": 100.0},
        {"path": "/home/user/Documents", "score": 80.0},
        {"path": "/home/user/.config", "score": 60.0},
    ]


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

    return {
        "id": f"zoxide:{path}",
        "name": name,
        "description": display_path,
        "icon": "folder_special",
        "keywords": path_parts,
        "verb": "Open",
        "execute": {"command": make_terminal_cmd(path)},
        "actions": [
            {
                "id": "files",
                "name": "Open in Files",
                "icon": "folder_open",
                "command": ["xdg-open", path],
            },
            {
                "id": "copy",
                "name": "Copy Path",
                "icon": "content_copy",
                "command": ["wl-copy", path],
            },
        ],
    }


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")

    if step == "index":
        mode = input_data.get("mode", "full")
        indexed_ids = set(input_data.get("indexedIds", []))

        if TEST_MODE:
            dirs = get_mock_dirs()
        else:
            dirs = get_zoxide_dirs()

        current_ids = {f"zoxide:{d['path']}" for d in dirs}

        if mode == "incremental":
            new_ids = current_ids - indexed_ids
            items = [
                dir_to_index_item(d) for d in dirs if f"zoxide:{d['path']}" in new_ids
            ]
            removed_ids = list(indexed_ids - current_ids)

            print(
                json.dumps(
                    {
                        "type": "index",
                        "mode": "incremental",
                        "items": items,
                        "remove": removed_ids,
                    }
                )
            )
        else:
            items = [dir_to_index_item(d) for d in dirs]
            print(json.dumps({"type": "index", "items": items}))
        return

    print(json.dumps({"type": "error", "message": "Zoxide is index-only"}))


if __name__ == "__main__":
    main()
