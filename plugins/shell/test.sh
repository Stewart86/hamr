#!/bin/bash
#
# Tests for shell plugin
# Run: ./test.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export HAMR_TEST_MODE=1
source "$SCRIPT_DIR/../test-helpers.sh"

# ============================================================================
# Config
# ============================================================================

TEST_NAME="Shell Plugin Tests"
HANDLER="$SCRIPT_DIR/handler.py"

# ============================================================================
# Helpers
# ============================================================================

# Check if a command exists in shell history (for testing purposes)
has_test_command() {
    local cmd="$1"
    # We can't easily mock shell history, so we'll just verify response structure
    # rather than actual history content
    return 0
}

# ============================================================================
# Tests
# ============================================================================

test_initial_returns_results() {
    local result=$(hamr_test initial)
    
    assert_type "$result" "results"
}

test_initial_realtime_mode() {
    local result=$(hamr_test initial)
    
    assert_realtime_mode "$result"
}

test_initial_has_results() {
    local result=$(hamr_test initial)
    
    # Should have at least some shell history (or empty results)
    local count=$(get_result_count "$result")
    if [[ $count -gt 0 ]]; then
        assert_has_result "$result" "$(json_get "$result" '.results[0].id')"
    fi
}

test_initial_result_structure() {
    local result=$(hamr_test initial)
    local count=$(get_result_count "$result")
    
    if [[ $count -gt 0 ]]; then
        local first_result=$(json_get "$result" '.results[0]')
        
        # Each result should have id and name
        local id=$(echo "$first_result" | jq -r '.id')
        local name=$(echo "$first_result" | jq -r '.name')
        assert_ok test -n "$id"
        assert_ok test -n "$name"
    fi
}

test_initial_result_has_actions() {
    local result=$(hamr_test initial)
    local count=$(get_result_count "$result")
    
    if [[ $count -gt 0 ]]; then
        local first_result=$(json_get "$result" '.results[0]')
        local actions=$(echo "$first_result" | jq '.actions | length')
        
        # Should have run-float, run-tiled, and copy actions
        assert_eq "$actions" "3" "First result should have 3 actions"
    fi
}

test_initial_action_ids() {
    local result=$(hamr_test initial)
    local count=$(get_result_count "$result")
    
    if [[ $count -gt 0 ]]; then
        local first_result=$(json_get "$result" '.results[0]')
        local action_ids=$(echo "$first_result" | jq -r '.actions[].id')
        
        assert_contains "$action_ids" "run-float"
        assert_contains "$action_ids" "run-tiled"
        assert_contains "$action_ids" "copy"
    fi
}

test_initial_limits_results_to_50() {
    local result=$(hamr_test initial)
    local count=$(get_result_count "$result")
    
    # Initial should return at most 50 results
    if [[ $count -gt 0 ]]; then
        assert_ok test $count -le 50
    fi
}

test_search_returns_results() {
    local result=$(hamr_test search --query "ls")
    
    assert_type "$result" "results"
}

test_search_realtime_mode() {
    local result=$(hamr_test search --query "test")
    
    assert_realtime_mode "$result"
}

test_search_empty_query_returns_results() {
    local result=$(hamr_test search --query "")
    
    # Empty query should return all history (up to 50)
    assert_type "$result" "results"
    local count=$(get_result_count "$result")
    assert_ok test $count -le 50
}

test_search_fuzzy_filter() {
    # Test with a query that unlikely to match much
    local result=$(hamr_test search --query "zzzzzzzzzzz")
    
    # Should return results type even if empty
    assert_type "$result" "results"
}

test_search_filters_results() {
    # Search for "ls" should return some results or none, but not error
    local result=$(hamr_test search --query "l")
    
    assert_type "$result" "results"
}

test_search_result_structure() {
    local result=$(hamr_test search --query "e")
    local count=$(get_result_count "$result")
    
    if [[ $count -gt 0 ]]; then
        local first_result=$(json_get "$result" '.results[0]')
        
        # Each result should have id, name, and actions
        local id=$(echo "$first_result" | jq -r '.id')
        local name=$(echo "$first_result" | jq -r '.name')
        local actions=$(echo "$first_result" | jq '.actions | length')
        
        assert_ok test -n "$id"
        assert_ok test -n "$name"
        assert_ok test $actions -gt 0
    fi
}

test_action_run_float_structure() {
    # Create a test by using initial results
    local initial=$(hamr_test initial)
    local count=$(get_result_count "$initial")
    
    if [[ $count -gt 0 ]]; then
        local cmd=$(json_get "$initial" '.results[0].id')
        local result=$(hamr_test action --id "$cmd" --action "run-float")
        
        assert_type "$result" "execute"
    fi
}

test_action_run_float_closes() {
    local initial=$(hamr_test initial)
    local count=$(get_result_count "$initial")
    
    if [[ $count -gt 0 ]]; then
        local cmd=$(json_get "$initial" '.results[0].id')
        local result=$(hamr_test action --id "$cmd" --action "run-float")
        
        assert_closes "$result"
    fi
}

test_action_run_float_has_name() {
    local initial=$(hamr_test initial)
    local count=$(get_result_count "$initial")
    
    if [[ $count -gt 0 ]]; then
        local cmd=$(json_get "$initial" '.results[0].id')
        local result=$(hamr_test action --id "$cmd" --action "run-float")
        
        local name=$(json_get "$result" '.execute.name')
        assert_ok test -n "$name"
        assert_contains "$name" "Run:"
    fi
}

test_action_run_float_has_icon() {
    local initial=$(hamr_test initial)
    local count=$(get_result_count "$initial")
    
    if [[ $count -gt 0 ]]; then
        local cmd=$(json_get "$initial" '.results[0].id')
        local result=$(hamr_test action --id "$cmd" --action "run-float")
        
        local icon=$(json_get "$result" '.execute.icon')
        assert_eq "$icon" "terminal"
    fi
}

test_action_run_float_has_command() {
    local initial=$(hamr_test initial)
    local count=$(get_result_count "$initial")
    
    if [[ $count -gt 0 ]]; then
        local cmd=$(json_get "$initial" '.results[0].id')
        local result=$(hamr_test action --id "$cmd" --action "run-float")
        
        local command=$(json_get "$result" '.execute.command')
        assert_ok test -n "$command"
    fi
}

test_action_run_tiled_structure() {
    local initial=$(hamr_test initial)
    local count=$(get_result_count "$initial")
    
    if [[ $count -gt 0 ]]; then
        local cmd=$(json_get "$initial" '.results[0].id')
        local result=$(hamr_test action --id "$cmd" --action "run-tiled")
        
        assert_type "$result" "execute"
        assert_closes "$result"
    fi
}

test_action_run_tiled_has_name() {
    local initial=$(hamr_test initial)
    local count=$(get_result_count "$initial")
    
    if [[ $count -gt 0 ]]; then
        local cmd=$(json_get "$initial" '.results[0].id')
        local result=$(hamr_test action --id "$cmd" --action "run-tiled")
        
        local name=$(json_get "$result" '.execute.name')
        assert_contains "$name" "Run:"
    fi
}

test_action_copy_has_icon_in_action() {
    # Verify copy action exists in initial results with proper icon
    local initial=$(hamr_test initial)
    local count=$(get_result_count "$initial")
    
    if [[ $count -gt 0 ]]; then
        local first_result=$(json_get "$initial" '.results[0]')
        local copy_action=$(echo "$first_result" | jq '.actions[] | select(.id == "copy")')
        
        if [[ -n "$copy_action" ]]; then
            local icon=$(echo "$copy_action" | jq -r '.icon')
            assert_eq "$icon" "content_copy"
        fi
    fi
}

test_action_copy_action_exists() {
    # Verify copy action is available (skipping actual execution as wl-copy blocks)
    local initial=$(hamr_test initial)
    local count=$(get_result_count "$initial")
    
    if [[ $count -gt 0 ]]; then
        local first_result=$(json_get "$initial" '.results[0]')
        local copy_action=$(echo "$first_result" | jq '.actions[] | select(.id == "copy")')
        
        assert_ok test -n "$copy_action"
    fi
}

test_action_default_action() {
    local initial=$(hamr_test initial)
    local count=$(get_result_count "$initial")
    
    if [[ $count -gt 0 ]]; then
        local cmd=$(json_get "$initial" '.results[0].id')
        # No action specified - should default to run-float
        local result=$(hamr_test action --id "$cmd")
        
        assert_type "$result" "execute"
        assert_closes "$result"
    fi
}

test_action_long_command_truncated() {
    # Create a long command string by searching
    local result=$(hamr_test search --query "a")
    local count=$(get_result_count "$result")
    
    if [[ $count -gt 0 ]]; then
        local cmd=$(json_get "$result" '.results[0].id')
        local action_result=$(hamr_test action --id "$cmd" --action "copy")
        
        local name=$(json_get "$action_result" '.execute.name')
        # If command > 50 chars, should be truncated with "..."
        local cmd_len=${#cmd}
        if [[ $cmd_len -gt 50 ]]; then
            assert_contains "$name" "..."
        fi
    fi
}

test_action_empty_id_error() {
    local result=$(hamr_test action --id "")
    
    assert_type "$result" "error"
}

test_all_responses_valid() {
    assert_ok hamr_test initial
    assert_ok hamr_test search --query "test"
    assert_ok hamr_test search --query ""
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_initial_returns_results \
    test_initial_realtime_mode \
    test_initial_has_results \
    test_initial_result_structure \
    test_initial_result_has_actions \
    test_initial_action_ids \
    test_initial_limits_results_to_50 \
    test_search_returns_results \
    test_search_realtime_mode \
    test_search_empty_query_returns_results \
    test_search_fuzzy_filter \
    test_search_filters_results \
    test_search_result_structure \
    test_action_run_float_structure \
    test_action_run_float_closes \
    test_action_run_float_has_name \
    test_action_run_float_has_icon \
    test_action_run_float_has_command \
    test_action_run_tiled_structure \
    test_action_run_tiled_has_name \
    test_action_copy_has_icon_in_action \
    test_action_copy_action_exists \
    test_action_default_action \
    test_action_long_command_truncated \
    test_action_empty_id_error \
    test_all_responses_valid
