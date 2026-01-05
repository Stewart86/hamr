#!/usr/bin/env python3
"""
Snippet workflow handler - manage and insert text snippets
Reads snippets from ~/.config/hamr/snippets.json

Features:
- Browse and search snippets by key
- Insert snippet value using ydotool
- Add new snippets (key + value)
- Edit existing snippets
- Delete snippets
- Daemon mode with inotify file watching for live index updates

Note: Uses a delay before typing to allow focus to return to previous window
"""

import ctypes
import ctypes.util
import json
import os
import select
import signal
import struct
import subprocess
import sys
import shutil
import time
from datetime import datetime
from pathlib import Path

# inotify constants
IN_CLOSE_WRITE = 0x00000008
IN_MOVED_TO = 0x00000080
IN_CREATE = 0x00000100

# Test mode - mock external tool availability
TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

SNIPPETS_PATH = Path.home() / ".config/hamr/snippets.json"
# Delay in ms before typing to allow focus to return
TYPE_DELAY_MS = 150


def get_clipboard_content() -> str:
    """Get current clipboard content"""
    if TEST_MODE:
        return "clipboard_content"
    try:
        result = subprocess.run(
            ["wl-paste", "-n"],
            capture_output=True,
            text=True,
            timeout=2,
        )
        return result.stdout if result.returncode == 0 else ""
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return ""


def expand_variables(value: str) -> str:
    """Expand variables in snippet value.

    Supported variables:
    - {date} - Current date (YYYY-MM-DD)
    - {time} - Current time (HH:MM:SS)
    - {datetime} - Current date and time (YYYY-MM-DD HH:MM:SS)
    - {year} - Current year
    - {month} - Current month (01-12)
    - {day} - Current day (01-31)
    - {clipboard} - Current clipboard content
    - {user} - Current username
    - {home} - Home directory path
    """
    now = datetime.now()

    replacements = {
        "{date}": now.strftime("%Y-%m-%d"),
        "{time}": now.strftime("%H:%M:%S"),
        "{datetime}": now.strftime("%Y-%m-%d %H:%M:%S"),
        "{year}": now.strftime("%Y"),
        "{month}": now.strftime("%m"),
        "{day}": now.strftime("%d"),
        "{clipboard}": get_clipboard_content(),
        "{user}": os.environ.get("USER", ""),
        "{home}": str(Path.home()),
    }

    result = value
    for var, replacement in replacements.items():
        result = result.replace(var, replacement)

    return result


def has_variables(value: str) -> bool:
    """Check if value contains any expandable variables"""
    variables = [
        "{date}",
        "{time}",
        "{datetime}",
        "{year}",
        "{month}",
        "{day}",
        "{clipboard}",
        "{user}",
        "{home}",
    ]
    return any(var in value for var in variables)


def load_snippets() -> list[dict]:
    """Load snippets from config file"""
    if not SNIPPETS_PATH.exists():
        return []
    try:
        with open(SNIPPETS_PATH) as f:
            data = json.load(f)
            return data.get("snippets", [])
    except Exception:
        return []


def save_snippets(snippets: list[dict]) -> bool:
    """Save snippets to config file"""
    try:
        SNIPPETS_PATH.parent.mkdir(parents=True, exist_ok=True)
        with open(SNIPPETS_PATH, "w") as f:
            json.dump({"snippets": snippets}, f, indent=2)
        return True
    except Exception:
        return False


def fuzzy_match(query: str, text: str) -> bool:
    """Simple fuzzy match - all query chars appear in order"""
    query = query.lower()
    text = text.lower()
    qi = 0
    for c in text:
        if qi < len(query) and c == query[qi]:
            qi += 1
    return qi == len(query)


def filter_snippets(query: str, snippets: list[dict]) -> list[dict]:
    """Filter snippets by key or value preview"""
    if not query:
        return snippets

    results = []
    for snippet in snippets:
        if fuzzy_match(query, snippet["key"]):
            results.append(snippet)
            continue
        # Also search in value preview
        if fuzzy_match(query, snippet.get("value", "")[:50]):
            results.append(snippet)
    return results


def get_plugin_actions(in_add_mode: bool = False) -> list[dict]:
    """Get plugin-level actions for the action bar"""
    if in_add_mode:
        return []  # No actions while in form
    return [
        {
            "id": "add",
            "name": "Add Snippet",
            "icon": "add_circle",
            "shortcut": "Ctrl+1",
        }
    ]


def show_add_form(key_default: str = "", key_error: str = ""):
    """Show form for adding a new snippet"""
    fields = [
        {
            "id": "key",
            "type": "text",
            "label": "Key",
            "placeholder": "Snippet key/name",
            "required": True,
            "default": key_default,
        },
        {
            "id": "value",
            "type": "textarea",
            "label": "Value",
            "placeholder": "Snippet content...\n\nSupports multiple lines.",
            "rows": 6,
            "required": True,
        },
    ]
    if key_error:
        fields[0]["hint"] = key_error

    print(
        json.dumps(
            {
                "type": "form",
                "form": {
                    "title": "Add New Snippet",
                    "submitLabel": "Save",
                    "cancelLabel": "Cancel",
                    "fields": fields,
                },
                "context": "__add__",
            }
        )
    )


def show_edit_form(key: str, current_value: str):
    """Show form for editing an existing snippet"""
    print(
        json.dumps(
            {
                "type": "form",
                "form": {
                    "title": f"Edit Snippet: {key}",
                    "submitLabel": "Save",
                    "cancelLabel": "Cancel",
                    "fields": [
                        {
                            "id": "value",
                            "type": "textarea",
                            "label": "Value",
                            "placeholder": "Snippet content...",
                            "rows": 6,
                            "required": True,
                            "default": current_value,
                        },
                    ],
                },
                "context": f"__edit__:{key}",
            }
        )
    )


def truncate_value(value: str, max_len: int = 60) -> str:
    """Truncate value for display"""
    # Replace newlines with spaces for preview
    preview = value.replace("\n", " ").replace("\r", "")
    if len(preview) > max_len:
        return preview[:max_len] + "..."
    return preview


def get_snippet_list(snippets: list[dict], show_actions: bool = True) -> list[dict]:
    """Convert snippets to result format for browsing"""
    results = []
    for snippet in snippets:
        result = {
            "id": snippet["key"],
            "name": snippet["key"],
            "description": truncate_value(snippet.get("value", "")),
            "icon": "content_paste",
            "verb": "Insert",
        }
        if show_actions:
            result["actions"] = [
                {"id": "copy", "name": "Copy", "icon": "content_copy"},
                {"id": "edit", "name": "Edit", "icon": "edit"},
                {"id": "delete", "name": "Delete", "icon": "delete"},
            ]
        results.append(result)
    return results


def get_main_menu(snippets: list[dict], query: str = "") -> list[dict]:
    """Get main menu with snippets (add option now in action bar)"""
    results = []

    # Filter snippets
    filtered = filter_snippets(query, snippets)
    results.extend(get_snippet_list(filtered))

    if not results:
        results.append(
            {
                "id": "__empty__",
                "name": "No snippets yet",
                "icon": "info",
                "description": "Use 'Add Snippet' button or Ctrl+1 to create one",
            }
        )

    return results


def check_ydotool() -> bool:
    """Check if ydotool is available"""
    if TEST_MODE:
        return True  # Assume available in test mode
    return shutil.which("ydotool") is not None


def snippet_to_index_item(snippet: dict) -> dict:
    """Convert a snippet to an index item for main search"""
    key = snippet["key"]
    value = snippet.get("value", "")
    has_vars = has_variables(value)
    description = truncate_value(value, 50)
    if has_vars:
        description = "(has variables) " + description

    return {
        "id": key,  # Use simple key (matches result IDs for frecency)
        "name": key,
        "description": description,
        "keywords": [truncate_value(value, 30)],
        "icon": "content_paste",
        "verb": "Insert",
        "actions": [
            {
                "id": "copy",
                "name": "Copy",
                "icon": "content_copy",
                "entryPoint": {
                    "step": "action",
                    "selected": {"id": key},
                    "action": "copy",
                },
            },
            {
                "id": "edit",
                "name": "Edit",
                "icon": "edit",
                "entryPoint": {
                    "step": "action",
                    "selected": {"id": key},
                    "action": "edit",
                },
                "keepOpen": True,
            },
            {
                "id": "delete",
                "name": "Delete",
                "icon": "delete",
                "entryPoint": {
                    "step": "action",
                    "selected": {"id": key},
                    "action": "delete",
                },
            },
        ],
        # Default action: type with ydotool (goes through daemon for variable expansion)
        "entryPoint": {
            "step": "action",
            "selected": {"id": key},
        },
    }


def get_file_mtime() -> float:
    """Get the modification time of snippets file"""
    if not SNIPPETS_PATH.exists():
        return 0
    try:
        return SNIPPETS_PATH.stat().st_mtime
    except OSError:
        return 0


def create_inotify_fd() -> int | None:
    """Create inotify fd watching the snippets directory. Returns fd or None."""
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

        SNIPPETS_PATH.parent.mkdir(parents=True, exist_ok=True)
        watch_dir = str(SNIPPETS_PATH.parent).encode()
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


def emit(data: dict) -> None:
    """Emit JSON response to stdout (line-buffered)."""
    print(json.dumps(data), flush=True)


def handle_request(request: dict, snippets: list[dict]) -> list[dict]:
    """Process a request and return updated snippets (may be modified by form/action)"""
    step = request.get("step", "initial")
    query = request.get("query", "").strip()
    selected = request.get("selected", {})
    action = request.get("action", "")
    context = request.get("context", "")
    selected_id = selected.get("id", "")

    if step == "index":
        mode = request.get("mode", "full")
        indexed_ids = set(request.get("indexedIds", []))

        # Build current ID set (use simple key, matches index item IDs)
        current_ids = {s["key"] for s in snippets}

        if mode == "incremental" and indexed_ids:
            # Find new items
            new_ids = current_ids - indexed_ids
            new_items = [
                snippet_to_index_item(s) for s in snippets if s["key"] in new_ids
            ]

            # Find removed items
            removed_ids = list(indexed_ids - current_ids)

            emit(
                {
                    "type": "index",
                    "mode": "incremental",
                    "items": new_items,
                    "remove": removed_ids,
                }
            )
        else:
            # Full reindex
            items = [snippet_to_index_item(s) for s in snippets]
            emit({"type": "index", "items": items})
        return snippets

    if step == "initial":
        results = get_main_menu(snippets)
        emit(
            {
                "type": "results",
                "results": results,
                "inputMode": "realtime",
                "placeholder": "Search snippets...",
                "pluginActions": get_plugin_actions(),
            }
        )
        return snippets

    if step == "form":
        form_data = request.get("formData", {})

        # Adding new snippet
        if context == "__add__":
            key = form_data.get("key", "").strip()
            value = form_data.get("value", "")

            if not key:
                emit({"type": "error", "message": "Key is required"})
                return snippets

            if any(s["key"] == key for s in snippets):
                emit({"type": "error", "message": f"Key '{key}' already exists"})
                return snippets

            if not value:
                emit({"type": "error", "message": "Value is required"})
                return snippets

            new_snippet = {"key": key, "value": value}
            snippets.append(new_snippet)

            if save_snippets(snippets):
                emit(
                    {
                        "type": "results",
                        "results": get_main_menu(snippets),
                        "inputMode": "realtime",
                        "clearInput": True,
                        "context": "",
                        "placeholder": "Search snippets...",
                        "pluginActions": get_plugin_actions(),
                        "navigateBack": True,
                    }
                )
            else:
                emit({"type": "error", "message": "Failed to save snippet"})
            return snippets

        # Editing existing snippet
        if context.startswith("__edit__:"):
            key = context.split(":", 1)[1]
            value = form_data.get("value", "")

            if not value:
                emit({"type": "error", "message": "Value is required"})
                return snippets

            for s in snippets:
                if s["key"] == key:
                    s["value"] = value
                    break

            if save_snippets(snippets):
                emit(
                    {
                        "type": "results",
                        "results": get_main_menu(snippets),
                        "inputMode": "realtime",
                        "clearInput": True,
                        "context": "",
                        "placeholder": "Search snippets...",
                        "pluginActions": get_plugin_actions(),
                        "navigateBack": True,
                    }
                )
            else:
                emit({"type": "error", "message": "Failed to save snippet"})
            return snippets

    if step == "search":
        # Normal snippet filtering (realtime mode)
        results = get_main_menu(snippets, query)
        emit(
            {
                "type": "results",
                "inputMode": "realtime",
                "results": results,
                "placeholder": "Search snippets...",
                "pluginActions": get_plugin_actions(),
            }
        )
        return snippets

    if step == "action":
        # Plugin-level action: add (from action bar)
        if selected_id == "__plugin__" and action == "add":
            show_add_form()
            return snippets

        # Form cancelled - return to list
        if selected_id == "__form_cancel__":
            emit(
                {
                    "type": "results",
                    "results": get_main_menu(snippets),
                    "inputMode": "realtime",
                    "clearInput": True,
                    "context": "",
                    "placeholder": "Search snippets...",
                    "pluginActions": get_plugin_actions(),
                }
            )
            return snippets

        # Back button (legacy support)
        if selected_id == "__back__":
            emit(
                {
                    "type": "results",
                    "results": get_main_menu(snippets),
                    "inputMode": "realtime",
                    "clearInput": True,
                    "context": "",
                    "placeholder": "Search snippets...",
                    "pluginActions": get_plugin_actions(),
                }
            )
            return snippets

        # Non-actionable items
        if selected_id in ("__error__", "__current_value__", "__tip__", "__empty__"):
            return snippets

        # Copy action
        if action == "copy":
            snippet = next((s for s in snippets if s["key"] == selected_id), None)
            if snippet:
                expanded_value = expand_variables(snippet["value"])
                emit(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["wl-copy", expanded_value],
                            "name": f"Copy snippet: {selected_id}",
                            "icon": "content_copy",
                            "notify": f"Copied '{selected_id}' to clipboard",
                            "close": True,
                        },
                    }
                )
            return snippets

        # Edit action - show edit form
        if action == "edit":
            snippet = next((s for s in snippets if s["key"] == selected_id), None)
            if snippet:
                show_edit_form(selected_id, snippet.get("value", ""))
            return snippets

        # Delete action
        if action == "delete":
            snippets = [s for s in snippets if s["key"] != selected_id]
            if save_snippets(snippets):
                emit(
                    {
                        "type": "results",
                        "results": get_main_menu(snippets),
                        "inputMode": "realtime",
                        "clearInput": True,
                        "context": "",
                        "placeholder": "Search snippets...",
                        "pluginActions": get_plugin_actions(),
                        "navigateBack": True,
                    }
                )
            else:
                emit({"type": "error", "message": "Failed to save snippet"})
            return snippets

        # Start adding new snippet (legacy item click support)
        if selected_id == "__add__":
            show_add_form()
            return snippets

        # Direct snippet selection - insert using ydotool
        snippet = next((s for s in snippets if s["key"] == selected_id), None)
        if not snippet:
            emit({"type": "error", "message": f"Snippet not found: {selected_id}"})
            return snippets

        # Check ydotool availability
        if not check_ydotool():
            # Fallback to clipboard
            expanded_value = expand_variables(snippet["value"])
            emit(
                {
                    "type": "execute",
                    "execute": {
                        "command": ["wl-copy", expanded_value],
                        "name": f"Copy snippet: {selected_id}",
                        "icon": "content_copy",
                        "notify": f"ydotool not found. Copied '{selected_id}' to clipboard instead.",
                        "close": True,
                    },
                }
            )
            return snippets

        # Use ydotool to type the snippet value
        # Add delay to allow launcher to close and focus to return
        raw_value = snippet["value"]
        expanded_value = expand_variables(raw_value)
        emit(
            {
                "type": "execute",
                "execute": {
                    "command": [
                        "bash",
                        "-c",
                        f"sleep 0.{TYPE_DELAY_MS} && ydotool type --key-delay 0 -- {repr(expanded_value)}",
                    ],
                    "name": f"Insert snippet: {selected_id}",
                    "icon": "content_paste",
                    "close": True,
                },
            }
        )
        return snippets

    return snippets


def main():
    """Main daemon loop with inotify file watching"""

    def shutdown_handler(signum, frame):
        sys.exit(0)

    signal.signal(signal.SIGTERM, shutdown_handler)
    signal.signal(signal.SIGINT, shutdown_handler)

    snippets = load_snippets()
    current_query = ""
    # Disable inotify in test mode to avoid race conditions
    inotify_fd = None if TEST_MODE else create_inotify_fd()

    # Emit full index on startup, but only if no input is waiting
    # (if input is waiting, we're likely being started for an entryPoint execution)
    if not TEST_MODE:
        # Check if stdin has data waiting (non-blocking)
        ready, _, _ = select.select([sys.stdin], [], [], 0)
        if not ready:
            items = [snippet_to_index_item(s) for s in snippets]
            emit({"type": "index", "mode": "full", "items": items})

    if inotify_fd is not None:
        snippets_filename = SNIPPETS_PATH.name

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
                    snippets = handle_request(request, snippets)
                except (json.JSONDecodeError, ValueError):
                    continue

            if inotify_fd in readable:
                changed_files = read_inotify_events(inotify_fd)
                if snippets_filename in changed_files:
                    snippets = load_snippets()
                    # Emit updated index and results
                    items = [snippet_to_index_item(s) for s in snippets]
                    emit({"type": "index", "items": items})
                    results = get_main_menu(snippets, current_query)
                    emit(
                        {
                            "type": "results",
                            "results": results,
                            "inputMode": "realtime",
                            "placeholder": "Search snippets...",
                            "pluginActions": get_plugin_actions(),
                        }
                    )
    else:
        # Fallback to mtime polling if inotify unavailable
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
                    snippets = handle_request(request, snippets)
                except (json.JSONDecodeError, ValueError):
                    continue

            now = time.time()
            if now - last_check >= check_interval:
                new_mtime = get_file_mtime()
                if new_mtime != last_mtime and new_mtime != 0:
                    last_mtime = new_mtime
                    snippets = load_snippets()
                    # Emit updated index and results
                    items = [snippet_to_index_item(s) for s in snippets]
                    emit({"type": "index", "items": items})
                    results = get_main_menu(snippets, current_query)
                    emit(
                        {
                            "type": "results",
                            "results": results,
                            "inputMode": "realtime",
                            "placeholder": "Search snippets...",
                            "pluginActions": get_plugin_actions(),
                        }
                    )
                last_check = now


if __name__ == "__main__":
    main()
