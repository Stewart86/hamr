#!/usr/bin/env python3
"""
Background OCR indexer for clipboard images.
Runs as a subprocess, indexes images, updates cache, then exits.
"""

import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

# Cache directory
CACHE_DIR = (
    Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache"))
    / "hamr"
    / "clipboard-thumbs"
)
OCR_CACHE_FILE = CACHE_DIR / "ocr-index.json"
LOCK_FILE = CACHE_DIR / "ocr-indexer.lock"

# Optimization settings
MAX_IMAGES_TO_OCR = 20  # Only OCR the most recent N images (OCR is slow)
MIN_IMAGE_WIDTH = 100  # Skip images smaller than this (for OCR)
MIN_IMAGE_HEIGHT = 50
MAX_THUMB_SIZE = 256  # Max thumbnail dimension


def is_already_running() -> bool:
    """Check if another indexer is already running"""
    if not LOCK_FILE.exists():
        return False
    try:
        pid = int(LOCK_FILE.read_text().strip())
        os.kill(pid, 0)
        return True
    except (ValueError, ProcessLookupError, PermissionError):
        # Stale lock file
        LOCK_FILE.unlink(missing_ok=True)
        return False


def acquire_lock() -> bool:
    """Try to acquire lock, return True if successful"""
    if is_already_running():
        return False
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    LOCK_FILE.write_text(str(os.getpid()))
    return True


def release_lock():
    """Release the lock"""
    LOCK_FILE.unlink(missing_ok=True)


def load_ocr_cache() -> dict[str, str]:
    """Load OCR cache from disk"""
    if OCR_CACHE_FILE.exists():
        try:
            return json.loads(OCR_CACHE_FILE.read_text())
        except (json.JSONDecodeError, IOError):
            pass
    return {}


def save_ocr_cache(cache: dict[str, str]) -> None:
    """Save OCR cache to disk"""
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    OCR_CACHE_FILE.write_text(json.dumps(cache))


def get_clipboard_entries() -> list[str]:
    """Get clipboard entries from cliphist"""
    try:
        result = subprocess.run(
            ["cliphist", "list"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        if result.returncode == 0:
            return [line for line in result.stdout.strip().split("\n") if line]
    except (subprocess.TimeoutExpired, FileNotFoundError):
        pass
    return []


def is_image(entry: str) -> bool:
    """Check if entry is an image"""
    return bool(re.match(r"^\d+\t\[\[.*binary data.*\d+x\d+.*\]\]$", entry))


def get_image_dimensions(entry: str) -> tuple[int, int] | None:
    """Extract image dimensions from entry"""
    match = re.search(r"(\d+)x(\d+)", entry)
    if match:
        return int(match.group(1)), int(match.group(2))
    return None


def is_image_worth_ocr(entry: str) -> bool:
    """Check if image is large enough to be worth OCR'ing"""
    dims = get_image_dimensions(entry)
    if not dims:
        return False
    width, height = dims
    return width >= MIN_IMAGE_WIDTH and height >= MIN_IMAGE_HEIGHT


def get_entry_hash(entry: str) -> str:
    """Get a stable hash for a clipboard entry"""
    return hashlib.md5(entry.encode()).hexdigest()[:16]


def get_tesseract_languages() -> str:
    """Get available tesseract languages as a + separated string."""
    try:
        result = subprocess.run(
            ["tesseract", "--list-langs"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        langs = [
            lang.strip()
            for lang in result.stdout.strip().split("\n")[1:]
            if lang.strip()
        ]
        return "+".join(langs) if langs else "eng"
    except (subprocess.TimeoutExpired, FileNotFoundError, subprocess.SubprocessError):
        return "eng"


def decode_image(entry: str) -> bytes | None:
    """Decode image from cliphist, return raw bytes or None"""
    try:
        decode_proc = subprocess.run(
            ["cliphist", "decode"],
            input=entry.encode("utf-8"),
            capture_output=True,
            timeout=5,
        )
        if decode_proc.returncode == 0 and decode_proc.stdout:
            return decode_proc.stdout
    except (subprocess.TimeoutExpired, FileNotFoundError, subprocess.SubprocessError):
        pass
    return None


def generate_thumbnail(entry: str, image_data: bytes) -> bool:
    """Generate thumbnail for an image entry, return True if successful"""
    entry_hash = get_entry_hash(entry)
    thumb_path = CACHE_DIR / f"{entry_hash}.png"

    if thumb_path.exists():
        return True

    try:
        dims = get_image_dimensions(entry)
        if dims and (dims[0] > MAX_THUMB_SIZE or dims[1] > MAX_THUMB_SIZE):
            # Resize with ImageMagick
            resize_proc = subprocess.run(
                [
                    "magick",
                    "-",
                    "-thumbnail",
                    f"{MAX_THUMB_SIZE}x{MAX_THUMB_SIZE}>",
                    str(thumb_path),
                ],
                input=image_data,
                capture_output=True,
                timeout=10,
            )
            if resize_proc.returncode == 0 and thumb_path.exists():
                return True

        # Save as-is if small or resize failed
        thumb_path.write_bytes(image_data)
        return True

    except (subprocess.TimeoutExpired, Exception):
        pass
    return False


def run_ocr_on_image(image_data: bytes, lang_str: str) -> str:
    """Run OCR on image data"""
    try:
        # Use --psm 3 (fully automatic page segmentation) for speed
        ocr_proc = subprocess.run(
            ["tesseract", "stdin", "stdout", "-l", lang_str, "--psm", "3"],
            input=image_data,
            capture_output=True,
            timeout=15,
        )
        return ocr_proc.stdout.decode("utf-8", errors="replace").strip()
    except (subprocess.TimeoutExpired, FileNotFoundError, subprocess.SubprocessError):
        return ""


def notify(message: str, title: str = "Clipboard"):
    """Send desktop notification"""
    try:
        subprocess.run(
            ["notify-send", title, message, "-a", "Hamr", "-t", "2000"],
            capture_output=True,
            timeout=5,
        )
    except (subprocess.TimeoutExpired, FileNotFoundError):
        pass


def main():
    # Try to acquire lock (exit if another indexer is running)
    if not acquire_lock():
        sys.exit(0)

    try:
        CACHE_DIR.mkdir(parents=True, exist_ok=True)

        # Load existing OCR cache
        ocr_cache = load_ocr_cache()

        # Get all clipboard entries
        entries = get_clipboard_entries()

        # Get all image entries (most recent first from cliphist)
        image_entries = [e for e in entries if is_image(e)]

        top_ocr_candidates = []
        for entry in image_entries:
            if is_image_worth_ocr(entry):
                top_ocr_candidates.append(entry)
                if len(top_ocr_candidates) >= MAX_IMAGES_TO_OCR:
                    break

        # Check which of the top N need OCR (not already cached)
        ocr_needed = []
        for entry in top_ocr_candidates:
            entry_hash = get_entry_hash(entry)
            if entry_hash not in ocr_cache:
                ocr_needed.append(entry)

        # Check which images need thumbnails (all images)
        thumbs_needed = []
        for entry in image_entries:
            entry_hash = get_entry_hash(entry)
            thumb_path = CACHE_DIR / f"{entry_hash}.png"
            if not thumb_path.exists():
                thumbs_needed.append(entry)

        if not ocr_needed and not thumbs_needed:
            sys.exit(0)

        # Only notify if there's OCR work to do (thumbnails are fast)
        if ocr_needed:
            notify(f"Indexing {len(ocr_needed)} images...")

        # Get tesseract languages once (for OCR)
        lang_str = get_tesseract_languages()

        # Process thumbnails first (fast)
        for entry in thumbs_needed:
            image_data = decode_image(entry)
            if image_data:
                generate_thumbnail(entry, image_data)

        # Process OCR (slow, only for top N recent images)
        ocr_count = 0
        for entry in ocr_needed:
            entry_hash = get_entry_hash(entry)
            image_data = decode_image(entry)
            if not image_data:
                continue

            text = run_ocr_on_image(image_data, lang_str)
            ocr_cache[entry_hash] = text
            save_ocr_cache(ocr_cache)
            ocr_count += 1

        # Notify done (only if OCR was performed)
        if ocr_count > 0:
            notify(f"Indexed {ocr_count} images")

    finally:
        release_lock()


if __name__ == "__main__":
    main()
