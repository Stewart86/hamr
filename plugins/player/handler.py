#!/usr/bin/env python3
"""
Player plugin for hamr - Socket-based version using playerctl.

Provides media player controls:
- List active media players
- Play/pause/skip controls
- Loop and shuffle controls
"""

import asyncio
import subprocess
import sys
import time
from pathlib import Path
from typing import Optional

# Add parent directory to path to import SDK
sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin


def run_playerctl(args: list[str]) -> tuple[str, int]:
    """Run playerctl command and return output and return code."""
    try:
        result = subprocess.run(
            ["playerctl"] + args,
            capture_output=True,
            text=True,
            timeout=5,
        )
        return result.stdout.strip(), result.returncode
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return "", 1


def get_players() -> list[dict]:
    """Get list of active media players with metadata."""
    output, code = run_playerctl(["-l"])
    if code != 0 or not output:
        return []

    players = []
    for player_name in output.split("\n"):
        if not player_name:
            continue

        status, _ = run_playerctl(["-p", player_name, "status"])
        metadata, _ = run_playerctl(
            [
                "-p",
                player_name,
                "metadata",
                "--format",
                "{{title}}\t{{artist}}\t{{album}}\t{{mpris:artUrl}}",
            ]
        )

        parts = metadata.split("\t") if metadata else ["", "", "", ""]
        title = parts[0] if len(parts) > 0 else ""
        artist = parts[1] if len(parts) > 1 else ""
        album = parts[2] if len(parts) > 2 else ""
        art_url = parts[3] if len(parts) > 3 else ""

        players.append(
            {
                "name": player_name,
                "status": status or "Unknown",
                "title": title,
                "artist": artist,
                "album": album,
                "artUrl": art_url,
            }
        )

    return players


def format_time(seconds: int) -> str:
    """Format seconds as MM:SS."""
    if seconds < 0:
        return "0:00"
    mins = seconds // 60
    secs = seconds % 60
    return f"{mins}:{secs:02d}"


def get_player_position(player_name: str) -> tuple[int, int]:
    """Get current position and duration for a player."""
    position_str, _ = run_playerctl(["-p", player_name, "position"])
    duration_str, _ = run_playerctl(
        ["-p", player_name, "metadata", "--format", "{{mpris:length}}"]
    )
    try:
        position = int(float(position_str)) if position_str else 0
        duration = int(duration_str) // 1000000 if duration_str else 0
        return position, duration
    except ValueError:
        return 0, 0


def get_status_icon(status: str) -> str:
    """Get icon for player status."""
    status_lower = status.lower()
    if status_lower == "playing":
        return "play_arrow"
    if status_lower == "paused":
        return "pause"
    if status_lower == "stopped":
        return "stop"
    return "music_note"


def get_status_badge(status: str) -> dict:
    """Get badge for player status."""
    status_lower = status.lower()
    if status_lower == "playing":
        return {"icon": "play_arrow", "color": "#4caf50"}
    if status_lower == "paused":
        return {"icon": "pause", "color": "#ff9800"}
    if status_lower == "stopped":
        return {"icon": "stop", "color": "#f44336"}
    return {"icon": "music_note", "color": "#2196f3"}


def get_art_path(art_url: str) -> str | None:
    """Convert art URL to local file path if it's a file:// URL."""
    if not art_url:
        return None
    if art_url.startswith("file://"):
        return art_url[7:]  # Strip "file://"
    return None


def player_to_result(player: dict) -> dict:
    """Convert player info to search result."""
    description = player["artist"]
    if player["album"]:
        description = (
            f"{player['artist']} - {player['album']}"
            if player["artist"]
            else player["album"]
        )

    status_text = f"[{player['status']}]"
    name = player["title"] or player["name"]

    art_path = get_art_path(player.get("artUrl", ""))

    result = {
        "id": f"player:{player['name']}",
        "name": f"{name} {status_text}",
        "description": description or player["name"],
        "verb": "Pause" if player["status"].lower() == "playing" else "Play",
        "actions": [
            {"id": "previous", "name": "Previous", "icon": "skip_previous"},
            {"id": "next", "name": "Next", "icon": "skip_next"},
            {"id": "stop", "name": "Stop", "icon": "stop"},
            {"id": "more", "name": "More", "icon": "tune"},
        ],
        "badges": [get_status_badge(player["status"])],
        "chips": [{"text": player["name"], "icon": "music_note"}],
    }

    position, duration = get_player_position(player["name"])
    if duration > 0:
        result["progress"] = {
            "value": position,
            "max": duration,
            "label": f"{format_time(position)} / {format_time(duration)}",
        }

    if art_path:
        result["thumbnail"] = art_path
    else:
        result["icon"] = get_status_icon(player["status"])

    return result


CONTROL_RESULTS = [
    {
        "id": "loop-none",
        "name": "Loop: None",
        "icon": "repeat",
        "cmd": ["loop", "None"],
    },
    {
        "id": "loop-track",
        "name": "Loop: Track",
        "icon": "repeat_one",
        "cmd": ["loop", "Track"],
    },
    {
        "id": "loop-playlist",
        "name": "Loop: Playlist",
        "icon": "repeat",
        "cmd": ["loop", "Playlist"],
    },
    {
        "id": "shuffle-on",
        "name": "Shuffle: On",
        "icon": "shuffle",
        "cmd": ["shuffle", "On"],
    },
    {
        "id": "shuffle-off",
        "name": "Shuffle: Off",
        "icon": "shuffle",
        "cmd": ["shuffle", "Off"],
    },
]


def control_to_result(control: dict, player_name: str) -> dict:
    """Convert control to search result."""
    return {
        "id": f"control:{player_name}:{control['id']}",
        "name": control["name"],
        "icon": control["icon"],
        "verb": "Set",
    }


def run_player_command(player_name: str, cmd: list[str]) -> None:
    """Execute a playerctl command for a player."""
    run_playerctl(["-p", player_name] + cmd)


def get_initial_results() -> list[dict]:
    """Get initial player results."""
    players = get_players()
    if not players:
        return [
            {
                "id": "__no_players__",
                "name": "No media players detected",
                "description": "Start playing media in a supported application",
                "icon": "music_off",
            }
        ]
    return [player_to_result(p) for p in players]


def get_initial_actions() -> list[dict]:
    """Get initial plugin actions."""
    return [
        {"id": "refresh", "name": "Refresh", "icon": "refresh"},
    ]


# Create plugin instance
plugin = HamrPlugin(
    id="player",
    name="Player",
    description="Media player controls via playerctl",
    icon="play_circle",
)

# Plugin state
state = {
    "context": "",  # "" = players view, "controls:NAME" = controls view
}


@plugin.on_initial
async def handle_initial(params=None):
    """Handle initial request."""
    state["context"] = ""
    return HamrPlugin.results(
        get_initial_results(),
        placeholder="Select a player...",
        plugin_actions=get_initial_actions(),
    )


@plugin.on_search
async def handle_search(query: str, context: Optional[str]):
    """Handle search request."""
    query_lower = query.lower() if query else ""

    if context and context.startswith("controls:"):
        # Controls view - filter control options
        player_name = context.split(":", 1)[1]
        filtered = (
            [
                c
                for c in CONTROL_RESULTS
                if query_lower in c["name"].lower() or query_lower in c["id"]
            ]
            if query_lower
            else CONTROL_RESULTS
        )
        results = [control_to_result(c, player_name) for c in filtered]
        return HamrPlugin.results(
            results
            if results
            else [
                {
                    "id": "__no_match__",
                    "name": f"No controls matching '{query}'",
                    "icon": "search_off",
                }
            ],
            plugin_actions=get_control_actions(player_name),
        )
    else:
        # Players view
        state["context"] = ""
        players = get_players()
        filtered = (
            [
                p
                for p in players
                if query_lower in p["name"].lower()
                or query_lower in p["title"].lower()
                or query_lower in p["artist"].lower()
            ]
            if query_lower
            else players
        )

        if not filtered:
            return HamrPlugin.results(
                [
                    {
                        "id": "__no_match__",
                        "name": f"No players matching '{query}'",
                        "icon": "search_off",
                    }
                ],
                plugin_actions=get_initial_actions(),
            )

        return HamrPlugin.results(
            [player_to_result(p) for p in filtered],
            plugin_actions=get_initial_actions(),
        )


@plugin.on_action
async def handle_action(item_id: str, action: Optional[str], context: Optional[str]):
    """Handle action request."""
    if item_id == "__plugin__":
        if action == "refresh":
            state["context"] = ""
            return HamrPlugin.results(
                get_initial_results(),
                placeholder="Select a player...",
                plugin_actions=get_initial_actions(),
            )

        if ":" in action:
            cmd_type, player_name = action.split(":", 1)
            cmd_map = {
                "play-pause": ["play-pause"],
                "previous": ["previous"],
                "next": ["next"],
                "stop": ["stop"],
            }
            if cmd_type in cmd_map:
                run_player_command(player_name, cmd_map[cmd_type])
                return {"type": "execute", "close": False}

    if item_id in ("__no_players__", "__no_match__"):
        return {}

    if item_id == "__back__":
        state["context"] = ""
        return HamrPlugin.results(
            get_initial_results(),
            placeholder="Select a player...",
            plugin_actions=get_initial_actions(),
        )

    if item_id.startswith("player:"):
        player_name = item_id.split(":", 1)[1]

        if action == "more":
            state["context"] = f"controls:{player_name}"
            results = [control_to_result(c, player_name) for c in CONTROL_RESULTS]
            return HamrPlugin.results(
                results,
                placeholder=f"Controls for {player_name}...",
                plugin_actions=get_control_actions(player_name),
            )

        cmd_map = {
            "previous": ["previous"],
            "next": ["next"],
            "stop": ["stop"],
        }

        if action in cmd_map:
            run_player_command(player_name, cmd_map[action])
            return {"type": "execute", "close": False}

        if not action:
            # Get current status to determine command
            status, _ = run_playerctl(["-p", player_name, "status"])
            if status.lower() == "playing":
                run_player_command(player_name, ["pause"])
            else:
                run_player_command(player_name, ["play"])
            return {"type": "execute", "close": False}

    if item_id.startswith("control:"):
        parts = item_id.split(":", 2)
        if len(parts) == 3:
            player_name = parts[1]
            control_id = parts[2]
            control = next((c for c in CONTROL_RESULTS if c["id"] == control_id), None)

            if control:
                run_player_command(player_name, control["cmd"])
                return {"type": "execute", "close": False}

    return {}


def get_control_actions(player_name: str) -> list[dict]:
    """Get control plugin actions for a player."""
    return [
        {"id": f"play-pause:{player_name}", "name": "Play/Pause", "icon": "play_pause"},
        {"id": f"previous:{player_name}", "name": "Previous", "icon": "skip_previous"},
        {"id": f"next:{player_name}", "name": "Next", "icon": "skip_next"},
        {"id": f"stop:{player_name}", "name": "Stop", "icon": "stop"},
    ]


@plugin.add_background_task
async def update_progress(p: HamrPlugin) -> None:
    """Background task to update player progress every second."""
    last_players: dict[
        str, tuple[int, int, str]
    ] = {}  # name -> (position, duration, status)

    while True:
        await asyncio.sleep(1)

        # Only update if we're in the players view (not controls view)
        if state["context"] and state["context"].startswith("controls:"):
            continue

        players = get_players()
        if not players:
            continue

        updates = []
        for player in players:
            player_name = player["name"]
            position, duration = get_player_position(player_name)
            status = player["status"]

            # Check if anything changed
            last = last_players.get(player_name)
            if (
                last
                and last[0] == position
                and last[1] == duration
                and last[2] == status
            ):
                continue

            last_players[player_name] = (position, duration, status)

            # Build update for this player
            update: dict = {
                "id": f"player:{player_name}",
                "badges": [get_status_badge(status)],
            }

            # Update name with status
            name = player["title"] or player_name
            status_text = f"[{status}]"
            update["name"] = f"{name} {status_text}"
            update["verb"] = "Pause" if status.lower() == "playing" else "Play"

            if duration > 0:
                update["progress"] = {
                    "value": position,
                    "max": duration,
                    "label": f"{format_time(position)} / {format_time(duration)}",
                }

            updates.append(update)

        if updates:
            await p.send_update(updates)


if __name__ == "__main__":
    plugin.run()
