#!/bin/bash
#
# Tests for wallpaper plugin
# Run: ./test.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export HAMR_TEST_MODE=1
source "$SCRIPT_DIR/../test-helpers.sh"

# ============================================================================
# Config
# ============================================================================

TEST_NAME="Wallpaper Plugin Tests"
HANDLER="$SCRIPT_DIR/handler.py"

# Directories (same as handler.py)
PICTURES_DIR="$HOME/Pictures"
WALLPAPERS_DIR="$PICTURES_DIR/Wallpapers"

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
    assert_eq "$title" "Select Wallpaper" "imageBrowser should have title 'Select Wallpaper'"
}

test_initial_image_browser_points_to_wallpapers_dir() {
    local result=$(hamr_test initial)
    
    local dir=$(json_get "$result" '.imageBrowser.directory')
    # Should point to Wallpapers dir if it exists, otherwise Pictures
    if [[ -d "$WALLPAPERS_DIR" ]]; then
        assert_contains "$dir" "Wallpapers" "Should point to Wallpapers directory"
    else
        assert_contains "$dir" "Pictures" "Should point to Pictures directory when Wallpapers doesn't exist"
    fi
}

test_initial_has_two_actions() {
    local result=$(hamr_test initial)
    
    local count=$(json_get "$result" '.imageBrowser.actions | length')
    assert_eq "$count" "2" "imageBrowser should have 2 actions"
}

test_initial_has_set_dark_action() {
    local result=$(hamr_test initial)
    
    local id=$(json_get "$result" '.imageBrowser.actions[] | select(.id == "set_dark") | .id')
    assert_eq "$id" "set_dark" "Should have 'set_dark' action"
    
    local name=$(json_get "$result" '.imageBrowser.actions[] | select(.id == "set_dark") | .name')
    assert_eq "$name" "Set (Dark Mode)" "Dark mode action should have correct name"
}

test_initial_has_set_light_action() {
    local result=$(hamr_test initial)
    
    local id=$(json_get "$result" '.imageBrowser.actions[] | select(.id == "set_light") | .id')
    assert_eq "$id" "set_light" "Should have 'set_light' action"
    
    local name=$(json_get "$result" '.imageBrowser.actions[] | select(.id == "set_light") | .name')
    assert_eq "$name" "Set (Light Mode)" "Light mode action should have correct name"
}

test_initial_actions_have_icons() {
    local result=$(hamr_test initial)
    
    local dark_icon=$(json_get "$result" '.imageBrowser.actions[] | select(.id == "set_dark") | .icon')
    assert_eq "$dark_icon" "dark_mode" "Dark action should have dark_mode icon"
    
    local light_icon=$(json_get "$result" '.imageBrowser.actions[] | select(.id == "set_light") | .icon')
    assert_eq "$light_icon" "light_mode" "Light action should have light_mode icon"
}

test_search_returns_image_browser() {
    local result=$(hamr_test search --query "test")
    
    assert_type "$result" "imageBrowser"
}

test_search_returns_same_structure_as_initial() {
    local initial=$(hamr_test initial)
    local search=$(hamr_test search --query "anything")
    
    # Both should have same actions
    local initial_actions=$(json_get "$initial" '.imageBrowser.actions | length')
    local search_actions=$(json_get "$search" '.imageBrowser.actions | length')
    assert_eq "$initial_actions" "$search_actions" "Search should return same action count as initial"
}

test_set_dark_action_returns_execute() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.jpg", "action": "set_dark"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    assert_type "$result" "execute"
}

test_set_dark_action_has_correct_command() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/wallpaper.jpg", "action": "set_dark"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    # Should pass dark mode to switchwall script
    assert_contains "$result" "--image"
    assert_contains "$result" "/tmp/wallpaper.jpg"
    assert_contains "$result" "--mode"
    assert_contains "$result" "dark"
}

test_set_dark_action_closes_launcher() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.jpg", "action": "set_dark"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    assert_closes "$result"
}

test_set_dark_action_has_name() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/wallpaper.jpg", "action": "set_dark"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    local name=$(json_get "$result" '.execute.name')
    assert_contains "$name" "wallpaper.jpg" "Should have action name with filename"
    assert_contains "$name" "Set wallpaper" "Should mention setting wallpaper"
}

test_set_dark_action_has_icon() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.jpg", "action": "set_dark"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    local icon=$(json_get "$result" '.execute.icon')
    assert_eq "$icon" "wallpaper" "Should have wallpaper icon"
}

test_set_dark_action_has_thumbnail() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/wallpaper.jpg", "action": "set_dark"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    local thumbnail=$(json_get "$result" '.execute.thumbnail')
    assert_eq "$thumbnail" "/tmp/wallpaper.jpg" "Should have thumbnail set to image path"
}

test_set_light_action_returns_execute() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.jpg", "action": "set_light"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    assert_type "$result" "execute"
}

test_set_light_action_has_correct_command() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/wallpaper.jpg", "action": "set_light"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    # Should pass light mode to switchwall script
    assert_contains "$result" "--mode"
    assert_contains "$result" "light"
}

test_set_light_action_closes_launcher() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.jpg", "action": "set_light"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    assert_closes "$result"
}

test_set_light_action_has_name() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/wallpaper.jpg", "action": "set_light"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    local name=$(json_get "$result" '.execute.name')
    assert_contains "$name" "wallpaper.jpg" "Should have action name with filename"
}

test_set_light_action_has_icon() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.jpg", "action": "set_light"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    local icon=$(json_get "$result" '.execute.icon')
    assert_eq "$icon" "wallpaper" "Should have wallpaper icon"
}

test_set_light_action_has_thumbnail() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/wallpaper.jpg", "action": "set_light"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    local thumbnail=$(json_get "$result" '.execute.thumbnail')
    assert_eq "$thumbnail" "/tmp/wallpaper.jpg" "Should have thumbnail set to image path"
}

test_dark_and_light_use_different_modes() {
    local dark=$(echo '{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.jpg", "action": "set_dark"}, "session": ""}' | "$HANDLER")
    local light=$(echo '{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.jpg", "action": "set_light"}, "session": ""}' | "$HANDLER")
    
    # Dark should have dark mode
    assert_contains "$dark" "dark"
    assert_not_contains "$dark" '"light"'
    
    # Light should have light mode
    assert_contains "$light" "light"
}

test_missing_file_path_returns_error() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "action": "set_dark"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    assert_type "$result" "error"
    assert_contains "$result" "No file selected"
}

test_empty_file_path_returns_error() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "", "action": "set_dark"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    assert_type "$result" "error"
}

test_default_action_is_set_dark() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.jpg"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    # When no action specified, should default to set_dark
    assert_contains "$result" "dark"
    assert_type "$result" "execute"
}

test_command_includes_full_path_to_switchwall() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.jpg", "action": "set_dark"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    # Should have a script path in the command
    assert_contains "$result" "switchwall"
}

test_execute_response_has_command_array() {
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.jpg", "action": "set_dark"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    local command_type=$(json_get "$result" '.execute.command | type')
    assert_eq "$command_type" "array" "Command should be an array"
}

test_multiple_files_with_different_names() {
    local result1=$(echo '{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/mountain.png", "action": "set_dark"}, "session": ""}' | "$HANDLER")
    local result2=$(echo '{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/ocean.jpg", "action": "set_dark"}, "session": ""}' | "$HANDLER")
    
    local name1=$(json_get "$result1" '.execute.name')
    local name2=$(json_get "$result2" '.execute.name')
    
    assert_contains "$name1" "mountain.png"
    assert_contains "$name2" "ocean.jpg"
    assert_not_contains "$name1" "ocean"
    assert_not_contains "$name2" "mountain"
}

test_all_responses_valid() {
    assert_ok hamr_test initial
    assert_ok hamr_test search --query "test"
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_initial_returns_image_browser \
    test_initial_image_browser_has_required_fields \
    test_initial_image_browser_points_to_wallpapers_dir \
    test_initial_has_two_actions \
    test_initial_has_set_dark_action \
    test_initial_has_set_light_action \
    test_initial_actions_have_icons \
    test_search_returns_image_browser \
    test_search_returns_same_structure_as_initial \
    test_set_dark_action_returns_execute \
    test_set_dark_action_has_correct_command \
    test_set_dark_action_closes_launcher \
    test_set_dark_action_has_name \
    test_set_dark_action_has_icon \
    test_set_dark_action_has_thumbnail \
    test_set_light_action_returns_execute \
    test_set_light_action_has_correct_command \
    test_set_light_action_closes_launcher \
    test_set_light_action_has_name \
    test_set_light_action_has_icon \
    test_set_light_action_has_thumbnail \
    test_dark_and_light_use_different_modes \
    test_missing_file_path_returns_error \
    test_empty_file_path_returns_error \
    test_default_action_is_set_dark \
    test_command_includes_full_path_to_switchwall \
    test_execute_response_has_command_array \
    test_multiple_files_with_different_names \
    test_all_responses_valid
