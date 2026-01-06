#!/usr/bin/env python3
import base64
import ctypes
import ctypes.util
import json
import os
import select
import signal
import struct
import subprocess
import sys
import time
from pathlib import Path

# Test mode - skip external tool calls
TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

# inotify constants
IN_CLOSE_WRITE = 0x00000008
IN_MOVED_TO = 0x00000080
IN_CREATE = 0x00000100

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
    pending = sum(1 for t in todos if not t.get("done", False))
    if pending > 0:
        label = "task" if pending == 1 else "tasks"
        return {"chips": [{"text": f"{pending} {label}", "icon": "task_alt"}]}
    return {}


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


def emit(data: dict) -> None:
    print(json.dumps(data), flush=True)


def emit_status(todos: list[dict]) -> None:
    emit({"type": "status", "status": get_status(todos)})


def emit_index(todos: list[dict]) -> None:
    emit({"type": "index", "items": get_todo_results(todos)})


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
    if navigate_forward is not None:
        response["navigateForward"] = navigate_forward
    if refresh_ui:
        refresh_sidebar()
    emit(response)


def handle_request(request: dict, current_query: str) -> tuple[str, list[dict]]:
    step = request.get("step", "initial")
    query = request.get("query", "").strip()
    selected = request.get("selected", {})
    action = request.get("action", "")
    context = request.get("context", "")

    todos = load_todos()

    if step == "index":
        emit_index(todos)
        return query, todos

    if step == "initial":
        respond(
            get_todo_results(todos),
            todos,
            plugin_actions=get_plugin_actions(todos),
        )
        return query, todos

    if step == "search":
        if context == "__add_mode__":
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
                return "", todos
            respond(
                [],
                todos,
                placeholder="Type new task... (Enter to add)",
                context="__add_mode__",
                input_mode="submit",
            )
            return query, todos

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
                    return "", todos
                respond(
                    [],
                    todos,
                    placeholder=f"Edit: {old_content[:50]}{'...' if len(old_content) > 50 else ''} (Enter to save)",
                    context=context,
                    input_mode="submit",
                )
            return query, todos

        # Hybrid search: only return "Add" shortcut - hamr appends builtin search results
        if query:
            encoded = base64.b64encode(query.encode()).decode()
            respond(
                [
                    {
                        "id": f"__add__:{encoded}",
                        "key": "__add__",
                        "name": f"Add: {query}",
                        "icon": "add_circle",
                        "description": "Press Enter to add as new task",
                    }
                ],
                todos,
                plugin_actions=get_plugin_actions(todos),
            )
        else:
            respond(
                get_todo_results(todos),
                todos,
                plugin_actions=get_plugin_actions(todos),
            )
        return query, todos

    if step == "action":
        item_id = selected.get("id", "")

        if item_id == "__plugin__":
            if action == "add":
                respond(
                    [],
                    todos,
                    placeholder="Type new task... (Enter to add)",
                    clear_input=True,
                    context="__add_mode__",
                    input_mode="submit",
                    plugin_actions=[],
                )
                return "", todos

            if action == "clear_completed":
                todos = [t for t in todos if not t.get("done", False)]
                save_todos(todos)
                respond(
                    get_todo_results(todos),
                    todos,
                    refresh_ui=True,
                    clear_input=True,
                    plugin_actions=get_plugin_actions(todos),
                    navigate_forward=False,
                )
                return "", todos

        if item_id == "__back__":
            respond(
                get_todo_results(todos),
                todos,
                clear_input=True,
                plugin_actions=get_plugin_actions(todos),
            )
            return query, todos

        if item_id == "__add__":
            respond(
                [],
                todos,
                placeholder="Type new task... (Enter to add)",
                clear_input=True,
                context="__add_mode__",
                input_mode="submit",
            )
            return "", todos

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
                        navigate_forward=False,
                    )
                    return "", todos
                except Exception:
                    pass
            respond(
                get_todo_results(todos),
                todos,
                plugin_actions=get_plugin_actions(todos),
                navigate_forward=False,
            )
            return "", todos

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
                        return "", todos
                    except Exception:
                        pass
            respond(
                get_todo_results(todos),
                todos,
                clear_input=True,
                plugin_actions=get_plugin_actions(todos),
            )
            return "", todos

        if item_id == "__empty__":
            respond(
                get_todo_results(todos),
                todos,
                plugin_actions=get_plugin_actions(todos),
            )
            return query, todos

        if item_id.startswith("todo:"):
            todo_idx = int(item_id.split(":")[1])

            if action == "toggle" or not action:
                if 0 <= todo_idx < len(todos):
                    todos[todo_idx]["done"] = not todos[todo_idx].get("done", False)
                    save_todos(todos)
                    respond(
                        get_todo_results(todos),
                        todos,
                        refresh_ui=True,
                        plugin_actions=get_plugin_actions(todos),
                        navigate_forward=False,
                    )
                return query, todos

            if action == "edit":
                if 0 <= todo_idx < len(todos):
                    content = todos[todo_idx].get("content", "")
                    respond(
                        [],
                        todos,
                        placeholder=f"Edit: {content[:50]}{'...' if len(content) > 50 else ''} (Enter to save)",
                        clear_input=True,
                        context=f"__edit__:{todo_idx}",
                        input_mode="submit",
                    )
                return query, todos

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
                return "", todos

    return query, todos


def get_file_mtime() -> float:
    if not TODO_FILE.exists():
        return 0
    try:
        return TODO_FILE.stat().st_mtime
    except OSError:
        return 0


def create_inotify_fd() -> int | None:
    """Create inotify fd watching the todo file directory. Returns fd or None."""
    try:
        libc_name = ctypes.util.find_library("c")
        if not libc_name:
            return None
        libc = ctypes.CDLL(libc_name, use_errno=True)

        inotify_init = libc.inotify_init
        inotify_init.argtypes = []
        inotify_init.restype = ctypes.c_int

        inotify_add_watch = libc.inotify_add_watch
        inotify_add_watch.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_uint32]
        inotify_add_watch.restype = ctypes.c_int

        fd = inotify_init()
        if fd < 0:
            return None

        TODO_FILE.parent.mkdir(parents=True, exist_ok=True)
        watch_dir = str(TODO_FILE.parent).encode()
        mask = IN_CLOSE_WRITE | IN_MOVED_TO | IN_CREATE
        wd = inotify_add_watch(fd, watch_dir, mask)
        if wd < 0:
            os.close(fd)
            return None

        return fd
    except Exception:
        return None


def read_inotify_events(fd: int) -> list[str]:
    """Read inotify events and return list of filenames that changed."""
    filenames = []
    try:
        buf = os.read(fd, 4096)
        offset = 0
        while offset < len(buf):
            wd, mask, cookie, length = struct.unpack_from("iIII", buf, offset)
            offset += 16
            if length > 0:
                name = buf[offset : offset + length].rstrip(b"\x00").decode()
                filenames.append(name)
                offset += length
    except (OSError, struct.error):
        pass
    return filenames


def main():
    def shutdown_handler(signum, frame):
        sys.exit(0)

    signal.signal(signal.SIGTERM, shutdown_handler)
    signal.signal(signal.SIGINT, shutdown_handler)

    todos = load_todos()
    if not TEST_MODE:
        emit_status(todos)
        emit_index(todos)

    current_query = ""
    inotify_fd = create_inotify_fd()

    if inotify_fd is not None:
        todo_filename = TODO_FILE.name

        while True:
            readable, _, _ = select.select([sys.stdin, inotify_fd], [], [], 1.0)

            stdin_ready = any(
                (f if isinstance(f, int) else f.fileno()) == sys.stdin.fileno()
                for f in readable
            )
            if stdin_ready:
                try:
                    line = sys.stdin.readline()
                    if not line:
                        break
                    request = json.loads(line.strip())
                    current_query, todos = handle_request(request, current_query)
                except (json.JSONDecodeError, ValueError):
                    continue

            if inotify_fd in readable:
                changed_files = read_inotify_events(inotify_fd)
                if todo_filename in changed_files:
                    todos = load_todos()
                    if not TEST_MODE:
                        emit_index(todos)
                        respond(
                            get_todo_results(todos),
                            todos,
                            plugin_actions=get_plugin_actions(todos),
                        )
    else:
        last_mtime = get_file_mtime()
        last_check = time.time()
        check_interval = 2.0

        while True:
            readable, _, _ = select.select([sys.stdin], [], [], 0.5)

            if readable:
                try:
                    line = sys.stdin.readline()
                    if not line:
                        break
                    request = json.loads(line.strip())
                    current_query, todos = handle_request(request, current_query)
                except (json.JSONDecodeError, ValueError):
                    continue

            now = time.time()
            if now - last_check >= check_interval:
                new_mtime = get_file_mtime()
                if new_mtime != last_mtime and new_mtime != 0:
                    last_mtime = new_mtime
                    todos = load_todos()
                    if not TEST_MODE:
                        emit_index(todos)
                        respond(
                            get_todo_results(todos),
                            todos,
                            plugin_actions=get_plugin_actions(todos),
                        )
                last_check = now


if __name__ == "__main__":
    main()
