#!/usr/bin/env python3
"""
Settings plugin - Configure Hamr launcher options
Reads/writes config from ~/.config/hamr/config.json

Features:
- Browse settings by category
- Search all settings from initial view
- Filter within category when navigated
- Edit settings via form
- Reset all settings to defaults
"""

import json
import os
import sys
from pathlib import Path

CONFIG_PATH = Path.home() / ".config/hamr/config.json"

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
    },
    "sizes": {
        "searchWidth": {
            "default": 580,
            "type": "number",
            "description": "Launcher search bar width (px)",
        },
        "searchInputHeight": {
            "default": 40,
            "type": "number",
            "description": "Search input height (px)",
        },
        "maxResultsHeight": {
            "default": 600,
            "type": "number",
            "description": "Max results panel height (px)",
        },
        "resultIconSize": {
            "default": 40,
            "type": "number",
            "description": "Result item icon size (px)",
        },
        "imageBrowserWidth": {
            "default": 1200,
            "type": "number",
            "description": "Image browser width (legacy panel, px)",
        },
        "imageBrowserHeight": {
            "default": 690,
            "type": "number",
            "description": "Image browser height (legacy panel, px)",
        },
        "imageBrowserGridWidth": {
            "default": 900,
            "type": "number",
            "description": "Image browser grid width (integrated, px)",
        },
        "imageBrowserGridHeight": {
            "default": 600,
            "type": "number",
            "description": "Image browser grid height (integrated, px)",
        },
        "windowPickerMaxWidth": {
            "default": 350,
            "type": "number",
            "description": "Window picker preview max width (px)",
        },
        "windowPickerMaxHeight": {
            "default": 220,
            "type": "number",
            "description": "Window picker preview max height (px)",
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
    {"prefix": "/", "icon": "extension", "label": "Plugins", "plugin": "plugins"},
    {"prefix": "!", "icon": "terminal", "label": "Shell", "plugin": "shell"},
    {"prefix": "=", "icon": "calculate", "label": "Math", "plugin": "calculate"},
    {"prefix": ":", "icon": "emoji_emotions", "label": "Emoji", "plugin": "emoji"},
]


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


def load_config() -> dict:
    if not CONFIG_PATH.exists():
        return {}
    try:
        with open(CONFIG_PATH) as f:
            return json.load(f)
    except Exception:
        return {}


def save_config(config: dict) -> bool:
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


def fuzzy_match(query: str, text: str) -> bool:
    """Simple fuzzy match - all query chars appear in order."""
    query = query.lower()
    text = text.lower()
    qi = 0
    for c in text:
        if qi < len(query) and c == query[qi]:
            qi += 1
    return qi == len(query)


def format_value(value) -> str:
    """Format a value for display."""
    if isinstance(value, bool):
        return "Yes" if value else "No"
    if isinstance(value, list):
        return ", ".join(str(v) for v in value)
    if value == "" or value is None:
        return "(empty)"
    return str(value)


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


def is_modified(config: dict, category: str, key: str) -> bool:
    """Check if a setting is modified from its default."""
    schema = SETTINGS_SCHEMA.get(category, {}).get(key, {})
    default = schema.get("default")
    current = get_current_value(config, category, key)
    return current != default


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
            result["value"] = bool(current)
            result["description"] = info.get("description", "")
            result["actions"] = [
                {"id": "reset", "name": "Reset to Default", "icon": "restart_alt"},
            ]
        elif setting_type == "slider":
            slider_default = info.get("default", 0)
            slider_value = current if current is not None else slider_default
            result["type"] = "slider"
            result["value"] = (
                float(slider_value) if isinstance(slider_value, (int, float)) else 0.0
            )
            result["min"] = info.get("min", 0)
            result["max"] = info.get("max", 1)
            result["step"] = info.get("step", 0.05)
            result["description"] = info.get("description", "")
            result["actions"] = [
                {
                    "id": "reset",
                    "name": f"Reset to {slider_default}",
                    "icon": "restart_alt",
                },
            ]
        elif setting_type == "select":
            options = info.get("options", [])
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


def get_all_settings(config: dict) -> list[dict]:
    """Get all settings as a flat list."""
    results = []
    for category, settings in SETTINGS_SCHEMA.items():
        for key, info in settings.items():
            setting_type = info.get("type", "string")

            # Skip actionBarHintsJson - we show individual actions instead
            if setting_type == "actionbarhints":
                continue

            current = get_current_value(config, category, key)
            default = info.get("default")
            modified = current != default

            result: dict = {
                "id": f"setting:{category}.{key}",
                "name": key,
                "icon": get_type_icon(setting_type),
                "category": category,
            }

            if modified:
                result["badges"] = [{"text": "*", "color": "#4caf50"}]

            if setting_type == "readonly":
                result["description"] = (
                    f"{CATEGORY_NAMES.get(category, category)} | {info.get('description', '')}"
                )
            elif setting_type == "boolean":
                result["type"] = "switch"
                result["value"] = bool(current)
                result["description"] = (
                    f"{CATEGORY_NAMES.get(category, category)} | {info.get('description', '')}"
                )
                result["actions"] = [
                    {
                        "id": "reset",
                        "name": "Reset to Default",
                        "icon": "restart_alt",
                    },
                ]
            elif setting_type == "slider":
                slider_value = current if current is not None else default
                result["type"] = "slider"
                result["value"] = (
                    float(slider_value)
                    if isinstance(slider_value, (int, float))
                    else 0.0
                )
                result["min"] = info.get("min", 0)
                result["max"] = info.get("max", 1)
                result["step"] = info.get("step", 0.05)
                result["description"] = (
                    f"{CATEGORY_NAMES.get(category, category)} | {info.get('description', '')}"
                )
                result["actions"] = [
                    {
                        "id": "reset",
                        "name": f"Reset to {default}",
                        "icon": "restart_alt",
                    },
                ]
            elif setting_type == "select":
                result["description"] = (
                    f"{CATEGORY_NAMES.get(category, category)} | {current}"
                )
                result["verb"] = "Edit"
                result["chips"] = [{"text": str(current)}]
                result["actions"] = [
                    {
                        "id": "reset",
                        "name": "Reset to Default",
                        "icon": "restart_alt",
                    },
                ]
            else:
                result["description"] = (
                    f"{CATEGORY_NAMES.get(category, category)} | {format_value(current)}"
                )
                result["verb"] = "Edit"
                result["actions"] = [
                    {
                        "id": "reset",
                        "name": "Reset to Default",
                        "icon": "restart_alt",
                    },
                ]

            results.append(result)

    return results


def filter_settings(settings: list[dict], query: str) -> list[dict]:
    """Filter settings by query matching name or description."""
    if not query:
        return settings
    results = []
    for setting in settings:
        name = setting.get("name", "")
        desc = setting.get("description", "")
        if fuzzy_match(query, name) or fuzzy_match(query, desc):
            results.append(setting)
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
    }
    return icons.get(setting_type, "settings")


def get_form_field_type(setting_type: str) -> str:
    """Map setting type to form field type."""
    if setting_type == "boolean":
        return "select"
    return "text"


def show_edit_form(category: str, key: str, info: dict, current_value):
    """Show form for editing a setting."""
    setting_type = info.get("type", "string")
    default = info.get("default")
    description = info.get("description", "")

    if setting_type == "boolean":
        fields = [
            {
                "id": "value",
                "type": "switch",
                "label": key,
                "default": current_value if current_value is not None else default,
                "hint": f"{description}\nDefault: {'Yes' if default else 'No'}",
            }
        ]
    elif setting_type == "select":
        options = info.get("options", [])
        fields = [
            {
                "id": "value",
                "type": "select",
                "label": key,
                "options": [{"value": opt, "label": opt} for opt in options],
                "default": str(current_value) if current_value else str(default),
                "hint": f"{description}\nDefault: {default}",
            }
        ]
    elif setting_type == "slider":
        min_val = info.get("min", 0)
        max_val = info.get("max", 100)
        step_val = info.get("step", 1)
        fields = [
            {
                "id": "value",
                "type": "slider",
                "label": key,
                "min": min_val,
                "max": max_val,
                "step": step_val,
                "default": current_value if current_value is not None else default,
                "hint": f"{description}\nDefault: {default}",
            }
        ]
    elif setting_type == "list":
        fields = [
            {
                "id": "value",
                "type": "text",
                "label": key,
                "default": ", ".join(str(v) for v in current_value)
                if current_value
                else "",
                "hint": f"{description}\nDefault: {', '.join(str(v) for v in (default or []))}\nEnter comma-separated values",
            }
        ]
    else:
        fields = [
            {
                "id": "value",
                "type": "text",
                "label": key,
                "default": str(current_value) if current_value is not None else "",
                "hint": f"{description}\nDefault: {default}",
            }
        ]

    print(
        json.dumps(
            {
                "type": "form",
                "form": {
                    "title": f"Edit: {key}",
                    "submitLabel": "Save",
                    "cancelLabel": "Cancel",
                    "fields": fields,
                },
                "context": f"edit:{category}.{key}",
                "navigateForward": True,
            }
        )
    )


def show_appearance_form(config: dict):
    """Show a live form with all appearance settings."""
    appearance = SETTINGS_SCHEMA.get("appearance", {})
    fields = []

    for key, info in appearance.items():
        setting_type = info.get("type", "string")
        current = get_current_value(config, "appearance", key)

        if setting_type == "boolean":
            fields.append(
                {
                    "id": f"appearance.{key}",
                    "type": "switch",
                    "label": key,
                    "default": bool(current)
                    if current is not None
                    else bool(info.get("default")),
                    "hint": info.get("description", ""),
                }
            )
        elif setting_type == "slider":
            fields.append(
                {
                    "id": f"appearance.{key}",
                    "type": "slider",
                    "label": key,
                    "min": info.get("min", 0),
                    "max": info.get("max", 1),
                    "step": info.get("step", 0.05),
                    "default": current if current is not None else info.get("default"),
                    "hint": info.get("description", ""),
                }
            )

    print(
        json.dumps(
            {
                "type": "form",
                "form": {
                    "title": "Appearance",
                    "liveUpdate": True,
                    "fields": fields,
                },
                "context": "liveform:appearance",
                "navigateForward": True,
            }
        )
    )


def parse_value(value_str, setting_type: str, default):
    """Parse string value to correct type."""
    if setting_type == "boolean":
        if isinstance(value_str, bool):
            return value_str
        return str(value_str).lower() in ("true", "yes", "1")
    if setting_type == "slider":
        # Slider values come as floats directly from form
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
        if not str(value_str).strip():
            return []
        return [v.strip() for v in value_str.split(",")]
    return value_str


def get_plugin_actions(in_form: bool = False) -> list[dict]:
    """Get plugin-level actions."""
    if in_form:
        return []
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


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")
    context = input_data.get("context", "")

    config = load_config()
    selected_id = selected.get("id", "")

    if step == "initial":
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": get_categories(),
                    "inputMode": "realtime",
                    "placeholder": "Search settings or select category...",
                    "pluginActions": get_plugin_actions(),
                }
            )
        )
        return

    # Handle live form slider changes
    if step == "formSlider":
        field_id = input_data.get("fieldId", "")
        value = input_data.get("value", 0)

        # field_id is like "appearance.backgroundTransparency"
        if "." in field_id:
            category, key = field_id.rsplit(".", 1)
            schema = SETTINGS_SCHEMA.get(category, {}).get(key, {})
            if schema:
                config = set_nested_value(config, field_id, float(value))
                save_config(config)

        # Return noop - UI already shows the new value
        print(json.dumps({"type": "noop"}))
        return

    # Handle live form switch changes
    if step == "formSwitch":
        field_id = input_data.get("fieldId", "")
        value = input_data.get("value", False)

        # For edit forms, field_id is "value" and context is "edit:category.key"
        # For live forms, field_id is the full path like "search.shellHistory.enable"
        if field_id == "value" and context.startswith("edit:"):
            path = context.split(":", 1)[1]
            parts = path.rsplit(".", 1)
            if len(parts) == 2:
                category, key = parts
                schema = SETTINGS_SCHEMA.get(category, {}).get(key, {})
                if schema:
                    config = set_nested_value(config, path, bool(value))
                    save_config(config)
        elif "." in field_id:
            # Live form with full path field id
            parts = field_id.rsplit(".", 1)
            if len(parts) == 2:
                category, key = parts
                schema = SETTINGS_SCHEMA.get(category, {}).get(key, {})
                if schema:
                    config = set_nested_value(config, field_id, bool(value))
                    save_config(config)

        # Return noop - UI already shows the new value
        print(json.dumps({"type": "noop"}))
        return

    if step == "search":
        if context.startswith("category:"):
            category = context.split(":", 1)[1]
            settings = get_settings_for_category(config, category)
            filtered = filter_settings(settings, query)
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": filtered,
                        "inputMode": "realtime",
                        "placeholder": f"Filter {CATEGORY_NAMES.get(category, category)} settings...",
                        "context": context,
                        "pluginActions": get_plugin_actions(),
                    }
                )
            )
        else:
            if query:
                all_settings = get_all_settings(config)
                filtered = filter_settings(all_settings, query)
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": filtered,
                            "inputMode": "realtime",
                            "placeholder": "Search settings or select category...",
                            "pluginActions": get_plugin_actions(),
                        }
                    )
                )
            else:
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": get_categories(),
                            "inputMode": "realtime",
                            "placeholder": "Search settings or select category...",
                            "pluginActions": get_plugin_actions(),
                        }
                    )
                )
        return

    if step == "form":
        form_data = input_data.get("formData", {})

        if context.startswith("editActionHint:"):
            hint_idx = int(context.split(":", 1)[1])
            hints = get_action_bar_hints(config)

            # Ensure we have enough hints
            while len(hints) <= hint_idx:
                hints.append({"prefix": "", "icon": "", "label": "", "plugin": ""})

            # Update the hint with form data
            hints[hint_idx] = {
                "prefix": form_data.get("prefix", "").strip(),
                "plugin": form_data.get("plugin", "").strip(),
                "label": form_data.get("label", "").strip(),
                "icon": form_data.get("icon", "").strip(),
            }

            config = set_nested_value(
                config, "search.actionBarHintsJson", json.dumps(hints)
            )
            if save_config(config):
                settings = get_action_bar_hints_list(config)
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": settings,
                            "inputMode": "realtime",
                            "clearInput": True,
                            "context": "category:actionBarHints",
                            "placeholder": "Configure action bar shortcuts...",
                            "pluginActions": get_plugin_actions(),
                            "navigateBack": True,
                        }
                    )
                )
            else:
                print(json.dumps({"type": "error", "message": "Failed to save config"}))
            return

        if context.startswith("edit:"):
            path = context.split(":", 1)[1]
            parts = path.rsplit(".", 1)
            if len(parts) == 2:
                category, key = parts
            else:
                print(json.dumps({"type": "error", "message": "Invalid setting path"}))
                return

            schema = SETTINGS_SCHEMA.get(category, {}).get(key, {})
            setting_type = schema.get("type", "string")
            default = schema.get("default")

            value_str = form_data.get("value", "")
            new_value = parse_value(value_str, setting_type, default)

            config = set_nested_value(config, path, new_value)
            if save_config(config):
                settings = get_settings_for_category(config, category)
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": settings,
                            "inputMode": "realtime",
                            "clearInput": True,
                            "context": f"category:{category}",
                            "placeholder": f"Filter {CATEGORY_NAMES.get(category, category)} settings...",
                            "pluginActions": get_plugin_actions(),
                            "navigateBack": True,
                        }
                    )
                )
            else:
                print(json.dumps({"type": "error", "message": "Failed to save config"}))

        return

    if step == "action":
        # Handle switch toggle for boolean settings
        if action == "switch" and selected_id.startswith("setting:"):
            path = selected_id.split(":", 1)[1]
            parts = path.rsplit(".", 1)
            if len(parts) == 2:
                category, key = parts
                schema = SETTINGS_SCHEMA.get(category, {}).get(key, {})
                if schema and schema.get("type") == "boolean":
                    new_value = input_data.get("value", False)
                    config = set_nested_value(config, path, bool(new_value))
                    save_config(config)
                    print(json.dumps({"type": "noop"}))
                    return

        # Handle inline slider changes in category view
        if action == "slider" and selected_id.startswith("setting:"):
            path = selected_id.split(":", 1)[1]
            parts = path.rsplit(".", 1)
            if len(parts) == 2:
                category, key = parts
                schema = SETTINGS_SCHEMA.get(category, {}).get(key, {})
                if schema and schema.get("type") == "slider":
                    new_value = input_data.get("value", 0)
                    config = set_nested_value(config, path, float(new_value))
                    save_config(config)
                    print(json.dumps({"type": "noop"}))
                    return

        if selected_id == "__plugin__" and action == "clear_cache":
            cache_path = Path.home() / ".config/hamr/plugin-indexes.json"
            try:
                if cache_path.exists():
                    cache_path.unlink()
                print(
                    json.dumps(
                        {
                            "type": "execute",
                            "notify": "Cache cleared. Restart Hamr to reindex plugins.",
                            "close": True,
                        }
                    )
                )
            except Exception as e:
                print(
                    json.dumps(
                        {"type": "error", "message": f"Failed to clear cache: {e}"}
                    )
                )
            return

        if selected_id == "__plugin__" and action == "reset_all":
            if save_config({}):
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": get_categories(),
                            "inputMode": "realtime",
                            "clearInput": True,
                            "context": "",
                            "placeholder": "Search settings or select category...",
                            "pluginActions": get_plugin_actions(),
                        }
                    )
                )
            else:
                print(
                    json.dumps({"type": "error", "message": "Failed to reset config"})
                )
            return

        if selected_id == "__form_cancel__":
            if context.startswith("liveform:"):
                # Live form cancel - go back to categories
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": get_categories(),
                            "inputMode": "realtime",
                            "clearInput": True,
                            "context": "",
                            "placeholder": "Search settings or select category...",
                            "pluginActions": get_plugin_actions(),
                        }
                    )
                )
            elif context.startswith("editActionHint:"):
                # Go back to action hints list
                settings = get_action_bar_hints_list(config)
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": settings,
                            "inputMode": "realtime",
                            "clearInput": True,
                            "context": "category:actionBarHints",
                            "placeholder": "Configure action bar shortcuts...",
                            "pluginActions": get_plugin_actions(),
                        }
                    )
                )
            elif context.startswith("edit:"):
                path = context.split(":", 1)[1]
                category = path.rsplit(".", 1)[0]
                settings = get_settings_for_category(config, category)
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": settings,
                            "inputMode": "realtime",
                            "clearInput": True,
                            "context": f"category:{category}",
                            "placeholder": f"Filter {CATEGORY_NAMES.get(category, category)} settings...",
                            "pluginActions": get_plugin_actions(),
                        }
                    )
                )
            else:
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": get_categories(),
                            "inputMode": "realtime",
                            "clearInput": True,
                            "context": "",
                            "placeholder": "Search settings or select category...",
                            "pluginActions": get_plugin_actions(),
                        }
                    )
                )
            return

        if selected_id == "__back__":
            if context.startswith("liveform:"):
                # Going back from live form to categories
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": get_categories(),
                            "inputMode": "realtime",
                            "clearInput": True,
                            "context": "",
                            "placeholder": "Search settings or select category...",
                            "pluginActions": get_plugin_actions(),
                            "navigationDepth": 0,
                        }
                    )
                )
            elif context.startswith("category:"):
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": get_categories(),
                            "inputMode": "realtime",
                            "clearInput": True,
                            "context": "",
                            "placeholder": "Search settings or select category...",
                            "pluginActions": get_plugin_actions(),
                            "navigationDepth": 0,
                        }
                    )
                )
            else:
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": get_categories(),
                            "inputMode": "realtime",
                            "clearInput": True,
                            "context": "",
                            "placeholder": "Search settings or select category...",
                            "pluginActions": get_plugin_actions(),
                        }
                    )
                )
            return

        if selected_id.startswith("category:"):
            category = selected_id.split(":", 1)[1]

            # Appearance category shows a live form with all sliders
            if category == "appearance":
                show_appearance_form(config)
                return

            settings = get_settings_for_category(config, category)
            placeholder = (
                "Configure action bar shortcuts..."
                if category == "actionBarHints"
                else f"Filter {CATEGORY_NAMES.get(category, category)} settings..."
            )
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": settings,
                        "inputMode": "realtime",
                        "clearInput": True,
                        "context": f"category:{category}",
                        "placeholder": placeholder,
                        "pluginActions": get_plugin_actions(),
                        "navigateForward": True,
                    }
                )
            )
            return

        if selected_id.startswith("actionHint:"):
            hint_idx = int(selected_id.split(":", 1)[1])
            hints = get_action_bar_hints(config)
            default_hint = (
                DEFAULT_ACTION_BAR_HINTS[hint_idx]
                if hint_idx < len(DEFAULT_ACTION_BAR_HINTS)
                else {}
            )
            current_hint = hints[hint_idx] if hint_idx < len(hints) else {}

            if action == "reset":
                if hint_idx < len(hints):
                    hints[hint_idx] = default_hint.copy()
                    config = set_nested_value(
                        config, "search.actionBarHintsJson", json.dumps(hints)
                    )
                    save_config(config)
                settings = get_action_bar_hints_list(config)
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": settings,
                            "inputMode": "realtime",
                            "context": "category:actionBarHints",
                            "placeholder": "Configure action bar shortcuts...",
                            "pluginActions": get_plugin_actions(),
                            "navigateForward": False,
                        }
                    )
                )
                return

            # Show form to edit this action hint
            print(
                json.dumps(
                    {
                        "type": "form",
                        "form": {
                            "title": f"Edit Action {hint_idx + 1}",
                            "submitLabel": "Save",
                            "cancelLabel": "Cancel",
                            "fields": [
                                {
                                    "id": "prefix",
                                    "type": "text",
                                    "label": "Prefix",
                                    "default": current_hint.get("prefix", ""),
                                    "hint": f"Keyboard shortcut (e.g., ~, ;, /)\nDefault: {default_hint.get('prefix', '')}",
                                },
                                {
                                    "id": "plugin",
                                    "type": "text",
                                    "label": "Plugin",
                                    "default": current_hint.get("plugin", ""),
                                    "hint": f"Plugin to open (e.g., files, clipboard, emoji)\nDefault: {default_hint.get('plugin', '')}",
                                },
                                {
                                    "id": "label",
                                    "type": "text",
                                    "label": "Label",
                                    "default": current_hint.get("label", ""),
                                    "hint": f"Display label\nDefault: {default_hint.get('label', '')}",
                                },
                                {
                                    "id": "icon",
                                    "type": "text",
                                    "label": "Icon",
                                    "default": current_hint.get("icon", ""),
                                    "hint": f"Material icon name\nDefault: {default_hint.get('icon', '')}",
                                },
                            ],
                        },
                        "context": f"editActionHint:{hint_idx}",
                        "navigateForward": True,
                    }
                )
            )
            return

        if selected_id.startswith("setting:"):
            path = selected_id.split(":", 1)[1]
            parts = path.rsplit(".", 1)
            if len(parts) == 2:
                category, key = parts
            else:
                print(json.dumps({"type": "error", "message": "Invalid setting path"}))
                return

            schema = SETTINGS_SCHEMA.get(category, {}).get(key, {})
            setting_type = schema.get("type", "string")

            if action == "reset":
                config = delete_nested_value(config, path)
                save_config(config)

                default = schema.get("default")
                current_category = (
                    context.split(":", 1)[1]
                    if context.startswith("category:")
                    else category
                )

                # For inline controls (slider/switch), use update response
                if setting_type in ("slider", "boolean"):
                    update_item: dict = {"id": selected_id}
                    if setting_type == "slider":
                        slider_val = (
                            default if isinstance(default, (int, float)) else 0.0
                        )
                        update_item["value"] = float(slider_val)
                    else:
                        update_item["value"] = bool(default)
                    # Remove badge since it's now at default
                    update_item["badges"] = []
                    print(
                        json.dumps(
                            {
                                "type": "update",
                                "items": [update_item],
                            }
                        )
                    )
                else:
                    settings = get_settings_for_category(config, current_category)
                    print(
                        json.dumps(
                            {
                                "type": "results",
                                "results": settings,
                                "inputMode": "realtime",
                                "context": f"category:{current_category}",
                                "placeholder": f"Filter {CATEGORY_NAMES.get(current_category, current_category)} settings...",
                                "pluginActions": get_plugin_actions(),
                                "navigateForward": False,
                            }
                        )
                    )
                return

            if not schema:
                print(
                    json.dumps({"type": "error", "message": f"Unknown setting: {path}"})
                )
                return

            # Readonly settings cannot be edited
            if setting_type == "readonly":
                current_category = (
                    context.split(":", 1)[1]
                    if context.startswith("category:")
                    else category
                )
                settings = get_settings_for_category(config, current_category)
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": settings,
                            "inputMode": "realtime",
                            "context": f"category:{current_category}",
                            "placeholder": f"Filter {CATEGORY_NAMES.get(current_category, current_category)} settings...",
                            "pluginActions": get_plugin_actions(),
                        }
                    )
                )
                return

            # Slider and boolean are inline - clicking on them should not open a form
            if setting_type in ("slider", "boolean"):
                print(json.dumps({"type": "noop"}))
                return

            current = get_current_value(config, category, key)
            show_edit_form(category, key, schema, current)
            return


if __name__ == "__main__":
    main()
