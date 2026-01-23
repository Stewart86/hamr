#!/usr/bin/env python3
import json
import shutil
import subprocess
import sys
from configparser import ConfigParser
from pathlib import Path


APP_DIRS = [
    Path.home() / ".local/share/applications",
    Path.home() / ".local/share/flatpak/exports/share/applications",
    Path("/usr/share/applications"),
    Path("/usr/local/share/applications"),
    Path("/var/lib/flatpak/exports/share/applications"),
    Path("/var/lib/snapd/desktop/applications"),
]

DEFAULT_TARGETS = [
    {
        "id": "browser",
        "name": "Web Browser",
        "icon": "language",
        "mime_types": ["x-scheme-handler/http", "x-scheme-handler/https"],
        "keywords": ["browser", "web", "internet", "http", "https"],
    },
    {
        "id": "email",
        "name": "Email Client",
        "icon": "mail",
        "mime_types": ["x-scheme-handler/mailto"],
        "keywords": ["email", "mail", "mailto"],
    },
    {
        "id": "text",
        "name": "Text Editor",
        "icon": "description",
        "mime_types": ["text/plain"],
        "keywords": ["text", "editor", "plain"],
    },
    {
        "id": "file-manager",
        "name": "File Manager",
        "icon": "folder_open",
        "mime_types": ["inode/directory"],
        "keywords": ["files", "folders", "directory"],
    },
    {
        "id": "pdf",
        "name": "PDF Viewer",
        "icon": "picture_as_pdf",
        "mime_types": ["application/pdf"],
        "keywords": ["pdf", "reader"],
    },
    {
        "id": "image",
        "name": "Image Viewer",
        "icon": "image",
        "mime_types": ["image/png", "image/jpeg", "image/webp", "image/gif"],
        "keywords": ["image", "photo", "viewer"],
    },
    {
        "id": "video",
        "name": "Video Player",
        "icon": "movie",
        "mime_types": [
            "video/mp4",
            "video/x-matroska",
            "video/webm",
            "video/quicktime",
            "video/x-msvideo",
        ],
        "keywords": ["video", "movie", "player", "mp4", "mkv"],
    },
    {
        "id": "audio",
        "name": "Music Player",
        "icon": "music_note",
        "mime_types": [
            "audio/mpeg",
            "audio/flac",
            "audio/ogg",
            "audio/x-wav",
            "audio/mp4",
            "audio/x-m4a",
        ],
        "keywords": ["audio", "music", "player", "mp3", "flac"],
    },
    {
        "id": "document",
        "name": "Word Processor",
        "icon": "article",
        "mime_types": [
            "application/msword",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "application/vnd.oasis.opendocument.text",
            "application/rtf",
            "text/rtf",
        ],
        "keywords": ["doc", "word", "document", "writer"],
    },
    {
        "id": "spreadsheet",
        "name": "Spreadsheet",
        "icon": "table_chart",
        "mime_types": [
            "application/vnd.ms-excel",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "application/vnd.oasis.opendocument.spreadsheet",
        ],
        "keywords": ["spreadsheet", "excel", "sheet", "numbers"],
    },
    {
        "id": "presentation",
        "name": "Presentation",
        "icon": "present_to_all",
        "mime_types": [
            "application/vnd.ms-powerpoint",
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            "application/vnd.oasis.opendocument.presentation",
        ],
        "keywords": ["presentation", "slides", "powerpoint", "keynote"],
    },
    {
        "id": "archive",
        "name": "Archive Manager",
        "icon": "archive",
        "mime_types": [
            "application/zip",
            "application/x-zip-compressed",
            "application/x-7z-compressed",
            "application/x-tar",
            "application/x-rar",
            "application/vnd.rar",
            "application/x-xz",
            "application/gzip",
            "application/x-gzip",
        ],
        "keywords": ["archive", "zip", "tar", "extract"],
    },
    {
        "id": "calendar",
        "name": "Calendar",
        "icon": "event",
        "mime_types": ["text/calendar"],
        "keywords": ["calendar", "ics", "event"],
    },
]


def emit(data):
    print(json.dumps(data), flush=True)


def command_available(command):
    return shutil.which(command) is not None


def run_xdg_mime(args):
    try:
        result = subprocess.run(
            ["xdg-mime"] + args,
            capture_output=True,
            text=True,
            check=False,
        )
    except Exception as exc:
        return None, str(exc)

    if result.returncode != 0:
        error = result.stderr.strip() or result.stdout.strip()
        return None, error or "xdg-mime failed"

    return result.stdout.strip(), None


def get_default_for_mime(mime_type):
    output, _ = run_xdg_mime(["query", "default", mime_type])
    if not output:
        return None
    first_line = output.splitlines()[0].strip()
    return first_line or None


def set_default_for_mimes(desktop_file, mime_types):
    _, error = run_xdg_mime(["default", desktop_file] + mime_types)
    return error


def parse_desktop_file(path, include_nodisplay=False):
    try:
        config = ConfigParser(interpolation=None)
        config.read(path, encoding="utf-8")

        if not config.has_section("Desktop Entry"):
            return None

        entry = config["Desktop Entry"]

        if entry.get("Type", "") != "Application":
            return None
        if entry.get("NoDisplay", "").lower() == "true" and not include_nodisplay:
            return None
        if entry.get("Hidden", "").lower() == "true":
            return None

        name = entry.get("Name", "").strip()
        if not name:
            return None

        categories_str = entry.get("Categories", "")
        categories = [c.strip() for c in categories_str.split(";") if c.strip()]

        keywords_str = entry.get("Keywords", "")
        keywords = [
            keyword.strip()
            for keyword in keywords_str.replace(";", " ").split()
            if keyword.strip()
        ]

        mime_types_str = entry.get("MimeType", "")
        mime_types = [m.strip() for m in mime_types_str.split(";") if m.strip()]

        return {
            "id": str(path),
            "desktop_id": path.stem,
            "desktop_file": path.name,
            "name": name,
            "generic_name": entry.get("GenericName", "").strip(),
            "comment": entry.get("Comment", "").strip(),
            "icon": entry.get("Icon", "application-x-executable"),
            "keywords": keywords,
            "mime_types": mime_types,
            "categories": categories,
        }
    except Exception:
        return None


def load_all_apps(include_nodisplay=False):
    apps = {}
    for app_dir in APP_DIRS:
        if not app_dir.exists():
            continue
        for desktop_file in app_dir.glob("*.desktop"):
            app = parse_desktop_file(desktop_file, include_nodisplay=include_nodisplay)
            if app and app["id"] not in apps:
                apps[app["id"]] = app
    return list(apps.values())


def build_app_lookup(apps):
    lookup = {}
    for app in apps:
        desktop_file = app.get("desktop_file")
        if desktop_file and desktop_file not in lookup:
            lookup[desktop_file] = app
        desktop_id = app.get("desktop_id")
        if desktop_id and desktop_id not in lookup:
            lookup[desktop_id] = app
    return lookup


def lookup_app_name(lookup, desktop_file):
    if not desktop_file:
        return "Not set"
    app = lookup.get(desktop_file)
    if app:
        return app.get("name", desktop_file)
    if desktop_file.endswith(".desktop"):
        app = lookup.get(desktop_file.removesuffix(".desktop"))
        if app:
            return app.get("name", desktop_file)
    return desktop_file


def fuzzy_match(query, text):
    query = query.lower()
    text = text.lower()

    if query in text:
        return True

    qi = 0
    last_match = -1
    max_gap = 5

    for i, char in enumerate(text):
        if qi < len(query) and char == query[qi]:
            if last_match >= 0 and (i - last_match) > max_gap:
                return False
            last_match = i
            qi += 1

    return qi == len(query)


def matches_app(query, app):
    if fuzzy_match(query, app.get("name", "")):
        return True
    if fuzzy_match(query, app.get("generic_name", "")):
        return True
    if fuzzy_match(query, app.get("comment", "")):
        return True
    for keyword in app.get("keywords", []):
        if fuzzy_match(query, keyword):
            return True
    return False


def matches_target(query, target):
    if fuzzy_match(query, target.get("name", "")):
        return True
    for keyword in target.get("keywords", []):
        if fuzzy_match(query, keyword):
            return True
    return False


def get_target(target_id):
    for target in DEFAULT_TARGETS:
        if target["id"] == target_id:
            return target
    return None


def get_current_defaults(target):
    defaults = []
    for mime_type in target.get("mime_types", []):
        default = get_default_for_mime(mime_type)
        if default:
            defaults.append(default)
    return defaults


def summarize_defaults(defaults, app_lookup):
    unique = [item for item in dict.fromkeys(defaults) if item]
    if not unique:
        return "Not set"
    names = [lookup_app_name(app_lookup, item) for item in unique]
    if len(names) == 1:
        return names[0]
    if len(names) == 2:
        return f"Mixed: {names[0]}, {names[1]}"
    return f"Mixed: {names[0]}, {names[1]} +{len(names) - 2}"


def build_default_results(app_lookup, query):
    results = []
    for target in DEFAULT_TARGETS:
        if query and not matches_target(query, target):
            continue
        defaults = get_current_defaults(target)
        summary = summarize_defaults(defaults, app_lookup)
        actions = [{"id": "edit", "name": "Edit", "icon": "edit"}]
        if len(target.get("mime_types", [])) > 1:
            actions.append({"id": "mimes", "name": "Per MIME", "icon": "tune"})
        results.append(
            {
                "id": f"target:{target['id']}",
                "name": target["name"],
                "description": summary,
                "icon": target["icon"],
                "actions": actions,
            }
        )

    if not results:
        results = [
            {
                "id": "__empty__",
                "name": f"No defaults match '{query}'",
                "icon": "search_off",
            }
        ]

    return {
        "type": "results",
        "results": results,
        "inputMode": "realtime",
        "placeholder": "Search default apps...",
        "context": "",
    }


def get_candidate_apps(target, apps):
    wanted = set(target.get("mime_types", []))
    candidates = [app for app in apps if wanted.intersection(app.get("mime_types", []))]
    if candidates:
        return candidates, False
    return apps, True


def get_candidate_apps_for_mime(mime_type, apps):
    candidates = [app for app in apps if mime_type in app.get("mime_types", [])]
    if candidates:
        return candidates, False
    return apps, True


def build_mime_results(target, app_lookup, query):
    results = []
    for mime_type in target.get("mime_types", []):
        if query and not fuzzy_match(query, mime_type):
            continue
        default = get_default_for_mime(mime_type)
        description = lookup_app_name(app_lookup, default)
        results.append(
            {
                "id": f"mime:{mime_type}",
                "name": mime_type,
                "description": description,
                "icon": "label",
                "actions": [{"id": "edit", "name": "Edit", "icon": "edit"}],
            }
        )

    if not results:
        results = [
            {
                "id": "__empty__",
                "name": f"No mime types match '{query}'",
                "icon": "search_off",
            }
        ]

    return {
        "type": "results",
        "results": results,
        "inputMode": "realtime",
        "placeholder": f"Select {target['name'].lower()} mime type...",
        "context": f"__mime__:{target['id']}",
    }


def build_mime_edit_results(target, mime_type, apps, query):
    candidates, fallback = get_candidate_apps_for_mime(mime_type, apps)
    current_default = get_default_for_mime(mime_type)

    results = []
    for app in candidates:
        if query and not matches_app(query, app):
            continue
        result = {
            "id": f"app:{app['desktop_file']}",
            "name": app["name"],
            "description": app.get("generic_name") or app.get("comment") or "",
            "icon": app.get("icon", "application-x-executable"),
            "iconType": "system",
            "verb": "Set Default",
        }
        if current_default and app.get("desktop_file") == current_default:
            result["chips"] = [{"text": "Current"}]
        results.append(result)

    if not results:
        results = [
            {
                "id": "__empty__",
                "name": f"No apps match '{query}'",
                "icon": "search_off",
            }
        ]

    placeholder = f"Select default for {mime_type}..."
    if fallback:
        placeholder = f"{placeholder} (showing all apps)"

    return {
        "type": "results",
        "results": results[:50],
        "inputMode": "realtime",
        "placeholder": placeholder,
        "context": f"__mime_edit__:{target['id']}:{mime_type}",
    }


def build_edit_results(target, apps, query):
    candidates, fallback = get_candidate_apps(target, apps)
    current_defaults = set(get_current_defaults(target))

    results = []
    for app in candidates:
        if query and not matches_app(query, app):
            continue
        result = {
            "id": f"app:{app['desktop_file']}",
            "name": app["name"],
            "description": app.get("generic_name") or app.get("comment") or "",
            "icon": app.get("icon", "application-x-executable"),
            "iconType": "system",
            "verb": "Set Default",
        }
        if app.get("desktop_file") in current_defaults:
            result["chips"] = [{"text": "Current"}]
        results.append(result)

    if not results:
        results = [
            {
                "id": "__empty__",
                "name": f"No apps match '{query}'",
                "icon": "search_off",
            }
        ]

    placeholder = f"Select default {target['name'].lower()}..."
    if fallback:
        placeholder = f"{placeholder} (showing all apps)"

    return {
        "type": "results",
        "results": results[:50],
        "inputMode": "realtime",
        "placeholder": placeholder,
        "context": f"__edit__:{target['id']}",
    }


def main():
    request = json.load(sys.stdin)

    if not command_available("xdg-mime"):
        emit(
            {
                "type": "error",
                "message": "xdg-mime not found",
                "details": "Install xdg-utils to manage defaults.",
            }
        )
        return

    step = request.get("step", "initial")
    query = request.get("query", "").strip()
    selected = request.get("selected") or {}
    action = request.get("action")
    context = request.get("context") or ""

    all_apps = load_all_apps(include_nodisplay=True)
    all_apps.sort(key=lambda app: app["name"].lower())
    app_lookup = build_app_lookup(all_apps)

    if step == "initial":
        emit(build_default_results(app_lookup, ""))
        return

    if step == "search":
        if context.startswith("__mime_edit__:"):
            parts = context.split(":", 2)
            if len(parts) != 3:
                emit({"type": "error", "message": "Invalid mime context"})
                return
            target_id, mime_type = parts[1], parts[2]
            target = get_target(target_id)
            if not target:
                emit({"type": "error", "message": "Unknown default target"})
                return
            emit(build_mime_edit_results(target, mime_type, all_apps, query))
            return

        if context.startswith("__mime__:"):
            target_id = context.split(":", 1)[1]
            target = get_target(target_id)
            if not target:
                emit({"type": "error", "message": "Unknown default target"})
                return
            emit(build_mime_results(target, app_lookup, query))
            return

        if context.startswith("__edit__:"):
            target_id = context.split(":", 1)[1]
            target = get_target(target_id)
            if not target:
                emit({"type": "error", "message": "Unknown default target"})
                return
            emit(build_edit_results(target, all_apps, query))
            return

        emit(build_default_results(app_lookup, query))
        return

    if step == "action":
        selected_id = selected.get("id", "")

        if selected_id == "__empty__":
            emit({"type": "noop"})
            return

        if selected_id == "__back__":
            if context.startswith("__mime_edit__:"):
                parts = context.split(":", 2)
                if len(parts) != 3:
                    emit({"type": "error", "message": "Invalid mime context"})
                    return
                target = get_target(parts[1])
                if not target:
                    emit({"type": "error", "message": "Unknown default target"})
                    return
                response = build_mime_results(target, app_lookup, "")
            elif context.startswith("__mime__:") or context.startswith("__edit__:"):
                response = build_default_results(app_lookup, "")
            else:
                response = build_default_results(app_lookup, "")
            response["navigateBack"] = True
            response["clearInput"] = True
            emit(response)
            return

        if context.startswith("__edit__:"):
            target_id = context.split(":", 1)[1]
            target = get_target(target_id)
            if not target:
                emit({"type": "error", "message": "Unknown default target"})
                return

            if selected_id.startswith("app:"):
                desktop_file = selected_id.split(":", 1)[1]
                if not desktop_file.endswith(".desktop"):
                    desktop_file = f"{desktop_file}.desktop"

                error = set_default_for_mimes(desktop_file, target["mime_types"])
                if error:
                    emit(
                        {
                            "type": "error",
                            "message": f"Failed to set {target['name']} default",
                            "details": error,
                        }
                    )
                    return

                app_name = lookup_app_name(app_lookup, desktop_file)
                response = build_default_results(app_lookup, "")
                response["navigateBack"] = True
                response["clearInput"] = True
                response["notify"] = f"Set {target['name']} to {app_name}"
                emit(response)
                return

            emit({"type": "error", "message": "Unsupported selection"})
            return

        if context.startswith("__mime_edit__:"):
            parts = context.split(":", 2)
            if len(parts) != 3:
                emit({"type": "error", "message": "Invalid mime context"})
                return
            target_id, mime_type = parts[1], parts[2]
            target = get_target(target_id)
            if not target:
                emit({"type": "error", "message": "Unknown default target"})
                return

            if selected_id.startswith("app:"):
                desktop_file = selected_id.split(":", 1)[1]
                if not desktop_file.endswith(".desktop"):
                    desktop_file = f"{desktop_file}.desktop"

                error = set_default_for_mimes(desktop_file, [mime_type])
                if error:
                    emit(
                        {
                            "type": "error",
                            "message": f"Failed to set {mime_type} default",
                            "details": error,
                        }
                    )
                    return

                app_name = lookup_app_name(app_lookup, desktop_file)
                response = build_mime_results(target, app_lookup, "")
                response["navigateBack"] = True
                response["clearInput"] = True
                response["notify"] = f"Set {mime_type} to {app_name}"
                emit(response)
                return

            emit({"type": "error", "message": "Unsupported selection"})
            return

        if context.startswith("__mime__:"):
            target_id = context.split(":", 1)[1]
            target = get_target(target_id)
            if not target:
                emit({"type": "error", "message": "Unknown default target"})
                return

            if selected_id.startswith("mime:"):
                mime_type = selected_id.split(":", 1)[1]
                if action and action != "edit":
                    emit({"type": "noop"})
                    return
                response = build_mime_edit_results(target, mime_type, all_apps, "")
                response["navigateForward"] = True
                response["clearInput"] = True
                emit(response)
                return

            emit({"type": "noop"})
            return

        if selected_id.startswith("target:"):
            target_id = selected_id.split(":", 1)[1]
            target = get_target(target_id)
            if not target:
                emit({"type": "error", "message": "Unknown default target"})
                return
            if action == "mimes":
                response = build_mime_results(target, app_lookup, "")
                response["navigateForward"] = True
                response["clearInput"] = True
                emit(response)
                return
            if action and action != "edit":
                emit({"type": "noop"})
                return
            response = build_edit_results(target, all_apps, "")
            response["navigateForward"] = True
            response["clearInput"] = True
            emit(response)
            return

        emit({"type": "noop"})
        return

    emit({"type": "error", "message": f"Unsupported step: {step}"})


if __name__ == "__main__":
    main()
