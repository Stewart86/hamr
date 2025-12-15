#!/usr/bin/env python3
"""
Apps workflow handler - browse and launch applications like rofi/fuzzel/dmenu.

Features:
- Lists all applications from .desktop files
- Category filtering (All, Development, Graphics, Internet, etc.)
- Fuzzy search within current category
- Frecency-based sorting (recently/frequently used apps first)
"""

import json
import os
import subprocess
import sys
from configparser import ConfigParser
from pathlib import Path

HISTORY_PATH = Path.home() / ".local/state/quickshell/user/search-history.json"

# XDG application directories
APP_DIRS = [
    Path.home() / ".local/share/applications",
    Path.home() / ".local/share/flatpak/exports/share/applications",
    Path("/usr/share/applications"),
    Path("/usr/local/share/applications"),
    Path("/var/lib/flatpak/exports/share/applications"),
    Path("/var/lib/snapd/desktop/applications"),
]

# Category mappings (FreeDesktop standard categories -> display names)
CATEGORY_MAP = {
    "AudioVideo": "Media",
    "Audio": "Media",
    "Video": "Media",
    "Development": "Development",
    "Education": "Education",
    "Game": "Games",
    "Graphics": "Graphics",
    "Network": "Internet",
    "Office": "Office",
    "Science": "Science",
    "Settings": "Settings",
    "System": "System",
    "Utility": "Utilities",
}

# Category icons
CATEGORY_ICONS = {
    "All": "apps",
    "Media": "play_circle",
    "Development": "code",
    "Education": "school",
    "Games": "sports_esports",
    "Graphics": "palette",
    "Internet": "language",
    "Office": "business_center",
    "Science": "science",
    "Settings": "settings",
    "System": "computer",
    "Utilities": "build",
    "Other": "more_horiz",
}


def parse_desktop_file(path: Path) -> dict | None:
    """Parse a .desktop file and return app info"""
    try:
        config = ConfigParser(interpolation=None)
        config.read(path, encoding="utf-8")

        if not config.has_section("Desktop Entry"):
            return None

        entry = config["Desktop Entry"]

        # Skip hidden/non-application entries
        if entry.get("Type", "") != "Application":
            return None
        if entry.get("NoDisplay", "").lower() == "true":
            return None
        if entry.get("Hidden", "").lower() == "true":
            return None

        name = entry.get("Name", "")
        if not name:
            return None

        # Get categories
        categories_str = entry.get("Categories", "")
        categories = [c.strip() for c in categories_str.split(";") if c.strip()]

        # Map to display category
        display_category = "Other"
        for cat in categories:
            if cat in CATEGORY_MAP:
                display_category = CATEGORY_MAP[cat]
                break

        # Get exec command (strip field codes like %f, %u, etc.)
        exec_str = entry.get("Exec", "")
        # Remove field codes
        exec_clean = " ".join(p for p in exec_str.split() if not p.startswith("%"))

        # Parse desktop actions (e.g., "new-window;new-private-window;")
        actions_str = entry.get("Actions", "")
        action_ids = [a.strip() for a in actions_str.split(";") if a.strip()]
        desktop_actions = []
        for action_id in action_ids:
            section_name = f"Desktop Action {action_id}"
            if config.has_section(section_name):
                action_section = config[section_name]
                action_name = action_section.get("Name", action_id)
                action_exec = action_section.get("Exec", "")
                action_exec_clean = " ".join(
                    p for p in action_exec.split() if not p.startswith("%")
                )
                action_icon = action_section.get("Icon", "")
                if action_exec_clean:
                    desktop_actions.append(
                        {
                            "id": action_id,
                            "name": action_name,
                            "exec": action_exec_clean,
                            "icon": action_icon,
                        }
                    )

        return {
            "id": str(path),
            "name": name,
            "generic_name": entry.get("GenericName", ""),
            "comment": entry.get("Comment", ""),
            "icon": entry.get("Icon", "application-x-executable"),
            "exec": exec_clean,
            "categories": categories,
            "display_category": display_category,
            "keywords": entry.get("Keywords", ""),
            "terminal": entry.get("Terminal", "").lower() == "true",
            "actions": desktop_actions,
        }
    except Exception:
        return None


def load_all_apps() -> list[dict]:
    """Load all applications from .desktop files"""
    apps = {}  # Use dict to dedupe by name

    for app_dir in APP_DIRS:
        if not app_dir.exists():
            continue
        for desktop_file in app_dir.glob("*.desktop"):
            app = parse_desktop_file(desktop_file)
            if app:
                # Dedupe by name (prefer user's local apps)
                if app["name"] not in apps:
                    apps[app["name"]] = app

    return list(apps.values())


def load_app_frecency() -> dict[str, float]:
    """Load frecency scores from search history"""
    frecency = {}
    if not HISTORY_PATH.exists():
        return frecency

    try:
        with open(HISTORY_PATH) as f:
            data = json.load(f)
            history = data.get("history", [])
            now = __import__("time").time() * 1000

            for h in history:
                if h.get("type") != "app":
                    continue
                name = h.get("name", "")
                if not name:
                    continue

                # Calculate frecency
                hours_since = (now - h.get("lastUsed", 0)) / (1000 * 60 * 60)
                if hours_since < 1:
                    mult = 4
                elif hours_since < 24:
                    mult = 2
                elif hours_since < 168:
                    mult = 1
                else:
                    mult = 0.5
                frecency[name] = h.get("count", 1) * mult
    except Exception:
        pass

    return frecency


def fuzzy_match(query: str, text: str) -> bool:
    """Fuzzy match - query is substring or all chars appear in order with reasonable gaps"""
    query = query.lower()
    text = text.lower()

    # Direct substring match
    if query in text:
        return True

    # Fuzzy: all query chars appear in order, but penalize large gaps
    qi = 0
    last_match = -1
    max_gap = 5  # Max chars between matches

    for i, char in enumerate(text):
        if qi < len(query) and char == query[qi]:
            # Check gap from last match
            if last_match >= 0 and (i - last_match) > max_gap:
                return False
            last_match = i
            qi += 1

    return qi == len(query)


def app_to_result(app: dict, show_category: bool = False) -> dict:
    """Convert app info to result format"""
    description = app.get("generic_name") or app.get("comment") or ""
    if show_category and app.get("display_category"):
        if description:
            description = f"{app['display_category']} - {description}"
        else:
            description = app["display_category"]

    # Convert desktop actions to result actions format
    actions = []
    for action in app.get("actions", []):
        # Use material icons for action buttons (more reliable than system icons)
        # Guess icon based on action name/id
        action_name_lower = action["name"].lower()
        action_id_lower = action["id"].lower()

        if "private" in action_name_lower or "incognito" in action_name_lower:
            icon = "visibility_off"
        elif "window" in action_name_lower or "window" in action_id_lower:
            icon = "open_in_new"
        elif "quit" in action_name_lower or "quit" in action_id_lower:
            icon = "close"
        elif "compose" in action_name_lower or "message" in action_name_lower:
            icon = "edit"
        elif "address" in action_name_lower or "contact" in action_name_lower:
            icon = "contacts"
        else:
            icon = "play_arrow"

        actions.append(
            {
                "id": f"__action__:{app['id']}:{action['id']}",
                "name": action["name"],
                "icon": icon,
            }
        )

    result = {
        "id": app["id"],
        "name": app["name"],
        "description": description,
        "icon": app["icon"],
        "iconType": "system",  # App icons are system icons from .desktop files
        "verb": "Launch",
    }
    if actions:
        result["actions"] = actions
    return result


def get_categories(apps: list[dict]) -> list[str]:
    """Get sorted list of categories with apps"""
    categories = set()
    for app in apps:
        categories.add(app.get("display_category", "Other"))

    # Sort with common categories first
    priority = [
        "Internet",
        "Development",
        "Media",
        "Graphics",
        "Office",
        "Games",
        "System",
        "Utilities",
        "Settings",
        "Education",
        "Science",
        "Other",
    ]
    result = []
    for cat in priority:
        if cat in categories:
            result.append(cat)
            categories.discard(cat)
    result.extend(sorted(categories))
    return result


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    context = input_data.get("context", "")  # Current category

    selected_id = selected.get("id", "")

    # Load all apps
    all_apps = load_all_apps()
    frecency = load_app_frecency()

    # Sort apps by frecency then name
    def sort_key(app):
        return (-frecency.get(app["name"], 0), app["name"].lower())

    all_apps.sort(key=sort_key)

    # ===== INITIAL: Show categories =====
    if step == "initial":
        categories = get_categories(all_apps)
        results = [
            {
                "id": "__cat__:All",
                "name": "All Applications",
                "description": f"{len(all_apps)} apps",
                "icon": "apps",
            }
        ]
        for cat in categories:
            count = sum(1 for a in all_apps if a.get("display_category") == cat)
            results.append(
                {
                    "id": f"__cat__:{cat}",
                    "name": cat,
                    "description": f"{count} apps",
                    "icon": CATEGORY_ICONS.get(cat, "folder"),
                }
            )

        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                    "placeholder": "Search apps or select category...",
                }
            )
        )
        return

    # ===== SEARCH: Filter apps or categories =====
    if step == "search":
        # If in a category context, filter apps in that category
        if context and context.startswith("__cat__:"):
            category = context.replace("__cat__:", "")
            if category == "All":
                apps = all_apps
            else:
                apps = [a for a in all_apps if a.get("display_category") == category]

            # Filter by query
            if query:
                apps = [
                    a
                    for a in apps
                    if fuzzy_match(query, a["name"])
                    or fuzzy_match(query, a.get("generic_name", ""))
                    or fuzzy_match(query, a.get("keywords", ""))
                ]

            results = [
                app_to_result(a, show_category=(category == "All")) for a in apps[:50]
            ]

            if not results:
                results = [
                    {
                        "id": "__empty__",
                        "name": f"No apps found for '{query}'"
                        if query
                        else "No apps in this category",
                        "icon": "search_off",
                    }
                ]

            # Add back button
            results.insert(
                0,
                {
                    "id": "__back__",
                    "name": "Back to categories",
                    "icon": "arrow_back",
                },
            )

            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": results,
                        "inputMode": "realtime",
                        "placeholder": f"Search in {category}..."
                        if category != "All"
                        else "Search all apps...",
                        "context": context,
                    }
                )
            )
            return

        # Not in category context - search all or show categories
        if query:
            # Search all apps
            apps = [
                a
                for a in all_apps
                if fuzzy_match(query, a["name"])
                or fuzzy_match(query, a.get("generic_name", ""))
                or fuzzy_match(query, a.get("keywords", ""))
            ]

            results = [app_to_result(a, show_category=True) for a in apps[:50]]

            if not results:
                results = [
                    {
                        "id": "__empty__",
                        "name": f"No apps found for '{query}'",
                        "icon": "search_off",
                    }
                ]

            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": results,
                        "inputMode": "realtime",
                        "placeholder": "Search apps or select category...",
                    }
                )
            )
        else:
            # Show categories
            categories = get_categories(all_apps)
            results = [
                {
                    "id": "__cat__:All",
                    "name": "All Applications",
                    "description": f"{len(all_apps)} apps",
                    "icon": "apps",
                }
            ]
            for cat in categories:
                count = sum(1 for a in all_apps if a.get("display_category") == cat)
                results.append(
                    {
                        "id": f"__cat__:{cat}",
                        "name": cat,
                        "description": f"{count} apps",
                        "icon": CATEGORY_ICONS.get(cat, "folder"),
                    }
                )

            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": results,
                        "inputMode": "realtime",
                        "placeholder": "Search apps or select category...",
                    }
                )
            )
        return

    # ===== ACTION: Handle selection =====
    if step == "action":
        # Back button
        if selected_id == "__back__":
            categories = get_categories(all_apps)
            results = [
                {
                    "id": "__cat__:All",
                    "name": "All Applications",
                    "description": f"{len(all_apps)} apps",
                    "icon": "apps",
                }
            ]
            for cat in categories:
                count = sum(1 for a in all_apps if a.get("display_category") == cat)
                results.append(
                    {
                        "id": f"__cat__:{cat}",
                        "name": cat,
                        "description": f"{count} apps",
                        "icon": CATEGORY_ICONS.get(cat, "folder"),
                    }
                )

            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": results,
                        "inputMode": "realtime",
                        "placeholder": "Search apps or select category...",
                        "clearInput": True,
                        "context": "",  # Clear context
                    }
                )
            )
            return

        # Empty state - ignore
        if selected_id == "__empty__":
            return

        # Desktop action (e.g., "New Window", "New Private Window")
        if selected_id.startswith("__action__:"):
            # Format: __action__:<desktop_path>:<action_id>
            parts = selected_id.split(":", 2)
            if len(parts) == 3:
                desktop_path = parts[1]
                action_id = parts[2]

                # Find the app and action
                app = None
                for a in all_apps:
                    if a["id"] == desktop_path:
                        app = a
                        break

                if app:
                    # Find the specific action
                    action = None
                    for act in app.get("actions", []):
                        if act["id"] == action_id:
                            action = act
                            break

                    if action:
                        # Execute the action's command
                        exec_parts = action["exec"].split()
                        print(
                            json.dumps(
                                {
                                    "type": "execute",
                                    "execute": {
                                        "command": exec_parts,
                                        "name": f"{app['name']}: {action['name']}",
                                        "icon": action.get("icon") or app["icon"],
                                        "iconType": "system",
                                        "close": True,
                                    },
                                }
                            )
                        )
                        return

            print(json.dumps({"type": "error", "message": "Action not found"}))
            return

        # Category selection
        if selected_id.startswith("__cat__:"):
            category = selected_id.replace("__cat__:", "")
            if category == "All":
                apps = all_apps
            else:
                apps = [a for a in all_apps if a.get("display_category") == category]

            results = [
                {
                    "id": "__back__",
                    "name": "Back to categories",
                    "icon": "arrow_back",
                }
            ]
            results.extend(
                [app_to_result(a, show_category=(category == "All")) for a in apps[:50]]
            )

            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": results,
                        "inputMode": "realtime",
                        "placeholder": f"Search in {category}..."
                        if category != "All"
                        else "Search all apps...",
                        "clearInput": True,
                        "context": selected_id,  # Set category context
                    }
                )
            )
            return

        # App launch - selected_id is the .desktop file path
        app = None
        for a in all_apps:
            if a["id"] == selected_id:
                app = a
                break

        if app:
            # Use gtk-launch for proper .desktop handling
            desktop_name = Path(selected_id).stem
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["gtk-launch", desktop_name],
                            "name": f"Launch {app['name']}",
                            "icon": app["icon"],
                            "iconType": "system",  # App icons are system icons
                            "close": True,
                        },
                    }
                )
            )
        else:
            print(
                json.dumps(
                    {"type": "error", "message": f"App not found: {selected_id}"}
                )
            )


if __name__ == "__main__":
    main()
