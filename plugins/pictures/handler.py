#!/usr/bin/env python3
"""
Pictures workflow handler - searches for images in ~/Downloads/
Demonstrates multi-turn workflow: browse -> select -> actions
"""

import json
import sys
from pathlib import Path

DOWNLOADS_DIR = Path.home() / "Downloads"
IMAGE_EXTENSIONS = {".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp", ".svg"}


def find_images(query: str = "") -> list[dict]:
    """Find images in Downloads folder, optionally filtered by query"""
    images = []

    if not DOWNLOADS_DIR.exists():
        return images

    for file in DOWNLOADS_DIR.iterdir():
        if file.is_file() and file.suffix.lower() in IMAGE_EXTENSIONS:
            if not query or query.lower() in file.name.lower():
                images.append(
                    {
                        "id": str(file),
                        "name": file.name,
                        "path": str(file),
                        "size": file.stat().st_size,
                        "mtime": file.stat().st_mtime,
                    }
                )

    # Sort by modification time (newest first)
    images.sort(key=lambda x: x["mtime"], reverse=True)
    return images[:50]  # Limit to 50 results


def format_size(size: float) -> str:
    """Format file size in human readable format"""
    for unit in ["B", "KB", "MB", "GB"]:
        if size < 1024:
            return f"{size:.1f} {unit}"
        size /= 1024
    return f"{size:.1f} TB"


def get_image_list_results(images: list[dict]) -> list[dict]:
    """Convert images to result format for browsing"""
    return [
        {
            "id": img["id"],
            "name": img["name"],
            "description": format_size(img["size"]),
            "icon": "image",
            "thumbnail": img["path"],
            "actions": [
                {"id": "open", "name": "Open", "icon": "open_in_new"},
                {"id": "copy-path", "name": "Copy Path", "icon": "content_copy"},
            ],
        }
        for img in images
    ]


def get_image_detail_results(image_path: str) -> list[dict]:
    """Show detail view for a selected image with back option"""
    return [
        {
            "id": "__back__",
            "name": "Back to list",
            "icon": "arrow_back",
        },
        {
            "id": f"open:{image_path}",
            "name": "Open in viewer",
            "icon": "open_in_new",
            "verb": "Open",
        },
        {
            "id": f"copy-path:{image_path}",
            "name": "Copy file path",
            "icon": "content_copy",
        },
        {
            "id": f"copy-image:{image_path}",
            "name": "Copy image to clipboard",
            "icon": "image",
        },
        {
            "id": f"delete:{image_path}",
            "name": "Move to trash",
            "icon": "delete",
        },
    ]


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    # Initial: show image list
    if step == "initial":
        images = find_images()
        results = get_image_list_results(images)
        print(
            json.dumps({"type": "results", "results": results, "inputMode": "realtime"})
        )
        return

    # Search: filter image list
    if step == "search":
        images = find_images(query)
        results = get_image_list_results(images)
        print(
            json.dumps({"type": "results", "results": results, "inputMode": "realtime"})
        )
        return

    # Action: handle item click or action button
    if step == "action":
        item_id = selected.get("id", "")

        # Back button - return to list
        if item_id == "__back__":
            images = find_images()
            results = get_image_list_results(images)
            print(json.dumps({"type": "results", "results": results}))
            return

        # Action button clicks (open, copy-path from list view)
        if action == "open":
            filename = Path(item_id).name
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["xdg-open", item_id],
                            "name": f"Open {filename}",
                            "icon": "image",
                            "thumbnail": item_id,
                            "close": True,
                        },
                    }
                )
            )
            return

        if action == "copy-path":
            filename = Path(item_id).name
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["wl-copy", item_id],
                            "notify": f"Copied: {item_id}",
                            "name": f"Copy path: {filename}",
                            "icon": "content_copy",
                            "close": True,
                        },
                    }
                )
            )
            return

        # Detail view actions (from clicking items in detail view)
        if item_id.startswith("open:"):
            path = item_id.split(":", 1)[1]
            filename = Path(path).name
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["xdg-open", path],
                            "name": f"Open {filename}",
                            "icon": "image",
                            "thumbnail": path,
                            "close": True,
                        },
                    }
                )
            )
            return

        if item_id.startswith("copy-path:"):
            path = item_id.split(":", 1)[1]
            filename = Path(path).name
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["wl-copy", path],
                            "notify": f"Copied: {path}",
                            "name": f"Copy path: {filename}",
                            "icon": "content_copy",
                            "close": True,
                        },
                    }
                )
            )
            return

        if item_id.startswith("copy-image:"):
            path = item_id.split(":", 1)[1]
            filename = Path(path).name
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["wl-copy", "-t", "image/png", path],
                            "notify": "Image copied to clipboard",
                            "name": f"Copy image: {filename}",
                            "icon": "image",
                            "thumbnail": path,
                            "close": True,
                        },
                    }
                )
            )
            return

        if item_id.startswith("delete:"):
            path = item_id.split(":", 1)[1]
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["gio", "trash", path],
                            "notify": f"Moved to trash: {Path(path).name}",
                            "close": True,
                        },
                    }
                )
            )
            return

        # Default click on image - show detail view (multi-turn!)
        if Path(item_id).exists():
            results = get_image_detail_results(item_id)
            print(
                json.dumps(
                    {"type": "results", "results": results, "inputMode": "realtime"}
                )
            )
            return

        # Unknown action
        print(json.dumps({"type": "error", "message": f"Unknown action: {item_id}"}))


if __name__ == "__main__":
    main()
