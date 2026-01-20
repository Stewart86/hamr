#!/usr/bin/env python3
"""
Pictures workflow handler - searches for images in XDG Pictures directory.
"""

import json
import os
import subprocess
import sys
from datetime import datetime
from pathlib import Path

PICTURES_DIR = Path(os.environ.get("XDG_PICTURES_DIR", Path.home() / "Pictures"))
IMAGE_EXTENSIONS = {".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp", ".svg"}


def get_image_dimensions(path: str) -> tuple[int, int] | None:
    """Get image dimensions using PIL if available"""
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


def format_size(size: float) -> str:
    """Format file size in human readable format"""
    for unit in ["B", "KB", "MB", "GB"]:
        if size < 1024:
            return f"{size:.1f} {unit}"
        size /= 1024
    return f"{size:.1f} TB"


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

    images.sort(key=lambda x: x["mtime"], reverse=True)
    return images[:50]


def get_image_list_results(images: list[dict]) -> list[dict]:
    """Convert images to result format for browsing"""
    results = []
    for img in images:
        metadata = [
            {"label": "Size", "value": format_size(img["size"])},
        ]

        dims = get_image_dimensions(img["path"])
        if dims:
            metadata.append({"label": "Dimensions", "value": f"{dims[0]} x {dims[1]}"})

        metadata.append({"label": "Modified", "value": format_date(img["mtime"])})
        metadata.append({"label": "Path", "value": img["path"]})

        description = format_size(img["size"])
        if dims:
            description = f"{dims[0]}x{dims[1]} - {description}"

        results.append(
            {
                "id": img["id"],
                "name": img["name"],
                "description": description,
                "icon": "image",
                "thumbnail": img["path"],
                "verb": "Open",
                "preview": {
                    "image": img["path"],
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
                },
                "actions": [
                    {"id": "open", "name": "Open", "icon": "open_in_new"},
                    {"id": "copy-path", "name": "Copy Path", "icon": "content_copy"},
                ],
            }
        )
    return results


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
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                }
            )
        )
        return

    if step == "search":
        images = find_images(query)
        results = get_image_list_results(images)
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": results,
                    "inputMode": "realtime",
                }
            )
        )
        return

    if step == "action":
        item_id = selected.get("id", "")

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

        if action == "copy-image":
            subprocess.Popen(["wl-copy", "-t", "image/png", item_id])
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

        # Default action (Enter key) - open the image
        if Path(item_id).exists():
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

        print(json.dumps({"type": "error", "message": f"File not found: {item_id}"}))


if __name__ == "__main__":
    main()
