#!/usr/bin/env python3
"""
Screen recorder plugin for hamr.
Uses wf-recorder for recording, slurp for region selection.
Automatically trims the end of recordings to remove hamr UI.
"""

import json
import os
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path

# Test mode - mock external tool calls
TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

# Directories
VIDEOS_DIR = Path.home() / "Videos"
CACHE_DIR = Path.home() / ".cache" / "hamr"
LAUNCH_TIMESTAMP_FILE = CACHE_DIR / "launch_timestamp"
RECORDING_STATE_FILE = CACHE_DIR / "screenrecord_state.json"

# Timing constants
START_DELAY_SECONDS = 3
TRIM_BUFFER_MS = 500  # Extra buffer to ensure hamr animation is trimmed


def is_recording() -> bool:
    """Check if wf-recorder is currently running."""
    if TEST_MODE:
        return False  # Not recording in test mode
    try:
        return (
            subprocess.run(["pgrep", "wf-recorder"], capture_output=True).returncode
            == 0
        )
    except FileNotFoundError:
        return False


def get_focused_monitor() -> str:
    """Get the name of the currently focused monitor."""
    if TEST_MODE:
        return "eDP-1"  # Mock monitor name
    try:
        result = subprocess.run(
            ["hyprctl", "monitors", "-j"], capture_output=True, text=True, timeout=5
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
    if TEST_MODE:
        return "alsa_output.pci-0000_00_1f.3.analog-stereo.monitor"  # Mock audio source
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


def get_hamr_launch_time() -> int:
    """Get timestamp (ms) when hamr was last opened."""
    try:
        return int(LAUNCH_TIMESTAMP_FILE.read_text().strip())
    except (FileNotFoundError, ValueError):
        return int(time.time() * 1000)


def save_recording_state(recording_path: str, start_time_ms: int) -> None:
    """Save recording state for later trimming."""
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    state = {
        "recording_path": recording_path,
        "start_time_ms": start_time_ms,
    }
    RECORDING_STATE_FILE.write_text(json.dumps(state))


def load_recording_state() -> dict:
    """Load recording state."""
    try:
        return json.loads(RECORDING_STATE_FILE.read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return {}


def clear_recording_state() -> None:
    """Clear recording state file."""
    try:
        RECORDING_STATE_FILE.unlink()
    except FileNotFoundError:
        pass


def get_video_duration(video_path: str) -> float:
    """Get video duration in seconds using ffprobe."""
    try:
        result = subprocess.run(
            [
                "ffprobe",
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                video_path,
            ],
            capture_output=True,
            text=True,
            timeout=10,
        )
        return float(result.stdout.strip())
    except (subprocess.TimeoutExpired, ValueError):
        return 0.0


def trim_video_end(video_path: str, trim_seconds: float) -> bool:
    """Trim the end of a video using ffmpeg. Returns True on success."""
    if trim_seconds <= 0:
        return True

    duration = get_video_duration(video_path)
    if duration <= 0:
        return False

    new_duration = duration - trim_seconds
    if new_duration <= 0:
        return False

    # Create temp output path
    temp_path = video_path.replace(".mp4", "_trimmed.mp4")

    try:
        result = subprocess.run(
            [
                "ffmpeg",
                "-y",  # Overwrite
                "-i",
                video_path,
                "-t",
                str(new_duration),
                "-c",
                "copy",  # No re-encoding
                temp_path,
            ],
            capture_output=True,
            timeout=60,
        )

        if result.returncode == 0:
            # Replace original with trimmed version
            Path(temp_path).replace(video_path)
            return True
        else:
            # Clean up temp file on failure
            Path(temp_path).unlink(missing_ok=True)
            return False
    except subprocess.TimeoutExpired:
        Path(temp_path).unlink(missing_ok=True)
        return False


def build_start_record_script(
    output_path: str, monitor: str = "", region: bool = False, audio: bool = False
) -> str:
    """Build shell script for starting recording with delay."""
    audio_source = get_audio_source()
    audio_flag = f'--audio="{audio_source}"' if audio and audio_source else ""

    if region:
        # Region selection with slurp
        record_cmd = f"wf-recorder --pixel-format yuv420p -f '{output_path}' --geometry \"$region\" {audio_flag}"
        region_select = 'region=$(slurp 2>&1) || { notify-send "Recording cancelled" "Selection was cancelled"; exit 1; }'
    else:
        # Full screen recording
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


def build_stop_record_script(recording_path: str, trim_seconds: float) -> str:
    """Build shell script for stopping and trimming recording."""
    if trim_seconds > 0:
        trim_cmd = f"""
# Trim the end of the recording
duration=$(ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 '{recording_path}')
new_duration=$(echo "$duration - {trim_seconds}" | bc)
if (( $(echo "$new_duration > 0" | bc -l) )); then
    temp_path='{recording_path.replace(".mp4", "_trimmed.mp4")}'
    ffmpeg -y -i '{recording_path}' -t "$new_duration" -c copy "$temp_path" 2>/dev/null
    if [ $? -eq 0 ]; then
        mv "$temp_path" '{recording_path}'
        notify-send "Recording Saved" "Trimmed {trim_seconds:.1f}s from end"
    else
        rm -f "$temp_path"
        notify-send "Recording Saved" "Could not trim, saved original"
    fi
else
    notify-send "Recording Saved" "Too short to trim"
fi
"""
    else:
        trim_cmd = 'notify-send "Recording Saved" "Saved to Videos folder"'

    script = f"""
pkill -INT wf-recorder
sleep 0.5
{trim_cmd}
"""
    return script.strip()


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    selected = input_data.get("selected", {})

    if step in ("initial", "search"):
        results = []

        if is_recording():
            # Recording in progress - show stop option
            results.append(
                {
                    "id": "stop",
                    "name": "Stop Recording",
                    "icon": "stop_circle",
                    "description": "Stop and save current recording",
                }
            )
        else:
            # Show recording options
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

        # Always show browse option
        results.append(
            {
                "id": "browse",
                "name": "Open Recordings Folder",
                "icon": "folder_open",
                "description": str(VIDEOS_DIR),
            }
        )

        print(json.dumps({"type": "results", "results": results}))
        return

    if step == "action":
        item_id = selected.get("id", "")

        if item_id == "stop":
            # Calculate trim amount based on when hamr was opened
            launch_time_ms = get_hamr_launch_time()
            now_ms = int(time.time() * 1000)
            time_since_launch_ms = now_ms - launch_time_ms

            # Add buffer to ensure animation is fully trimmed
            trim_ms = time_since_launch_ms + TRIM_BUFFER_MS
            trim_seconds = trim_ms / 1000.0

            # Get recording path from state
            state = load_recording_state()
            recording_path = state.get("recording_path", "")

            if recording_path and Path(recording_path).exists():
                script = build_stop_record_script(recording_path, trim_seconds)
            else:
                # Fallback: just stop without trimming
                script = """
pkill -INT wf-recorder
notify-send "Recording Stopped" "Saved to Videos folder"
"""

            clear_recording_state()

            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["bash", "-c", script],
                            "close": True,
                        },
                    }
                )
            )
            return

        if item_id == "browse":
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["xdg-open", str(VIDEOS_DIR)],
                            "close": True,
                        },
                    }
                )
            )
            return

        # Recording actions
        output_path = get_output_path()
        monitor = get_focused_monitor()

        if item_id == "record_screen":
            script = build_start_record_script(output_path, monitor=monitor)
            # Save state for later trimming
            start_time_ms = int(time.time() * 1000) + (START_DELAY_SECONDS * 1000)
            save_recording_state(output_path, start_time_ms)

            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["bash", "-c", script],
                            "close": True,
                        },
                    }
                )
            )
            return

        if item_id == "record_screen_audio":
            script = build_start_record_script(output_path, monitor=monitor, audio=True)
            start_time_ms = int(time.time() * 1000) + (START_DELAY_SECONDS * 1000)
            save_recording_state(output_path, start_time_ms)

            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["bash", "-c", script],
                            "close": True,
                        },
                    }
                )
            )
            return

        if item_id == "record_region":
            script = build_start_record_script(output_path, region=True)
            start_time_ms = int(time.time() * 1000) + (START_DELAY_SECONDS * 1000)
            save_recording_state(output_path, start_time_ms)

            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["bash", "-c", script],
                            "close": True,
                        },
                    }
                )
            )
            return

        if item_id == "record_region_audio":
            script = build_start_record_script(output_path, region=True, audio=True)
            start_time_ms = int(time.time() * 1000) + (START_DELAY_SECONDS * 1000)
            save_recording_state(output_path, start_time_ms)

            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["bash", "-c", script],
                            "close": True,
                        },
                    }
                )
            )
            return

    print(json.dumps({"type": "error", "message": f"Unknown action: {selected}"}))


if __name__ == "__main__":
    main()
