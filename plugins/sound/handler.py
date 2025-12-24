#!/usr/bin/env python3
import json
import os
import subprocess
import sys

TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

SOUND_ACTIONS = [
    {
        "id": "vol-up",
        "name": "Volume Up",
        "description": "Increase volume by 5%",
        "icon": "volume_up",
        "cmd": ["wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", "5%+"],
    },
    {
        "id": "vol-down",
        "name": "Volume Down",
        "description": "Decrease volume by 5%",
        "icon": "volume_down",
        "cmd": ["wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", "5%-"],
    },
    {
        "id": "mute-toggle",
        "name": "Toggle Mute",
        "description": "Mute/unmute audio",
        "icon": "volume_off",
        "cmd": ["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"],
    },
    {
        "id": "mic-mute-toggle",
        "name": "Toggle Mic Mute",
        "description": "Mute/unmute microphone",
        "icon": "mic_off",
        "cmd": ["wpctl", "set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"],
    },
]


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


def action_to_result(action: dict) -> dict:
    return {
        "id": action["id"],
        "name": action["name"],
        "description": action["description"],
        "icon": action["icon"],
        "verb": "Run",
    }


def action_to_index_item(action: dict) -> dict:
    return {
        "id": f"sound:{action['id']}",
        "name": action["name"],
        "description": action["description"],
        "icon": action["icon"],
        "verb": "Run",
        "keywords": ["sound", "volume", "audio", action["id"].replace("-", " ")],
        "execute": {
            "command": action["cmd"],
            "name": action["name"],
            "icon": action["icon"],
        },
        "keepOpen": True,
    }


def get_plugin_actions() -> list[dict]:
    return [
        {"id": "vol-down", "name": "Vol-", "icon": "volume_down"},
        {"id": "vol-up", "name": "Vol+", "icon": "volume_up"},
        {"id": "mute-toggle", "name": "Mute", "icon": "volume_off"},
    ]


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip().lower()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    if step == "index":
        items = [action_to_index_item(a) for a in SOUND_ACTIONS]
        print(json.dumps({"type": "index", "items": items}))
        return

    if step == "initial":
        vol_info = get_volume_info()
        vol_pct = int(vol_info["volume"] * 100)
        mute_status = " [MUTED]" if vol_info["muted"] else ""

        results = [action_to_result(a) for a in SOUND_ACTIONS]
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "placeholder": f"Volume: {vol_pct}%{mute_status}",
                    "pluginActions": get_plugin_actions(),
                }
            )
        )
        return

    if step == "search":
        filtered = [
            a
            for a in SOUND_ACTIONS
            if query in a["id"]
            or query in a["name"].lower()
            or query in a["description"].lower()
        ]
        results = [action_to_result(a) for a in filtered]
        if not results:
            results = [
                {
                    "id": "__no_match__",
                    "name": f"No actions matching '{query}'",
                    "icon": "search_off",
                }
            ]
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "pluginActions": get_plugin_actions(),
                }
            )
        )
        return

    if step == "action":
        selected_id = selected.get("id", "")

        if selected_id == "__plugin__" and action:
            selected_id = action

        if selected_id == "__no_match__":
            print(json.dumps({"type": "execute", "execute": {"close": False}}))
            return

        sound_action = next((a for a in SOUND_ACTIONS if a["id"] == selected_id), None)
        if sound_action:
            run_cmd(sound_action["cmd"])
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {"close": False},
                    }
                )
            )
            return

        print(
            json.dumps({"type": "error", "message": f"Unknown action: {selected_id}"})
        )
        return

    print(json.dumps({"type": "error", "message": f"Unknown step: {step}"}))


if __name__ == "__main__":
    main()
