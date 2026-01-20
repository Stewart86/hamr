#!/usr/bin/env python3
"""
Notes workflow handler - quick notes with CRUD operations.
Features: list, add, view, edit, delete, copy

Runs as a socket daemon with proactive index emission on startup.
"""

import json
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

CONFIG_DIR = Path.home() / ".config"
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
        return []
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

    if show_add:
        results.append(
            {
                "id": "__add__",
                "name": "Add new note...",
                "icon": "add_circle",
                "description": "Create a new note",
            }
        )

    sorted_notes = sorted(notes, key=lambda n: n.get("updated", 0), reverse=True)

    for note in sorted_notes:
        note_id = note.get("id", "")
        title = note.get("title", "Untitled")
        content = note.get("content", "")

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
                "entryPoint": {
                    "step": "action",
                    "selected": {"id": note_id},
                    "action": "copy",
                },
            },
        ],
        "entryPoint": {
            "step": "action",
            "selected": {"id": note_id},
            "action": "view",
        },
        "keepOpen": True,
    }


def get_index_items(notes: list[dict]) -> list[dict]:
    """Generate full index items from notes list"""
    return [note_to_index_item(n) for n in notes]


plugin = HamrPlugin(
    id="notes",
    name="Notes",
    description="Quick notes - create, read, edit, delete",
    icon="sticky_note_2",
)

notes_cache: list[dict] = []


@plugin.add_background_task
async def emit_initial_index(p: HamrPlugin):
    """Emit index on startup"""
    global notes_cache
    notes_cache = load_notes()
    items = get_index_items(notes_cache)
    await p.send_index(items)


@plugin.on_initial
def handle_initial():
    """Handle initial request"""
    global notes_cache
    notes_cache = load_notes()
    return HamrPlugin.results(
        get_note_results(notes_cache),
        input_mode="realtime",
        placeholder="Search notes...",
        plugin_actions=get_plugin_actions(),
    )


@plugin.on_search
def handle_search(query: str, context: str | None):
    """Handle search request"""
    global notes_cache
    notes_cache = load_notes()
    filtered = filter_notes(query, notes_cache)
    results = []

    if query:
        results.append(
            {
                "id": f"__add_quick__:{query}",
                "name": f"Create note: {query}",
                "icon": "add_circle",
                "description": "Quick create with this as title",
            }
        )

    results.extend(get_note_results(filtered))

    return HamrPlugin.results(
        results,
        input_mode="realtime",
        placeholder="Search notes...",
        plugin_actions=get_plugin_actions(),
    )


@plugin.on_action
def handle_action(item_id: str, action: str | None, context: str | None):
    """Handle action request"""
    global notes_cache
    notes_cache = load_notes()

    if item_id == "__plugin__" and action == "add":
        return show_add_form()

    if item_id == "__form_cancel__":
        return HamrPlugin.results(
            get_note_results(notes_cache),
            input_mode="realtime",
            clear_input=True,
            context="",
            placeholder="Search notes...",
            plugin_actions=get_plugin_actions(),
        )

    if item_id in ("__info__", "__current__", "__empty__"):
        return {}

    if item_id == "__back__" or action == "back":
        return HamrPlugin.results(
            get_note_results(notes_cache),
            input_mode="realtime",
            clear_input=True,
            context="",
            placeholder="Search notes...",
            plugin_actions=get_plugin_actions(),
        )

    if item_id == "__add__":
        return show_add_form()

    if item_id.startswith("__add_quick__:"):
        title = item_id.split(":", 1)[1]
        return show_add_form(title_default=title)

    note = next((n for n in notes_cache if n.get("id") == item_id), None)
    if not note:
        return HamrPlugin.error(f"Note not found: {item_id}")

    if action == "view" or not action:
        return {
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
            "context": item_id,
        }

    if action == "edit":
        return show_edit_form(note)

    if action == "copy":
        content = f"{note.get('title', '')}\n\n{note.get('content', '')}"
        subprocess.run(["wl-copy", content], check=False)
        response = HamrPlugin.close()
        response["notify"] = f"Note '{truncate(note.get('title', ''), 20)}' copied"
        return response

    if action == "delete":
        notes_cache = [n for n in notes_cache if n.get("id") != item_id]
        if save_notes(notes_cache):
            return HamrPlugin.results(
                get_note_results(notes_cache),
                input_mode="realtime",
                clear_input=True,
                context="",
                placeholder="Search notes...",
                plugin_actions=get_plugin_actions(),
            )
        return HamrPlugin.error("Failed to delete note")

    return {}


@plugin.on_form_submitted
def handle_form_submitted(form_data: dict, context: str | None):
    """Handle form submission"""
    global notes_cache
    notes_cache = load_notes()

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
            notes_cache.append(new_note)
            if save_notes(notes_cache):
                response = HamrPlugin.results(
                    get_note_results(notes_cache),
                    input_mode="realtime",
                    clear_input=True,
                    context="",
                    placeholder="Search notes...",
                    plugin_actions=get_plugin_actions(),
                )
                response["navigateBack"] = True
                return response
            return HamrPlugin.error("Failed to save note")
        return HamrPlugin.error("Title is required")

    if context and context.startswith("__edit__:"):
        note_id = context.split(":", 1)[1]
        note = next((n for n in notes_cache if n.get("id") == note_id), None)

        if not note:
            return HamrPlugin.error("Note not found")

        title = form_data.get("title", "").strip()
        content = form_data.get("content", "")

        if title:
            note["title"] = title
            note["content"] = content
            note["updated"] = int(time.time() * 1000)
            if save_notes(notes_cache):
                response = HamrPlugin.results(
                    get_note_results(notes_cache),
                    input_mode="realtime",
                    clear_input=True,
                    context="",
                    placeholder="Search notes...",
                    plugin_actions=get_plugin_actions(),
                )
                response["navigateBack"] = True
                return response
            return HamrPlugin.error("Failed to save note")
        return HamrPlugin.error("Title is required")

    return HamrPlugin.error(f"Unknown context: {context}")


def show_add_form(title_default: str = "", content_default: str = ""):
    """Show form for adding a new note"""
    return HamrPlugin.form(
        {
            "title": "Add New Note",
            "submit_label": "Save",
            "cancel_label": "Cancel",
            "fields": [
                {
                    "id": "title",
                    "type": "text",
                    "label": "Title",
                    "placeholder": "Enter note title...",
                    "required": True,
                    "default_value": title_default,
                },
                {
                    "id": "content",
                    "type": "textarea",
                    "label": "Content",
                    "placeholder": "Enter note content...\n\nSupports multiple lines.",
                    "rows": 6,
                    "default_value": content_default,
                },
            ],
        },
        context="__add__",
    )


def show_edit_form(note: dict):
    """Show form for editing an existing note"""
    return HamrPlugin.form(
        {
            "title": "Edit Note",
            "submit_label": "Save",
            "cancel_label": "Cancel",
            "fields": [
                {
                    "id": "title",
                    "type": "text",
                    "label": "Title",
                    "placeholder": "Enter note title...",
                    "required": True,
                    "default_value": note.get("title", ""),
                },
                {
                    "id": "content",
                    "type": "textarea",
                    "label": "Content",
                    "placeholder": "Enter note content...",
                    "rows": 6,
                    "default_value": note.get("content", ""),
                },
            ],
        },
        context=f"__edit__:{note.get('id', '')}",
    )


if __name__ == "__main__":
    plugin.run()
