#!/usr/bin/env python3
"""
Emoji plugin - search and copy emojis.
Emojis are loaded from bundled emojis.txt file.
Runs as a daemon and emits full index on startup.
"""

import json
import os
import select
import signal
import subprocess
import sys
from pathlib import Path

# Test mode for development
TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

# Load emojis from bundled file
PLUGIN_DIR = Path(__file__).parent
EMOJIS_FILE = PLUGIN_DIR / "emojis.txt"

# Recently used emojis tracking
CACHE_DIR = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "hamr"
RECENT_EMOJIS_FILE = CACHE_DIR / "recent-emojis.json"
MAX_RECENT_EMOJIS = 20


def load_recent_emojis() -> list[str]:
    """Load recently used emojis from cache"""
    if TEST_MODE:
        return []
    if not RECENT_EMOJIS_FILE.exists():
        return []
    try:
        return json.loads(RECENT_EMOJIS_FILE.read_text())
    except (json.JSONDecodeError, OSError):
        return []


def save_recent_emoji(emoji: str) -> None:
    """Save emoji to recent list (most recent first)"""
    if TEST_MODE:
        return
    recents = load_recent_emojis()
    if emoji in recents:
        recents.remove(emoji)
    recents.insert(0, emoji)
    recents = recents[:MAX_RECENT_EMOJIS]
    try:
        CACHE_DIR.mkdir(parents=True, exist_ok=True)
        RECENT_EMOJIS_FILE.write_text(json.dumps(recents))
    except OSError:
        pass


def load_emojis() -> list[dict]:
    """Load emojis from text file. Format: emoji description keywords"""
    emojis = []
    if not EMOJIS_FILE.exists():
        return emojis

    with open(EMOJIS_FILE, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            # Format: emoji_char description/keywords
            parts = line.split(" ", 1)
            if len(parts) >= 1:
                emoji = parts[0]
                description = parts[1] if len(parts) > 1 else ""
                emojis.append(
                    {
                        "emoji": emoji,
                        "description": description,
                        "searchable": f"{emoji} {description}".lower(),
                    }
                )
    return emojis


def fuzzy_match(query: str, emojis: list[dict]) -> list[dict]:
    """Simple fuzzy matching - all query words must appear in searchable text."""
    if not query.strip():
        return emojis[:100]  # Return first 100 when no query

    query_words = query.lower().split()
    results = []

    for e in emojis:
        searchable = e["searchable"]
        if all(word in searchable for word in query_words):
            results.append(e)
        if len(results) >= 50:
            break

    return results


def format_results(emojis: list[dict]) -> list[dict]:
    """Format emojis as hamr results (list view)."""
    return [
        {
            "id": f"emoji:{e['emoji']}",  # Match index ID format for frecency
            "name": e["description"][:50] if e["description"] else e["emoji"],
            "icon": e["emoji"],
            "iconType": "text",
            "verb": "Copy",
            "actions": [
                {"id": "copy", "name": "Copy", "icon": "content_copy"},
                {"id": "type", "name": "Type", "icon": "keyboard"},
            ],
        }
        for e in emojis
    ]


def format_grid_items(
    emojis: list[dict], recent_emojis: list[str] | None = None
) -> list[dict]:
    """Format emojis as grid items for gridBrowser.

    If recent_emojis is provided, prepend them to the grid.
    """
    items = []

    # Add recently used emojis first (with special styling)
    if recent_emojis:
        emoji_lookup = {e["emoji"]: e for e in emojis}
        for emoji_char in recent_emojis:
            if emoji_char in emoji_lookup:
                e = emoji_lookup[emoji_char]
                items.append(
                    {
                        "id": f"emoji:{e['emoji']}",  # Match index ID format
                        "name": e["description"][:20] if e["description"] else "",
                        "keywords": e["description"].split()
                        if e["description"]
                        else [],
                        "icon": e["emoji"],
                        "iconType": "text",
                    }
                )

    # Add all emojis (will include duplicates of recent, but that's OK for grid)
    for e in emojis:
        items.append(
            {
                "id": f"emoji:{e['emoji']}",  # Match index ID format
                "name": e["description"][:20] if e["description"] else "",
                "keywords": e["description"].split() if e["description"] else [],
                "icon": e["emoji"],
                "iconType": "text",
            }
        )

    return items


def copy_to_clipboard(text: str) -> None:
    """Copy text to clipboard using wl-copy."""
    if TEST_MODE:
        return  # Skip clipboard in test mode
    try:
        subprocess.run(["wl-copy", text], check=True)
    except FileNotFoundError:
        # Fallback to xclip if wl-copy not available
        try:
            subprocess.run(
                ["xclip", "-selection", "clipboard"], input=text.encode(), check=True
            )
        except FileNotFoundError:
            pass


def type_text(text: str) -> None:
    """Type text using wtype (wayland) or xdotool (x11)."""
    if TEST_MODE:
        return  # Skip typing in test mode
    try:
        subprocess.run(["wtype", text], check=True)
    except FileNotFoundError:
        try:
            subprocess.run(["xdotool", "type", "--", text], check=True)
        except FileNotFoundError:
            # Fallback to clipboard
            copy_to_clipboard(text)


def format_index_items(emojis: list[dict]) -> list[dict]:
    """Format emojis as indexable items for main search.

    Each item includes execute command so it can be run directly from main search
    without entering the plugin. The execute.name field enables history tracking
    so frequently used emojis rank higher.
    """
    return [
        {
            "id": f"emoji:{e['emoji']}",
            "name": e["description"][:50] if e["description"] else e["emoji"],
            "keywords": e["description"].split() if e["description"] else [],
            "icon": e["emoji"],
            "iconType": "text",
            "verb": "Copy",
            "execute": {
                "command": ["wl-copy", e["emoji"]],
                "notify": f"Copied {e['emoji']}",
                "name": f"{e['emoji']} {e['description'][:30]}"
                if e["description"]
                else e["emoji"],
            },
            "actions": [
                {
                    "id": "type",
                    "name": "Type",
                    "icon": "keyboard",
                    "command": ["wtype", e["emoji"]],
                }
            ],
        }
        for e in emojis
    ]


def handle_request(request: dict, emojis: list[dict]) -> None:
    """Handle a single request from the launcher."""
    step = request.get("step", "initial")
    query = request.get("query", "")
    selected = request.get("selected", {})
    action = request.get("action", "")

    if step == "index":
        items = format_index_items(emojis)
        print(
            json.dumps({"type": "index", "mode": "full", "items": items}),
            flush=True,
        )
        return

    if step == "initial":
        # Show all emojis in grid view, with recently used at the top
        recent_emojis = load_recent_emojis()
        grid_items = format_grid_items(emojis, recent_emojis)
        print(
            json.dumps(
                {
                    "type": "gridBrowser",
                    "gridBrowser": {
                        "title": "Select Emoji",
                        "items": grid_items,
                        "columns": 10,
                        "cellAspectRatio": 1.0,
                        "actions": [
                            {"id": "copy", "name": "Copy", "icon": "content_copy"},
                            {"id": "type", "name": "Type", "icon": "keyboard"},
                        ],
                    },
                }
            ),
            flush=True,
        )
        return

    if step == "search":
        # Search returns list view for better readability
        matches = fuzzy_match(query, emojis)
        results = format_results(matches)
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "placeholder": "Search emojis...",
                }
            ),
            flush=True,
        )
        return

    if step == "action":
        selected_id = selected.get("id", "")

        # Handle gridBrowser selection
        if selected_id == "gridBrowser":
            item_id = selected.get("itemId", "")
            # Extract emoji from prefixed ID (emoji:X -> X)
            emoji = item_id[6:] if item_id.startswith("emoji:") else item_id
            action_id = selected.get("action", "") or action or "copy"
        else:
            # Extract emoji from prefixed ID (emoji:X -> X)
            emoji = selected_id[6:] if selected_id.startswith("emoji:") else selected_id
            action_id = action if action else "copy"

        if not emoji:
            print(
                json.dumps({"type": "error", "message": "No emoji selected"}),
                flush=True,
            )
            return

        # Look up emoji description for history tracking
        emoji_data = next((e for e in emojis if e["emoji"] == emoji), None)
        description = emoji_data["description"][:30] if emoji_data else ""
        history_name = f"{emoji} {description}" if description else emoji

        if action_id == "type":
            type_text(emoji)
            save_recent_emoji(emoji)
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "notify": f"Typed {emoji}",
                            "close": True,
                            "name": history_name,
                        },
                    }
                ),
                flush=True,
            )
        else:
            copy_to_clipboard(emoji)
            save_recent_emoji(emoji)
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "notify": f"Copied {emoji}",
                            "close": True,
                            "name": history_name,
                        },
                    }
                ),
                flush=True,
            )
        return

    print(
        json.dumps({"type": "error", "message": f"Unknown step: {step}"}),
        flush=True,
    )


def main():
    signal.signal(signal.SIGTERM, lambda s, f: sys.exit(0))
    signal.signal(signal.SIGINT, lambda s, f: sys.exit(0))

    # Load emojis once at startup
    emojis = load_emojis()

    # Emit full index on startup (skip in test mode - tests use explicit index step)
    if not TEST_MODE:
        items = format_index_items(emojis)
        print(
            json.dumps({"type": "index", "mode": "full", "items": items}),
            flush=True,
        )

    # Daemon loop
    while True:
        readable, _, _ = select.select([sys.stdin], [], [], 1.0)
        if readable:
            line = sys.stdin.readline()
            if not line:
                break
            try:
                request = json.loads(line.strip())
                handle_request(request, emojis)
            except json.JSONDecodeError:
                print(
                    json.dumps({"type": "error", "message": "Invalid JSON"}),
                    flush=True,
                )


if __name__ == "__main__":
    main()
