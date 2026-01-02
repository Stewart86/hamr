#!/usr/bin/env python3
"""
Emoji plugin - search and copy emojis.
Emojis are loaded from bundled emojis.txt file.
"""

import json
import os
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
            "id": e["emoji"],
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
                        "id": e["emoji"],
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
                "id": e["emoji"],
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


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "")
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    # Load emojis
    emojis = load_emojis()

    if step == "index":
        items = format_index_items(emojis)
        print(json.dumps({"type": "index", "items": items}))
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
            )
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
            )
        )
        return

    if step == "action":
        selected_id = selected.get("id", "")

        # Handle gridBrowser selection
        if selected_id == "gridBrowser":
            emoji = selected.get("itemId", "")
            action_id = selected.get("action", "") or action or "copy"
        else:
            emoji = selected_id
            action_id = action if action else "copy"

        if not emoji:
            print(json.dumps({"type": "error", "message": "No emoji selected"}))
            return

        if action_id == "type":
            type_text(emoji)
            save_recent_emoji(emoji)
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {"notify": f"Typed {emoji}", "close": True},
                    }
                )
            )
        else:
            copy_to_clipboard(emoji)
            save_recent_emoji(emoji)
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {"notify": f"Copied {emoji}", "close": True},
                    }
                )
            )
        return

    print(json.dumps({"type": "error", "message": f"Unknown step: {step}"}))


if __name__ == "__main__":
    main()
