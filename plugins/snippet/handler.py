#!/usr/bin/env python3
"""
Snippet workflow handler - manage and insert text snippets
Reads snippets from ~/.config/hamr/snippets.json

Features:
- Browse and search snippets by key
- Insert snippet value using ydotool
- Add new snippets (key + value)
- Edit existing snippets
- Delete snippets

Note: Uses a delay before typing to allow focus to return to previous window
"""

import json
import os
import sys
import shutil
from pathlib import Path

# Test mode - mock external tool availability
TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

SNIPPETS_PATH = Path.home() / ".config/hamr/snippets.json"
# Delay in ms before typing to allow focus to return
TYPE_DELAY_MS = 150


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


def fuzzy_match(query: str, text: str) -> bool:
    """Simple fuzzy match - all query chars appear in order"""
    query = query.lower()
    text = text.lower()
    qi = 0
    for c in text:
        if qi < len(query) and c == query[qi]:
            qi += 1
    return qi == len(query)


def filter_snippets(query: str, snippets: list[dict]) -> list[dict]:
    """Filter snippets by key or value preview"""
    if not query:
        return snippets

    results = []
    for snippet in snippets:
        if fuzzy_match(query, snippet["key"]):
            results.append(snippet)
            continue
        # Also search in value preview
        if fuzzy_match(query, snippet.get("value", "")[:50]):
            results.append(snippet)
    return results


def get_plugin_actions(in_add_mode: bool = False) -> list[dict]:
    """Get plugin-level actions for the action bar"""
    if in_add_mode:
        return []  # No actions while in form
    return [
        {
            "id": "add",
            "name": "Add Snippet",
            "icon": "add_circle",
            "shortcut": "Ctrl+1",
        }
    ]


def show_add_form(key_default: str = "", key_error: str = ""):
    """Show form for adding a new snippet"""
    fields = [
        {
            "id": "key",
            "type": "text",
            "label": "Key",
            "placeholder": "Snippet key/name",
            "required": True,
            "default": key_default,
        },
        {
            "id": "value",
            "type": "textarea",
            "label": "Value",
            "placeholder": "Snippet content...\n\nSupports multiple lines.",
            "rows": 6,
            "required": True,
        },
    ]
    if key_error:
        fields[0]["hint"] = key_error

    print(
        json.dumps(
            {
                "type": "form",
                "form": {
                    "title": "Add New Snippet",
                    "submitLabel": "Save",
                    "cancelLabel": "Cancel",
                    "fields": fields,
                },
                "context": "__add__",
            }
        )
    )


def show_edit_form(key: str, current_value: str):
    """Show form for editing an existing snippet"""
    print(
        json.dumps(
            {
                "type": "form",
                "form": {
                    "title": f"Edit Snippet: {key}",
                    "submitLabel": "Save",
                    "cancelLabel": "Cancel",
                    "fields": [
                        {
                            "id": "value",
                            "type": "textarea",
                            "label": "Value",
                            "placeholder": "Snippet content...",
                            "rows": 6,
                            "required": True,
                            "default": current_value,
                        },
                    ],
                },
                "context": f"__edit__:{key}",
            }
        )
    )


def truncate_value(value: str, max_len: int = 60) -> str:
    """Truncate value for display"""
    # Replace newlines with spaces for preview
    preview = value.replace("\n", " ").replace("\r", "")
    if len(preview) > max_len:
        return preview[:max_len] + "..."
    return preview


def get_snippet_list(snippets: list[dict], show_actions: bool = True) -> list[dict]:
    """Convert snippets to result format for browsing"""
    results = []
    for snippet in snippets:
        result = {
            "id": snippet["key"],
            "name": snippet["key"],
            "description": truncate_value(snippet.get("value", "")),
            "icon": "content_paste",
            "verb": "Insert",
        }
        if show_actions:
            result["actions"] = [
                {"id": "copy", "name": "Copy", "icon": "content_copy"},
                {"id": "edit", "name": "Edit", "icon": "edit"},
                {"id": "delete", "name": "Delete", "icon": "delete"},
            ]
        results.append(result)
    return results


def get_main_menu(snippets: list[dict], query: str = "") -> list[dict]:
    """Get main menu with snippets (add option now in action bar)"""
    results = []

    # Filter snippets
    filtered = filter_snippets(query, snippets)
    results.extend(get_snippet_list(filtered))

    # Empty state hint
    if not results:
        results.append(
            {
                "id": "__empty__",
                "name": "No snippets yet",
                "icon": "info",
                "description": "Use 'Add Snippet' button or Ctrl+1 to create one",
            }
        )

    return results


def check_ydotool() -> bool:
    """Check if ydotool is available"""
    if TEST_MODE:
        return True  # Assume available in test mode
    return shutil.which("ydotool") is not None


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")
    context = input_data.get("context", "")

    snippets = load_snippets()
    selected_id = selected.get("id", "")

    # ===== INITIAL: Show all snippets =====
    if step == "initial":
        results = get_main_menu(snippets)
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                    "placeholder": "Search snippets...",
                    "pluginActions": get_plugin_actions(),
                }
            )
        )
        return

    # ===== FORM: Handle form submission =====
    if step == "form":
        form_data = input_data.get("formData", {})

        # Adding new snippet
        if context == "__add__":
            key = form_data.get("key", "").strip()
            value = form_data.get("value", "")

            if not key:
                print(json.dumps({"type": "error", "message": "Key is required"}))
                return

            # Check if key already exists
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

            new_snippet = {"key": key, "value": value}
            snippets.append(new_snippet)

            if save_snippets(snippets):
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": get_main_menu(snippets),
                            "inputMode": "realtime",
                            "clearInput": True,
                            "context": "",
                            "placeholder": "Search snippets...",
                            "pluginActions": get_plugin_actions(),
                        }
                    )
                )
            else:
                print(
                    json.dumps({"type": "error", "message": "Failed to save snippet"})
                )
            return

        # Editing existing snippet
        if context.startswith("__edit__:"):
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
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": get_main_menu(snippets),
                            "inputMode": "realtime",
                            "clearInput": True,
                            "context": "",
                            "placeholder": "Search snippets...",
                            "pluginActions": get_plugin_actions(),
                        }
                    )
                )
            else:
                print(
                    json.dumps({"type": "error", "message": "Failed to save snippet"})
                )
            return

    # ===== SEARCH: Filter snippets =====
    if step == "search":
        # Normal snippet filtering (realtime mode)
        results = get_main_menu(snippets, query)
        print(
            json.dumps(
                {
                    "type": "results",
                    "inputMode": "realtime",
                    "results": results,
                    "placeholder": "Search snippets...",
                    "pluginActions": get_plugin_actions(),
                }
            )
        )
        return

    # ===== ACTION: Handle selection =====
    if step == "action":
        # Plugin-level action: add (from action bar)
        if selected_id == "__plugin__" and action == "add":
            show_add_form()
            return

        # Form cancelled - return to list
        if selected_id == "__form_cancel__":
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": get_main_menu(snippets),
                        "inputMode": "realtime",
                        "clearInput": True,
                        "context": "",
                        "placeholder": "Search snippets...",
                        "pluginActions": get_plugin_actions(),
                    }
                )
            )
            return

        # Back button (legacy support)
        if selected_id == "__back__":
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": get_main_menu(snippets),
                        "inputMode": "realtime",
                        "clearInput": True,
                        "context": "",
                        "placeholder": "Search snippets...",
                        "pluginActions": get_plugin_actions(),
                    }
                )
            )
            return

        # Non-actionable items
        if selected_id in ("__error__", "__current_value__", "__tip__", "__empty__"):
            return

        # Copy action
        if action == "copy":
            snippet = next((s for s in snippets if s["key"] == selected_id), None)
            if snippet:
                print(
                    json.dumps(
                        {
                            "type": "execute",
                            "execute": {
                                "command": ["wl-copy", snippet["value"]],
                                "name": f"Copy snippet: {selected_id}",
                                "icon": "content_copy",
                                "notify": f"Copied '{selected_id}' to clipboard",
                                "close": True,
                            },
                        }
                    )
                )
            return

        # Edit action - show edit form
        if action == "edit":
            snippet = next((s for s in snippets if s["key"] == selected_id), None)
            if snippet:
                show_edit_form(selected_id, snippet.get("value", ""))
            return

        # Delete action
        if action == "delete":
            snippets = [s for s in snippets if s["key"] != selected_id]
            if save_snippets(snippets):
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "results": get_main_menu(snippets),
                            "inputMode": "realtime",
                            "clearInput": True,
                            "placeholder": "Search snippets...",
                            "pluginActions": get_plugin_actions(),
                        }
                    )
                )
            else:
                print(
                    json.dumps({"type": "error", "message": "Failed to save snippets"})
                )
            return

        # Start adding new snippet (legacy item click support)
        if selected_id == "__add__":
            show_add_form()
            return

        # Direct snippet selection - insert using ydotool
        snippet = next((s for s in snippets if s["key"] == selected_id), None)
        if not snippet:
            print(
                json.dumps(
                    {"type": "error", "message": f"Snippet not found: {selected_id}"}
                )
            )
            return

        # Check ydotool availability
        if not check_ydotool():
            # Fallback to clipboard
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["wl-copy", snippet["value"]],
                            "name": f"Copy snippet: {selected_id}",
                            "icon": "content_copy",
                            "notify": f"ydotool not found. Copied '{selected_id}' to clipboard instead.",
                            "close": True,
                        },
                    }
                )
            )
            return

        # Use ydotool to type the snippet value
        # Add delay to allow launcher to close and focus to return
        # Using bash to chain sleep + ydotool
        value = snippet["value"]
        print(
            json.dumps(
                {
                    "type": "execute",
                    "execute": {
                        "command": [
                            "bash",
                            "-c",
                            f"sleep 0.{TYPE_DELAY_MS} && ydotool type --key-delay 0 -- {repr(value)}",
                        ],
                        "name": f"Insert snippet: {selected_id}",
                        "icon": "content_paste",
                        "close": True,
                    },
                }
            )
        )


if __name__ == "__main__":
    main()
