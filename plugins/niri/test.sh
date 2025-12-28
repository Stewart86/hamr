#!/bin/bash
# Niri plugin tests

export HAMR_TEST_MODE=1
source "$(dirname "$0")/../test-helpers.sh"

TEST_NAME="Niri Plugin Tests"
HANDLER="$(dirname "$0")/handler.py"

# ============================================================================
# Tests
# ============================================================================

test_index_returns_items() {
    local result=$(hamr_test index)
    assert_type "$result" "index"
    local count=$(json_get "$result" '.items | length')
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
    assert_eq "$cmd" "niri"
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

test_index_has_actions() {
    local result=$(hamr_test index)
    assert_contains "$result" "action:close-window"
    assert_contains "$result" "action:fullscreen-window"
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
    assert_has_result "$result" "window:1"
    assert_has_result "$result" "window:2"
}

test_result_shows_title() {
    local result=$(hamr_test initial)
    assert_contains "$result" "Terminal"
    assert_contains "$result" "GitHub - Mozilla Firefox"
}

test_result_shows_app_id_in_description() {
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
    assert_has_result "$result" "window:2"
    assert_no_result "$result" "window:1"
}

test_search_filters_by_app_id() {
    local result=$(hamr_test search --query "ghostty")
    assert_type "$result" "results"
    assert_has_result "$result" "window:1"
    assert_no_result "$result" "window:2"
}

test_search_case_insensitive() {
    local result=$(hamr_test search --query "GITHUB")
    assert_type "$result" "results"
    assert_has_result "$result" "window:2"
}

test_search_no_match_shows_empty() {
    local result=$(hamr_test search --query "nonexistent12345")
    assert_type "$result" "results"
    assert_has_result "$result" "__empty__"
}

test_action_focus_closes_launcher() {
    local result=$(hamr_test action --id "window:1")
    assert_closes "$result"
}

test_action_close_refreshes() {
    local result=$(hamr_test action --id "window:1" --action "close")
    assert_type "$result" "results"
    assert_contains "$result" "Closed"
}

test_action_empty_closes_launcher() {
    local result=$(hamr_test action --id "__empty__")
    assert_closes "$result"
}

test_result_has_move_actions() {
    local result=$(hamr_test initial)
    local move=$(json_get "$result" '.results[0].actions[] | select(.id | startswith("move:")) | .id' | head -1)
    assert_contains "$move" "move:"
}

test_action_move_refreshes() {
    local result=$(hamr_test action --id "window:1" --action "move:2")
    assert_type "$result" "results"
    assert_contains "$result" "Moved"
}

# ============================================================================
# Action Tests
# ============================================================================

test_search_fullscreen() {
    local result=$(hamr_test search --query "fullscreen")
    assert_type "$result" "results"
    assert_has_result "$result" "action:fullscreen-window"
}

test_search_toggle_floating() {
    local result=$(hamr_test search --query "floating")
    assert_type "$result" "results"
    assert_has_result "$result" "action:toggle-floating"
}

test_search_center() {
    local result=$(hamr_test search --query "center")
    assert_type "$result" "results"
    assert_has_result "$result" "action:center-column"
}

test_search_overview() {
    local result=$(hamr_test search --query "overview")
    assert_type "$result" "results"
    assert_has_result "$result" "action:toggle-overview"
}

test_action_has_run_verb() {
    local result=$(hamr_test search --query "fullscreen")
    local verb=$(json_get "$result" '.results[0].verb')
    assert_eq "$verb" "Run"
}

test_action_execute_closes() {
    local result=$(hamr_test action --id "action:toggle-overview")
    assert_closes "$result"
}

test_action_workspace_goto() {
    local result=$(hamr_test action --id "action:goto-workspace:2")
    assert_closes "$result"
}

test_action_workspace_move() {
    local result=$(hamr_test action --id "action:move-to-workspace:3")
    assert_closes "$result"
}

test_index_has_workspace_shortcuts() {
    local result=$(hamr_test index)
    assert_contains "$result" "goto-workspace:1"
    assert_contains "$result" "move-to-workspace:1"
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
    test_index_has_actions \
    test_index_has_workspace_shortcuts \
    test_initial_returns_results \
    test_initial_has_placeholder \
    test_initial_realtime_mode \
    test_initial_has_mock_windows \
    test_result_shows_title \
    test_result_shows_app_id_in_description \
    test_result_shows_workspace \
    test_result_has_focus_verb \
    test_result_has_close_action \
    test_result_has_system_icon_type \
    test_result_has_move_actions \
    test_search_filters_by_title \
    test_search_filters_by_app_id \
    test_search_case_insensitive \
    test_search_no_match_shows_empty \
    test_search_fullscreen \
    test_search_toggle_floating \
    test_search_center \
    test_search_overview \
    test_action_has_run_verb \
    test_action_focus_closes_launcher \
    test_action_close_refreshes \
    test_action_move_refreshes \
    test_action_execute_closes \
    test_action_workspace_goto \
    test_action_workspace_move \
    test_action_empty_closes_launcher
