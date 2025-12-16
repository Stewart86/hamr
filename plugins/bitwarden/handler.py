#!/usr/bin/env python3
"""
Bitwarden workflow handler - search and copy credentials from Bitwarden vault.

Requires:
- bw (Bitwarden CLI) installed and in PATH
- User must login and unlock vault manually: `bw login && bw unlock`
- Session token saved to ~/.bw_session or BW_SESSION env var
"""

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

# Test mode - return mock data instead of calling real Bitwarden CLI
TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

# Mock vault items for testing
MOCK_VAULT_ITEMS = [
    {
        "id": "mock-github-id",
        "name": "GitHub",
        "type": 1,
        "login": {
            "username": "testuser@example.com",
            "password": "mock-password-123",
            "totp": "JBSWY3DPEHPK3PXP",
        },
        "notes": "Personal GitHub account",
    },
    {
        "id": "mock-google-id",
        "name": "Google",
        "type": 1,
        "login": {
            "username": "testuser@gmail.com",
            "password": "mock-google-pass",
            "totp": None,
        },
        "notes": "",
    },
    {
        "id": "mock-note-id",
        "name": "Secret Note",
        "type": 2,
        "login": {},
        "notes": "This is a secure note with sensitive info",
    },
    {
        "id": "mock-card-id",
        "name": "Credit Card",
        "type": 3,
        "login": {},
        "notes": "Main credit card",
    },
]

# Cache directory for vault items
CACHE_DIR = (
    Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache"))
    / "hamr"
    / "bitwarden"
)
ITEMS_CACHE_FILE = CACHE_DIR / "items.json"
CACHE_MAX_AGE_SECONDS = 300  # 5 minutes


def find_bw() -> str | None:
    """Find bw executable, checking common user paths"""
    if TEST_MODE:
        return "/mock/bw"  # Return fake path in test mode

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


def get_session() -> str | None:
    """Get session from file locations (env var often stale in graphical apps)"""
    if TEST_MODE:
        return "mock-session-token"  # Return fake session in test mode

    home = Path.home()
    session_files = [
        home / ".bw_session",
        home / ".config" / "bw" / "session",
        home / ".cache" / "bw" / "session",
    ]

    for sf in session_files:
        if sf.exists():
            session = sf.read_text().strip()
            if session:
                return session

    # Fall back to env var
    return os.environ.get("BW_SESSION")


def run_bw(args: list[str], session: str | None = None) -> tuple[bool, str]:
    """Run bw command and return (success, output)"""
    # In test mode, return mock responses
    if TEST_MODE:
        if args == ["list", "items"]:
            return True, json.dumps(MOCK_VAULT_ITEMS)
        elif args == ["sync"]:
            return True, "Syncing complete."
        elif len(args) >= 3 and args[0] == "get" and args[1] == "item":
            item_id = args[2]
            for item in MOCK_VAULT_ITEMS:
                if item["id"] == item_id:
                    return True, json.dumps(item)
            return False, "Item not found"
        elif len(args) >= 3 and args[0] == "get" and args[1] == "totp":
            return True, "123456"  # Mock TOTP code
        return True, ""

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
        # Clean node.js warnings from output
        stdout = "\n".join(
            line
            for line in result.stdout.split("\n")
            if not any(
                skip in line
                for skip in [
                    "DeprecationWarning",
                    "ExperimentalWarning",
                    "--trace-deprecation",
                    "Support for loading ES Module",
                ]
            )
        ).strip()
        stderr = "\n".join(
            line
            for line in result.stderr.split("\n")
            if not any(
                skip in line
                for skip in [
                    "DeprecationWarning",
                    "ExperimentalWarning",
                    "--trace-deprecation",
                    "Support for loading ES Module",
                ]
            )
        ).strip()

        if result.returncode == 0:
            return True, stdout
        return False, stderr or stdout
    except subprocess.TimeoutExpired:
        return False, "Command timed out"
    except Exception as e:
        return False, str(e)




def get_cache_age() -> float | None:
    """Get age of cache in seconds"""
    if not ITEMS_CACHE_FILE.exists():
        return None
    import time

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


def get_cached_item(item_id: str) -> dict | None:
    """Get a single item from cache by ID"""
    items = load_cached_items()
    if items:
        for item in items:
            if item.get("id") == item_id:
                return item
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


def sync_vault_background(session: str):
    """Sync vault in background"""
    if not BW_PATH:
        return
    pid = os.fork()
    if pid == 0:
        try:
            os.setsid()
            subprocess.run(
                [BW_PATH, "sync"],
                capture_output=True,
                timeout=60,
                env={**os.environ, "BW_SESSION": session, "NODE_NO_WARNINGS": "1"},
            )
            result = subprocess.run(
                [BW_PATH, "list", "items"],
                capture_output=True,
                text=True,
                timeout=60,
                env={**os.environ, "BW_SESSION": session, "NODE_NO_WARNINGS": "1"},
            )
            if result.returncode == 0:
                items = json.loads(result.stdout)
                save_items_cache(items)
        except Exception:
            pass
        finally:
            os._exit(0)


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
        # Filter locally
        if query:
            results = [
                item for item in cached_items if matches_query(item, query.lower())
            ]
        else:
            results = cached_items

        # Background sync if stale
        if not is_cache_fresh():
            sync_vault_background(session)

        return results[:50]

    # No cache - fetch from bw
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




def get_plugin_actions(cache_age: float | None = None) -> list[dict]:
    """Get plugin-level actions for the action bar"""
    # Cache status for sync button
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
        }
    ]




def get_item_icon(item: dict) -> str:
    """Get icon for item type"""
    item_type = item.get("type", 1)
    icons = {1: "password", 2: "note", 3: "credit_card", 4: "person"}
    return icons.get(item_type, "key")


def format_item_results(items: list[dict]) -> list[dict]:
    """Format vault items as results"""
    results = []
    for item in items:
        item_id = item.get("id", "")
        name = item.get("name", "Unknown")
        login = item.get("login", {})
        username = login.get("username", "")
        has_totp = bool(login.get("totp"))

        actions = []
        if username:
            actions.append(
                {"id": "copy_username", "name": "Copy Username", "icon": "person"}
            )
        if login.get("password"):
            actions.append(
                {"id": "copy_password", "name": "Copy Password", "icon": "key"}
            )
        if has_totp:
            actions.append({"id": "copy_totp", "name": "Copy TOTP", "icon": "schedule"})

        results.append(
            {
                "id": item_id,
                "name": name,
                "description": username
                or (item.get("notes", "")[:50] if item.get("notes") else ""),
                "icon": get_item_icon(item),
                "verb": "Copy Password" if login.get("password") else "View",
                "actions": actions,
            }
        )

    return results




def respond_results(
    results: list[dict], placeholder: str = "Search vault...", **kwargs
):
    """Send results response"""
    response = {
        "type": "results",
        "results": results,
        "inputMode": kwargs.get("input_mode", "realtime"),
        "placeholder": placeholder,
    }
    if kwargs.get("clear_input"):
        response["clearInput"] = True
    print(json.dumps(response))


def respond_card(title: str, content: str, **kwargs):
    """Send card response"""
    print(
        json.dumps(
            {
                "type": "card",
                "card": {"title": title, "content": content, "markdown": True},
                "inputMode": kwargs.get("input_mode", "realtime"),
                "placeholder": kwargs.get("placeholder", ""),
            }
        )
    )


def respond_execute(
    command: list[str] | None = None,
    close: bool = True,
    notify: str = "",
    name: str = "",
    icon: str = "",
    entry_point: dict | None = None,
):
    """Send execute response.

    Args:
        command: Shell command to run (optional if using entry_point)
        close: Whether to close the launcher
        notify: Notification message to show
        name: Action name for history tracking (required for history)
        icon: Material icon for history entry
        entry_point: Workflow entry point for complex replay (re-invokes handler)
                    Used instead of command when action needs handler logic
                    (e.g., fetching fresh credentials from API)
    """
    execute: dict = {"close": close}
    if command:
        execute["command"] = command
    if notify:
        execute["notify"] = notify
    if name:
        execute["name"] = name
    if icon:
        execute["icon"] = icon
    if entry_point:
        execute["entryPoint"] = entry_point
    print(json.dumps({"type": "execute", "execute": execute}))




def item_to_index_item(item: dict) -> dict:
    """Convert a vault item to an index item for main search.

    IMPORTANT: Never include passwords or sensitive data in index items.
    Uses entryPoint for execution so credentials are fetched fresh on replay.
    """
    item_id = item.get("id", "")
    name = item.get("name", "Unknown")
    login = item.get("login", {}) or {}
    username = login.get("username", "")
    has_password = bool(login.get("password"))
    has_totp = bool(login.get("totp"))

    # Build actions with entryPoints (no passwords stored!)
    actions = []
    if username:
        actions.append(
            {
                "id": "copy_username",
                "name": "Copy Username",
                "icon": "person",
                "entryPoint": {
                    "step": "action",
                    "selected": {"id": item_id},
                    "action": "copy_username",
                },
            }
        )
    if has_password:
        actions.append(
            {
                "id": "copy_password",
                "name": "Copy Password",
                "icon": "key",
                "entryPoint": {
                    "step": "action",
                    "selected": {"id": item_id},
                    "action": "copy_password",
                },
            }
        )
    if has_totp:
        actions.append(
            {
                "id": "copy_totp",
                "name": "Copy TOTP",
                "icon": "schedule",
                "entryPoint": {
                    "step": "action",
                    "selected": {"id": item_id},
                    "action": "copy_totp",
                },
            }
        )

    return {
        "id": f"bitwarden:{item_id}",
        "name": name,
        "description": username,
        "keywords": [username] if username else [],
        "icon": get_item_icon(item),
        "verb": "Copy Password" if has_password else "Copy Username",
        "actions": actions,
        # Default action: copy password if available, else username
        "entryPoint": {
            "step": "action",
            "selected": {"id": item_id},
            "action": "copy_password" if has_password else "copy_username",
        },
    }




def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    # Uses cached items only - does not require active session
    # Never includes passwords - uses entryPoint for secure execution
    if step == "index":
        cached_items = load_cached_items()
        if cached_items:
            items = [item_to_index_item(item) for item in cached_items]
            print(json.dumps({"type": "index", "items": items}))
        else:
            # No cache - return empty index
            print(json.dumps({"type": "index", "items": []}))
        return

    # Check bw is installed
    if not BW_PATH:
        respond_card(
            "Bitwarden CLI Required",
            "**Bitwarden CLI (`bw`) is not installed.**\n\n"
            "Install with: `npm install -g @bitwarden/cli`\n\n"
            "Then login and unlock:\n```bash\nbw login\nexport BW_SESSION=$(bw unlock --raw)\necho $BW_SESSION > ~/.bw_session\n```",
        )
        return

    # Get session
    session = get_session()
    if not session:
        respond_card(
            "Session Required",
            "**No Bitwarden session found.**\n\n"
            "Please login and unlock your vault:\n```bash\nbw login\nexport BW_SESSION=$(bw unlock --raw)\necho $BW_SESSION > ~/.bw_session\n```",
        )
        return

    if step == "initial":
        items = search_items("", session)
        if not items:
            # Might be locked or no items
            respond_card(
                "No Items Found",
                "Either your vault is empty, locked, or the session expired.\n\n"
                "Try unlocking:\n```bash\nexport BW_SESSION=$(bw unlock --raw)\necho $BW_SESSION > ~/.bw_session\n```",
            )
            return

        results = format_item_results(items)
        cache_age = get_cache_age()

        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                    "placeholder": "Search vault...",
                    "pluginActions": get_plugin_actions(cache_age),
                }
            )
        )
        return

    if step == "search":
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

        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                    "placeholder": "Search vault...",
                    "pluginActions": get_plugin_actions(cache_age),
                }
            )
        )
        return

    if step == "action":
        item_id = selected.get("id", "")

        # Plugin-level action: sync (from action bar)
        if item_id == "__plugin__" and action == "sync":
            run_bw(["sync"], session=session)
            clear_items_cache()
            items = search_items("", session, force_refresh=True)
            results = format_item_results(items)
            cache_age = get_cache_age()
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": results,
                        "inputMode": "realtime",
                        "placeholder": "Vault synced!",
                        "clearInput": True,
                        "pluginActions": get_plugin_actions(cache_age),
                    }
                )
            )
            return

        # Legacy sync button (keep for backwards compatibility)
        if item_id == "__sync__":
            run_bw(["sync"], session=session)
            clear_items_cache()
            items = search_items("", session, force_refresh=True)
            results = format_item_results(items)
            cache_age = get_cache_age()
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": results,
                        "inputMode": "realtime",
                        "placeholder": "Vault synced!",
                        "clearInput": True,
                        "pluginActions": get_plugin_actions(cache_age),
                    }
                )
            )
            return

        # No results placeholder
        if item_id == "__no_results__":
            return

        # Get item from cache first (fast), fall back to API (slow)
        item = get_cached_item(item_id)
        if not item:
            # Not in cache, fetch from API
            success, output = run_bw(["get", "item", item_id], session=session)
            if not success:
                respond_card("Error", f"Failed to get item: {output}")
                return
            try:
                item = json.loads(output)
            except json.JSONDecodeError:
                respond_card("Error", "Failed to parse item data")
                return

        login = item.get("login", {}) or {}
        username = login.get("username", "") or ""
        password = login.get("password", "") or ""
        name = item.get("name", "Unknown")

        # Copy username (from cache - instant)
        # Uses entryPoint for history so credentials aren't stored in command
        if action == "copy_username" and username:
            if not TEST_MODE:
                subprocess.run(["wl-copy", username], check=False)
            respond_execute(
                notify=f"Username copied: {username[:30]}{'...' if len(username) > 30 else ''}",
                name=f"Copy username: {name}",
                icon="person",
                entry_point={
                    "step": "action",
                    "selected": {"id": item_id},
                    "action": "copy_username",
                },
            )
            return

        # Copy password (from cache - instant)
        # Uses entryPoint - password is NEVER stored in history
        if action == "copy_password" and password:
            if not TEST_MODE:
                subprocess.run(["wl-copy", password], check=False)
            respond_execute(
                notify="Password copied to clipboard",
                name=f"Copy password: {name}",
                icon="key",
                entry_point={
                    "step": "action",
                    "selected": {"id": item_id},
                    "action": "copy_password",
                },
            )
            return

        # Copy TOTP (must fetch live - changes every 30s)
        # Uses entryPoint - TOTP must always be fetched fresh
        if action == "copy_totp":
            totp = get_totp(item_id, session)
            if totp:
                if not TEST_MODE:
                    subprocess.run(["wl-copy", totp], check=False)
                respond_execute(
                    notify=f"TOTP copied: {totp}",
                    name=f"Copy TOTP: {name}",
                    icon="schedule",
                    entry_point={
                        "step": "action",
                        "selected": {"id": item_id},
                        "action": "copy_totp",
                    },
                )
            else:
                respond_card("Error", "Failed to get TOTP code")
            return

        # Default: copy password or username (from cache - instant)
        # Uses entryPoint for history tracking
        if password:
            if not TEST_MODE:
                subprocess.run(["wl-copy", password], check=False)
            respond_execute(
                notify="Password copied to clipboard",
                name=f"Copy password: {name}",
                icon="key",
                entry_point={
                    "step": "action",
                    "selected": {"id": item_id},
                    "action": "copy_password",
                },
            )
        elif username:
            if not TEST_MODE:
                subprocess.run(["wl-copy", username], check=False)
            respond_execute(
                notify=f"Username copied: {username[:30]}...",
                name=f"Copy username: {name}",
                icon="person",
                entry_point={
                    "step": "action",
                    "selected": {"id": item_id},
                    "action": "copy_username",
                },
            )
        else:
            respond_card("Error", "No credentials to copy")


if __name__ == "__main__":
    main()
