#!/usr/bin/env python3
"""
Quicklinks workflow handler - search the web with predefined quicklinks
Reads quicklinks from ~/.config/hamr/quicklinks.json

Features:
- Browse and search quicklinks
- Execute search with query placeholder
- Add new quicklinks (via form)
- Delete existing quicklinks
- Edit existing quicklinks (via form)
- Daemon mode with inotify file watching for live index updates
"""

import ctypes
import ctypes.util
import json
import os
import select
import signal
import struct
import sys
import time
import urllib.parse
from pathlib import Path

# inotify constants
IN_CLOSE_WRITE = 0x00000008
IN_MOVED_TO = 0x00000080
IN_CREATE = 0x00000100

QUICKLINKS_PATH = Path.home() / ".config/hamr/quicklinks.json"


def load_quicklinks() -> list[dict]:
    """Load quicklinks from config file"""
    if not QUICKLINKS_PATH.exists():
        return []
    try:
        with open(QUICKLINKS_PATH) as f:
            data = json.load(f)
            return data.get("quicklinks", [])
    except Exception:
        return []


def save_quicklinks(quicklinks: list[dict]) -> bool:
    """Save quicklinks to config file"""
    try:
        QUICKLINKS_PATH.parent.mkdir(parents=True, exist_ok=True)
        with open(QUICKLINKS_PATH, "w") as f:
            json.dump({"quicklinks": quicklinks}, f, indent=2)
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


def filter_quicklinks(query: str, quicklinks: list[dict]) -> list[dict]:
    """Filter quicklinks by name or aliases"""
    if not query:
        return quicklinks

    results = []
    for link in quicklinks:
        if fuzzy_match(query, link["name"]):
            results.append(link)
            continue
        for alias in link.get("aliases", []):
            if fuzzy_match(query, alias):
                results.append(link)
                break
    return results


def get_plugin_actions(in_form_mode: bool = False) -> list[dict]:
    """Get plugin-level actions for the action bar"""
    if in_form_mode:
        return []  # No actions while in form
    return [
        {
            "id": "add",
            "name": "Add Quicklink",
            "icon": "add_circle",
            "shortcut": "Ctrl+1",
        }
    ]


def show_add_form(
    name_default: str = "", url_default: str = "", icon_default: str = "link"
):
    """Show form for adding a new quicklink"""
    emit(
        {
            "type": "form",
            "form": {
                "title": "Add New Quicklink",
                "submitLabel": "Save",
                "cancelLabel": "Cancel",
                "fields": [
                    {
                        "id": "name",
                        "type": "text",
                        "label": "Name",
                        "placeholder": "Quicklink name (e.g., Google)",
                        "required": True,
                        "default": name_default,
                    },
                    {
                        "id": "url",
                        "type": "text",
                        "label": "URL",
                        "placeholder": "https://example.com/search?q={query}",
                        "required": True,
                        "default": url_default,
                        "hint": "Use {query} as placeholder for search queries",
                    },
                    {
                        "id": "icon",
                        "type": "text",
                        "label": "Icon",
                        "placeholder": "Material icon name (optional)",
                        "default": icon_default,
                    },
                ],
            },
            "context": "__add__",
        }
    )


def show_edit_form(name: str, current_url: str, current_icon: str = "link"):
    """Show form for editing an existing quicklink"""
    emit(
        {
            "type": "form",
            "form": {
                "title": f"Edit Quicklink: {name}",
                "submitLabel": "Save",
                "cancelLabel": "Cancel",
                "fields": [
                    {
                        "id": "url",
                        "type": "text",
                        "label": "URL",
                        "placeholder": "https://example.com/search?q={query}",
                        "required": True,
                        "default": current_url,
                        "hint": "Use {query} as placeholder for search queries",
                    },
                    {
                        "id": "icon",
                        "type": "text",
                        "label": "Icon",
                        "placeholder": "Material icon name",
                        "default": current_icon,
                    },
                ],
            },
            "context": f"__edit__:{name}",
        }
    )


def get_quicklink_list(quicklinks: list[dict], show_actions: bool = True) -> list[dict]:
    """Convert quicklinks to result format for browsing"""
    results = []
    for link in quicklinks:
        has_query = "{query}" in link.get("url", "")
        result = {
            "id": link["name"],
            "name": link["name"],
            "icon": link.get("icon", "link"),
            "verb": "Search" if has_query else "Open",
        }
        if link.get("aliases"):
            result["description"] = ", ".join(link["aliases"])
        if show_actions:
            result["actions"] = [
                {"id": "edit", "name": "Edit", "icon": "edit"},
                {"id": "delete", "name": "Delete", "icon": "delete"},
            ]
        results.append(result)
    return results


def is_quicklink_with_query(name: str, quicklinks: list[dict]) -> dict | None:
    """Check if a quicklink name exists and requires a query"""
    link = next((l for l in quicklinks if l["name"] == name), None)
    if link and "{query}" in link.get("url", ""):
        return link
    return None


def get_main_menu(quicklinks: list[dict], query: str = "") -> list[dict]:
    """Get main menu with quicklinks (add option now in action bar)"""
    results = []

    # Filter quicklinks
    filtered = filter_quicklinks(query, quicklinks)
    results.extend(get_quicklink_list(filtered))

    if not results:
        results.append(
            {
                "id": "__empty__",
                "name": "No quicklinks yet",
                "icon": "info",
                "description": "Use 'Add Quicklink' button or Ctrl+1 to create one",
            }
        )

    return results


def emit(data: dict) -> None:
    """Emit JSON response to stdout (line-buffered)."""
    print(json.dumps(data), flush=True)


def get_file_mtime() -> float:
    """Get the modification time of quicklinks file."""
    if not QUICKLINKS_PATH.exists():
        return 0
    try:
        return QUICKLINKS_PATH.stat().st_mtime
    except OSError:
        return 0


def create_inotify_fd() -> int | None:
    """Create inotify fd watching the quicklinks file directory. Returns fd or None."""
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

        QUICKLINKS_PATH.parent.mkdir(parents=True, exist_ok=True)
        watch_dir = str(QUICKLINKS_PATH.parent).encode()
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


def quicklink_to_index_item(link: dict) -> dict:
    """Convert a quicklink to indexable item format for main search."""
    has_query = "{query}" in link.get("url", "")
    url = link.get("url", "")
    name = link["name"]

    # Build keywords from name and aliases
    keywords = name.lower().split()
    for alias in link.get("aliases", []):
        keywords.extend(alias.lower().split())

    item = {
        "id": name,  # Use simple name (matches result IDs for frecency)
        "name": name,
        "description": ", ".join(link.get("aliases", []))
        if link.get("aliases")
        else ("Search" if has_query else "Open"),
        "keywords": keywords,
        "icon": link.get("icon", "link"),
        "verb": "Search" if has_query else "Open",
        "actions": [
            {
                "id": "edit",
                "name": "Edit",
                "icon": "edit",
                "entryPoint": {
                    "step": "action",
                    "selected": {"id": name},
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
                    "selected": {"id": name},
                    "action": "delete",
                },
            },
        ],
    }

    # If no query placeholder, can execute directly
    if not has_query:
        item["execute"] = {
            "openUrl": url,
        }
    # If has query, open the plugin for user to enter search term
    else:
        item["entryPoint"] = {
            "step": "action",
            "selected": {"id": name},
            "action": "search",
        }
        item["keepOpen"] = True

    return item


def handle_request(request: dict, quicklinks: list[dict], current_query: str) -> str:
    """Handle a single request. Returns updated query."""
    step = request.get("step", "initial")
    query = request.get("query", "").strip()
    selected = request.get("selected", {})
    action = request.get("action", "")

    selected_id = selected.get("id", "")
    context = request.get("context", "")

    if step == "index":
        mode = request.get("mode", "full")
        indexed_ids = set(request.get("indexedIds", []))

        # Build current ID set
        current_ids = {f"quicklink:{link['name']}" for link in quicklinks}

        if mode == "incremental" and indexed_ids:
            # Find new items
            new_ids = current_ids - indexed_ids
            new_items = [
                quicklink_to_index_item(link)
                for link in quicklinks
                if f"quicklink:{link['name']}" in new_ids
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
            items = [quicklink_to_index_item(link) for link in quicklinks]
            emit({"type": "index", "items": items})
        return current_query

    if step == "initial":
        results = get_main_menu(quicklinks)
        emit(
            {
                "type": "results",
                "results": results,
                "inputMode": "realtime",
                "placeholder": "Search quicklinks...",
                "pluginActions": get_plugin_actions(),
            }
        )
        return current_query

    if step == "form":
        form_data = request.get("formData", {})

        # Adding new quicklink
        if context == "__add__":
            name = form_data.get("name", "").strip()
            url = form_data.get("url", "").strip()
            icon = form_data.get("icon", "").strip() or "link"

            if not name:
                emit({"type": "error", "message": "Name is required"})
                return current_query

            if any(l["name"] == name for l in quicklinks):
                emit({"type": "error", "message": f"'{name}' already exists"})
                return current_query

            if not url:
                emit({"type": "error", "message": "URL is required"})
                return current_query

            # Add https:// if no protocol
            if not url.startswith("http://") and not url.startswith("https://"):
                url = "https://" + url

            new_link = {"name": name, "url": url, "icon": icon}
            quicklinks.append(new_link)

            if save_quicklinks(quicklinks):
                emit(
                    {
                        "type": "results",
                        "results": get_main_menu(quicklinks),
                        "inputMode": "realtime",
                        "clearInput": True,
                        "context": "",
                        "placeholder": "Search quicklinks...",
                        "pluginActions": get_plugin_actions(),
                        "navigateBack": True,
                    }
                )
            else:
                emit({"type": "error", "message": "Failed to save quicklink"})
            return ""

        # Editing existing quicklink
        if context.startswith("__edit__:"):
            name = context.split(":", 1)[1]
            url = form_data.get("url", "").strip()
            icon = form_data.get("icon", "").strip() or "link"

            if not url:
                emit({"type": "error", "message": "URL is required"})
                return current_query

            # Add https:// if no protocol
            if not url.startswith("http://") and not url.startswith("https://"):
                url = "https://" + url

            for link in quicklinks:
                if link["name"] == name:
                    link["url"] = url
                    link["icon"] = icon
                    break

            if save_quicklinks(quicklinks):
                emit(
                    {
                        "type": "results",
                        "results": get_main_menu(quicklinks),
                        "inputMode": "realtime",
                        "clearInput": True,
                        "context": "",
                        "placeholder": "Search quicklinks...",
                        "pluginActions": get_plugin_actions(),
                        "navigateBack": True,
                    }
                )
            else:
                emit({"type": "error", "message": "Failed to save quicklink"})
            return ""

    if step == "search":
        # Search mode for a quicklink with {query} (submit mode for search input)
        search_context = context if context.startswith("__search__:") else None
        if search_context:
            link_name = search_context.split(":", 1)[1]
            link = next((l for l in quicklinks if l["name"] == link_name), None)
        else:
            link = is_quicklink_with_query(selected_id, quicklinks)
            link_name = selected_id if link else None

        if link:
            search_placeholder = f"Search {link['name']}... (Enter to search)"
            if query:
                # Execute search directly on Enter (single press)
                url = link["url"].replace("{query}", urllib.parse.quote(query))
                emit(
                    {
                        "type": "execute",
                        "openUrl": url,
                        "close": True,
                    }
                )
            else:
                # Empty input - allow opening the link directly (without query)
                base_url = link.get("url", "").replace("{query}", "")
                emit(
                    {
                        "type": "results",
                        "inputMode": "submit",
                        "context": f"__search__:{link_name}",
                        "placeholder": search_placeholder,
                        "navigateForward": True,  # Entering search mode
                        "results": [
                            {
                                "id": f"__open_direct__:{link_name}",
                                "name": f"Open {link['name']}",
                                "description": base_url,
                                "icon": link.get("icon", "link"),
                                "verb": "Open",
                            },
                        ],
                    }
                )
            return query

        # Normal quicklink filtering (realtime mode)
        results = get_main_menu(quicklinks, query)
        emit(
            {
                "type": "results",
                "inputMode": "realtime",
                "results": results,
                "placeholder": "Search quicklinks...",
                "pluginActions": get_plugin_actions(),
            }
        )
        return query

    if step == "action":
        # Plugin-level action: add (from action bar)
        if selected_id == "__plugin__" and action == "add":
            show_add_form()
            return current_query

        # Form cancelled - return to list
        if selected_id == "__form_cancel__":
            emit(
                {
                    "type": "results",
                    "results": get_main_menu(quicklinks),
                    "inputMode": "realtime",
                    "clearInput": True,
                    "context": "",
                    "placeholder": "Search quicklinks...",
                    "pluginActions": get_plugin_actions(),
                }
            )
            return current_query

        # Back button - return to main list (core handles depth via pendingBack)
        if selected_id == "__back__":
            results = get_main_menu(quicklinks)
            emit(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                    "clearInput": True,
                    "context": "",
                    "placeholder": "Search quicklinks...",
                    "pluginActions": get_plugin_actions(),
                }
            )
            return current_query

        # Non-actionable items
        if selected_id in ("__error__", "__current_url__", "__empty__"):
            return current_query

        # Edit action - show edit form
        if action == "edit":
            link_name = selected_id
            link = next((l for l in quicklinks if l["name"] == link_name), None)
            if link:
                show_edit_form(link_name, link.get("url", ""), link.get("icon", "link"))
            return current_query

        # Delete action on a quicklink
        if action == "delete":
            link_name = selected_id
            quicklinks = [l for l in quicklinks if l["name"] != link_name]
            if save_quicklinks(quicklinks):
                emit(
                    {
                        "type": "results",
                        "results": get_main_menu(quicklinks),
                        "inputMode": "realtime",
                        "clearInput": True,
                        "placeholder": "Search quicklinks...",
                        "pluginActions": get_plugin_actions(),
                    }
                )
            else:
                emit({"type": "error", "message": "Failed to save quicklinks"})
            return ""

        # Legacy item click support for __add__
        if selected_id == "__add__":
            show_add_form()
            return current_query

        # Open link directly (without query parameter)
        if selected_id.startswith("__open_direct__:"):
            link_name = selected_id.split(":", 1)[1]
            link = next((l for l in quicklinks if l["name"] == link_name), None)
            if link:
                # Remove {query} placeholder for direct open
                url = link["url"].replace("{query}", "")
                emit(
                    {
                        "type": "execute",
                        "openUrl": url,
                        "close": True,
                    }
                )
            return current_query

        # Execute search (legacy support)
        if selected_id.startswith("__execute__:"):
            parts = selected_id.split(":", 2)
            link_name = parts[1]
            search_query = parts[2] if len(parts) > 2 else ""

            link = next((l for l in quicklinks if l["name"] == link_name), None)
            if link:
                url = link["url"].replace("{query}", urllib.parse.quote(search_query))
                emit(
                    {
                        "type": "execute",
                        "openUrl": url,
                        "close": True,
                    }
                )
            return current_query

        # Direct quicklink selection
        link = next((l for l in quicklinks if l["name"] == selected_id), None)
        if not link:
            emit({"type": "error", "message": f"Quicklink not found: {selected_id}"})
            return current_query

        url_template = link.get("url", "")

        # If URL has {query} placeholder, enter search mode (submit mode)
        # Core sets pendingNavigation for default clicks, so no need for navigateForward
        if "{query}" in url_template:
            base_url = url_template.replace("{query}", "")
            emit(
                {
                    "type": "results",
                    "inputMode": "submit",
                    "clearInput": True,
                    "context": f"__search__:{link['name']}",
                    "placeholder": f"Search {link['name']}... (Enter to search)",
                    "results": [
                        {
                            "id": f"__open_direct__:{link['name']}",
                            "name": f"Open {link['name']}",
                            "description": base_url,
                            "icon": link.get("icon", "link"),
                            "verb": "Open",
                        },
                    ],
                }
            )
            return current_query

        # No placeholder - just open the URL directly
        emit(
            {
                "type": "execute",
                "openUrl": url_template,
                "close": True,
            }
        )
        return current_query

    return current_query


TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"


def main():
    """Daemon mode event loop."""

    def shutdown_handler(signum, frame):
        sys.exit(0)

    signal.signal(signal.SIGTERM, shutdown_handler)
    signal.signal(signal.SIGINT, shutdown_handler)

    quicklinks = load_quicklinks()
    current_query = ""

    # Emit full index on startup (skip in test mode - tests use explicit index step)
    if not TEST_MODE:
        items = [quicklink_to_index_item(link) for link in quicklinks]
        emit({"type": "index", "mode": "full", "items": items})

    inotify_fd = create_inotify_fd()

    if inotify_fd is not None:
        quicklinks_filename = QUICKLINKS_PATH.name

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
                    current_query = handle_request(request, quicklinks, current_query)
                    # Reload quicklinks in case the request modified them
                    quicklinks = load_quicklinks()
                except (json.JSONDecodeError, ValueError):
                    continue

            if inotify_fd in readable:
                changed_files = read_inotify_events(inotify_fd)
                if quicklinks_filename in changed_files:
                    quicklinks = load_quicklinks()
                    # Emit updated index when file changes
                    items = [quicklink_to_index_item(link) for link in quicklinks]
                    emit({"type": "index", "items": items})
                    # Also emit results update for open plugin view
                    emit(
                        {
                            "type": "results",
                            "results": get_main_menu(quicklinks, current_query),
                            "inputMode": "realtime",
                            "placeholder": "Search quicklinks...",
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
                    current_query = handle_request(request, quicklinks, current_query)
                    # Reload quicklinks in case the request modified them
                    quicklinks = load_quicklinks()
                except (json.JSONDecodeError, ValueError):
                    continue

            now = time.time()
            if now - last_check >= check_interval:
                new_mtime = get_file_mtime()
                if new_mtime != last_mtime and new_mtime != 0:
                    last_mtime = new_mtime
                    quicklinks = load_quicklinks()
                    # Emit updated index when file changes
                    items = [quicklink_to_index_item(link) for link in quicklinks]
                    emit({"type": "index", "items": items})
                    # Also emit results update for open plugin view
                    emit(
                        {
                            "type": "results",
                            "results": get_main_menu(quicklinks, current_query),
                            "inputMode": "realtime",
                            "placeholder": "Search quicklinks...",
                            "pluginActions": get_plugin_actions(),
                        }
                    )
                last_check = now


if __name__ == "__main__":
    main()
