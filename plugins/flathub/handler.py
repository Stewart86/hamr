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

TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

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
    if TEST_MODE:
        return [
            {
                "app_id": "org.mozilla.firefox",
                "name": "Firefox",
                "summary": "Fast, Private & Safe Web Browser",
                "icon": "https://dl.flathub.org/repo/appstream/x86_64/icons/128x128/org.mozilla.firefox.png",
                "developer_name": "Mozilla",
                "installs_last_month": 314667,
                "verification_verified": True,
            },
            {
                "app_id": "org.videolan.VLC",
                "name": "VLC",
                "summary": "VLC media player",
                "icon": "https://dl.flathub.org/repo/appstream/x86_64/icons/128x128/org.videolan.VLC.png",
                "developer_name": "VideoLAN",
                "installs_last_month": 200000,
                "verification_verified": True,
            },
        ]

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
        "thumbnail": app.get("icon", ""),
        "verb": "Open" if is_installed else "Install",
        "actions": actions,
    }

    if badges:
        result["badges"] = badges
    if chips:
        result["chips"] = chips

    return result


def get_plugin_actions(in_search_mode: bool = False) -> list[dict]:
    """Get plugin-level actions for the action bar"""
    if in_search_mode:
        return []
    return [
        {
            "id": "search_new",
            "name": "Install New",
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

    selected_id = selected.get("id", "")

    # Initial: show installed apps
    if step == "initial":
        installed_apps = get_installed_apps()
        if TEST_MODE:
            installed_apps = [
                {
                    "app_id": "org.mozilla.firefox",
                    "name": "Firefox",
                    "summary": "Web Browser",
                    "icon": "file:///var/lib/flatpak/exports/share/icons/hicolor/128x128/apps/org.mozilla.firefox.png",
                },
                {
                    "app_id": "org.videolan.VLC",
                    "name": "VLC",
                    "summary": "Media Player",
                    "icon": "file:///var/lib/flatpak/exports/share/icons/hicolor/128x128/apps/org.videolan.VLC.png",
                },
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

        if not results:
            results = [
                {
                    "id": "__empty__",
                    "name": "No Flatpak apps installed",
                    "description": "Type to search Flathub for apps",
                    "icon": "search",
                }
            ]

        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                    "placeholder": "Search installed apps...",
                    "pluginActions": get_plugin_actions(),
                }
            )
        )
        return

    # Search mode: searching for new apps to install
    if step == "search" and context == "__search_new__":
        if not query or len(query) < 2:
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": [
                            {
                                "id": "__prompt__",
                                "name": "Search Flathub",
                                "description": "Type at least 2 characters to search",
                                "icon": "search",
                            }
                        ],
                        "inputMode": "realtime",
                        "placeholder": "Search Flathub for new apps...",
                        "context": "__search_new__",
                        "pluginActions": get_plugin_actions(in_search_mode=True),
                    }
                )
            )
            return

        apps = search_flathub(query)
        installed_app_ids = get_installed_app_ids()

        if not apps:
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": [
                            {
                                "id": "__empty__",
                                "name": "No apps found",
                                "description": f"No results for '{query}'",
                                "icon": "search_off",
                            }
                        ],
                        "inputMode": "realtime",
                        "placeholder": "Search Flathub for new apps...",
                        "context": "__search_new__",
                        "pluginActions": get_plugin_actions(in_search_mode=True),
                    }
                )
            )
            return

        results = [app_to_result(app, installed_app_ids) for app in apps[:15]]
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                    "placeholder": "Search Flathub for new apps...",
                    "context": "__search_new__",
                    "pluginActions": get_plugin_actions(in_search_mode=True),
                }
            )
        )
        return

    # Default search: filter installed apps
    if step == "search":
        installed_apps = get_installed_apps()
        if TEST_MODE:
            installed_apps = [
                {
                    "app_id": "org.mozilla.firefox",
                    "name": "Firefox",
                    "summary": "Web Browser",
                    "icon": "file:///var/lib/flatpak/exports/share/icons/hicolor/128x128/apps/org.mozilla.firefox.png",
                },
                {
                    "app_id": "org.videolan.VLC",
                    "name": "VLC",
                    "summary": "Media Player",
                    "icon": "file:///var/lib/flatpak/exports/share/icons/hicolor/128x128/apps/org.videolan.VLC.png",
                },
            ]

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

        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                    "placeholder": "Search installed apps...",
                    "pluginActions": get_plugin_actions(),
                }
            )
        )
        return

    # Action handling
    if step == "action":
        if selected_id in ("__prompt__", "__empty__"):
            return

        # Plugin-level action: Install New
        if selected_id == "__plugin__" and action == "search_new":
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": [
                            {
                                "id": "__prompt__",
                                "name": "Search Flathub",
                                "description": "Type to search for new apps to install",
                                "icon": "search",
                            }
                        ],
                        "inputMode": "realtime",
                        "placeholder": "Search Flathub for new apps...",
                        "context": "__search_new__",
                        "clearInput": True,
                        "pluginActions": get_plugin_actions(in_search_mode=True),
                        "navigateForward": True,
                    }
                )
            )
            return

        # Back navigation
        if selected_id == "__back__":
            installed_apps = get_installed_apps()
            if TEST_MODE:
                installed_apps = [
                    {
                        "app_id": "org.mozilla.firefox",
                        "name": "Firefox",
                        "summary": "Web Browser",
                        "icon": "file:///var/lib/flatpak/exports/share/icons/hicolor/128x128/apps/org.mozilla.firefox.png",
                    },
                    {
                        "app_id": "org.videolan.VLC",
                        "name": "VLC",
                        "summary": "Media Player",
                        "icon": "file:///var/lib/flatpak/exports/share/icons/hicolor/128x128/apps/org.videolan.VLC.png",
                    },
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

            if not results:
                results = [
                    {
                        "id": "__empty__",
                        "name": "No Flatpak apps installed",
                        "description": "Use Ctrl+1 to search Flathub for apps",
                        "icon": "search",
                    }
                ]

            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": results,
                        "inputMode": "realtime",
                        "placeholder": "Search installed apps...",
                        "context": "",
                        "clearInput": True,
                        "pluginActions": get_plugin_actions(),
                        "navigationDepth": 0,
                    }
                )
            )
            return

        installed_app_ids = get_installed_app_ids()
        is_installed = selected_id in installed_app_ids
        app_name = selected.get("name", selected_id)

        # Uninstall action
        if action == "uninstall":
            try:
                cmd = (
                    f'notify-send "Flathub" "Uninstalling {app_name}..." -a "Hamr" && '
                    f"(flatpak uninstall --user -y {selected_id} 2>/dev/null || flatpak uninstall -y {selected_id}) && "
                    f'notify-send "Flathub" "{app_name} uninstalled" -a "Hamr" && '
                    f"qs -c hamr ipc call pluginRunner reindex apps || "
                    f'notify-send "Flathub" "Failed to uninstall {app_name}" -a "Hamr"'
                )
                subprocess.Popen(["bash", "-c", cmd])
                print(json.dumps({"type": "close"}))
            except Exception as e:
                print(json.dumps({"type": "error", "message": f"Failed to uninstall: {str(e)}"}))
            return

        # Open on Flathub website
        if action == "open_web":
            print(json.dumps({"type": "execute", "openUrl": f"{FLATHUB_WEB}/{selected_id}", "close": True}))
            return

        # Install action
        if action == "install":
            try:
                cmd = (
                    f'notify-send "Flathub" "Installing {app_name}..." -a "Hamr" && '
                    f"(flatpak install --user -y flathub {selected_id} 2>/dev/null || flatpak install -y flathub {selected_id}) && "
                    f'notify-send "Flathub" "{app_name} installed" -a "Hamr" && '
                    f"qs -c hamr ipc call pluginRunner reindex apps || "
                    f'notify-send "Flathub" "Failed to install {app_name}" -a "Hamr"'
                )
                subprocess.Popen(["bash", "-c", cmd])
                print(json.dumps({"type": "close"}))
            except Exception as e:
                print(json.dumps({"type": "error", "message": f"Failed to install: {str(e)}"}))
            return

        # Default action: Install or Open
        if is_installed:
            # Open the installed app
            try:
                subprocess.Popen(["flatpak", "run", selected_id])
                print(json.dumps({"type": "close"}))
            except Exception as e:
                print(json.dumps({"type": "error", "message": f"Failed to open app: {str(e)}"}))
        else:
            # Install the app (non-blocking with notifications)
            # Try user install first, fall back to system install
            try:
                cmd = (
                    f'notify-send "Flathub" "Installing {app_name}..." -a "Hamr" && '
                    f"(flatpak install --user -y flathub {selected_id} 2>/dev/null || flatpak install -y flathub {selected_id}) && "
                    f'notify-send "Flathub" "{app_name} installed" -a "Hamr" && '
                    f"qs -c hamr ipc call pluginRunner reindex apps || "
                    f'notify-send "Flathub" "Failed to install {app_name}" -a "Hamr"'
                )
                subprocess.Popen(["bash", "-c", cmd])
                print(json.dumps({"type": "close"}))
            except Exception as e:
                print(json.dumps({"type": "error", "message": f"Failed to install: {str(e)}"}))
        return


if __name__ == "__main__":
    main()
