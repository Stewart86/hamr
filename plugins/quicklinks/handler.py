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
    print(
        json.dumps(
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
    )


def show_edit_form(name: str, current_url: str, current_icon: str = "link"):
    """Show form for editing an existing quicklink"""
    print(
        json.dumps(
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

    # Empty state hint
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
        "id": f"quicklink:{name}",
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
            "command": ["xdg-open", url],
            "name": f"Open {name}",  # Enable history tracking
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


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    quicklinks = load_quicklinks()
    selected_id = selected.get("id", "")
    context = input_data.get("context", "")

    # ===== INDEX: Provide searchable items for main search =====
    if step == "index":
        items = [quicklink_to_index_item(link) for link in quicklinks]
        print(json.dumps({"type": "index", "items": items}))
        return

    # ===== INITIAL: Show all quicklinks =====
    if step == "initial":
        results = get_main_menu(quicklinks)
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

    # ===== FORM: Handle form submission =====
    if step == "form":
        form_data = input_data.get("formData", {})

        # Adding new quicklink
        if context == "__add__":
            name = form_data.get("name", "").strip()
            url = form_data.get("url", "").strip()
            icon = form_data.get("icon", "").strip() or "link"

            if not name:
                print(json.dumps({"type": "error", "message": "Name is required"}))
                return

            # Check if name already exists
            if any(l["name"] == name for l in quicklinks):
                print(
                    json.dumps({"type": "error", "message": f"'{name}' already exists"})
                )
                return

            if not url:
                print(json.dumps({"type": "error", "message": "URL is required"}))
                return

            # Add https:// if no protocol
            if not url.startswith("http://") and not url.startswith("https://"):
                url = "https://" + url

            new_link = {"name": name, "url": url, "icon": icon}
            quicklinks.append(new_link)

            if save_quicklinks(quicklinks):
                print(
                    json.dumps(
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
                )
            else:
                print(
                    json.dumps({"type": "error", "message": "Failed to save quicklink"})
                )
            return

        # Editing existing quicklink
        if context.startswith("__edit__:"):
            name = context.split(":", 1)[1]
            url = form_data.get("url", "").strip()
            icon = form_data.get("icon", "").strip() or "link"

            if not url:
                print(json.dumps({"type": "error", "message": "URL is required"}))
                return

            # Add https:// if no protocol
            if not url.startswith("http://") and not url.startswith("https://"):
                url = "https://" + url

            for link in quicklinks:
                if link["name"] == name:
                    link["url"] = url
                    link["icon"] = icon
                    break

            if save_quicklinks(quicklinks):
                print(
                    json.dumps(
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
                )
            else:
                print(
                    json.dumps({"type": "error", "message": "Failed to save quicklink"})
                )
            return

    # ===== SEARCH: Context-dependent search =====
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
                print(
                    json.dumps(
                        {
                            "type": "execute",
                            "execute": {
                                "command": ["xdg-open", url],
                                "name": f"{link['name']}: {query}",
                                "icon": link.get("icon", "search"),
                                "close": True,
                            },
                        }
                    )
                )
            else:
                # Empty input - allow opening the link directly (without query)
                base_url = link.get("url", "").replace("{query}", "")
                print(
                    json.dumps(
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
                    "pluginActions": get_plugin_actions(),
                }
            )
        )
        return

    # ===== ACTION: Handle selection =====
    if step == "action":
        # Plugin-level action: add (from action bar)
        if selected_id == "__plugin__" and action == "add":
            show_add_form()
            return

        # Form cancelled - return to list
        if selected_id == "__form_cancel__":
            print(
                json.dumps(
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
            )
            return

        # Back button - return to main list (core handles depth via pendingBack)
        if selected_id == "__back__":
            results = get_main_menu(quicklinks)
            print(
                json.dumps(
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
            )
            return

        # Non-actionable items
        if selected_id in ("__error__", "__current_url__", "__empty__"):
            return

        # Edit action - show edit form
        if action == "edit":
            link_name = selected_id
            link = next((l for l in quicklinks if l["name"] == link_name), None)
            if link:
                show_edit_form(link_name, link.get("url", ""), link.get("icon", "link"))
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
                            "pluginActions": get_plugin_actions(),
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

        # Legacy item click support for __add__
        if selected_id == "__add__":
            show_add_form()
            return

        # Open link directly (without query parameter)
        if selected_id.startswith("__open_direct__:"):
            link_name = selected_id.split(":", 1)[1]
            link = next((l for l in quicklinks if l["name"] == link_name), None)
            if link:
                # Remove {query} placeholder for direct open
                url = link["url"].replace("{query}", "")
                print(
                    json.dumps(
                        {
                            "type": "execute",
                            "execute": {
                                "command": ["xdg-open", url],
                                "name": f"Open {link_name}",
                                "icon": link.get("icon", "link"),
                                "close": True,
                            },
                        }
                    )
                )
            return

        # Execute search (legacy support)
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
        # Core sets pendingNavigation for default clicks, so no need for navigateForward
        if "{query}" in url_template:
            base_url = url_template.replace("{query}", "")
            print(
                json.dumps(
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
