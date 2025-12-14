#!/bin/bash
#
# Tests for screenshot plugin
# Run: ./test.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export HAMR_TEST_MODE=1
source "$SCRIPT_DIR/../test-helpers.sh"

# ============================================================================
# Config
# ============================================================================

TEST_NAME="Screenshot Plugin Tests"
HANDLER="$SCRIPT_DIR/handler.py"

# Directories (same as handler.py)
PICTURES_DIR="$HOME/Pictures"
SCREENSHOTS_DIR="$PICTURES_DIR/Screenshots"
CACHE_DIR="$HOME/.cache/hamr/screenshot-ocr"
CACHE_FILE="$CACHE_DIR/ocr_index.json"

# ============================================================================
# Setup / Teardown
# ============================================================================

setup() {
    # Backup existing cache
    if [[ -f "$CACHE_FILE" ]]; then
        cp "$CACHE_FILE" "$CACHE_FILE.backup"
    fi
}

teardown() {
    # Restore original cache
    if [[ -f "$CACHE_FILE.backup" ]]; then
        mv "$CACHE_FILE.backup" "$CACHE_FILE"
    else
        rm -f "$CACHE_FILE"
    fi
}

# ============================================================================
# Tests
# ============================================================================

test_initial_returns_image_browser() {
    local result=$(hamr_test initial)
    
    assert_type "$result" "imageBrowser"
}

test_initial_image_browser_has_required_fields() {
    local result=$(hamr_test initial)
    
    # Check imageBrowser object exists
    local browser=$(json_get "$result" '.imageBrowser')
    assert_not_contains "$(echo "$browser" | jq 'type')" "null" "imageBrowser object should exist"
    
    # Check required fields
    local title=$(json_get "$result" '.imageBrowser.title')
    assert_eq "$title" "Screenshots" "imageBrowser should have title 'Screenshots'"
    
    local ocr=$(json_get "$result" '.imageBrowser.enableOcr')
    assert_eq "$ocr" "true" "imageBrowser should have enableOcr enabled"
}

test_initial_image_browser_points_to_screenshots_dir() {
    local result=$(hamr_test initial)
    
    local dir=$(json_get "$result" '.imageBrowser.directory')
    # Should point to Screenshots dir if it exists, otherwise Pictures
    if [[ -d "$SCREENSHOTS_DIR" ]]; then
        assert_contains "$dir" "Screenshots" "Should point to Screenshots directory"
    else
        assert_contains "$dir" "Pictures" "Should point to Pictures directory when Screenshots doesn't exist"
    fi
}

test_initial_has_four_actions() {
    local result=$(hamr_test initial)
    
    local count=$(json_get "$result" '.imageBrowser.actions | length')
    assert_eq "$count" "4" "imageBrowser should have 4 actions"
}

test_initial_has_open_action() {
    local result=$(hamr_test initial)
    
    local id=$(json_get "$result" '.imageBrowser.actions[] | select(.id == "open") | .id')
    assert_eq "$id" "open" "Should have 'open' action"
    
    local name=$(json_get "$result" '.imageBrowser.actions[] | select(.id == "open") | .name')
    assert_eq "$name" "Open" "Open action should have correct name"
}

test_initial_has_copy_action() {
    local result=$(hamr_test initial)
    
    local id=$(json_get "$result" '.imageBrowser.actions[] | select(.id == "copy") | .id')
    assert_eq "$id" "copy" "Should have 'copy' action"
}

test_initial_has_ocr_action() {
    local result=$(hamr_test initial)
    
    local id=$(json_get "$result" '.imageBrowser.actions[] | select(.id == "ocr") | .id')
    assert_eq "$id" "ocr" "Should have 'ocr' action"
    
    local name=$(json_get "$result" '.imageBrowser.actions[] | select(.id == "ocr") | .name')
    assert_contains "$name" "OCR" "OCR action name should mention OCR"
}

test_initial_has_delete_action() {
    local result=$(hamr_test initial)
    
    local id=$(json_get "$result" '.imageBrowser.actions[] | select(.id == "delete") | .id')
    assert_eq "$id" "delete" "Should have 'delete' action"
}

test_search_returns_image_browser() {
    local result=$(hamr_test search --query "test")
    
    assert_type "$result" "imageBrowser"
}

test_open_action_returns_execute() {
    # This test uses a mock path since we don't want to actually interact with images
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.png", "action": "open"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    assert_type "$result" "execute"
}

test_open_action_has_correct_command() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/screenshot.png", "action": "open"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    # Should use xdg-open
    assert_contains "$result" "xdg-open"
    assert_contains "$result" "/tmp/screenshot.png"
}

test_open_action_closes_launcher() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.png", "action": "open"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    assert_closes "$result"
}

test_open_action_has_thumbnail() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/screenshot.png", "action": "open"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    local thumbnail=$(json_get "$result" '.execute.thumbnail')
    assert_eq "$thumbnail" "/tmp/screenshot.png" "Should have thumbnail set to image path"
}

test_open_action_has_name() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/screenshot.png", "action": "open"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    local name=$(json_get "$result" '.execute.name')
    assert_contains "$name" "screenshot.png" "Should have action name with filename"
}

test_open_action_has_icon() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.png", "action": "open"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    local icon=$(json_get "$result" '.execute.icon')
    assert_eq "$icon" "screenshot_monitor" "Should have screenshot_monitor icon"
}

test_copy_action_returns_execute() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.png", "action": "copy"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    assert_type "$result" "execute"
}

test_copy_action_uses_wl_copy() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.png", "action": "copy"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    assert_contains "$result" "wl-copy"
}

test_copy_action_closes_launcher() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.png", "action": "copy"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    assert_closes "$result"
}

test_copy_action_has_notify() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.png", "action": "copy"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    local notify=$(json_get "$result" '.execute.notify')
    assert_contains "$notify" "Copied"
}

test_copy_action_has_thumbnail() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/screenshot.png", "action": "copy"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    local thumbnail=$(json_get "$result" '.execute.thumbnail')
    assert_eq "$thumbnail" "/tmp/screenshot.png" "Should have thumbnail set to image path"
}

test_delete_action_returns_execute() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.png", "action": "delete"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    assert_type "$result" "execute"
}

test_delete_action_uses_gio_trash() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.png", "action": "delete"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    assert_contains "$result" "gio"
    assert_contains "$result" "trash"
}

test_delete_action_stays_open() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.png", "action": "delete"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    assert_stays_open "$result"
}

test_delete_action_has_notify() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.png", "action": "delete"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    local notify=$(json_get "$result" '.execute.notify')
    assert_contains "$notify" "Deleted"
}

test_ocr_action_returns_execute() {
    # Create a temporary test image file so handler can stat it
    local tmpimg=$(mktemp --suffix=.png)
    trap "rm -f '$tmpimg'" RETURN
    
    # Create a minimal PNG file (1x1 transparent PNG)
    printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89\x00\x00\x00\nIDATx\x9cc\x00\x01\x00\x00\x05\x00\x01\r\n-\xb4\x00\x00\x00\x00IEND\xaeB`\x82' > "$tmpimg"
    
    local input="{\"step\": \"action\", \"selected\": {\"id\": \"imageBrowser\", \"path\": \"$tmpimg\", \"action\": \"ocr\"}, \"session\": \"\"}"
    local result=$(echo "$input" | "$HANDLER")
    
    # Will either return execute or error depending on tesseract availability
    local type=$(json_get "$result" '.type')
    assert_eq "$type" "execute" "Should return execute response for OCR action"
}

test_ocr_action_has_notify() {
    # Create a temporary test image file so handler can stat it
    local tmpimg=$(mktemp --suffix=.png)
    trap "rm -f '$tmpimg'" RETURN
    
    # Create a minimal PNG file (1x1 transparent PNG)
    printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89\x00\x00\x00\nIDATx\x9cc\x00\x01\x00\x00\x05\x00\x01\r\n-\xb4\x00\x00\x00\x00IEND\xaeB`\x82' > "$tmpimg"
    
    local input="{\"step\": \"action\", \"selected\": {\"id\": \"imageBrowser\", \"path\": \"$tmpimg\", \"action\": \"ocr\"}, \"session\": \"\"}"
    local result=$(echo "$input" | "$HANDLER")
    
    # Expect either "No text found" or actual OCR result notification
    local notify=$(json_get "$result" '.execute.notify')
    assert_not_contains "$(echo "$notify" | jq 'type')" "null" "Should have notify field set"
}

test_missing_file_path_returns_error() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "action": "open"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    assert_type "$result" "error"
    assert_contains "$result" "No file selected"
}

test_empty_file_path_returns_error() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "", "action": "open"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    assert_type "$result" "error"
}

test_unknown_action_defaults_to_open() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.png", "action": "unknown_action"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    assert_type "$result" "execute"
    assert_contains "$result" "xdg-open"
}

test_all_initial_responses_valid() {
    assert_ok hamr_test initial
    assert_ok hamr_test search --query "test"
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_initial_returns_image_browser \
    test_initial_image_browser_has_required_fields \
    test_initial_image_browser_points_to_screenshots_dir \
    test_initial_has_four_actions \
    test_initial_has_open_action \
    test_initial_has_copy_action \
    test_initial_has_ocr_action \
    test_initial_has_delete_action \
    test_search_returns_image_browser \
    test_open_action_returns_execute \
    test_open_action_has_correct_command \
    test_open_action_closes_launcher \
    test_open_action_has_thumbnail \
    test_open_action_has_name \
    test_open_action_has_icon \
    test_copy_action_returns_execute \
    test_copy_action_uses_wl_copy \
    test_copy_action_closes_launcher \
    test_copy_action_has_notify \
    test_copy_action_has_thumbnail \
    test_delete_action_returns_execute \
    test_delete_action_uses_gio_trash \
    test_delete_action_stays_open \
    test_delete_action_has_notify \
    test_ocr_action_returns_execute \
    test_ocr_action_has_notify \
    test_missing_file_path_returns_error \
    test_empty_file_path_returns_error \
    test_unknown_action_defaults_to_open \
    test_all_initial_responses_valid
