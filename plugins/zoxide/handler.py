#!/usr/bin/env python3
"""
Zoxide plugin handler - index frequently used directories from zoxide.
"""

import ctypes
import json
import os
import select
import shutil
import struct
import subprocess
import sys
from pathlib import Path

TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"
IS_NIRI = bool(os.environ.get("NIRI_SOCKET"))

MAX_ITEMS = 50

# inotify constants
IN_CLOSE_WRITE = 0x00000008
IN_MOVED_TO = 0x00000080
IN_CREATE = 0x00000100

# Zoxide database location
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


def create_inotify_fd(watch_path: Path) -> int | None:
    """Create inotify fd watching a directory. Returns fd or None."""
    try:
        libc = ctypes.CDLL("libc.so.6", use_errno=True)
        fd = libc.inotify_init()
        if fd < 0:
            return None
        mask = IN_CLOSE_WRITE | IN_MOVED_TO | IN_CREATE
        wd = libc.inotify_add_watch(fd, str(watch_path).encode(), mask)
        if wd < 0:
            os.close(fd)
            return None
        return fd
    except (OSError, AttributeError):
        return None


def read_inotify_events(fd: int) -> list[str]:
    """Read pending inotify events, return list of changed filenames."""
    filenames = []
    try:
        buf = os.read(fd, 4096)
        offset = 0
        while offset < len(buf):
            wd, mask, cookie, length = struct.unpack_from("iIII", buf, offset)
            offset += 16
            if length:
                name = buf[offset : offset + length].rstrip(b"\x00").decode()
                filenames.append(name)
                offset += length
    except (OSError, struct.error):
        pass
    return filenames


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

    # Get directory contents preview
    if TEST_MODE:
        preview_content = "subdir1/\nsubdir2/\nfile1.txt\nfile2.py"
    else:
        preview_content = get_directory_preview(path)

    return {
        "id": f"zoxide:{path}",
        "name": name,
        "description": display_path,
        "icon": "folder_special",
        "keywords": path_parts,
        "verb": "Open",
        "entryPoint": path,
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


def handle_request(input_data: dict) -> None:
    """Handle a single request."""
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

    if step == "action":
        action_id = input_data.get("actionId")
        path = input_data.get("entryPoint")

        if not path:
            print(json.dumps({"type": "error", "message": "Missing path"}))
            return

        if action_id == "files":
            try:
                if not TEST_MODE:
                    subprocess.Popen(["xdg-open", path])
                print(json.dumps({"type": "execute", "close": True}))
            except Exception as e:
                print(json.dumps({"type": "error", "message": str(e)}))
            return

        if action_id == "copy":
            try:
                if not TEST_MODE:
                    subprocess.run(
                        ["wl-copy"],
                        input=path.encode(),
                        timeout=5,
                    )
                print(json.dumps({"type": "execute", "close": True}))
            except Exception as e:
                print(json.dumps({"type": "error", "message": str(e)}))
            return

        # Default action: open terminal
        try:
            if not TEST_MODE:
                cmd = make_terminal_cmd(path)
                subprocess.Popen(cmd)
            print(json.dumps({"type": "execute", "close": True}))
        except Exception as e:
            print(json.dumps({"type": "error", "message": str(e)}))
        return

    print(json.dumps({"type": "error", "message": "Invalid request"}))


def emit_full_index() -> None:
    """Emit full index of zoxide directories."""
    if TEST_MODE:
        dirs = get_mock_dirs()
    else:
        dirs = get_zoxide_dirs()

    items = [dir_to_index_item(d) for d in dirs]
    print(
        json.dumps({"type": "index", "mode": "full", "items": items}),
        flush=True,
    )


def main():
    if TEST_MODE:
        input_data = json.load(sys.stdin)
        handle_request(input_data)
        return

    import signal

    signal.signal(signal.SIGTERM, lambda s, f: sys.exit(0))
    signal.signal(signal.SIGINT, lambda s, f: sys.exit(0))

    # Emit full index on startup
    emit_full_index()

    watch_dir = ZOXIDE_DB.parent
    watch_filename = "db.zo"

    if not watch_dir.exists():
        watch_dir.mkdir(parents=True, exist_ok=True)

    inotify_fd = create_inotify_fd(watch_dir)

    if inotify_fd is not None:
        # Daemon mode with inotify
        while True:
            readable, _, _ = select.select([sys.stdin, inotify_fd], [], [], 1.0)

            for r in readable:
                if r == sys.stdin:
                    try:
                        line = sys.stdin.readline()
                        if not line:
                            return
                        input_data = json.loads(line)
                        handle_request(input_data)
                        sys.stdout.flush()
                    except json.JSONDecodeError:
                        continue

                elif r == inotify_fd:
                    changed = read_inotify_events(inotify_fd)
                    if watch_filename in changed:
                        print(json.dumps({"type": "index"}), flush=True)
    else:
        # Fallback: mtime polling
        last_mtime = ZOXIDE_DB.stat().st_mtime if ZOXIDE_DB.exists() else 0

        while True:
            readable, _, _ = select.select([sys.stdin], [], [], 2.0)

            if readable:
                try:
                    line = sys.stdin.readline()
                    if not line:
                        return
                    input_data = json.loads(line)
                    handle_request(input_data)
                    sys.stdout.flush()
                except json.JSONDecodeError:
                    continue

            # Check mtime
            if ZOXIDE_DB.exists():
                current = ZOXIDE_DB.stat().st_mtime
                if current != last_mtime:
                    last_mtime = current
                    print(json.dumps({"type": "index"}), flush=True)


if __name__ == "__main__":
    main()
