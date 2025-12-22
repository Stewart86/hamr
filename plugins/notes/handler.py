#!/usr/bin/env python3
"""
Notes workflow handler - quick notes with CRUD operations.
Features: list, add, view, edit, delete, copy

Uses the Form API for multi-field input (title + content).
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path

# Notes file location
CONFIG_DIR = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
NOTES_FILE = CONFIG_DIR / "hamr" / "notes.json"


def load_notes() -> list[dict]:
    """Load notes from file"""
    if not NOTES_FILE.exists():
        return []
    try:
        with open(NOTES_FILE) as f:
            data = json.load(f)
            return data.get("notes", [])
    except (json.JSONDecodeError, IOError):
        return []


def save_notes(notes: list[dict]) -> bool:
    """Save notes to file"""
    try:
        NOTES_FILE.parent.mkdir(parents=True, exist_ok=True)
        with open(NOTES_FILE, "w") as f:
            json.dump({"notes": notes}, f, indent=2)
        return True
    except IOError:
        return False


def generate_id() -> str:
    """Generate a unique ID for a note"""
    return f"note_{int(time.time() * 1000)}"


def truncate(text: str, max_len: int = 60) -> str:
    """Truncate text with ellipsis"""
    if len(text) <= max_len:
        return text
    return text[: max_len - 3] + "..."


def get_plugin_actions(in_form_mode: bool = False) -> list[dict]:
    """Get plugin-level actions for the action bar"""
    if in_form_mode:
        return []  # No actions while in form
    return [
        {
            "id": "add",
            "name": "Add Note",
            "icon": "add_circle",
            "shortcut": "Ctrl+1",
        }
    ]


def get_note_results(notes: list[dict], show_add: bool = False) -> list[dict]:
    """Convert notes to result format"""
    results = []

    # Add option is now in plugin action bar, but keep for legacy support
    if show_add:
        results.append(
            {
                "id": "__add__",
                "name": "Add new note...",
                "icon": "add_circle",
                "description": "Create a new note",
            }
        )

    # Sort notes by updated time (most recent first)
    sorted_notes = sorted(notes, key=lambda n: n.get("updated", 0), reverse=True)

    for note in sorted_notes:
        note_id = note.get("id", "")
        title = note.get("title", "Untitled")
        content = note.get("content", "")

        # Show first line of content as description
        first_line = content.split("\n")[0] if content else ""
        description = truncate(first_line, 50) if first_line else "Empty note"

        results.append(
            {
                "id": note_id,
                "name": title,
                "icon": "sticky_note_2",
                "description": description,
                "verb": "View",
                "preview": {
                    "type": "markdown",
                    "content": format_note_card(note),
                    "title": title,
                    "actions": [
                        {"id": "edit", "name": "Edit", "icon": "edit"},
                        {"id": "copy", "name": "Copy", "icon": "content_copy"},
                    ],
                    "detachable": True,
                },
                "actions": [
                    {"id": "view", "name": "View", "icon": "visibility"},
                    {"id": "edit", "name": "Edit", "icon": "edit"},
                    {"id": "copy", "name": "Copy", "icon": "content_copy"},
                    {"id": "delete", "name": "Delete", "icon": "delete"},
                ],
            }
        )

    if not notes and show_add:
        results.append(
            {
                "id": "__empty__",
                "name": "No notes yet",
                "icon": "info",
                "description": "Click 'Add new note' to get started",
            }
        )

    return results


def filter_notes(query: str, notes: list[dict]) -> list[dict]:
    """Filter notes by title or content"""
    if not query:
        return notes
    query_lower = query.lower()
    return [
        n
        for n in notes
        if query_lower in n.get("title", "").lower()
        or query_lower in n.get("content", "").lower()
    ]


def format_note_card(note: dict) -> str:
    """Format note as markdown for card display"""
    title = note.get("title", "Untitled")
    content = note.get("content", "")
    return f"## {title}\n\n{content}"


def respond(response: dict):
    """Send JSON response"""
    print(json.dumps(response))


def show_add_form(title_default: str = "", content_default: str = ""):
    """Show form for adding a new note"""
    respond(
        {
            "type": "form",
            "form": {
                "title": "Add New Note",
                "submitLabel": "Save",
                "cancelLabel": "Cancel",
                "fields": [
                    {
                        "id": "title",
                        "type": "text",
                        "label": "Title",
                        "placeholder": "Enter note title...",
                        "required": True,
                        "default": title_default,
                    },
                    {
                        "id": "content",
                        "type": "textarea",
                        "label": "Content",
                        "placeholder": "Enter note content...\n\nSupports multiple lines.",
                        "rows": 6,
                        "default": content_default,
                    },
                ],
            },
            "context": "__add__",
        }
    )


def show_edit_form(note: dict):
    """Show form for editing an existing note"""
    respond(
        {
            "type": "form",
            "form": {
                "title": f"Edit Note",
                "submitLabel": "Save",
                "cancelLabel": "Cancel",
                "fields": [
                    {
                        "id": "title",
                        "type": "text",
                        "label": "Title",
                        "placeholder": "Enter note title...",
                        "required": True,
                        "default": note.get("title", ""),
                    },
                    {
                        "id": "content",
                        "type": "textarea",
                        "label": "Content",
                        "placeholder": "Enter note content...",
                        "rows": 6,
                        "default": note.get("content", ""),
                    },
                ],
            },
            "context": f"__edit__:{note.get('id', '')}",
        }
    )


def note_to_index_item(note: dict) -> dict:
    """Convert a note to an index item for main search"""
    note_id = note.get("id", "")
    title = note.get("title", "Untitled")
    content = note.get("content", "")
    first_line = content.split("\n")[0] if content else ""
    return {
        "id": f"notes:{note_id}",
        "name": title,
        "description": truncate(first_line, 50) if first_line else "",
        "keywords": [truncate(first_line, 30)] if first_line else [],
        "icon": "sticky_note_2",
        "verb": "View",
        "actions": [
            {
                "id": "copy",
                "name": "Copy",
                "icon": "content_copy",
                "command": ["wl-copy", f"{title}\n\n{content}"],
            },
        ],
        # Default action: view note in plugin (keepOpen = show UI)
        "entryPoint": {
            "step": "action",
            "selected": {"id": note_id},
            "action": "view",
        },
        "keepOpen": True,  # Don't close launcher, show plugin UI
    }


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")
    context = input_data.get("context", "")
    form_data = input_data.get("formData", {})

    notes = load_notes()

    if step == "index":
        mode = input_data.get("mode", "full")
        indexed_ids = set(input_data.get("indexedIds", []))

        # Build current ID set
        current_ids = {f"notes:{n.get('id', '')}" for n in notes}

        if mode == "incremental" and indexed_ids:
            # Find new items
            new_ids = current_ids - indexed_ids
            new_items = [
                note_to_index_item(n)
                for n in notes
                if f"notes:{n.get('id', '')}" in new_ids
            ]

            # Find removed items
            removed_ids = list(indexed_ids - current_ids)

            print(
                json.dumps(
                    {
                        "type": "index",
                        "mode": "incremental",
                        "items": new_items,
                        "remove": removed_ids,
                    }
                )
            )
        else:
            # Full reindex
            items = [note_to_index_item(n) for n in notes]
            print(json.dumps({"type": "index", "items": items}))
        return

    if step == "initial":
        respond(
            {
                "type": "results",
                "results": get_note_results(notes),
                "inputMode": "realtime",
                "placeholder": "Search notes...",
                "pluginActions": get_plugin_actions(),
            }
        )
        return

    if step == "search":
        filtered = filter_notes(query, notes)
        results = []

        if query:
            # Show add option when searching
            results.append(
                {
                    "id": f"__add_quick__:{query}",
                    "name": f"Create note: {query}",
                    "icon": "add_circle",
                    "description": "Quick create with this as title",
                }
            )

        results.extend(get_note_results(filtered))

        respond(
            {
                "type": "results",
                "results": results,
                "inputMode": "realtime",
                "placeholder": "Search notes...",
                "pluginActions": get_plugin_actions(),
            }
        )
        return

    if step == "form":
        # Adding new note
        if context == "__add__":
            title = form_data.get("title", "").strip()
            content = form_data.get("content", "")

            if title:
                new_note = {
                    "id": generate_id(),
                    "title": title,
                    "content": content,
                    "created": int(time.time() * 1000),
                    "updated": int(time.time() * 1000),
                }
                notes.append(new_note)
                if save_notes(notes):
                    respond(
                        {
                            "type": "results",
                            "results": get_note_results(notes),
                            "inputMode": "realtime",
                            "clearInput": True,
                            "context": "",
                            "placeholder": "Search notes...",
                            "pluginActions": get_plugin_actions(),
                            "navigateBack": True,
                        }
                    )
                else:
                    respond({"type": "error", "message": "Failed to save note"})
            else:
                respond({"type": "error", "message": "Title is required"})
            return

        # Editing existing note
        if context.startswith("__edit__:"):
            note_id = context.split(":", 1)[1]
            note = next((n for n in notes if n.get("id") == note_id), None)

            if not note:
                respond({"type": "error", "message": "Note not found"})
                return

            title = form_data.get("title", "").strip()
            content = form_data.get("content", "")

            if title:
                note["title"] = title
                note["content"] = content
                note["updated"] = int(time.time() * 1000)
                if save_notes(notes):
                    respond(
                        {
                            "type": "results",
                            "results": get_note_results(notes),
                            "inputMode": "realtime",
                            "clearInput": True,
                            "context": "",
                            "placeholder": "Search notes...",
                            "pluginActions": get_plugin_actions(),
                            "navigateBack": True,
                        }
                    )
                else:
                    respond({"type": "error", "message": "Failed to save note"})
            else:
                respond({"type": "error", "message": "Title is required"})
            return

    if step == "action":
        item_id = selected.get("id", "")

        # Plugin-level action: add (from action bar)
        if item_id == "__plugin__" and action == "add":
            show_add_form()
            return

        # Form cancelled - return to list
        if item_id == "__form_cancel__":
            respond(
                {
                    "type": "results",
                    "results": get_note_results(notes),
                    "inputMode": "realtime",
                    "clearInput": True,
                    "context": "",
                    "placeholder": "Search notes...",
                    "pluginActions": get_plugin_actions(),
                }
            )
            return

        # Info items - not actionable
        if item_id in ("__info__", "__current__", "__empty__"):
            return

        # Back navigation (from Escape key or back button)
        if item_id == "__back__" or action == "back":
            respond(
                {
                    "type": "results",
                    "results": get_note_results(notes),
                    "inputMode": "realtime",
                    "clearInput": True,
                    "context": "",
                    "placeholder": "Search notes...",
                    "pluginActions": get_plugin_actions(),
                }
            )
            return

        # Start adding new note - show form
        if item_id == "__add__":
            show_add_form()
            return

        # Quick add from search - show form with title prefilled
        if item_id.startswith("__add_quick__:"):
            title = item_id.split(":", 1)[1]
            show_add_form(title_default=title)
            return

        note = next((n for n in notes if n.get("id") == item_id), None)
        if not note:
            respond({"type": "error", "message": f"Note not found: {item_id}"})
            return

        # View action (default) - show as card
        if action == "view" or not action:
            respond(
                {
                    "type": "card",
                    "card": {
                        "content": format_note_card(note),
                        "markdown": True,
                        "actions": [
                            {"id": "edit", "name": "Edit", "icon": "edit"},
                            {"id": "copy", "name": "Copy", "icon": "content_copy"},
                            {"id": "delete", "name": "Delete", "icon": "delete"},
                            {"id": "back", "name": "Back", "icon": "arrow_back"},
                        ],
                    },
                    "context": item_id,  # Store note ID for card actions
                }
            )
            return

        # Edit action - show form
        if action == "edit":
            show_edit_form(note)
            return

        # Copy action
        if action == "copy":
            content = f"{note.get('title', '')}\n\n{note.get('content', '')}"
            subprocess.run(["wl-copy", content], check=False)
            respond(
                {
                    "type": "execute",
                    "execute": {
                        "notify": f"Note '{truncate(note.get('title', ''), 20)}' copied",
                        "close": True,
                    },
                }
            )
            return

        # Delete action
        if action == "delete":
            notes = [n for n in notes if n.get("id") != item_id]
            if save_notes(notes):
                respond(
                    {
                        "type": "results",
                        "results": get_note_results(notes),
                        "inputMode": "realtime",
                        "clearInput": True,
                        "context": "",
                        "placeholder": "Search notes...",
                        "pluginActions": get_plugin_actions(),
                    }
                )
            else:
                respond({"type": "error", "message": "Failed to delete note"})
            return

    # Unknown step
    respond({"type": "error", "message": f"Unknown step: {step}"})


if __name__ == "__main__":
    main()
