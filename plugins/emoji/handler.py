#!/usr/bin/env python3
"""
Emoji plugin - search and copy emojis.
Emojis are loaded from bundled emojis.tsv file.
Runs as a socket daemon and indexes emojis on startup.
"""

import asyncio
import json
import os
import subprocess
import sys
from pathlib import Path

# Add parent directory to path to import SDK
sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

# Load emojis from bundled file
PLUGIN_DIR = Path(__file__).parent
EMOJIS_FILE = PLUGIN_DIR / "emojis.tsv"

# Recently used emojis tracking
CACHE_DIR = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "hamr"
RECENT_EMOJIS_FILE = CACHE_DIR / "recent-emojis.json"
MAX_RECENT_EMOJIS = 20


def load_recent_emojis() -> list[str]:
    """Load recently used emojis from cache"""
    if not RECENT_EMOJIS_FILE.exists():
        return []
    try:
        return json.loads(RECENT_EMOJIS_FILE.read_text())
    except (json.JSONDecodeError, OSError):
        return []


def save_recent_emoji(emoji: str) -> None:
    """Save emoji to recent list (most recent first)"""
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
    """Load emojis from TSV file. Format: emoji<TAB>name<TAB>keywords"""
    emojis = []
    if not EMOJIS_FILE.exists():
        return emojis

    with open(EMOJIS_FILE, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            parts = line.split("\t")
            if len(parts) >= 1:
                emoji = parts[0]
                name = parts[1] if len(parts) > 1 else ""
                keywords = parts[2] if len(parts) > 2 else ""
                emojis.append(
                    {
                        "emoji": emoji,
                        "name": name,
                        "keywords": keywords.split() if keywords else [],
                        "searchable": f"{emoji} {name} {keywords}".lower(),
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


def copy_to_clipboard(text: str) -> None:
    """Copy text to clipboard using wl-copy."""
    try:
        subprocess.run(["wl-copy"], input=text.encode(), check=True)
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
    try:
        subprocess.run(["wtype", text], check=True)
    except FileNotFoundError:
        try:
            subprocess.run(["xdotool", "type", "--", text], check=True)
        except FileNotFoundError:
            # Fallback to clipboard
            copy_to_clipboard(text)


def format_index_items(emojis: list[dict]) -> list[dict]:
    """Format emojis as indexable items for main search."""
    return [
        {
            "id": f"emoji:{e['emoji']}",
            "name": e["name"] if e["name"] else e["emoji"],
            "keywords": e["keywords"],
            "icon": e["emoji"],
            "iconType": "text",
            "verb": "Copy",
            "actions": [
                {
                    "id": "type",
                    "name": "Type",
                    "icon": "keyboard",
                }
            ],
        }
        for e in emojis
    ]


def format_results(emojis: list[dict]) -> list[dict]:
    """Format emojis as search results."""
    return [
        {
            "id": f"emoji:{e['emoji']}",
            "name": e["name"] if e["name"] else e["emoji"],
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


# Create plugin instance
plugin = HamrPlugin(
    id="emoji",
    name="Emoji",
    description="Search and copy emojis",
    icon="emoji_emotions",
)

# Load emojis once at startup
emojis = load_emojis()


@plugin.on_initial
async def handle_initial(params=None):
    """Handle initial request when plugin is opened."""
    results = format_results(emojis)
    return HamrPlugin.results(
        results, placeholder="Search emojis...", display_hint="grid"
    )


@plugin.on_search
async def handle_search(query: str, context=None):
    """Handle search request."""
    matches = fuzzy_match(query, emojis)
    results = format_results(matches)
    return HamrPlugin.results(
        results, placeholder="Search emojis...", display_hint="grid"
    )


@plugin.on_action
async def handle_action(item_id: str, action=None, context=None):
    """Handle action request (copy or type emoji)."""
    # Extract emoji from item ID (emoji:X -> X)
    emoji = item_id[6:] if item_id.startswith("emoji:") else item_id

    if not emoji:
        return {"error": "No emoji selected"}

    # Look up emoji name for history tracking
    emoji_data = next((e for e in emojis if e["emoji"] == emoji), None)
    name = emoji_data["name"][:30] if emoji_data else ""
    history_name = f"{emoji} {name}" if name else emoji

    if action == "type":
        type_text(emoji)
        save_recent_emoji(emoji)
        await plugin.send_execute(
            {
                "type": "notify",
                "message": f"Typed {emoji}",
            }
        )
        return HamrPlugin.close()
    else:
        # Default to copy
        copy_to_clipboard(emoji)
        save_recent_emoji(emoji)
        await plugin.send_execute(
            {
                "type": "notify",
                "message": f"Copied {emoji}",
            }
        )
        return HamrPlugin.close()


@plugin.add_background_task
async def emit_index(p: HamrPlugin):
    """Background task to emit full index on startup."""
    items = format_index_items(emojis)
    await p.send_index(items)
    # Keep task alive but don't do anything else
    while True:
        await asyncio.sleep(1)


if __name__ == "__main__":
    plugin.run()
