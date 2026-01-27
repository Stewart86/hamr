#!/usr/bin/env python3
"""
Bitwarden plugin - Search and copy credentials from Bitwarden vault.

Requires:
- bw (Bitwarden CLI) installed and in PATH
- python-keyring (optional, for secure session storage)

The plugin will guide users through login/unlock if no session is found.
"""

import asyncio
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

# Add parent directory to path to import SDK
sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

# Optional keyring support for secure session storage
KEYRING_SERVICE = "hamr-bitwarden"
KEYRING_USERNAME = "session"

CACHE_DIR = Path(os.environ.get("XDG_RUNTIME_DIR", "/tmp")) / "hamr" / "bitwarden"
ITEMS_CACHE_FILE = CACHE_DIR / "items.json"
LAST_EMAIL_FILE = CACHE_DIR / "last_email"
CACHE_MAX_AGE_SECONDS = 300  # 5 minutes


def _get_keyring():
    """Lazy import keyring module"""
    try:
        import importlib

        return importlib.import_module("keyring")
    except ImportError:
        return None


def get_last_email() -> str:
    """Get last used email for login convenience"""
    if LAST_EMAIL_FILE.exists():
        try:
            return LAST_EMAIL_FILE.read_text().strip()
        except OSError:
            pass
    return ""


def save_last_email(email: str):
    """Save last used email"""
    try:
        CACHE_DIR.mkdir(parents=True, exist_ok=True)
        LAST_EMAIL_FILE.write_text(email)
    except OSError:
        pass


def find_bw() -> str | None:
    """Find bw executable, checking common user paths"""
    bw_path = shutil.which("bw")
    if bw_path:
        return bw_path

    home = Path.home()
    common_paths = [
        home / ".local" / "share" / "pnpm" / "bw",
        home / ".local" / "bin" / "bw",
        home / ".npm-global" / "bin" / "bw",
        home / "bin" / "bw",
        Path("/usr/local/bin/bw"),
    ]

    nvm_dir = home / ".nvm" / "versions" / "node"
    if nvm_dir.exists():
        for node_version in nvm_dir.iterdir():
            bw_in_nvm = node_version / "bin" / "bw"
            if bw_in_nvm.exists() and os.access(bw_in_nvm, os.X_OK):
                return str(bw_in_nvm)

    for path in common_paths:
        if path.exists() and os.access(path, os.X_OK):
            return str(path)

    return None


BW_PATH = find_bw()


def get_session_from_keyring() -> str | None:
    """Get session from keyring"""
    kr = _get_keyring()
    if not kr:
        return None
    try:
        return kr.get_password(KEYRING_SERVICE, KEYRING_USERNAME)
    except Exception:
        return None


def save_session_to_keyring(session: str) -> bool:
    """Save session to keyring"""
    kr = _get_keyring()
    if not kr:
        return False
    try:
        kr.set_password(KEYRING_SERVICE, KEYRING_USERNAME, session)
        return True
    except Exception:
        return False


def clear_session_from_keyring() -> bool:
    """Clear session from keyring"""
    kr = _get_keyring()
    if not kr:
        return False
    try:
        kr.delete_password(KEYRING_SERVICE, KEYRING_USERNAME)
        return True
    except Exception:
        return False


def get_session() -> str | None:
    """Get session from keyring"""
    return get_session_from_keyring()


def get_bw_status(session: str | None = None) -> dict:
    """Get Bitwarden CLI status"""
    try:
        env = os.environ.copy()
        if session:
            env["BW_SESSION"] = session
        env["NODE_NO_WARNINGS"] = "1"

        result = subprocess.run(
            [BW_PATH, "status"],
            capture_output=True,
            text=True,
            timeout=10,
            env=env,
        )
        if result.returncode == 0:
            try:
                return json.loads(result.stdout)
            except json.JSONDecodeError:
                pass
    except Exception:
        pass
    return {"status": "unauthenticated"}


def unlock_vault(password: str) -> tuple[bool, str]:
    """Unlock vault with master password, returns (success, session_or_error)"""
    if not BW_PATH:
        return False, "Bitwarden CLI not found"

    try:
        result = subprocess.run(
            [BW_PATH, "unlock", "--raw", password],
            capture_output=True,
            text=True,
            timeout=30,
            env={**os.environ, "NODE_NO_WARNINGS": "1"},
        )
        if result.returncode == 0:
            session = result.stdout.strip()
            if session:
                save_session_to_keyring(session)
                return True, session
        return False, result.stderr.strip() or "Failed to unlock"
    except subprocess.TimeoutExpired:
        return False, "Unlock timed out"
    except Exception as e:
        return False, str(e)


def login_vault(email: str, password: str, code: str = "") -> tuple[bool, str]:
    """Login to vault, returns (success, session_or_error)"""
    if not BW_PATH:
        return False, "Bitwarden CLI not found"

    try:
        args = [BW_PATH, "login", "--raw", email, password]
        if code:
            args.extend(["--code", code])

        result = subprocess.run(
            args,
            capture_output=True,
            text=True,
            timeout=60,
            env={**os.environ, "NODE_NO_WARNINGS": "1"},
        )
        if result.returncode == 0:
            session = result.stdout.strip()
            if session:
                save_session_to_keyring(session)
                return True, session
        error = result.stderr.strip() or result.stdout.strip()
        return False, error or "Failed to login"
    except subprocess.TimeoutExpired:
        return False, "Login timed out"
    except Exception as e:
        return False, str(e)


def run_bw(args: list[str], session: str | None = None) -> tuple[bool, str]:
    """Run bw command and return (success, output)"""
    if not BW_PATH:
        return False, "Bitwarden CLI not found"

    env = os.environ.copy()
    if session:
        env["BW_SESSION"] = session
    env["NODE_NO_WARNINGS"] = "1"

    try:
        result = subprocess.run(
            [BW_PATH] + args,
            capture_output=True,
            text=True,
            timeout=30,
            env=env,
        )
        if result.returncode == 0:
            return True, result.stdout.strip()
        return False, result.stderr.strip() or result.stdout.strip()
    except subprocess.TimeoutExpired:
        return False, "Command timed out"
    except Exception as e:
        return False, str(e)


def get_cache_age() -> float | None:
    """Get age of cache in seconds"""
    if not ITEMS_CACHE_FILE.exists():
        return None
    return time.time() - ITEMS_CACHE_FILE.stat().st_mtime


def is_cache_fresh() -> bool:
    """Check if cache is fresh"""
    age = get_cache_age()
    return age is not None and age < CACHE_MAX_AGE_SECONDS


def load_cached_items() -> list[dict] | None:
    """Load items from cache"""
    if not ITEMS_CACHE_FILE.exists():
        return None
    try:
        return json.loads(ITEMS_CACHE_FILE.read_text())
    except (json.JSONDecodeError, OSError):
        return None


def save_items_cache(items: list[dict]):
    """Save items to cache"""
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    ITEMS_CACHE_FILE.write_text(json.dumps(items))
    ITEMS_CACHE_FILE.chmod(0o600)


def clear_items_cache():
    """Clear items cache"""
    if ITEMS_CACHE_FILE.exists():
        ITEMS_CACHE_FILE.unlink()


def fetch_all_items(session: str) -> list[dict]:
    """Fetch all vault items"""
    success, output = run_bw(["list", "items"], session=session)
    if success:
        try:
            return json.loads(output)
        except json.JSONDecodeError:
            pass
    return []


def search_items(query: str, session: str, force_refresh: bool = False) -> list[dict]:
    """Search vault items using cache"""
    cached_items = None if force_refresh else load_cached_items()

    def matches_query(item: dict, q: str) -> bool:
        """Check if item matches query"""
        name = item.get("name", "") or ""
        username = item.get("login", {}).get("username", "") or ""
        return q in name.lower() or q in username.lower()

    if cached_items is not None:
        if query:
            results = [
                item for item in cached_items if matches_query(item, query.lower())
            ]
        else:
            results = cached_items

        if not is_cache_fresh():
            # Sync in background
            try:
                run_bw(["sync"], session=session)
                new_items = fetch_all_items(session)
                if new_items:
                    save_items_cache(new_items)
            except Exception:
                pass

        return results[:50]

    items = fetch_all_items(session)
    if items:
        save_items_cache(items)

    if query:
        items = [item for item in items if matches_query(item, query.lower())]

    return items[:50]


def get_totp(item_id: str, session: str) -> str | None:
    """Get TOTP code for item"""
    success, output = run_bw(["get", "totp", item_id], session=session)
    return output if success else None


def get_item_icon(item: dict) -> str:
    """Get icon for item type"""
    item_type = item.get("type", 1)
    icons = {1: "password", 2: "note", 3: "credit_card", 4: "person"}
    return icons.get(item_type, "key")


def get_item_uris(item: dict) -> list[str]:
    """Extract URIs from vault item"""
    login = item.get("login", {}) or {}
    uris = login.get("uris", []) or []
    return [u.get("uri", "") for u in uris if u.get("uri")]


def get_item_type_badge(item: dict) -> dict | None:
    """Get badge for item type"""
    item_type = item.get("type", 1)
    badges = {
        1: None,
        2: {"icon": "note", "color": "#9c27b0"},
        3: {"icon": "credit_card", "color": "#ff9800"},
        4: {"icon": "person", "color": "#4caf50"},
    }
    return badges.get(item_type)


def get_item_chips(item: dict) -> list[dict]:
    """Get feature chips for item"""
    chips = []
    login = item.get("login", {}) or {}
    uris = get_item_uris(item)

    if login.get("totp"):
        chips.append({"text": "2FA", "icon": "schedule"})
    if len(uris) > 1:
        chips.append({"text": f"{len(uris)} URLs", "icon": "link"})

    return chips


def format_item_results(items: list[dict]) -> list[dict]:
    """Format vault items as results"""
    results = []
    for item in items:
        item_id = item.get("id", "")
        name = item.get("name", "Unknown")
        login = item.get("login", {}) or {}
        username = login.get("username", "")

        actions = []
        if username:
            actions.append(
                {"id": "copy_username", "name": "Copy Username", "icon": "person"}
            )
        if login.get("password"):
            actions.append(
                {"id": "copy_password", "name": "Copy Password", "icon": "key"}
            )
        if login.get("totp"):
            actions.append({"id": "copy_totp", "name": "Copy TOTP", "icon": "schedule"})
        if get_item_uris(item):
            actions.append(
                {"id": "open_url", "name": "Open URL", "icon": "open_in_new"}
            )

        badges = []
        type_badge = get_item_type_badge(item)
        if type_badge:
            badges.append(type_badge)

        chips = get_item_chips(item)

        result = {
            "id": item_id,
            "name": name,
            "description": username
            or (item.get("notes", "")[:50] if item.get("notes") else ""),
            "icon": get_item_icon(item),
            "verb": "Copy Password" if login.get("password") else "View",
            "actions": actions,
        }
        if badges:
            result["badges"] = badges
        if chips:
            result["chips"] = chips

        results.append(result)

    return results


def get_plugin_actions(cache_age: float | None = None) -> list[dict]:
    """Get plugin-level actions for the action bar"""
    if cache_age is not None:
        if cache_age < 60:
            cache_status = "just now"
        elif cache_age < 3600:
            cache_status = f"{int(cache_age // 60)}m ago"
        else:
            cache_status = f"{int(cache_age // 3600)}h ago"
        sync_name = f"Sync ({cache_status})"
    else:
        sync_name = "Sync Vault"

    return [
        {
            "id": "sync",
            "name": sync_name,
            "icon": "sync",
            "shortcut": "Ctrl+1",
        },
        {
            "id": "lock",
            "name": "Lock Vault",
            "icon": "lock",
            "shortcut": "Ctrl+2",
        },
        {
            "id": "logout",
            "name": "Logout",
            "icon": "logout",
            "shortcut": "Ctrl+3",
        },
    ]


def lock_vault() -> tuple[bool, str]:
    """Lock the vault and clear session"""
    if not BW_PATH:
        return False, "Bitwarden CLI not found"

    clear_session_from_keyring()
    clear_items_cache()

    try:
        result = subprocess.run(
            [BW_PATH, "lock"],
            capture_output=True,
            text=True,
            timeout=10,
            env={**os.environ, "NODE_NO_WARNINGS": "1"},
        )
        if result.returncode == 0:
            return True, "Vault locked"
        return False, result.stderr.strip() or "Failed to lock"
    except Exception as e:
        return False, str(e)


def logout_vault() -> tuple[bool, str]:
    """Logout from the vault completely"""
    if not BW_PATH:
        return False, "Bitwarden CLI not found"

    clear_session_from_keyring()
    clear_items_cache()

    if LAST_EMAIL_FILE.exists():
        try:
            LAST_EMAIL_FILE.unlink()
        except OSError:
            pass

    try:
        result = subprocess.run(
            [BW_PATH, "logout"],
            capture_output=True,
            text=True,
            timeout=10,
            env={**os.environ, "NODE_NO_WARNINGS": "1"},
        )
        if result.returncode == 0:
            return True, "Logged out"
        if "not logged in" in result.stderr.lower():
            return True, "Logged out"
        return False, result.stderr.strip() or "Failed to logout"
    except Exception as e:
        return False, str(e)


# Create plugin instance
plugin = HamrPlugin(
    id="bitwarden",
    name="Bitwarden",
    description="Search and copy credentials from Bitwarden vault",
    icon="key",
)


@plugin.on_initial
async def handle_initial(params=None):
    """Handle initial request when plugin is opened."""
    if not BW_PATH:
        return HamrPlugin.results(
            [],
            card=HamrPlugin.card(
                "Bitwarden CLI Required",
                markdown="**Bitwarden CLI (`bw`) is not installed.**\n\n"
                "Install with: `npm install -g @bitwarden/cli`",
            ),
        )

        if bw_status == "locked":
            user_email = status.get("userEmail", "")
            return HamrPlugin.form(
                {
                    "id": "unlock",
                    "title": f"Unlock Vault ({user_email})"
                    if user_email
                    else "Unlock Vault",
                    "fields": [
                        {
                            "id": "password",
                            "label": "Master Password",
                            "type": "password",
                            "placeholder": "Enter your master password",
                        },
                    ],
                    "submitLabel": "Unlock",
                },
            )

    items = search_items("", session)
    if not items:
        return HamrPlugin.results(
            [],
            card=HamrPlugin.card(
                "No Items Found",
                content="Either your vault is empty, locked, or the session expired.",
                markdown=True,
            ),
        )

    results = format_item_results(items)
    cache_age = get_cache_age()

    return HamrPlugin.results(
        results,
        status={"chips": [{"text": "Vault unlocked", "icon": "lock"}]},
        plugin_actions=get_plugin_actions(cache_age),
        placeholder="Search vault...",
    )


@plugin.on_search
async def handle_search(query: str, context: str | None):
    """Handle search request."""
    if not BW_PATH:
        return HamrPlugin.results([])

    session = get_session()
    if not session:
        return HamrPlugin.results([])

    items = search_items(query, session)
    results = format_item_results(items)
    cache_age = get_cache_age()

    if not results:
        results = [
            {
                "id": "__no_results__",
                "name": f"No results for '{query}'",
                "icon": "search_off",
            }
        ]

    return HamrPlugin.results(
        results,
        status={"chips": [{"text": "Vault unlocked", "icon": "lock"}]},
        plugin_actions=get_plugin_actions(cache_age),
        placeholder="Search vault...",
    )


@plugin.on_action
async def handle_action(item_id: str, action: str | None, context: str | None):
    """Handle action request."""
    if not BW_PATH:
        return HamrPlugin.noop()

    session = get_session()
    if not session:
        return HamrPlugin.noop()

    # Plugin-level actions
    if item_id == "__plugin__":
        if action == "sync":
            run_bw(["sync"], session=session)
            clear_items_cache()
            items = search_items("", session, force_refresh=True)
            results = format_item_results(items)
            cache_age = get_cache_age()
            return HamrPlugin.results(
                results,
                clear_input=True,
                plugin_actions=get_plugin_actions(cache_age),
                placeholder="Vault synced!",
            )

        if action == "lock":
            success, message = lock_vault()
            if success:
                return HamrPlugin.execute(close=True, notify="Vault locked")
            return HamrPlugin.card("Error", content=f"Failed: {message}")

        if action == "logout":
            success, message = logout_vault()
            if success:
                return HamrPlugin.execute(close=True, notify="Logged out of Bitwarden")
            return HamrPlugin.card("Error", content=f"Failed: {message}")

    if item_id == "__no_results__":
        return HamrPlugin.noop()

    # Item actions
    item = None
    cached_items = load_cached_items()
    if cached_items:
        item = next((i for i in cached_items if i.get("id") == item_id), None)

    if not item:
        success, output = run_bw(["get", "item", item_id], session=session)
        if not success:
            return {
                "card": {"title": "Error", "content": f"Failed to get item: {output}"}
            }
        try:
            item = json.loads(output)
        except json.JSONDecodeError:
            return {"card": {"title": "Error", "content": "Failed to parse item data"}}

    login = item.get("login", {}) or {}
    username = login.get("username", "") or ""
    password = login.get("password", "") or ""

    if action == "copy_username" and username:
        subprocess.run(["wl-copy", username], check=False)
        return HamrPlugin.copy_and_close(username)

    if action == "copy_password" and password:
        subprocess.run(["wl-copy", password], check=False)
        return HamrPlugin.execute(close=True, notify="Password copied to clipboard")

    if action == "copy_totp":
        totp = get_totp(item_id, session)
        if totp:
            subprocess.run(["wl-copy", totp], check=False)
            return HamrPlugin.execute(close=True, notify=f"TOTP copied: {totp}")
        return HamrPlugin.card("Error", content="Failed to get TOTP code")

    if action == "open_url":
        uris = get_item_uris(item)
        if uris:
            url = uris[0]
            subprocess.Popen(["xdg-open", url])
            return HamrPlugin.execute(close=True, notify=f"Opening {url[:40]}")
        return HamrPlugin.card("Error", content="No URL found for this item")

    # Default action
    if password:
        subprocess.run(["wl-copy", password], check=False)
        return HamrPlugin.execute(close=True, notify="Password copied to clipboard")
    elif username:
        subprocess.run(["wl-copy", username], check=False)
        return HamrPlugin.copy_and_close(username)

    return HamrPlugin.noop()


@plugin.on_form_submitted
async def handle_form_submitted(form_data: dict, context: str | None):
    """Handle form submission."""
    if context == "login":
        email = form_data.get("email", "")
        password = form_data.get("password", "")
        code = form_data.get("code", "")

        success, result = login_vault(email, password, code)
        if success:
            save_last_email(email)
            items = fetch_all_items(result)
            if items:
                save_items_cache(items)
                results = format_item_results(items)
                return HamrPlugin.results(
                    results,
                    plugin_actions=get_plugin_actions(get_cache_age()),
                    placeholder="Logged in! Search...",
                )
            return HamrPlugin.card(
                "Logged In",
                content="Logged in but no items found. Your vault may be empty.",
            )
        if "Two-step" in result or "code" in result.lower():
            return HamrPlugin.form(
                {
                    "id": "login",
                    "title": "Two-Factor Authentication Required",
                    "fields": [
                        {"id": "email", "type": "hidden", "value": email},
                        {"id": "password", "type": "hidden", "value": password},
                        {
                            "id": "code",
                            "label": "2FA Code",
                            "type": "text",
                            "placeholder": "Enter your 2FA code",
                        },
                    ],
                    "submitLabel": "Verify",
                }
            )
        return HamrPlugin.card(
            "Login Failed",
            content=f"**Error:** {result}",
            markdown=True,
        )

    if context == "unlock":
        password = form_data.get("password", "")
        success, result = unlock_vault(password)
        if success:
            items = fetch_all_items(result)
            if items:
                save_items_cache(items)
                results = format_item_results(items)
                return HamrPlugin.results(
                    results,
                    plugin_actions=get_plugin_actions(get_cache_age()),
                    placeholder="Vault unlocked! Search...",
                )
            return HamrPlugin.card(
                "Vault Unlocked",
                content="Vault unlocked but no items found. Your vault may be empty.",
            )
        return HamrPlugin.card(
            "Unlock Failed",
            content=f"**Error:** {result}",
            markdown=True,
        )

    return HamrPlugin.noop()


if __name__ == "__main__":
    plugin.run()
