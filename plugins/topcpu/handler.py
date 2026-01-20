#!/usr/bin/env python3
"""
Top CPU plugin for hamr - Show processes sorted by CPU usage.

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


def get_processes() -> list[dict]:
    """Get processes sorted by CPU usage (real-time, like top/btop)."""
    try:
        # Use top in batch mode for real-time CPU usage
        # -b: batch mode, -n2: two iterations (second has accurate CPU delta)
        # -d0.5: 0.5s delay between iterations, -w512: wide output
        result = subprocess.run(
            ["top", "-b", "-n2", "-d0.5", "-w512"],
            capture_output=True,
            text=True,
            check=True,
        )

        processes = []
        lines = result.stdout.strip().split("\n")

        header_idx = -1
        header_count = 0
        for i, line in enumerate(lines):
            if "PID" in line and "USER" in line and "%CPU" in line:
                header_count += 1
                if header_count == 2:
                    header_idx = i
                    break

        # Fall back to first header if only one iteration
        if header_idx == -1:
            for i, line in enumerate(lines):
                if "PID" in line and "USER" in line and "%CPU" in line:
                    header_idx = i
                    break

        if header_idx == -1:
            return []

        # Parse process lines after header - collect all first, then filter
        # Parse many lines because kernel threads (root, 0% CPU) come before user processes
        all_procs = []
        for line in lines[header_idx + 1 :]:
            parts = line.split()
            if len(parts) >= 12:
                try:
                    pid = parts[0]
                    user = parts[1]
                    # top format: PID USER PR NI VIRT RES SHR S %CPU %MEM TIME+ COMMAND...
                    cpu = float(parts[8])
                    mem = float(parts[9])
                    # COMMAND is everything after TIME+ (index 11+), join in case of spaces
                    name = " ".join(parts[11:])

                    all_procs.append(
                        {
                            "pid": pid,
                            "name": name,
                            "cpu": cpu,
                            "mem": mem,
                            "user": user,
                        }
                    )
                except (ValueError, IndexError):
                    continue

        # Filter strategy:
        # 1. Always show processes with CPU > 0
        # 2. For ~0% CPU, only show user (non-root) processes
        # This avoids cluttering with kernel threads while showing real apps

        active_procs = [p for p in all_procs if p["cpu"] >= 0.1]
        idle_user_procs = [
            p for p in all_procs if p["cpu"] < 0.1 and p["user"] != "root"
        ]

        processes = active_procs + idle_user_procs
        return processes[:30]
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
        cpu = proc["cpu"]
        badges = []

        if cpu > 50:
            badges.append({"icon": "warning", "color": "#f44336"})

        results.append(
            {
                "id": f"proc:{proc['pid']}",
                "name": proc["name"],
                "gauge": {
                    "value": cpu,
                    "max": 100,
                    "label": f"{cpu:.0f}%",
                },
                "description": f"PID {proc['pid']}  •  Mem: {proc['mem']:.1f}%  •  {proc['user']}",
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
                "name": "No processes found" if query else "No high CPU processes",
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
    id="topcpu",
    name="Top CPU",
    description="Processes sorted by CPU usage",
    icon="speed",
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
