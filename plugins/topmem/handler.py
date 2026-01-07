#!/usr/bin/env python3
"""
Top Memory daemon - show processes sorted by memory usage.
"""

import json
import select
import signal
import subprocess
import sys
import time


def format_bytes(bytes_val: int) -> str:
    """Format bytes to human-readable format (e.g., 1.2 GB, 256 MB)"""
    value = float(bytes_val)
    for unit in ("B", "KB", "MB", "GB", "TB"):
        if value < 1024:
            if unit == "B":
                return f"{value:.0f} {unit}"
            return f"{value:.1f} {unit}"
        value /= 1024
    return f"{value:.1f} PB"


def get_processes() -> list[dict]:
    """Get processes sorted by memory usage"""
    try:
        # Use ps to get process info sorted by memory
        result = subprocess.run(
            ["ps", "axo", "pid,user,%cpu,%mem,comm,rss", "--sort=-%mem"],
            capture_output=True,
            text=True,
            check=True,
        )

        processes = []
        for line in result.stdout.strip().split("\n")[1:51]:  # Skip header, limit to 50
            parts = line.split()
            if len(parts) >= 6:
                pid = parts[0]
                user = parts[1]
                cpu = float(parts[2])
                mem = float(parts[3])
                name = parts[4]
                rss_kb = float(parts[5])
                rss = int(rss_kb * 1024)  # Convert KB to bytes

                # Skip kernel threads and very low memory processes
                if mem < 0.1:
                    continue

                processes.append(
                    {
                        "pid": pid,
                        "name": name,
                        "cpu": cpu,
                        "mem": mem,
                        "rss": rss,
                        "user": user,
                    }
                )

        return processes[:30]  # Limit results
    except (subprocess.CalledProcessError, FileNotFoundError):
        return []


def get_process_results(processes: list[dict], query: str = "") -> list[dict]:
    """Convert processes to result format"""
    results = []

    # Filter by query if provided
    if query:
        query_lower = query.lower()
        processes = [
            p
            for p in processes
            if query_lower in p["name"].lower() or query_lower in p["pid"]
        ]

    for proc in processes:
        mem = proc["mem"]
        badges = []

        if mem > 10:
            badges.append({"icon": "warning", "color": "#f44336"})

        results.append(
            {
                "id": f"proc:{proc['pid']}",
                "name": proc["name"],
                "gauge": {
                    "value": mem,
                    "max": 100,
                    "label": f"{mem:.0f}%",
                },
                "description": f"PID {proc['pid']}  •  {format_bytes(proc['rss'])}  •  {proc['user']}",
                "badges": badges,
                "verb": "Kill",
                "actions": [
                    {"id": "kill", "name": "Kill (SIGTERM)", "icon": "cancel"},
                    {
                        "id": "kill9",
                        "name": "Force Kill (SIGKILL)",
                        "icon": "dangerous",
                    },
                ],
            }
        )

    if not results:
        results.append(
            {
                "id": "__empty__",
                "name": "No processes found" if query else "No high memory processes",
                "icon": "info",
                "description": "Try a different search" if query else "System is idle",
            }
        )

    return results


def kill_process(pid: str, force: bool = False) -> tuple[bool, str]:
    """Kill a process by PID"""
    try:
        signal = "-9" if force else "-15"
        subprocess.run(["kill", signal, pid], check=True)
        return True, f"Process {pid} {'force killed' if force else 'terminated'}"
    except subprocess.CalledProcessError:
        return False, f"Failed to kill process {pid}"


def emit(data: dict) -> None:
    print(json.dumps(data), flush=True)


def handle_request(request: dict, current_query: str) -> str:
    step = request.get("step", "initial")
    query = request.get("query", "").strip()
    selected = request.get("selected", {})
    action = request.get("action", "")

    if step == "initial":
        processes = get_processes()
        emit(
            {
                "type": "results",
                "results": get_process_results(processes),
                "placeholder": "Filter processes...",
                "inputMode": "realtime",
            }
        )
        return query

    if step == "search":
        processes = get_processes()
        emit(
            {
                "type": "results",
                "results": get_process_results(processes, query),
                "inputMode": "realtime",
            }
        )
        return query

    if step == "poll":
        processes = get_processes()
        emit(
            {
                "type": "results",
                "results": get_process_results(processes, query),
            }
        )
        return current_query

    if step == "action":
        item_id = selected.get("id", "")

        if item_id == "__empty__":
            processes = get_processes()
            emit(
                {
                    "type": "results",
                    "results": get_process_results(processes),
                }
            )
            return current_query

        if item_id.startswith("proc:"):
            pid = item_id.split(":")[1]

            if action in ("kill", ""):
                success, message = kill_process(pid, force=False)
            elif action == "kill9":
                success, message = kill_process(pid, force=True)
            else:
                success, message = False, "Unknown action"

            processes = get_processes()
            emit(
                {
                    "type": "results",
                    "results": get_process_results(processes),
                    "notify": message if success else None,
                }
            )
            return current_query

    emit({"type": "error", "message": f"Unknown step: {step}"})
    return current_query


def main():
    def shutdown_handler(signum, frame):
        sys.exit(0)

    signal.signal(signal.SIGTERM, shutdown_handler)
    signal.signal(signal.SIGINT, shutdown_handler)

    current_query = ""
    last_refresh = time.time()
    refresh_interval = 2.0

    while True:
        readable, _, _ = select.select([sys.stdin], [], [], 0.5)

        if readable:
            try:
                line = sys.stdin.readline()
                if not line:
                    break
                request = json.loads(line.strip())
                current_query = handle_request(request, current_query)
            except (json.JSONDecodeError, ValueError):
                continue

        now = time.time()
        if now - last_refresh >= refresh_interval:
            processes = get_processes()
            emit(
                {
                    "type": "results",
                    "results": get_process_results(processes, current_query),
                }
            )
            last_refresh = now


if __name__ == "__main__":
    main()
