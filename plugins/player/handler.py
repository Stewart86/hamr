#!/usr/bin/env python3
import json
import os
import signal
import select
import subprocess
import sys
import time

TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

shutdown = False

MOCK_PLAYERS = [
    {
        "name": "spotify",
        "status": "Playing",
        "title": "Bohemian Rhapsody",
        "artist": "Queen",
        "album": "A Night at the Opera",
        "artUrl": "file:///tmp/album-art.jpg",
    },
    {
        "name": "firefox",
        "status": "Paused",
        "title": "YouTube Video",
        "artist": "YouTube",
        "album": "",
        "artUrl": "",
    },
]


def emit(data: dict):
    print(json.dumps(data), flush=True)


def handle_shutdown(signum, frame):
    global shutdown
    shutdown = True


def run_playerctl(args: list[str]) -> tuple[str, int]:
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
    if TEST_MODE:
        return MOCK_PLAYERS

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
    if seconds < 0:
        return "0:00"
    mins = seconds // 60
    secs = seconds % 60
    return f"{mins}:{secs:02d}"


def get_player_position(player_name: str) -> tuple[int, int]:
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
    status_lower = status.lower()
    if status_lower == "playing":
        return "play_arrow"
    if status_lower == "paused":
        return "pause"
    if status_lower == "stopped":
        return "stop"
    return "music_note"


def get_status_badge(status: str) -> dict:
    status_lower = status.lower()
    if status_lower == "playing":
        return {"icon": "play_arrow", "color": "#4caf50"}
    if status_lower == "paused":
        return {"icon": "pause", "color": "#ff9800"}
    if status_lower == "stopped":
        return {"icon": "stop", "color": "#f44336"}
    return {"icon": "music_note", "color": "#2196f3"}


def get_art_path(art_url: str) -> str | None:
    """Convert art URL to local file path if it's a file:// URL"""
    if not art_url:
        return None
    if art_url.startswith("file://"):
        return art_url[7:]  # Strip "file://"
    return None


def player_to_result(player: dict) -> dict:
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


def get_initial_plugin_actions() -> list[dict]:
    return [
        {"id": "refresh", "name": "Refresh", "icon": "refresh"},
    ]


def get_control_plugin_actions(player_name: str) -> list[dict]:
    return [
        {"id": f"play-pause:{player_name}", "name": "Play/Pause", "icon": "play_pause"},
        {"id": f"previous:{player_name}", "name": "Previous", "icon": "skip_previous"},
        {"id": f"next:{player_name}", "name": "Next", "icon": "skip_next"},
        {"id": f"stop:{player_name}", "name": "Stop", "icon": "stop"},
    ]


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
    return {
        "id": f"control:{player_name}:{control['id']}",
        "name": control["name"],
        "icon": control["icon"],
        "verb": "Set",
    }


def run_player_command(player_name: str, cmd: list[str]):
    if not TEST_MODE:
        run_playerctl(["-p", player_name] + cmd)


def return_players_view():
    players = get_players()
    if not players:
        emit(
            {
                "type": "results",
                "results": [
                    {
                        "id": "__no_players__",
                        "name": "No media players detected",
                        "description": "Start playing media in a supported application",
                        "icon": "music_off",
                    }
                ],
                "placeholder": "Waiting for players...",
                "pluginActions": get_initial_plugin_actions(),
            }
        )
    else:
        emit(
            {
                "type": "results",
                "results": [player_to_result(p) for p in players],
                "placeholder": "Select a player...",
                "pluginActions": get_initial_plugin_actions(),
            }
        )


def return_controls_view(player_name: str, navigate_forward: bool = False):
    results = [control_to_result(c, player_name) for c in CONTROL_RESULTS]
    response = {
        "type": "results",
        "results": results,
        "placeholder": f"Controls for {player_name}...",
        "context": f"controls:{player_name}",
        "pluginActions": get_control_plugin_actions(player_name),
    }
    if navigate_forward:
        response["navigateForward"] = True
        response["clearInput"] = True
    emit(response)


def handle_step(input_data: dict):
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip().lower()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")
    context = input_data.get("context", "")

    if step == "initial":
        return_players_view()
        return

    if step == "search":
        if context.startswith("controls:"):
            player_name = context.split(":", 1)[1]
            filtered = (
                [
                    c
                    for c in CONTROL_RESULTS
                    if query in c["name"].lower() or query in c["id"]
                ]
                if query
                else CONTROL_RESULTS
            )
            results = [control_to_result(c, player_name) for c in filtered]
            emit(
                {
                    "type": "results",
                    "results": results
                    if results
                    else [
                        {
                            "id": "__no_match__",
                            "name": f"No controls matching '{query}'",
                            "icon": "search_off",
                        }
                    ],
                    "context": context,
                    "pluginActions": get_control_plugin_actions(player_name),
                }
            )
            return

        players = get_players()
        filtered = (
            [
                p
                for p in players
                if query in p["name"].lower()
                or query in p["title"].lower()
                or query in p["artist"].lower()
            ]
            if query
            else players
        )

        if not filtered:
            emit(
                {
                    "type": "results",
                    "results": [
                        {
                            "id": "__no_match__",
                            "name": f"No players matching '{query}'",
                            "icon": "search_off",
                        }
                    ],
                    "pluginActions": get_initial_plugin_actions(),
                }
            )
            return

        emit(
            {
                "type": "results",
                "results": [player_to_result(p) for p in filtered],
                "pluginActions": get_initial_plugin_actions(),
            }
        )
        return

    if step == "action":
        selected_id = selected.get("id", "")

        if selected_id == "__plugin__":
            if action == "refresh":
                return_players_view()
                return

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
                    emit(
                        {
                            "type": "execute",
                            "close": False,
                        }
                    )
                    return

        if selected_id in ("__no_players__", "__no_match__"):
            emit({"type": "execute", "close": False})
            return

        if selected_id == "__back__":
            return_players_view()
            return

        if selected_id.startswith("player:"):
            player_name = selected_id.split(":", 1)[1]

            if action == "more":
                return_controls_view(player_name, navigate_forward=True)
                return

            cmd_map = {
                "previous": ["previous"],
                "next": ["next"],
                "stop": ["stop"],
            }

            if action in cmd_map:
                run_player_command(player_name, cmd_map[action])
                emit(
                    {
                        "type": "execute",
                        "close": False,
                    }
                )
                return

            if not action:
                # Get current status to determine command
                status, _ = run_playerctl(["-p", player_name, "status"])
                if status.lower() == "playing":
                    run_player_command(player_name, ["pause"])
                else:
                    run_player_command(player_name, ["play"])
                emit(
                    {
                        "type": "execute",
                        "close": False,
                    }
                )
                return

        if selected_id.startswith("control:"):
            parts = selected_id.split(":", 2)
            if len(parts) == 3:
                player_name = parts[1]
                control_id = parts[2]
                control = next(
                    (c for c in CONTROL_RESULTS if c["id"] == control_id), None
                )

                if control:
                    run_player_command(player_name, control["cmd"])
                    emit(
                        {
                            "type": "execute",
                            "close": False,
                        }
                    )
                    return

        emit({"type": "error", "message": f"Unknown selection: {selected_id}"})
        return

    emit({"type": "error", "message": f"Unknown step: {step}"})


def main():
    signal.signal(signal.SIGTERM, handle_shutdown)
    signal.signal(signal.SIGINT, handle_shutdown)

    last_refresh = 0
    current_context = ""  # Track current view: "" = players, "controls:X" = controls

    while not shutdown:
        try:
            ready, _, _ = select.select([sys.stdin], [], [], 0.5)
            if ready:
                line = sys.stdin.readline()
                if not line:
                    break
                input_data = json.loads(line.strip())
                # Track context from input
                current_context = input_data.get("context", "")
                handle_step(input_data)
                last_refresh = time.time()
            elif time.time() - last_refresh > 1.0:
                # Only auto-refresh on players view, not controls
                if not current_context.startswith("controls:"):
                    return_players_view()
                last_refresh = time.time()
        except json.JSONDecodeError:
            pass
        except Exception:
            pass


if __name__ == "__main__":
    main()
