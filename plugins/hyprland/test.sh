#!/bin/bash
# Hyprland plugin tests

export HAMR_TEST_MODE=1
source "$(dirname "$0")/../test-helpers.sh"

TEST_NAME="Hyprland Plugin Tests"
HANDLER="$(dirname "$0")/handler.py"

# ============================================================================
# Tests
# ============================================================================

test_index_returns_items() {
    local result=$(hamr_test index)
    assert_type "$result" "index"
    local count=$(json_get "$result" '.items | length')
    # 3 windows + 38 dispatchers
    [ "$count" -gt "3" ] || fail "Expected more than 3 items, got $count"
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
    assert_contains "$result" "placeholder"
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
# Dispatcher Tests
# ============================================================================

test_search_matches_dispatcher_pattern() {
    local result=$(hamr_test search --query "move to 3")
    assert_type "$result" "results"
    assert_contains "$result" "dispatch:move-to-workspace:3"
}

test_search_workspace_pattern() {
    local result=$(hamr_test search --query "workspace 5")
    assert_type "$result" "results"
    assert_contains "$result" "dispatch:goto-workspace:5"
}

test_search_toggle_floating() {
    local result=$(hamr_test search --query "floating")
    assert_type "$result" "results"
    assert_has_result "$result" "dispatch:toggle-floating"
}

test_search_fullscreen() {
    local result=$(hamr_test search --query "fullscreen")
    assert_type "$result" "results"
    assert_has_result "$result" "dispatch:fullscreen"
}

test_dispatcher_has_run_verb() {
    local result=$(hamr_test search --query "floating")
    local verb=$(json_get "$result" '.results[0].verb')
    assert_eq "$verb" "Run"
}

test_index_has_dispatchers() {
    local result=$(hamr_test index)
    assert_contains "$result" "dispatch:toggle-floating"
    assert_contains "$result" "dispatch:fullscreen"
}

test_action_dispatcher_closes() {
    local result=$(hamr_test action --id "dispatch:toggle-floating")
    assert_closes "$result"
}

test_search_center_window() {
    local result=$(hamr_test search --query "center")
    assert_type "$result" "results"
    assert_has_result "$result" "dispatch:center-window"
}

test_search_focus_direction() {
    local result=$(hamr_test search --query "focus left")
    assert_type "$result" "results"
    assert_has_result "$result" "dispatch:focus-left"
}

test_search_scratchpad() {
    local result=$(hamr_test search --query "scratchpad")
    assert_type "$result" "results"
    assert_has_result "$result" "dispatch:toggle-special"
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
    test_index_has_dispatchers \
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
    test_search_matches_dispatcher_pattern \
    test_search_workspace_pattern \
    test_search_toggle_floating \
    test_search_fullscreen \
    test_search_center_window \
    test_search_focus_direction \
    test_search_scratchpad \
    test_dispatcher_has_run_verb \
    test_action_focus_closes_launcher \
    test_action_close_refreshes \
    test_action_move_refreshes \
    test_action_dispatcher_closes \
    test_action_empty_closes_launcher
