#!/bin/bash
# Emoji plugin tests

export HAMR_TEST_MODE=1
source "$(dirname "$0")/../test-helpers.sh"

TEST_NAME="Emoji Plugin Tests"
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
    assert_json "$result" '.placeholder' "Search emojis..."
}

test_initial_returns_many_results() {
    local result=$(hamr_test initial)
    local count=$(get_result_count "$result")
    # Should return up to 100 results when no query
    [[ "$count" -gt 50 ]] || { echo "Expected >50 results, got $count"; return 1; }
}

test_search_filters_results() {
    local result=$(hamr_test search --query "smile")
    assert_type "$result" "results"
    # Should have some smiling emoji results
    local count=$(get_result_count "$result")
    [[ "$count" -gt 0 ]] || { echo "Expected results for 'smile', got none"; return 1; }
}

test_search_heart() {
    local result=$(hamr_test search --query "heart")
    assert_type "$result" "results"
    local count=$(get_result_count "$result")
    [[ "$count" -gt 0 ]] || { echo "Expected results for 'heart', got none"; return 1; }
}

test_result_has_emoji_icon() {
    local result=$(hamr_test search --query "smile")
    # Results should have iconType: "text" for emoji display
    local icon_type=$(json_get "$result" '.results[0].iconType')
    assert_eq "$icon_type" "text"
}

test_result_has_copy_verb() {
    local result=$(hamr_test search --query "smile")
    local verb=$(json_get "$result" '.results[0].verb')
    assert_eq "$verb" "Copy"
}

test_result_has_actions() {
    local result=$(hamr_test search --query "smile")
    local actions=$(json_get "$result" '.results[0].actions | length')
    assert_eq "$actions" "2"  # copy and type
}

test_action_copy() {
    # Get first smile result
    local results=$(hamr_test search --query "smile")
    local emoji_id=$(json_get "$results" '.results[0].id')
    
    local result=$(hamr_test action --id "$emoji_id" --action "copy")
    assert_type "$result" "execute"
    assert_closes "$result"
    assert_contains "$result" "Copied"
}

test_action_type() {
    local results=$(hamr_test search --query "smile")
    local emoji_id=$(json_get "$results" '.results[0].id')
    
    local result=$(hamr_test action --id "$emoji_id" --action "type")
    assert_type "$result" "execute"
    assert_closes "$result"
    assert_contains "$result" "Typed"
}

test_action_default_is_copy() {
    local results=$(hamr_test search --query "smile")
    local emoji_id=$(json_get "$results" '.results[0].id')
    
    # No action specified should default to copy
    local result=$(hamr_test action --id "$emoji_id")
    assert_type "$result" "execute"
    assert_contains "$result" "Copied"
}

test_index_returns_items() {
    local result=$(hamr_test index)
    assert_type "$result" "index"
    # Should have many indexed items
    local count=$(json_get "$result" '.items | length')
    [[ "$count" -gt 100 ]] || { echo "Expected >100 indexed items, got $count"; return 1; }
}

test_index_items_have_execute() {
    local result=$(hamr_test index)
    # Each indexed item should have execute.command for direct search usage
    local cmd=$(json_get "$result" '.items[0].execute.command[0]')
    assert_eq "$cmd" "wl-copy"
}

test_index_items_have_execute_name() {
    local result=$(hamr_test index)
    # Each indexed item should have execute.name for history tracking
    local name=$(json_get "$result" '.items[0].execute.name')
    [[ -n "$name" ]] || { echo "Expected execute.name to be set for history tracking"; return 1; }
}

test_index_items_have_id_prefix() {
    local result=$(hamr_test index)
    local id=$(json_get "$result" '.items[0].id')
    assert_contains "$id" "emoji:"
}

test_empty_query_returns_many() {
    local result=$(hamr_test search --query "")
    local count=$(get_result_count "$result")
    [[ "$count" -gt 50 ]] || { echo "Expected >50 results for empty query, got $count"; return 1; }
}

test_no_match_returns_empty() {
    local result=$(hamr_test search --query "xyznonexistent12345")
    local count=$(get_result_count "$result")
    assert_eq "$count" "0"
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_initial_returns_results \
    test_initial_has_placeholder \
    test_initial_returns_many_results \
    test_search_filters_results \
    test_search_heart \
    test_result_has_emoji_icon \
    test_result_has_copy_verb \
    test_result_has_actions \
    test_action_copy \
    test_action_type \
    test_action_default_is_copy \
    test_index_returns_items \
    test_index_items_have_execute \
    test_index_items_have_execute_name \
    test_index_items_have_id_prefix \
    test_empty_query_returns_many \
    test_no_match_returns_empty
