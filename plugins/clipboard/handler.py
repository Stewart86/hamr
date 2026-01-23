#!/usr/bin/env python3
"""
Clipboard plugin - browse and manage clipboard history via cliphist.
Socket-based daemon version for hamr.
Features: list, search, copy, delete, wipe
"""

import asyncio
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

# Add parent directory to path to import SDK
sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

# Cache directory for clipboard images (GTK handles thumbnail generation)
CACHE_DIR = (
    Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache"))
    / "hamr"
    / "clipboard-images"
)
PINNED_FILE = CACHE_DIR / "pinned.json"

# Cliphist database location
CLIPHIST_DB = (
    Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "cliphist" / "db"
)


def load_pinned_entries() -> list[str]:
    """Load pinned entry hashes from cache"""
    if not PINNED_FILE.exists():
        return []
    try:
        return json.loads(PINNED_FILE.read_text())
    except (json.JSONDecodeError, OSError):
        return []


def save_pinned_entries(pinned: list[str]) -> None:
    """Save pinned entry hashes to cache"""
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    PINNED_FILE.write_text(json.dumps(pinned))


def pin_entry(entry: str) -> None:
    """Pin an entry (by hash)"""
    entry_hash = get_entry_hash(entry)
    pinned = load_pinned_entries()
    if entry_hash not in pinned:
        pinned.insert(0, entry_hash)
        save_pinned_entries(pinned)


def unpin_entry(entry: str) -> None:
    """Unpin an entry (by hash)"""
    entry_hash = get_entry_hash(entry)
    pinned = load_pinned_entries()
    if entry_hash in pinned:
        pinned.remove(entry_hash)
        save_pinned_entries(pinned)


def is_pinned(entry: str) -> bool:
    """Check if entry is pinned"""
    return get_entry_hash(entry) in load_pinned_entries()


def get_clipboard_entries() -> list[str]:
    """Get clipboard entries from cliphist"""
    try:
        result = subprocess.run(
            ["cliphist", "list"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        if result.returncode == 0:
            return [line for line in result.stdout.strip().split("\n") if line]
    except (subprocess.TimeoutExpired, FileNotFoundError):
        pass
    return []


def clean_entry(entry: str) -> str:
    """Clean cliphist entry for display (remove ID prefix)"""
    # Entry format: "ID\tCONTENT"
    return re.sub(r"^\s*\S+\s+", "", entry)


def get_full_entry_content(entry: str) -> str:
    """Get the full content of a clipboard entry using cliphist decode."""
    try:
        proc = subprocess.run(
            f"printf '%s' '{shell_escape(entry)}' | cliphist decode",
            shell=True,
            capture_output=True,
            text=True,
            timeout=2,
        )
        if proc.returncode == 0:
            return proc.stdout
    except (subprocess.TimeoutExpired, Exception):
        pass
    # Fallback to cleaned entry (truncated)
    return clean_entry(entry)


def get_entry_id(entry: str) -> str:
    """Extract the cliphist ID from entry"""
    match = re.match(r"^\s*(\S+)\s+", entry)
    return match.group(1) if match else ""


def is_image(entry: str) -> bool:
    """Check if entry is an image"""
    return bool(re.match(r"^\d+\t\[\[.*binary data.*\d+x\d+.*\]\]$", entry))


def get_image_dimensions(entry: str) -> tuple[int, int] | None:
    """Extract image dimensions from entry"""
    match = re.search(r"(\d+)x(\d+)", entry)
    if match:
        return int(match.group(1)), int(match.group(2))
    return None


def get_entry_hash(entry: str) -> str:
    """Get a stable hash for a clipboard entry based on content only."""
    content = clean_entry(entry)
    return hashlib.md5(content.encode()).hexdigest()[:16]


def get_cached_image_path(entry: str) -> str | None:
    """Get cached image path for clipboard entry.

    Saves the raw image to cache if not present. GTK handles thumbnail generation.
    """
    if not is_image(entry):
        return None

    entry_hash = hashlib.md5(entry.encode()).hexdigest()[:16]
    image_path = CACHE_DIR / f"{entry_hash}.png"

    if image_path.exists():
        return str(image_path)

    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    try:
        decode_proc = subprocess.run(
            ["cliphist", "decode"],
            input=entry.encode("utf-8"),
            capture_output=True,
            timeout=5,
        )
        if decode_proc.returncode == 0 and decode_proc.stdout:
            image_path.write_bytes(decode_proc.stdout)
            if image_path.exists():
                return str(image_path)
    except (subprocess.TimeoutExpired, Exception):
        pass

    return None


def copy_entry(entry: str) -> None:
    """Copy entry to clipboard"""
    subprocess.Popen(
        f"printf '%s' '{shell_escape(entry)}' | cliphist decode | wl-copy",
        shell=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def delete_entry(entry: str) -> None:
    """Delete entry from clipboard history"""
    subprocess.Popen(
        f"printf '%s' '{shell_escape(entry)}' | cliphist delete",
        shell=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def wipe_clipboard() -> None:
    """Wipe entire clipboard history"""
    subprocess.Popen(
        ["cliphist", "wipe"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    # Clear cached images
    if CACHE_DIR.exists():
        for f in CACHE_DIR.iterdir():
            f.unlink()


def shell_escape(s: str) -> str:
    """Escape string for single-quoted shell argument"""
    return s.replace("'", "'\\''")


def fuzzy_match(query: str, text: str) -> bool:
    """Simple fuzzy match - all query chars appear in order"""
    query = query.lower()
    text = text.lower()
    qi = 0
    for char in text:
        if qi < len(query) and char == query[qi]:
            qi += 1
    return qi == len(query)


def detect_content_type(content: str) -> str | None:
    """Detect content type from text content"""
    content = content.strip()
    if not content:
        return None

    # URL detection
    if content.startswith(("http://", "https://", "www.")):
        return "url"

    # Email detection
    if "@" in content and "." in content and " " not in content:
        if re.match(r"^[^@\s]+@[^@\s]+\.[^@\s]+$", content):
            return "email"

    # Path detection
    if content.startswith(("/", "~/")):
        return "path"

    # JSON detection
    if (content.startswith("{") and content.endswith("}")) or (
        content.startswith("[") and content.endswith("]")
    ):
        try:
            json.loads(content)
            return "json"
        except json.JSONDecodeError:
            pass

    # Code detection (simple heuristics)
    code_indicators = [
        "def ",
        "function ",
        "const ",
        "let ",
        "var ",
        "import ",
        "class ",
    ]
    if any(content.startswith(ind) for ind in code_indicators):
        return "code"

    return None


def get_content_chips(content: str, is_img: bool) -> list[dict]:
    """Get chips for clipboard entry based on content type"""
    chips = []

    if is_img:
        dims = get_image_dimensions(content)
        if dims:
            chips.append({"text": f"{dims[0]}x{dims[1]}", "icon": "image"})
    else:
        content_type = detect_content_type(content)
        if content_type == "url":
            chips.append({"text": "URL", "icon": "link"})
        elif content_type == "email":
            chips.append({"text": "Email", "icon": "email"})
        elif content_type == "path":
            chips.append({"text": "Path", "icon": "folder"})
        elif content_type == "json":
            chips.append({"text": "JSON", "icon": "data_object"})
        elif content_type == "code":
            chips.append({"text": "Code", "icon": "code"})

        # Add length indicator for long content
        if len(content) > 200:
            chips.append({"text": "Long", "icon": "notes"})

    return chips


def format_entry_age(index: int) -> str:
    """Format entry age based on position in list."""
    if index == 0:
        return "Just now"
    elif index < 3:
        return "Moments ago"
    elif index < 10:
        return "Recent"
    elif index < 25:
        return "Earlier"
    else:
        return "Older"


def get_entry_results(
    entries: list[str], query: str = "", filter_type: str = "", limit: int = 20
) -> list[dict]:
    """Convert clipboard entries to result format"""
    results = []
    pinned_hashes = set(load_pinned_entries())

    # Sort entries: pinned first, then by original order
    def sort_key(entry: str) -> tuple[int, int]:
        entry_hash = get_entry_hash(entry)
        is_pin = entry_hash in pinned_hashes
        return (0 if is_pin else 1, entries.index(entry))

    sorted_entries = sorted(entries, key=sort_key)
    entry_index = 0

    for entry in sorted_entries:
        if len(results) >= limit:
            break

        # Apply type filter
        is_img = is_image(entry)
        if filter_type == "images" and not is_img:
            continue
        if filter_type == "text" and is_img:
            continue

        # Apply search query
        if query:
            content_match = fuzzy_match(query, clean_entry(entry))
            if not content_match:
                continue

        display = clean_entry(entry)
        age_label = format_entry_age(entry_index)
        entry_index += 1

        # For images, show dimensions
        if is_image(entry):
            dims = get_image_dimensions(entry)
            display = f"Image {dims[0]}x{dims[1]}" if dims else "Image"
            entry_type = f"{age_label} · Image"
            icon = "image"
        else:
            # Truncate long text entries
            if len(display) > 100:
                display = display[:100] + "..."
            entry_type = f"{age_label} · Text"
            icon = "content_paste"

        entry_is_pinned = get_entry_hash(entry) in pinned_hashes
        pin_action = (
            {"id": "unpin", "name": "Unpin", "icon": "push_pin"}
            if entry_is_pinned
            else {"id": "pin", "name": "Pin", "icon": "push_pin"}
        )

        item_id = f"clip:{get_entry_hash(entry)}"
        result = {
            "id": item_id,
            "_entry": entry,  # Keep raw entry for action handling
            "name": display,
            "icon": icon,
            "description": ("Pinned · " if entry_is_pinned else "") + entry_type,
            "verb": "Copy",
            "actions": [
                pin_action,
                {"id": "delete", "name": "Delete", "icon": "delete"},
            ],
        }

        chips = get_content_chips(display, is_image(entry))
        if chips:
            result["chips"] = chips

        # Add thumbnail and preview panel data
        if is_image(entry):
            image_path = get_cached_image_path(entry)
            dims = get_image_dimensions(entry)

            # Thumbnail for result list (GTK handles resizing)
            if image_path:
                result["thumbnail"] = image_path

            # Preview panel data
            preview_metadata = []
            if dims:
                preview_metadata.append(
                    {"label": "Size", "value": f"{dims[0]}x{dims[1]}"}
                )

            result["preview"] = {
                "title": display,
                "image": image_path,
                "metadata": preview_metadata,
                "actions": [
                    {"id": "copy", "name": "Copy", "icon": "content_copy"},
                ],
            }
        else:
            # Text preview
            full_content = get_full_entry_content(entry)
            char_count = len(full_content)
            line_count = full_content.count("\n") + 1

            result["preview"] = {
                "content": full_content,
                "title": "Text Clip",
                "metadata": [
                    {"label": "Characters", "value": str(char_count)},
                    {"label": "Lines", "value": str(line_count)},
                ],
                "actions": [
                    {"id": "copy", "name": "Copy", "icon": "content_copy"},
                ],
            }

        results.append(result)

    if not results:
        results.append(
            {
                "id": "__empty__",
                "name": "No clipboard entries",
                "icon": "info",
                "description": "Copy something to see it here",
            }
        )

    return results


def get_plugin_actions(active_filter: str = "") -> list[dict]:
    """Get plugin-level actions for the action bar"""
    return [
        {
            "id": "filter_images",
            "name": "Images",
            "icon": "image",
            "shortcut": "Ctrl+1",
            "active": active_filter == "images",
        },
        {
            "id": "filter_text",
            "name": "Text",
            "icon": "text_fields",
            "shortcut": "Ctrl+2",
            "active": active_filter == "text",
        },
        {
            "id": "wipe",
            "name": "Wipe All",
            "icon": "delete_sweep",
            "confirm": "Wipe all clipboard history? This cannot be undone.",
            "shortcut": "Ctrl+3",
        },
    ]


def get_status() -> dict:
    """Get current clipboard status for badge display."""
    try:
        entries = get_clipboard_entries()
        count = len(entries)

        badges = []
        if count > 0:
            badges.append({"text": str(count)})

        return {"badges": badges}
    except Exception:
        return {}


# Create plugin instance
plugin = HamrPlugin(
    id="clipboard",
    name="Clipboard",
    description="Browse and manage clipboard history",
    icon="content_paste",
)

# Plugin state
state = {
    "entries": [],
    "current_query": "",
    "current_filter": "",
    "plugin_active": False,
}


@plugin.on_initial
async def handle_initial(params=None):
    """Handle initial request when plugin is opened."""
    state["entries"] = get_clipboard_entries()
    state["plugin_active"] = True
    state["current_filter"] = ""
    results = get_entry_results(state["entries"])
    return HamrPlugin.results(
        results,
        plugin_actions=get_plugin_actions(),
        status=get_status(),
        placeholder="Search clipboard...",
    )


@plugin.on_search
async def handle_search(query: str, context=None):
    """Handle search request."""
    state["current_query"] = query
    state["entries"] = get_clipboard_entries()
    current_filter = state["current_filter"]
    results = get_entry_results(state["entries"], query, current_filter)
    return HamrPlugin.results(
        results,
        plugin_actions=get_plugin_actions(current_filter),
        status=get_status(),
        placeholder="Search clipboard...",
    )


@plugin.on_action
async def handle_action(item_id: str, action=None, context=None):
    """Handle action request."""
    entries = state["entries"]
    current_filter = state["current_filter"]

    # Plugin-level actions
    if item_id == "__plugin__":
        if action == "filter_images":
            new_filter = "" if current_filter == "images" else "images"
            state["current_filter"] = new_filter
            results = get_entry_results(
                state["entries"], state["current_query"], new_filter
            )
            await plugin.send_results(
                results,
                plugin_actions=get_plugin_actions(new_filter),
                status=get_status(),
                placeholder="Search clipboard...",
            )
            return HamrPlugin.noop()

        if action == "filter_text":
            new_filter = "" if current_filter == "text" else "text"
            state["current_filter"] = new_filter
            results = get_entry_results(
                state["entries"], state["current_query"], new_filter
            )
            await plugin.send_results(
                results,
                plugin_actions=get_plugin_actions(new_filter),
                status=get_status(),
                placeholder="Search clipboard...",
            )
            return HamrPlugin.noop()

        if action == "wipe":
            wipe_clipboard()
            await plugin.send_execute(
                {
                    "type": "notify",
                    "message": "Clipboard history cleared",
                }
            )
            return HamrPlugin.close()
        return HamrPlugin.noop()

    # Handle gridBrowser selection or regular item
    if item_id == "gridBrowser":
        entry = context if context else ""
    else:
        # Get raw entry from _entry field or look up by hash
        entry = ""
        if item_id.startswith("clip:"):
            target_hash = item_id[5:]
            entry = next((e for e in entries if get_entry_hash(e) == target_hash), "")
        else:
            entry = item_id

    if not entry:
        return HamrPlugin.noop()

    # Clipboard entry actions
    if action == "delete":
        delete_entry(entry)
        entries = [e for e in entries if e != entry]
        state["entries"] = entries
        results = get_entry_results(entries, state["current_query"], current_filter)
        await plugin.send_results(
            results,
            plugin_actions=get_plugin_actions(current_filter),
            status=get_status(),
            placeholder="Search clipboard...",
        )
        return HamrPlugin.noop()

    if action == "pin":
        pin_entry(entry)
        results = get_entry_results(
            state["entries"], state["current_query"], current_filter
        )
        await plugin.send_results(
            results,
            plugin_actions=get_plugin_actions(current_filter),
            status=get_status(),
            placeholder="Search clipboard...",
        )
        return HamrPlugin.noop()

    if action == "unpin":
        unpin_entry(entry)
        results = get_entry_results(
            state["entries"], state["current_query"], current_filter
        )
        await plugin.send_results(
            results,
            plugin_actions=get_plugin_actions(current_filter),
            status=get_status(),
            placeholder="Search clipboard...",
        )
        return HamrPlugin.noop()

    # Default action (click) or explicit copy
    if action == "copy" or not action:
        copy_entry(entry)
        await plugin.send_execute(
            {
                "type": "notify",
                "message": "Copied to clipboard",
            }
        )
        return HamrPlugin.close()

    return HamrPlugin.noop()


@plugin.add_background_task
async def emit_status_updates(p: HamrPlugin):
    """Background task to emit status updates and index changes."""
    last_db_mtime = get_db_mtime() if CLIPHIST_DB.exists() else 0
    indexed_ids: set[str] = set()

    # Emit initial status and index
    await p.send_status(get_status())

    # Emit initial index
    entries = get_clipboard_entries()
    items = []
    for entry in entries[:100]:
        item_id = f"clip:{get_entry_hash(entry)}"
        indexed_ids.add(item_id)
        display = clean_entry(entry)
        if len(display) > 80:
            display = display[:80] + "..."

        icon = "image" if is_image(entry) else "content_paste"
        item = {
            "id": item_id,
            "name": display,
            "icon": icon,
            "description": "Text" if not is_image(entry) else "Image",
            "keywords": display.lower().split()[:10],
            "verb": "Copy",
            "actions": [
                {"id": "delete", "name": "Delete", "icon": "delete"},
            ],
        }
        items.append(item)

    await p.send_index(items)

    # Watch for clipboard changes
    check_interval = 1.0
    last_check = time.time()

    while True:
        await asyncio.sleep(0.5)

        now = time.time()
        if now - last_check >= check_interval:
            last_check = now

            if CLIPHIST_DB.exists():
                current_mtime = get_db_mtime()
                if current_mtime != last_db_mtime:
                    last_db_mtime = current_mtime

                    # Update status
                    await p.send_status(get_status())

                    # Update index if plugin is active
                    if state["plugin_active"]:
                        state["entries"] = get_clipboard_entries()
                        current_filter = state["current_filter"]
                        results = get_entry_results(
                            state["entries"], state["current_query"], current_filter
                        )
                        await p.send_results(
                            results,
                            pluginActions=get_plugin_actions(current_filter),
                            status=get_status(),
                            placeholder="Search clipboard...",
                        )


def get_db_mtime() -> float:
    """Get modification time of cliphist database."""
    try:
        return CLIPHIST_DB.stat().st_mtime
    except (FileNotFoundError, OSError):
        return 0


if __name__ == "__main__":
    plugin.run()
