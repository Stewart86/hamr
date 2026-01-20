#!/usr/bin/env python3
"""
Web Apps plugin for hamr - Install and manage web apps.

Socket-based daemon plugin that stores web apps in ~/.config/hamr/webapps.json
and launches them in app mode (standalone browser window) via the bundled
launch-webapp script.

Features:
- Install web apps from URL + icon
- Browse and search installed web apps
- Launch web apps in standalone browser window
- Delete web apps
- Index support for main search integration
- Daemon mode with file watching for automatic reindexing
"""

import asyncio
import json
import subprocess
import sys
from pathlib import Path
from uuid import uuid4

sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

CONFIG_DIR = Path.home() / ".config/hamr"
WEBAPPS_FILE = CONFIG_DIR / "webapps.json"
ICONS_DIR = CONFIG_DIR / "webapp-icons"
PLUGIN_DIR = Path(__file__).parent
LAUNCHER_SCRIPT = PLUGIN_DIR / "launch-webapp"


def ensure_dirs():
    """Ensure required directories exist."""
    CONFIG_DIR.mkdir(parents=True, exist_ok=True)
    ICONS_DIR.mkdir(parents=True, exist_ok=True)


def load_webapps() -> list[dict]:
    """Load web apps from config file."""
    if not WEBAPPS_FILE.exists():
        return []
    try:
        with open(WEBAPPS_FILE) as f:
            data = json.load(f)
            return data.get("webapps", [])
    except Exception:
        return []


def save_webapps(webapps: list[dict]) -> bool:
    """Save web apps to config file."""
    try:
        ensure_dirs()
        with open(WEBAPPS_FILE, "w") as f:
            json.dump({"webapps": webapps}, f, indent=2)
        return True
    except Exception:
        return False


def sanitize_name(name: str) -> str:
    """Sanitize app name for use in filenames."""
    safe = "".join(c if c.isalnum() else "-" for c in name)
    while "--" in safe:
        safe = safe.replace("--", "-")
    return safe.strip("-").lower()


async def download_icon(url: str, name: str) -> str | None:
    """Download icon from URL, return local path or None on failure."""
    ensure_dirs()
    icon_path = ICONS_DIR / f"{sanitize_name(name)}.png"

    try:
        result = await asyncio.to_thread(
            subprocess.run,
            ["curl", "-sL", "-o", str(icon_path), url],
            capture_output=True,
            timeout=30,
        )
        if (
            result.returncode == 0
            and icon_path.exists()
            and icon_path.stat().st_size > 0
        ):
            return str(icon_path)
    except Exception:
        pass

    # Cleanup failed download
    if icon_path.exists():
        icon_path.unlink()
    return None


def delete_icon(name: str):
    """Delete icon file for a web app."""
    icon_path = ICONS_DIR / f"{sanitize_name(name)}.png"
    if icon_path.exists():
        icon_path.unlink()


def get_plugin_actions() -> list[dict]:
    """Get plugin-level actions."""
    return [
        {
            "id": "add",
            "name": "Install Web App",
            "icon": "add_circle",
            "shortcut": "Ctrl+1",
        }
    ]


def get_webapp_results(webapps: list[dict]) -> list[dict]:
    """Convert webapps to result format."""
    results = []
    for app in webapps:
        icon_path = app.get("icon", "")
        results.append(
            {
                "id": app["id"],
                "name": app["name"],
                "description": app["url"],
                "thumbnail": icon_path
                if icon_path and Path(icon_path).exists()
                else None,
                "icon": "web"
                if not icon_path or not Path(icon_path).exists()
                else None,
                "verb": "Launch",
                "actions": [
                    {
                        "id": "floating",
                        "name": "Open Floating",
                        "icon": "picture_in_picture",
                    },
                    {"id": "edit", "name": "Edit", "icon": "edit"},
                    {"id": "delete", "name": "Delete", "icon": "delete"},
                ],
            }
        )
    return results


def get_empty_results() -> list[dict]:
    """Return empty state results."""
    return [
        {
            "id": "__empty__",
            "name": "No web apps installed",
            "icon": "info",
            "description": "Use 'Install Web App' button or Ctrl+1",
        }
    ]


def webapp_to_index_item(app: dict) -> dict:
    """Convert webapp to index item for main search."""
    icon_path = app.get("icon", "")
    has_icon = icon_path and Path(icon_path).exists()

    return {
        "id": app["id"],
        "name": app["name"],
        "description": app["url"],
        "keywords": app["name"].lower().split(),
        "icon": None if has_icon else "web",
        "thumbnail": icon_path if has_icon else None,
        "verb": "Launch",
        "entryPoint": {
            "step": "action",
            "selected": {"id": app["id"]},
        },
        "actions": [
            {
                "id": "floating",
                "name": "Open Floating",
                "icon": "picture_in_picture",
                "entryPoint": {
                    "step": "action",
                    "selected": {"id": app["id"]},
                    "action": "floating",
                },
            },
            {
                "id": "delete",
                "name": "Delete",
                "icon": "delete",
                "entryPoint": {
                    "step": "action",
                    "selected": {"id": app["id"]},
                    "action": "delete",
                },
            },
        ],
    }


plugin = HamrPlugin(
    id="webapp",
    name="Web Apps",
    description="Install and manage web apps",
    icon="web",
)


@plugin.on_initial
async def handle_initial(params=None):
    """Handle initial request."""
    webapps = load_webapps()
    results = get_webapp_results(webapps) if webapps else get_empty_results()
    return HamrPlugin.results(
        results,
        input_mode="realtime",
        placeholder="Search web apps...",
        plugin_actions=get_plugin_actions(),
    )


@plugin.on_search
async def handle_search(query: str, context: str | None):
    """Handle search request."""
    webapps = load_webapps()

    if query:
        query_lower = query.lower()
        filtered = [
            app
            for app in webapps
            if query_lower in app["name"].lower()
            or query_lower in app.get("url", "").lower()
        ]
    else:
        filtered = webapps

    results = (
        get_webapp_results(filtered)
        if filtered
        else [
            {
                "id": "__empty__",
                "name": "No matching web apps",
                "icon": "search_off",
            }
        ]
    )

    return HamrPlugin.results(
        results,
        input_mode="realtime",
        placeholder="Search web apps...",
        plugin_actions=get_plugin_actions(),
    )


@plugin.on_action
async def handle_action(item_id: str, action: str | None, context: str | None):
    """Handle action request."""
    webapps = load_webapps()

    if item_id == "__plugin__" and action == "add":
        return HamrPlugin.form(
            {
                "title": "Install Web App",
                "submitLabel": "Install",
                "cancelLabel": "Cancel",
                "fields": [
                    {
                        "id": "name",
                        "type": "text",
                        "label": "App Name",
                        "placeholder": "My Favorite Web App",
                        "required": True,
                    },
                    {
                        "id": "url",
                        "type": "text",
                        "label": "URL",
                        "placeholder": "https://example.com",
                        "required": True,
                    },
                    {
                        "id": "icon_url",
                        "type": "text",
                        "label": "Icon URL",
                        "placeholder": "https://example.com/icon.png",
                        "required": True,
                        "hint": "PNG icon URL (try dashboardicons.com)",
                    },
                ],
            },
            context="__add__",
        )

    if item_id == "__form_cancel__":
        results = get_webapp_results(webapps) if webapps else get_empty_results()
        return HamrPlugin.results(
            results,
            input_mode="realtime",
            clear_input=True,
        )

    if action == "edit":
        app = next((a for a in webapps if a["id"] == item_id), None)
        if app:
            return HamrPlugin.form(
                {
                    "title": f"Edit {app['name']}",
                    "submitLabel": "Save",
                    "cancelLabel": "Cancel",
                    "fields": [
                        {
                            "id": "name",
                            "type": "text",
                            "label": "App Name",
                            "default": app["name"],
                            "required": True,
                        },
                        {
                            "id": "url",
                            "type": "text",
                            "label": "URL",
                            "default": app["url"],
                            "required": True,
                        },
                        {
                            "id": "icon_url",
                            "type": "text",
                            "label": "Icon URL (leave empty to keep current)",
                            "required": False,
                            "hint": "Leave empty to keep current icon",
                        },
                    ],
                },
                context=f"__edit__:{app['id']}",
            )

    if action == "floating":
        app = next((a for a in webapps if a["id"] == item_id), None)
        if app:
            try:
                await asyncio.to_thread(
                    subprocess.Popen,
                    [str(LAUNCHER_SCRIPT), "--floating", app["url"]],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                )
                return HamrPlugin.close()
            except Exception:
                return HamrPlugin.error("Failed to launch app")

    if action == "delete":
        app = next((a for a in webapps if a["id"] == item_id), None)
        if app:
            delete_icon(app["name"])
            webapps = [a for a in webapps if a["id"] != item_id]
            save_webapps(webapps)

        results = get_webapp_results(webapps) if webapps else get_empty_results()
        return HamrPlugin.results(
            results,
            input_mode="realtime",
            clear_input=True,
            placeholder="Search web apps...",
            plugin_actions=get_plugin_actions(),
        )

    # Launch web app (default action)
    app = next((a for a in webapps if a["id"] == item_id), None)
    if app:
        try:
            await asyncio.to_thread(
                subprocess.Popen,
                [str(LAUNCHER_SCRIPT), app["url"]],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            return HamrPlugin.close()
        except Exception:
            return HamrPlugin.error("Failed to launch app")

    return {}


@plugin.on_form_submitted
async def handle_form_submitted(form_data: dict, context: str | None):
    """Handle form submission."""
    webapps = load_webapps()

    if context == "__add__":
        name = form_data.get("name", "").strip()
        url = form_data.get("url", "").strip()
        icon_url = form_data.get("icon_url", "").strip()

        if not name:
            return {"type": "error", "message": "App name is required"}
        if not url:
            return {"type": "error", "message": "URL is required"}
        if not icon_url:
            return {"type": "error", "message": "Icon URL is required"}

        # Add https:// if missing
        if not url.startswith("http://") and not url.startswith("https://"):
            url = "https://" + url

        if not icon_url.startswith("http://") and not icon_url.startswith("https://"):
            icon_url = "https://" + icon_url

        # Check if already exists
        app_id = sanitize_name(name)
        if any(app["id"] == app_id for app in webapps):
            return {"type": "error", "message": f"'{name}' already exists"}

        # Download icon
        icon_path = await download_icon(icon_url, name)
        if not icon_path:
            return {"type": "error", "message": "Failed to download icon"}

        # Add new webapp
        new_app = {
            "id": app_id,
            "name": name,
            "url": url,
            "icon": icon_path,
        }
        webapps.append(new_app)

        if save_webapps(webapps):
            return HamrPlugin.results(
                get_webapp_results(webapps),
                input_mode="realtime",
                clear_input=True,
                placeholder="Search web apps...",
                plugin_actions=get_plugin_actions(),
            )
        else:
            return HamrPlugin.error("Failed to save web app")

    if context and context.startswith("__edit__:"):
        app_id = context.split(":", 1)[1]
        app = next((a for a in webapps if a["id"] == app_id), None)

        if not app:
            return {"type": "error", "message": "Web app not found"}

        name = form_data.get("name", "").strip()
        url = form_data.get("url", "").strip()
        icon_url = form_data.get("icon_url", "").strip()

        if not name:
            return {"type": "error", "message": "App name is required"}
        if not url:
            return {"type": "error", "message": "URL is required"}

        # Add https:// if missing
        if not url.startswith("http://") and not url.startswith("https://"):
            url = "https://" + url

        # Update app
        app["name"] = name
        app["url"] = url

        # Download new icon if provided
        if icon_url:
            if not icon_url.startswith("http://") and not icon_url.startswith(
                "https://"
            ):
                icon_url = "https://" + icon_url

            new_icon_path = await download_icon(icon_url, name)
            if new_icon_path:
                # Delete old icon if different
                old_icon = app.get("icon", "")
                if old_icon and old_icon != new_icon_path and Path(old_icon).exists():
                    Path(old_icon).unlink()
                app["icon"] = new_icon_path
            else:
                return HamrPlugin.error("Failed to download new icon")

        if save_webapps(webapps):
            return HamrPlugin.results(
                get_webapp_results(webapps),
                input_mode="realtime",
                clear_input=True,
                placeholder="Search web apps...",
                plugin_actions=get_plugin_actions(),
            )
        else:
            return HamrPlugin.error("Failed to save web app")

    return {}


if __name__ == "__main__":
    plugin.run()
