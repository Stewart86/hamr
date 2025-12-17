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

TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"
CONFIG_PATH = Path.home() / ".config/hamr/config.json"

SETTINGS_SCHEMA: dict[str, dict[str, dict]] = {
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
    "search.prefix": {
        "action": {
            "default": "/",
            "type": "string",
            "description": "Prefix for actions/plugins",
        },
        "app": {
            "default": ">",
            "type": "string",
            "description": "Prefix for app search",
        },
        "clipboard": {
            "default": ";",
            "type": "string",
            "description": "Prefix for clipboard history",
        },
        "emojis": {
            "default": ":",
            "type": "string",
            "description": "Prefix for emoji picker",
        },
        "file": {
            "default": "~",
            "type": "string",
            "description": "Prefix for file search",
        },
        "math": {
            "default": "=",
            "type": "string",
            "description": "Prefix for calculator",
        },
        "shellCommand": {
            "default": "$",
            "type": "string",
            "description": "Prefix for shell commands",
        },
        "shellHistory": {
            "default": "!",
            "type": "string",
            "description": "Prefix for shell history",
        },
        "webSearch": {
            "default": "?",
            "type": "string",
            "description": "Prefix for web search",
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
    },
    "appearance": {
        "backgroundTransparency": {
            "default": 0.2,
            "type": "number",
            "description": "Background transparency (0=opaque, 1=transparent)",
        },
        "contentTransparency": {
            "default": 0.2,
            "type": "number",
            "description": "Content transparency (0=opaque, 1=transparent)",
        },
        "launcherXRatio": {
            "default": 0.5,
            "type": "number",
            "description": "Launcher X position (0.0-1.0, 0.5=center)",
        },
        "launcherYRatio": {
            "default": 0.1,
            "type": "number",
            "description": "Launcher Y position (0.0-1.0, 0.1=10% from top)",
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
            "description": "Image browser width (px)",
        },
        "imageBrowserHeight": {
            "default": 690,
            "type": "number",
            "description": "Image browser height (px)",
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
    "search.prefix": "tag",
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
    "search.prefix": "Search Prefixes",
    "search.shellHistory": "Shell History",
    "imageBrowser": "Image Browser",
    "behavior": "Behavior",
    "appearance": "Appearance",
    "sizes": "Sizes",
    "fonts": "Fonts",
    "paths": "Paths",
}


def load_config() -> dict:
    if TEST_MODE:
        return {}
    if not CONFIG_PATH.exists():
        return {}
    try:
        with open(CONFIG_PATH) as f:
            return json.load(f)
    except Exception:
        return {}


def save_config(config: dict) -> bool:
    if TEST_MODE:
        return True
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


def get_categories() -> list[dict]:
    """Get list of categories."""
    results = []
    for category in SETTINGS_SCHEMA:
        settings_count = len(SETTINGS_SCHEMA[category])
        results.append(
            {
                "id": f"category:{category}",
                "name": CATEGORY_NAMES.get(category, category),
                "description": f"{settings_count} settings",
                "icon": CATEGORY_ICONS.get(category, "settings"),
                "verb": "Browse",
            }
        )
    return results


def get_settings_for_category(config: dict, category: str) -> list[dict]:
    """Get settings list for a specific category."""
    results = []
    schema = SETTINGS_SCHEMA.get(category, {})
    for key, info in schema.items():
        current = get_current_value(config, category, key)
        results.append(
            {
                "id": f"setting:{category}.{key}",
                "name": key,
                "description": format_value(current),
                "icon": get_type_icon(info.get("type", "string")),
                "verb": "Edit",
                "actions": [
                    {"id": "reset", "name": "Reset to Default", "icon": "restart_alt"},
                ],
            }
        )
    return results


def get_all_settings(config: dict) -> list[dict]:
    """Get all settings as a flat list."""
    results = []
    for category, settings in SETTINGS_SCHEMA.items():
        for key, info in settings.items():
            current = get_current_value(config, category, key)
            results.append(
                {
                    "id": f"setting:{category}.{key}",
                    "name": key,
                    "description": f"{CATEGORY_NAMES.get(category, category)} | {format_value(current)}",
                    "icon": get_type_icon(info.get("type", "string")),
                    "verb": "Edit",
                    "category": category,
                    "actions": [
                        {
                            "id": "reset",
                            "name": "Reset to Default",
                            "icon": "restart_alt",
                        },
                    ],
                }
            )
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
        "boolean": "toggle_on",
        "list": "list",
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
                "type": "select",
                "label": key,
                "options": [
                    {"value": "true", "label": "Yes"},
                    {"value": "false", "label": "No"},
                ],
                "default": "true" if current_value else "false",
                "hint": f"{description}\nDefault: {'Yes' if default else 'No'}",
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


def parse_value(value_str: str, setting_type: str, default):
    """Parse string value to correct type."""
    if setting_type == "boolean":
        return value_str.lower() in ("true", "yes", "1")
    if setting_type == "number":
        try:
            if "." in value_str:
                return float(value_str)
            return int(value_str)
        except ValueError:
            return default
    if setting_type == "list":
        if not value_str.strip():
            return []
        return [v.strip() for v in value_str.split(",")]
    return value_str


def get_plugin_actions(in_form: bool = False) -> list[dict]:
    """Get plugin-level actions."""
    if in_form:
        return []
    return [
        {
            "id": "reset_all",
            "name": "Reset All",
            "icon": "restart_alt",
            "confirm": "Reset all settings to defaults? This cannot be undone.",
        }
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
            if context.startswith("edit:"):
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
            if context.startswith("category:"):
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

            if action == "reset":
                config = delete_nested_value(config, path)
                if save_config(config):
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
                else:
                    print(
                        json.dumps(
                            {"type": "error", "message": "Failed to save config"}
                        )
                    )
                return

            schema = SETTINGS_SCHEMA.get(category, {}).get(key, {})
            if not schema:
                print(
                    json.dumps({"type": "error", "message": f"Unknown setting: {path}"})
                )
                return

            current = get_current_value(config, category, key)
            show_edit_form(category, key, schema, current)
            return


if __name__ == "__main__":
    main()
