#!/bin/bash
# Windows plugin tests

export HAMR_TEST_MODE=1
source "$(dirname "$0")/../test-helpers.sh"

TEST_NAME="Windows Plugin Tests"
HANDLER="$(dirname "$0")/handler.py"

# ============================================================================
# Tests
# ============================================================================

test_index_returns_items() {
    local result=$(hamr_test index)
    assert_type "$result" "index"
    local count=$(json_get "$result" '.items | length')
    assert_eq "$count" "3"
}

test_index_item_has_window_id() {
    local result=$(hamr_test index)
    local id=$(json_get "$result" '.items[0].id')
    assert_contains "$id" "window:"
}

test_index_item_has_execute() {
    local result=$(hamr_test index)
    local cmd=$(json_get "$result" '.items[0].execute.command[0]')
    assert_eq "$cmd" "hyprctl"
}

test_index_item_has_focus_verb() {
    local result=$(hamr_test index)
    local verb=$(json_get "$result" '.items[0].verb')
    assert_eq "$verb" "Focus"
}

test_index_item_has_icon() {
    local result=$(hamr_test index)
    local icon=$(json_get "$result" '.items[0].icon')
    assert_eq "$icon" "com.mitchellh.ghostty"
}

test_initial_returns_results() {
    local result=$(hamr_test initial)
    assert_type "$result" "results"
}

test_initial_has_placeholder() {
    local result=$(hamr_test initial)
    assert_json "$result" '.placeholder' "Filter windows..."
}

test_initial_realtime_mode() {
    local result=$(hamr_test initial)
    assert_realtime_mode "$result"
}

test_initial_has_mock_windows() {
    local result=$(hamr_test initial)
    assert_has_result "$result" "window:0x55587961e9a0"
    assert_has_result "$result" "window:0x55587961e9b0"
    assert_has_result "$result" "window:0x55587961e9c0"
}

test_result_shows_title() {
    local result=$(hamr_test initial)
    assert_contains "$result" "Terminal"
    assert_contains "$result" "GitHub - Mozilla Firefox"
    assert_contains "$result" "Visual Studio Code"
}

test_result_shows_class_in_description() {
    local result=$(hamr_test initial)
    local desc=$(json_get "$result" '.results[0].description')
    assert_contains "$desc" "com.mitchellh.ghostty"
}

test_result_shows_workspace() {
    local result=$(hamr_test initial)
    local desc=$(json_get "$result" '.results[0].description')
    assert_contains "$desc" "workspace"
}

test_result_has_focus_verb() {
    local result=$(hamr_test initial)
    local verb=$(json_get "$result" '.results[0].verb')
    assert_eq "$verb" "Focus"
}

test_result_has_close_action() {
    local result=$(hamr_test initial)
    local action=$(json_get "$result" '.results[0].actions[0].id')
    assert_eq "$action" "close"
}

test_result_has_system_icon_type() {
    local result=$(hamr_test initial)
    local iconType=$(json_get "$result" '.results[0].iconType')
    assert_eq "$iconType" "system"
}

test_search_filters_by_title() {
    local result=$(hamr_test search --query "firefox")
    assert_type "$result" "results"
    assert_has_result "$result" "window:0x55587961e9b0"
    assert_no_result "$result" "window:0x55587961e9a0"
    assert_no_result "$result" "window:0x55587961e9c0"
}

test_search_filters_by_class() {
    local result=$(hamr_test search --query "ghostty")
    assert_type "$result" "results"
    assert_has_result "$result" "window:0x55587961e9a0"
    assert_no_result "$result" "window:0x55587961e9b0"
}

test_search_case_insensitive() {
    local result=$(hamr_test search --query "GITHUB")
    assert_type "$result" "results"
    assert_has_result "$result" "window:0x55587961e9b0"
}

test_search_no_match_shows_empty() {
    local result=$(hamr_test search --query "nonexistent12345")
    assert_type "$result" "results"
    assert_has_result "$result" "__empty__"
}

test_action_focus_closes_launcher() {
    local result=$(hamr_test action --id "window:0x55587961e9a0")
    assert_closes "$result"
}

test_action_close_refreshes() {
    local result=$(hamr_test action --id "window:0x55587961e9a0" --action "close")
    assert_type "$result" "results"
    assert_contains "$result" "Closed"
}

test_action_empty_closes_launcher() {
    local result=$(hamr_test action --id "__empty__")
    assert_closes "$result"
}

test_result_has_move_actions() {
    local result=$(hamr_test initial)
    # Window on workspace 1 should have move actions for workspace 2 and 3
    local move_2=$(json_get "$result" '.results[0].actions[] | select(.id == "move:2") | .id')
    local move_3=$(json_get "$result" '.results[0].actions[] | select(.id == "move:3") | .id')
    assert_eq "$move_2" "move:2"
    assert_eq "$move_3" "move:3"
}

test_result_move_action_format() {
    local result=$(hamr_test initial)
    # Window on workspace 1 should have move:2 and move:3 actions
    local move_action=$(json_get "$result" '.results[0].actions[1].id')
    assert_contains "$move_action" "move:"
}

test_action_move_refreshes() {
    local result=$(hamr_test action --id "window:0x55587961e9a0" --action "move:2")
    assert_type "$result" "results"
    assert_contains "$result" "Moved"
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_index_returns_items \
    test_index_item_has_window_id \
    test_index_item_has_execute \
    test_index_item_has_focus_verb \
    test_index_item_has_icon \
    test_initial_returns_results \
    test_initial_has_placeholder \
    test_initial_realtime_mode \
    test_initial_has_mock_windows \
    test_result_shows_title \
    test_result_shows_class_in_description \
    test_result_shows_workspace \
    test_result_has_focus_verb \
    test_result_has_close_action \
    test_result_has_system_icon_type \
    test_result_has_move_actions \
    test_result_move_action_format \
    test_search_filters_by_title \
    test_search_filters_by_class \
    test_search_case_insensitive \
    test_search_no_match_shows_empty \
    test_action_focus_closes_launcher \
    test_action_close_refreshes \
    test_action_move_refreshes \
    test_action_empty_closes_launcher
