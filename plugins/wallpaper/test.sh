#!/bin/bash
#
# Tests for wallpaper plugin
# Run: ./test.sh
#
# The plugin behavior depends on whether a switchwall.sh script exists:
# - With switchwall.sh: Shows dark/light mode actions (2 actions)
# - Without switchwall.sh: Shows simple "Set Wallpaper" action (1 action)
#
# Tests automatically detect which mode we're in.
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

# Check if switchwall.sh exists (same paths as handler.py)
XDG_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}"
HAMR_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")"
SWITCHWALL_PATHS=(
    "$HAMR_DIR/scripts/colors/switchwall.sh"
    "$XDG_CONFIG/hamr/scripts/switchwall.sh"
)

HAS_SWITCHWALL=false
for script in "${SWITCHWALL_PATHS[@]}"; do
    if [[ -x "$script" ]]; then
        HAS_SWITCHWALL=true
        break
    fi
done

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

test_initial_has_correct_action_count() {
    local result=$(hamr_test initial)
    
    local count=$(json_get "$result" '.imageBrowser.actions | length')
    if [[ "$HAS_SWITCHWALL" == "true" ]]; then
        assert_eq "$count" "2" "imageBrowser should have 2 actions (with switchwall.sh)"
    else
        assert_eq "$count" "1" "imageBrowser should have 1 action (no switchwall.sh)"
    fi
}

test_initial_has_correct_actions() {
    local result=$(hamr_test initial)
    
    if [[ "$HAS_SWITCHWALL" == "true" ]]; then
        # With switchwall.sh, should have dark/light actions
        local dark_id=$(json_get "$result" '.imageBrowser.actions[] | select(.id == "set_dark") | .id')
        assert_eq "$dark_id" "set_dark" "Should have 'set_dark' action"
        
        local light_id=$(json_get "$result" '.imageBrowser.actions[] | select(.id == "set_light") | .id')
        assert_eq "$light_id" "set_light" "Should have 'set_light' action"
    else
        # Without switchwall.sh, should have simple set action
        local id=$(json_get "$result" '.imageBrowser.actions[0].id')
        assert_eq "$id" "set" "Should have 'set' action when no switchwall.sh"
    fi
}

test_initial_actions_have_correct_names() {
    local result=$(hamr_test initial)
    
    if [[ "$HAS_SWITCHWALL" == "true" ]]; then
        local dark_name=$(json_get "$result" '.imageBrowser.actions[] | select(.id == "set_dark") | .name')
        assert_eq "$dark_name" "Set (Dark Mode)" "Dark mode action should have correct name"
        
        local light_name=$(json_get "$result" '.imageBrowser.actions[] | select(.id == "set_light") | .name')
        assert_eq "$light_name" "Set (Light Mode)" "Light mode action should have correct name"
    else
        local name=$(json_get "$result" '.imageBrowser.actions[0].name')
        assert_eq "$name" "Set Wallpaper" "Set action should have correct name"
    fi
}

test_initial_actions_have_icons() {
    local result=$(hamr_test initial)
    
    if [[ "$HAS_SWITCHWALL" == "true" ]]; then
        local dark_icon=$(json_get "$result" '.imageBrowser.actions[] | select(.id == "set_dark") | .icon')
        assert_eq "$dark_icon" "dark_mode" "Dark action should have dark_mode icon"
        
        local light_icon=$(json_get "$result" '.imageBrowser.actions[] | select(.id == "set_light") | .icon')
        assert_eq "$light_icon" "light_mode" "Light action should have light_mode icon"
    else
        local icon=$(json_get "$result" '.imageBrowser.actions[0].icon')
        assert_eq "$icon" "wallpaper" "Set action should have wallpaper icon"
    fi
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

test_set_action_returns_execute() {
    local action="set"
    [[ "$HAS_SWITCHWALL" == "true" ]] && action="set_dark"
    
    local input="{\"step\": \"action\", \"selected\": {\"id\": \"imageBrowser\", \"path\": \"/tmp/test.jpg\", \"action\": \"$action\"}, \"session\": \"\"}"
    local result=$(echo "$input" | "$HANDLER")
    
    assert_type "$result" "execute"
}

test_set_action_has_command() {
    local action="set"
    [[ "$HAS_SWITCHWALL" == "true" ]] && action="set_dark"
    
    local input="{\"step\": \"action\", \"selected\": {\"id\": \"imageBrowser\", \"path\": \"/tmp/wallpaper.jpg\", \"action\": \"$action\"}, \"session\": \"\"}"
    local result=$(echo "$input" | "$HANDLER")
    
    # Should have a command array
    local command_type=$(json_get "$result" '.execute.command | type')
    assert_eq "$command_type" "array" "Command should be an array"
    
    # Command should reference the file path
    assert_contains "$result" "/tmp/wallpaper.jpg"
}

test_set_action_closes_launcher() {
    local action="set"
    [[ "$HAS_SWITCHWALL" == "true" ]] && action="set_dark"
    
    local input="{\"step\": \"action\", \"selected\": {\"id\": \"imageBrowser\", \"path\": \"/tmp/test.jpg\", \"action\": \"$action\"}, \"session\": \"\"}"
    local result=$(echo "$input" | "$HANDLER")
    
    assert_closes "$result"
}

test_set_action_has_name() {
    local action="set"
    [[ "$HAS_SWITCHWALL" == "true" ]] && action="set_dark"
    
    local input="{\"step\": \"action\", \"selected\": {\"id\": \"imageBrowser\", \"path\": \"/tmp/wallpaper.jpg\", \"action\": \"$action\"}, \"session\": \"\"}"
    local result=$(echo "$input" | "$HANDLER")
    
    local name=$(json_get "$result" '.execute.name')
    assert_contains "$name" "wallpaper.jpg" "Should have action name with filename"
    assert_contains "$name" "Set wallpaper" "Should mention setting wallpaper"
}

test_set_action_has_icon() {
    local action="set"
    [[ "$HAS_SWITCHWALL" == "true" ]] && action="set_dark"
    
    local input="{\"step\": \"action\", \"selected\": {\"id\": \"imageBrowser\", \"path\": \"/tmp/test.jpg\", \"action\": \"$action\"}, \"session\": \"\"}"
    local result=$(echo "$input" | "$HANDLER")
    
    local icon=$(json_get "$result" '.execute.icon')
    assert_eq "$icon" "wallpaper" "Should have wallpaper icon"
}

test_set_action_has_thumbnail() {
    local action="set"
    [[ "$HAS_SWITCHWALL" == "true" ]] && action="set_dark"
    
    local input="{\"step\": \"action\", \"selected\": {\"id\": \"imageBrowser\", \"path\": \"/tmp/wallpaper.jpg\", \"action\": \"$action\"}, \"session\": \"\"}"
    local result=$(echo "$input" | "$HANDLER")
    
    local thumbnail=$(json_get "$result" '.execute.thumbnail')
    assert_eq "$thumbnail" "/tmp/wallpaper.jpg" "Should have thumbnail set to image path"
}

test_dark_light_modes_differ() {
    # Only test if switchwall.sh exists
    if [[ "$HAS_SWITCHWALL" != "true" ]]; then
        echo "    [SKIP] No switchwall.sh - dark/light modes not available"
        return 0
    fi
    
    local dark=$(echo '{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.jpg", "action": "set_dark"}, "session": ""}' | "$HANDLER")
    local light=$(echo '{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.jpg", "action": "set_light"}, "session": ""}' | "$HANDLER")
    
    # Dark should have dark mode
    assert_contains "$dark" "dark"
    
    # Light should have light mode
    assert_contains "$light" "light"
}

test_missing_file_path_returns_error() {
    local action="set"
    [[ "$HAS_SWITCHWALL" == "true" ]] && action="set_dark"
    
    local input="{\"step\": \"action\", \"selected\": {\"id\": \"imageBrowser\", \"action\": \"$action\"}, \"session\": \"\"}"
    local result=$(echo "$input" | "$HANDLER")
    
    assert_type "$result" "error"
    assert_contains "$result" "No file selected"
}

test_empty_file_path_returns_error() {
    local action="set"
    [[ "$HAS_SWITCHWALL" == "true" ]] && action="set_dark"
    
    local input="{\"step\": \"action\", \"selected\": {\"id\": \"imageBrowser\", \"path\": \"\", \"action\": \"$action\"}, \"session\": \"\"}"
    local result=$(echo "$input" | "$HANDLER")
    
    assert_type "$result" "error"
}

test_default_action_works() {
    # When no action specified, should still work
    local input='{"step": "action", "selected": {"id": "imageBrowser", "path": "/tmp/test.jpg"}, "session": ""}'
    local result=$(echo "$input" | "$HANDLER")
    
    assert_type "$result" "execute"
}

test_execute_response_has_command_array() {
    local action="set"
    [[ "$HAS_SWITCHWALL" == "true" ]] && action="set_dark"
    
    local input="{\"step\": \"action\", \"selected\": {\"id\": \"imageBrowser\", \"path\": \"/tmp/test.jpg\", \"action\": \"$action\"}, \"session\": \"\"}"
    local result=$(echo "$input" | "$HANDLER")
    
    local command_type=$(json_get "$result" '.execute.command | type')
    assert_eq "$command_type" "array" "Command should be an array"
}

test_multiple_files_with_different_names() {
    local action="set"
    [[ "$HAS_SWITCHWALL" == "true" ]] && action="set_dark"
    
    local result1=$(echo "{\"step\": \"action\", \"selected\": {\"id\": \"imageBrowser\", \"path\": \"/tmp/mountain.png\", \"action\": \"$action\"}, \"session\": \"\"}" | "$HANDLER")
    local result2=$(echo "{\"step\": \"action\", \"selected\": {\"id\": \"imageBrowser\", \"path\": \"/tmp/ocean.jpg\", \"action\": \"$action\"}, \"session\": \"\"}" | "$HANDLER")
    
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

test_initial_has_plugin_actions() {
    local result=$(hamr_test initial)
    
    assert_contains "$result" "pluginActions"
}

test_random_action_available() {
    local result=$(hamr_test initial)
    
    local random_id=$(json_get "$result" '.pluginActions[] | select(.id == "random") | .id')
    assert_eq "$random_id" "random"
}

test_history_action_available() {
    local result=$(hamr_test initial)
    
    local history_id=$(json_get "$result" '.pluginActions[] | select(.id == "history") | .id')
    assert_eq "$history_id" "history"
}

test_history_action_shows_results() {
    local result=$(hamr_test action --id "__plugin__" --action "history")
    
    assert_type "$result" "results"
}

# ============================================================================
# Run
# ============================================================================

# Print mode info
if [[ "$HAS_SWITCHWALL" == "true" ]]; then
    echo "  [INFO] switchwall.sh detected - testing dark/light mode"
else
    echo "  [INFO] No switchwall.sh - testing simple set mode"
fi
echo ""

run_tests \
    test_initial_returns_image_browser \
    test_initial_image_browser_has_required_fields \
    test_initial_image_browser_points_to_wallpapers_dir \
    test_initial_has_correct_action_count \
    test_initial_has_correct_actions \
    test_initial_actions_have_correct_names \
    test_initial_actions_have_icons \
    test_search_returns_image_browser \
    test_search_returns_same_structure_as_initial \
    test_set_action_returns_execute \
    test_set_action_has_command \
    test_set_action_closes_launcher \
    test_set_action_has_name \
    test_set_action_has_icon \
    test_set_action_has_thumbnail \
    test_dark_light_modes_differ \
    test_missing_file_path_returns_error \
    test_empty_file_path_returns_error \
    test_default_action_works \
    test_execute_response_has_command_array \
    test_multiple_files_with_different_names \
    test_all_responses_valid \
    test_initial_has_plugin_actions \
    test_random_action_available \
    test_history_action_available \
    test_history_action_shows_results
