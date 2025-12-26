#!/usr/bin/env python3
"""
Shell workflow handler - search and execute shell commands.
Indexes:
- Shell history (from zsh/bash/fish)
- Binaries from $PATH (for command auto-detection)
"""

import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

IS_NIRI = bool(os.environ.get("NIRI_SOCKET"))


def get_path_binaries() -> list[str]:
    """Get all executable binaries from $PATH directories."""
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
                        binaries.add(entry.name)
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
        parse_func = lambda line: line.strip()

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


def make_terminal_cmd_for_index(cmd: str, execute: bool = True) -> list[str]:
    """Build command for index items (can't use function, needs inline script)."""
    terminal = os.environ.get("TERMINAL", "ghostty")
    cmd_repr = repr(cmd)
    enter_key = "&& ydotool key 28:1 28:0" if execute else ""

    if IS_NIRI:
        script = f"""
niri msg action spawn -- {terminal}
sleep 0.3
ydotool type --key-delay=0 -- {cmd_repr} {enter_key}
"""
    else:
        script = f"""
hyprctl dispatch exec '[float] {terminal}'
for i in $(seq 1 50); do
    active=$(hyprctl activewindow -j 2>/dev/null | jq -r '.class // empty' 2>/dev/null)
    case "$active" in *ghostty*|*kitty*|*alacritty*|*foot*|*{terminal}*) ydotool type --key-delay=0 -- {cmd_repr} {enter_key}; exit 0 ;; esac
    sleep 0.02
done
ydotool type --key-delay=0 -- {cmd_repr} {enter_key}
"""
    return ["bash", "-c", script.strip()]


def binary_to_index_item(binary: str) -> dict:
    """Convert a binary name to indexable item format for main search."""
    return {
        "id": f"bin:{binary}",
        "name": binary,
        "description": "Command",
        "icon": "terminal",
        "verb": "Run",
        "execute": {"command": make_terminal_cmd_for_index(binary, execute=False)},
        "actions": [
            {
                "id": "run",
                "name": "Run Now",
                "icon": "play_arrow",
                "command": make_terminal_cmd_for_index(binary, execute=True),
            },
            {
                "id": "copy",
                "name": "Copy",
                "icon": "content_copy",
                "command": ["wl-copy", binary],
            },
        ],
    }


def get_cmd_hash(cmd: str) -> str:
    """Get a stable hash for a command."""
    return hashlib.md5(cmd.encode()).hexdigest()[:12]


def make_terminal_cmd(cmd: str, floating: bool = True) -> list[str]:
    """Build command to run in terminal, waiting for terminal to be ready."""
    terminal = os.environ.get("TERMINAL", "ghostty")
    cmd_repr = repr(cmd)

    if IS_NIRI:
        # Niri doesn't have hyprctl - use simple sleep approach
        wait_script = f"""
niri msg action spawn -- {terminal}
sleep 0.3
ydotool type --key-delay=0 -- {cmd_repr} && ydotool key 28:1 28:0
"""
    else:
        # Wait for terminal window to become active instead of fixed sleep
        # Uses hyprctl to poll for active window class matching terminal
        wait_script = f"""
terminal_class="{terminal}"
hyprctl dispatch exec '{f"[float] " if floating else ""}{terminal}'
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
    return ["bash", "-c", wait_script.strip()]


def history_to_index_item(cmd: str) -> dict:
    """Convert a shell history command to indexable item format for main search."""
    display_cmd = cmd if len(cmd) <= 60 else cmd[:60] + "..."

    return {
        "id": f"history:{get_cmd_hash(cmd)}",
        "name": display_cmd,
        "description": "History",
        "keywords": cmd.lower().split()[:10],
        "icon": "history",
        "verb": "Run",
        "execute": {"command": make_terminal_cmd_for_index(cmd, execute=True)},
        "actions": [
            {
                "id": "copy",
                "name": "Copy",
                "icon": "content_copy",
                "command": ["wl-copy", cmd],
            }
        ],
    }


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    # Indexes both binaries from $PATH and shell history
    if step == "index":
        mode = input_data.get("mode", "full")
        indexed_ids = set(input_data.get("indexedIds", []))

        # Get current binaries and history
        binaries = get_path_binaries()
        commands = get_shell_history()[:30]  # Limit to 30 for main search

        # Build current ID sets
        current_bin_ids = {f"bin:{b}" for b in binaries}
        current_hist_ids = {f"history:{get_cmd_hash(c)}" for c in commands}
        current_ids = current_bin_ids | current_hist_ids

        if mode == "incremental" and indexed_ids:
            # Find new items
            new_ids = current_ids - indexed_ids

            items = []
            for binary in binaries:
                if f"bin:{binary}" in new_ids:
                    items.append(binary_to_index_item(binary))
            for cmd in commands:
                if f"history:{get_cmd_hash(cmd)}" in new_ids:
                    items.append(history_to_index_item(cmd))

            # Find removed items
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
            # Full reindex
            items = []
            for binary in binaries:
                items.append(binary_to_index_item(binary))
            for cmd in commands:
                items.append(history_to_index_item(cmd))
            print(json.dumps({"type": "index", "items": items}))
        return

    if step == "initial":
        # Load and show initial history
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

        print(
            json.dumps({"type": "results", "results": results, "inputMode": "realtime"})
        )
        return

    if step == "search":
        # Filter history by query
        commands = get_shell_history()
        filtered = fuzzy_filter(query, commands)

        results = []

        # Always offer to run the raw query as a command (first result)
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

        # Add history matches (skip if exact match with query to avoid duplicate)
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

        print(
            json.dumps({"type": "results", "results": results, "inputMode": "realtime"})
        )
        return

    if step == "action":
        cmd = selected.get("id", "")
        if not cmd:
            print(json.dumps({"type": "error", "message": "No command selected"}))
            return

        display_cmd = cmd if len(cmd) <= 50 else cmd[:50] + "..."

        if action == "run-float":
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": make_terminal_cmd(cmd, floating=True),
                            "name": f"Run: {display_cmd}",
                            "icon": "terminal",
                            "close": True,
                        },
                    }
                )
            )
        elif action == "run-tiled":
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": make_terminal_cmd(cmd, floating=False),
                            "name": f"Run: {display_cmd}",
                            "icon": "terminal",
                            "close": True,
                        },
                    }
                )
            )
        elif action == "copy":
            subprocess.run(["wl-copy", cmd], check=False)
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["true"],
                            "name": f"Copy: {display_cmd}",
                            "icon": "content_copy",
                            "close": True,
                        },
                    }
                )
            )
        else:
            # Default: run floating
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": make_terminal_cmd(cmd, floating=True),
                            "name": f"Run: {display_cmd}",
                            "icon": "terminal",
                            "close": True,
                        },
                    }
                )
            )


if __name__ == "__main__":
    main()
