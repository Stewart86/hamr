#!/bin/bash
# Top CPU plugin tests

export HAMR_TEST_MODE=1
source "$(dirname "$0")/../test-helpers.sh"

TEST_NAME="Top CPU Plugin Tests"
HANDLER="$(dirname "$0")/handler.py"

# ============================================================================
# Tests
# ============================================================================

test_initial_returns_results() {
    local result=$(hamr_test initial)
    assert_type "$result" "results"
}

test_initial_has_placeholder() {
    local result=$(hamr_test initial)
    assert_json "$result" '.placeholder' "Filter processes..."
}

test_initial_realtime_mode() {
    local result=$(hamr_test initial)
    assert_realtime_mode "$result"
}

test_initial_has_mock_processes() {
    local result=$(hamr_test initial)
    # Mock data has 3 processes
    assert_has_result "$result" "proc:1234"
    assert_has_result "$result" "proc:5678"
    assert_has_result "$result" "proc:9012"
}

test_result_shows_process_name() {
    local result=$(hamr_test initial)
    assert_contains "$result" "firefox"
    assert_contains "$result" "code"
    assert_contains "$result" "python3"
}

test_result_shows_cpu_percentage() {
    local result=$(hamr_test initial)
    assert_contains "$result" "CPU:"
    assert_contains "$result" "25.5%"  # firefox CPU from mock
}

test_result_shows_memory() {
    local result=$(hamr_test initial)
    assert_contains "$result" "Mem:"
}

test_result_has_kill_verb() {
    local result=$(hamr_test initial)
    local verb=$(json_get "$result" '.results[0].verb')
    assert_eq "$verb" "Kill"
}

test_result_has_actions() {
    local result=$(hamr_test initial)
    local actions=$(json_get "$result" '.results[0].actions | length')
    assert_eq "$actions" "2"  # kill and kill9
}

test_action_kill_names() {
    local result=$(hamr_test initial)
    local kill_name=$(json_get "$result" '.results[0].actions[0].name')
    local kill9_name=$(json_get "$result" '.results[0].actions[1].name')
    assert_contains "$kill_name" "SIGTERM"
    assert_contains "$kill9_name" "SIGKILL"
}

test_search_filters_by_name() {
    local result=$(hamr_test search --query "firefox")
    assert_type "$result" "results"
    assert_has_result "$result" "proc:1234"
    assert_no_result "$result" "proc:5678"  # code
    assert_no_result "$result" "proc:9012"  # python3
}

test_search_filters_by_pid() {
    local result=$(hamr_test search --query "5678")
    assert_type "$result" "results"
    assert_has_result "$result" "proc:5678"
    assert_no_result "$result" "proc:1234"
}

test_search_no_match_shows_empty() {
    local result=$(hamr_test search --query "nonexistent12345")
    assert_type "$result" "results"
    assert_has_result "$result" "__empty__"
}

test_poll_returns_results() {
    local result=$(hamr_test poll)
    assert_type "$result" "results"
    # Should still have mock processes
    assert_has_result "$result" "proc:1234"
}

test_poll_with_query() {
    local result=$(hamr_test poll --query "firefox")
    assert_type "$result" "results"
    assert_has_result "$result" "proc:1234"
    assert_no_result "$result" "proc:5678"
}

test_action_kill() {
    local result=$(hamr_test action --id "proc:1234" --action "kill")
    assert_type "$result" "results"
    # Should show notify message
    assert_contains "$result" "killed"
}

test_action_kill9() {
    local result=$(hamr_test action --id "proc:1234" --action "kill9")
    assert_type "$result" "results"
    assert_contains "$result" "killed"
}

test_action_default_is_kill() {
    # No action specified should default to kill (SIGTERM)
    local result=$(hamr_test action --id "proc:1234")
    assert_type "$result" "results"
    assert_contains "$result" "killed"
}

test_action_empty_refreshes() {
    local result=$(hamr_test action --id "__empty__")
    assert_type "$result" "results"
    # Should refresh and show processes again
    assert_has_result "$result" "proc:1234"
}

test_process_id_format() {
    local result=$(hamr_test initial)
    # Process IDs should be in format "proc:PID"
    local id=$(json_get "$result" '.results[0].id')
    assert_contains "$id" "proc:"
}

test_process_has_memory_icon() {
    local result=$(hamr_test initial)
    local icon=$(json_get "$result" '.results[0].icon')
    assert_eq "$icon" "memory"
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_initial_returns_results \
    test_initial_has_placeholder \
    test_initial_realtime_mode \
    test_initial_has_mock_processes \
    test_result_shows_process_name \
    test_result_shows_cpu_percentage \
    test_result_shows_memory \
    test_result_has_kill_verb \
    test_result_has_actions \
    test_action_kill_names \
    test_search_filters_by_name \
    test_search_filters_by_pid \
    test_search_no_match_shows_empty \
    test_poll_returns_results \
    test_poll_with_query \
    test_action_kill \
    test_action_kill9 \
    test_action_default_is_kill \
    test_action_empty_refreshes \
    test_process_id_format \
    test_process_has_memory_icon
