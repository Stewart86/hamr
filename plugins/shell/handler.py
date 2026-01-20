#!/usr/bin/env python3
"""
Shell workflow handler - search and execute shell commands.
Indexes:
- Shell history commands (from zsh/bash/fish)
- Binaries from $PATH that appear in shell history (commands actually used)
"""

import ctypes
import hashlib
import json
import os
import select
import struct
import subprocess
import sys
from pathlib import Path

# Add parent directory to path to import SDK
sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

IS_NIRI = bool(os.environ.get("NIRI_SOCKET"))

IN_CLOSE_WRITE = 0x00000008
IN_MOVED_TO = 0x00000080
IN_CREATE = 0x00000100


def create_inotify_fd(watch_paths: list[Path]) -> tuple[int | None, dict[int, Path]]:
    """Create inotify fd watching multiple directories.
    Returns (fd, {wd: path}) or (None, {}) if unavailable.
    """
    try:
        libc = ctypes.CDLL("libc.so.6", use_errno=True)
        fd = libc.inotify_init()
        if fd < 0:
            return None, {}

        wd_to_path = {}
        mask = IN_CLOSE_WRITE | IN_MOVED_TO | IN_CREATE

        for path in watch_paths:
            if path.exists():
                wd = libc.inotify_add_watch(fd, str(path).encode(), mask)
                if wd >= 0:
                    wd_to_path[wd] = path

        if not wd_to_path:
            os.close(fd)
            return None, {}

        return fd, wd_to_path
    except (OSError, AttributeError):
        return None, {}


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


def get_path_binaries(filter_set: set[str] | None = None) -> list[str]:
    """Get executable binaries from $PATH directories.

    If filter_set is provided, only return binaries whose names are in the set.
    This allows filtering to only commands that appear in shell history.
    """
    path_dirs = os.environ.get("PATH", "").split(":")
    binaries = set()

    for dir_path in path_dirs:
        if not dir_path:
            continue
        try:
            p = Path(dir_path)
            if p.exists() and p.is_dir():
                for entry in p.iterdir():
                    if entry.is_file() and os.access(entry, os.X_OK):
                        name = entry.name
                        if filter_set is None or name in filter_set:
                            binaries.add(name)
        except (PermissionError, OSError):
            continue

    return sorted(binaries)


def get_shell_history() -> list[str]:
    """Get shell history from zsh, bash, or fish"""
    shell = os.environ.get("SHELL", "/bin/bash")
    home = Path.home()

    history_file = None
    parse_func = None

    if "zsh" in shell:
        history_file = home / ".zsh_history"

        def parse_zsh(line):
            # Format: : TIMESTAMP:DURATION;COMMAND
            if line.startswith(": "):
                parts = line.split(";", 1)
                if len(parts) > 1:
                    return parts[1].strip()
            return line.strip()

        parse_func = parse_zsh
    elif "fish" in shell:
        history_file = home / ".local/share/fish/fish_history"

        def parse_fish(line):
            # Format: - cmd: COMMAND
            if line.startswith("- cmd: "):
                return line[7:].strip()
            return None

        parse_func = parse_fish
    else:
        history_file = home / ".bash_history"

        def parse_bash(line: str) -> str:
            return line.strip()

        parse_func = parse_bash

    if not history_file or not history_file.exists():
        return []

    try:
        with open(history_file, "r", errors="ignore") as f:
            lines = f.readlines()
    except Exception:
        return []

    # Parse and deduplicate
    seen = set()
    commands = []
    for line in reversed(lines):
        cmd = parse_func(line)
        if cmd and cmd not in seen and len(cmd) > 1:
            seen.add(cmd)
            commands.append(cmd)
            if len(commands) >= 500:
                break

    return commands


def get_history_command_names() -> set[str]:
    """Extract unique command names (first word) from shell history.

    Returns a set of command names that have been used, for filtering PATH binaries.
    """
    commands = get_shell_history()
    names = set()
    for cmd in commands:
        first_word = cmd.split()[0] if cmd.split() else ""
        if first_word:
            names.add(first_word)
    return names


def fuzzy_filter(query: str, commands: list[str]) -> list[str]:
    """Simple fuzzy filter - matches if all query chars appear in order"""
    if not query:
        return commands[:50]

    query_lower = query.lower()
    results = []

    for cmd in commands:
        cmd_lower = cmd.lower()
        qi = 0
        for c in cmd_lower:
            if qi < len(query_lower) and c == query_lower[qi]:
                qi += 1
        if qi == len(query_lower):
            results.append(cmd)
            if len(results) >= 50:
                break

    return results


def binary_to_index_item(binary: str) -> dict:
    """Convert a binary name to indexable item format for main search."""
    item_id = f"bin:{binary}"
    return {
        "id": item_id,
        "name": binary,
        "description": "Command",
        "icon": "terminal",
        "verb": "Run",
        "entryPoint": {
            "step": "action",
            "selected": {"id": item_id},
        },
        "actions": [
            {
                "id": "run",
                "name": "Run Now",
                "icon": "play_arrow",
            },
            {
                "id": "copy",
                "name": "Copy",
                "icon": "content_copy",
            },
        ],
    }


def get_cmd_hash(cmd: str) -> str:
    """Get a stable hash for a command."""
    return hashlib.md5(cmd.encode()).hexdigest()[:12]


def run_in_terminal(cmd: str, floating: bool = True) -> None:
    """Open terminal and type command, then press enter."""
    terminal = os.environ.get("TERMINAL", "ghostty")
    cmd_repr = repr(cmd)

    if IS_NIRI:
        # Niri: spawn terminal, wait, then type command
        wait_script = f"""
niri msg action spawn -- {terminal}
sleep 0.3
ydotool type --key-delay=0 -- {cmd_repr} && ydotool key 28:1 28:0
"""
    else:
        # Hyprland: spawn terminal with optional float, poll for active window, then type
        wait_script = f"""
terminal_class="{terminal}"
hyprctl dispatch exec '{"[float] " if floating else ""}{terminal}'
for i in $(seq 1 50); do
    active=$(hyprctl activewindow -j 2>/dev/null | jq -r '.class // empty' 2>/dev/null)
    if [[ "$active" == *"$terminal_class"* ]] || [[ "$active" == *"ghostty"* ]] || [[ "$active" == *"kitty"* ]] || [[ "$active" == *"alacritty"* ]] || [[ "$active" == *"foot"* ]]; then
        ydotool type --key-delay=0 -- {cmd_repr} && ydotool key 28:1 28:0
        exit 0
    fi
    sleep 0.02
done
# Fallback after 1 second
ydotool type --key-delay=0 -- {cmd_repr} && ydotool key 28:1 28:0
"""
    # Run in background so we don't block the handler
    subprocess.Popen(
        ["bash", "-c", wait_script.strip()],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )


def history_to_index_item(cmd: str) -> dict:
    """Convert a shell history command to indexable item format for main search."""
    display_cmd = cmd if len(cmd) <= 60 else cmd[:60] + "..."
    item_id = f"history:{get_cmd_hash(cmd)}"

    return {
        "id": item_id,
        "name": display_cmd,
        "description": "History",
        "keywords": cmd.lower().split()[:10],
        "icon": "history",
        "verb": "Run",
        "entryPoint": {
            "step": "action",
            "selected": {"id": item_id},
        },
        "actions": [
            {
                "id": "copy",
                "name": "Copy",
                "icon": "content_copy",
            }
        ],
    }


def get_index_items() -> list[dict]:
    """Get all indexable items (binaries and history commands)."""
    history_cmd_names = get_history_command_names()
    binaries = get_path_binaries(filter_set=history_cmd_names)
    commands = get_shell_history()[:50]

    items = []
    for binary in binaries:
        items.append(binary_to_index_item(binary))
    for cmd in commands:
        items.append(history_to_index_item(cmd))
    return items


def handle_request(input_data: dict):
    """Handle a single request (initial, search, action, index)."""
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    if step == "index":
        mode = input_data.get("mode", "full")
        indexed_ids = set(input_data.get("indexedIds", []))

        history_cmd_names = get_history_command_names()

        binaries = get_path_binaries(filter_set=history_cmd_names)
        commands = get_shell_history()[:50]

        current_bin_ids = {f"bin:{b}" for b in binaries}
        current_hist_ids = {f"history:{get_cmd_hash(c)}" for c in commands}
        current_ids = current_bin_ids | current_hist_ids

        if mode == "incremental" and indexed_ids:
            new_ids = current_ids - indexed_ids

            items = []
            for binary in binaries:
                if f"bin:{binary}" in new_ids:
                    items.append(binary_to_index_item(binary))
            for cmd in commands:
                if f"history:{get_cmd_hash(cmd)}" in new_ids:
                    items.append(history_to_index_item(cmd))

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
            items = get_index_items()
            print(json.dumps({"type": "index", "items": items}))
        return

    if step == "initial":
        commands = get_shell_history()[:50]
        results = [
            {
                "id": cmd,
                "name": cmd,
                "verb": "Run",
                "actions": [
                    {
                        "id": "run-float",
                        "name": "Run (floating)",
                        "icon": "open_in_new",
                    },
                    {"id": "run-tiled", "name": "Run (tiled)", "icon": "terminal"},
                    {"id": "copy", "name": "Copy", "icon": "content_copy"},
                ],
            }
            for cmd in commands
        ]

        print(json.dumps(HamrPlugin.results(results, input_mode="realtime")))
        return

    if step == "search":
        commands = get_shell_history()
        filtered = fuzzy_filter(query, commands)

        results = []

        if query:
            results.append(
                {
                    "id": query,
                    "name": query,
                    "description": "Run command",
                    "verb": "Run",
                    "actions": [
                        {
                            "id": "run-float",
                            "name": "Run (floating)",
                            "icon": "open_in_new",
                        },
                        {"id": "run-tiled", "name": "Run (tiled)", "icon": "terminal"},
                        {"id": "copy", "name": "Copy", "icon": "content_copy"},
                    ],
                }
            )

        for cmd in filtered:
            if cmd == query:
                continue
            results.append(
                {
                    "id": cmd,
                    "name": cmd,
                    "description": "History",
                    "verb": "Run",
                    "actions": [
                        {
                            "id": "run-float",
                            "name": "Run (floating)",
                            "icon": "open_in_new",
                        },
                        {"id": "run-tiled", "name": "Run (tiled)", "icon": "terminal"},
                        {"id": "copy", "name": "Copy", "icon": "content_copy"},
                    ],
                }
            )

        print(json.dumps(HamrPlugin.results(results, input_mode="realtime")))
        return

    if step == "action":
        item_id = selected.get("id", "")
        if not item_id:
            print(json.dumps(HamrPlugin.error("No command selected")))
            return

        # Extract command from item_id
        # Index items have format "bin:command" or "history:hash"
        # Interactive items use the raw command as id
        if item_id.startswith("bin:"):
            cmd = item_id[4:]  # Remove "bin:" prefix
        elif item_id.startswith("history:"):
            # For history items, we need to look up the command by hash
            cmd_hash = item_id[8:]  # Remove "history:" prefix
            commands = get_shell_history()
            cmd = next((c for c in commands if get_cmd_hash(c) == cmd_hash), None)
            if not cmd:
                print(
                    json.dumps(
                        {"type": "error", "message": "Command not found in history"}
                    )
                )
                return
        else:
            # Interactive mode: id is the raw command
            cmd = item_id

        if action == "copy":
            print(json.dumps(HamrPlugin.copy_and_close(cmd)))
        elif action == "run-tiled":
            run_in_terminal(cmd, floating=False)
            print(json.dumps(HamrPlugin.close()))
        else:
            # Default: run floating (covers "run-float", "run", and no action)
            run_in_terminal(cmd, floating=True)
            print(json.dumps(HamrPlugin.close()))


def main():
    """Main entry point - daemon mode with inotify file watching."""
    # Force line-buffered stdout to prevent partial writes
    sys.stdout = open(sys.stdout.fileno(), "w", buffering=1, closefd=False)

    shell = os.environ.get("SHELL", "/bin/bash")
    home = Path.home()

    history_files = []
    if "zsh" in shell:
        history_files.append(home / ".zsh_history")
    elif "fish" in shell:
        history_files.append(home / ".local/share/fish/fish_history")
    else:
        history_files.append(home / ".bash_history")

    watch_dirs = list({f.parent for f in history_files if f.parent.exists()})
    history_names = {f.name for f in history_files}

    items = get_index_items()
    print(json.dumps({"type": "index", "mode": "full", "items": items}), flush=True)

    inotify_fd, wd_to_path = create_inotify_fd(watch_dirs)

    if inotify_fd is not None:
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
                    if any(name in history_names for name in changed):
                        items = get_index_items()
                        print(
                            json.dumps(
                                {"type": "index", "mode": "full", "items": items}
                            )
                        )
                        sys.stdout.flush()
    else:
        last_mtime = {f: f.stat().st_mtime if f.exists() else 0 for f in history_files}

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

            for f in history_files:
                if f.exists():
                    current = f.stat().st_mtime
                    if current != last_mtime.get(f, 0):
                        last_mtime[f] = current
                        items = get_index_items()
                        print(
                            json.dumps(
                                {"type": "index", "mode": "full", "items": items}
                            )
                        )
                        sys.stdout.flush()
                        break


if __name__ == "__main__":
    main()
