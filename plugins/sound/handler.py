#!/usr/bin/env python3
import json
import os
import subprocess
import sys

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
            "value": vol_pct,
            "min": 0,
            "max": 100,
            "step": 5,
            "unit": "%",
            "badges": [
                {
                    "icon": "volume_off" if vol_info["muted"] else "volume_up",
                    "background": "#f44336" if vol_info["muted"] else "#4caf50",
                    "color": "#ffffff",
                }
            ],
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
            "value": mic_pct,
            "min": 0,
            "max": 100,
            "step": 5,
            "unit": "%",
            "badges": [
                {
                    "icon": "mic_off" if mic_info["muted"] else "mic",
                    "background": "#f44336" if mic_info["muted"] else "#4caf50",
                    "color": "#ffffff",
                }
            ],
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


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    if step == "index":
        vol_info = get_volume_info()
        vol_pct = int(vol_info["volume"] * 100)
        items = [
            {
                "id": "sound:volume",
                "name": "Volume Control",
                "description": f"Current: {vol_pct}%",
                "icon": get_volume_icon(vol_info["volume"], vol_info["muted"]),
                "verb": "Open",
                "keywords": ["sound", "volume", "audio", "speaker"],
                "entryPoint": {"step": "initial"},
                "keepOpen": True,
            },
            {
                "id": "sound:mute",
                "name": "Toggle Mute",
                "description": "Mute/unmute audio",
                "icon": "volume_off",
                "verb": "Toggle",
                "keywords": ["sound", "mute", "audio", "silent"],
                "execute": {
                    "command": ["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"],
                    "name": "Toggle Mute",
                },
            },
        ]
        print(json.dumps({"type": "index", "items": items}))
        return

    vol_info = get_volume_info()
    mic_info = get_mic_info()

    if step in ("initial", "search"):
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": get_results(),
                    "pluginActions": get_plugin_actions(vol_info, mic_info),
                }
            )
        )
        return

    if step == "action":
        selected_id = selected.get("id", "")

        if selected_id == "__plugin__" and action:
            selected_id = action

        # Handle slider value change
        if action == "slider":
            new_value = int(input_data.get("value", 0))

            if selected_id == "volume":
                set_volume(new_value)
            elif selected_id == "mic":
                set_mic_volume(new_value)

            print(json.dumps({"type": "noop"}))
            return

        # Handle mute toggles (from action buttons on slider items or plugin actions)
        if action == "mute-toggle" or selected_id == "mute-toggle":
            run_cmd(["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
        elif action == "mic-mute-toggle" or selected_id == "mic-mute-toggle":
            run_cmd(["wpctl", "set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"])

        vol_info = get_volume_info()
        mic_info = get_mic_info()
        vol_pct = int(vol_info["volume"] * 100)
        mute_status = " [MUTED]" if vol_info["muted"] else ""
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": get_results(),
                    "placeholder": f"Volume: {vol_pct}%{mute_status}",
                    "pluginActions": get_plugin_actions(vol_info, mic_info),
                    "navigateForward": False,
                }
            )
        )
        return

    print(json.dumps({"type": "error", "message": f"Unknown step: {step}"}))


if __name__ == "__main__":
    main()
