#!/usr/bin/env python3
import json
import os
import select
import signal
import sys
import time
import uuid
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path

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
        if self.state == TimerState.PAUSED:
            paused_duration = time.time() - self.paused_at
            self.started_at += paused_duration
        else:
            self.started_at = time.time() - (self.duration - self.remaining)
        self.state = TimerState.RUNNING

    def pause(self) -> None:
        if self.state == TimerState.RUNNING:
            self.tick()
            self.paused_at = time.time()
            self.state = TimerState.PAUSED

    def reset(self) -> None:
        self.remaining = self.duration
        self.state = TimerState.PAUSED
        self.started_at = 0
        self.paused_at = 0


def load_timers() -> list[Timer]:
    if not DATA_FILE.exists():
        return []
    try:
        with open(DATA_FILE) as f:
            data = json.load(f)
            return [Timer.from_dict(t) for t in data.get("timers", [])]
    except (json.JSONDecodeError, IOError, KeyError):
        return []


def save_timers(timers: list[Timer]) -> None:
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    with open(DATA_FILE, "w") as f:
        json.dump({"timers": [t.to_dict() for t in timers]}, f)


def format_time(seconds: int) -> str:
    if seconds >= 3600:
        h = seconds // 3600
        m = (seconds % 3600) // 60
        s = seconds % 60
        return f"{h}:{m:02d}:{s:02d}"
    m = seconds // 60
    s = seconds % 60
    return f"{m:02d}:{s:02d}"


def parse_duration(query: str) -> int | None:
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


def emit(data: dict) -> None:
    print(json.dumps(data), flush=True)


def get_timer_icon(timer: Timer) -> str:
    if timer.state == TimerState.COMPLETED:
        return "alarm"
    if timer.state == TimerState.PAUSED:
        return "pause_circle"
    return "timer"


def get_timer_actions(timer: Timer) -> list[dict]:
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
    results = []

    active_timers = [t for t in timers if t.state != TimerState.COMPLETED]
    completed_timers = [t for t in timers if t.state == TimerState.COMPLETED]

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
        for preset in PRESETS:
            results.append(
                {
                    "id": preset["id"],
                    "name": preset["name"],
                    "description": f"Start a {preset['name'].lower()} timer",
                    "icon": "timer",
                }
            )

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
                {"id": "stop", "icon": "stop", "name": "Stop"},
            ]
        else:
            item["actions"] = [
                {"id": "resume", "icon": "play_arrow", "name": "Resume"},
                {"id": "stop", "icon": "stop", "name": "Stop"},
            ]
        items.append(item)

    return items


def respond(
    timers: list[Timer],
    results: list[dict] | None = None,
    query: str = "",
    clear_input: bool = False,
    navigate_forward: bool | None = None,
) -> None:
    if results is None:
        results = get_timer_results(timers, query)

    status = get_status(timers)
    fab = get_fab_override(timers)
    ambient = get_ambient_items(timers)

    if fab:
        fab["showFab"] = True
        status["fab"] = fab
    else:
        status["fab"] = None

    if ambient:
        status["ambient"] = ambient
    else:
        status["ambient"] = None

    response: dict = {
        "type": "results",
        "results": results,
        "placeholder": "Search timers or enter duration (e.g., 5m, 1h30m)...",
        "status": status,
        "pluginActions": get_plugin_actions(timers),
    }

    if clear_input:
        response["clearInput"] = True
    if navigate_forward is not None:
        response["navigateForward"] = navigate_forward

    emit(response)


def emit_status(timers: list[Timer]) -> None:
    status = get_status(timers)
    fab = get_fab_override(timers)
    ambient = get_ambient_items(timers)

    if fab:
        fab["showFab"] = True
        status["fab"] = fab
    else:
        status["fab"] = None

    if ambient:
        status["ambient"] = ambient
    else:
        status["ambient"] = None

    emit({"type": "status", "status": status})


def handle_timer_completion(timer: Timer) -> None:
    emit(
        {
            "type": "execute",
            "sound": "alarm",
            "notify": f"Timer completed: {timer.name}",
        }
    )


def handle_request(
    request: dict,
    timers: list[Timer],
    current_query: str,
    plugin_active: bool,
) -> tuple[list[Timer], str, bool]:
    step = request.get("step", "initial")
    query = request.get("query", "").strip()
    selected = request.get("selected", {})
    action = request.get("action", "")
    source = request.get("source", "")

    if step == "initial":
        respond(timers)
        return timers, "", True

    if step == "search":
        respond(timers, query=query)
        return timers, query, plugin_active

    if step == "action":
        item_id = selected.get("id", "")

        if source == "ambient":
            timer_id = item_id.replace("timer:", "")
            timer = next((t for t in timers if t.id == timer_id), None)
            if timer:
                if action == "pause":
                    timer.pause()
                    save_timers(timers)
                    emit_status(timers)
                elif action == "resume":
                    timer.start()
                    save_timers(timers)
                    emit_status(timers)
                elif action == "stop" or action == "__dismiss__":
                    timers = [t for t in timers if t.id != timer_id]
                    save_timers(timers)
                    emit_status(timers)
            return timers, current_query, plugin_active

        if item_id == "__plugin__":
            if action == "pause_all":
                for t in timers:
                    if t.state == TimerState.RUNNING:
                        t.pause()
                save_timers(timers)
                respond(timers, query=current_query)
                return timers, current_query, plugin_active

            if action == "resume_all":
                for t in timers:
                    if t.state == TimerState.PAUSED:
                        t.start()
                save_timers(timers)
                respond(timers, query=current_query)
                return timers, current_query, plugin_active

            if action == "clear_completed":
                timers = [t for t in timers if t.state != TimerState.COMPLETED]
                save_timers(timers)
                respond(timers, query=current_query)
                return timers, current_query, plugin_active

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
                respond(timers, clear_input=True, navigate_forward=False)
                return timers, "", plugin_active

        if item_id.startswith("__create__:"):
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
            respond(timers, clear_input=True, navigate_forward=False)
            return timers, "", plugin_active

        if item_id.startswith("timer:"):
            timer_id = item_id.replace("timer:", "")
            timer = next((t for t in timers if t.id == timer_id), None)
            if timer:
                if action == "pause" or (
                    not action and timer.state == TimerState.RUNNING
                ):
                    timer.pause()
                    save_timers(timers)
                    respond(timers, query=current_query, navigate_forward=False)
                elif action == "resume" or (
                    not action and timer.state == TimerState.PAUSED
                ):
                    timer.start()
                    save_timers(timers)
                    respond(timers, query=current_query, navigate_forward=False)
                elif action == "restart" or (
                    not action and timer.state == TimerState.COMPLETED
                ):
                    timer.reset()
                    timer.start()
                    save_timers(timers)
                    respond(timers, query=current_query, navigate_forward=False)
                elif action == "reset":
                    timer.reset()
                    save_timers(timers)
                    respond(timers, query=current_query, navigate_forward=False)
                elif action == "delete":
                    timers = [t for t in timers if t.id != timer_id]
                    save_timers(timers)
                    respond(timers, query=current_query)
            return timers, current_query, plugin_active

        if item_id == "__empty__":
            respond(timers, query=current_query)
            return timers, current_query, plugin_active

    return timers, current_query, plugin_active


def main():
    def shutdown_handler(signum, frame):
        sys.exit(0)

    signal.signal(signal.SIGTERM, shutdown_handler)
    signal.signal(signal.SIGINT, shutdown_handler)

    timers = load_timers()
     for timer in timers:
         if timer.state == TimerState.RUNNING:
             timer.tick()
 
     emit_status(timers)

    current_query = ""
    plugin_active = False
    last_tick = time.time()
    tick_interval = 1.0

    while True:
        readable, _, _ = select.select([sys.stdin], [], [], 0.5)

        if readable:
            try:
                line = sys.stdin.readline()
                if not line:
                    break
                request = json.loads(line.strip())
                timers, current_query, plugin_active = handle_request(
                    request, timers, current_query, plugin_active
                )
            except (json.JSONDecodeError, ValueError):
                continue

        now = time.time()
        if now - last_tick >= tick_interval:
            last_tick = now

            running_timers = [t for t in timers if t.state == TimerState.RUNNING]
            if running_timers:
                completed_any = False
                for timer in running_timers:
                    if timer.tick():
                        completed_any = True
                        handle_timer_completion(timer)

                 if completed_any:
                     save_timers(timers)
 
                 if plugin_active:
                     respond(timers, query=current_query)
                 else:
                     emit_status(timers)


if __name__ == "__main__":
    main()
