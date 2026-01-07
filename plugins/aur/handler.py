#!/usr/bin/env python3
"""
AUR plugin - Search and install packages from the Arch User Repository.

Features:
- Search AUR for packages using the RPC API
- Detect available AUR helper (yay or paru)
- Install/uninstall AUR packages
- Show package details (votes, popularity, maintainer)
- View package on AUR website
"""

import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
import urllib.parse
import urllib.request
import urllib.error
from pathlib import Path

AUR_RPC = "https://aur.archlinux.org/rpc/v5/search"
AUR_INFO = "https://aur.archlinux.org/rpc/v5/info"
AUR_WEB = "https://aur.archlinux.org/packages"
CACHE_DIR = (
    Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "hamr" / "aur"
)
CACHE_TTL = 300


def detect_aur_helper() -> str | None:
    """Detect available AUR helper (yay or paru)"""
    for helper in ("paru", "yay"):
        if shutil.which(helper):
            return helper
    return None


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


def get_installed_packages() -> set[str]:
    """Get set of installed package names"""
    try:
        result = subprocess.run(
            ["pacman", "-Qq"],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            return set(result.stdout.strip().split("\n")) - {""}
    except Exception:
        pass
    return set()


def search_aur(query: str) -> list[dict]:
    """Search AUR API for packages with caching"""
    cached = get_cached_results(query)
    if cached is not None:
        return cached

    try:
        url = f"{AUR_RPC}/{urllib.parse.quote(query)}"
        req = urllib.request.Request(
            url,
            headers={"User-Agent": "hamr-aur-plugin/1.0"},
        )
        with urllib.request.urlopen(req, timeout=10) as response:
            result = json.loads(response.read().decode("utf-8"))
            if result.get("type") == "search":
                hits = result.get("results", [])
                save_cached_results(query, hits)
                return hits
    except Exception:
        pass

    return []


def format_popularity(pop: float) -> str:
    """Format popularity score for display"""
    if pop >= 100:
        return f"{pop:.0f}"
    if pop >= 10:
        return f"{pop:.1f}"
    return f"{pop:.2f}"


def format_votes(votes: int) -> str:
    """Format vote count for display"""
    if votes >= 1000:
        return f"{votes / 1000:.1f}K"
    return str(votes)


def format_date(timestamp: int | None) -> str:
    """Format Unix timestamp to readable date"""
    if not timestamp:
        return ""
    try:
        return time.strftime("%Y-%m-%d", time.localtime(timestamp))
    except Exception:
        return ""


def pkg_to_result(pkg: dict, installed_pkgs: set[str], aur_helper: str | None) -> dict:
    """Convert AUR package to result format"""
    name = pkg.get("Name", "")
    is_installed = name in installed_pkgs
    votes = pkg.get("NumVotes", 0)
    popularity = pkg.get("Popularity", 0)
    maintainer = pkg.get("Maintainer", "")
    out_of_date = pkg.get("OutOfDate")
    version = pkg.get("Version", "")

    description = pkg.get("Description", "")

    badges = []
    if is_installed:
        badges.append({"icon": "check_circle", "color": "#4caf50"})
    if out_of_date:
        badges.append({"icon": "warning", "color": "#ff9800"})

    chips = []
    if maintainer:
        chips.append({"text": maintainer, "icon": "person"})
    if votes:
        chips.append({"text": f"{format_votes(votes)} votes", "icon": "thumb_up"})
    if popularity:
        chips.append(
            {"text": f"{format_popularity(popularity)}", "icon": "trending_up"}
        )

    actions = []
    if is_installed:
        if aur_helper:
            actions.append({"id": "uninstall", "name": "Uninstall", "icon": "delete"})
    elif aur_helper:
        actions.append({"id": "install", "name": "Install", "icon": "download"})
    actions.append({"id": "open_web", "name": "View on AUR", "icon": "open_in_new"})

    verb = "Open" if is_installed else "Install"
    if not aur_helper:
        verb = "View"

    result: dict = {
        "id": name,
        "name": f"{name} {version}",
        "description": description,
        "icon": "package_2",
        "verb": verb,
        "actions": actions,
    }

    if badges:
        result["badges"] = badges
    if chips:
        result["chips"] = chips

    return result


def get_plugin_actions() -> list[dict]:
    """Get plugin-level actions for the action bar"""
    return []


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    selected_id = selected.get("id", "")
    aur_helper = detect_aur_helper()

    if step == "initial":
        helper_msg = (
            f"Using {aur_helper}"
            if aur_helper
            else "No AUR helper found (install yay or paru)"
        )
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": [
                        {
                            "id": "__prompt__",
                            "name": "Search AUR",
                            "description": helper_msg,
                            "icon": "search",
                        }
                    ],
                    "inputMode": "realtime",
                    "placeholder": "Search AUR packages...",
                    "pluginActions": get_plugin_actions(),
                }
            )
        )
        return

    if step == "search":
        if not query or len(query) < 2:
            helper_msg = f"Using {aur_helper}" if aur_helper else "No AUR helper found"
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": [
                            {
                                "id": "__prompt__",
                                "name": "Search AUR",
                                "description": f"Type at least 2 characters to search. {helper_msg}",
                                "icon": "search",
                            }
                        ],
                        "inputMode": "realtime",
                        "placeholder": "Search AUR packages...",
                        "pluginActions": get_plugin_actions(),
                    }
                )
            )
            return

        packages = search_aur(query)
        installed_pkgs = get_installed_packages()

        if not packages:
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": [
                            {
                                "id": "__empty__",
                                "name": "No packages found",
                                "description": f"No AUR packages matching '{query}'",
                                "icon": "search_off",
                            }
                        ],
                        "inputMode": "realtime",
                        "placeholder": "Search AUR packages...",
                        "pluginActions": get_plugin_actions(),
                    }
                )
            )
            return

        results = [
            pkg_to_result(pkg, installed_pkgs, aur_helper) for pkg in packages[:30]
        ]
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                    "placeholder": "Search AUR packages...",
                    "pluginActions": get_plugin_actions(),
                }
            )
        )
        return

    if step == "action":
        if selected_id in ("__prompt__", "__empty__"):
            return

        pkg_name = selected_id

        if action == "open_web":
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "openUrl": f"{AUR_WEB}/{pkg_name}",
                        "close": True,
                    }
                )
            )
            return

        if not aur_helper:
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "openUrl": f"{AUR_WEB}/{pkg_name}",
                        "close": True,
                    }
                )
            )
            return

        if action == "uninstall":
            cmd = (
                f'notify-send "AUR" "Uninstalling {pkg_name}..." -a "Hamr" && '
                f"{aur_helper} -Rns --noconfirm {pkg_name} && "
                f'notify-send "AUR" "{pkg_name} uninstalled" -a "Hamr" || '
                f'notify-send "AUR" "Failed to uninstall {pkg_name}" -a "Hamr"'
            )
            subprocess.Popen(["bash", "-c", cmd])
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "close": True,
                    }
                )
            )
            return

        if action == "install":
            cmd = (
                f'notify-send "AUR" "Installing {pkg_name}..." -a "Hamr" && '
                f"{aur_helper} -S --noconfirm {pkg_name} && "
                f'notify-send "AUR" "{pkg_name} installed" -a "Hamr" || '
                f'notify-send "AUR" "Failed to install {pkg_name}" -a "Hamr"'
            )
            subprocess.Popen(["bash", "-c", cmd])
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "close": True,
                    }
                )
            )
            return

        installed_pkgs = get_installed_packages()
        is_installed = pkg_name in installed_pkgs

        if is_installed:
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "openUrl": f"{AUR_WEB}/{pkg_name}",
                        "close": True,
                    }
                )
            )
        else:
            cmd = (
                f'notify-send "AUR" "Installing {pkg_name}..." -a "Hamr" && '
                f"{aur_helper} -S --noconfirm {pkg_name} && "
                f'notify-send "AUR" "{pkg_name} installed" -a "Hamr" || '
                f'notify-send "AUR" "Failed to install {pkg_name}" -a "Hamr"'
            )
            subprocess.Popen(["bash", "-c", cmd])
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "close": True,
                    }
                )
            )
        return


if __name__ == "__main__":
    main()
