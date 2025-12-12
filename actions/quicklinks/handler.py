#!/usr/bin/env python3
"""
Quicklinks workflow handler - search the web with predefined quicklinks
Reads quicklinks from ~/.config/hamr/quicklinks.json

Features:
- Browse and search quicklinks
- Execute search with query placeholder
- Add new quicklinks
- Delete existing quicklinks
- Edit existing quicklinks
"""

import json
import sys
import urllib.parse
from pathlib import Path

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
    """Get main menu with quicklinks and add option"""
    results = []

    # Filter quicklinks
    filtered = filter_quicklinks(query, quicklinks)
    results.extend(get_quicklink_list(filtered))

    # Add "Add new quicklink" option at the end
    results.append(
        {
            "id": "__add__",
            "name": "Add new quicklink",
            "description": "Create a custom quicklink",
            "icon": "add_circle",
        }
    )

    return results


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    quicklinks = load_quicklinks()
    selected_id = selected.get("id", "")

    # ===== INITIAL: Show all quicklinks + add option =====
    if step == "initial":
        results = get_main_menu(quicklinks)
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                    "placeholder": "Search quicklinks...",
                }
            )
        )
        return

    # ===== SEARCH: Context-dependent search =====
    if step == "search":
        # Adding new quicklink - step 1: entering name (submit mode)
        if selected_id == "__add__":
            if query:
                # Check if name already exists
                exists = any(l["name"] == query for l in quicklinks)
                if exists:
                    print(
                        json.dumps(
                            {
                                "type": "results",
                                "inputMode": "submit",
                                "placeholder": "Enter quicklink name (Enter to confirm)",
                                "results": [
                                    {
                                        "id": "__back__",
                                        "name": "Back",
                                        "icon": "arrow_back",
                                    },
                                    {
                                        "id": "__error__",
                                        "name": f"'{query}' already exists",
                                        "icon": "error",
                                        "description": "Choose a different name",
                                    },
                                ],
                            }
                        )
                    )
                else:
                    print(
                        json.dumps(
                            {
                                "type": "results",
                                "inputMode": "submit",
                                "placeholder": "Enter quicklink name (Enter to confirm)",
                                "results": [
                                    {
                                        "id": f"__add_name__:{query}",
                                        "name": f"Create '{query}'",
                                        "description": "Next: enter URL",
                                        "icon": "add_circle",
                                        "verb": "Next",
                                    },
                                    {
                                        "id": "__back__",
                                        "name": "Back",
                                        "icon": "arrow_back",
                                    },
                                ],
                            }
                        )
                    )
            else:
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "inputMode": "submit",
                            "placeholder": "Enter quicklink name (Enter to confirm)",
                            "results": [
                                {"id": "__back__", "name": "Back", "icon": "arrow_back"}
                            ],
                        }
                    )
                )
            return

        # Editing quicklink URL (submit mode)
        if selected_id.startswith("__edit__:"):
            name = selected_id.split(":", 1)[1]
            link = next((l for l in quicklinks if l["name"] == name), None)
            current_url = link.get("url", "") if link else ""

            if query:
                has_placeholder = "{query}" in query
                display_url = query if query.startswith("http") else f"https://{query}"

                print(
                    json.dumps(
                        {
                            "type": "results",
                            "inputMode": "submit",
                            "placeholder": f"Edit URL for '{name}' (Enter to save)",
                            "results": [
                                {
                                    "id": f"__edit_save__:{name}:{query}",
                                    "name": f"Save '{name}'",
                                    "description": display_url
                                    + (" (with search)" if has_placeholder else ""),
                                    "icon": "save",
                                    "verb": "Save",
                                },
                                {
                                    "id": "__back__",
                                    "name": "Cancel",
                                    "icon": "arrow_back",
                                },
                            ],
                        }
                    )
                )
            else:
                # Show current URL as hint
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "inputMode": "submit",
                            "placeholder": f"Edit URL for '{name}' (Enter to save)",
                            "results": [
                                {
                                    "id": "__current_url__",
                                    "name": f"Current: {current_url}",
                                    "description": "Type new URL above",
                                    "icon": "info",
                                },
                                {
                                    "id": "__back__",
                                    "name": "Cancel",
                                    "icon": "arrow_back",
                                },
                            ],
                        }
                    )
                )
            return

        # Adding new quicklink - step 2: entering URL (submit mode)
        if selected_id.startswith("__add_name__:"):
            name = selected_id.split(":", 1)[1]
            if query:
                has_placeholder = "{query}" in query
                # Show URL with https:// prefix if not present
                display_url = query if query.startswith("http") else f"https://{query}"

                print(
                    json.dumps(
                        {
                            "type": "results",
                            "inputMode": "submit",
                            "placeholder": "Enter URL (Enter to save)",
                            "results": [
                                {
                                    "id": f"__add_save__:{name}:{query}",
                                    "name": f"Save '{name}'",
                                    "description": display_url
                                    + (" (with search)" if has_placeholder else ""),
                                    "icon": "save",
                                    "verb": "Save",
                                },
                                {
                                    "id": "__back__",
                                    "name": "Back",
                                    "icon": "arrow_back",
                                },
                            ],
                        }
                    )
                )
            else:
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "inputMode": "submit",
                            "placeholder": "Enter URL (Enter to save)",
                            "results": [
                                {"id": "__back__", "name": "Back", "icon": "arrow_back"}
                            ],
                        }
                    )
                )
            return

        # Search mode for a quicklink with {query} (submit mode for search input)
        link = is_quicklink_with_query(selected_id, quicklinks)
        if link:
            search_placeholder = f"Search {link['name']}... (Enter to search)"
            if query:
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "inputMode": "submit",
                            "placeholder": search_placeholder,
                            "results": [
                                {
                                    "id": f"__execute__:{selected_id}:{query}",
                                    "name": query,
                                    "description": f"Search {link['name']}",
                                    "icon": link.get("icon", "search"),
                                    "verb": "Search",
                                },
                                {
                                    "id": "__back__",
                                    "name": "Back to quicklinks",
                                    "icon": "arrow_back",
                                },
                            ],
                        }
                    )
                )
            else:
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "inputMode": "submit",
                            "placeholder": search_placeholder,
                            "results": [
                                {
                                    "id": "__back__",
                                    "name": "Back to quicklinks",
                                    "icon": "arrow_back",
                                }
                            ],
                        }
                    )
                )
            return

        # Normal quicklink filtering (realtime mode)
        results = get_main_menu(quicklinks, query)
        print(
            json.dumps(
                {
                    "type": "results",
                    "inputMode": "realtime",
                    "results": results,
                    "placeholder": "Search quicklinks...",
                }
            )
        )
        return

    # ===== ACTION: Handle selection =====
    if step == "action":
        # Back button
        if selected_id == "__back__":
            results = get_main_menu(quicklinks)
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": results,
                        "inputMode": "realtime",
                        "clearInput": True,
                        "placeholder": "Search quicklinks...",
                    }
                )
            )
            return

        # Error items are not actionable
        if selected_id == "__error__":
            return

        # Current URL info item is not actionable
        if selected_id == "__current_url__":
            return

        # Edit action on a quicklink - enter edit mode (submit mode)
        if action == "edit":
            link_name = selected_id
            link = next((l for l in quicklinks if l["name"] == link_name), None)
            if link:
                current_url = link.get("url", "")
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "inputMode": "submit",
                            "clearInput": True,
                            "context": f"__edit__:{link_name}",  # Set context for search calls
                            "placeholder": f"Edit URL for '{link_name}' (Enter to save)",
                            "results": [
                                {
                                    "id": "__current_url__",
                                    "name": f"Current: {current_url}",
                                    "description": "Type new URL above",
                                    "icon": "info",
                                },
                                {
                                    "id": "__back__",
                                    "name": "Cancel",
                                    "icon": "arrow_back",
                                },
                            ],
                        }
                    )
                )
            return

        # Delete action on a quicklink
        if action == "delete":
            link_name = selected_id
            quicklinks = [l for l in quicklinks if l["name"] != link_name]
            if save_quicklinks(quicklinks):
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": get_main_menu(quicklinks),
                            "inputMode": "realtime",
                            "clearInput": True,
                            "placeholder": "Search quicklinks...",
                        }
                    )
                )
            else:
                print(
                    json.dumps(
                        {"type": "error", "message": "Failed to save quicklinks"}
                    )
                )
            return

        # Start adding new quicklink (submit mode)
        if selected_id == "__add__":
            print(
                json.dumps(
                    {
                        "type": "results",
                        "inputMode": "submit",
                        "clearInput": True,
                        "placeholder": "Enter quicklink name (Enter to confirm)",
                        "results": [
                            {"id": "__back__", "name": "Back", "icon": "arrow_back"}
                        ],
                    }
                )
            )
            return

        # Confirm quicklink name, move to URL input (submit mode)
        if selected_id.startswith("__add_name__:"):
            print(
                json.dumps(
                    {
                        "type": "results",
                        "inputMode": "submit",
                        "clearInput": True,
                        "placeholder": "Enter URL (Enter to save)",
                        "results": [
                            {"id": "__back__", "name": "Back", "icon": "arrow_back"}
                        ],
                    }
                )
            )
            return

        # Save edited quicklink
        if selected_id.startswith("__edit_save__:"):
            parts = selected_id.split(":", 2)
            name = parts[1]
            url = parts[2] if len(parts) > 2 else ""

            # Add https:// if no protocol
            if not url.startswith("http://") and not url.startswith("https://"):
                url = "https://" + url

            # Update existing quicklink
            for link in quicklinks:
                if link["name"] == name:
                    link["url"] = url
                    break

            if save_quicklinks(quicklinks):
                quicklinks = load_quicklinks()
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": get_main_menu(quicklinks),
                            "inputMode": "realtime",
                            "clearInput": True,
                            "placeholder": "Search quicklinks...",
                        }
                    )
                )
            else:
                print(
                    json.dumps(
                        {"type": "error", "message": "Failed to save quicklinks"}
                    )
                )
            return

        # Save new quicklink
        if selected_id.startswith("__add_save__:"):
            parts = selected_id.split(":", 2)
            name = parts[1]
            url = parts[2] if len(parts) > 2 else ""

            # Add https:// if no protocol
            if not url.startswith("http://") and not url.startswith("https://"):
                url = "https://" + url

            # Add new quicklink
            new_link = {"name": name, "url": url, "icon": "link"}
            quicklinks.append(new_link)

            if save_quicklinks(quicklinks):
                # Reload and show updated list with notification
                quicklinks = load_quicklinks()
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": get_main_menu(quicklinks),
                            "inputMode": "realtime",
                            "clearInput": True,
                            "placeholder": "Search quicklinks...",
                        }
                    )
                )
            else:
                print(
                    json.dumps(
                        {"type": "error", "message": "Failed to save quicklinks"}
                    )
                )
            return

        # Execute search
        if selected_id.startswith("__execute__:"):
            parts = selected_id.split(":", 2)
            link_name = parts[1]
            search_query = parts[2] if len(parts) > 2 else ""

            link = next((l for l in quicklinks if l["name"] == link_name), None)
            if link:
                url = link["url"].replace("{query}", urllib.parse.quote(search_query))
                print(
                    json.dumps(
                        {
                            "type": "execute",
                            "execute": {
                                "command": ["xdg-open", url],
                                "name": f"{link_name}: {search_query}",
                                "icon": link.get("icon", "search"),
                                "close": True,
                            },
                        }
                    )
                )
            return

        # Direct quicklink selection
        link = next((l for l in quicklinks if l["name"] == selected_id), None)
        if not link:
            print(
                json.dumps(
                    {"type": "error", "message": f"Quicklink not found: {selected_id}"}
                )
            )
            return

        url_template = link.get("url", "")

        # If URL has {query} placeholder, enter search mode (submit mode)
        if "{query}" in url_template:
            print(
                json.dumps(
                    {
                        "type": "results",
                        "inputMode": "submit",
                        "clearInput": True,
                        "placeholder": f"Search {link['name']}... (Enter to search)",
                        "results": [
                            {
                                "id": "__back__",
                                "name": "Back to quicklinks",
                                "icon": "arrow_back",
                            }
                        ],
                    }
                )
            )
            return

        # No placeholder - just open the URL directly
        print(
            json.dumps(
                {
                    "type": "execute",
                    "execute": {
                        "command": ["xdg-open", url_template],
                        "name": f"Open {link['name']}",
                        "icon": link.get("icon", "link"),
                        "close": True,
                    },
                }
            )
        )


if __name__ == "__main__":
    main()
