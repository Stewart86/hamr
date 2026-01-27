#!/usr/bin/env python3
"""
Screenshot plugin for hamr - Browse and manage screenshots with OCR.

Features:
- Browse screenshots from Pictures/Screenshots directory
- Background OCR indexing when plugin is open
- Search by filename or OCR text content
- Copy images to clipboard
- Extract and copy text from images
- Delete screenshots
"""

import asyncio
import hashlib
import json
import os
import shutil
import subprocess
import sys
from datetime import datetime
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from sdk.hamr_sdk import HamrPlugin

# Directories
PICTURES_DIR = Path.home() / "Pictures"
SCREENSHOTS_DIR = PICTURES_DIR / "Screenshots"
CACHE_DIR = Path.home() / ".cache" / "hamr" / "screenshot-ocr"
CACHE_FILE = CACHE_DIR / "ocr_index.json"

IMAGE_EXTENSIONS = {".png", ".jpg", ".jpeg", ".webp"}


def load_ocr_cache() -> dict:
    """Load OCR cache from disk."""
    if CACHE_FILE.exists():
        try:
            return json.loads(CACHE_FILE.read_text())
        except (json.JSONDecodeError, IOError):
            pass
    return HamrPlugin.noop()


def save_ocr_cache(cache: dict) -> None:
    """Save OCR cache to disk."""
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    CACHE_FILE.write_text(json.dumps(cache, indent=2))


def get_file_hash(filepath: Path) -> str:
    """Get a hash based on file path and mtime for cache invalidation."""
    try:
        stat = filepath.stat()
        key = f"{filepath}:{stat.st_mtime}:{stat.st_size}"
        return hashlib.md5(key.encode()).hexdigest()
    except (FileNotFoundError, OSError):
        return ""


def has_tesseract() -> bool:
    """Check if tesseract is available."""
    return shutil.which("tesseract") is not None


def get_tesseract_languages() -> str:
    """Get available tesseract languages."""
    try:
        result = subprocess.run(
            ["tesseract", "--list-langs"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        langs = [
            line.strip()
            for line in result.stdout.strip().split("\n")[1:]
            if line.strip()
        ]
        return "+".join(langs) if langs else "eng"
    except (subprocess.TimeoutExpired, FileNotFoundError, subprocess.SubprocessError):
        return "eng"


def run_ocr(filepath: Path, lang_str: str) -> str:
    """Run tesseract OCR on an image file."""
    try:
        result = subprocess.run(
            ["tesseract", str(filepath), "stdout", "-l", lang_str],
            capture_output=True,
            text=True,
            timeout=30,
        )
        return result.stdout.strip()
    except (subprocess.TimeoutExpired, FileNotFoundError, subprocess.SubprocessError):
        return ""


def format_date(timestamp: float) -> str:
    """Format timestamp to human readable date."""
    dt = datetime.fromtimestamp(timestamp)
    return dt.strftime("%Y-%m-%d %H:%M")


def format_size(size: float) -> str:
    """Format file size in human readable format."""
    for unit in ["B", "KB", "MB", "GB"]:
        if size < 1024:
            return f"{size:.1f} {unit}"
        size /= 1024
    return f"{size:.1f} TB"


def get_screenshots() -> list[dict]:
    """Get all screenshots from the directory."""
    directory = SCREENSHOTS_DIR if SCREENSHOTS_DIR.exists() else PICTURES_DIR
    if not directory.exists():
        return []

    screenshots = []
    for f in directory.iterdir():
        if f.is_file() and f.suffix.lower() in IMAGE_EXTENSIONS:
            try:
                stat = f.stat()
                screenshots.append(
                    {
                        "path": str(f),
                        "name": f.name,
                        "stem": f.stem,
                        "size": stat.st_size,
                        "mtime": stat.st_mtime,
                    }
                )
            except (FileNotFoundError, OSError):
                continue

    screenshots.sort(key=lambda x: x["mtime"], reverse=True)
    return screenshots


def build_results(
    screenshots: list[dict], ocr_cache: dict, query: str = ""
) -> list[dict]:
    """Build result items from screenshots."""
    results = []
    query_lower = query.lower() if query else ""

    for screenshot in screenshots:
        path = screenshot["path"]
        name = screenshot["name"]
        stem = screenshot["stem"]

        # Get OCR text from cache
        ocr_text = ""
        if path in ocr_cache:
            ocr_text = ocr_cache[path].get("text", "")

        # Filter by query (match name or OCR text)
        if query_lower:
            name_match = query_lower in name.lower()
            ocr_match = query_lower in ocr_text.lower()
            if not name_match and not ocr_match:
                continue

        # Build description
        description = format_date(screenshot["mtime"])
        if ocr_text:
            preview = ocr_text[:50].replace("\n", " ")
            if len(ocr_text) > 50:
                preview += "..."
            description += f" | {preview}"

        # Build metadata for preview panel
        metadata = [
            {"label": "Size", "value": format_size(screenshot["size"])},
            {"label": "Modified", "value": format_date(screenshot["mtime"])},
        ]
        if ocr_text:
            metadata.append({"label": "Text", "value": f"{len(ocr_text)} chars"})

        results.append(
            {
                "id": path,
                "name": stem,
                "description": description,
                "icon": "screenshot_monitor",
                "thumbnail": path,
                "verb": "Open",
                "preview": {
                    "image": path,
                    "title": name,
                    "metadata": metadata,
                    "actions": [
                        {"id": "open", "name": "Open", "icon": "open_in_new"},
                        {"id": "copy", "name": "Copy Image", "icon": "content_copy"},
                        {"id": "ocr", "name": "Copy Text", "icon": "text_fields"},
                        {"id": "delete", "name": "Delete", "icon": "delete"},
                    ],
                },
                "actions": [
                    {"id": "copy", "name": "Copy Image", "icon": "content_copy"},
                    {"id": "ocr", "name": "Copy Text (OCR)", "icon": "text_fields"},
                    {"id": "delete", "name": "Delete", "icon": "delete"},
                ],
            }
        )

    if not results:
        if query:
            results.append(
                {
                    "id": "__empty__",
                    "name": f"No screenshots matching '{query}'",
                    "icon": "search_off",
                    "description": "Try a different search term",
                }
            )
        else:
            results.append(
                {
                    "id": "__empty__",
                    "name": "No screenshots found",
                    "icon": "info",
                    "description": f"Add screenshots to {SCREENSHOTS_DIR}",
                }
            )

    return results


plugin = HamrPlugin(
    id="screenshot",
    name="Screenshots",
    description="Browse and search screenshots with OCR",
    icon="screenshot_monitor",
)

state = {
    "screenshots": [],
    "ocr_cache": {},
    "current_query": "",
    "plugin_active": False,
}


def get_plugin_actions() -> list[dict]:
    """Get plugin-level actions."""
    return [
        {
            "id": "refresh",
            "name": "Refresh",
            "icon": "refresh",
            "shortcut": "Ctrl+R",
        },
    ]


@plugin.on_initial
async def handle_initial(params=None):
    """Handle initial request when plugin is opened."""
    state["plugin_active"] = True
    state["screenshots"] = get_screenshots()
    state["ocr_cache"] = load_ocr_cache()
    state["current_query"] = ""

    results = build_results(
        state["screenshots"], state["ocr_cache"], state["current_query"]
    )

    return HamrPlugin.results(
        results,
        display_hint="large_grid",
        plugin_actions=get_plugin_actions(),
        placeholder="Search screenshots...",
    )


@plugin.on_search
async def handle_search(query: str, context=None):
    """Handle search request."""
    state["current_query"] = query
    results = build_results(state["screenshots"], state["ocr_cache"], query)

    return HamrPlugin.results(
        results,
        display_hint="large_grid",
        plugin_actions=get_plugin_actions(),
        placeholder="Search screenshots...",
    )


@plugin.on_action
async def handle_action(item_id: str, action=None, context=None):
    """Handle action request."""
    # Plugin-level actions
    if item_id == "__plugin__":
        if action == "refresh":
            state["screenshots"] = get_screenshots()
            results = build_results(
                state["screenshots"], state["ocr_cache"], state["current_query"]
            )
            await plugin.send_results(
                results,
                displayHint="large_grid",
                pluginActions=get_plugin_actions(),
                placeholder="Search screenshots...",
            )
            return HamrPlugin.noop()
        return HamrPlugin.noop()

    # Skip empty items
    if item_id == "__empty__":
        return HamrPlugin.noop()

    filepath = Path(item_id)
    if not filepath.exists():
        return HamrPlugin.error("File not found")

    filename = filepath.name

    # Open image (default action)
    if not action or action == "open":
        return HamrPlugin.execute(url=f"file://{item_id}", close=True)

    # Copy image to clipboard
    if action == "copy":
        try:
            with open(item_id, "rb") as f:
                subprocess.Popen(["wl-copy"], stdin=f)
            await plugin.send_execute(
                {"type": "notify", "message": f"Copied: {filename}"}
            )
            return HamrPlugin.close()
        except (FileNotFoundError, subprocess.SubprocessError, IOError):
            return HamrPlugin.error("Failed to copy image")

    # OCR and copy text
    if action == "ocr":
        ocr_text = state["ocr_cache"].get(item_id, {}).get("text", "")

        if not ocr_text:
            # Run OCR if not cached
            lang_str = get_tesseract_languages()
            ocr_text = run_ocr(filepath, lang_str)
            if ocr_text:
                state["ocr_cache"][item_id] = {
                    "hash": get_file_hash(filepath),
                    "text": ocr_text,
                }
                save_ocr_cache(state["ocr_cache"])

        if not ocr_text:
            await plugin.send_execute(
                {"type": "notify", "message": "No text found in image"}
            )
            return HamrPlugin.noop()

        try:
            process = subprocess.Popen(["wl-copy"], stdin=subprocess.PIPE, text=True)
            process.communicate(input=ocr_text)
            await plugin.send_execute(
                {"type": "notify", "message": f"Copied text ({len(ocr_text)} chars)"}
            )
            return HamrPlugin.close()
        except (subprocess.SubprocessError, OSError):
            return HamrPlugin.error("Failed to copy text")

    # Delete (move to trash)
    if action == "delete":
        try:
            subprocess.run(["gio", "trash", item_id], check=True, capture_output=True)
            state["screenshots"] = [
                s for s in state["screenshots"] if s["path"] != item_id
            ]
            results = build_results(
                state["screenshots"], state["ocr_cache"], state["current_query"]
            )
            await plugin.send_results(
                results,
                displayHint="large_grid",
                pluginActions=get_plugin_actions(),
                placeholder="Search screenshots...",
            )
            await plugin.send_execute(
                {"type": "notify", "message": f"Deleted: {filename}"}
            )
            return HamrPlugin.noop()
        except (subprocess.CalledProcessError, FileNotFoundError):
            return HamrPlugin.error("Failed to delete file")

    return HamrPlugin.noop()


@plugin.add_background_task
async def ocr_indexer(p: HamrPlugin):
    """Background task to OCR index screenshots.

    Runs until plugin process is killed (on launcher close or navigate back).
    """
    if not has_tesseract():
        p._log("Tesseract not found, OCR indexing disabled")
        return

    lang_str = get_tesseract_languages()
    p._log(f"OCR languages: {lang_str}")

    while True:
        await asyncio.sleep(0.5)

        if not state["plugin_active"]:
            continue

        screenshots = state["screenshots"]
        ocr_cache = state["ocr_cache"]

        # Find screenshots that need OCR
        needs_ocr = []
        for screenshot in screenshots:
            path = screenshot["path"]
            file_hash = get_file_hash(Path(path))

            if not file_hash:
                continue

            cached = ocr_cache.get(path)
            if cached and cached.get("hash") == file_hash:
                continue

            needs_ocr.append((path, file_hash))

        if not needs_ocr:
            continue

        # Notify starting OCR
        await p.send_execute(
            {"type": "notify", "message": f"Scanning {len(needs_ocr)} screenshots..."}
        )

        indexed_count = 0
        for path, file_hash in needs_ocr:
            filepath = Path(path)
            if not filepath.exists():
                continue

            ocr_text = run_ocr(filepath, lang_str)
            ocr_cache[path] = {"hash": file_hash, "text": ocr_text}
            indexed_count += 1

            # Update results periodically
            if indexed_count % 5 == 0:
                save_ocr_cache(ocr_cache)
                results = build_results(screenshots, ocr_cache, state["current_query"])
                await p.send_results(
                    results,
                    displayHint="large_grid",
                    pluginActions=get_plugin_actions(),
                    placeholder="Search screenshots...",
                )

        # Final save and update
        if indexed_count > 0:
            save_ocr_cache(ocr_cache)
            results = build_results(screenshots, ocr_cache, state["current_query"])
            await p.send_results(
                results,
                displayHint="large_grid",
                pluginActions=get_plugin_actions(),
                placeholder="Search screenshots...",
            )
            await p.send_execute(
                {
                    "type": "notify",
                    "message": f"OCR complete: {indexed_count} screenshots",
                }
            )


if __name__ == "__main__":
    plugin.run()
