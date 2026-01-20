#!/usr/bin/env python3
"""Widget Showcase - Demo plugin to showcase all Hamr UI widgets.

This plugin demonstrates:
- Graphs (sparklines)
- Gauges (circular progress)
- Progress bars
- Sliders
- Switches
- Badges (with text, icons, and colors)
- Chips (with text, icons, and colors)
- Actions (action buttons)
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

plugin = HamrPlugin(
    id="test-widgets",
    name="Widget Showcase",
    description="Demo all Hamr UI widgets",
    icon="widgets",
)


def get_widget_showcase() -> list[dict]:
    """Return all widget showcase items."""
    return [
        # Graph example - CPU sparkline
        {
            "id": "cpu-monitor",
            "name": "CPU Usage",
            "description": "10-point sparkline graph",
            "icon": "memory",
            "graph": {"data": [45, 52, 48, 61, 55, 70, 65, 72, 68, 75]},
            "badges": [{"text": "75%", "color": "#ff9800"}],
        },
        # Gauge example - Memory usage
        {
            "id": "memory-usage",
            "name": "Memory",
            "description": "Circular gauge indicator",
            "icon": "pie_chart",
            "gauge": {"value": 62, "max": 100, "label": "62%"},
            "chips": [{"text": "12.4 GB / 20 GB"}],
        },
        # Progress bar example - Disk space
        {
            "id": "disk-space",
            "name": "Disk Space",
            "description": "Linear progress bar",
            "icon": "storage",
            "progress": {"value": 456, "max": 512, "label": "456 GB / 512 GB"},
        },
        # Slider example - Volume control
        {
            "id": "volume-control",
            "name": "Volume",
            "description": "Adjustable slider control",
            "icon": "volume_up",
            "type": "slider",
            "value": 75.0,
            "min": 0.0,
            "max": 100.0,
            "step": 5.0,
            "displayValue": "75%",
        },
        # Slider example - Brightness
        {
            "id": "brightness",
            "name": "Brightness",
            "description": "Screen brightness control",
            "icon": "brightness_6",
            "type": "slider",
            "value": 80.0,
            "min": 0.0,
            "max": 100.0,
            "step": 10.0,
            "displayValue": "80%",
        },
        # Switch example - WiFi ON
        {
            "id": "wifi-toggle",
            "name": "Turn Off WiFi",
            "description": "Connected to HomeNetwork",
            "icon": "wifi",
            "type": "switch",
            "value": True,  # ON
        },
        # Switch example - Bluetooth OFF
        {
            "id": "bluetooth-toggle",
            "name": "Enable Bluetooth",
            "description": "Bluetooth is off",
            "icon": "bluetooth_disabled",
            "type": "switch",
            "value": False,  # OFF
        },
        # Badges and chips with actions - PR review
        {
            "id": "pr-review",
            "name": "Review: Add user authentication",
            "description": "stewart86/hamr #142",
            "icon": "code",
            "badges": [
                {"text": "JD"},
                {"text": "SK"},
            ],
            "chips": [
                {
                    "text": "Ready for Review",
                    "icon": "check_circle",
                    "color": "#4caf50",  # Only text/icon color, no background
                },
            ],
            "verb": "Open",
            "actions": [
                {"id": "approve", "name": "Approve", "icon": "thumb_up"},
                {"id": "comment", "name": "Comment", "icon": "comment"},
                {"id": "request-changes", "name": "Request Changes", "icon": "edit"},
            ],
        },
        # Badge with icon color - Urgent task
        {
            "id": "task-urgent",
            "name": "Fix login bug on Safari",
            "description": "Due today",
            "icon": "bug_report",
            "badges": [
                {"icon": "priority_high", "color": "#f44336"},
            ],
            "chips": [
                {"text": "Bug", "color": "#f44336"},
                {"text": "Frontend"},
            ],
            "verb": "Mark Done",
            "actions": [
                {"id": "edit", "name": "Edit", "icon": "edit"},
                {"id": "snooze", "name": "Snooze", "icon": "snooze"},
            ],
        },
        # Progress bar - Download
        {
            "id": "download-progress",
            "name": "ubuntu-24.04-desktop-amd64.iso",
            "description": "Downloading...",
            "icon": "downloading",
            "progress": {"value": 2.8, "max": 4.7, "label": "2.8 GB / 4.7 GB"},
            "verb": "Cancel",
        },
        # Progress bar - Sync status
        {
            "id": "sync-status",
            "name": "Syncing photos...",
            "description": "Cloud sync in progress",
            "icon": "cloud_sync",
            "progress": {"value": 847, "max": 1200, "label": "847 / 1200 files"},
        },
        # Gauge with chips - Battery
        {
            "id": "battery-status",
            "name": "Battery",
            "description": "Current charge level",
            "icon": "battery_5_bar",
            "gauge": {"value": 45, "max": 100, "label": "45%"},
            "chips": [{"text": "2h 30m remaining", "icon": "schedule"}],
        },
        # Gauge with custom color - Temperature
        {
            "id": "cpu-temp",
            "name": "CPU Temperature",
            "description": "Thermal monitor",
            "gauge": {
                "value": 72,
                "min": 20,
                "max": 100,
                "label": "72C",
                "color": "#ff5722",
            },
            "chips": [
                {"text": "Warning", "color": "#ff5722"},
                {"text": "Throttling soon"},
            ],
        },
        # Graph with multiple chips - Network traffic
        {
            "id": "network-traffic",
            "name": "Network Traffic",
            "description": "Real-time bandwidth",
            "icon": "router",
            "graph": {
                "data": [12, 45, 23, 67, 34, 89, 56, 78, 43, 91],
                "min": 0,
                "max": 100,
            },
            "chips": [
                {"text": "91 Mbps", "icon": "arrow_downward"},
                {"text": "23 Mbps", "icon": "arrow_upward"},
            ],
        },
        # Graph with many data points - Stock price (volatile)
        {
            "id": "stock-chart",
            "name": "AAPL Stock Price",
            "description": "30-day trend",
            "graph": {
                "data": [
                    182,
                    175,
                    188,
                    171,
                    195,
                    168,
                    185,
                    192,
                    178,
                    165,
                    198,
                    172,
                    205,
                    180,
                    169,
                    210,
                    175,
                    190,
                    162,
                    202,
                    185,
                    170,
                    215,
                    178,
                    195,
                    160,
                    208,
                    172,
                    188,
                    200,
                ],
            },
            "chips": [
                {"text": "+9.8%", "color": "#4caf50"},
                {"text": "30 days"},
            ],
        },
        # Badge with icon color - Meeting reminder
        {
            "id": "meeting-reminder",
            "name": "Team Standup",
            "description": "in 15 minutes",
            "icon": "event",
            "badges": [
                {"icon": "videocam", "color": "#2196f3"},
            ],
            "chips": [{"text": "10:00 AM"}],
            "verb": "Join",
            "actions": [
                {"id": "snooze", "name": "Snooze", "icon": "snooze"},
                {"id": "dismiss", "name": "Dismiss", "icon": "close"},
            ],
        },
        # Progress with actions - Music player
        {
            "id": "music-player",
            "name": "Bohemian Rhapsody",
            "description": "Queen - A Night at the Opera",
            "icon": "music_note",
            "progress": {"value": 180, "max": 354, "label": "3:00 / 5:54"},
            "verb": "Pause",
            "actions": [
                {"id": "prev", "name": "Previous", "icon": "skip_previous"},
                {"id": "next", "name": "Next", "icon": "skip_next"},
                {"id": "shuffle", "name": "Shuffle", "icon": "shuffle"},
            ],
        },
        # Badge and chips - Server status
        {
            "id": "server-status",
            "name": "Production Server",
            "description": "us-east-1.aws.example.com",
            "icon": "dns",
            "badges": [
                {"icon": "check_circle", "color": "#4caf50"},
            ],
            "chips": [
                {"text": "Healthy", "color": "#4caf50"},
                {"text": "99.9% uptime"},
            ],
        },
        # Build status with colored chips
        {
            "id": "build-status",
            "name": "Build #1847",
            "description": "main branch",
            "icon": "construction",
            "badges": [
                {"icon": "check", "color": "#4caf50"},
            ],
            "chips": [
                {"text": "Passed", "color": "#4caf50"},
                {"text": "2m 34s"},
            ],
            "verb": "View Logs",
        },
        # Slider with display value - Playback speed
        {
            "id": "playback-speed",
            "name": "Playback Speed",
            "description": "Video/audio playback rate",
            "icon": "speed",
            "type": "slider",
            "value": 1.5,
            "min": 0.5,
            "max": 2.0,
            "step": 0.25,
            "displayValue": "1.5x",
        },
        # Multiple badges example
        {
            "id": "multi-badges",
            "name": "Multiple Badges Example",
            "description": "Shows text and icon badges",
            "icon": "stars",
            "badges": [
                {"text": "NEW", "color": "#2196f3"},
                {"text": "HOT", "color": "#f44336"},
                {"icon": "favorite", "color": "#e91e63"},
                {"text": "99"},
            ],
        },
        # Multiple chips example
        {
            "id": "multi-chips",
            "name": "Multiple Chips Example",
            "description": "Various chip styles",
            "icon": "label",
            "chips": [
                {"text": "Default"},
                {"text": "With Icon", "icon": "star"},
                {"text": "Green", "color": "#4caf50"},
                {"text": "Red", "color": "#f44336"},
                {"text": "Blue", "color": "#2196f3"},
            ],
        },
        # Failed build status
        {
            "id": "build-failed",
            "name": "Build #1846",
            "description": "feature/dark-mode branch",
            "icon": "error",
            "badges": [
                {"icon": "close", "color": "#f44336"},
            ],
            "chips": [
                {"text": "Failed", "color": "#f44336"},
                {"text": "5m 12s"},
            ],
            "verb": "View Logs",
        },
        # Warning status
        {
            "id": "warning-status",
            "name": "Disk Space Warning",
            "description": "Storage almost full",
            "icon": "warning",
            "badges": [
                {"icon": "warning", "color": "#ff9800"},
            ],
            "chips": [
                {"text": "Warning", "color": "#ff9800"},
            ],
            "progress": {"value": 95, "max": 100, "label": "95% used"},
        },
    ]


@plugin.on_initial
def handle_initial(params=None):
    """Handle initial request - show all widget examples."""
    return HamrPlugin.results(
        get_widget_showcase(),
        input_mode="realtime",
        placeholder="Browse widget examples...",
        plugin_actions=[
            {"id": "refresh", "name": "Refresh", "icon": "refresh"},
        ],
    )


@plugin.on_search
def handle_search(query: str, context: str | None):
    """Handle search - filter widget examples."""
    results = get_widget_showcase()

    if query:
        query_lower = query.lower()
        results = [
            r
            for r in results
            if query_lower in r.get("name", "").lower()
            or query_lower in r.get("description", "").lower()
            or query_lower in r.get("id", "").lower()
        ]

    return HamrPlugin.results(
        results,
        input_mode="realtime",
        placeholder="Filter widget examples...",
    )


@plugin.on_action
def handle_action(item_id: str, action: str | None, context: str | None):
    """Handle action - just show a message for demo purposes."""
    if action:
        return HamrPlugin.card(
            f"Action: {action}",
            content=f"You triggered action '{action}' on item '{item_id}'.\n\nThis is a demo - no actual action performed.",
        )
    return HamrPlugin.card(
        f"Selected: {item_id}",
        content=f"You selected item '{item_id}'.\n\nThis is a widget showcase demo.",
    )


@plugin.on_slider_changed
def handle_slider_changed(slider_id: str, value: float):
    """Handle slider changes - just log for demo."""
    # In a real plugin, you would update some setting here
    return {"type": "noop"}


@plugin.on_switch_toggled
def handle_switch_toggled(switch_id: str, value: bool):
    """Handle switch toggles - just log for demo."""
    # In a real plugin, you would toggle some setting here
    return {"type": "noop"}


if __name__ == "__main__":
    plugin.run()
