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
import select
import signal
import subprocess
import sys
from configparser import ConfigParser
from pathlib import Path

# Add parent directory to path to import SDK
sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

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


def emit(data: dict) -> None:
    """Emit JSON response to stdout (line-buffered)."""
    print(json.dumps(data), flush=True)


def parse_desktop_file(path: Path) -> dict | None:
    """Parse a .desktop file and return app info"""
    try:
        config = ConfigParser(interpolation=None)
        config.read(path, encoding="utf-8")

        if not config.has_section("Desktop Entry"):
            return None

        entry = config["Desktop Entry"]

        if entry.get("Type", "") != "Application":
            return None
        if entry.get("NoDisplay", "").lower() == "true":
            return None
        if entry.get("Hidden", "").lower() == "true":
            return None

        name = entry.get("Name", "")
        if not name:
            return None

        categories_str = entry.get("Categories", "")
        categories = [c.strip() for c in categories_str.split(";") if c.strip()]

        display_category = "Other"
        for cat in categories:
            if cat in CATEGORY_MAP:
                display_category = CATEGORY_MAP[cat]
                break

        exec_str = entry.get("Exec", "")
        exec_clean = " ".join(p for p in exec_str.split() if not p.startswith("%"))

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

        # StartupWMClass is what the compositor uses as app_id (most reliable)
        # Desktop filename is fallback (e.g., "com.microsoft.Edge")
        startup_wm_class = entry.get("StartupWMClass", "")
        desktop_id = path.stem  # filename without .desktop

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
            "startup_wm_class": startup_wm_class,  # May be empty
            "desktop_id": desktop_id,  # Always present
        }
    except Exception:
        return None


def load_all_apps() -> list[dict]:
    """Load all applications from .desktop files"""
    apps = {}  # Use dict to dedupe by file path

    for app_dir in APP_DIRS:
        if not app_dir.exists():
            continue
        for desktop_file in app_dir.glob("*.desktop"):
            app = parse_desktop_file(desktop_file)
            if app:
                # Dedupe by file path (id) - each .desktop file is unique
                if app["id"] not in apps:
                    apps[app["id"]] = app

    return list(apps.values())


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


def app_to_index_item(app: dict) -> dict:
    """Convert app info to indexable item format for main search.

    Uses entryPoint for execution so handler controls the launch.
    """
    # Build keywords from name, generic name, comment, and keywords field
    keywords = []
    if app.get("generic_name"):
        keywords.extend(app["generic_name"].lower().split())
    if app.get("comment"):
        keywords.extend(app["comment"].lower().split()[:5])  # First 5 words
    if app.get("keywords"):
        keywords.extend(app["keywords"].lower().replace(";", " ").split())

    actions = []
    for action in app.get("actions", [])[:4]:  # Max 4 actions
        action_name_lower = action["name"].lower()
        action_id_lower = action["id"].lower()

        if "private" in action_name_lower or "incognito" in action_name_lower:
            icon = "visibility_off"
        elif "window" in action_name_lower or "window" in action_id_lower:
            icon = "open_in_new"
        elif "quit" in action_name_lower or "quit" in action_id_lower:
            icon = "close"
        else:
            icon = "play_arrow"

        actions.append(
            {
                "id": action["id"],
                "name": action["name"],
                "icon": icon,
                "entryPoint": {
                    "step": "action",
                    "selected": {"id": f"__action__:{app['id']}:{action['id']}"},
                },
            }
        )

    # Add "New Window" action to all apps (launches new instance)
    new_window_action = {
        "id": "new-window",
        "name": "New Window",
        "icon": "open_in_new",
        "entryPoint": {
            "step": "action",
            "selected": {"id": app["id"]},
        },
    }
    actions.insert(0, new_window_action)

    # Window matching fallback chain:
    # 1. StartupWMClass (most reliable when present)
    # 2. Desktop filename (e.g., "com.microsoft.Edge")
    # 3. No match -> launch new instance
    startup_wm_class = app.get("startup_wm_class", "")
    desktop_id = app.get("desktop_id", "")

    item = {
        "id": app["id"],  # Use full path as ID (matches result IDs for frecency)
        "name": app["name"],
        "description": app.get("generic_name") or app.get("display_category") or "",
        "keywords": keywords,
        "icon": app["icon"],
        "iconType": "system",
        "verb": "Open",
        "appId": startup_wm_class,  # Primary: StartupWMClass (may be empty)
        "appIdFallback": desktop_id,  # Fallback: desktop filename
        "entryPoint": {
            "step": "action",
            "selected": {"id": app["id"]},
        },
    }
    if actions:
        item["actions"] = actions

    return item


def app_to_result(app: dict, show_category: bool = False) -> dict:
    """Convert app info to result format"""
    description = app.get("generic_name") or app.get("comment") or ""
    if show_category and app.get("display_category"):
        if description:
            description = f"{app['display_category']} - {description}"
        else:
            description = app["display_category"]

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

    # Window matching fallback chain:
    # 1. StartupWMClass (most reliable when present)
    # 2. Desktop filename (e.g., "com.microsoft.Edge")
    startup_wm_class = app.get("startup_wm_class", "")
    desktop_id = app.get("desktop_id", "")

    result = {
        "id": app["id"],
        "name": app["name"],
        "description": description,
        "icon": app["icon"],
        "iconType": "system",  # App icons are system icons from .desktop files
        "verb": "Launch",
        "appId": startup_wm_class,  # Primary: StartupWMClass (may be empty)
        "appIdFallback": desktop_id,  # Fallback: desktop filename
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


def handle_request(request: dict, all_apps: list[dict]) -> None:
    """Handle incoming request from hamr."""
    step = request.get("step", "initial")
    query = request.get("query", "").strip()
    selected = request.get("selected", {})
    context = request.get("context", "")

    selected_id = selected.get("id", "")

    if step == "index":
        mode = request.get("mode", "full")
        indexed_ids = set(request.get("indexedIds", []))

        # Build current ID set (desktop filename without .desktop)
        current_ids = {
            f"app:{Path(app['id']).name.removesuffix('.desktop')}" for app in all_apps
        }

        if mode == "incremental" and indexed_ids:
            # Find new items
            new_ids = current_ids - indexed_ids
            new_items = [
                app_to_index_item(app)
                for app in all_apps
                if f"app:{Path(app['id']).name.removesuffix('.desktop')}" in new_ids
            ]

            # Find removed items
            removed_ids = list(indexed_ids - current_ids)

            emit(
                {
                    "type": "index",
                    "mode": "incremental",
                    "items": new_items,
                    "remove": removed_ids,
                }
            )
        else:
            # Full reindex
            items = [app_to_index_item(app) for app in all_apps]
            emit({"type": "index", "items": items})
        return

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

        emit(
            HamrPlugin.results(
                results,
                input_mode="realtime",
                placeholder="Search apps or select category...",
            )
        )
        return

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
                app_to_result(a, show_category=(category == "All")) for a in apps
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

            emit(
                HamrPlugin.results(
                    results,
                    input_mode="realtime",
                    placeholder=f"Search in {category}..."
                    if category != "All"
                    else "Search all apps...",
                    context=context,
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

            results = [app_to_result(a, show_category=True) for a in apps]

            if not results:
                results = [
                    {
                        "id": "__empty__",
                        "name": f"No apps found for '{query}'",
                        "icon": "search_off",
                    }
                ]

            emit(
                HamrPlugin.results(
                    results,
                    input_mode="realtime",
                    placeholder="Search apps or select category...",
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

            emit(
                HamrPlugin.results(
                    results,
                    input_mode="realtime",
                    placeholder="Search apps or select category...",
                )
            )
        return

    if step == "action":
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

            response = HamrPlugin.results(
                results,
                input_mode="realtime",
                placeholder="Search apps or select category...",
                clear_input=True,
                context="",
            )
            response["navigateBack"] = True
            emit(response)
            return

        if selected_id == "__empty__":
            return

        if selected_id.startswith("__action__:"):
            # Format: __action__:<desktop_path>:<action_id>
            parts = selected_id.split(":", 2)
            if len(parts) == 3:
                desktop_path = parts[1]
                action_id = parts[2]

                app = None
                for a in all_apps:
                    if a["id"] == desktop_path:
                        app = a
                        break

                if app:
                    action = None
                    for act in app.get("actions", []):
                        if act["id"] == action_id:
                            action = act
                            break

                    if action and action.get("exec"):
                        # Execute desktop action command in handler
                        exec_parts = action["exec"].split()
                        subprocess.Popen(
                            exec_parts,
                            stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL,
                            start_new_session=True,
                        )
                        response = HamrPlugin.close()
                        response["name"] = f"{app['name']}: {action['name']}"
                        response["icon"] = action.get("icon") or app["icon"]
                        response["iconType"] = "system"
                        emit(response)
                        return

            emit(HamrPlugin.error("Action not found"))
            return

        if selected_id.startswith("__cat__:"):
            category = selected_id.replace("__cat__:", "")
            if category == "All":
                apps = all_apps
            else:
                apps = [a for a in all_apps if a.get("display_category") == category]

            results = [
                app_to_result(a, show_category=(category == "All")) for a in apps
            ]

            response = HamrPlugin.results(
                results,
                input_mode="realtime",
                placeholder=f"Search in {category}..."
                if category != "All"
                else "Search all apps...",
                clear_input=True,
                context=selected_id,
                navigate_forward=True,
            )
            emit(response)
            return

        app = None
        for a in all_apps:
            if a["id"] == selected_id:
                app = a
                break

        if app:
            # Use safe launch API - hamr will run gio launch
            response = HamrPlugin.execute(launch=selected_id, close=True)
            response["name"] = f"Launch {app['name']}"
            response["icon"] = app["icon"]
            response["iconType"] = "system"
            emit(response)
        else:
            emit(HamrPlugin.error(f"App not found: {selected_id}"))


def main():
    def shutdown_handler(signum, frame):
        sys.exit(0)

    signal.signal(signal.SIGTERM, shutdown_handler)
    signal.signal(signal.SIGINT, shutdown_handler)

    # Load apps once at startup
    all_apps = load_all_apps()

    # Sort apps alphabetically (frecency handled by hamr's unified system)
    all_apps.sort(key=lambda app: app["name"].lower())

    # Emit initial full index on startup (for background daemon)
    items = [app_to_index_item(app) for app in all_apps]
    emit({"type": "index", "mode": "full", "items": items})

    # Main daemon loop - read requests from stdin
    while True:
        readable, _, _ = select.select([sys.stdin], [], [], 1.0)

        if readable:
            try:
                line = sys.stdin.readline()
                if not line:
                    break
                request = json.loads(line.strip())
                handle_request(request, all_apps)
            except (json.JSONDecodeError, ValueError):
                continue


if __name__ == "__main__":
    main()
