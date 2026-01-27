#!/usr/bin/env python3
"""
Snippet plugin - text snippets for quick insertion.
Reads snippets from ~/.config/hamr/snippets.json

Features:
- Insert snippet value using ydotool (or copy to clipboard as fallback)
- Variable expansion: {date}, {time}, {clipboard}, etc.
- CRUD operations via forms
"""

import json
import os
import shutil
import subprocess
import sys
from datetime import datetime
from pathlib import Path

SNIPPETS_PATH = Path.home() / ".config/hamr/snippets.json"


def get_clipboard_content() -> str:
    """Get current clipboard content"""
    try:
        result = subprocess.run(
            ["wl-paste", "-n"],
            capture_output=True,
            text=True,
            timeout=2,
        )
        return result.stdout if result.returncode == 0 else ""
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return ""


def expand_variables(value: str) -> str:
    """Expand variables in snippet value."""
    now = datetime.now()
    replacements = {
        "{date}": now.strftime("%Y-%m-%d"),
        "{time}": now.strftime("%H:%M:%S"),
        "{datetime}": now.strftime("%Y-%m-%d %H:%M:%S"),
        "{year}": now.strftime("%Y"),
        "{month}": now.strftime("%m"),
        "{day}": now.strftime("%d"),
        "{clipboard}": get_clipboard_content(),
        "{user}": os.environ.get("USER", ""),
        "{home}": str(Path.home()),
    }
    result = value
    for var, replacement in replacements.items():
        result = result.replace(var, replacement)
    return result


def has_variables(value: str) -> bool:
    """Check if value contains any expandable variables"""
    variables = [
        "{date}",
        "{time}",
        "{datetime}",
        "{year}",
        "{month}",
        "{day}",
        "{clipboard}",
        "{user}",
        "{home}",
    ]
    return any(var in value for var in variables)


def load_snippets() -> list[dict]:
    """Load snippets from config file"""
    if not SNIPPETS_PATH.exists():
        return []
    try:
        with open(SNIPPETS_PATH) as f:
            data = json.load(f)
            return data.get("snippets", [])
    except Exception:
        return []


def save_snippets(snippets: list[dict]) -> bool:
    """Save snippets to config file"""
    try:
        SNIPPETS_PATH.parent.mkdir(parents=True, exist_ok=True)
        with open(SNIPPETS_PATH, "w") as f:
            json.dump({"snippets": snippets}, f, indent=2)
        return True
    except Exception:
        return False


def truncate_value(value: str, max_len: int = 60) -> str:
    """Truncate value for display"""
    preview = value.replace("\n", " ").replace("\r", "")
    if len(preview) > max_len:
        return preview[:max_len] + "..."
    return preview


def check_ydotool() -> bool:
    """Check if ydotool is available"""
    return shutil.which("ydotool") is not None


def snippet_to_result(snippet: dict) -> dict:
    """Convert snippet to result item"""
    return {
        "id": snippet["key"],
        "name": snippet["key"],
        "description": truncate_value(snippet.get("value", "")),
        "icon": "content_paste",
        "verb": "Insert",
        "actions": [
            {"id": "copy", "name": "Copy", "icon": "content_copy"},
            {"id": "edit", "name": "Edit", "icon": "edit"},
            {"id": "delete", "name": "Delete", "icon": "delete"},
        ],
    }


def snippet_to_index_item(snippet: dict) -> dict:
    """Convert snippet to index item for main search"""
    value = snippet.get("value", "")
    description = truncate_value(value, 50)
    if has_variables(value):
        description = "(has variables) " + description
    return {
        "id": snippet["key"],
        "name": snippet["key"],
        "description": description,
        "keywords": [truncate_value(value, 30)],
        "icon": "content_paste",
        "verb": "Insert",
    }


def get_results(snippets: list[dict], query: str = "") -> list[dict]:
    """Get results, optionally filtered by query"""
    if not query:
        return [snippet_to_result(s) for s in snippets]

    query_lower = query.lower()
    results = []
    for snippet in snippets:
        if (
            query_lower in snippet["key"].lower()
            or query_lower in snippet.get("value", "")[:50].lower()
        ):
            results.append(snippet_to_result(snippet))
    return results


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")
    context = input_data.get("context", "")
    form_data = input_data.get("formData", {})

    snippets = load_snippets()

    if step == "index":
        items = [snippet_to_index_item(s) for s in snippets]
        print(json.dumps({"type": "index", "items": items}))
        return

    if step == "initial":
        results = get_results(snippets)
        if not results:
            results = [
                {
                    "id": "__empty__",
                    "name": "No snippets yet",
                    "icon": "info",
                    "description": "Use 'Add Snippet' to create one",
                }
            ]
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                    "placeholder": "Search snippets...",
                    "pluginActions": [
                        {
                            "id": "add",
                            "name": "Add Snippet",
                            "icon": "add_circle",
                            "shortcut": "Ctrl+1",
                        }
                    ],
                }
            )
        )
        return

    if step == "search":
        results = get_results(snippets, query)
        if not results:
            results = [
                {
                    "id": "__no_match__",
                    "name": f"No snippets matching '{query}'",
                    "icon": "search_off",
                }
            ]
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                    "placeholder": "Search snippets...",
                    "pluginActions": [
                        {
                            "id": "add",
                            "name": "Add Snippet",
                            "icon": "add_circle",
                            "shortcut": "Ctrl+1",
                        }
                    ],
                }
            )
        )
        return

    if step == "action":
        item_id = selected.get("id", "")

        # Plugin-level action: add
        if item_id == "__plugin__" and action == "add":
            print(
                json.dumps(
                    {
                        "type": "form",
                        "form": {
                            "title": "Add New Snippet",
                            "fields": [
                                {
                                    "id": "key",
                                    "type": "text",
                                    "label": "Key",
                                    "placeholder": "Snippet key/name",
                                    "required": True,
                                },
                                {
                                    "id": "value",
                                    "type": "textarea",
                                    "label": "Value",
                                    "placeholder": "Snippet content...\n\nSupports variables: {date}, {time}, {clipboard}",
                                    "rows": 6,
                                    "required": True,
                                },
                            ],
                            "submitLabel": "Save",
                        },
                        "context": "__add__",
                    }
                )
            )
            return

        # Non-actionable items
        if item_id in ("__empty__", "__no_match__"):
            print(json.dumps({"type": "results", "results": []}))
            return

        snippet = next((s for s in snippets if s["key"] == item_id), None)

        # Copy action
        if action == "copy":
            if snippet:
                expanded_value = expand_variables(snippet["value"])
                subprocess.run(
                    ["wl-copy", expanded_value],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                )
                print(json.dumps({"type": "execute", "close": True}))
            return

        # Edit action
        if action == "edit":
            if snippet:
                print(
                    json.dumps(
                        {
                            "type": "form",
                            "form": {
                                "title": f"Edit Snippet: {item_id}",
                                "fields": [
                                    {
                                        "id": "value",
                                        "type": "textarea",
                                        "label": "Value",
                                        "placeholder": "Snippet content...",
                                        "rows": 6,
                                        "required": True,
                                        "default": snippet.get("value", ""),
                                    },
                                ],
                                "submitLabel": "Save",
                            },
                            "context": f"__edit__:{item_id}",
                        }
                    )
                )
            return

        # Delete action
        if action == "delete":
            snippets = [s for s in snippets if s["key"] != item_id]
            if save_snippets(snippets):
                results = get_results(snippets)
                if not results:
                    results = [
                        {"id": "__empty__", "name": "No snippets yet", "icon": "info"}
                    ]
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": results,
                            "inputMode": "realtime",
                            "clearInput": True,
                            "placeholder": "Search snippets...",
                            "pluginActions": [
                                {
                                    "id": "add",
                                    "name": "Add Snippet",
                                    "icon": "add_circle",
                                    "shortcut": "Ctrl+1",
                                }
                            ],
                        }
                    )
                )
            return

        # Default action: insert snippet
        if snippet:
            expanded_value = expand_variables(snippet["value"])

            if check_ydotool():
                # Let daemon close launcher then type
                print(
                    json.dumps(
                        {
                            "type": "execute",
                            "typeText": expanded_value,
                            "close": True,
                        }
                    )
                )
            else:
                # Fallback to clipboard
                subprocess.run(
                    ["wl-copy", expanded_value],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                )
                print(
                    json.dumps(
                        {
                            "type": "execute",
                            "notify": f"ydotool not found. Copied '{item_id}' to clipboard.",
                            "close": True,
                        }
                    )
                )
        return

    if step == "form":
        # Adding new snippet
        if context == "__add__":
            key = form_data.get("key", "").strip()
            value = form_data.get("value", "")

            if not key:
                print(json.dumps({"type": "error", "message": "Key is required"}))
                return

            if any(s["key"] == key for s in snippets):
                print(
                    json.dumps(
                        {"type": "error", "message": f"Key '{key}' already exists"}
                    )
                )
                return

            if not value:
                print(json.dumps({"type": "error", "message": "Value is required"}))
                return

            snippets.append({"key": key, "value": value})
            if save_snippets(snippets):
                results = get_results(snippets)
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": results,
                            "inputMode": "realtime",
                            "clearInput": True,
                            "placeholder": "Search snippets...",
                            "pluginActions": [
                                {
                                    "id": "add",
                                    "name": "Add Snippet",
                                    "icon": "add_circle",
                                    "shortcut": "Ctrl+1",
                                }
                            ],
                        }
                    )
                )
            else:
                print(
                    json.dumps({"type": "error", "message": "Failed to save snippet"})
                )
            return

        # Editing existing snippet
        if context and context.startswith("__edit__:"):
            key = context.split(":", 1)[1]
            value = form_data.get("value", "")

            if not value:
                print(json.dumps({"type": "error", "message": "Value is required"}))
                return

            for s in snippets:
                if s["key"] == key:
                    s["value"] = value
                    break

            if save_snippets(snippets):
                results = get_results(snippets)
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": results,
                            "inputMode": "realtime",
                            "clearInput": True,
                            "placeholder": "Search snippets...",
                            "pluginActions": [
                                {
                                    "id": "add",
                                    "name": "Add Snippet",
                                    "icon": "add_circle",
                                    "shortcut": "Ctrl+1",
                                }
                            ],
                        }
                    )
                )
            else:
                print(
                    json.dumps({"type": "error", "message": "Failed to save snippet"})
                )
            return

    print(json.dumps({"type": "results", "results": []}))


if __name__ == "__main__":
    main()
