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
import time
from pathlib import Path

# Test mode - skip external tool calls
TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

# Todo file location
# Prefer illogical-impulse path for seamless sync between hamr and ii sidebar
# Fallback to hamr-specific path for standalone users
STATE_DIR = Path(os.environ.get("XDG_STATE_HOME", Path.home() / ".local" / "state"))
II_TODO_FILE = STATE_DIR / "quickshell" / "user" / "todo.json"
HAMR_TODO_FILE = Path.home() / ".config" / "hamr" / "todo.json"


def get_todo_file() -> Path:
    """Get the todo file path, preferring ii path if ii is installed."""
    # If ii todo file exists, use it (sync with ii sidebar)
    if II_TODO_FILE.exists():
        return II_TODO_FILE
    # If ii config dir exists (ii is installed), use ii path even if file doesn't exist yet
    ii_config = Path.home() / ".config" / "quickshell" / "ii"
    if ii_config.exists():
        return II_TODO_FILE
    # Standalone mode: use hamr path
    return HAMR_TODO_FILE


TODO_FILE = get_todo_file()


def load_todos() -> list[dict]:
    """Load todos from file, sorted by creation date (newest first)"""
    if not TODO_FILE.exists():
        return []
    try:
        with open(TODO_FILE) as f:
            todos = json.load(f)
            # Sort by created timestamp (newest first), fallback to 0 for old items
            todos.sort(key=lambda x: x.get("created", 0), reverse=True)
            return todos
    except (json.JSONDecodeError, IOError):
        return []


def get_status(todos: list[dict]) -> dict:
    """Get plugin status with pending count badge"""
    pending = sum(1 for t in todos if not t.get("done", False))
    if pending > 0:
        return {"badges": [{"text": str(pending)}]}
    return {"badges": []}


def save_todos(todos: list[dict]) -> None:
    """Save todos to file (sorted by creation date, newest first)"""
    # Sort before saving to maintain consistent order
    todos.sort(key=lambda x: x.get("created", 0), reverse=True)
    TODO_FILE.parent.mkdir(parents=True, exist_ok=True)
    with open(TODO_FILE, "w") as f:
        json.dump(todos, f)


def get_plugin_actions(todos: list[dict], in_add_mode: bool = False) -> list[dict]:
    """Get plugin-level actions for the action bar"""
    actions = []
    if not in_add_mode:
        actions.append(
            {
                "id": "add",
                "name": "Add Task",
                "icon": "add_circle",
                "shortcut": "Ctrl+1",
            }
        )
        # Show clear completed if there are any completed todos
        completed_count = sum(1 for t in todos if t.get("done", False))
        if completed_count > 0:
            actions.append(
                {
                    "id": "clear_completed",
                    "name": f"Clear Done ({completed_count})",
                    "icon": "delete_sweep",
                    "confirm": f"Remove {completed_count} completed task(s)?",
                    "shortcut": "Ctrl+2",
                }
            )
    return actions


def get_todo_results(todos: list[dict], show_add: bool = False) -> list[dict]:
    """Convert todos to result format"""
    results = []

    # No longer add "Add" as a result item - it's now a plugin action

    for i, todo in enumerate(todos):
        done = todo.get("done", False)
        content = todo.get("content", "")
        results.append(
            {
                "id": f"todo:{i}",
                "name": content,
                "icon": "check_circle" if done else "radio_button_unchecked",
                "description": "Done" if done else "Pending",
                "verb": "Undone" if done else "Done",
                "actions": [
                    {
                        "id": "toggle",
                        "name": "Undone" if done else "Done",
                        "icon": "undo" if done else "check_circle",
                    },
                    {"id": "edit", "name": "Edit", "icon": "edit"},
                    {"id": "delete", "name": "Delete", "icon": "delete"},
                ],
            }
        )

    if not todos:
        results.append(
            {
                "id": "__empty__",
                "name": "No tasks yet",
                "icon": "info",
                "description": "Use 'Add Task' button or Ctrl+1 to get started",
            }
        )

    return results


def refresh_sidebar():
    """Refresh the Todo sidebar via IPC"""
    if TEST_MODE:
        return  # Skip IPC in test mode
    try:
        subprocess.Popen(
            ["qs", "-c", "ii", "ipc", "call", "todo", "refresh"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except FileNotFoundError:
        pass  # qs not installed, skip refresh


def respond(
    results: list[dict],
    todos: list[dict],
    refresh_ui: bool = False,
    clear_input: bool = False,
    context: str = "",
    placeholder: str = "Search tasks...",
    input_mode: str = "realtime",
    plugin_actions: list[dict] | None = None,
    navigate_forward: bool | None = None,
):
    """Send a results response"""
    response = {
        "type": "results",
        "results": results,
        "inputMode": input_mode,
        "placeholder": placeholder,
        "status": get_status(todos),
    }
    if plugin_actions is not None:
        response["pluginActions"] = plugin_actions
    if clear_input:
        response["clearInput"] = True
    if context:
        response["context"] = context
    # Explicit navigation control - override core's pendingNavigation
    if navigate_forward is not None:
        response["navigateForward"] = navigate_forward
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

    if step == "index":
        print(json.dumps({"type": "index", "items": [], "status": get_status(todos)}))
        return

    if step == "initial":
        respond(
            get_todo_results(todos),
            todos,
            plugin_actions=get_plugin_actions(todos),
        )
        return

    if step == "search":
        # Add mode (submit mode)
        # In submit mode, Enter should perform the action directly (single press).
        if context == "__add_mode__":
            # Only add when query is provided (user pressed Enter in submit mode)
            if query:
                todos.append(
                    {
                        "content": query,
                        "done": False,
                        "created": int(time.time() * 1000),
                    }
                )
                save_todos(todos)
                respond(
                    get_todo_results(todos),
                    todos,
                    refresh_ui=True,
                    clear_input=True,
                    plugin_actions=get_plugin_actions(todos),
                )
                return
            # Empty query - stay in add mode
            respond(
                [],
                todos,
                placeholder="Type new task... (Enter to add)",
                context="__add_mode__",
                input_mode="submit",
            )
            return

        # Edit mode (submit mode)
        # In submit mode, Enter should save directly (single press).
        if context.startswith("__edit__:"):
            todo_idx = int(context.split(":")[1])
            if 0 <= todo_idx < len(todos):
                old_content = todos[todo_idx].get("content", "")
                if query:
                    todos[todo_idx]["content"] = query
                    save_todos(todos)
                    respond(
                        get_todo_results(todos),
                        todos,
                        refresh_ui=True,
                        clear_input=True,
                        plugin_actions=get_plugin_actions(todos),
                        navigate_forward=False,
                    )
                else:
                    # Show current value in placeholder
                    respond(
                        [],
                        todos,
                        placeholder=f"Edit: {old_content[:50]}{'...' if len(old_content) > 50 else ''} (Enter to save)",
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
                            "verb": "Undone" if done else "Done",
                            "actions": [
                                {
                                    "id": "toggle",
                                    "name": "Undone" if done else "Done",
                                    "icon": "undo" if done else "check_circle",
                                },
                                {"id": "edit", "name": "Edit", "icon": "edit"},
                                {"id": "delete", "name": "Delete", "icon": "delete"},
                            ],
                        }
                    )
            respond(filtered, todos, plugin_actions=get_plugin_actions(todos))
        else:
            respond(
                get_todo_results(todos),
                todos,
                plugin_actions=get_plugin_actions(todos),
            )
        return

    # Action: handle clicks
    if step == "action":
        item_id = selected.get("id", "")

        # Plugin-level actions (from action bar)
        if item_id == "__plugin__":
            if action == "add":
                # Enter add mode (submit mode) - empty results, placeholder tells user what to do
                respond(
                    [],
                    todos,
                    placeholder="Type new task... (Enter to add)",
                    clear_input=True,
                    context="__add_mode__",
                    input_mode="submit",
                    plugin_actions=[],  # Hide actions in add mode
                )
                return

            if action == "clear_completed":
                # Remove all completed todos - this is a modification, not navigation
                todos = [t for t in todos if not t.get("done", False)]
                save_todos(todos)
                respond(
                    get_todo_results(todos),
                    todos,
                    refresh_ui=True,
                    clear_input=True,
                    plugin_actions=get_plugin_actions(todos),
                    navigate_forward=False,  # Override pendingNavigation
                )
                return

        # Back
        if item_id == "__back__":
            respond(
                get_todo_results(todos),
                todos,
                clear_input=True,
                plugin_actions=get_plugin_actions(todos),
            )
            return

        # Enter add mode (submit mode) - legacy item click support
        if item_id == "__add__":
            respond(
                [],
                todos,
                placeholder="Type new task... (Enter to add)",
                clear_input=True,
                context="__add_mode__",
                input_mode="submit",
            )
            return

        # Add task (content encoded in ID)
        # This is a modification, not navigation - don't increase depth
        if item_id.startswith("__add__:"):
            encoded = item_id.split(":", 1)[1]
            if encoded:
                try:
                    task_content = base64.b64decode(encoded).decode()
                    todos.append(
                        {
                            "content": task_content,
                            "done": False,
                            "created": int(time.time() * 1000),
                        }
                    )
                    save_todos(todos)
                    respond(
                        get_todo_results(todos),
                        todos,
                        refresh_ui=True,
                        clear_input=True,
                        plugin_actions=get_plugin_actions(todos),
                        navigate_forward=False,  # Override pendingNavigation
                    )
                    return
                except Exception:
                    pass
            respond(
                get_todo_results(todos),
                todos,
                plugin_actions=get_plugin_actions(todos),
                navigate_forward=False,  # Override pendingNavigation
            )
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
                            get_todo_results(todos),
                            todos,
                            refresh_ui=True,
                            clear_input=True,
                            plugin_actions=get_plugin_actions(todos),
                            navigate_forward=False,
                        )
                        return
                    except Exception:
                        pass
            respond(
                get_todo_results(todos),
                todos,
                clear_input=True,
                plugin_actions=get_plugin_actions(todos),
            )
            return

        if item_id == "__empty__":
            respond(
                get_todo_results(todos),
                todos,
                plugin_actions=get_plugin_actions(todos),
            )
            return

        # Todo item actions
        if item_id.startswith("todo:"):
            todo_idx = int(item_id.split(":")[1])

            if action == "toggle" or not action:
                # Toggle done (default click action)
                # This is a modification, not navigation - don't increase depth
                if 0 <= todo_idx < len(todos):
                    todos[todo_idx]["done"] = not todos[todo_idx].get("done", False)
                    save_todos(todos)
                    respond(
                        get_todo_results(todos),
                        todos,
                        refresh_ui=True,
                        plugin_actions=get_plugin_actions(todos),
                        navigate_forward=False,  # Override pendingNavigation
                    )
                return

            if action == "edit":
                if 0 <= todo_idx < len(todos):
                    content = todos[todo_idx].get("content", "")
                    # Show current value in placeholder - empty results for clean UI
                    respond(
                        [],
                        todos,
                        placeholder=f"Edit: {content[:50]}{'...' if len(content) > 50 else ''} (Enter to save)",
                        clear_input=True,
                        context=f"__edit__:{todo_idx}",
                        input_mode="submit",
                    )
                return

            if action == "delete":
                if 0 <= todo_idx < len(todos):
                    todos.pop(todo_idx)
                    save_todos(todos)
                    respond(
                        get_todo_results(todos),
                        todos,
                        refresh_ui=True,
                        plugin_actions=get_plugin_actions(todos),
                    )
                return

    # Unknown
    print(json.dumps({"type": "error", "message": f"Unknown step: {step}"}))


if __name__ == "__main__":
    main()
