#!/usr/bin/env python3
"""
Files workflow handler - search and browse files using fd + fzf

Features:
- Fuzzy file search using fd + fzf
- Recent files from search history
- Actions: Open, Open folder, Copy path, Delete
- Directory navigation
"""

import json
import os
import subprocess
import sys
from datetime import datetime
from pathlib import Path

# Search history path (same as LauncherSearch.qml)
HAMR_CONFIG = Path.home() / ".config" / "hamr"
HISTORY_PATH = HAMR_CONFIG / "search-history.json"
HOME = str(Path.home())


def load_recent_files() -> list[dict]:
    """Load recent files from search history"""
    if not HISTORY_PATH.exists():
        return []
    try:
        with open(HISTORY_PATH) as f:
            data = json.load(f)
            history = data.get("history", [])
            # Filter to file entries and sort by frecency
            files = [h for h in history if h.get("type") == "file" and h.get("name")]
            # Simple frecency: count * recency factor
            now = __import__("time").time() * 1000
            for f in files:
                hours_since = (now - f.get("lastUsed", 0)) / (1000 * 60 * 60)
                if hours_since < 1:
                    mult = 4
                elif hours_since < 24:
                    mult = 2
                elif hours_since < 168:
                    mult = 1
                else:
                    mult = 0.5
                f["_frecency"] = f.get("count", 1) * mult
            files.sort(key=lambda x: x.get("_frecency", 0), reverse=True)
            return files[:20]
    except Exception:
        return []


def search_files(query: str, limit: int = 30) -> list[str]:
    """Search files using fd + fzf"""
    if not query:
        return []

    # fd command with exclusions
    fd_cmd = [
        "fd",
        "--type",
        "f",
        "--type",
        "d",
        "--hidden",
        "--follow",
        "--max-depth",
        "8",
        "--exclude",
        ".git",
        "--exclude",
        "node_modules",
        "--exclude",
        ".cache",
        "--exclude",
        ".local/share",
        "--exclude",
        ".mozilla",
        "--exclude",
        ".thunderbird",
        "--exclude",
        ".steam",
        "--exclude",
        ".wine",
        "--exclude",
        "__pycache__",
        "--exclude",
        ".npm",
        "--exclude",
        ".cargo",
        "--exclude",
        ".rustup",
        ".",
        HOME,
    ]

    # Pipe to fzf for fuzzy filtering
    fzf_cmd = ["fzf", "--filter", query]

    try:
        fd_proc = subprocess.Popen(
            fd_cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL
        )
        fzf_proc = subprocess.Popen(
            fzf_cmd,
            stdin=fd_proc.stdout,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        fd_proc.stdout.close()
        output, _ = fzf_proc.communicate(timeout=5)

        lines = output.decode().strip().split("\n")
        return [l for l in lines if l][:limit]
    except Exception:
        return []


def format_path(path: str) -> str:
    """Format path for display (replace home with ~)"""
    if path.startswith(HOME):
        return "~" + path[len(HOME) :]
    return path


def get_file_icon(path: str) -> str:
    """Get appropriate icon for file type"""
    if os.path.isdir(path):
        return "folder"

    ext = Path(path).suffix.lower()
    icon_map = {
        # Images
        ".png": "image",
        ".jpg": "image",
        ".jpeg": "image",
        ".gif": "image",
        ".webp": "image",
        ".svg": "image",
        ".bmp": "image",
        ".ico": "image",
        # Videos
        ".mp4": "movie",
        ".mkv": "movie",
        ".avi": "movie",
        ".mov": "movie",
        ".webm": "movie",
        # Audio
        ".mp3": "music_note",
        ".flac": "music_note",
        ".wav": "music_note",
        ".ogg": "music_note",
        ".m4a": "music_note",
        # Documents
        ".pdf": "picture_as_pdf",
        ".doc": "description",
        ".docx": "description",
        ".xls": "table_chart",
        ".xlsx": "table_chart",
        ".ppt": "slideshow",
        ".pptx": "slideshow",
        ".txt": "article",
        ".md": "article",
        ".rst": "article",
        # Code
        ".py": "code",
        ".js": "code",
        ".ts": "code",
        ".rs": "code",
        ".go": "code",
        ".c": "code",
        ".cpp": "code",
        ".h": "code",
        ".hpp": "code",
        ".java": "code",
        ".kt": "code",
        ".html": "html",
        ".css": "css",
        ".scss": "css",
        ".json": "data_object",
        ".yaml": "data_object",
        ".yml": "data_object",
        ".toml": "data_object",
        ".xml": "data_object",
        ".sh": "terminal",
        ".bash": "terminal",
        ".zsh": "terminal",
        # Archives
        ".zip": "folder_zip",
        ".tar": "folder_zip",
        ".gz": "folder_zip",
        ".7z": "folder_zip",
        ".rar": "folder_zip",
        # Config
        ".conf": "settings",
        ".cfg": "settings",
        ".ini": "settings",
    }
    return icon_map.get(ext, "description")


def format_size(size: int) -> str:
    """Format file size in human readable format"""
    size_float = float(size)
    for unit in ["B", "KB", "MB", "GB"]:
        if size_float < 1024:
            return f"{size_float:.1f} {unit}"
        size_float /= 1024
    return f"{size_float:.1f} TB"


def get_file_preview(path: str) -> dict | None:
    """Generate preview data for a file"""
    if not os.path.exists(path):
        return None

    is_dir = os.path.isdir(path)
    name = os.path.basename(path) or path
    ext = Path(path).suffix.lower()

    # Get file stats
    try:
        stat = os.stat(path)
        size = stat.st_size
        mtime = datetime.fromtimestamp(stat.st_mtime).strftime("%Y-%m-%d %H:%M")
    except OSError:
        size = 0
        mtime = "Unknown"

    metadata = [
        {"label": "Size", "value": format_size(size) if not is_dir else "Directory"},
        {"label": "Modified", "value": mtime},
        {"label": "Path", "value": path},
    ]

    actions = [
        {"id": "open", "name": "Open", "icon": "open_in_new"},
        {"id": "copy_path", "name": "Copy Path", "icon": "content_copy"},
        {"id": "open_folder", "name": "Open Folder", "icon": "folder_open"},
    ]

    # Image files - show image preview
    if ext in [".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp", ".svg"]:
        return {
            "type": "image",
            "content": path,
            "title": name,
            "metadata": metadata,
            "actions": actions,
        }

    # Text/code files - show text preview
    text_extensions = [
        ".txt",
        ".md",
        ".rst",
        ".py",
        ".js",
        ".ts",
        ".jsx",
        ".tsx",
        ".c",
        ".cpp",
        ".h",
        ".hpp",
        ".rs",
        ".go",
        ".java",
        ".kt",
        ".html",
        ".css",
        ".scss",
        ".json",
        ".yaml",
        ".yml",
        ".toml",
        ".xml",
        ".sh",
        ".bash",
        ".zsh",
        ".conf",
        ".cfg",
        ".ini",
        ".lua",
        ".vim",
        ".rb",
        ".php",
        ".sql",
        ".r",
        ".m",
        ".swift",
    ]
    if ext in text_extensions:
        try:
            with open(path, "r", errors="replace") as f:
                content = f.read(5000)  # Read first 5KB
                if len(content) == 5000:
                    content += "\n\n... (truncated)"

            # Use markdown for .md files
            preview_type = "markdown" if ext == ".md" else "text"

            return {
                "type": preview_type,
                "content": content,
                "title": name,
                "metadata": metadata,
                "actions": actions,
            }
        except Exception:
            pass

    # For other files, show metadata only
    if not is_dir:
        return {
            "type": "metadata",
            "content": "",
            "title": name,
            "metadata": metadata,
            "actions": actions,
        }

    return None


def path_to_result(path: str, show_actions: bool = True) -> dict:
    """Convert a file path to a result dict"""
    # Normalize path (remove trailing slash for directories)
    path = path.rstrip("/")
    is_dir = os.path.isdir(path)
    name = os.path.basename(path) or path
    folder_path = os.path.dirname(path)

    result = {
        "id": path,
        "name": name,
        "description": format_path(folder_path),
        "icon": get_file_icon(path),
        "verb": "Open",
    }

    if show_actions:
        actions = [
            {"id": "open_folder", "name": "Open folder", "icon": "folder_open"},
            {"id": "copy_path", "name": "Copy path", "icon": "content_copy"},
        ]
        # Add delete action for files (not directories for safety)
        if not is_dir:
            actions.append({"id": "delete", "name": "Delete", "icon": "delete"})
        result["actions"] = actions

    # Add thumbnail for images
    ext = Path(path).suffix.lower()
    if ext in [".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp"]:
        result["thumbnail"] = path

    # Add preview panel data
    preview = get_file_preview(path)
    if preview:
        result["preview"] = preview

    return result


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    selected_id = selected.get("id", "")

    if step == "initial":
        recent = load_recent_files()
        if recent:
            results = [
                path_to_result(f["name"]) for f in recent if os.path.exists(f["name"])
            ]
        else:
            results = [
                {
                    "id": "__info__",
                    "name": "Type to search files",
                    "description": "Using fd + fzf for fast fuzzy search",
                    "icon": "info",
                }
            ]

        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                    "placeholder": "Search files...",
                }
            )
        )
        return

    if step == "search":
        if query:
            paths = search_files(query)
            results = [path_to_result(p) for p in paths if os.path.exists(p)]
            if not results:
                results = [
                    {
                        "id": "__no_results__",
                        "name": f"No files found for '{query}'",
                        "icon": "search_off",
                    }
                ]
        else:
            # Empty query - show recent
            recent = load_recent_files()
            results = [
                path_to_result(f["name"]) for f in recent if os.path.exists(f["name"])
            ]

        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                    "placeholder": "Search files...",
                }
            )
        )
        return

    if step == "action":
        # Info/no-results items are not actionable
        if selected_id in ["__info__", "__no_results__"]:
            return

        path = selected_id

        # Copy path action
        if action == "copy_path":
            # Use wl-copy for Wayland
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["wl-copy", path],
                            "notify": f"Copied: {format_path(path)}",
                            "close": True,
                        },
                    }
                )
            )
            return

        # Open folder action
        if action == "open_folder":
            folder_path = os.path.dirname(path)
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["xdg-open", folder_path],
                            "name": f"Open {format_path(folder_path)}",
                            "icon": "folder_open",
                            "close": True,
                        },
                    }
                )
            )
            return

        # Delete action
        if action == "delete":
            if os.path.isfile(path):
                # Move to trash using gio
                print(
                    json.dumps(
                        {
                            "type": "execute",
                            "execute": {
                                "command": ["gio", "trash", path],
                                "notify": f"Moved to trash: {os.path.basename(path)}",
                                "close": False,
                            },
                        }
                    )
                )
                # Refresh results
                return
            return

        # Default action: Open file/folder
        if os.path.exists(path):
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["xdg-open", path],
                            "name": f"Open {os.path.basename(path)}",
                            "icon": get_file_icon(path),
                            "thumbnail": path
                            if Path(path).suffix.lower()
                            in [".png", ".jpg", ".jpeg", ".gif", ".webp"]
                            else "",
                            "close": True,
                        },
                    }
                )
            )
        else:
            print(json.dumps({"type": "error", "message": f"File not found: {path}"}))


if __name__ == "__main__":
    main()
