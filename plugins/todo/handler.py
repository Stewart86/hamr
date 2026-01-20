#!/usr/bin/env python3
"""
Todo plugin - Manage your todo list.

Stores tasks in XDG_STATE_HOME/quickshell/user/todo.json (shared with illogical-impulse)
or ~/.config/hamr/todo.json (standalone).
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path

# Add parent directory to path to import SDK
sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

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


def save_todos(todos: list[dict]) -> None:
    """Save todos to file (sorted by creation date, newest first)"""
    # Sort before saving to maintain consistent order
    todos.sort(key=lambda x: x.get("created", 0), reverse=True)
    TODO_FILE.parent.mkdir(parents=True, exist_ok=True)
    with open(TODO_FILE, "w") as f:
        json.dump(todos, f)


def get_status(todos: list[dict]) -> dict:
    """Build status object with chips and badges"""
    pending = sum(1 for t in todos if not t.get("done", False))
    if pending > 0:
        label = "task" if pending == 1 else "tasks"
        return {"chips": [{"text": f"{pending} {label}", "icon": "task_alt"}]}
    return {}


def get_plugin_actions(todos: list[dict]) -> list[dict]:
    """Get plugin-level actions for the action bar"""
    actions = [
        {
            "id": "add",
            "name": "Add Task",
            "icon": "add_circle",
            "shortcut": "Ctrl+1",
        }
    ]
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


def get_todo_results(todos: list[dict]) -> list[dict]:
    """Convert todos to result format"""
    results = []

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
    try:
        subprocess.Popen(
            ["qs", "-c", "ii", "ipc", "call", "todo", "refresh"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except FileNotFoundError:
        pass


# Create plugin instance
plugin = HamrPlugin(
    id="todo",
    name="Todo",
    description="Manage your todo list",
    icon="checklist",
)

# Plugin state
plugin_state = {
    "todos": [],
    "current_query": "",
}


@plugin.on_initial
async def handle_initial(params=None):
    """Handle initial request when plugin is opened."""
    todos = load_todos()
    plugin_state["todos"] = todos
    results = get_todo_results(todos)

    return HamrPlugin.results(
        results,
        status=get_status(todos),
        input_mode="realtime",
        placeholder="Search tasks...",
        plugin_actions=get_plugin_actions(todos),
    )


@plugin.on_search
async def handle_search(query: str, context: str | None):
    """Handle search request."""
    todos = plugin_state["todos"]
    plugin_state["current_query"] = query

    # Edit mode: save the edited task
    if context and context.startswith("__edit__:") and query:
        try:
            todo_idx = int(context.split(":")[1])
            if 0 <= todo_idx < len(todos):
                todos[todo_idx]["content"] = query
                plugin_state["todos"] = todos
                save_todos(todos)
                refresh_sidebar()
                return HamrPlugin.results(
                    get_todo_results(todos),
                    status=get_status(todos),
                    clear_input=True,
                    context=None,  # Clear context to exit edit mode
                    input_mode="realtime",
                    placeholder="Search tasks...",
                    plugin_actions=get_plugin_actions(todos),
                )
        except (ValueError, IndexError):
            pass

    # Add new task mode (when plugin action "add" was triggered)
    if context == "__add_mode__" and query:
        todos.append(
            {
                "content": query,
                "done": False,
                "created": int(time.time() * 1000),
            }
        )
        plugin_state["todos"] = todos
        save_todos(todos)
        refresh_sidebar()
        return HamrPlugin.results(
            get_todo_results(todos),
            status=get_status(todos),
            clear_input=True,
            context=None,
            input_mode="realtime",
            placeholder="Search tasks...",
            plugin_actions=get_plugin_actions(todos),
        )

    # Quick add mode (typing in main view)
    if query and not context:
        return HamrPlugin.results(
            [
                {
                    "id": f"__add__:{query}",
                    "name": f"Add: {query}",
                    "icon": "add_circle",
                    "description": "Press Enter to add as new task",
                }
            ],
            status=get_status(todos),
            input_mode="realtime",
            placeholder="Search tasks...",
            plugin_actions=get_plugin_actions(todos),
        )

    # Search existing todos (empty query or edit mode without query)
    results = get_todo_results(todos)
    return HamrPlugin.results(
        results,
        status=get_status(todos),
        input_mode="realtime",
        placeholder="Search tasks...",
        plugin_actions=get_plugin_actions(todos),
    )


@plugin.on_action
async def handle_action(
    item_id: str, action: str | None, context: str | None, source: str | None = None
):
    """Handle action request."""
    todos = plugin_state["todos"]

    # Plugin-level actions
    if item_id == "__plugin__":
        if action == "add":
            return HamrPlugin.results(
                [],
                clear_input=True,
                context="__add_mode__",
                input_mode="submit",
                placeholder="Type new task... (Enter to add)",
                status=get_status(todos),
            )

        if action == "clear_completed":
            todos = [t for t in todos if not t.get("done", False)]
            plugin_state["todos"] = todos
            save_todos(todos)
            refresh_sidebar()
            return HamrPlugin.results(
                get_todo_results(todos),
                status=get_status(todos),
                clear_input=True,
                input_mode="realtime",
                placeholder="Search tasks...",
                plugin_actions=get_plugin_actions(todos),
            )

    # Empty state
    if item_id == "__empty__":
        return {}

    # Add quick item
    if item_id.startswith("__add__:"):
        task_content = item_id.split(":", 1)[1]
        if task_content:
            todos.append(
                {
                    "content": task_content,
                    "done": False,
                    "created": int(time.time() * 1000),
                }
            )
            plugin_state["todos"] = todos
            save_todos(todos)
            refresh_sidebar()
            return HamrPlugin.results(
                get_todo_results(todos),
                status=get_status(todos),
                clear_input=True,
                input_mode="realtime",
                placeholder="Search tasks...",
                plugin_actions=get_plugin_actions(todos),
            )
        return {}

    # Todo item actions
    if item_id.startswith("todo:"):
        try:
            todo_idx = int(item_id.split(":")[1])
        except (ValueError, IndexError):
            return {}

        if action == "toggle" or not action:
            if 0 <= todo_idx < len(todos):
                todos[todo_idx]["done"] = not todos[todo_idx].get("done", False)
                plugin_state["todos"] = todos
                save_todos(todos)
                refresh_sidebar()
                return HamrPlugin.results(
                    get_todo_results(todos),
                    status=get_status(todos),
                    input_mode="realtime",
                    placeholder="Search tasks...",
                    plugin_actions=get_plugin_actions(todos),
                )

        if action == "edit":
            if 0 <= todo_idx < len(todos):
                content = todos[todo_idx].get("content", "")
                return HamrPlugin.results(
                    [],
                    clear_input=True,
                    context=f"__edit__:{todo_idx}",
                    input_mode="submit",
                    placeholder=f"Edit: {content[:50]}{'...' if len(content) > 50 else ''} (Enter to save)",
                    status=get_status(todos),
                )

        if action == "delete":
            if 0 <= todo_idx < len(todos):
                todos.pop(todo_idx)
                plugin_state["todos"] = todos
                save_todos(todos)
                refresh_sidebar()
                return HamrPlugin.results(
                    get_todo_results(todos),
                    status=get_status(todos),
                    input_mode="realtime",
                    placeholder="Search tasks...",
                    plugin_actions=get_plugin_actions(todos),
                )

    return {}


@plugin.on_form_submitted
async def handle_form_submitted(form_data: dict, context: str | None):
    """Handle form submission."""
    # This plugin uses search mode for input instead of forms
    return {}


@plugin.add_background_task
async def emit_initial_status(p: HamrPlugin):
    """Background task to emit initial status on startup."""
    todos = load_todos()
    await p.send_status(get_status(todos))


if __name__ == "__main__":
    plugin.run()
