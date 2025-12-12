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
import sys
import shutil
from pathlib import Path

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
    """Get main menu with snippets and add option"""
    results = []

    # Filter snippets
    filtered = filter_snippets(query, snippets)
    results.extend(get_snippet_list(filtered))

    # Add "Add new snippet" option at the end
    results.append(
        {
            "id": "__add__",
            "name": "Add new snippet",
            "description": "Create a new text snippet",
            "icon": "add_circle",
        }
    )

    return results


def check_ydotool() -> bool:
    """Check if ydotool is available"""
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

    # ===== INITIAL: Show all snippets + add option =====
    if step == "initial":
        results = get_main_menu(snippets)
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                    "placeholder": "Search snippets...",
                }
            )
        )
        return

    # ===== SEARCH: Context-dependent search =====
    if step == "search":
        # Adding new snippet - step 2: entering value (submit mode)
        # Check this BEFORE step 1 because selected_id might still be "__add__"
        # but context has been updated to "__add_key__:..."
        if context.startswith("__add_key__:"):
            key = context.split(":", 1)[1]
            if query:
                # Save directly on Enter
                # Process escape sequences
                value = query.replace("\\n", "\n").replace("\\t", "\t")

                new_snippet = {"key": key, "value": value}
                snippets.append(new_snippet)

                if save_snippets(snippets):
                    snippets = load_snippets()
                    print(
                        json.dumps(
                            {
                                "type": "results",
                                "results": get_main_menu(snippets),
                                "inputMode": "realtime",
                                "clearInput": True,
                                "context": "",
                                "placeholder": "Search snippets...",
                            }
                        )
                    )
                else:
                    print(
                        json.dumps(
                            {"type": "error", "message": "Failed to save snippets"}
                        )
                    )
            else:
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "inputMode": "submit",
                            "context": f"__add_key__:{key}",
                            "placeholder": f"Enter value for '{key}' (Enter to save)",
                            "results": [
                                {
                                    "id": "__back__",
                                    "name": "Back",
                                    "icon": "arrow_back",
                                },
                                {
                                    "id": "__tip__",
                                    "name": "Tip: Use \\n for newlines",
                                    "icon": "info",
                                    "description": "e.g., Line 1\\nLine 2",
                                },
                            ],
                        }
                    )
                )
            return

        # Adding new snippet - step 1: entering key (submit mode)
        if context == "__add__" or selected_id == "__add__":
            if query:
                # Check if key already exists
                exists = any(s["key"] == query for s in snippets)
                if exists:
                    print(
                        json.dumps(
                            {
                                "type": "results",
                                "inputMode": "submit",
                                "context": "__add__",
                                "placeholder": "Enter snippet key (Enter to confirm)",
                                "results": [
                                    {
                                        "id": "__back__",
                                        "name": "Back",
                                        "icon": "arrow_back",
                                    },
                                    {
                                        "id": "__error__",
                                        "name": f"'{query}' already exists",
                                        "icon": "error",
                                        "description": "Choose a different key",
                                    },
                                ],
                            }
                        )
                    )
                else:
                    # Move to value entry on Enter
                    print(
                        json.dumps(
                            {
                                "type": "results",
                                "inputMode": "submit",
                                "clearInput": True,
                                "context": f"__add_key__:{query}",
                                "placeholder": f"Enter value for '{query}' (Enter to save)",
                                "results": [
                                    {
                                        "id": "__back__",
                                        "name": "Back",
                                        "icon": "arrow_back",
                                    },
                                    {
                                        "id": "__tip__",
                                        "name": "Tip: Use \\n for newlines",
                                        "icon": "info",
                                        "description": "e.g., Line 1\\nLine 2",
                                    },
                                ],
                            }
                        )
                    )
            else:
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "inputMode": "submit",
                            "context": "__add__",
                            "placeholder": "Enter snippet key (Enter to confirm)",
                            "results": [
                                {"id": "__back__", "name": "Back", "icon": "arrow_back"}
                            ],
                        }
                    )
                )
            return

        # Editing snippet value (submit mode)
        edit_context = context if context.startswith("__edit__:") else None
        if edit_context or selected_id.startswith("__edit__:"):
            key = (edit_context or selected_id).split(":", 1)[1]
            snippet = next((s for s in snippets if s["key"] == key), None)
            current_value = snippet.get("value", "") if snippet else ""

            if query:
                # Save directly on Enter
                # Process escape sequences
                value = query.replace("\\n", "\n").replace("\\t", "\t")

                for s in snippets:
                    if s["key"] == key:
                        s["value"] = value
                        break

                if save_snippets(snippets):
                    snippets = load_snippets()
                    print(
                        json.dumps(
                            {
                                "type": "results",
                                "results": get_main_menu(snippets),
                                "inputMode": "realtime",
                                "clearInput": True,
                                "context": "",
                                "placeholder": "Search snippets...",
                            }
                        )
                    )
                else:
                    print(
                        json.dumps(
                            {"type": "error", "message": "Failed to save snippets"}
                        )
                    )
            else:
                # Show current value as hint
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "inputMode": "submit",
                            "context": f"__edit__:{key}",
                            "placeholder": f"Edit value for '{key}' (Enter to save)",
                            "results": [
                                {
                                    "id": "__current_value__",
                                    "name": f"Current: {truncate_value(current_value)}",
                                    "description": "Type new value above",
                                    "icon": "info",
                                },
                                {
                                    "id": "__back__",
                                    "name": "Cancel",
                                    "icon": "arrow_back",
                                },
                            ],
                        }
                    )
                )
            return

        # Normal snippet filtering (realtime mode)
        results = get_main_menu(snippets, query)
        print(
            json.dumps(
                {
                    "type": "results",
                    "inputMode": "realtime",
                    "results": results,
                    "placeholder": "Search snippets...",
                }
            )
        )
        return

    # ===== ACTION: Handle selection =====
    if step == "action":
        # Back button
        if selected_id == "__back__":
            results = get_main_menu(snippets)
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": results,
                        "inputMode": "realtime",
                        "clearInput": True,
                        "context": "",
                        "placeholder": "Search snippets...",
                    }
                )
            )
            return

        # Non-actionable items
        if selected_id in ("__error__", "__current_value__", "__tip__"):
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

        # Edit action - enter edit mode
        if action == "edit":
            snippet = next((s for s in snippets if s["key"] == selected_id), None)
            if snippet:
                current_value = snippet.get("value", "")
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "inputMode": "submit",
                            "clearInput": True,
                            "context": f"__edit__:{selected_id}",
                            "placeholder": f"Edit value for '{selected_id}' (Enter to save)",
                            "results": [
                                {
                                    "id": "__current_value__",
                                    "name": f"Current: {truncate_value(current_value)}",
                                    "description": "Type new value above",
                                    "icon": "info",
                                },
                                {
                                    "id": "__back__",
                                    "name": "Cancel",
                                    "icon": "arrow_back",
                                },
                            ],
                        }
                    )
                )
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
                        }
                    )
                )
            else:
                print(
                    json.dumps({"type": "error", "message": "Failed to save snippets"})
                )
            return

        # Start adding new snippet
        if selected_id == "__add__":
            print(
                json.dumps(
                    {
                        "type": "results",
                        "inputMode": "submit",
                        "clearInput": True,
                        "context": "__add__",
                        "placeholder": "Enter snippet key (Enter to confirm)",
                        "results": [
                            {"id": "__back__", "name": "Back", "icon": "arrow_back"}
                        ],
                    }
                )
            )
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
