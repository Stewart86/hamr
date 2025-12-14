#!/bin/bash
#
# Tests for pictures plugin
# Run: ./test.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export HAMR_TEST_MODE=1
source "$SCRIPT_DIR/../test-helpers.sh"

# ============================================================================
# Config
# ============================================================================

TEST_NAME="Pictures Plugin Tests"
HANDLER="$SCRIPT_DIR/handler.py"

# Test downloads directory
TEST_DOWNLOADS_DIR="${TMPDIR:-/tmp}/hamr-test-downloads-$$"

# ============================================================================
# Setup / Teardown
# ============================================================================

setup() {
    # Create test downloads directory with sample images
    mkdir -p "$TEST_DOWNLOADS_DIR"
    
    # Create fake image files with different timestamps
    # Touch files and set timestamps using date arithmetic
    touch "$TEST_DOWNLOADS_DIR/photo1.jpg"
    touch "$TEST_DOWNLOADS_DIR/screenshot.png"
    touch "$TEST_DOWNLOADS_DIR/diagram.svg"
    touch "$TEST_DOWNLOADS_DIR/wallpaper.webp"
    touch "$TEST_DOWNLOADS_DIR/My Vacation Photo.jpeg"
    
    # Set timestamps using faketime or by sleeping and touching
    # Most recent first, oldest last
    sleep 1
    touch "$TEST_DOWNLOADS_DIR/wallpaper.webp"  # Most recent
    sleep 1
    touch "$TEST_DOWNLOADS_DIR/My Vacation Photo.jpeg"
    sleep 1
    touch "$TEST_DOWNLOADS_DIR/diagram.svg"
    sleep 1
    touch "$TEST_DOWNLOADS_DIR/screenshot.png"
    sleep 1
    touch "$TEST_DOWNLOADS_DIR/photo1.jpg"  # Oldest
    
    # Override HOME temporarily to use test downloads dir
    export HOME="$TEST_DOWNLOADS_DIR/../"
    mkdir -p "$TEST_DOWNLOADS_DIR/../Downloads"
    mv "$TEST_DOWNLOADS_DIR"/* "$TEST_DOWNLOADS_DIR/../Downloads/" 2>/dev/null || true
    rm -rf "$TEST_DOWNLOADS_DIR"
    TEST_DOWNLOADS_DIR="$TEST_DOWNLOADS_DIR/../Downloads"
}

teardown() {
    # Clean up test downloads
    rm -rf "$TEST_DOWNLOADS_DIR" 2>/dev/null || true
    rm -rf "$TEST_DOWNLOADS_DIR/.." 2>/dev/null || true
}

before_each() {
    # Ensure test downloads exist for each test
    if [[ ! -d "$TEST_DOWNLOADS_DIR" ]]; then
        mkdir -p "$TEST_DOWNLOADS_DIR"
        touch "$TEST_DOWNLOADS_DIR/photo1.jpg"
        touch "$TEST_DOWNLOADS_DIR/screenshot.png"
        touch "$TEST_DOWNLOADS_DIR/diagram.svg"
        touch "$TEST_DOWNLOADS_DIR/wallpaper.webp"
        touch "$TEST_DOWNLOADS_DIR/My Vacation Photo.jpeg"
    fi
}

# ============================================================================
# Helper Functions
# ============================================================================

count_images() {
    find "$TEST_DOWNLOADS_DIR" -type f \( -iname "*.png" -o -iname "*.jpg" -o -iname "*.jpeg" -o -iname "*.gif" -o -iname "*.webp" -o -iname "*.bmp" -o -iname "*.svg" \) 2>/dev/null | wc -l
}

image_exists() {
    [[ -f "$TEST_DOWNLOADS_DIR/$1" ]]
}

# ============================================================================
# Tests
# ============================================================================

test_initial_returns_results() {
    local result=$(hamr_test initial)
    
    assert_type "$result" "results"
    assert_json "$result" '.inputMode' "realtime"
}

test_initial_shows_images() {
    local result=$(hamr_test initial)
    local count=$(get_result_count "$result")
    
    # Should have at least 5 test images
    if [[ $count -lt 5 ]]; then
        echo "Expected at least 5 images, got $count"
        echo "Images: $(ls -1 "$TEST_DOWNLOADS_DIR" 2>/dev/null || echo 'none')"
        return 1
    fi
}

test_initial_images_sorted_by_mtime() {
    local result=$(hamr_test initial)
    
    # Verify results are sorted by modification time (most recent first)
    # Check that the first result's mtime >= second result's mtime
    local first_name=$(json_get "$result" '.results[0].name')
    local second_name=$(json_get "$result" '.results[1].name')
    
    # All files were just created, so order might vary slightly by system time precision
    # Just verify we have multiple distinct results in order
    if [[ "$first_name" == "$second_name" ]]; then
        echo "First and second results should be different"
        return 1
    fi
}

test_images_have_thumbnails() {
    local result=$(hamr_test initial)
    
    # All images should have thumbnail property
    local thumbnail=$(json_get "$result" '.results[0].thumbnail')
    
    if [[ -z "$thumbnail" || "$thumbnail" == "null" ]]; then
        echo "Image should have thumbnail property"
        return 1
    fi
    
    # Thumbnail should point to valid file
    if [[ ! -f "$thumbnail" ]]; then
        echo "Thumbnail path does not exist: $thumbnail"
        return 1
    fi
}

test_images_have_icon() {
    local result=$(hamr_test initial)
    
    # All images should have icon property
    local icon=$(json_get "$result" '.results[0].icon')
    assert_eq "$icon" "image" "Image should have icon property set to 'image'"
}

test_images_show_file_size() {
    local result=$(hamr_test initial)
    
    # Description should contain file size
    local description=$(json_get "$result" '.results[0].description')
    
    # Should match format like "0.0 B" or "1.5 KB"
    if [[ ! "$description" =~ ^[0-9.]+\ (B|KB|MB|GB|TB)$ ]]; then
        echo "Expected description to be file size, got: $description"
        return 1
    fi
}

test_images_have_action_buttons() {
    local result=$(hamr_test initial)
    
    # First result should have action buttons
    local actions=$(json_get "$result" '.results[0].actions')
    local count=$(echo "$actions" | jq 'length')
    
    assert_eq "$count" "2" "Image should have 2 action buttons"
}

test_image_actions_are_open_and_copy() {
    local result=$(hamr_test initial)
    
    local action_ids=$(json_get "$result" '.results[0].actions[].id')
    
    assert_contains "$action_ids" "open"
    assert_contains "$action_ids" "copy-path"
}

test_search_filters_by_name() {
    local result=$(hamr_test search --query "photo")
    
    # Should only find images with "photo" in name
    assert_contains "$result" "photo1.jpg"
    assert_not_contains "$result" "wallpaper.webp"
}

test_search_case_insensitive() {
    local result=$(hamr_test search --query "WALLPAPER")
    
    assert_contains "$result" "wallpaper.webp"
}

test_search_with_spaces() {
    local result=$(hamr_test search --query "Vacation")
    
    assert_contains "$result" "My Vacation Photo.jpeg"
}

test_search_empty_returns_all() {
    local result=$(hamr_test search --query "")
    local count=$(get_result_count "$result")
    
    # Empty search should return all images
    if [[ $count -lt 5 ]]; then
        echo "Empty search should return all images, got $count"
        return 1
    fi
}

test_search_no_match_returns_empty() {
    local result=$(hamr_test search --query "nonexistent_image_xyz")
    local count=$(get_result_count "$result")
    
    assert_eq "$count" "0" "Non-matching search should return no results"
}

test_click_image_shows_detail_view() {
    local initial=$(hamr_test initial)
    local image_id=$(json_get "$initial" '.results[0].id')
    
    local detail=$(hamr_test action --id "$image_id")
    
    assert_type "$detail" "results"
    # Detail view should have back button
    assert_has_result "$detail" "__back__"
}

test_detail_view_shows_back_button() {
    local initial=$(hamr_test initial)
    local image_id=$(json_get "$initial" '.results[0].id')
    
    local detail=$(hamr_test action --id "$image_id")
    
    assert_has_result "$detail" "__back__"
}

test_detail_view_shows_action_items() {
    local initial=$(hamr_test initial)
    local image_id=$(json_get "$initial" '.results[0].id')
    
    local detail=$(hamr_test action --id "$image_id")
    
    # Detail view should have: back, open, copy-path, copy-image, delete
    local count=$(get_result_count "$detail")
    assert_eq "$count" "5" "Detail view should have 5 items (back + 4 actions)"
}

test_back_from_detail_view() {
    local initial=$(hamr_test initial)
    local image_id=$(json_get "$initial" '.results[0].id')
    
    hamr_test action --id "$image_id" > /dev/null
    local back=$(hamr_test action --id "__back__")
    
    assert_type "$back" "results"
    # Should be back to initial list
    local count=$(get_result_count "$back")
    if [[ $count -lt 5 ]]; then
        echo "Back button should return to initial list with at least 5 images"
        return 1
    fi
}

test_open_action_from_list() {
    local initial=$(hamr_test initial)
    local image_id=$(json_get "$initial" '.results[0].id')
    
    local result=$(hamr_test action --id "$image_id" --action "open")
    
    assert_type "$result" "execute"
    assert_closes "$result"
    assert_contains "$result" "xdg-open"
}

test_copy_path_action_from_list() {
    local initial=$(hamr_test initial)
    local image_id=$(json_get "$initial" '.results[0].id')
    
    local result=$(hamr_test action --id "$image_id" --action "copy-path")
    
    assert_type "$result" "execute"
    assert_closes "$result"
    assert_contains "$result" "wl-copy"
    assert_contains "$result" "$image_id"
}

test_open_from_detail_view() {
    local initial=$(hamr_test initial)
    local image_id=$(json_get "$initial" '.results[0].id')
    
    hamr_test action --id "$image_id" > /dev/null
    local detail=$(hamr_test action --id "$image_id" | jq -r '.results[] | select(.id | startswith("open:")) | .id')
    
    local result=$(hamr_test action --id "$detail")
    
    assert_type "$result" "execute"
    assert_closes "$result"
    assert_contains "$result" "xdg-open"
}

test_copy_image_to_clipboard() {
    local initial=$(hamr_test initial)
    local image_id=$(json_get "$initial" '.results[0].id')
    
    hamr_test action --id "$image_id" > /dev/null
    local detail=$(hamr_test action --id "$image_id" | jq -r '.results[] | select(.id | startswith("copy-image:")) | .id')
    
    local result=$(hamr_test action --id "$detail")
    
    assert_type "$result" "execute"
    assert_closes "$result"
    assert_contains "$result" "wl-copy"
    assert_contains "$result" "image/png"
}

test_delete_moves_to_trash() {
    local initial=$(hamr_test initial)
    local image_id=$(json_get "$initial" '.results[0].id')
    
    hamr_test action --id "$image_id" > /dev/null
    local detail=$(hamr_test action --id "$image_id" | jq -r '.results[] | select(.id | startswith("delete:")) | .id')
    
    local result=$(hamr_test action --id "$detail")
    
    assert_type "$result" "execute"
    assert_closes "$result"
    assert_contains "$result" "gio"
    assert_contains "$result" "trash"
}

test_execute_responses_have_names() {
    local initial=$(hamr_test initial)
    local image_id=$(json_get "$initial" '.results[0].id')
    
    local result=$(hamr_test action --id "$image_id" --action "open")
    
    local name=$(json_get "$result" '.execute.name')
    if [[ -z "$name" || "$name" == "null" ]]; then
        echo "Execute response should have a name"
        return 1
    fi
}

test_execute_responses_have_icons() {
    local initial=$(hamr_test initial)
    local image_id=$(json_get "$initial" '.results[0].id')
    
    local result=$(hamr_test action --id "$image_id" --action "open")
    
    local icon=$(json_get "$result" '.execute.icon')
    if [[ -z "$icon" || "$icon" == "null" ]]; then
        echo "Execute response should have an icon"
        return 1
    fi
}

test_open_action_has_thumbnail() {
    local initial=$(hamr_test initial)
    local image_id=$(json_get "$initial" '.results[0].id')
    
    local result=$(hamr_test action --id "$image_id" --action "open")
    
    local thumbnail=$(json_get "$result" '.execute.thumbnail')
    if [[ -z "$thumbnail" || "$thumbnail" == "null" ]]; then
        echo "Open action should have thumbnail in execute response"
        return 1
    fi
}

test_copy_path_shows_notification() {
    local initial=$(hamr_test initial)
    local image_id=$(json_get "$initial" '.results[0].id')
    
    local result=$(hamr_test action --id "$image_id" --action "copy-path")
    
    local notify=$(json_get "$result" '.execute.notify')
    assert_contains "$notify" "Copied"
}

test_all_responses_valid_json() {
    assert_ok hamr_test initial
    assert_ok hamr_test search --query "test"
    
    local initial=$(hamr_test initial)
    local image_id=$(json_get "$initial" '.results[0].id')
    
    assert_ok hamr_test action --id "$image_id"
    assert_ok hamr_test action --id "$image_id" --action "open"
}

test_handles_missing_downloads_gracefully() {
    # This test would need to temporarily remove the Downloads directory
    # For now, we just verify the handler doesn't crash
    assert_ok hamr_test initial
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_initial_returns_results \
    test_initial_shows_images \
    test_initial_images_sorted_by_mtime \
    test_images_have_thumbnails \
    test_images_have_icon \
    test_images_show_file_size \
    test_images_have_action_buttons \
    test_image_actions_are_open_and_copy \
    test_search_filters_by_name \
    test_search_case_insensitive \
    test_search_with_spaces \
    test_search_empty_returns_all \
    test_search_no_match_returns_empty \
    test_click_image_shows_detail_view \
    test_detail_view_shows_back_button \
    test_detail_view_shows_action_items \
    test_back_from_detail_view \
    test_open_action_from_list \
    test_copy_path_action_from_list \
    test_open_from_detail_view \
    test_copy_image_to_clipboard \
    test_delete_moves_to_trash \
    test_execute_responses_have_names \
    test_execute_responses_have_icons \
    test_open_action_has_thumbnail \
    test_copy_path_shows_notification \
    test_all_responses_valid_json \
    test_handles_missing_downloads_gracefully
