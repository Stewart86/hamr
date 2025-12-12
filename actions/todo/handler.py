#!/usr/bin/env python3
"""
Todo workflow handler - manage your todo list.
Features: list, add, toggle done/undone, edit, delete
"""

import base64
import json
import os
import subprocess
import sys
from pathlib import Path

# Todo file location (same as Quickshell's Todo.qml)
STATE_DIR = Path(os.environ.get("XDG_STATE_HOME", Path.home() / ".local" / "state"))
TODO_FILE = STATE_DIR / "quickshell" / "user" / "todo.json"


def load_todos() -> list[dict]:
    """Load todos from file"""
    if not TODO_FILE.exists():
        return []
    try:
        with open(TODO_FILE) as f:
            return json.load(f)
    except (json.JSONDecodeError, IOError):
        return []


def save_todos(todos: list[dict]) -> None:
    """Save todos to file"""
    TODO_FILE.parent.mkdir(parents=True, exist_ok=True)
    with open(TODO_FILE, "w") as f:
        json.dump(todos, f)


def get_todo_results(todos: list[dict], show_add: bool = True) -> list[dict]:
    """Convert todos to result format"""
    results = []

    if show_add:
        results.append(
            {
                "id": "__add__",
                "name": "Add new task...",
                "icon": "add_circle",
                "description": "Type to create a new task",
            }
        )

    for i, todo in enumerate(todos):
        done = todo.get("done", False)
        content = todo.get("content", "")
        results.append(
            {
                "id": f"todo:{i}",
                "name": content,
                "icon": "check_circle" if done else "radio_button_unchecked",
                "description": "Done" if done else "Pending",
                "actions": [
                    {
                        "id": "toggle",
                        "name": "Toggle done",
                        "icon": "check_circle" if not done else "undo",
                    },
                    {"id": "edit", "name": "Edit", "icon": "edit"},
                    {"id": "delete", "name": "Delete", "icon": "delete"},
                ],
            }
        )

    if not todos and show_add:
        results.append(
            {
                "id": "__empty__",
                "name": "No tasks yet",
                "icon": "info",
                "description": "Click 'Add new task' to get started",
            }
        )

    return results


def refresh_sidebar():
    """Refresh the Todo sidebar via IPC"""
    subprocess.Popen(
        ["qs", "-c", "ii", "ipc", "call", "todo", "refresh"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def respond(
    results: list[dict],
    refresh_ui: bool = False,
    clear_input: bool = False,
    context: str = "",
    placeholder: str = "Search tasks or type to add...",
    input_mode: str = "realtime",
):
    """Send a results response"""
    response = {
        "type": "results",
        "results": results,
        "inputMode": input_mode,
        "placeholder": placeholder,
    }
    if clear_input:
        response["clearInput"] = True
    if context:
        response["context"] = context
    if refresh_ui:
        refresh_sidebar()
    print(json.dumps(response))


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")
    context = input_data.get("context", "")

    todos = load_todos()

    # Initial: show todo list
    if step == "initial":
        respond(get_todo_results(todos))
        return

    # Search: filter todos or prepare to add
    if step == "search":
        # Add mode: show add option with encoded query (submit mode)
        if context == "__add_mode__":
            if query:
                encoded = base64.b64encode(query.encode()).decode()
                results = [
                    {"id": "__back__", "name": "Cancel", "icon": "arrow_back"},
                    {
                        "id": f"__add__:{encoded}",
                        "name": f"Add: {query}",
                        "icon": "add_circle",
                        "description": "Press Enter to add this task",
                    },
                ]
            else:
                results = [
                    {"id": "__back__", "name": "Cancel", "icon": "arrow_back"},
                    {
                        "id": "__add__:",
                        "name": "Type a task...",
                        "icon": "add_circle",
                        "description": "Start typing to add a new task",
                    },
                ]
            respond(
                results,
                placeholder="Type new task... (Enter to add)",
                context="__add_mode__",
                input_mode="submit",
            )
            return

        # Edit mode: show save option (submit mode)
        if context.startswith("__edit__:"):
            todo_idx = int(context.split(":")[1])
            if 0 <= todo_idx < len(todos):
                old_content = todos[todo_idx].get("content", "")
                encoded = base64.b64encode(query.encode()).decode() if query else ""
                results = [
                    {"id": "__back__", "name": "Cancel", "icon": "arrow_back"},
                    {
                        "id": f"__save__:{todo_idx}:{encoded}",
                        "name": f"Save: {query}" if query else "Type new content...",
                        "icon": "save",
                        "description": f"Original: {old_content}",
                    },
                ]
                respond(
                    results,
                    placeholder="Type new task content... (Enter to save)",
                    context=context,
                    input_mode="submit",
                )
            return

        # Normal search: filter + add option
        if query:
            filtered = []
            encoded = base64.b64encode(query.encode()).decode()
            filtered.append(
                {
                    "id": f"__add__:{encoded}",
                    "name": f"Add: {query}",
                    "icon": "add_circle",
                    "description": "Press Enter to add as new task",
                }
            )
            for i, todo in enumerate(todos):
                content = todo.get("content", "")
                if query.lower() in content.lower():
                    done = todo.get("done", False)
                    filtered.append(
                        {
                            "id": f"todo:{i}",
                            "name": content,
                            "icon": "check_circle"
                            if done
                            else "radio_button_unchecked",
                            "description": "Done" if done else "Pending",
                            "actions": [
                                {
                                    "id": "toggle",
                                    "name": "Toggle",
                                    "icon": "check_circle" if not done else "undo",
                                },
                                {"id": "edit", "name": "Edit", "icon": "edit"},
                                {"id": "delete", "name": "Delete", "icon": "delete"},
                            ],
                        }
                    )
            respond(filtered)
        else:
            respond(get_todo_results(todos))
        return

    # Action: handle clicks
    if step == "action":
        item_id = selected.get("id", "")

        # Back
        if item_id == "__back__":
            respond(get_todo_results(todos), clear_input=True)
            return

        # Enter add mode (submit mode)
        if item_id == "__add__":
            results = [
                {"id": "__back__", "name": "Cancel", "icon": "arrow_back"},
                {
                    "id": "__add__:",
                    "name": "Type a task and press Enter...",
                    "icon": "add_circle",
                    "description": "Start typing to add a new task",
                },
            ]
            respond(
                results,
                placeholder="Type new task... (Enter to add)",
                clear_input=True,
                context="__add_mode__",
                input_mode="submit",
            )
            return

        # Add task (content encoded in ID)
        if item_id.startswith("__add__:"):
            encoded = item_id.split(":", 1)[1]
            if encoded:
                try:
                    task_content = base64.b64decode(encoded).decode()
                    todos.append({"content": task_content, "done": False})
                    save_todos(todos)
                    respond(get_todo_results(todos), refresh_ui=True, clear_input=True)
                    return
                except Exception:
                    pass
            respond(get_todo_results(todos))
            return

        # Save edit (content encoded in ID)
        if item_id.startswith("__save__:"):
            parts = item_id.split(":", 2)
            if len(parts) >= 3:
                todo_idx = int(parts[1])
                encoded = parts[2]
                if encoded and 0 <= todo_idx < len(todos):
                    try:
                        new_content = base64.b64decode(encoded).decode()
                        todos[todo_idx]["content"] = new_content
                        save_todos(todos)
                        respond(
                            get_todo_results(todos), refresh_ui=True, clear_input=True
                        )
                        return
                    except Exception:
                        pass
            respond(get_todo_results(todos), clear_input=True)
            return

        # Empty state - ignore
        if item_id == "__empty__":
            respond(get_todo_results(todos))
            return

        # Todo item actions
        if item_id.startswith("todo:"):
            todo_idx = int(item_id.split(":")[1])

            if action == "toggle" or not action:
                # Toggle done (default click action)
                if 0 <= todo_idx < len(todos):
                    todos[todo_idx]["done"] = not todos[todo_idx].get("done", False)
                    save_todos(todos)
                    respond(get_todo_results(todos), refresh_ui=True)
                return

            if action == "edit":
                if 0 <= todo_idx < len(todos):
                    content = todos[todo_idx].get("content", "")
                    results = [
                        {"id": "__back__", "name": "Cancel", "icon": "arrow_back"},
                        {
                            "id": f"__save__:{todo_idx}:",
                            "name": "Save changes",
                            "icon": "save",
                            "description": f"Current: {content}",
                        },
                    ]
                    respond(
                        results,
                        placeholder="Type new content... (Enter to save)",
                        clear_input=True,
                        context=f"__edit__:{todo_idx}",
                        input_mode="submit",
                    )
                return

            if action == "delete":
                if 0 <= todo_idx < len(todos):
                    todos.pop(todo_idx)
                    save_todos(todos)
                    respond(get_todo_results(todos), refresh_ui=True)
                return

    # Unknown
    print(json.dumps({"type": "error", "message": f"Unknown step: {step}"}))


if __name__ == "__main__":
    main()
