#!/usr/bin/env python3
"""
Pictures workflow handler - searches for images in XDG Pictures directory
Demonstrates multi-turn workflow: browse -> select -> actions
"""

import json
import os
import subprocess
import sys
from datetime import datetime
from pathlib import Path

# Test mode for development
TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

PICTURES_DIR = Path(os.environ.get("XDG_PICTURES_DIR", Path.home() / "Pictures"))
IMAGE_EXTENSIONS = {".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp", ".svg"}


def get_image_dimensions(path: str) -> tuple[int, int] | None:
    """Get image dimensions using PIL if available, else return None"""
    if TEST_MODE:
        return (1920, 1080)  # Mock dimensions
    try:
        from PIL import Image

        with Image.open(path) as img:
            return img.size
    except Exception:
        return None


def format_date(timestamp: float) -> str:
    """Format timestamp to human readable date"""
    dt = datetime.fromtimestamp(timestamp)
    return dt.strftime("%Y-%m-%d %H:%M")


def find_images(query: str = "") -> list[dict]:
    """Find images in Pictures folder, optionally filtered by query"""
    images = []

    if not PICTURES_DIR.exists():
        return images

    for file in PICTURES_DIR.iterdir():
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
    results = []
    for img in images:
        # Build metadata with dimensions if available
        metadata = [
            {"label": "Size", "value": format_size(img["size"])},
        ]

        # Try to get image dimensions
        dims = get_image_dimensions(img["path"])
        if dims:
            metadata.append({"label": "Dimensions", "value": f"{dims[0]} x {dims[1]}"})

        # Add modification date
        metadata.append({"label": "Modified", "value": format_date(img["mtime"])})
        metadata.append({"label": "Path", "value": img["path"]})

        # Build description with dimensions if available
        description = format_size(img["size"])
        if dims:
            description = f"{dims[0]}x{dims[1]} · {description}"

        results.append(
            {
                "id": img["id"],
                "name": img["name"],
                "description": description,
                "icon": "image",
                "thumbnail": img["path"],
                "preview": {
                    "type": "image",
                    "content": img["path"],
                    "title": img["name"],
                    "metadata": metadata,
                    "actions": [
                        {"id": "open", "name": "Open", "icon": "open_in_new"},
                        {
                            "id": "copy-path",
                            "name": "Copy Path",
                            "icon": "content_copy",
                        },
                        {"id": "copy-image", "name": "Copy Image", "icon": "image"},
                    ],
                    "detachable": True,
                },
                "actions": [
                    {"id": "open", "name": "Open", "icon": "open_in_new"},
                    {"id": "copy-path", "name": "Copy Path", "icon": "content_copy"},
                ],
            }
        )
    return results


def get_image_detail_results(image_path: str) -> list[dict]:
    """Show detail view for a selected image"""
    return [
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

    if step == "initial":
        images = find_images()
        results = get_image_list_results(images)
        print(
            json.dumps({"type": "results", "results": results, "inputMode": "realtime"})
        )
        return

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
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": results,
                        "navigateBack": True,  # Going back to list
                    }
                )
            )
            return

        # Action button clicks (open, copy-path from list view)
        if action == "open":
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "open": item_id,
                        "close": True,
                    }
                )
            )
            return

        if action == "copy-path":
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "copy": item_id,
                        "notify": f"Copied: {item_id}",
                        "close": True,
                    }
                )
            )
            return

        # Detail view actions (from clicking items in detail view)
        if item_id.startswith("open:"):
            path = item_id.split(":", 1)[1]
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "open": path,
                        "close": True,
                    }
                )
            )
            return

        if item_id.startswith("copy-path:"):
            path = item_id.split(":", 1)[1]
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "copy": path,
                        "notify": f"Copied: {path}",
                        "close": True,
                    }
                )
            )
            return

        if item_id.startswith("copy-image:"):
            path = item_id.split(":", 1)[1]
            subprocess.Popen(["wl-copy", "-t", "image/png", path])
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "notify": "Image copied to clipboard",
                        "close": True,
                    }
                )
            )
            return

        if item_id.startswith("delete:"):
            path = item_id.split(":", 1)[1]
            filename = Path(path).name
            subprocess.Popen(["gio", "trash", path])
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "notify": f"Moved to trash: {filename}",
                        "close": True,
                    }
                )
            )
            return

        # Default click on image - show detail view (multi-turn!)
        if Path(item_id).exists():
            results = get_image_detail_results(item_id)
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": results,
                        "inputMode": "realtime",
                        "navigateForward": True,  # Drilling into image detail
                    }
                )
            )
            return

        # Unknown action
        print(json.dumps({"type": "error", "message": f"Unknown action: {item_id}"}))


if __name__ == "__main__":
    main()
