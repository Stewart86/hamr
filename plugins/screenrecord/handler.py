#!/usr/bin/env python3
"""
Screen recorder plugin for hamr.

Uses wf-recorder for recording, slurp for region selection.
"""

import json
import os
import subprocess
import sys
from datetime import datetime
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

IS_NIRI = bool(os.environ.get("NIRI_SOCKET"))

VIDEOS_DIR = Path.home() / "Videos"
START_DELAY_SECONDS = 3


def is_recording() -> bool:
    """Check if wf-recorder is currently running."""
    try:
        return (
            subprocess.run(["pgrep", "wf-recorder"], capture_output=True).returncode
            == 0
        )
    except FileNotFoundError:
        return False


def has_wf_recorder() -> bool:
    """Check if wf-recorder is installed."""
    try:
        subprocess.run(["which", "wf-recorder"], capture_output=True, check=True)
        return True
    except (subprocess.CalledProcessError, FileNotFoundError):
        return False


def get_focused_monitor() -> str:
    """Get the name of the currently focused monitor."""
    try:
        if IS_NIRI:
            result = subprocess.run(
                ["niri", "msg", "-j", "focused-output"],
                capture_output=True,
                text=True,
                timeout=5,
            )
            output = json.loads(result.stdout)
            return output.get("name", "")
        else:
            result = subprocess.run(
                ["hyprctl", "monitors", "-j"],
                capture_output=True,
                text=True,
                timeout=5,
            )
            monitors = json.loads(result.stdout)
            for monitor in monitors:
                if monitor.get("focused"):
                    return monitor.get("name", "")
    except (
        subprocess.TimeoutExpired,
        json.JSONDecodeError,
        KeyError,
        FileNotFoundError,
    ):
        pass
    return ""


def get_audio_source() -> str:
    """Get the monitor audio source for recording system audio."""
    try:
        result = subprocess.run(
            ["pactl", "list", "sources"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        for line in result.stdout.split("\n"):
            if "Name:" in line and "monitor" in line.lower():
                return line.split("Name:")[1].strip()
    except (subprocess.TimeoutExpired, FileNotFoundError):
        pass
    return ""


def get_output_path() -> str:
    """Generate output path for recording."""
    VIDEOS_DIR.mkdir(parents=True, exist_ok=True)
    timestamp = datetime.now().strftime("%Y-%m-%d_%H.%M.%S")
    return str(VIDEOS_DIR / f"recording_{timestamp}.mp4")


def build_start_record_script(
    output_path: str, monitor: str = "", region: bool = False, audio: bool = False
) -> str:
    """Build shell script for starting recording with delay."""
    audio_source = get_audio_source()
    audio_flag = f'--audio="{audio_source}"' if audio and audio_source else ""

    if region:
        record_cmd = f"wf-recorder --pixel-format yuv420p -f '{output_path}' --geometry \"$region\" {audio_flag}"
        region_select = 'region=$(slurp 2>&1) || { notify-send "Recording cancelled" "Selection was cancelled"; exit 1; }'
    else:
        monitor_flag = f'-o "{monitor}"' if monitor else ""
        record_cmd = f"wf-recorder --pixel-format yuv420p -f '{output_path}' {monitor_flag} {audio_flag}"
        region_select = ""

    script = f"""
{region_select}
notify-send "Screen Recording" "Recording starts in {START_DELAY_SECONDS} seconds..." -t 2500
sleep {START_DELAY_SECONDS}
{record_cmd}
"""
    return script.strip()


def build_stop_record_script() -> str:
    """Build shell script for stopping recording."""
    return (
        'pkill -INT wf-recorder; notify-send "Recording Saved" "Saved to Videos folder"'
    )


def get_results() -> list[dict]:
    """Get results based on current recording state."""
    results = []
    recording = is_recording()

    if recording:
        results.append(
            {
                "id": "stop",
                "name": "Stop Recording",
                "icon": "stop_circle",
                "description": "Stop and save current recording",
            }
        )
    else:
        results.extend(
            [
                {
                    "id": "record_screen",
                    "name": "Record Screen",
                    "icon": "screen_record",
                    "description": f"Record focused monitor (starts in {START_DELAY_SECONDS}s)",
                },
                {
                    "id": "record_screen_audio",
                    "name": "Record Screen with Audio",
                    "icon": "mic",
                    "description": f"Record with system audio (starts in {START_DELAY_SECONDS}s)",
                },
                {
                    "id": "record_region",
                    "name": "Record Region",
                    "icon": "crop",
                    "description": f"Select area to record (starts in {START_DELAY_SECONDS}s)",
                },
                {
                    "id": "record_region_audio",
                    "name": "Record Region with Audio",
                    "icon": "settings_voice",
                    "description": f"Select area with audio (starts in {START_DELAY_SECONDS}s)",
                },
            ]
        )

    results.append(
        {
            "id": "browse",
            "name": "Open Recordings Folder",
            "icon": "folder_open",
            "description": str(VIDEOS_DIR),
        }
    )

    return results


plugin = HamrPlugin(
    id="screenrecord",
    name="Screen Record",
    description="Record screen or region with wf-recorder",
    icon="screen_record",
)


@plugin.on_initial
async def handle_initial(params=None):
    """Handle initial request when plugin is opened."""
    if not has_wf_recorder():
        return HamrPlugin.error(
            "wf-recorder not found. Install it to use screen recording."
        )

    return HamrPlugin.results(
        get_results(),
        placeholder="Select recording option...",
    )


@plugin.on_search
async def handle_search(query: str, context=None):
    """Handle search request."""
    results = get_results()
    if query:
        query_lower = query.lower()
        results = [
            r
            for r in results
            if query_lower in r["name"].lower()
            or query_lower in r.get("description", "").lower()
        ]

    if not results:
        results = [
            {
                "id": "__empty__",
                "name": f"No options matching '{query}'",
                "icon": "search_off",
            }
        ]

    return HamrPlugin.results(results, placeholder="Select recording option...")


@plugin.on_action
async def handle_action(item_id: str, action=None, context=None):
    """Handle action request."""
    if item_id == "__empty__":
        return HamrPlugin.close()

    if item_id == "stop":
        script = build_stop_record_script()
        subprocess.Popen(["bash", "-c", script], start_new_session=True)
        return HamrPlugin.close()

    if item_id == "browse":
        subprocess.Popen(["xdg-open", str(VIDEOS_DIR)], start_new_session=True)
        return HamrPlugin.close()

    output_path = get_output_path()
    monitor = get_focused_monitor()

    if item_id == "record_screen":
        script = build_start_record_script(output_path, monitor=monitor)
    elif item_id == "record_screen_audio":
        script = build_start_record_script(output_path, monitor=monitor, audio=True)
    elif item_id == "record_region":
        script = build_start_record_script(output_path, region=True)
    elif item_id == "record_region_audio":
        script = build_start_record_script(output_path, region=True, audio=True)
    else:
        return HamrPlugin.error(f"Unknown action: {item_id}")

    subprocess.Popen(["bash", "-c", script], start_new_session=True)
    return HamrPlugin.close()


if __name__ == "__main__":
    plugin.run()
