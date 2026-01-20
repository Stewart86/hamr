#!/usr/bin/env python3
"""
Top Memory plugin for hamr - Show processes sorted by memory usage.

Uses socket SDK for daemon mode with real-time auto-refresh.
"""

import asyncio
import subprocess
import sys
from pathlib import Path
from typing import Optional

# Add parent directory to path to import SDK
sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin


def format_bytes(bytes_val: int) -> str:
    """Format bytes to human-readable format (e.g., 1.2 GB, 256 MB)."""
    value = float(bytes_val)
    for unit in ("B", "KB", "MB", "GB", "TB"):
        if value < 1024:
            if unit == "B":
                return f"{value:.0f} {unit}"
            return f"{value:.1f} {unit}"
        value /= 1024
    return f"{value:.1f} PB"


def get_processes() -> list[dict]:
    """Get processes sorted by memory usage."""
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
                try:
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
                except ValueError:
                    continue

        return processes[:30]  # Limit results
    except (subprocess.CalledProcessError, FileNotFoundError):
        return []


def get_process_results(processes: list[dict], query: str = "") -> list[dict]:
    """Convert processes to result format."""
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
    """Kill a process by PID."""
    try:
        signal = "-9" if force else "-15"
        subprocess.run(["kill", signal, pid], check=True)
        return True, f"Process {pid} {'force killed' if force else 'terminated'}"
    except subprocess.CalledProcessError:
        return False, f"Failed to kill process {pid}"


# Create plugin instance
plugin = HamrPlugin(
    id="topmem",
    name="Top Memory",
    description="Processes sorted by memory usage",
    icon="memory",
)

# Plugin state
state = {
    "current_query": "",
}


@plugin.on_initial
async def handle_initial(params=None):
    """Handle initial request."""
    state["current_query"] = ""
    processes = get_processes()
    return HamrPlugin.results(
        get_process_results(processes),
        placeholder="Filter processes...",
    )


@plugin.on_search
async def handle_search(query: str, context: Optional[str]):
    """Handle search request."""
    state["current_query"] = query
    processes = get_processes()
    return HamrPlugin.results(
        get_process_results(processes, query),
    )


@plugin.on_action
async def handle_action(item_id: str, action: Optional[str], context: Optional[str]):
    """Handle action request."""
    if item_id == "__empty__":
        processes = get_processes()
        return HamrPlugin.results(
            get_process_results(processes),
        )

    if item_id.startswith("proc:"):
        pid = item_id.split(":")[1]

        if action in ("kill", ""):
            success, message = kill_process(pid, force=False)
        elif action == "kill9":
            success, message = kill_process(pid, force=True)
        else:
            success, message = False, "Unknown action"

        processes = get_processes()
        response = HamrPlugin.results(
            get_process_results(processes, state["current_query"]),
        )
        if success:
            response["status"] = {"notify": message}
        return response

    return {}


@plugin.add_background_task
async def refresh_task(p: HamrPlugin):
    """Background task that refreshes process list every 2 seconds."""
    while True:
        await asyncio.sleep(2)
        processes = get_processes()
        await p.send_results(
            get_process_results(processes, state["current_query"]),
        )


if __name__ == "__main__":
    plugin.run()
