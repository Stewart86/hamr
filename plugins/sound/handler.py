#!/usr/bin/env python3
"""
Sound plugin for hamr - Socket-based version

Provides system volume controls via WirePlumber:
- Volume slider with gauge
- Microphone slider with gauge
- Mute toggles for volume and mic
- Real-time monitoring for external changes
"""

import asyncio
import subprocess
import sys
from pathlib import Path
from typing import Any, Optional

# Add parent directory to path to import SDK
sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin


def run_cmd(cmd: list[str]) -> tuple[str, int]:
    """Run a command and return stdout and return code."""
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=5)
        return result.stdout.strip(), result.returncode
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return "", 1


def get_volume_info() -> dict[str, Any]:
    """Get current volume info from WirePlumber."""
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


def get_mic_info() -> dict[str, Any]:
    """Get current microphone info from WirePlumber."""
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
    """Set system volume."""
    vol_decimal = max(0, min(100, volume_pct)) / 100.0
    run_cmd(["wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", str(vol_decimal)])


def set_mic_volume(volume_pct: int) -> None:
    """Set microphone volume."""
    vol_decimal = max(0, min(100, volume_pct)) / 100.0
    run_cmd(["wpctl", "set-volume", "@DEFAULT_AUDIO_SOURCE@", str(vol_decimal)])


def set_mute(device: str, muted: bool) -> None:
    """Set mute state for a device."""
    run_cmd(["wpctl", "set-mute", device, "1" if muted else "0"])


def get_volume_icon(volume: float, muted: bool) -> str:
    """Get appropriate icon for volume level."""
    if muted:
        return "volume_off"
    if volume <= 0:
        return "volume_mute"
    if volume < 0.5:
        return "volume_down"
    return "volume_up"


def get_results() -> list[dict[str, Any]]:
    """Build results list with sliders and switches."""
    vol_info = get_volume_info()
    mic_info = get_mic_info()

    vol_pct = int(vol_info["volume"] * 100)
    mic_pct = int(mic_info["volume"] * 100)

    return [
        {
            "id": "volume",
            "name": "Volume",
            "icon": get_volume_icon(vol_info["volume"], vol_info["muted"]),
            "resultType": "slider",
            "value": {
                "value": vol_pct,
                "min": 0,
                "max": 100,
                "step": 5,
                "displayValue": f"{vol_pct}%",
            },
            "gauge": {
                "value": vol_pct,
                "max": 100,
                "label": f"{vol_pct}%",
            },
        },
        {
            "id": "volume-mute",
            "name": "Unmute Volume" if vol_info["muted"] else "Mute Volume",
            "description": "Volume is muted"
            if vol_info["muted"]
            else "Mute system audio output",
            "icon": "volume_off" if vol_info["muted"] else "volume_up",
            "resultType": "switch",
            "value": {
                "value": 1 if vol_info["muted"] else 0,
                "min": 0,
                "max": 1,
                "step": 1,
            },
        },
        {
            "id": "mic",
            "name": "Microphone",
            "icon": "mic_off" if mic_info["muted"] else "mic",
            "resultType": "slider",
            "value": {
                "value": mic_pct,
                "min": 0,
                "max": 100,
                "step": 5,
                "displayValue": f"{mic_pct}%",
            },
            "gauge": {
                "value": mic_pct,
                "max": 100,
                "label": f"{mic_pct}%",
            },
        },
        {
            "id": "mic-mute",
            "name": "Unmute Microphone" if mic_info["muted"] else "Mute Microphone",
            "description": "Microphone is muted"
            if mic_info["muted"]
            else "Mute microphone input",
            "icon": "mic_off" if mic_info["muted"] else "mic",
            "resultType": "switch",
            "value": {
                "value": 1 if mic_info["muted"] else 0,
                "min": 0,
                "max": 1,
                "step": 1,
            },
        },
    ]


# Create plugin instance
plugin = HamrPlugin(
    id="sound",
    name="Sound",
    description="System volume controls",
    icon="volume_up",
)

# Track last known values for change detection
state: dict[str, Any] = {
    "last_vol": None,
    "last_mic": None,
}


@plugin.on_initial
async def handle_initial(params=None) -> None:
    """Handle initial plugin open."""
    state["last_vol"] = get_volume_info()
    state["last_mic"] = get_mic_info()
    await plugin.send_results(get_results())


@plugin.on_search
async def handle_search(query: str, context: Optional[str]) -> None:
    """Handle search - just return current state."""
    await plugin.send_results(get_results())


@plugin.on_slider_changed
async def handle_slider(slider_id: str, value: float) -> None:
    """Handle slider value changes."""
    int_value = int(value)

    if slider_id == "volume":
        set_volume(int_value)
        state["last_vol"] = get_volume_info()
    elif slider_id == "mic":
        set_mic_volume(int_value)
        state["last_mic"] = get_mic_info()

    # Send updated results
    await plugin.send_update(
        [
            {
                "id": slider_id,
                "value": {
                    "value": int_value,
                    "min": 0,
                    "max": 100,
                    "step": 5,
                    "displayValue": f"{int_value}%",
                },
                "gauge": {"value": int_value, "max": 100, "label": f"{int_value}%"},
            }
        ]
    )


@plugin.on_switch_toggled
async def handle_switch(switch_id: str, value: bool) -> None:
    """Handle switch toggle."""
    if switch_id == "volume-mute":
        set_mute("@DEFAULT_AUDIO_SINK@", value)
        state["last_vol"] = get_volume_info()
    elif switch_id == "mic-mute":
        set_mute("@DEFAULT_AUDIO_SOURCE@", value)
        state["last_mic"] = get_mic_info()

    # Send updated switch state
    vol_info = get_volume_info() if switch_id == "volume-mute" else None
    mic_info = get_mic_info() if switch_id == "mic-mute" else None

    if vol_info:
        await plugin.send_update(
            [
                {
                    "id": "volume-mute",
                    "value": {
                        "value": 1 if vol_info["muted"] else 0,
                        "min": 0,
                        "max": 1,
                        "step": 1,
                    },
                    "name": "Unmute Volume" if vol_info["muted"] else "Mute Volume",
                    "description": "Volume is muted"
                    if vol_info["muted"]
                    else "Mute system audio output",
                    "icon": "volume_off" if vol_info["muted"] else "volume_up",
                }
            ]
        )
    elif mic_info:
        await plugin.send_update(
            [
                {
                    "id": "mic-mute",
                    "value": {
                        "value": 1 if mic_info["muted"] else 0,
                        "min": 0,
                        "max": 1,
                        "step": 1,
                    },
                    "name": "Unmute Microphone"
                    if mic_info["muted"]
                    else "Mute Microphone",
                    "description": "Microphone is muted"
                    if mic_info["muted"]
                    else "Mute microphone input",
                    "icon": "mic_off" if mic_info["muted"] else "mic",
                }
            ]
        )


@plugin.add_background_task
async def monitor_changes(p: HamrPlugin) -> None:
    """Background task to monitor external volume changes."""
    while True:
        await asyncio.sleep(1)

        current_vol = get_volume_info()
        current_mic = get_mic_info()

        updates: list[dict[str, Any]] = []

        # Check volume changes
        if state["last_vol"] and (
            current_vol["volume"] != state["last_vol"]["volume"]
            or current_vol["muted"] != state["last_vol"]["muted"]
        ):
            vol_pct = int(current_vol["volume"] * 100)
            updates.append(
                {
                    "id": "volume",
                    "value": {
                        "value": vol_pct,
                        "min": 0,
                        "max": 100,
                        "step": 5,
                        "displayValue": f"{vol_pct}%",
                    },
                    "gauge": {"value": vol_pct, "max": 100, "label": f"{vol_pct}%"},
                    "icon": get_volume_icon(
                        current_vol["volume"], current_vol["muted"]
                    ),
                }
            )
            updates.append(
                {
                    "id": "volume-mute",
                    "name": "Unmute Volume" if current_vol["muted"] else "Mute Volume",
                    "description": "Volume is muted"
                    if current_vol["muted"]
                    else "Mute system audio output",
                    "icon": "volume_off" if current_vol["muted"] else "volume_up",
                    "value": {
                        "value": 1 if current_vol["muted"] else 0,
                        "min": 0,
                        "max": 1,
                        "step": 1,
                    },
                }
            )
            state["last_vol"] = current_vol

        # Check mic changes
        if state["last_mic"] and (
            current_mic["volume"] != state["last_mic"]["volume"]
            or current_mic["muted"] != state["last_mic"]["muted"]
        ):
            mic_pct = int(current_mic["volume"] * 100)
            updates.append(
                {
                    "id": "mic",
                    "value": {
                        "value": mic_pct,
                        "min": 0,
                        "max": 100,
                        "step": 5,
                        "displayValue": f"{mic_pct}%",
                    },
                    "gauge": {"value": mic_pct, "max": 100, "label": f"{mic_pct}%"},
                    "icon": "mic_off" if current_mic["muted"] else "mic",
                }
            )
            updates.append(
                {
                    "id": "mic-mute",
                    "name": "Unmute Microphone"
                    if current_mic["muted"]
                    else "Mute Microphone",
                    "description": "Microphone is muted"
                    if current_mic["muted"]
                    else "Mute microphone input",
                    "icon": "mic_off" if current_mic["muted"] else "mic",
                    "value": {
                        "value": 1 if current_mic["muted"] else 0,
                        "min": 0,
                        "max": 1,
                        "step": 1,
                    },
                }
            )
            state["last_mic"] = current_mic

        # Send updates if any
        if updates:
            await p.send_update(updates)


if __name__ == "__main__":
    plugin.run()
