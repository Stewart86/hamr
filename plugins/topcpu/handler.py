#!/usr/bin/env python3
"""
Top CPU workflow handler - show processes sorted by CPU usage.
"""

import json
import os
import subprocess
import sys

TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

MOCK_PROCESSES = [
    {"pid": "1234", "name": "firefox", "cpu": 25.5, "mem": 8.2, "user": "user"},
    {"pid": "5678", "name": "code", "cpu": 15.3, "mem": 12.1, "user": "user"},
    {"pid": "9012", "name": "python3", "cpu": 8.7, "mem": 2.5, "user": "user"},
]


def get_processes() -> list[dict]:
    """Get processes sorted by CPU usage (real-time, like top/btop)"""
    if TEST_MODE:
        return MOCK_PROCESSES

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
        results.append(
            {
                "id": f"proc:{proc['pid']}",
                "name": f"{proc['name']} ({proc['pid']})",
                "icon": "memory",
                "description": f"CPU: {proc['cpu']:.1f}%  |  Mem: {proc['mem']:.1f}%  |  User: {proc['user']}",
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
    """Kill a process by PID"""
    if TEST_MODE:
        return True, f"Process {pid} killed"

    try:
        signal = "-9" if force else "-15"
        subprocess.run(["kill", signal, pid], check=True)
        return True, f"Process {pid} {'force killed' if force else 'terminated'}"
    except subprocess.CalledProcessError:
        return False, f"Failed to kill process {pid}"


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    if step == "initial":
        processes = get_processes()
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": get_process_results(processes),
                    "placeholder": "Filter processes...",
                    "inputMode": "realtime",
                }
            )
        )
        return

    if step == "search":
        processes = get_processes()
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": get_process_results(processes, query),
                    "inputMode": "realtime",
                }
            )
        )
        return

    # Poll: refresh with current query (called periodically by PluginRunner)
    if step == "poll":
        processes = get_processes()
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": get_process_results(processes, query),
                }
            )
        )
        return

    # Action: handle clicks
    if step == "action":
        item_id = selected.get("id", "")

        if item_id == "__empty__":
            processes = get_processes()
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": get_process_results(processes),
                    }
                )
            )
            return

        if item_id.startswith("proc:"):
            pid = item_id.split(":")[1]

            if action in ("kill", ""):
                success, message = kill_process(pid, force=False)
            elif action == "kill9":
                success, message = kill_process(pid, force=True)
            else:
                success, message = False, "Unknown action"

            # Refresh process list after kill
            processes = get_processes()
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": get_process_results(processes),
                        "notify": message if success else None,
                    }
                )
            )
            return

    print(json.dumps({"type": "error", "message": f"Unknown step: {step}"}))


if __name__ == "__main__":
    main()
