#!/usr/bin/env python3
"""
OCR indexing script for ImageBrowser.
Runs tesseract on images in a directory and caches results for fast text search.
Similar to thumbgen.py but for OCR text extraction.

Output format (machine_progress mode):
  PROGRESS current/total
  FILE /path/to/file.png
  OCR /path/to/file.png|extracted text here (newlines replaced with \\n)

Cache stored in ~/.cache/hamr/ocr-index/<directory-hash>.json
"""

import hashlib
import json
import os
import subprocess
import sys
from multiprocessing import Pool
from pathlib import Path
from typing import Optional

import click

# Cache directory
CACHE_DIR = Path.home() / ".cache" / "hamr" / "ocr-index"

# Image extensions to process
IMAGE_EXTENSIONS = {".png", ".jpg", ".jpeg", ".webp"}

# Global state for multiprocessing
_lang_str = "eng"
_cache = {}


def get_directory_hash(directory: str) -> str:
    """Get a hash for the directory path to use as cache filename."""
    return hashlib.md5(directory.encode()).hexdigest()


def get_file_hash(filepath: Path) -> str:
    """Get a hash based on file path and mtime for cache invalidation."""
    stat = filepath.stat()
    key = f"{filepath}:{stat.st_mtime}:{stat.st_size}"
    return hashlib.md5(key.encode()).hexdigest()


def load_cache(directory: str) -> dict:
    """Load OCR cache for a directory."""
    cache_file = CACHE_DIR / f"{get_directory_hash(directory)}.json"
    if cache_file.exists():
        try:
            return json.loads(cache_file.read_text())
        except (json.JSONDecodeError, IOError):
            pass
    return {"directory": directory, "files": {}}


def save_cache(directory: str, cache: dict) -> None:
    """Save OCR cache for a directory."""
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    cache_file = CACHE_DIR / f"{get_directory_hash(directory)}.json"
    cache_file.write_text(json.dumps(cache, indent=2))


def get_tesseract_languages() -> str:
    """Get available tesseract languages as a + separated string."""
    try:
        result = subprocess.run(
            ["tesseract", "--list-langs"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        langs = [l.strip() for l in result.stdout.strip().split("\n")[1:] if l.strip()]
        return "+".join(langs) if langs else "eng"
    except (subprocess.TimeoutExpired, FileNotFoundError, subprocess.SubprocessError):
        return "eng"


def run_ocr(filepath: str) -> tuple[str, str]:
    """Run tesseract OCR on an image file. Returns (filepath, text)."""
    global _lang_str, _cache

    path = Path(filepath)
    file_hash = get_file_hash(path)

    # Check cache
    if filepath in _cache.get("files", {}):
        cached = _cache["files"][filepath]
        if cached.get("hash") == file_hash:
            return (filepath, cached.get("text", ""))

    # Run OCR
    try:
        result = subprocess.run(
            ["tesseract", filepath, "stdout", "-l", _lang_str],
            capture_output=True,
            text=True,
            timeout=30,
        )
        text = result.stdout.strip()
    except (subprocess.TimeoutExpired, FileNotFoundError, subprocess.SubprocessError):
        text = ""

    return (filepath, text)


def process_file(filepath: str) -> tuple[str, str, str]:
    """Process a single file. Returns (filepath, text, hash)."""
    filepath_obj = Path(filepath)
    file_hash = get_file_hash(filepath_obj)
    _, text = run_ocr(filepath)
    return (filepath, text, file_hash)


@click.command()
@click.option("-d", "--directory", required=True, help="Directory to index")
@click.option("-w", "--workers", default=2, help="Number of parallel workers")
@click.option(
    "--machine_progress",
    is_flag=True,
    default=False,
    help="Print machine-readable progress",
)
def main(directory: str, workers: int, machine_progress: bool) -> None:
    global _lang_str, _cache

    dir_path = Path(directory).expanduser().resolve()
    if not dir_path.exists() or not dir_path.is_dir():
        print(f"Error: {directory} is not a valid directory", file=sys.stderr)
        sys.exit(1)

    # Get tesseract languages
    _lang_str = get_tesseract_languages()

    # Load existing cache
    _cache = load_cache(str(dir_path))

    # Find all image files
    all_files = []
    for f in dir_path.iterdir():
        if f.is_file() and f.suffix.lower() in IMAGE_EXTENSIONS:
            all_files.append(str(f))

    if not all_files:
        if machine_progress:
            print("PROGRESS 0/0")
        else:
            print(f"No images found in {dir_path}")
        return

    # Filter to files that need processing (not in cache or changed)
    files_to_process = []
    for filepath in all_files:
        path = Path(filepath)
        file_hash = get_file_hash(path)
        cached = _cache.get("files", {}).get(filepath, {})
        if cached.get("hash") != file_hash:
            files_to_process.append(filepath)

    if machine_progress:
        print(f"PROGRESS 0/{len(all_files)}")
        sys.stdout.flush()

    # Process files that need OCR
    if files_to_process:
        completed = len(all_files) - len(files_to_process)

        with Pool(processes=workers) as p:
            for filepath, text, file_hash in p.imap(process_file, files_to_process):
                # Update cache
                if "files" not in _cache:
                    _cache["files"] = {}
                _cache["files"][filepath] = {"hash": file_hash, "text": text}

                completed += 1
                if machine_progress:
                    # Escape newlines in OCR text for single-line output
                    escaped_text = text.replace("\\", "\\\\").replace("\n", "\\n")
                    print(f"PROGRESS {completed}/{len(all_files)}")
                    print(f"FILE {filepath}")
                    print(f"OCR {filepath}|{escaped_text}")
                    sys.stdout.flush()

        # Save updated cache
        save_cache(str(dir_path), _cache)
    else:
        # All files already cached, output them
        if machine_progress:
            for i, filepath in enumerate(all_files):
                cached = _cache.get("files", {}).get(filepath, {})
                text = cached.get("text", "")
                escaped_text = text.replace("\\", "\\\\").replace("\n", "\\n")
                print(f"PROGRESS {i + 1}/{len(all_files)}")
                print(f"FILE {filepath}")
                print(f"OCR {filepath}|{escaped_text}")
                sys.stdout.flush()

    if not machine_progress:
        print(f"OCR indexing completed for {len(all_files)} files")


if __name__ == "__main__":
    main()
