#!/usr/bin/env python3
import json
import os
import select
import signal
import subprocess
import sys
import time

TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"


def run_cmd(cmd: list[str]) -> tuple[str, int]:
    if TEST_MODE:
        return "", 0
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=5)
        return result.stdout.strip(), result.returncode
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return "", 1


def get_volume_info() -> dict:
    if TEST_MODE:
        return {"volume": 0.50, "muted": False}

    output, code = run_cmd(["wpctl", "get-volume", "@DEFAULT_AUDIO_SINK@"])
    if code != 0:
        return {"volume": 0, "muted": False}

    volume = 0.0
    muted = False
    parts = output.split()
    if len(parts) >= 2:
        try:
            volume = float(parts[1])
        except ValueError:
            pass
    if "[MUTED]" in output:
        muted = True

    return {"volume": volume, "muted": muted}


def get_mic_info() -> dict:
    if TEST_MODE:
        return {"volume": 0.80, "muted": False}

    output, code = run_cmd(["wpctl", "get-volume", "@DEFAULT_AUDIO_SOURCE@"])
    if code != 0:
        return {"volume": 0, "muted": False}

    volume = 0.0
    muted = False
    parts = output.split()
    if len(parts) >= 2:
        try:
            volume = float(parts[1])
        except ValueError:
            pass
    if "[MUTED]" in output:
        muted = True

    return {"volume": volume, "muted": muted}


def set_volume(volume_pct: int) -> None:
    vol_decimal = max(0, min(100, volume_pct)) / 100.0
    run_cmd(["wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", str(vol_decimal)])


def set_mic_volume(volume_pct: int) -> None:
    vol_decimal = max(0, min(100, volume_pct)) / 100.0
    run_cmd(["wpctl", "set-volume", "@DEFAULT_AUDIO_SOURCE@", str(vol_decimal)])


def get_volume_icon(volume: float, muted: bool) -> str:
    if muted:
        return "volume_off"
    if volume <= 0:
        return "volume_mute"
    if volume < 0.5:
        return "volume_down"
    return "volume_up"


def get_results(
    vol_override: int | None = None, mic_override: int | None = None
) -> list[dict]:
    vol_info = get_volume_info()
    mic_info = get_mic_info()

    vol_pct = (
        vol_override if vol_override is not None else int(vol_info["volume"] * 100)
    )
    mic_pct = (
        mic_override if mic_override is not None else int(mic_info["volume"] * 100)
    )

    return [
        {
            "id": "volume",
            "type": "slider",
            "name": "Volume",
            "icon": get_volume_icon(vol_pct / 100, vol_info["muted"]),
            "gauge": {
                "value": vol_pct,
                "max": 100,
                "label": f"{vol_pct}%",
            },
            "value": vol_pct,
            "min": 0,
            "max": 100,
            "step": 5,
            "badges": [{"icon": "volume_off", "color": "#f44336"}]
            if vol_info["muted"]
            else [],
            "actions": [
                {
                    "id": "mute-toggle",
                    "name": "Unmute" if vol_info["muted"] else "Mute",
                    "icon": "volume_up" if vol_info["muted"] else "volume_off",
                }
            ],
        },
        {
            "id": "mic",
            "type": "slider",
            "name": "Microphone",
            "icon": "mic_off" if mic_info["muted"] else "mic",
            "gauge": {
                "value": mic_pct,
                "max": 100,
                "label": f"{mic_pct}%",
            },
            "value": mic_pct,
            "min": 0,
            "max": 100,
            "step": 5,
            "badges": [{"icon": "mic_off", "color": "#f44336"}]
            if mic_info["muted"]
            else [],
            "actions": [
                {
                    "id": "mic-mute-toggle",
                    "name": "Unmute" if mic_info["muted"] else "Mute",
                    "icon": "mic" if mic_info["muted"] else "mic_off",
                }
            ],
        },
    ]


def get_plugin_actions(vol_info: dict, mic_info: dict) -> list[dict]:
    return [
        {
            "id": "mute-toggle",
            "name": "Mute" if not vol_info["muted"] else "Unmute",
            "icon": "volume_off" if not vol_info["muted"] else "volume_up",
            "active": vol_info["muted"],
        },
        {
            "id": "mic-mute-toggle",
            "name": "Mute Mic" if not mic_info["muted"] else "Unmute Mic",
            "icon": "mic_off" if not mic_info["muted"] else "mic",
            "active": mic_info["muted"],
        },
    ]


def emit(data: dict) -> None:
    print(json.dumps(data), flush=True)


def emit_index() -> None:
    """Emit index with current volume state for search/history."""
    emit({"type": "index", "mode": "full", "items": get_results()})


def handle_request(request: dict) -> None:
    step = request.get("step", "initial")
    selected = request.get("selected", {})
    action = request.get("action", "")

    vol_info = get_volume_info()
    mic_info = get_mic_info()

    if step in ("initial", "search"):
        emit(
            {
                "type": "results",
                "results": get_results(),
                "pluginActions": get_plugin_actions(vol_info, mic_info),
            }
        )
        return

    if step == "action":
        selected_id = selected.get("id", "")

        if selected_id == "__plugin__" and action:
            selected_id = action

        if action == "slider":
            new_value = int(request.get("value", 0))

            if selected_id == "volume":
                set_volume(new_value)
            elif selected_id == "mic":
                set_mic_volume(new_value)

            vol_info = get_volume_info()
            mic_info = get_mic_info()

            # Get updated values for the changed slider
            if selected_id == "volume":
                vol_pct = int(vol_info["volume"] * 100)
                emit(
                    {
                        "type": "update",
                        "items": [
                            {
                                "id": "volume",
                                "value": vol_pct,
                                "gauge": {
                                    "value": vol_pct,
                                    "max": 100,
                                    "label": f"{vol_pct}%",
                                },
                                "icon": get_volume_icon(
                                    vol_info["volume"], vol_info["muted"]
                                ),
                            }
                        ],
                    }
                )
            elif selected_id == "mic":
                mic_pct = int(mic_info["volume"] * 100)
                emit(
                    {
                        "type": "update",
                        "items": [
                            {
                                "id": "mic",
                                "value": mic_pct,
                                "gauge": {
                                    "value": mic_pct,
                                    "max": 100,
                                    "label": f"{mic_pct}%",
                                },
                            }
                        ],
                    }
                )
            return

        if action == "mute-toggle" or selected_id == "mute-toggle":
            run_cmd(["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
        elif action == "mic-mute-toggle" or selected_id == "mic-mute-toggle":
            run_cmd(["wpctl", "set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"])

        vol_info = get_volume_info()
        mic_info = get_mic_info()
        vol_pct = int(vol_info["volume"] * 100)
        mute_status = " [MUTED]" if vol_info["muted"] else ""
        emit(
            {
                "type": "results",
                "results": get_results(),
                "placeholder": f"Volume: {vol_pct}%{mute_status}",
                "pluginActions": get_plugin_actions(vol_info, mic_info),
                "navigateForward": False,
            }
        )
        return

    emit({"type": "error", "message": f"Unknown step: {step}"})


def main():
    def shutdown_handler(signum, frame):
        sys.exit(0)

    signal.signal(signal.SIGTERM, shutdown_handler)
    signal.signal(signal.SIGINT, shutdown_handler)

    # Emit initial index on startup
    emit_index()

    # Track last known values to detect external changes
    last_vol = get_volume_info()
    last_mic = get_mic_info()

    while True:
        readable, _, _ = select.select([sys.stdin], [], [], 1.0)

        if readable:
            try:
                line = sys.stdin.readline()
                if not line:
                    break
                request = json.loads(line.strip())
                handle_request(request)
                # Update tracked values after handling request
                last_vol = get_volume_info()
                last_mic = get_mic_info()
            except (json.JSONDecodeError, ValueError):
                continue
        else:
            # Timeout - check for external volume changes
            current_vol = get_volume_info()
            current_mic = get_mic_info()

            updates = []

            if (
                current_vol["volume"] != last_vol["volume"]
                or current_vol["muted"] != last_vol["muted"]
            ):
                vol_pct = int(current_vol["volume"] * 100)
                updates.append(
                    {
                        "id": "volume",
                        "value": vol_pct,
                        "gauge": {"value": vol_pct, "max": 100, "label": f"{vol_pct}%"},
                        "icon": get_volume_icon(
                            current_vol["volume"], current_vol["muted"]
                        ),
                        "badges": [{"icon": "volume_off", "color": "#f44336"}]
                        if current_vol["muted"]
                        else [],
                    }
                )
                last_vol = current_vol

            if (
                current_mic["volume"] != last_mic["volume"]
                or current_mic["muted"] != last_mic["muted"]
            ):
                mic_pct = int(current_mic["volume"] * 100)
                updates.append(
                    {
                        "id": "mic",
                        "value": mic_pct,
                        "gauge": {"value": mic_pct, "max": 100, "label": f"{mic_pct}%"},
                        "icon": "mic_off" if current_mic["muted"] else "mic",
                        "badges": [{"icon": "mic_off", "color": "#f44336"}]
                        if current_mic["muted"]
                        else [],
                    }
                )
                last_mic = current_mic

            if updates:
                emit({"type": "update", "items": updates})


if __name__ == "__main__":
    main()
