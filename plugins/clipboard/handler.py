#!/usr/bin/env python3
"""
Clipboard workflow handler - browse and manage clipboard history via cliphist.
Features: list, search, copy, delete, wipe, image thumbnails
"""

import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path


# Cache directory for image thumbnails
CACHE_DIR = (
    Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache"))
    / "hamr"
    / "clipboard-thumbs"
)
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


def get_image_thumbnail(entry: str) -> str | None:
    """Get or create thumbnail for image entry, return path or None"""
    if not is_image(entry):
        return None

    CACHE_DIR.mkdir(parents=True, exist_ok=True)

    # Use entry hash as filename
    entry_hash = hashlib.md5(entry.encode()).hexdigest()[:16]
    thumb_path = CACHE_DIR / f"{entry_hash}.png"

    # Return cached thumbnail if exists
    if thumb_path.exists():
        return str(thumb_path)

    # Decode and save thumbnail
    try:
        decode_proc = subprocess.run(
            ["cliphist", "decode"],
            input=entry.encode("utf-8"),
            capture_output=True,
            timeout=5,
        )
        if decode_proc.returncode != 0 or not decode_proc.stdout:
            return None

        # Check if we need to resize (use ImageMagick if available)
        dims = get_image_dimensions(entry)
        if dims and (dims[0] > MAX_THUMB_SIZE or dims[1] > MAX_THUMB_SIZE):
            # Resize with ImageMagick convert
            resize_proc = subprocess.run(
                [
                    "magick",
                    "-",
                    "-thumbnail",
                    f"{MAX_THUMB_SIZE}x{MAX_THUMB_SIZE}>",
                    str(thumb_path),
                ],
                input=decode_proc.stdout,
                capture_output=True,
                timeout=10,
            )
            if resize_proc.returncode == 0 and thumb_path.exists():
                return str(thumb_path)
            # Fall back to saving full size if resize fails

        # Save as-is if small or resize failed
        thumb_path.write_bytes(decode_proc.stdout)
        return str(thumb_path)

    except (subprocess.TimeoutExpired, Exception) as e:
        print(f"Error decoding image: {e}", file=sys.stderr)

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


def get_entry_results(entries: list[str], query: str = "") -> list[dict]:
    """Convert clipboard entries to result format"""
    results = []

    # No longer add wipe as a result item - it's now a plugin action

    for entry in entries:
        if query and not fuzzy_match(query, clean_entry(entry)):
            continue

        display = clean_entry(entry)
        is_img = is_image(entry)

        # For images, show dimensions
        if is_img:
            dims = get_image_dimensions(entry)
            display = f"Image {dims[0]}x{dims[1]}" if dims else "Image"
            entry_type = "Image"
            icon = "image"
            thumbnail = get_image_thumbnail(entry)
        else:
            # Truncate long text entries
            if len(display) > 100:
                display = display[:100] + "..."
            entry_type = "Text"
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


def get_plugin_actions(show_wipe: bool = True) -> list[dict]:
    """Get plugin-level actions for the action bar"""
    actions = []
    if show_wipe:
        actions.append(
            {
                "id": "wipe",
                "name": "Wipe All",
                "icon": "delete_sweep",
                "confirm": "Wipe all clipboard history? This cannot be undone.",
                "shortcut": "Ctrl+1",
            }
        )
    return actions


def respond(results: list[dict], **kwargs):
    """Send a results response"""
    response = {
        "type": "results",
        "results": results,
        "inputMode": "realtime",
        "placeholder": kwargs.get("placeholder", "Search clipboard..."),
        "pluginActions": kwargs.get("plugin_actions", get_plugin_actions()),
    }
    if kwargs.get("clear_input"):
        response["clearInput"] = True
    print(json.dumps(response))


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    entries = get_clipboard_entries()

    # Initial: show clipboard list
    if step == "initial":
        respond(get_entry_results(entries))
        return

    # Search: filter entries
    if step == "search":
        respond(get_entry_results(entries, query))
        return

    # Action: handle clicks
    if step == "action":
        item_id = selected.get("id", "")

        # Plugin-level action: wipe (from action bar)
        if item_id == "__plugin__" and action == "wipe":
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

        # Empty state - ignore
        if item_id == "__empty__":
            respond(get_entry_results(entries))
            return

        # Clipboard entry actions
        entry = item_id

        if action == "delete":
            delete_entry(entry)
            # Refresh entries after delete
            entries = [e for e in entries if e != entry]
            respond(get_entry_results(entries))
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
