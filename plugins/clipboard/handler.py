#!/usr/bin/env python3
"""
Clipboard workflow handler - browse and manage clipboard history via cliphist.
Features: list, search, copy, delete, wipe, image thumbnails, OCR search
"""

import hashlib
import json
import os
import re
import select
import signal
import subprocess
import sys
import time
from pathlib import Path

TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

# Cache directory for image thumbnails and OCR
CACHE_DIR = (
    Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache"))
    / "hamr"
    / "clipboard-thumbs"
)
OCR_CACHE_FILE = CACHE_DIR / "ocr-index.json"
PINNED_FILE = CACHE_DIR / "pinned.json"
SCRIPT_DIR = Path(__file__).parent
# Max thumbnail size (width or height)
MAX_THUMB_SIZE = 256

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
    """Get the full content of a clipboard entry using cliphist decode.

    cliphist list truncates long entries, so we need to decode for full content.
    """
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
    """Get a stable hash for a clipboard entry based on content only.

    The entry format is "ID\\tCONTENT" - we hash only the content part
    so the hash stays stable when the same content moves positions in cliphist.
    """
    content = clean_entry(entry)
    return hashlib.md5(content.encode()).hexdigest()[:16]


def load_ocr_cache() -> dict[str, str]:
    """Load OCR cache from disk"""
    if OCR_CACHE_FILE.exists():
        try:
            return json.loads(OCR_CACHE_FILE.read_text())
        except (json.JSONDecodeError, IOError):
            pass
    return {}


def save_ocr_cache(cache: dict[str, str]) -> None:
    """Save OCR cache to disk"""
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    OCR_CACHE_FILE.write_text(json.dumps(cache))


def spawn_ocr_indexer():
    """Spawn background OCR indexer process (non-blocking)"""
    indexer_script = SCRIPT_DIR / "ocr-indexer.py"
    if indexer_script.exists():
        # Preserve DBUS for notifications in detached process
        env = os.environ.copy()
        subprocess.Popen(
            [sys.executable, str(indexer_script)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
            env=env,
        )


def get_ocr_text_for_entries(
    entries: list[str], ocr_cache: dict[str, str]
) -> dict[str, str]:
    """Get OCR text for all image entries from cache only (no blocking OCR)"""
    result = {}
    for entry in entries:
        if is_image(entry):
            entry_hash = get_entry_hash(entry)
            if entry_hash in ocr_cache:
                result[entry] = ocr_cache[entry_hash]
    return result


def get_image_thumbnail(entry: str) -> str | None:
    """Get cached thumbnail for image entry, return path or None.

    Does NOT generate thumbnails - that's done by the background indexer.
    """
    if not is_image(entry):
        return None

    entry_hash = hashlib.md5(entry.encode()).hexdigest()[:16]
    thumb_path = CACHE_DIR / f"{entry_hash}.png"

    # Only return if cached - don't block on generation
    if thumb_path.exists():
        return str(thumb_path)

    return None


def copy_entry(entry: str):
    """Copy entry to clipboard"""
    # Pipe entry to cliphist decode, then to wl-copy
    subprocess.Popen(
        f"printf '%s' '{shell_escape(entry)}' | cliphist decode | wl-copy",
        shell=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def delete_entry(entry: str):
    """Delete entry from clipboard history"""
    subprocess.Popen(
        f"printf '%s' '{shell_escape(entry)}' | cliphist delete",
        shell=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    # Also remove thumbnail if exists
    entry_hash = hashlib.md5(entry.encode()).hexdigest()[:16]
    thumb_path = CACHE_DIR / f"{entry_hash}.png"
    if thumb_path.exists():
        thumb_path.unlink()


def wipe_clipboard():
    """Wipe entire clipboard history"""
    subprocess.Popen(
        ["cliphist", "wipe"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    # Clear thumbnail cache
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


def get_content_chips(
    content: str, is_img: bool, ocr_text: str = "", dims: tuple[int, int] | None = None
) -> list[dict]:
    """Get chips for clipboard entry based on content type"""
    chips = []

    if is_img:
        if dims:
            chips.append({"text": f"{dims[0]}x{dims[1]}", "icon": "image"})
        if ocr_text:
            chips.append({"text": "OCR", "icon": "text_fields"})
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
    """Format entry age based on position in list.

    Since cliphist doesn't provide timestamps, we use position as a proxy.
    Position 0-2: very recent, 3-9: recent, 10-24: earlier, 25+: older
    """
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


def get_incremental_results(
    entries: list[str],
    offset: int = 0,
    limit: int = 20,
    query: str = "",
    filter_type: str = "",
    ocr_texts: dict[str, str] | None = None,
) -> list[dict]:
    """Get paginated results with offset for incremental loading.

    Returns 'limit' items starting from 'offset', with a hint if more items available.
    """
    results = get_entry_results(
        entries, query, filter_type, ocr_texts, limit=offset + limit
    )

    # Slice results to get the requested page
    paged_results = results[offset : offset + limit]

    # Add a load-more hint if more items available
    if len(results) > offset + limit:
        paged_results.append(
            {
                "id": "__load_more__",
                "name": f"Load more ({len(results) - (offset + limit)} more)",
                "icon": "expand_more",
                "description": "Load additional clipboard entries",
            }
        )

    return paged_results


def get_entry_results(
    entries: list[str],
    query: str = "",
    filter_type: str = "",
    ocr_texts: dict[str, str] | None = None,
    limit: int = 20,
) -> list[dict]:
    """Convert clipboard entries to result format"""
    results = []
    ocr_texts = ocr_texts or {}
    pinned_hashes = set(load_pinned_entries())

    # Sort entries: pinned first, then by original order
    def sort_key(entry: str) -> tuple[int, int]:
        entry_hash = get_entry_hash(entry)
        is_pin = entry_hash in pinned_hashes
        # Pinned items first (0), then regular items (1)
        # Within each group, maintain original order
        return (0 if is_pin else 1, entries.index(entry))

    sorted_entries = sorted(entries, key=sort_key)
    entry_index = 0

    for entry in sorted_entries:
        # Stop once we have enough results
        if len(results) >= limit:
            break
        # Apply type filter
        is_img = is_image(entry)
        if filter_type == "images" and not is_img:
            continue
        if filter_type == "text" and is_img:
            continue

        # Apply search query (check both content and OCR text for images)
        if query:
            content_match = fuzzy_match(query, clean_entry(entry))
            ocr_text = ocr_texts.get(entry, "")
            ocr_match = is_img and ocr_text and fuzzy_match(query, ocr_text)
            if not content_match and not ocr_match:
                continue

        display = clean_entry(entry)
        age_label = format_entry_age(entry_index)
        entry_index += 1

        # For images, show dimensions and OCR preview if available
        if is_img:
            dims = get_image_dimensions(entry)
            display = f"Image {dims[0]}x{dims[1]}" if dims else "Image"
            ocr_text = ocr_texts.get(entry, "")
            if ocr_text:
                # Show OCR text preview in description
                ocr_preview = ocr_text.replace("\n", " ")[:60]
                if len(ocr_text) > 60:
                    ocr_preview += "..."
                entry_type = f"{age_label} · {ocr_preview}"
            else:
                entry_type = f"{age_label} · Image"
            icon = "image"
            thumbnail = get_image_thumbnail(entry)
        else:
            # Truncate long text entries
            if len(display) > 100:
                display = display[:100] + "..."
            entry_type = f"{age_label} · Text"
            icon = "content_paste"
            thumbnail = None

        entry_is_pinned = get_entry_hash(entry) in pinned_hashes
        pin_action = (
            {"id": "unpin", "name": "Unpin", "icon": "push_pin"}
            if entry_is_pinned
            else {"id": "pin", "name": "Pin", "icon": "push_pin"}
        )

        img_dims = get_image_dimensions(entry) if is_img else None
        chips = get_content_chips(display, is_img, ocr_texts.get(entry, ""), img_dims)

        # Use clip:{hash} format to match index IDs for frecency tracking
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

        if chips:
            result["chips"] = chips

        if thumbnail:
            result["thumbnail"] = thumbnail

        # Add preview panel data
        if is_img and thumbnail:
            preview_metadata = []
            img_dims = get_image_dimensions(entry)
            if img_dims:
                preview_metadata.append(
                    {"label": "Size", "value": f"{img_dims[0]}x{img_dims[1]}"}
                )
            ocr_text = ocr_texts.get(entry, "")
            if ocr_text:
                preview_metadata.append(
                    {
                        "label": "OCR",
                        "value": ocr_text[:200]
                        + ("..." if len(ocr_text) > 200 else ""),
                    }
                )

            result["preview"] = {
                "type": "image",
                "content": thumbnail,
                "title": display,
                "metadata": preview_metadata,
                "actions": [
                    {"id": "copy", "name": "Copy", "icon": "content_copy"},
                ],
            }
        elif not is_img:
            # Text preview - show full content (use cliphist decode for untruncated content)
            full_content = get_full_entry_content(entry)
            char_count = len(full_content)
            line_count = full_content.count("\n") + 1

            result["preview"] = {
                "type": "text",
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


def get_image_grid_items(
    entries: list[str],
    ocr_texts: dict[str, str],
    offset: int = 0,
    limit: int = 200,
) -> list[dict]:
    """Convert image entries to grid items for gridBrowser display.

    Shows image dimensions as name, thumbnail as image, and OCR text as keywords.
    Limits to 'limit' items starting from 'offset' for performance.
    """
    items = []
    count = 0
    for entry in entries:
        if not is_image(entry):
            continue

        if count < offset:
            count += 1
            continue

        if len(items) >= limit:
            break

        thumbnail = get_image_thumbnail(entry)
        if not thumbnail:
            continue  # Skip images without thumbnails

        dims = get_image_dimensions(entry)
        ocr_text = ocr_texts.get(entry, "")
        items.append(
            {
                "id": entry,  # Use full entry as ID for action handling
                "name": f"{dims[0]}x{dims[1]}" if dims else "Image",
                "icon": thumbnail,
                "iconType": "image",
                "keywords": ocr_text.lower().split()[:10] if ocr_text else [],
            }
        )
        count += 1

    return items


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
        image_count = sum(1 for e in entries if is_image(e))

        badges = []
        if count > 0:
            badges.append({"text": str(count)})

        chips = []
        if image_count > 0:
            chips.append({"text": f"{image_count} images", "icon": "image"})

        return {"badges": badges, "chips": chips}
    except Exception:
        return {}


def emit_status() -> None:
    """Emit status update for background daemon."""
    print(json.dumps({"type": "status", "status": get_status()}), flush=True)


def emit_incremental_index(last_indexed_ids: set[str]) -> set[str]:
    """Emit incremental index update with new/removed items.

    Returns the updated set of indexed IDs.
    """
    entries = get_clipboard_entries()
    ocr_cache = load_ocr_cache()
    ocr_texts = get_ocr_text_for_entries(entries, ocr_cache)

    # Get current entries (limit to recent 100 for index)
    current_entries = entries[:100]
    current_ids = {f"clip:{get_entry_hash(e)}" for e in current_entries}

    # Find new items (in current but not previously indexed)
    new_ids = current_ids - last_indexed_ids
    new_items = [
        entry_to_index_item(e, ocr_texts)
        for e in current_entries
        if f"clip:{get_entry_hash(e)}" in new_ids
    ]

    # Find removed items (previously indexed but no longer in current)
    removed_ids = list(last_indexed_ids - current_ids)

    if new_items or removed_ids:
        print(
            json.dumps(
                {
                    "type": "index",
                    "mode": "incremental",
                    "items": new_items,
                    "remove": removed_ids,
                }
            ),
            flush=True,
        )

    return current_ids


def get_db_mtime() -> float:
    """Get modification time of cliphist database."""
    try:
        return CLIPHIST_DB.stat().st_mtime
    except (FileNotFoundError, OSError):
        return 0


def respond(results: list[dict], **kwargs):
    """Send a results response"""
    active_filter = kwargs.get("active_filter", "")
    response = {
        "type": "results",
        "results": results,
        "inputMode": "realtime",
        "placeholder": kwargs.get("placeholder", "Search clipboard..."),
        "pluginActions": get_plugin_actions(active_filter),
        "status": get_status(),
    }
    if active_filter:
        response["context"] = active_filter
    if kwargs.get("clear_input"):
        response["clearInput"] = True
    if kwargs.get("navigate_forward") is False:
        response["navigateForward"] = False
    print(json.dumps(response), flush=True)


def entry_to_index_item(entry: str, ocr_texts: dict[str, str]) -> dict:
    """Convert a clipboard entry to indexable item format for main search."""
    display = clean_entry(entry)
    is_img = is_image(entry)

    if is_img:
        dims = get_image_dimensions(entry)
        name = f"Image {dims[0]}x{dims[1]}" if dims else "Image"
        ocr_text = ocr_texts.get(entry, "")
        keywords = ocr_text.lower().split()[:20] if ocr_text else []  # First 20 words
        icon = "image"
        thumbnail = get_image_thumbnail(entry)
        description = (
            ocr_text[:60] + "..."
            if ocr_text and len(ocr_text) > 60
            else (ocr_text or "Image")
        )
    else:
        # Truncate long text entries
        name = display[:80] + "..." if len(display) > 80 else display
        # Use first few words as keywords for searchability
        keywords = display.lower().split()[:10]
        icon = "content_paste"
        thumbnail = None
        description = "Text"

    entry_id = get_entry_id(entry)

    img_dims = get_image_dimensions(entry) if is_img else None
    chips = get_content_chips(display, is_img, ocr_texts.get(entry, ""), img_dims)

    entry_hash = get_entry_hash(entry)
    item = {
        "id": f"clip:{entry_hash}",
        "name": name,
        "description": description,
        "keywords": keywords,
        "icon": icon,
        "verb": "Copy",
        "actions": [
            {
                "id": "delete",
                "name": "Delete",
                "icon": "delete",
                "entryPoint": {
                    "step": "action",
                    "selected": {"id": f"clip:{entry_hash}"},
                    "action": "delete",
                },
            },
        ],
        "entryPoint": {
            "step": "action",
            "selected": {"id": f"clip:{entry_hash}"},
        },
    }

    if chips:
        item["chips"] = chips

    if thumbnail:
        item["thumbnail"] = thumbnail

    return item


def handle_request(input_data: dict) -> None:
    """Handle a single request from the launcher."""
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")
    context = input_data.get("context", "")  # Active filter: "", "images", or "text"

    entries = get_clipboard_entries()

    # Load OCR cache for image text search
    ocr_cache = load_ocr_cache()
    ocr_texts = get_ocr_text_for_entries(entries, ocr_cache)

    if step == "index":
        mode = input_data.get("mode", "full")
        indexed_ids = set(input_data.get("indexedIds", []))

        # Build current ID set from entries (limit to 100 for faster initial load)
        current_entries = entries[:100]
        current_ids = {f"clip:{get_entry_hash(e)}" for e in current_entries}

        if mode == "incremental" and indexed_ids:
            # Find new items (in current but not indexed)
            new_ids = current_ids - indexed_ids
            new_items = [
                entry_to_index_item(e, ocr_texts)
                for e in current_entries
                if f"clip:{get_entry_hash(e)}" in new_ids
            ]

            # Find removed items (in indexed but not current)
            removed_ids = list(indexed_ids - current_ids)

            print(
                json.dumps(
                    {
                        "type": "index",
                        "mode": "incremental",
                        "items": new_items,
                        "remove": removed_ids,
                    }
                ),
                flush=True,
            )
        else:
            # Full reindex
            items = [entry_to_index_item(e, ocr_texts) for e in current_entries]
            print(json.dumps({"type": "index", "items": items}), flush=True)
        return

    if step == "initial":
        spawn_ocr_indexer()
        respond(get_entry_results(entries, ocr_texts=ocr_texts))
        return

    if step == "search":
        offset = input_data.get("offset", 0)
        respond(
            get_incremental_results(entries, offset, 20, query, context, ocr_texts),
            active_filter=context,
        )
        return

    # Action: handle clicks
    if step == "action":
        item_id = selected.get("id", "")

        # Plugin-level actions (from action bar)
        if item_id == "__plugin__":
            # Filter by images - show grid or toggle off
            if action == "filter_images":
                if context == "images":
                    # Toggle off images filter - return to all entries
                    respond(
                        get_incremental_results(entries, 0, 20, query, "", ocr_texts),
                        active_filter="",
                        navigate_forward=False,
                    )
                else:
                    # Show images in gridBrowser (first 200 with thumbnails)
                    image_entries = [e for e in entries if is_image(e)]
                    grid_items = get_image_grid_items(image_entries, ocr_texts, 0, 200)
                    total_images = sum(
                        1 for e in image_entries if get_image_thumbnail(e)
                    )
                    if grid_items:
                        print(
                            json.dumps(
                                {
                                    "type": "gridBrowser",
                                    "gridBrowser": {
                                        "title": f"Clipboard Images ({total_images})",
                                        "items": grid_items,
                                        "columns": 8,
                                        "cellAspectRatio": 1.0,
                                        "actions": [
                                            {
                                                "id": "copy",
                                                "name": "Copy",
                                                "icon": "content_copy",
                                            },
                                            {
                                                "id": "delete",
                                                "name": "Delete",
                                                "icon": "delete",
                                            },
                                        ],
                                    },
                                }
                            ),
                            flush=True,
                        )
                    else:
                        respond(
                            [
                                {
                                    "id": "__empty__",
                                    "name": "No images in clipboard",
                                    "icon": "info",
                                    "description": "Copy an image to see it here",
                                }
                            ],
                            active_filter="images",
                        )
                return

            # Filter by text - toggle (view modification, not navigation)
            if action == "filter_text":
                new_filter = "" if context == "text" else "text"
                respond(
                    get_incremental_results(
                        entries, 0, 20, query, new_filter, ocr_texts
                    ),
                    active_filter=new_filter,
                    navigate_forward=False,
                )
                return

            # Wipe all
            if action == "wipe":
                wipe_clipboard()
                print(
                    json.dumps(
                        {
                            "type": "execute",
                            "notify": "Clipboard history cleared",
                            "close": True,
                        }
                    ),
                    flush=True,
                )
                return

        # Back action - clear filter and go back to unfiltered list
        if item_id == "__back__":
            respond(
                get_incremental_results(entries, 0, 20, query, "", ocr_texts),
                active_filter="",
            )
            return

        if item_id == "__empty__":
            respond(
                get_incremental_results(entries, 0, 20, query, context, ocr_texts),
                active_filter=context,
            )
            return

        # Handle gridBrowser selection (from image grid)
        if item_id == "gridBrowser":
            entry = selected.get("itemId", "")
            action_id = selected.get("action", "") or action or "copy"
        else:
            # Get raw entry from _entry field, or look up by hash if clip: prefixed ID
            entry = selected.get("_entry", "")
            if not entry and item_id.startswith("clip:"):
                # Look up entry by hash from current entries
                target_hash = item_id[5:]  # Remove "clip:" prefix
                entry = next(
                    (e for e in entries if get_entry_hash(e) == target_hash), ""
                )
            elif not entry:
                entry = item_id  # Fallback for legacy IDs
            action_id = action

        # Clipboard entry actions

        if action_id == "delete":
            delete_entry(entry)
            # Refresh entries after delete
            entries = [e for e in entries if e != entry]
            # Also remove from OCR cache
            entry_hash = get_entry_hash(entry)
            if entry_hash in ocr_cache:
                del ocr_cache[entry_hash]
                save_ocr_cache(ocr_cache)
            ocr_texts = {k: v for k, v in ocr_texts.items() if k != entry}
            respond(
                get_incremental_results(entries, 0, 20, query, context, ocr_texts),
                active_filter=context,
            )
            return

        if action_id == "pin":
            pin_entry(entry)
            respond(
                get_incremental_results(entries, 0, 20, query, context, ocr_texts),
                active_filter=context,
                navigate_forward=False,
            )
            return

        if action_id == "unpin":
            unpin_entry(entry)
            respond(
                get_incremental_results(entries, 0, 20, query, context, ocr_texts),
                active_filter=context,
                navigate_forward=False,
            )
            return

        # Default action (click) or explicit copy
        if action_id == "copy" or not action_id:
            copy_entry(entry)
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "notify": "Copied to clipboard",
                        "close": True,
                    }
                ),
                flush=True,
            )
            return

    # Unknown
    print(json.dumps({"type": "error", "message": f"Unknown step: {step}"}), flush=True)


def main():
    """Run clipboard handler in daemon mode with file watching."""
    signal.signal(signal.SIGTERM, lambda s, f: sys.exit(0))
    signal.signal(signal.SIGINT, lambda s, f: sys.exit(0))

    # Track indexed item IDs for incremental updates
    indexed_ids: set[str] = set()

    # Emit initial status and full index on startup (for background daemon)
    if not TEST_MODE:
        emit_status()
        # Build and emit full initial index
        entries = get_clipboard_entries()
        ocr_cache = load_ocr_cache()
        ocr_texts = get_ocr_text_for_entries(entries, ocr_cache)
        initial_entries = entries[:100]
        indexed_ids = {f"clip:{get_entry_hash(e)}" for e in initial_entries}
        initial_items = [entry_to_index_item(e, ocr_texts) for e in initial_entries]
        print(
            json.dumps({"type": "index", "mode": "full", "items": initial_items}),
            flush=True,
        )

    # Track state for refreshing results when clipboard changes
    last_db_mtime = get_db_mtime()
    last_check = time.time()
    check_interval = 1.0  # Check for clipboard changes every 1 second

    # Track if plugin is active and current view state
    plugin_active = False
    current_query = ""
    current_context = ""  # Active filter: "", "images", "text"

    while True:
        readable, _, _ = select.select([sys.stdin], [], [], 0.5)

        if readable:
            try:
                line = sys.stdin.readline()
                if not line:
                    break
                request = json.loads(line.strip())
                step = request.get("step", "")

                # Track plugin state
                if step == "initial":
                    plugin_active = True
                    current_query = ""
                    current_context = ""
                elif step == "search":
                    current_query = request.get("query", "").strip()
                    current_context = request.get("context", "")
                elif step == "action":
                    # Update context from action responses
                    current_context = request.get("context", current_context)

                handle_request(request)
                # Update mtime after handling request (in case we modified clipboard)
                last_db_mtime = get_db_mtime()
            except (json.JSONDecodeError, ValueError):
                continue

        # Periodically check for external clipboard changes
        now = time.time()
        if now - last_check >= check_interval:
            last_check = now
            current_mtime = get_db_mtime()
            if current_mtime != last_db_mtime:
                last_db_mtime = current_mtime
                if not TEST_MODE:
                    # 1. Update status badge (item count)
                    emit_status()

                    # 2. Update index (new items become searchable from main launcher)
                    indexed_ids = emit_incremental_index(indexed_ids)

                    # 3. If plugin is open, refresh the results list
                    if plugin_active:
                        entries = get_clipboard_entries()
                        ocr_cache = load_ocr_cache()
                        ocr_texts = get_ocr_text_for_entries(entries, ocr_cache)
                        respond(
                            get_entry_results(
                                entries, current_query, current_context, ocr_texts
                            ),
                            active_filter=current_context,
                        )


if __name__ == "__main__":
    main()
