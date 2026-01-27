#!/usr/bin/env python3
"""
Timer plugin for hamr - Socket-based version

Provides countdown timers with:
- Preset durations (1m, 5m, 10m, etc.)
- Custom duration parsing (e.g., "5m30s", "1h")
- Background tick updates
- Ambient items showing active timers
- Status badges/chips for running/paused timers
"""

import asyncio
import json
import sys
import time
import uuid
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Optional

# Add parent directory to path to import SDK
sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

DATA_DIR = Path.home() / ".config" / "hamr" / "data" / "timer"
DATA_FILE = DATA_DIR / "data.json"

PRESETS = [
    {"id": "preset:1m", "name": "1 minute", "duration": 60},
    {"id": "preset:5m", "name": "5 minutes", "duration": 300},
    {"id": "preset:10m", "name": "10 minutes", "duration": 600},
    {"id": "preset:15m", "name": "15 minutes", "duration": 900},
    {"id": "preset:25m", "name": "25 minutes (Pomodoro)", "duration": 1500},
    {"id": "preset:30m", "name": "30 minutes", "duration": 1800},
    {"id": "preset:45m", "name": "45 minutes", "duration": 2700},
    {"id": "preset:60m", "name": "1 hour", "duration": 3600},
]


class TimerState(Enum):
    RUNNING = "running"
    PAUSED = "paused"
    COMPLETED = "completed"


@dataclass
class Timer:
    id: str
    name: str
    duration: int
    remaining: int
    state: TimerState
    started_at: float = 0
    paused_at: float = 0
    created_at: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return {
            "id": self.id,
            "name": self.name,
            "duration": self.duration,
            "remaining": self.remaining,
            "state": self.state.value,
            "started_at": self.started_at,
            "paused_at": self.paused_at,
            "created_at": self.created_at,
        }

    @classmethod
    def from_dict(cls, data: dict) -> "Timer":
        return cls(
            id=data["id"],
            name=data["name"],
            duration=data["duration"],
            remaining=data["remaining"],
            state=TimerState(data["state"]),
            started_at=data.get("started_at", 0),
            paused_at=data.get("paused_at", 0),
            created_at=data.get("created_at", time.time()),
        )

    def tick(self) -> bool:
        """Tick the timer. Returns True if timer completed."""
        if self.state != TimerState.RUNNING:
            return False

        now = time.time()
        elapsed = now - self.started_at
        self.remaining = max(0, self.duration - int(elapsed))

        if self.remaining <= 0:
            self.state = TimerState.COMPLETED
            return True
        return False

    def start(self) -> None:
        """Start or resume the timer."""
        if self.state == TimerState.PAUSED:
            paused_duration = time.time() - self.paused_at
            self.started_at += paused_duration
        else:
            self.started_at = time.time() - (self.duration - self.remaining)
        self.state = TimerState.RUNNING

    def pause(self) -> None:
        """Pause the timer."""
        if self.state == TimerState.RUNNING:
            self.tick()
            self.paused_at = time.time()
            self.state = TimerState.PAUSED

    def reset(self) -> None:
        """Reset the timer to initial state."""
        self.remaining = self.duration
        self.state = TimerState.PAUSED
        self.started_at = 0
        self.paused_at = 0


def load_timers() -> list[Timer]:
    """Load timers from persistent storage."""
    if not DATA_FILE.exists():
        return []
    try:
        with open(DATA_FILE) as f:
            data = json.load(f)
            return [Timer.from_dict(t) for t in data.get("timers", [])]
    except (json.JSONDecodeError, IOError, KeyError):
        return []


def save_timers(timers: list[Timer]) -> None:
    """Save timers to persistent storage."""
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    with open(DATA_FILE, "w") as f:
        json.dump({"timers": [t.to_dict() for t in timers]}, f)


def format_time(seconds: int) -> str:
    """Format seconds as HH:MM:SS or MM:SS."""
    if seconds >= 3600:
        h = seconds // 3600
        m = (seconds % 3600) // 60
        s = seconds % 60
        return f"{h}:{m:02d}:{s:02d}"
    m = seconds // 60
    s = seconds % 60
    return f"{m:02d}:{s:02d}"


def parse_duration(query: str) -> int | None:
    """Parse duration string like '5m', '1h30m', '90s'."""
    query = query.strip().lower()
    if not query:
        return None

    total = 0
    current_num = ""

    for char in query:
        if char.isdigit():
            current_num += char
        elif char in "hms" and current_num:
            num = int(current_num)
            if char == "h":
                total += num * 3600
            elif char == "m":
                total += num * 60
            elif char == "s":
                total += num
            current_num = ""
        elif char == " ":
            continue
        else:
            return None

    if current_num:
        num = int(current_num)
        if total == 0:
            total = num * 60
        else:
            total += num

    return total if total > 0 else None


def get_timer_icon(timer: Timer) -> str:
    """Get icon for timer based on state."""
    if timer.state == TimerState.COMPLETED:
        return "alarm"
    if timer.state == TimerState.PAUSED:
        return "pause_circle"
    return "timer"


def get_timer_actions(timer: Timer) -> list[dict]:
    """Get available actions for a timer."""
    actions = []
    if timer.state == TimerState.RUNNING:
        actions.append({"id": "pause", "name": "Pause", "icon": "pause"})
    elif timer.state == TimerState.PAUSED:
        actions.append({"id": "resume", "name": "Resume", "icon": "play_arrow"})
    elif timer.state == TimerState.COMPLETED:
        actions.append({"id": "restart", "name": "Restart", "icon": "replay"})

    if timer.state != TimerState.COMPLETED:
        actions.append({"id": "reset", "name": "Reset", "icon": "refresh"})

    actions.append({"id": "delete", "name": "Delete", "icon": "delete"})
    return actions


def get_timer_results(timers: list[Timer], query: str = "") -> list[dict]:
    """Get search results based on current timers and query."""
    results = []

    active_timers = [t for t in timers if t.state != TimerState.COMPLETED]
    completed_timers = [t for t in timers if t.state == TimerState.COMPLETED]

    # Active timers
    for timer in active_timers:
        timer.tick()
        state_desc = "Running" if timer.state == TimerState.RUNNING else "Paused"
        results.append(
            {
                "id": f"timer:{timer.id}",
                "name": timer.name,
                "description": f"{format_time(timer.remaining)} - {state_desc}",
                "icon": get_timer_icon(timer),
                "verb": "Pause" if timer.state == TimerState.RUNNING else "Resume",
                "actions": get_timer_actions(timer),
            }
        )

    # Completed timers
    for timer in completed_timers:
        results.append(
            {
                "id": f"timer:{timer.id}",
                "name": timer.name,
                "description": "Completed",
                "icon": "alarm",
                "verb": "Restart",
                "actions": get_timer_actions(timer),
            }
        )

    # Handle search query
    if query:
        query_lower = query.lower()
        results = [r for r in results if query_lower in r["name"].lower()]

        duration = parse_duration(query)
        if duration:
            results.insert(
                0,
                {
                    "id": f"__create__:{duration}",
                    "name": f"Start {format_time(duration)} timer",
                    "description": f"New timer for {format_time(duration)}",
                    "icon": "add_circle",
                },
            )
    else:
        # Show presets when no query
        for preset in PRESETS:
            results.append(
                {
                    "id": preset["id"],
                    "name": preset["name"],
                    "description": f"Start a {preset['name'].lower()} timer",
                    "icon": "timer",
                }
            )

    # Empty state
    if not results:
        results.append(
            {
                "id": "__empty__",
                "name": "No timers",
                "description": "Type a duration (e.g., '5m', '1h30m') or select a preset",
                "icon": "info",
            }
        )

    return results


def get_plugin_actions(timers: list[Timer]) -> list[dict]:
    """Get available plugin-level actions."""
    actions = []
    running = [t for t in timers if t.state == TimerState.RUNNING]
    paused = [t for t in timers if t.state == TimerState.PAUSED]
    completed = [t for t in timers if t.state == TimerState.COMPLETED]

    if running:
        actions.append(
            {
                "id": "pause_all",
                "name": f"Pause All ({len(running)})",
                "icon": "pause",
                "shortcut": "Ctrl+1",
            }
        )
    if paused:
        actions.append(
            {
                "id": "resume_all",
                "name": f"Resume All ({len(paused)})",
                "icon": "play_arrow",
                "shortcut": "Ctrl+2" if not running else "Ctrl+1",
            }
        )
    if completed:
        actions.append(
            {
                "id": "clear_completed",
                "name": f"Clear Done ({len(completed)})",
                "icon": "delete_sweep",
                "confirm": f"Remove {len(completed)} completed timer(s)?",
            }
        )
    return actions


def get_status(timers: list[Timer]) -> dict:
    """Build status object with chips and badges."""
    running = [t for t in timers if t.state == TimerState.RUNNING]
    paused = [t for t in timers if t.state == TimerState.PAUSED]
    completed = [t for t in timers if t.state == TimerState.COMPLETED]

    status: dict = {}

    if running or paused:
        chips = []
        if running:
            chips.append({"text": f"{len(running)} running", "icon": "timer"})
        if paused:
            chips.append({"text": f"{len(paused)} paused", "icon": "pause"})
        status["chips"] = chips

    if completed:
        status.setdefault("badges", []).append(
            {"text": str(len(completed)), "color": "#4caf50"}
        )

    return status


def get_fab_override(timers: list[Timer]) -> dict | None:
    """Get FAB (floating action button) override showing quickest timer."""
    running = [t for t in timers if t.state == TimerState.RUNNING]
    if not running:
        return None

    timer = min(running, key=lambda t: t.remaining)
    timer.tick()

    return {
        "chips": [{"text": format_time(timer.remaining), "icon": "timer"}],
        "priority": 10,
    }


def get_ambient_items(timers: list[Timer]) -> list[dict] | None:
    """Get ambient items showing active timers."""
    active = [t for t in timers if t.state in (TimerState.RUNNING, TimerState.PAUSED)]
    if not active:
        return None

    items = []
    for timer in sorted(active, key=lambda t: t.remaining):
        timer.tick()
        state_icon = "pause" if timer.state == TimerState.PAUSED else None
        item: dict = {
            "id": f"timer:{timer.id}",
            "name": timer.name,
            "description": format_time(timer.remaining),
            "icon": "timer",
        }
        if state_icon:
            item["badges"] = [{"icon": state_icon}]

        if timer.state == TimerState.RUNNING:
            item["actions"] = [
                {"id": "pause", "icon": "pause", "name": "Pause"},
                {"id": "delete", "icon": "delete", "name": "Delete"},
            ]
        else:
            item["actions"] = [
                {"id": "resume", "icon": "play_arrow", "name": "Resume"},
                {"id": "delete", "icon": "delete", "name": "Delete"},
            ]
        items.append(item)

    return items


# Create plugin instance
plugin = HamrPlugin(
    id="timer",
    name="Timer",
    description="Countdown timers and stopwatch",
    icon="timer",
)

# Plugin state
state = {
    "timers": [],
    "current_query": "",
    "plugin_active": False,
}


@plugin.on_initial
async def handle_initial(params=None):
    """Handle initial request when plugin is opened."""
    state["timers"] = load_timers()
    state["plugin_active"] = True
    return build_response()


@plugin.on_search
async def handle_search(query: str, context: Optional[str]):
    """Handle search request."""
    state["current_query"] = query
    return build_response(query)


async def handle_ambient_action(
    item_id: str, action: Optional[str], timers: list[Timer]
) -> dict:
    """
    Handle ambient actions (from ambient bar).

    For ambient actions, we only emit status updates - we don't return results
    which would open the plugin view.
    """
    if not item_id.startswith("timer:"):
        return HamrPlugin.noop()

    timer_id = item_id.replace("timer:", "")
    timer = next((t for t in timers if t.id == timer_id), None)

    if not timer:
        return HamrPlugin.noop()

    if action == "pause":
        timer.pause()
        save_timers(timers)
    elif action == "resume":
        timer.start()
        save_timers(timers)
    elif action == "delete" or action == "__dismiss__":
        timers = [t for t in timers if t.id != timer_id]
        state["timers"] = timers
        save_timers(timers)

    # Send status update (explicitly set fab/ambient to null to clear)
    status = get_status(timers)
    fab = get_fab_override(timers)
    if fab:
        fab["showFab"] = True
    status["fab"] = fab  # Always set, even if None (to clear)
    ambient = get_ambient_items(timers)
    status["ambient"] = ambient  # Always set, even if None (to clear)

    await plugin.send_status(status)

    # Return empty dict - don't return results which would open the plugin view
    return HamrPlugin.noop()


@plugin.on_action
async def handle_action(
    item_id: str, action: Optional[str], context: Optional[str], source: Optional[str]
):
    """Handle action request."""
    timers = state["timers"]

    # Handle ambient actions (from ambient bar) - only emit status, no results
    if source == "ambient":
        return await handle_ambient_action(item_id, action, timers)

    # Handle preset selection
    if item_id.startswith("preset:"):
        preset = next((p for p in PRESETS if p["id"] == item_id), None)
        if preset:
            timer = Timer(
                id=str(uuid.uuid4())[:8],
                name=preset["name"],
                duration=preset["duration"],
                remaining=preset["duration"],
                state=TimerState.RUNNING,
            )
            timer.start()
            timers.append(timer)
            save_timers(timers)
            state["current_query"] = ""
            return {"clearInput": True, **build_response()}

    # Handle custom duration creation
    if item_id.startswith("__create__:"):
        try:
            duration = int(item_id.split(":")[1])
            timer = Timer(
                id=str(uuid.uuid4())[:8],
                name=format_time(duration),
                duration=duration,
                remaining=duration,
                state=TimerState.RUNNING,
            )
            timer.start()
            timers.append(timer)
            save_timers(timers)
            state["current_query"] = ""
            return {"clearInput": True, **build_response()}
        except (ValueError, IndexError):
            return build_response(state["current_query"])

    # Handle timer actions
    if item_id.startswith("timer:"):
        timer_id = item_id.replace("timer:", "")
        timer = next((t for t in timers if t.id == timer_id), None)
        if timer:
            if action == "pause" or (not action and timer.state == TimerState.RUNNING):
                timer.pause()
            elif action == "resume" or (
                not action and timer.state == TimerState.PAUSED
            ):
                timer.start()
            elif action == "restart" or (
                not action and timer.state == TimerState.COMPLETED
            ):
                timer.reset()
                timer.start()
            elif action == "reset":
                timer.reset()
            elif action == "delete":
                timers = [t for t in timers if t.id != timer_id]
                state["timers"] = timers

            save_timers(timers)
            return build_response(state["current_query"])

    # Handle plugin-level actions
    if item_id == "__plugin__":
        if action == "pause_all":
            for t in timers:
                if t.state == TimerState.RUNNING:
                    t.pause()
            save_timers(timers)
            return build_response(state["current_query"])

        if action == "resume_all":
            for t in timers:
                if t.state == TimerState.PAUSED:
                    t.start()
            save_timers(timers)
            return build_response(state["current_query"])

        if action == "clear_completed":
            timers = [t for t in timers if t.state != TimerState.COMPLETED]
            state["timers"] = timers
            save_timers(timers)
            return build_response(state["current_query"])

    return build_response(state["current_query"])


def build_response(query: str = "") -> dict:
    """Build the results response with status and actions."""
    timers = state["timers"]
    results = get_timer_results(timers, query)
    status = get_status(timers)

    # Timer uses ambient items for FAB display (with actions), not FAB override
    ambient = get_ambient_items(timers)
    status["ambient"] = ambient  # Always set, even if None (to clear)

    return HamrPlugin.results(
        results,
        status=status,
        plugin_actions=get_plugin_actions(timers),
        placeholder="Search timers or enter duration (e.g., 5m, 1h30m)...",
    )


@plugin.add_background_task
async def timer_tick(p: HamrPlugin):
    """Background task that ticks timers every second and sends updates."""
    while True:
        await asyncio.sleep(1)

        timers = state["timers"]
        running = [t for t in timers if t.state == TimerState.RUNNING]

        if not running:
            continue

        completed_any = False
        for timer in running:
            if timer.tick():
                completed_any = True
                # Play alarm sound
                await p.send_execute({"type": "sound", "sound": "alarm"})
                # Show notification
                await p.send_execute(
                    {"type": "notify", "message": f"Timer completed: {timer.name}"}
                )

        if completed_any:
            save_timers(timers)

        # Send status update - timer uses ambient items for FAB display
        status = get_status(timers)
        ambient = get_ambient_items(timers)
        status["ambient"] = ambient  # Always set, even if None (to clear)

        await p.send_status(status)

        # If plugin is active, also send results update
        if state["plugin_active"]:
            await p.send_results(
                get_timer_results(timers, state["current_query"]),
                status=status,
                pluginActions=get_plugin_actions(timers),
                placeholder="Search timers or enter duration (e.g., 5m, 1h30m)...",
            )


if __name__ == "__main__":
    plugin.run()
