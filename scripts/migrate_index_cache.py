#!/usr/bin/env python3
"""
Migration script for hamr index cache v1 -> v2 format.

Changes in v2:
- Frecency fields moved from flat underscore-prefixed to nested object
- Widget data moved from flat fields to nested widget object
- Added explicit version field

Usage:
    python migrate_index_cache.py                    # Migrate default location
    python migrate_index_cache.py path/to/cache.json # Migrate specific file
    python migrate_index_cache.py --dry-run          # Show changes without writing
"""

import argparse
import json
import shutil
import sys
from pathlib import Path
from typing import Any

# Mapping from v1 underscore-prefixed fields to v2 nested frecency fields
FRECENCY_FIELD_MAPPING = {
    "_count": "count",
    "_lastUsed": "lastUsed",
    "_recentSearchTerms": "recentSearchTerms",
    "_hourSlotCounts": "hourSlotCounts",
    "_dayOfWeekCounts": "dayOfWeekCounts",
    "_consecutiveDays": "consecutiveDays",
    "_lastConsecutiveDate": "lastConsecutiveDate",
    "_launchFromEmptyCount": "launchFromEmptyCount",
    "_sessionStartCount": "sessionStartCount",
    "_workspaceCounts": "workspaceCounts",
    "_monitorCounts": "monitorCounts",
    "_launchedAfter": "launchedAfter",
    "_resumeFromIdleCount": "resumeFromIdleCount",
    "_displayCountCounts": "displayCountCounts",
    "_sessionDurationCounts": "sessionDurationCounts",
}

# Default values for frecency fields
FRECENCY_DEFAULTS = {
    "count": 0,
    "lastUsed": 0,
    "recentSearchTerms": [],
    "hourSlotCounts": [0] * 24,
    "dayOfWeekCounts": [0] * 7,
    "consecutiveDays": 0,
    "lastConsecutiveDate": None,
    "launchFromEmptyCount": 0,
    "sessionStartCount": 0,
    "workspaceCounts": {},
    "monitorCounts": {},
    "launchedAfter": {},
    "resumeFromIdleCount": 0,
    "displayCountCounts": {},
    "sessionDurationCounts": [0] * 5,
}

# Widget types that require migration
WIDGET_TYPES = {"slider", "switch"}


def detect_version(cache: dict[str, Any]) -> int:
    """Detect the cache format version."""
    if "version" in cache:
        return cache["version"]
    return 1


def coerce_bool(value: Any) -> bool:
    """Convert various representations to boolean."""
    if isinstance(value, bool):
        return value
    if isinstance(value, str):
        return value.lower() in ("true", "1", "yes", "on")
    return bool(value)


def migrate_item(item: dict[str, Any]) -> dict[str, Any]:
    """Migrate a single item from v1 to v2 format."""
    result = {}

    # Build frecency object from underscore-prefixed fields
    frecency = dict(FRECENCY_DEFAULTS)  # Start with defaults

    for old_field, new_field in FRECENCY_FIELD_MAPPING.items():
        if old_field in item:
            frecency[new_field] = item[old_field]

    # Handle _isPluginEntry separately (goes to top level, not frecency)
    is_plugin_entry = item.get("_isPluginEntry")

    # Determine widget type from type or resultType field
    item_type = item.get("type") or item.get("resultType")

    # Build widget object if this is a widget type
    widget = None
    if item_type in WIDGET_TYPES:
        value = item.get("value")

        if item_type == "slider":
            widget = {
                "type": "slider",
                "value": value if not isinstance(value, dict) else 0.0,
                "min": item.get("min", 0.0),
                "max": item.get("max", 100.0),
                "step": item.get("step", 1.0),
            }
            if "displayValue" in item:
                widget["displayValue"] = item["displayValue"]

        elif item_type == "switch":
            widget = {
                "type": "switch",
                "value": coerce_bool(value),
            }

    # Copy all fields except the ones we're migrating
    fields_to_skip = set(FRECENCY_FIELD_MAPPING.keys()) | {
        "_isPluginEntry",
        "frecency",  # Remove old incomplete frecency object
    }

    # Also skip flat widget fields if we're creating a widget object
    if widget is not None:
        fields_to_skip |= {"value", "min", "max", "step", "displayValue"}

    for key, value in item.items():
        if key not in fields_to_skip:
            result[key] = value

    # Add the new nested structures
    result["frecency"] = frecency

    if widget is not None:
        result["widget"] = widget

    if is_plugin_entry is not None:
        result["isPluginEntry"] = is_plugin_entry

    return result


def migrate_cache(cache: dict[str, Any]) -> dict[str, Any]:
    """Migrate a full cache from v1 to v2 format."""
    version = detect_version(cache)

    if version >= 2:
        # Already v2, return as-is
        return cache

    result = {"version": 2, "indexes": {}}

    indexes = cache.get("indexes", {})
    for index_name, index_data in indexes.items():
        items = index_data.get("items", [])
        migrated_items = [migrate_item(item) for item in items]
        result["indexes"][index_name] = {"items": migrated_items}

    return result


def migrate_file(file_path: str, dry_run: bool = False) -> dict[str, Any]:
    """
    Migrate a cache file from v1 to v2 format.

    Args:
        file_path: Path to the cache file
        dry_run: If True, don't write changes

    Returns:
        Dict with migration results
    """
    path = Path(file_path)

    if not path.exists():
        return {"error": f"File not found: {file_path}", "migrated": False}

    with open(path) as f:
        cache = json.load(f)

    version = detect_version(cache)

    if version >= 2:
        print(f"Cache is already v{version}, no migration needed")
        return {"migrated": False, "version": version}

    migrated = migrate_cache(cache)

    if dry_run:
        print("Dry run - would migrate to:")
        print(json.dumps(migrated, indent=2))
        return {"migrated": False, "dry_run": True, "result": migrated}

    # Create backup
    backup_path = path.with_suffix(".json.v1.bak")
    shutil.copy2(path, backup_path)
    print(f"Created backup: {backup_path}")

    # Write migrated cache
    with open(path, "w") as f:
        json.dump(migrated, f, indent=2)

    print(f"Migrated {file_path} to v2 format")

    # Count items migrated
    item_count = sum(
        len(idx.get("items", [])) for idx in migrated.get("indexes", {}).values()
    )
    print(
        f"Migrated {item_count} items across {len(migrated.get('indexes', {}))} indexes"
    )

    return {"migrated": True, "item_count": item_count}


def get_default_cache_path() -> Path:
    """Get the default cache file path."""
    cache_dir = Path.home() / ".cache" / "hamr"
    return cache_dir / "plugin-indexes.json"


def main() -> int:
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="Migrate hamr index cache from v1 to v2 format"
    )
    parser.add_argument(
        "file",
        nargs="?",
        default=None,
        help="Cache file to migrate (default: ~/.cache/hamr/plugin-indexes.json)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would change without writing",
    )

    args = parser.parse_args()

    file_path = args.file if args.file else str(get_default_cache_path())

    print(f"Migrating: {file_path}")
    print()

    result = migrate_file(file_path, dry_run=args.dry_run)

    if "error" in result:
        print(f"Error: {result['error']}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
