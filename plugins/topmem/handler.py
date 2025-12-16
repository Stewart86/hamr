#!/usr/bin/env python3
"""
Top Memory workflow handler - show processes sorted by memory usage.
"""

import json
import os
import subprocess
import sys

TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

MOCK_PROCESSES = [
    {"pid": "5678", "name": "code", "cpu": 15.3, "mem": 12.1, "user": "user"},
    {"pid": "1234", "name": "firefox", "cpu": 25.5, "mem": 8.2, "user": "user"},
    {"pid": "9012", "name": "python3", "cpu": 8.7, "mem": 2.5, "user": "user"},
]


def get_processes() -> list[dict]:
    """Get processes sorted by memory usage"""
    if TEST_MODE:
        return MOCK_PROCESSES

    try:
        # Use ps to get process info sorted by memory
        result = subprocess.run(
            ["ps", "axo", "pid,user,%cpu,%mem,comm", "--sort=-%mem"],
            capture_output=True,
            text=True,
            check=True,
        )

        processes = []
        for line in result.stdout.strip().split("\n")[1:51]:  # Skip header, limit to 50
            parts = line.split()
            if len(parts) >= 5:
                pid = parts[0]
                user = parts[1]
                cpu = float(parts[2])
                mem = float(parts[3])
                name = parts[4]

                # Skip kernel threads and very low memory processes
                if mem < 0.1:
                    continue

                processes.append(
                    {
                        "pid": pid,
                        "name": name,
                        "cpu": cpu,
                        "mem": mem,
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
        results.append(
            {
                "id": f"proc:{proc['pid']}",
                "name": f"{proc['name']} ({proc['pid']})",
                "icon": "memory",
                "description": f"Mem: {proc['mem']:.1f}%  |  CPU: {proc['cpu']:.1f}%  |  User: {proc['user']}",
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
