#!/usr/bin/env python3
"""
Screenshot workflow handler - browse and manage screenshots using the image browser.
Uses tesseract for OCR text extraction (Copy Text action).
"""

import json
import subprocess
import sys
import hashlib
from pathlib import Path

# Directories
PICTURES_DIR = Path.home() / "Pictures"
SCREENSHOTS_DIR = PICTURES_DIR / "Screenshots"
CACHE_DIR = Path.home() / ".cache" / "hamr" / "screenshot-ocr"
CACHE_FILE = CACHE_DIR / "ocr_index.json"


def load_cache() -> dict:
    """Load OCR cache from disk."""
    if CACHE_FILE.exists():
        try:
            return json.loads(CACHE_FILE.read_text())
        except (json.JSONDecodeError, IOError):
            pass
    return {}


def save_cache(cache: dict) -> None:
    """Save OCR cache to disk."""
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    CACHE_FILE.write_text(json.dumps(cache, indent=2))


def get_file_hash(filepath: Path) -> str:
    """Get a hash based on file path and mtime for cache invalidation."""
    stat = filepath.stat()
    key = f"{filepath}:{stat.st_mtime}:{stat.st_size}"
    return hashlib.md5(key.encode()).hexdigest()


def run_ocr(filepath: Path) -> str:
    """Run tesseract OCR on an image file."""
    try:
        # Get available languages
        lang_result = subprocess.run(
            ["tesseract", "--list-langs"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        langs = [
            l.strip() for l in lang_result.stdout.strip().split("\n")[1:] if l.strip()
        ]
        lang_str = "+".join(langs) if langs else "eng"

        # Run OCR
        result = subprocess.run(
            ["tesseract", str(filepath), "stdout", "-l", lang_str],
            capture_output=True,
            text=True,
            timeout=30,
        )
        return result.stdout.strip()
    except (subprocess.TimeoutExpired, FileNotFoundError, subprocess.SubprocessError):
        return ""


def get_ocr_text(filepath: Path, cache: dict) -> str:
    """Get OCR text for a file, using cache if available."""
    file_hash = get_file_hash(filepath)
    str_path = str(filepath)

    # Check cache
    if str_path in cache and cache[str_path].get("hash") == file_hash:
        return cache[str_path].get("text", "")

    # Run OCR and cache result
    text = run_ocr(filepath)
    cache[str_path] = {"hash": file_hash, "text": text}
    return text


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    selected = input_data.get("selected", {})

    # Initial or search: open image browser in Screenshots directory
    if step in ("initial", "search"):
        print(
            json.dumps(
                {
                    "type": "imageBrowser",
                    "imageBrowser": {
                        "directory": str(SCREENSHOTS_DIR)
                        if SCREENSHOTS_DIR.exists()
                        else str(PICTURES_DIR),
                        "title": "Screenshots",
                        "enableOcr": True,  # Enable background OCR indexing for text search
                        "actions": [
                            {"id": "open", "name": "Open", "icon": "open_in_new"},
                            {
                                "id": "copy",
                                "name": "Copy Image",
                                "icon": "content_copy",
                            },
                            {
                                "id": "ocr",
                                "name": "Copy Text (OCR)",
                                "icon": "text_fields",
                            },
                            {"id": "delete", "name": "Delete", "icon": "delete"},
                        ],
                    },
                }
            )
        )
        return

    # Handle image browser selection
    if step == "action" and selected.get("id") == "imageBrowser":
        file_path = selected.get("path", "")
        action_id = selected.get("action", "open")

        if not file_path:
            print(json.dumps({"type": "error", "message": "No file selected"}))
            return

        filepath = Path(file_path)
        filename = filepath.name

        # Open image
        if action_id == "open":
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["xdg-open", file_path],
                            "name": f"Open {filename}",
                            "icon": "screenshot_monitor",
                            "thumbnail": file_path,
                            "close": True,
                        },
                    }
                )
            )
            return

        # Copy image to clipboard
        if action_id == "copy":
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["bash", "-c", f"wl-copy < '{file_path}'"],
                            "notify": f"Copied: {filename}",
                            "name": f"Copy image: {filename}",
                            "icon": "content_copy",
                            "thumbnail": file_path,
                            "close": True,
                        },
                    }
                )
            )
            return

        # OCR and copy text
        if action_id == "ocr":
            cache = load_cache()
            ocr_text = get_ocr_text(filepath, cache)
            save_cache(cache)

            if not ocr_text:
                print(
                    json.dumps(
                        {
                            "type": "execute",
                            "execute": {
                                "notify": "No text found in image",
                                "close": False,
                            },
                        }
                    )
                )
                return

            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["wl-copy", ocr_text],
                            "notify": f"Copied text ({len(ocr_text)} chars)",
                            "name": f"Copy OCR: {filename}",
                            "icon": "text_fields",
                            "thumbnail": file_path,
                            "close": True,
                        },
                    }
                )
            )
            return

        # Delete (move to trash)
        if action_id == "delete":
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["gio", "trash", file_path],
                            "notify": f"Deleted: {filename}",
                            "close": False,
                        },
                    }
                )
            )
            return

        # Default: open
        print(
            json.dumps(
                {
                    "type": "execute",
                    "execute": {
                        "command": ["xdg-open", file_path],
                        "name": f"Open {filename}",
                        "icon": "screenshot_monitor",
                        "thumbnail": file_path,
                        "close": True,
                    },
                }
            )
        )
        return

    # Unknown step
    print(json.dumps({"type": "error", "message": f"Unknown step: {step}"}))


if __name__ == "__main__":
    main()
