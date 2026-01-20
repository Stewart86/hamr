#!/usr/bin/env python3
"""
Quicklinks plugin - search the web with predefined quicklinks.
Reads quicklinks from ~/.config/hamr/quicklinks.json

Simple stdio plugin that indexes quicklinks for main search.
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


def quicklink_to_result(link: dict) -> dict:
    """Convert quicklink to result item"""
    has_query = "{query}" in link.get("url", "")
    result = {
        "id": link["name"],
        "name": link["name"],
        "icon": link.get("icon", "link"),
        "verb": "Search" if has_query else "Open",
    }
    if link.get("aliases"):
        result["description"] = ", ".join(link["aliases"])
        result["keywords"] = link["aliases"]
    return result


def quicklink_to_index_item(link: dict) -> dict:
    """Convert quicklink to index item for main search"""
    has_query = "{query}" in link.get("url", "")
    item = {
        "id": link["name"],
        "name": link["name"],
        "icon": link.get("icon", "link"),
        "verb": "Search" if has_query else "Open",
    }
    if link.get("aliases"):
        item["keywords"] = link["aliases"]
    return item


def get_results(quicklinks: list[dict], query: str = "") -> list[dict]:
    """Get results, optionally filtered by query"""
    if not query:
        return [quicklink_to_result(l) for l in quicklinks]

    query_lower = query.lower()
    results = []
    for link in quicklinks:
        name_match = query_lower in link["name"].lower()
        alias_match = any(query_lower in a.lower() for a in link.get("aliases", []))
        if name_match or alias_match:
            results.append(quicklink_to_result(link))
    return results


def get_plugin_actions() -> list[dict]:
    """Get plugin-level actions for the action bar"""
    return [
        {
            "id": "add",
            "name": "Add Quicklink",
            "icon": "add_circle",
            "shortcut": "Ctrl+1",
        }
    ]


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")
    context = input_data.get("context", "")

    quicklinks = load_quicklinks()

    if step == "index":
        items = [quicklink_to_index_item(l) for l in quicklinks]
        print(json.dumps({"type": "index", "items": items}))
        return

    if step == "initial":
        results = get_results(quicklinks)
        if not results:
            results = [
                {
                    "id": "__empty__",
                    "name": "No quicklinks yet",
                    "icon": "info",
                    "description": "Use 'Add Quicklink' to create one",
                }
            ]
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                    "placeholder": "Search quicklinks...",
                    "pluginActions": get_plugin_actions(),
                }
            )
        )
        return

    if step == "search":
        # Check if we're in search mode for a specific quicklink
        # Context comes from plugin response, selected.id comes from daemon
        link_name = None
        if context and context.startswith("__search__:"):
            link_name = context.split(":", 1)[1]
        elif selected.get("id"):
            # Fallback: daemon passes selected.id when in submit mode
            link_name = selected.get("id")

        if link_name:
            link = next((l for l in quicklinks if l["name"] == link_name), None)
            if link and "{query}" in link.get("url", ""):
                if query:
                    # User entered search query, open URL
                    url = link["url"].replace("{query}", urllib.parse.quote(query))
                    print(
                        json.dumps(
                            {
                                "type": "execute",
                                "openUrl": url,
                                "close": True,
                            }
                        )
                    )
                    return
                # No query yet, show prompt
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": [
                                {
                                    "id": f"__open_direct__:{link_name}",
                                    "name": f"Open {link_name} homepage",
                                    "icon": link.get("icon", "link"),
                                    "verb": "Open",
                                }
                            ],
                            "inputMode": "submit",
                            "placeholder": f"Search {link_name}...",
                            "context": f"__search__:{link_name}",
                        }
                    )
                )
                return

        # Normal search filtering
        results = get_results(quicklinks, query)
        if not results:
            results = [
                {
                    "id": "__no_match__",
                    "name": f"No quicklinks matching '{query}'",
                    "icon": "search_off",
                }
            ]
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                    "placeholder": "Search quicklinks...",
                    "pluginActions": get_plugin_actions(),
                }
            )
        )
        return

    if step == "action":
        item_id = selected.get("id", "")

        # Plugin-level action: add
        if item_id == "__plugin__" and action == "add":
            print(
                json.dumps(
                    {
                        "type": "form",
                        "form": {
                            "title": "Add New Quicklink",
                            "fields": [
                                {
                                    "id": "name",
                                    "type": "text",
                                    "label": "Name",
                                    "placeholder": "e.g., Google",
                                    "required": True,
                                },
                                {
                                    "id": "url",
                                    "type": "text",
                                    "label": "URL",
                                    "placeholder": "https://google.com/search?q={query}",
                                    "required": True,
                                    "hint": "Use {query} as placeholder for search terms",
                                },
                                {
                                    "id": "icon",
                                    "type": "text",
                                    "label": "Icon",
                                    "placeholder": "Material icon name (optional)",
                                    "default": "link",
                                },
                            ],
                            "submitLabel": "Save",
                        },
                        "context": "__add__",
                    }
                )
            )
            return

        # Non-actionable items
        if item_id in ("__empty__", "__no_match__"):
            print(json.dumps({"type": "results", "results": []}))
            return

        # Back navigation - return to main list
        if item_id == "__back__":
            results = get_results(quicklinks)
            if not results:
                results = [
                    {"id": "__empty__", "name": "No quicklinks yet", "icon": "info"}
                ]
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": results,
                        "inputMode": "realtime",
                        "clearInput": True,
                        "placeholder": "Search quicklinks...",
                        "pluginActions": get_plugin_actions(),
                    }
                )
            )
            return

        # Open direct (homepage without query)
        if item_id.startswith("__open_direct__:"):
            link_name = item_id.split(":", 1)[1]
            link = next((l for l in quicklinks if l["name"] == link_name), None)
            if link:
                url = link["url"].replace("{query}", "")
                print(
                    json.dumps(
                        {
                            "type": "execute",
                            "openUrl": url,
                            "close": True,
                        }
                    )
                )
            return

        # Find the selected quicklink
        link = next((l for l in quicklinks if l["name"] == item_id), None)
        if not link:
            print(
                json.dumps(
                    {"type": "error", "message": f"Quicklink not found: {item_id}"}
                )
            )
            return

        url_template = link.get("url", "")

        # If URL has {query} placeholder, enter search mode
        if "{query}" in url_template:
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": [
                            {
                                "id": f"__open_direct__:{link['name']}",
                                "name": f"Open {link['name']} homepage",
                                "icon": link.get("icon", "link"),
                                "verb": "Open",
                            }
                        ],
                        "inputMode": "submit",
                        "clearInput": True,
                        "context": f"__search__:{link['name']}",
                        "placeholder": f"Search {link['name']}...",
                        # Activate plugin for multi-step flow when selected from main search
                        "activate": True,
                    }
                )
            )
            return

        # No placeholder - just open the URL
        print(
            json.dumps(
                {
                    "type": "execute",
                    "openUrl": url_template,
                    "close": True,
                }
            )
        )
        return

    if step == "form":
        form_data = input_data.get("formData", {})

        if context == "__add__":
            name = form_data.get("name", "").strip()
            url = form_data.get("url", "").strip()
            icon = form_data.get("icon", "link").strip() or "link"

            if not name:
                print(json.dumps({"type": "error", "message": "Name is required"}))
                return

            if not url:
                print(json.dumps({"type": "error", "message": "URL is required"}))
                return

            if any(l["name"] == name for l in quicklinks):
                print(
                    json.dumps(
                        {
                            "type": "error",
                            "message": f"Quicklink '{name}' already exists",
                        }
                    )
                )
                return

            quicklinks.append({"name": name, "url": url, "icon": icon})
            if save_quicklinks(quicklinks):
                results = get_results(quicklinks)
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": results,
                            "inputMode": "realtime",
                            "clearInput": True,
                            "placeholder": "Search quicklinks...",
                            "pluginActions": get_plugin_actions(),
                        }
                    )
                )
            else:
                print(
                    json.dumps({"type": "error", "message": "Failed to save quicklink"})
                )
            return

    print(json.dumps({"type": "results", "results": []}))


if __name__ == "__main__":
    main()
