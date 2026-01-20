#!/usr/bin/env python3
"""Tests for the index cache migration script (v1 -> v2 format)."""

import json
import pytest
import tempfile
import shutil
from pathlib import Path

from migrate_index_cache import (
    detect_version,
    migrate_item,
    migrate_cache,
    migrate_file,
    FRECENCY_FIELD_MAPPING,
)


class TestVersionDetection:
    """Tests for version detection logic."""

    def test_detect_v1_no_version_field(self):
        cache = {"indexes": {"apps": {"items": []}}}
        assert detect_version(cache) == 1

    def test_detect_v2_explicit_version(self):
        cache = {"version": 2, "indexes": {"apps": {"items": []}}}
        assert detect_version(cache) == 2

    def test_detect_v1_with_underscore_fields(self):
        cache = {
            "indexes": {
                "apps": {"items": [{"id": "test", "name": "Test", "_count": 5}]}
            }
        }
        assert detect_version(cache) == 1

    def test_detect_v2_with_nested_frecency(self):
        cache = {
            "version": 2,
            "indexes": {
                "apps": {
                    "items": [{"id": "test", "name": "Test", "frecency": {"count": 5}}]
                }
            },
        }
        assert detect_version(cache) == 2


class TestFrecencyMigration:
    """Tests for frecency field migration."""

    def test_migrate_basic_frecency_fields(self):
        v1_item = {
            "id": "firefox",
            "name": "Firefox",
            "_count": 15,
            "_lastUsed": 1737012000000,
            "_recentSearchTerms": ["browser"],
        }
        v2_item = migrate_item(v1_item)

        assert "frecency" in v2_item
        assert v2_item["frecency"]["count"] == 15
        assert v2_item["frecency"]["lastUsed"] == 1737012000000
        assert v2_item["frecency"]["recentSearchTerms"] == ["browser"]
        assert "_count" not in v2_item
        assert "_lastUsed" not in v2_item
        assert "_recentSearchTerms" not in v2_item

    def test_migrate_all_frecency_fields(self):
        v1_item = {
            "id": "test",
            "name": "Test",
            "_count": 10,
            "_lastUsed": 1737012000000,
            "_recentSearchTerms": ["term1", "term2"],
            "_hourSlotCounts": [0] * 24,
            "_dayOfWeekCounts": [1, 2, 3, 4, 5, 0, 0],
            "_consecutiveDays": 5,
            "_lastConsecutiveDate": "2025-01-16",
            "_launchFromEmptyCount": 3,
            "_sessionStartCount": 2,
            "_workspaceCounts": {"workspace1": 5},
            "_monitorCounts": {"monitor1": 3},
            "_launchedAfter": {"other_app": 2},
            "_resumeFromIdleCount": 1,
            "_displayCountCounts": {"2": 10},
            "_sessionDurationCounts": [1, 2, 3, 4, 5],
        }
        v2_item = migrate_item(v1_item)

        frecency = v2_item["frecency"]
        assert frecency["count"] == 10
        assert frecency["lastUsed"] == 1737012000000
        assert frecency["recentSearchTerms"] == ["term1", "term2"]
        assert frecency["hourSlotCounts"] == [0] * 24
        assert frecency["dayOfWeekCounts"] == [1, 2, 3, 4, 5, 0, 0]
        assert frecency["consecutiveDays"] == 5
        assert frecency["lastConsecutiveDate"] == "2025-01-16"
        assert frecency["launchFromEmptyCount"] == 3
        assert frecency["sessionStartCount"] == 2
        assert frecency["workspaceCounts"] == {"workspace1": 5}
        assert frecency["monitorCounts"] == {"monitor1": 3}
        assert frecency["launchedAfter"] == {"other_app": 2}
        assert frecency["resumeFromIdleCount"] == 1
        assert frecency["displayCountCounts"] == {"2": 10}
        assert frecency["sessionDurationCounts"] == [1, 2, 3, 4, 5]

        # Verify all underscore fields removed
        for old_field in FRECENCY_FIELD_MAPPING.keys():
            assert old_field not in v2_item

    def test_migrate_is_plugin_entry(self):
        v1_item = {
            "id": "test",
            "name": "Test",
            "_isPluginEntry": True,
            "_count": 5,
        }
        v2_item = migrate_item(v1_item)

        assert v2_item["isPluginEntry"] is True
        assert "_isPluginEntry" not in v2_item

    def test_removes_old_incomplete_frecency_object(self):
        v1_item = {
            "id": "test",
            "name": "Test",
            "_count": 10,
            "frecency": {"count": 5},  # old incomplete nested object
        }
        v2_item = migrate_item(v1_item)

        # Should use _count (10), not old incomplete frecency.count (5)
        assert v2_item["frecency"]["count"] == 10

    def test_item_without_frecency_fields(self):
        v1_item = {"id": "test", "name": "Test"}
        v2_item = migrate_item(v1_item)

        # Should still have frecency object with defaults
        assert "frecency" in v2_item
        assert v2_item["frecency"]["count"] == 0
        assert v2_item["frecency"]["lastUsed"] == 0


class TestWidgetMigration:
    """Tests for widget field migration."""

    def test_migrate_slider_widget(self):
        v1_item = {
            "id": "volume",
            "name": "Volume",
            "type": "slider",
            "value": 75.0,
            "min": 0.0,
            "max": 100.0,
            "step": 1.0,
        }
        v2_item = migrate_item(v1_item)

        assert "widget" in v2_item
        assert v2_item["widget"]["type"] == "slider"
        assert v2_item["widget"]["value"] == 75.0
        assert v2_item["widget"]["min"] == 0.0
        assert v2_item["widget"]["max"] == 100.0
        assert v2_item["widget"]["step"] == 1.0
        # Flat fields should be removed
        assert "value" not in v2_item
        assert "min" not in v2_item
        assert "max" not in v2_item
        assert "step" not in v2_item

    def test_migrate_slider_with_display_value(self):
        v1_item = {
            "id": "brightness",
            "name": "Brightness",
            "type": "slider",
            "value": 50.0,
            "min": 0.0,
            "max": 100.0,
            "step": 5.0,
            "displayValue": "50%",
        }
        v2_item = migrate_item(v1_item)

        assert v2_item["widget"]["displayValue"] == "50%"
        assert "displayValue" not in v2_item

    def test_migrate_switch_widget(self):
        v1_item = {
            "id": "wifi",
            "name": "WiFi",
            "type": "switch",
            "value": True,
        }
        v2_item = migrate_item(v1_item)

        assert "widget" in v2_item
        assert v2_item["widget"]["type"] == "switch"
        assert v2_item["widget"]["value"] is True
        assert "value" not in v2_item

    def test_migrate_switch_with_false_value(self):
        v1_item = {
            "id": "bluetooth",
            "name": "Bluetooth",
            "type": "switch",
            "value": False,
        }
        v2_item = migrate_item(v1_item)

        assert v2_item["widget"]["type"] == "switch"
        assert v2_item["widget"]["value"] is False

    def test_migrate_result_type_field(self):
        """Test that resultType is also recognized (not just type)."""
        v1_item = {
            "id": "volume",
            "name": "Volume",
            "resultType": "slider",
            "value": 50.0,
        }
        v2_item = migrate_item(v1_item)

        assert v2_item["widget"]["type"] == "slider"
        assert v2_item["widget"]["value"] == 50.0

    def test_preserve_non_widget_items(self):
        v1_item = {"id": "firefox", "name": "Firefox", "_count": 5}
        v2_item = migrate_item(v1_item)

        assert "widget" not in v2_item

    def test_preserve_normal_type_items(self):
        v1_item = {
            "id": "firefox",
            "name": "Firefox",
            "type": "normal",
            "_count": 5,
        }
        v2_item = migrate_item(v1_item)

        assert "widget" not in v2_item

    def test_slider_with_defaults(self):
        """Slider with missing optional fields should use defaults."""
        v1_item = {
            "id": "volume",
            "name": "Volume",
            "type": "slider",
            "value": 50.0,
        }
        v2_item = migrate_item(v1_item)

        assert v2_item["widget"]["value"] == 50.0
        assert v2_item["widget"]["min"] == 0.0
        assert v2_item["widget"]["max"] == 100.0
        assert v2_item["widget"]["step"] == 1.0


class TestPreserveOtherFields:
    """Tests that non-migrated fields are preserved."""

    def test_preserve_basic_fields(self):
        v1_item = {
            "id": "firefox",
            "name": "Firefox",
            "description": "Web browser",
            "icon": "firefox",
            "iconType": "system",
            "_count": 5,
        }
        v2_item = migrate_item(v1_item)

        assert v2_item["id"] == "firefox"
        assert v2_item["name"] == "Firefox"
        assert v2_item["description"] == "Web browser"
        assert v2_item["icon"] == "firefox"
        assert v2_item["iconType"] == "system"

    def test_preserve_actions(self):
        v1_item = {
            "id": "test",
            "name": "Test",
            "actions": [{"id": "open", "name": "Open"}],
            "_count": 1,
        }
        v2_item = migrate_item(v1_item)

        assert v2_item["actions"] == [{"id": "open", "name": "Open"}]

    def test_preserve_badges_and_chips(self):
        v1_item = {
            "id": "test",
            "name": "Test",
            "badges": [{"text": "NEW", "color": "#4caf50"}],
            "chips": [{"text": "Tag", "icon": "label"}],
            "_count": 1,
        }
        v2_item = migrate_item(v1_item)

        assert v2_item["badges"] == [{"text": "NEW", "color": "#4caf50"}]
        assert v2_item["chips"] == [{"text": "Tag", "icon": "label"}]

    def test_preserve_index_fields(self):
        v1_item = {
            "id": "firefox",
            "name": "Firefox",
            "keywords": ["browser", "web"],
            "appId": "firefox",
            "appIdFallback": "org.mozilla.firefox",
            "entryPoint": {"plugin": "apps", "action": "launch"},
            "_count": 5,
        }
        v2_item = migrate_item(v1_item)

        assert v2_item["keywords"] == ["browser", "web"]
        assert v2_item["appId"] == "firefox"
        assert v2_item["appIdFallback"] == "org.mozilla.firefox"
        assert v2_item["entryPoint"] == {"plugin": "apps", "action": "launch"}


class TestFullCacheMigration:
    """Tests for full cache migration."""

    def test_migrate_full_cache(self):
        v1_cache = {
            "indexes": {
                "apps": {
                    "items": [
                        {"id": "firefox", "name": "Firefox", "_count": 5},
                        {
                            "id": "vol",
                            "name": "Volume",
                            "type": "slider",
                            "value": 50.0,
                        },
                    ]
                }
            }
        }
        v2_cache = migrate_cache(v1_cache)

        assert v2_cache["version"] == 2
        items = v2_cache["indexes"]["apps"]["items"]
        assert items[0]["frecency"]["count"] == 5
        assert items[1]["widget"]["type"] == "slider"
        assert items[1]["widget"]["value"] == 50.0

    def test_migrate_multiple_indexes(self):
        v1_cache = {
            "indexes": {
                "apps": {"items": [{"id": "app1", "name": "App 1", "_count": 5}]},
                "files": {"items": [{"id": "file1", "name": "File 1", "_count": 3}]},
            }
        }
        v2_cache = migrate_cache(v1_cache)

        assert v2_cache["version"] == 2
        assert v2_cache["indexes"]["apps"]["items"][0]["frecency"]["count"] == 5
        assert v2_cache["indexes"]["files"]["items"][0]["frecency"]["count"] == 3

    def test_idempotent_migration(self):
        """Migrating v2 cache should return unchanged."""
        v2_cache = {
            "version": 2,
            "indexes": {
                "apps": {
                    "items": [
                        {
                            "id": "test",
                            "name": "Test",
                            "frecency": {"count": 5, "lastUsed": 1000},
                        }
                    ]
                }
            },
        }
        result = migrate_cache(v2_cache)

        assert result["version"] == 2
        assert result["indexes"]["apps"]["items"][0]["frecency"]["count"] == 5

    def test_empty_indexes(self):
        v1_cache = {"indexes": {}}
        v2_cache = migrate_cache(v1_cache)

        assert v2_cache["version"] == 2
        assert v2_cache["indexes"] == {}

    def test_empty_items(self):
        v1_cache = {"indexes": {"apps": {"items": []}}}
        v2_cache = migrate_cache(v1_cache)

        assert v2_cache["version"] == 2
        assert v2_cache["indexes"]["apps"]["items"] == []


class TestFileMigration:
    """Tests for file-based migration."""

    def test_migrate_file_creates_backup(self, tmp_path):
        cache_file = tmp_path / "plugin-indexes.json"
        v1_cache = {
            "indexes": {
                "apps": {"items": [{"id": "test", "name": "Test", "_count": 5}]}
            }
        }
        cache_file.write_text(json.dumps(v1_cache))

        migrate_file(str(cache_file), dry_run=False)

        backup_file = tmp_path / "plugin-indexes.json.v1.bak"
        assert backup_file.exists()
        backup_content = json.loads(backup_file.read_text())
        assert "_count" in backup_content["indexes"]["apps"]["items"][0]

    def test_migrate_file_writes_v2(self, tmp_path):
        cache_file = tmp_path / "plugin-indexes.json"
        v1_cache = {
            "indexes": {
                "apps": {"items": [{"id": "test", "name": "Test", "_count": 5}]}
            }
        }
        cache_file.write_text(json.dumps(v1_cache))

        migrate_file(str(cache_file), dry_run=False)

        migrated = json.loads(cache_file.read_text())
        assert migrated["version"] == 2
        assert migrated["indexes"]["apps"]["items"][0]["frecency"]["count"] == 5

    def test_migrate_file_dry_run(self, tmp_path):
        cache_file = tmp_path / "plugin-indexes.json"
        v1_cache = {
            "indexes": {
                "apps": {"items": [{"id": "test", "name": "Test", "_count": 5}]}
            }
        }
        original_content = json.dumps(v1_cache)
        cache_file.write_text(original_content)

        migrate_file(str(cache_file), dry_run=True)

        # File should be unchanged
        assert cache_file.read_text() == original_content
        # No backup should be created
        backup_file = tmp_path / "plugin-indexes.json.v1.bak"
        assert not backup_file.exists()

    def test_migrate_file_already_v2(self, tmp_path):
        cache_file = tmp_path / "plugin-indexes.json"
        v2_cache = {
            "version": 2,
            "indexes": {
                "apps": {
                    "items": [{"id": "test", "name": "Test", "frecency": {"count": 5}}]
                }
            },
        }
        cache_file.write_text(json.dumps(v2_cache))

        result = migrate_file(str(cache_file), dry_run=False)

        # Should indicate no migration needed
        assert result["migrated"] is False
        # No backup should be created for v2 files
        backup_file = tmp_path / "plugin-indexes.json.v1.bak"
        assert not backup_file.exists()


class TestEdgeCases:
    """Tests for edge cases and error handling."""

    def test_item_with_null_values(self):
        v1_item = {
            "id": "test",
            "name": "Test",
            "description": None,
            "icon": None,
            "_count": 5,
        }
        v2_item = migrate_item(v1_item)

        assert v2_item["description"] is None
        assert v2_item["icon"] is None
        assert v2_item["frecency"]["count"] == 5

    def test_item_with_zero_frecency(self):
        v1_item = {
            "id": "test",
            "name": "Test",
            "_count": 0,
            "_lastUsed": 0,
        }
        v2_item = migrate_item(v1_item)

        assert v2_item["frecency"]["count"] == 0
        assert v2_item["frecency"]["lastUsed"] == 0

    def test_slider_with_nested_value(self):
        """Handle case where value might already be in widget format."""
        v1_item = {
            "id": "volume",
            "name": "Volume",
            "type": "slider",
            "value": {"nested": True},  # Invalid but should not crash
        }
        # Should handle gracefully
        v2_item = migrate_item(v1_item)
        assert "widget" in v2_item

    def test_switch_with_string_value(self):
        """Some plugins send boolean as string."""
        v1_item = {
            "id": "wifi",
            "name": "WiFi",
            "type": "switch",
            "value": "true",
        }
        v2_item = migrate_item(v1_item)

        # Should convert string to bool
        assert v2_item["widget"]["value"] is True

    def test_switch_with_string_false(self):
        v1_item = {
            "id": "wifi",
            "name": "WiFi",
            "type": "switch",
            "value": "false",
        }
        v2_item = migrate_item(v1_item)

        assert v2_item["widget"]["value"] is False
