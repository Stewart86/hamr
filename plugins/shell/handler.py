#!/usr/bin/env python3
"""
Shell history workflow handler - search and execute shell commands
"""

import json
import os
import subprocess
import sys
from pathlib import Path


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


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    if step == "initial":
        # Load and show initial history
        commands = get_shell_history()[:50]
        results = [
            {
                "id": cmd,
                "name": cmd,
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

        results = [
            {
                "id": cmd,
                "name": cmd,
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
            for cmd in filtered
        ]

        print(
            json.dumps({"type": "results", "results": results, "inputMode": "realtime"})
        )
        return

    if step == "action":
        cmd = selected.get("id", "")
        if not cmd:
            print(json.dumps({"type": "error", "message": "No command selected"}))
            return

        # Truncate command for history display
        display_cmd = cmd if len(cmd) <= 50 else cmd[:50] + "..."

        # Use Python repr for proper escaping in bash
        cmd_repr = repr(cmd)

        # Use $TERMINAL env var, fall back to common terminals
        terminal = os.environ.get("TERMINAL", "ghostty")

        if action == "run-float":
            # Launch floating terminal, type command with ydotool, press Enter
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": [
                                "bash",
                                "-c",
                                f"hyprctl dispatch exec '[float] {terminal}' && sleep 0.3 && ydotool type --key-delay=0 -- {cmd_repr} && ydotool key 28:1 28:0",
                            ],
                            "name": f"Run: {display_cmd}",
                            "icon": "terminal",
                            "close": True,
                        },
                    }
                )
            )
        elif action == "run-tiled":
            # Launch tiled terminal, type command with ydotool, press Enter
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": [
                                "bash",
                                "-c",
                                f"{terminal} & sleep 0.3 && ydotool type --key-delay=0 -- {cmd_repr} && ydotool key 28:1 28:0",
                            ],
                            "name": f"Run: {display_cmd}",
                            "icon": "terminal",
                            "close": True,
                        },
                    }
                )
            )
        elif action == "copy":
            # Copy to clipboard using wl-copy
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
            # Default action: run in floating terminal with ydotool, press Enter
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": [
                                "bash",
                                "-c",
                                f"hyprctl dispatch exec '[float] {terminal}' && sleep 0.3 && ydotool type --key-delay=0 -- {cmd_repr} && ydotool key 28:1 28:0",
                            ],
                            "name": f"Run: {display_cmd}",
                            "icon": "terminal",
                            "close": True,
                        },
                    }
                )
            )


if __name__ == "__main__":
    main()
