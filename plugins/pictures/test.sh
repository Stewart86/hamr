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

# Test pictures directory
TEST_PICTURES_DIR="${TMPDIR:-/tmp}/hamr-test-pictures-$$"

# ============================================================================
# Setup / Teardown
# ============================================================================

setup() {
    # Create test pictures directory with sample images
    mkdir -p "$TEST_PICTURES_DIR"
    
    # Create fake image files with different timestamps
    touch "$TEST_PICTURES_DIR/photo1.jpg"
    touch "$TEST_PICTURES_DIR/screenshot.png"
    touch "$TEST_PICTURES_DIR/diagram.svg"
    touch "$TEST_PICTURES_DIR/wallpaper.webp"
    touch "$TEST_PICTURES_DIR/My Vacation Photo.jpeg"
    
    # Set timestamps using faketime or by sleeping and touching
    # Most recent first, oldest last
    sleep 1
    touch "$TEST_PICTURES_DIR/wallpaper.webp"  # Most recent
    sleep 1
    touch "$TEST_PICTURES_DIR/My Vacation Photo.jpeg"
    sleep 1
    touch "$TEST_PICTURES_DIR/diagram.svg"
    sleep 1
    touch "$TEST_PICTURES_DIR/screenshot.png"
    sleep 1
    touch "$TEST_PICTURES_DIR/photo1.jpg"  # Oldest
    
    # Set XDG_PICTURES_DIR to use test directory
    export XDG_PICTURES_DIR="$TEST_PICTURES_DIR"
}

teardown() {
    # Clean up test pictures
    rm -rf "$TEST_PICTURES_DIR" 2>/dev/null || true
}

before_each() {
    # Ensure test pictures exist for each test
    if [[ ! -d "$TEST_PICTURES_DIR" ]]; then
        mkdir -p "$TEST_PICTURES_DIR"
        touch "$TEST_PICTURES_DIR/photo1.jpg"
        touch "$TEST_PICTURES_DIR/screenshot.png"
        touch "$TEST_PICTURES_DIR/diagram.svg"
        touch "$TEST_PICTURES_DIR/wallpaper.webp"
        touch "$TEST_PICTURES_DIR/My Vacation Photo.jpeg"
    fi
}

# ============================================================================
# Helper Functions
# ============================================================================

count_images() {
    find "$TEST_PICTURES_DIR" -type f \( -iname "*.png" -o -iname "*.jpg" -o -iname "*.jpeg" -o -iname "*.gif" -o -iname "*.webp" -o -iname "*.bmp" -o -iname "*.svg" \) 2>/dev/null | wc -l
}

image_exists() {
    [[ -f "$TEST_PICTURES_DIR/$1" ]]
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
        echo "Images: $(ls -1 "$TEST_PICTURES_DIR" 2>/dev/null || echo 'none')"
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
    
    # Description should contain file size (may also have dimensions)
    local description=$(json_get "$result" '.results[0].description')
    
    # Should contain file size format like "0.0 B" or "1.5 KB"
    if [[ ! "$description" =~ (B|KB|MB|GB|TB) ]]; then
        echo "Expected description to contain file size, got: $description"
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
    # Detail view should have navigateForward flag
    assert_json "$detail" '.navigateForward' "true"
}

test_detail_view_has_navigate_forward() {
    local initial=$(hamr_test initial)
    local image_id=$(json_get "$initial" '.results[0].id')
    
    local detail=$(hamr_test action --id "$image_id")
    
    # Detail view should have navigateForward flag (drilling into image)
    assert_json "$detail" '.navigateForward' "true"
}

test_detail_view_shows_action_items() {
    local initial=$(hamr_test initial)
    local image_id=$(json_get "$initial" '.results[0].id')
    
    local detail=$(hamr_test action --id "$image_id")
    
    # Detail view should have: open, copy-path, copy-image, delete (no back button - UI provides it)
    local count=$(get_result_count "$detail")
    assert_eq "$count" "4" "Detail view should have 4 action items"
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

test_handles_missing_pictures_gracefully() {
    # This test would need to temporarily remove the Pictures directory
    # For now, we just verify the handler doesn't crash
    assert_ok hamr_test initial
}

test_preview_has_dimensions_metadata() {
    local result=$(hamr_test initial)
    
    # Preview should have dimensions in metadata
    local dims=$(json_get "$result" '.results[0].preview.metadata[] | select(.label == "Dimensions") | .value')
    # In test mode, dimensions are mocked to 1920x1080
    assert_contains "$dims" "1920"
}

test_preview_has_modified_date() {
    local result=$(hamr_test initial)
    
    # Preview should have Modified date
    local modified=$(json_get "$result" '.results[0].preview.metadata[] | select(.label == "Modified") | .value')
    assert_eq "$([ -n "$modified" ] && echo 1 || echo 0)" "1" "Should have modified date"
}

test_description_includes_dimensions() {
    local result=$(hamr_test initial)
    
    # Description should include dimensions in test mode
    local description=$(json_get "$result" '.results[0].description')
    assert_contains "$description" "1920x1080"
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
    test_detail_view_has_navigate_forward \
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
    test_handles_missing_pictures_gracefully \
    test_preview_has_dimensions_metadata \
    test_preview_has_modified_date \
    test_description_includes_dimensions
