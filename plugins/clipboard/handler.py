#!/usr/bin/env python3
"""
Clipboard workflow handler - browse and manage clipboard history via cliphist.
Features: list, search, copy, delete, wipe, image thumbnails, OCR search
"""

import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

# Cache directory for image thumbnails and OCR
CACHE_DIR = (
    Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache"))
    / "hamr"
    / "clipboard-thumbs"
)
OCR_CACHE_FILE = CACHE_DIR / "ocr-index.json"
SCRIPT_DIR = Path(__file__).parent
# Max thumbnail size (width or height)
MAX_THUMB_SIZE = 256


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
    """Get a stable hash for a clipboard entry"""
    return hashlib.md5(entry.encode()).hexdigest()[:16]


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


def get_entry_results(
    entries: list[str],
    query: str = "",
    filter_type: str = "",
    ocr_texts: dict[str, str] | None = None,
    limit: int = 50,
) -> list[dict]:
    """Convert clipboard entries to result format"""
    results = []
    ocr_texts = ocr_texts or {}
    entry_index = 0

    for entry in entries:
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

        result = {
            "id": entry,
            "name": display,
            "icon": icon,
            "description": entry_type,
            "verb": "Paste",
            "actions": [
                {"id": "copy", "name": "Copy", "icon": "content_copy"},
                {"id": "delete", "name": "Delete", "icon": "delete"},
            ],
        }

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


def respond(results: list[dict], **kwargs):
    """Send a results response"""
    active_filter = kwargs.get("active_filter", "")
    response = {
        "type": "results",
        "results": results,
        "inputMode": "realtime",
        "placeholder": kwargs.get("placeholder", "Search clipboard..."),
        "pluginActions": get_plugin_actions(active_filter),
    }
    if active_filter:
        response["context"] = active_filter
    if kwargs.get("clear_input"):
        response["clearInput"] = True
    if kwargs.get("navigate_forward") is False:
        response["navigateForward"] = False
    print(json.dumps(response))


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

    item = {
        "id": f"clip:{get_entry_hash(entry)}",
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
                "command": [
                    "bash",
                    "-c",
                    f"printf '%s' '{shell_escape(entry)}' | cliphist delete",
                ],
            },
        ],
        "execute": {
            "command": [
                "bash",
                "-c",
                f"printf '%s' '{shell_escape(entry)}' | cliphist decode | wl-copy",
            ],
            "notify": "Copied to clipboard",
        },
    }

    if thumbnail:
        item["thumbnail"] = thumbnail

    return item


def main():
    input_data = json.load(sys.stdin)
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

        # Build current ID set from entries
        current_entries = entries[:300]
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
                )
            )
        else:
            # Full reindex
            items = [entry_to_index_item(e, ocr_texts) for e in current_entries]
            print(json.dumps({"type": "index", "items": items}))
        return

    if step == "initial":
        spawn_ocr_indexer()
        respond(get_entry_results(entries, ocr_texts=ocr_texts))
        return

    if step == "search":
        respond(
            get_entry_results(entries, query, context, ocr_texts),
            active_filter=context,
        )
        return

    # Action: handle clicks
    if step == "action":
        item_id = selected.get("id", "")

        # Plugin-level actions (from action bar)
        if item_id == "__plugin__":
            # Filter by images - toggle (view modification, not navigation)
            if action == "filter_images":
                new_filter = "" if context == "images" else "images"
                respond(
                    get_entry_results(entries, query, new_filter, ocr_texts),
                    active_filter=new_filter,
                    navigate_forward=False,
                )
                return

            # Filter by text - toggle (view modification, not navigation)
            if action == "filter_text":
                new_filter = "" if context == "text" else "text"
                respond(
                    get_entry_results(entries, query, new_filter, ocr_texts),
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
                            "execute": {
                                "command": [
                                    "notify-send",
                                    "Clipboard",
                                    "History cleared",
                                    "-a",
                                    "Shell",
                                ],
                                "close": True,
                            },
                        }
                    )
                )
                return

        # Back action - clear filter and go back to unfiltered list
        if item_id == "__back__":
            respond(
                get_entry_results(entries, query, "", ocr_texts),
                active_filter="",
            )
            return

        if item_id == "__empty__":
            respond(
                get_entry_results(entries, query, context, ocr_texts),
                active_filter=context,
            )
            return

        # Clipboard entry actions
        entry = item_id

        if action == "delete":
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
                get_entry_results(entries, query, context, ocr_texts),
                active_filter=context,
            )
            return

        # Default action (click) or explicit copy
        if action == "copy" or not action:
            copy_entry(entry)
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["true"],  # No-op, just close
                            "close": True,
                        },
                    }
                )
            )
            return

    # Unknown
    print(json.dumps({"type": "error", "message": f"Unknown step: {step}"}))


if __name__ == "__main__":
    main()
