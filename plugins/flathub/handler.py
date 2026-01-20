#!/usr/bin/env python3
"""
Flathub plugin - Search and install apps from Flathub.

Features:
- Search Flathub for apps
- Install apps (non-blocking with notifications)
- Uninstall installed apps
- Open app page on Flathub website
- Detect already installed apps
"""

import hashlib
import json
import os
import subprocess
import sys
import time
import urllib.request
import urllib.error
from pathlib import Path

# Add parent directory to path to import SDK
sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

FLATHUB_API = "https://flathub.org/api/v2/search"
FLATHUB_WEB = "https://flathub.org/apps"
CACHE_DIR = (
    Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "hamr" / "flathub"
)
CACHE_TTL = 3600


def get_cache_path(query: str) -> Path:
    """Get cache file path for a query"""
    query_hash = hashlib.md5(query.lower().encode()).hexdigest()
    return CACHE_DIR / f"{query_hash}.json"


def get_cached_results(query: str) -> list[dict] | None:
    """Get cached search results if valid"""
    cache_path = get_cache_path(query)
    if not cache_path.exists():
        return None

    try:
        with open(cache_path) as f:
            cached = json.load(f)

        if time.time() - cached.get("timestamp", 0) < CACHE_TTL:
            return cached.get("results", [])
    except Exception:
        pass

    return None


def save_cached_results(query: str, results: list[dict]) -> None:
    """Save search results to cache"""
    try:
        CACHE_DIR.mkdir(parents=True, exist_ok=True)
        cache_path = get_cache_path(query)
        with open(cache_path, "w") as f:
            json.dump({"timestamp": time.time(), "results": results}, f)
    except Exception:
        pass


ICON_DIRS = [
    Path("/var/lib/flatpak/exports/share/icons"),
    Path.home() / ".local/share/flatpak/exports/share/icons",
]
ICON_SIZES = ["128x128", "scalable", "64x64", "48x48", "256x256", "512x512"]


def get_app_icon(app_id: str) -> str:
    """Find icon path for a flatpak app"""
    for icon_dir in ICON_DIRS:
        for size in ICON_SIZES:
            for ext in ["png", "svg"]:
                icon_path = icon_dir / "hicolor" / size / "apps" / f"{app_id}.{ext}"
                if icon_path.exists():
                    return f"file://{icon_path}"
    return ""


def get_installed_apps() -> list[dict]:
    """Get list of installed Flatpak apps with details"""
    try:
        result = subprocess.run(
            ["flatpak", "list", "--app", "--columns=application,name,description"],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            apps = []
            for line in result.stdout.strip().split("\n"):
                if not line:
                    continue
                parts = line.split("\t")
                if len(parts) >= 1:
                    app_id = parts[0]
                    apps.append(
                        {
                            "app_id": app_id,
                            "name": parts[1] if len(parts) > 1 else app_id,
                            "summary": parts[2] if len(parts) > 2 else "",
                            "icon": get_app_icon(app_id),
                        }
                    )
            return apps
    except Exception:
        pass
    return []


def get_installed_app_ids() -> set[str]:
    """Get set of installed Flatpak app IDs"""
    try:
        result = subprocess.run(
            ["flatpak", "list", "--app", "--columns=application"],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            return set(result.stdout.strip().split("\n")) - {""}
    except Exception:
        pass
    return set()


def search_flathub(query: str) -> list[dict]:
    """Search Flathub API for apps with caching"""
    cached = get_cached_results(query)
    if cached is not None:
        return cached

    try:
        data = json.dumps({"query": query}).encode("utf-8")
        req = urllib.request.Request(
            FLATHUB_API,
            data=data,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=10) as response:
            result = json.loads(response.read().decode("utf-8"))
            hits = result.get("hits", [])
            save_cached_results(query, hits)
            return hits
    except Exception:
        return []


def format_installs(count: int) -> str:
    """Format install count for display"""
    if count >= 1_000_000:
        return f"{count / 1_000_000:.1f}M"
    if count >= 1_000:
        return f"{count / 1_000:.1f}K"
    return str(count)


def app_to_result(app: dict, installed_apps: set[str]) -> dict:
    """Convert Flathub app to result format"""
    app_id = app.get("app_id", "")
    is_installed = app_id in installed_apps
    installs = app.get("installs_last_month", 0)
    verified = app.get("verification_verified", False)
    developer = app.get("developer_name", "")

    description = app.get("summary", "")

    badges = []
    if verified:
        badges.append({"icon": "verified", "color": "#4caf50"})
    if is_installed:
        badges.append({"icon": "check_circle", "color": "#2196f3"})

    chips = []
    if developer:
        chips.append({"text": developer, "icon": "business"})
    if installs:
        chips.append({"text": f"{format_installs(installs)}/mo", "icon": "download"})

    actions = []
    if is_installed:
        actions.append({"id": "uninstall", "name": "Uninstall", "icon": "delete"})
    else:
        actions.append({"id": "install", "name": "Install", "icon": "download"})
    actions.append({"id": "open_web", "name": "View on Flathub", "icon": "open_in_new"})

    result = {
        "id": app_id,
        "name": app.get("name", app_id),
        "description": description,
        "verb": "Open" if is_installed else "Install",
        "actions": actions,
    }

    if badges:
        result["badges"] = badges
    if chips:
        result["chips"] = chips

    return result


def get_plugin_actions() -> list[dict]:
    """Get plugin-level actions for the action bar"""
    return [
        {
            "id": "search_new",
            "name": "Install New",
            "icon": "add_circle",
            "shortcut": "Ctrl+1",
        }
    ]


# Create plugin instance
plugin = HamrPlugin(
    id="flathub",
    name="Flathub",
    description="Search and install apps from Flathub",
    icon="deployed_code",
)


@plugin.on_initial
async def handle_initial(params=None):
    """Handle initial request when plugin is opened."""
    installed_apps = get_installed_apps()

    results = []
    for app in installed_apps:
        app_id = app.get("app_id", "")
        result = {
            "id": app_id,
            "name": app.get("name", app_id),
            "description": app.get("summary", "Installed"),
            "verb": "Open",
            "actions": [
                {"id": "uninstall", "name": "Uninstall", "icon": "delete"},
                {
                    "id": "open_web",
                    "name": "View on Flathub",
                    "icon": "open_in_new",
                },
            ],
        }
        icon = app.get("icon", "")
        if icon:
            result["thumbnail"] = icon
        results.append(result)

    if not results:
        results = [
            {
                "id": "__empty__",
                "name": "No Flatpak apps installed",
                "description": "Type to search Flathub for apps",
                "icon": "search",
            }
        ]

    return HamrPlugin.results(
        results,
        input_mode="realtime",
        placeholder="Search installed apps...",
        plugin_actions=get_plugin_actions(),
    )


@plugin.on_search
async def handle_search(query: str, context: str | None):
    """Handle search request."""
    # Search mode: searching for new apps to install
    if context == "__search_new__":
        if not query or len(query) < 2:
            return HamrPlugin.results(
                [
                    {
                        "id": "__prompt__",
                        "name": "Search Flathub",
                        "description": "Type at least 2 characters to search",
                        "icon": "search",
                    }
                ],
                input_mode="realtime",
                placeholder="Search Flathub for new apps...",
                context="__search_new__",
                plugin_actions=[],
            )

        apps = search_flathub(query)
        installed_app_ids = get_installed_app_ids()

        if not apps:
            return HamrPlugin.results(
                [
                    {
                        "id": "__empty__",
                        "name": "No apps found",
                        "description": f"No results for '{query}'",
                        "icon": "search_off",
                    }
                ],
                input_mode="realtime",
                placeholder="Search Flathub for new apps...",
                context="__search_new__",
                plugin_actions=[],
            )

        results = [app_to_result(app, installed_app_ids) for app in apps[:15]]
        return HamrPlugin.results(
            results,
            input_mode="realtime",
            placeholder="Search Flathub for new apps...",
            context="__search_new__",
            plugin_actions=[],
        )

    # Default search: filter installed apps
    installed_apps = get_installed_apps()

    # Filter installed apps by query
    if query:
        query_lower = query.lower()
        installed_apps = [
            app
            for app in installed_apps
            if query_lower in app.get("name", "").lower()
            or query_lower in app.get("app_id", "").lower()
        ]

    results = []
    for app in installed_apps:
        app_id = app.get("app_id", "")
        result = {
            "id": app_id,
            "name": app.get("name", app_id),
            "description": app.get("summary", "Installed"),
            "verb": "Open",
            "actions": [
                {"id": "uninstall", "name": "Uninstall", "icon": "delete"},
                {
                    "id": "open_web",
                    "name": "View on Flathub",
                    "icon": "open_in_new",
                },
            ],
        }
        icon = app.get("icon", "")
        if icon:
            result["thumbnail"] = icon
        results.append(result)

    if not results and query:
        results = [
            {
                "id": "__empty__",
                "name": f"No installed apps match '{query}'",
                "description": "Use Ctrl+1 to search Flathub for new apps",
                "icon": "search_off",
            }
        ]

    return HamrPlugin.results(
        results,
        input_mode="realtime",
        placeholder="Search installed apps...",
        plugin_actions=get_plugin_actions(),
    )


@plugin.on_action
async def handle_action(item_id: str, action: str | None, context: str | None):
    """Handle action request."""
    if item_id in ("__prompt__", "__empty__"):
        return {}

    # Plugin-level action: Install New
    if item_id == "__plugin__" and action == "search_new":
        return HamrPlugin.results(
            [
                {
                    "id": "__prompt__",
                    "name": "Search Flathub",
                    "description": "Type to search for new apps to install",
                    "icon": "search",
                }
            ],
            input_mode="realtime",
            placeholder="Search Flathub for new apps...",
            context="__search_new__",
            clear_input=True,
            navigate_forward=True,
        )

    # Back navigation
    if item_id == "__back__":
        installed_apps = get_installed_apps()

        results = []
        for app in installed_apps:
            app_id = app.get("app_id", "")
            result = {
                "id": app_id,
                "name": app.get("name", app_id),
                "description": app.get("summary", "Installed"),
                "verb": "Open",
                "actions": [
                    {"id": "uninstall", "name": "Uninstall", "icon": "delete"},
                    {
                        "id": "open_web",
                        "name": "View on Flathub",
                        "icon": "open_in_new",
                    },
                ],
            }
            icon = app.get("icon", "")
            if icon:
                result["thumbnail"] = icon
            results.append(result)

        if not results:
            results = [
                {
                    "id": "__empty__",
                    "name": "No Flatpak apps installed",
                    "description": "Use Ctrl+1 to search Flathub for apps",
                    "icon": "search",
                }
            ]

        return HamrPlugin.results(
            results,
            input_mode="realtime",
            placeholder="Search installed apps...",
            context="",
            clear_input=True,
            plugin_actions=get_plugin_actions(),
            navigation_depth=0,
        )

    installed_app_ids = get_installed_app_ids()
    is_installed = item_id in installed_app_ids

    # Uninstall action
    if action == "uninstall":
        app_name = ""
        try:
            cmd = (
                f'notify-send "Flathub" "Uninstalling {app_name}..." -a "Hamr" && '
                f"(flatpak uninstall --user -y {item_id} 2>/dev/null || flatpak uninstall -y {item_id})"
            )
            subprocess.Popen(
                ["bash", "-c", cmd],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            return HamrPlugin.close()
        except Exception as e:
            return {}

    # Open on Flathub website
    if action == "open_web":
        return HamrPlugin.open_url(f"{FLATHUB_WEB}/{item_id}", close=True)

    # Install action
    if action == "install":
        try:
            cmd = (
                f'notify-send "Flathub" "Installing {item_id}..." -a "Hamr" && '
                f"(flatpak install --user -y flathub {item_id} 2>/dev/null || flatpak install -y flathub {item_id})"
            )
            subprocess.Popen(
                ["bash", "-c", cmd],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            return HamrPlugin.close()
        except Exception as e:
            return {}

    # Default action: Install or Open
    if is_installed:
        # Open the installed app
        try:
            subprocess.Popen(
                ["flatpak", "run", item_id],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            return HamrPlugin.close()
        except Exception:
            return {}
    else:
        # Install the app
        try:
            cmd = (
                f'notify-send "Flathub" "Installing {item_id}..." -a "Hamr" && '
                f"(flatpak install --user -y flathub {item_id} 2>/dev/null || flatpak install -y flathub {item_id})"
            )
            subprocess.Popen(
                ["bash", "-c", cmd],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            return HamrPlugin.close()
        except Exception:
            return {}


if __name__ == "__main__":
    plugin.run()
