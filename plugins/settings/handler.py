#!/usr/bin/env python3
"""
Settings plugin for hamr - Configure Hamr launcher options.

Socket-based daemon plugin providing access to hamr configuration through
an interactive settings browser with category-based navigation, search,
and live form editing.

Features:
- Browse settings by category
- Search all settings
- Edit settings via forms with live updates
- Reset individual settings or all settings
- Slider and switch controls
- Action bar hints configuration
"""

import json
import logging
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

logger = logging.getLogger(__name__)

CONFIG_PATH = Path.home() / ".config/hamr/config.json"

# Schema defining all configuration options and their metadata
SETTINGS_SCHEMA: dict = {
    "apps": {
        "terminal": {
            "default": "ghostty",
            "type": "string",
            "description": "Terminal emulator for shell actions",
        },
        "terminalArgs": {
            "default": "--class=floating.terminal",
            "type": "string",
            "description": "Terminal window class arguments",
        },
        "shell": {
            "default": "zsh",
            "type": "string",
            "description": "Shell for command execution (zsh, bash, fish)",
        },
    },
    "search": {
        "nonAppResultDelay": {
            "default": 30,
            "type": "number",
            "description": "Delay (ms) before showing non-app results",
        },
        "debounceMs": {
            "default": 50,
            "type": "number",
            "description": "Debounce for search input (ms)",
        },
        "pluginDebounceMs": {
            "default": 150,
            "type": "number",
            "description": "Plugin search debounce (ms)",
        },
        "maxHistoryItems": {
            "default": 500,
            "type": "number",
            "description": "Max search history entries",
        },
        "maxDisplayedResults": {
            "default": 16,
            "type": "number",
            "description": "Max results shown in launcher",
        },
        "maxRecentItems": {
            "default": 20,
            "type": "number",
            "description": "Max recent history items shown",
        },
        "diversityDecay": {
            "default": 0.7,
            "type": "slider",
            "min": 0,
            "max": 1,
            "step": 0.05,
            "description": "Decay factor for consecutive results from same plugin (lower = more diverse)",
        },
        "maxResultsPerPlugin": {
            "default": 0,
            "type": "number",
            "description": "Hard limit per plugin (0 = no limit, relies on decay only)",
        },
        "pluginRankingBonus": {
            "default": {},
            "type": "json",
            "description": 'Per-plugin ranking bonus (e.g., {"apps": 200, "settings": 150})',
        },
        "shellHistoryLimit": {
            "default": 50,
            "type": "number",
            "description": "Shell history results limit",
        },
        "engineBaseUrl": {
            "default": "https://www.google.com/search?q=",
            "type": "string",
            "description": "Web search engine base URL",
        },
        "excludedSites": {
            "default": ["quora.com", "facebook.com"],
            "type": "list",
            "description": "Sites to exclude from web search",
        },
        "actionKeys": {
            "default": ["u", "i", "o", "p"],
            "type": "list",
            "description": "Action button shortcuts (Ctrl + key)",
        },
    },
    "search.shellHistory": {
        "enable": {
            "default": True,
            "type": "boolean",
            "description": "Enable shell history integration",
        },
        "shell": {
            "default": "auto",
            "type": "string",
            "description": "Shell type (auto, zsh, bash, fish)",
        },
        "customHistoryPath": {
            "default": "",
            "type": "string",
            "description": "Custom shell history file path",
        },
        "maxEntries": {
            "default": 500,
            "type": "number",
            "description": "Max shell history entries to load",
        },
    },
    "imageBrowser": {
        "useSystemFileDialog": {
            "default": False,
            "type": "boolean",
            "description": "Use system file dialog instead of built-in",
        },
        "columns": {
            "default": 4,
            "type": "number",
            "description": "Grid columns in image browser",
        },
        "cellAspectRatio": {
            "default": 1.333,
            "type": "number",
            "description": "Cell aspect ratio (4:3 = 1.333)",
        },
        "sidebarWidth": {
            "default": 140,
            "type": "number",
            "description": "Quick dirs sidebar width (px)",
        },
    },
    "behavior": {
        "stateRestoreWindowMs": {
            "default": 30000,
            "type": "number",
            "description": "Time (ms) to preserve state after soft close",
        },
        "clickOutsideAction": {
            "default": "intuitive",
            "type": "select",
            "options": ["intuitive", "close", "minimize"],
            "description": "Action when clicking outside (intuitive/close/minimize)",
        },
    },
    "appearance": {
        "backgroundTransparency": {
            "default": 0.2,
            "type": "slider",
            "min": 0,
            "max": 1,
            "step": 0.05,
            "description": "Background transparency (0=opaque, 1=transparent)",
        },
        "contentTransparency": {
            "default": 0.2,
            "type": "slider",
            "min": 0,
            "max": 1,
            "step": 0.05,
            "description": "Content transparency (0=opaque, 1=transparent)",
        },
        "launcherXRatio": {
            "default": 0.5,
            "type": "slider",
            "min": 0,
            "max": 1,
            "step": 0.05,
            "description": "Launcher X position (0=left, 0.5=center, 1=right)",
        },
        "launcherYRatio": {
            "default": 0.1,
            "type": "slider",
            "min": 0,
            "max": 1,
            "step": 0.05,
            "description": "Launcher Y position (0=top, 0.5=center, 1=bottom)",
        },
        "fontScale": {
            "default": 1.0,
            "type": "slider",
            "min": 0.75,
            "max": 1.5,
            "step": 0.05,
            "description": "Font scale (0.75=75%, 1.0=100%, 1.5=150%)",
        },
        "uiScale": {
            "default": 1.0,
            "type": "slider",
            "min": 0.8,
            "max": 1.5,
            "step": 0.1,
            "description": "UI scale (0.8=compact, 1.0=default, 1.5=large)",
        },
    },
    "sizes": {
        "searchWidth": {
            "default": 640,
            "type": "slider",
            "min": 400,
            "max": 1000,
            "step": 20,
            "description": "Launcher width (px)",
        },
        "maxResultsHeight": {
            "default": 600,
            "type": "slider",
            "min": 300,
            "max": 900,
            "step": 50,
            "description": "Max results panel height (px)",
        },
    },
    "fonts": {
        "main": {
            "default": "Google Sans Flex",
            "type": "string",
            "description": "Main UI font",
        },
        "monospace": {
            "default": "JetBrains Mono NF",
            "type": "string",
            "description": "Monospace font for code",
        },
        "reading": {
            "default": "Readex Pro",
            "type": "string",
            "description": "Reading/content font",
        },
        "icon": {
            "default": "Material Symbols Rounded",
            "type": "string",
            "description": "Icon font family",
        },
    },
    "paths": {
        "wallpaperDir": {
            "default": "",
            "type": "string",
            "description": "Wallpaper directory (empty=~/Pictures/Wallpapers)",
        },
        "colorsJson": {
            "default": "",
            "type": "string",
            "description": "Material theme colors.json path",
        },
    },
}

CATEGORY_ICONS = {
    "apps": "terminal",
    "search": "search",
    "search.shellHistory": "history",
    "imageBrowser": "image",
    "behavior": "psychology",
    "appearance": "palette",
    "sizes": "straighten",
    "fonts": "font_download",
    "paths": "folder",
}

CATEGORY_NAMES = {
    "apps": "Apps",
    "search": "Search",
    "search.shellHistory": "Shell History",
    "imageBrowser": "Image Browser",
    "behavior": "Behavior",
    "appearance": "Appearance",
    "sizes": "Sizes",
    "fonts": "Fonts",
    "paths": "Paths",
}

DEFAULT_ACTION_BAR_HINTS = [
    {"prefix": "~", "icon": "folder", "label": "Files", "plugin": "files"},
    {
        "prefix": ";",
        "icon": "content_paste",
        "label": "Clipboard",
        "plugin": "clipboard",
    },
    {"prefix": "!", "icon": "terminal", "label": "Shell", "plugin": "shell"},
    {"prefix": "=", "icon": "calculate", "label": "Math", "plugin": "calculate"},
    {"prefix": ":", "icon": "emoji_emotions", "label": "Emoji", "plugin": "emoji"},
]

# Reserved prefixes that cannot be used for action bar hints
RESERVED_PREFIXES = ["/"]


def load_config() -> dict:
    """Load configuration from file."""
    if not CONFIG_PATH.exists():
        return {}
    try:
        with open(CONFIG_PATH) as f:
            return json.load(f)
    except Exception:
        return {}


def save_config(config: dict) -> bool:
    """Save configuration to file."""
    try:
        CONFIG_PATH.parent.mkdir(parents=True, exist_ok=True)
        with open(CONFIG_PATH, "w") as f:
            json.dump(config, f, indent=2)
        return True
    except Exception:
        return False


def get_nested_value(config: dict, path: str, default=None):
    """Get a nested value from config using dot notation."""
    keys = path.split(".")
    obj = config
    for key in keys:
        if not isinstance(obj, dict) or key not in obj:
            return default
        obj = obj[key]
    return obj


def set_nested_value(config: dict, path: str, value) -> dict:
    """Set a nested value in config using dot notation."""
    keys = path.split(".")
    obj = config
    for key in keys[:-1]:
        if key not in obj or not isinstance(obj[key], dict):
            obj[key] = {}
        obj = obj[key]
    obj[keys[-1]] = value
    return config


def delete_nested_value(config: dict, path: str) -> dict:
    """Delete a nested value from config."""
    keys = path.split(".")
    obj = config
    for key in keys[:-1]:
        if key not in obj or not isinstance(obj[key], dict):
            return config
        obj = obj[key]
    if keys[-1] in obj:
        del obj[keys[-1]]
    return config


def get_current_value(config: dict, category: str, key: str):
    """Get current value for a setting, falling back to default."""
    schema = SETTINGS_SCHEMA.get(category, {}).get(key, {})
    default = schema.get("default")
    path = f"{category}.{key}"
    return get_nested_value(config, path, default)


def is_modified(config: dict, category: str, key: str) -> bool:
    """Check if a setting is modified from its default."""
    schema = SETTINGS_SCHEMA.get(category, {}).get(key, {})
    default = schema.get("default")
    current = get_current_value(config, category, key)
    return current != default


def count_modified_in_category(config: dict, category: str) -> int:
    """Count how many settings in a category are modified from default."""
    schema = SETTINGS_SCHEMA.get(category, {})
    modified = 0
    for key, info in schema.items():
        default = info.get("default")
        current = get_current_value(config, category, key)
        if current != default:
            modified += 1
    return modified


def format_value(value) -> str:
    """Format a value for display."""
    if isinstance(value, bool):
        return "Yes" if value else "No"
    if isinstance(value, list):
        return ", ".join(str(v) for v in value)
    if isinstance(value, dict):
        if not value:
            return "(empty)"
        return json.dumps(value)
    if value == "" or value is None:
        return "(empty)"
    return str(value)


def get_action_bar_hints(config: dict) -> list[dict]:
    """Get current action bar hints from config, parsing JSON string."""
    hints_json = get_nested_value(config, "search.actionBarHintsJson", None)
    if hints_json and isinstance(hints_json, str):
        try:
            hints = json.loads(hints_json)
            if isinstance(hints, list):
                return hints
        except (json.JSONDecodeError, TypeError):
            pass
    return DEFAULT_ACTION_BAR_HINTS


def is_hint_modified(hint: dict, default_hint: dict) -> bool:
    """Check if a hint differs from its default."""
    for key in ("prefix", "icon", "label", "plugin"):
        if hint.get(key, "") != default_hint.get(key, ""):
            return True
    return False


def get_categories() -> list[dict]:
    """Get list of categories."""
    config = load_config()
    results = []

    for category in SETTINGS_SCHEMA:
        settings_count = len(SETTINGS_SCHEMA[category])
        modified_count = count_modified_in_category(config, category)

        item: dict = {
            "id": f"category:{category}",
            "name": CATEGORY_NAMES.get(category, category),
            "description": f"{settings_count} settings",
            "icon": CATEGORY_ICONS.get(category, "settings"),
            "verb": "Browse",
        }

        if modified_count > 0:
            item["chips"] = [
                {
                    "text": f"{modified_count} modified",
                    "icon": "edit",
                    "color": "#4caf50",
                }
            ]

        results.append(item)

    # Add special category for action bar hints
    hints = get_action_bar_hints(config)
    hints_modified_count = sum(
        1
        for i, hint in enumerate(hints)
        if i < len(DEFAULT_ACTION_BAR_HINTS)
        and is_hint_modified(hint, DEFAULT_ACTION_BAR_HINTS[i])
    )
    hint_item: dict = {
        "id": "category:actionBarHints",
        "name": "Action Bar Hints",
        "description": f"{len(hints)} shortcuts",
        "icon": "keyboard_command_key",
        "verb": "Configure",
    }
    if hints_modified_count > 0:
        hint_item["chips"] = [
            {
                "text": f"{hints_modified_count} modified",
                "icon": "edit",
                "color": "#4caf50",
            }
        ]
    results.append(hint_item)

    return results


def get_action_bar_hints_list(config: dict) -> list[dict]:
    """Get action bar hints as a list of result items."""
    hints = get_action_bar_hints(config)
    results = []

    for i, hint in enumerate(hints):
        prefix = hint.get("prefix", "")
        icon = hint.get("icon", "extension")
        label = hint.get("label", "")
        plugin = hint.get("plugin", "")

        default_hint = (
            DEFAULT_ACTION_BAR_HINTS[i] if i < len(DEFAULT_ACTION_BAR_HINTS) else {}
        )
        modified = is_hint_modified(hint, default_hint)

        item: dict = {
            "id": f"actionHint:{i}",
            "name": f"{prefix} {label}",
            "description": f"Opens {plugin}",
            "icon": icon,
            "verb": "Edit",
            "actions": [
                {"id": "reset", "name": "Reset to Default", "icon": "restart_alt"},
            ],
        }

        if modified:
            item["badges"] = [{"text": "*", "color": "#4caf50"}]

        results.append(item)

    return results


def get_settings_for_category(config: dict, category: str) -> list[dict]:
    """Get settings list for a specific category."""
    results = []

    # Special handling for action bar hints
    if category == "actionBarHints":
        return get_action_bar_hints_list(config)

    schema = SETTINGS_SCHEMA.get(category, {})
    for key, info in schema.items():
        setting_type = info.get("type", "string")
        current = get_current_value(config, category, key)
        default = info.get("default")
        modified = current != default

        result: dict = {
            "id": f"setting:{category}.{key}",
            "name": key,
            "description": format_value(current),
            "icon": get_type_icon(setting_type),
        }

        if modified:
            result["badges"] = [{"text": "*", "color": "#4caf50"}]

        if setting_type == "readonly":
            result["description"] = info.get("description", "")
        elif setting_type == "boolean":
            result["type"] = "switch"
            # Value must be a SliderValue struct, not a boolean
            result["value"] = {
                "value": 1.0 if current else 0.0,
                "min": 0.0,
                "max": 1.0,
                "step": 1.0,
            }
            result["description"] = info.get("description", "")
            result["actions"] = [
                {"id": "reset", "name": "Reset to Default", "icon": "restart_alt"},
            ]
        elif setting_type == "slider":
            slider_default = info.get("default", 0)
            slider_value = current if current is not None else slider_default
            result["type"] = "slider"
            # Value must be a SliderValue struct with value, min, max, step
            result["value"] = {
                "value": float(slider_value)
                if isinstance(slider_value, (int, float))
                else 0.0,
                "min": float(info.get("min", 0)),
                "max": float(info.get("max", 1)),
                "step": float(info.get("step", 0.05)),
            }
            result["description"] = info.get("description", "")
            result["actions"] = [
                {
                    "id": "reset",
                    "name": f"Reset to {slider_default}",
                    "icon": "restart_alt",
                },
            ]
        elif setting_type == "select":
            result["description"] = f"{current} | {info.get('description', '')}"
            result["verb"] = "Edit"
            result["chips"] = [{"text": str(current)}]
            result["actions"] = [
                {"id": "reset", "name": "Reset to Default", "icon": "restart_alt"},
            ]
        else:
            result["verb"] = "Edit"
            result["actions"] = [
                {"id": "reset", "name": "Reset to Default", "icon": "restart_alt"},
            ]

        results.append(result)

    return results


def get_type_icon(setting_type: str) -> str:
    """Get icon for setting type."""
    icons = {
        "string": "text_fields",
        "number": "123",
        "slider": "tune",
        "boolean": "toggle_on",
        "list": "list",
        "readonly": "info",
        "select": "arrow_drop_down",
        "json": "data_object",
    }
    return icons.get(setting_type, "settings")


def parse_value(value_str, setting_type: str, default):
    """Parse string value to correct type."""
    if setting_type == "boolean":
        if isinstance(value_str, bool):
            return value_str
        return str(value_str).lower() in ("true", "yes", "1")
    if setting_type == "slider":
        if isinstance(value_str, (int, float)):
            return float(value_str)
        try:
            return float(value_str)
        except (ValueError, TypeError):
            return default
    if setting_type == "number":
        try:
            if isinstance(value_str, (int, float)):
                return value_str
            if "." in str(value_str):
                return float(value_str)
            return int(value_str)
        except (ValueError, TypeError):
            return default
    if setting_type == "list":
        if isinstance(value_str, list):
            return value_str
        if not str(value_str).strip():
            return []
        return [v.strip() for v in str(value_str).split(",")]
    if setting_type == "json":
        if isinstance(value_str, dict):
            return value_str
        if not str(value_str).strip():
            return default
        try:
            return json.loads(value_str)
        except (json.JSONDecodeError, TypeError):
            return default
    return value_str


def show_edit_form(category: str, key: str, schema: dict, current_value):
    """Show form for editing a setting."""
    setting_type = schema.get("type", "string")
    default = schema.get("default")
    description = schema.get("description", "")

    if setting_type == "boolean":
        fields = [
            {
                "id": "value",
                "type": "switch",
                "label": key,
                "default_value": str(current_value).lower()
                if current_value is not None
                else str(default).lower(),
                "hint": f"{description}\nDefault: {'Yes' if default else 'No'}",
            }
        ]
    elif setting_type == "select":
        options = schema.get("options", [])
        fields = [
            {
                "id": "value",
                "type": "select",
                "label": key,
                "options": [{"value": opt, "label": opt} for opt in options],
                "default_value": str(current_value) if current_value else str(default),
                "hint": f"{description}\nDefault: {default}",
            }
        ]
    elif setting_type == "slider":
        min_val = schema.get("min", 0)
        max_val = schema.get("max", 100)
        step_val = schema.get("step", 1)
        fields = [
            {
                "id": "value",
                "type": "slider",
                "label": key,
                "min": min_val,
                "max": max_val,
                "step": step_val,
                "default_value": str(current_value)
                if current_value is not None
                else str(default),
                "hint": f"{description}\nDefault: {default}",
            }
        ]
    elif setting_type == "list":
        fields = [
            {
                "id": "value",
                "type": "text",
                "label": key,
                "default_value": ", ".join(str(v) for v in current_value)
                if current_value
                else "",
                "hint": f"{description}\nDefault: {', '.join(str(v) for v in (default or []))}\nEnter comma-separated values",
            }
        ]
    elif setting_type == "json":
        fields = [
            {
                "id": "value",
                "type": "textarea",
                "label": key,
                "default_value": json.dumps(current_value, indent=2)
                if current_value
                else "{}",
                "hint": f"{description}\nDefault: {json.dumps(default)}\nEnter valid JSON",
            }
        ]
    else:
        # string, number, and other types
        fields = [
            {
                "id": "value",
                "type": "number" if setting_type == "number" else "text",
                "label": key,
                "default_value": str(current_value)
                if current_value is not None
                else "",
                "hint": f"{description}\nDefault: {default}",
            }
        ]

    return {
        "type": "form",
        "form": {
            "title": f"Edit: {key}",
            "submit_label": "Save",
            "cancel_label": "Cancel",
            "fields": fields,
        },
        "context": f"edit:{category}.{key}",
    }


def get_plugin_actions() -> list[dict]:
    """Get plugin-level actions."""
    return [
        {
            "id": "clear_cache",
            "name": "Clear Cache",
            "icon": "delete_sweep",
            "confirm": "Clear plugin index cache? Plugins will reindex on next launch.",
        },
        {
            "id": "reset_all",
            "name": "Reset All",
            "icon": "restart_alt",
            "confirm": "Reset all settings to defaults? This cannot be undone.",
        },
    ]


plugin = HamrPlugin(
    id="settings",
    name="Settings",
    description="Configure Hamr launcher options",
    icon="settings",
)


@plugin.on_initial
def handle_initial(params=None):
    """Handle initial request."""
    return HamrPlugin.results(
        get_categories(),
        input_mode="realtime",
        placeholder="Search settings or select category...",
        plugin_actions=get_plugin_actions(),
    )


@plugin.on_search
def handle_search(query: str, context: str | None):
    """Handle search request."""
    config = load_config()

    if context and context.startswith("category:"):
        category = context.split(":", 1)[1]
        settings = get_settings_for_category(config, category)

        # Filter by query
        if query:
            query_lower = query.lower()
            settings = [
                s
                for s in settings
                if query_lower in s.get("name", "").lower()
                or query_lower in s.get("description", "").lower()
            ]

        placeholder = (
            "Configure action bar shortcuts..."
            if category == "actionBarHints"
            else f"Filter {CATEGORY_NAMES.get(category, category)} settings..."
        )

        return HamrPlugin.results(
            settings,
            input_mode="realtime",
            placeholder=placeholder,
            context=context,
            plugin_actions=get_plugin_actions(),
        )
    else:
        if query:
            # Search all settings
            all_settings = []
            for category in SETTINGS_SCHEMA:
                for key in SETTINGS_SCHEMA[category]:
                    current = get_current_value(config, category, key)
                    setting_type = SETTINGS_SCHEMA[category][key].get("type", "string")

                    result = {
                        "id": f"setting:{category}.{key}",
                        "name": key,
                        "description": f"{CATEGORY_NAMES.get(category, category)} | {format_value(current)}",
                        "icon": get_type_icon(setting_type),
                    }

                    all_settings.append(result)

            query_lower = query.lower()
            filtered = [
                s
                for s in all_settings
                if query_lower in s.get("name", "").lower()
                or query_lower in s.get("description", "").lower()
            ]

            return HamrPlugin.results(
                filtered,
                input_mode="realtime",
                placeholder="Search settings or select category...",
                plugin_actions=get_plugin_actions(),
            )
        else:
            return HamrPlugin.results(
                get_categories(),
                input_mode="realtime",
                placeholder="Search settings or select category...",
                plugin_actions=get_plugin_actions(),
            )


@plugin.on_action
def handle_action(item_id: str, action: str | None, context: str | None):
    """Handle action request."""
    logger.debug(
        f"handle_action: item_id={item_id}, action={action}, context={context}"
    )

    config = load_config()

    # Plugin actions
    if item_id == "__plugin__":
        if action == "reset_all":
            if save_config({}):
                return HamrPlugin.results(
                    get_categories(),
                    input_mode="realtime",
                    clear_input=True,
                    placeholder="Search settings or select category...",
                    plugin_actions=get_plugin_actions(),
                )
            else:
                return HamrPlugin.error("Failed to reset config")
        elif action == "clear_cache":
            cache_path = Path.home() / ".config/hamr/plugin-indexes.json"
            try:
                if cache_path.exists():
                    cache_path.unlink()
                return HamrPlugin.execute()
            except Exception as e:
                return HamrPlugin.error(f"Failed to clear cache: {e}")

    # Back navigation - return to categories
    if item_id == "__back__":
        if context and context.startswith("edit:"):
            # Going back from edit form to category view
            # Context is "edit:category.key", extract category
            path = context.split(":", 1)[1]  # "category.key"
            parts = path.rsplit(".", 1)
            if len(parts) == 2:
                category = parts[0]
                settings = get_settings_for_category(config, category)
                placeholder = (
                    "Configure action bar shortcuts..."
                    if category == "actionBarHints"
                    else f"Filter {CATEGORY_NAMES.get(category, category)} settings..."
                )
                return HamrPlugin.results(
                    settings,
                    input_mode="realtime",
                    clear_input=True,
                    context=f"category:{category}",
                    placeholder=placeholder,
                    plugin_actions=get_plugin_actions(),
                )
            # Fallback if parse fails
            return HamrPlugin.results(
                get_categories(),
                input_mode="realtime",
                clear_input=True,
                context="",
                placeholder="Search settings or select category...",
                plugin_actions=get_plugin_actions(),
            )
        elif context and (
            context.startswith("category:") or context.startswith("liveform:")
        ):
            # Going back from a category or live form to the main categories list
            return HamrPlugin.results(
                get_categories(),
                input_mode="realtime",
                clear_input=True,
                context="",
                placeholder="Search settings or select category...",
                plugin_actions=get_plugin_actions(),
                navigation_depth=0,  # Jump back to root
            )
        else:
            # Default: return to categories
            return HamrPlugin.results(
                get_categories(),
                input_mode="realtime",
                clear_input=True,
                context="",
                placeholder="Search settings or select category...",
                plugin_actions=get_plugin_actions(),
            )

    # Category selection
    if item_id.startswith("category:"):
        category = item_id.split(":", 1)[1]
        logger.debug(f"Category selection: {category}")
        settings = get_settings_for_category(config, category)
        logger.debug(f"Got {len(settings)} settings")

        placeholder = (
            "Configure action bar shortcuts..."
            if category == "actionBarHints"
            else f"Filter {CATEGORY_NAMES.get(category, category)} settings..."
        )

        result = HamrPlugin.results(
            settings,
            input_mode="realtime",
            clear_input=True,
            context=f"category:{category}",
            placeholder=placeholder,
            plugin_actions=get_plugin_actions(),
            navigate_forward=True,
        )
        logger.debug(f"Returning result with {len(result['results'])} items")
        return result

    # Action bar hint actions
    if item_id.startswith("actionHint:"):
        hint_idx = int(item_id.split(":", 1)[1])
        hints = get_action_bar_hints(config)
        default_hint = (
            DEFAULT_ACTION_BAR_HINTS[hint_idx]
            if hint_idx < len(DEFAULT_ACTION_BAR_HINTS)
            else {}
        )

        if action == "reset":
            if hint_idx < len(hints):
                hints[hint_idx] = default_hint.copy()
                config = set_nested_value(
                    config, "search.actionBarHintsJson", json.dumps(hints)
                )
                save_config(config)
            settings = get_action_bar_hints_list(config)
            return HamrPlugin.results(
                settings,
                input_mode="realtime",
                context="category:actionBarHints",
                placeholder="Configure action bar shortcuts...",
                plugin_actions=get_plugin_actions(),
            )

        # Show form for editing action hint
        hint_idx = int(item_id.split(":", 1)[1])
        hints = get_action_bar_hints(config)
        default_hint = (
            DEFAULT_ACTION_BAR_HINTS[hint_idx]
            if hint_idx < len(DEFAULT_ACTION_BAR_HINTS)
            else {}
        )
        current_hint = hints[hint_idx] if hint_idx < len(hints) else {}

        return HamrPlugin.form(
            {
                "title": f"Edit Action {hint_idx + 1}",
                "submit_label": "Save",
                "cancel_label": "Cancel",
                "fields": [
                    {
                        "id": "prefix",
                        "type": "text",
                        "label": "Prefix",
                        "default_value": current_hint.get("prefix", ""),
                        "hint": f"Keyboard shortcut (e.g., ~, ;, !)\nDefault: {default_hint.get('prefix', '')}\nNote: '/' is reserved for plugin management",
                    },
                    {
                        "id": "plugin",
                        "type": "text",
                        "label": "Plugin",
                        "default_value": current_hint.get("plugin", ""),
                        "hint": f"Plugin to open (e.g., files, clipboard, emoji)\nDefault: {default_hint.get('plugin', '')}",
                    },
                    {
                        "id": "label",
                        "type": "text",
                        "label": "Label",
                        "default_value": current_hint.get("label", ""),
                        "hint": f"Display label\nDefault: {default_hint.get('label', '')}",
                    },
                    {
                        "id": "icon",
                        "type": "text",
                        "label": "Icon",
                        "default_value": current_hint.get("icon", ""),
                        "hint": f"Material icon name\nDefault: {default_hint.get('icon', '')}",
                    },
                ],
            },
            context=f"editActionHint:{hint_idx}",
        )

    # Setting actions
    if item_id.startswith("setting:"):
        path = item_id.split(":", 1)[1]
        parts = path.rsplit(".", 1)
        if len(parts) == 2:
            category, key = parts
        else:
            return {"type": "error", "message": "Invalid setting path"}

        schema = SETTINGS_SCHEMA.get(category, {}).get(key, {})
        if not schema:
            return {"type": "error", "message": "Setting not found"}

        if action == "reset":
            config = delete_nested_value(config, path)
            save_config(config)
            settings = get_settings_for_category(config, category)

            return HamrPlugin.results(
                settings,
                input_mode="realtime",
                clear_input=True,
                context=f"category:{category}",
                placeholder=f"Filter {CATEGORY_NAMES.get(category, category)} settings...",
                plugin_actions=get_plugin_actions(),
            )

        # Show form for editing setting
        current = get_current_value(config, category, key)
        return show_edit_form(category, key, schema, current)

    return {}


@plugin.on_slider_changed
def handle_slider_changed(slider_id: str, value: float):
    """Handle slider changes in settings."""
    if slider_id.startswith("setting:"):
        path = slider_id.split(":", 1)[1]
        config = load_config()
        # Size settings need integer values for GTK config parsing
        parts = path.rsplit(".", 1)
        if len(parts) == 2:
            category, key = parts
            schema = SETTINGS_SCHEMA.get(category, {}).get(key, {})
            # Use int for size settings (pixels), float for appearance (ratios)
            if category == "sizes":
                config = set_nested_value(config, path, int(value))
            else:
                config = set_nested_value(config, path, float(value))
        else:
            config = set_nested_value(config, path, float(value))
        save_config(config)
    return {"type": "noop"}


@plugin.on_switch_toggled
def handle_switch_toggled(switch_id: str, value: bool):
    """Handle switch toggles in settings."""
    if switch_id.startswith("setting:"):
        path = switch_id.split(":", 1)[1]
        config = load_config()
        config = set_nested_value(config, path, bool(value))
        save_config(config)
    return {"type": "noop"}


@plugin.on_form_submitted
def handle_form_submitted(form_data: dict, context: str | None):
    """Handle form submission for settings and action hint editing."""
    if not context:
        return {"type": "error", "message": "Invalid context"}

    config = load_config()

    # Handle action hint editing
    if context.startswith("editActionHint:"):
        hint_idx = int(context.split(":", 1)[1])
        hints = get_action_bar_hints(config)

        # Validate prefix is not reserved
        new_prefix = form_data.get("prefix", "").strip()
        if new_prefix in RESERVED_PREFIXES:
            return HamrPlugin.error(
                f"'{new_prefix}' is reserved for plugin management and cannot be used"
            )

        # Ensure we have enough hints
        while len(hints) <= hint_idx:
            hints.append({"prefix": "", "icon": "", "label": "", "plugin": ""})

        # Update the hint with form data
        hints[hint_idx] = {
            "prefix": new_prefix,
            "plugin": form_data.get("plugin", "").strip(),
            "label": form_data.get("label", "").strip(),
            "icon": form_data.get("icon", "").strip(),
        }

        config = set_nested_value(
            config, "search.actionBarHintsJson", json.dumps(hints)
        )
        if not save_config(config):
            return HamrPlugin.error("Failed to save config")

        # Return to action hints list
        settings = get_action_bar_hints_list(config)
        return HamrPlugin.results(
            settings,
            input_mode="realtime",
            clear_input=True,
            context="category:actionBarHints",
            placeholder="Configure action bar shortcuts...",
            plugin_actions=get_plugin_actions(),
        )

    # Handle setting editing
    if context.startswith("edit:"):
        path = context.split(":", 1)[1]
        parts = path.rsplit(".", 1)
        if len(parts) != 2:
            return HamrPlugin.error("Invalid setting path")

        category, key = parts

        schema = SETTINGS_SCHEMA.get(category, {}).get(key, {})
        if not schema:
            return HamrPlugin.error("Setting not found")

        # Get the value from form data
        value_str = form_data.get("value", "")
        setting_type = schema.get("type", "string")
        default = schema.get("default")

        # Parse value based on type
        new_value = parse_value(value_str, setting_type, default)

        # Save to config
        config = set_nested_value(config, path, new_value)
        if not save_config(config):
            return HamrPlugin.error("Failed to save config")

        # Return to category view with updated settings
        settings = get_settings_for_category(config, category)
        return HamrPlugin.results(
            settings,
            input_mode="realtime",
            clear_input=True,
            context=f"category:{category}",
            placeholder=f"Filter {CATEGORY_NAMES.get(category, category)} settings...",
            plugin_actions=get_plugin_actions(),
        )

    return HamrPlugin.error("Invalid context")


if __name__ == "__main__":
    plugin.run()
