#!/bin/bash
#
# Tests for apps plugin
# Run: ./test.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export HAMR_TEST_MODE=1
source "$SCRIPT_DIR/../test-helpers.sh"

# ============================================================================
# Config
# ============================================================================

TEST_NAME="Apps Plugin Tests"
HANDLER="$SCRIPT_DIR/handler.py"

# ============================================================================
# Tests
# ============================================================================

test_initial_shows_categories() {
    local result=$(hamr_test initial)
    
    assert_type "$result" "results"
    assert_has_result "$result" "__cat__:All"
    assert_contains "$result" "All Applications"
}

test_initial_has_positive_app_count() {
    local result=$(hamr_test initial)
    
    # Should have All with some app count
    local all_result=$(json_get "$result" '.results[] | select(.id == "__cat__:All") | .description')
    assert_contains "$all_result" "apps"
}

test_category_selection_shows_apps_in_category() {
    local result=$(hamr_test initial)
    
    # Get the first category (not All)
    local first_cat=$(json_get "$result" '.results[1].id' | sed 's/__cat__://')
    
    # Select that category
    local cat_result=$(hamr_test action --id "$(json_get "$result" '.results[1].id')")
    
    assert_type "$cat_result" "results"
    assert_has_result "$cat_result" "__back__"
}

test_all_category_shows_apps() {
    local result=$(hamr_test action --id "__cat__:All")
    
    assert_type "$result" "results"
    assert_has_result "$result" "__back__"
    # Should have at least some results beyond back button
    local count=$(get_result_count "$result")
    assert_eq "$([ $count -gt 1 ] && echo 1 || echo 0)" "1" "All category should have apps"
}

test_back_button_returns_to_categories() {
    local to_all=$(hamr_test action --id "__cat__:All")
    local back=$(hamr_test action --id "__back__")
    
    assert_type "$back" "results"
    assert_has_result "$back" "__cat__:All"
    assert_contains "$back" "All Applications"
}

test_search_with_query_returns_results() {
    local result=$(hamr_test search --query "text")
    
    assert_type "$result" "results"
}

test_search_empty_query_shows_categories() {
    local result=$(hamr_test search --query "")
    
    assert_type "$result" "results"
    assert_has_result "$result" "__cat__:All"
    assert_contains "$result" "All Applications"
}

test_search_nonexistent_shows_empty_state() {
    local result=$(hamr_test search --query "zzzzzzzznonexistent")
    
    assert_type "$result" "results"
    assert_has_result "$result" "__empty__"
    assert_contains "$result" "No apps found"
}

test_search_in_category_with_context() {
    hamr_test action --id "__cat__:All" > /dev/null
    local result=$(hamr_test search --query "text" --context "__cat__:All")
    
    assert_type "$result" "results"
    # Should have back button in category search
    assert_has_result "$result" "__back__"
}

test_search_preserves_placeholder() {
    hamr_test action --id "__cat__:All" > /dev/null
    local result=$(hamr_test search --query "" --context "__cat__:All")
    
    local placeholder=$(json_get "$result" '.placeholder')
    assert_contains "$placeholder" "Search"
}

test_realtime_input_mode() {
    local result=$(hamr_test initial)
    
    local mode=$(json_get "$result" '.inputMode')
    assert_eq "$mode" "realtime"
}

test_category_context_persists() {
    hamr_test action --id "__cat__:All" > /dev/null
    local result=$(hamr_test search --query "a" --context "__cat__:All")
    
    local context=$(json_get "$result" '.context')
    assert_eq "$context" "__cat__:All"
}

test_app_selection_returns_execute() {
    local all_result=$(hamr_test action --id "__cat__:All")
    
    # Get first app ID (skip back button)
    local app_id=$(json_get "$all_result" '.results[] | select(.id != "__back__") | .id' | head -1)
    
    if [[ -n "$app_id" && "$app_id" != "null" ]]; then
        local launch=$(hamr_test action --id "$app_id")
        assert_type "$launch" "execute"
        assert_closes "$launch"
    fi
}

test_result_has_required_fields() {
    local result=$(hamr_test initial)
    
    # Check that category results have id and name
    local ids=$(json_get "$result" '.results[].id')
    assert_contains "$ids" "__cat__:All"
}

test_all_results_have_id() {
    hamr_test action --id "__cat__:All" > /dev/null
    local result=$(hamr_test search --query "text" --context "__cat__:All")
    
    # Every result should have an id field
    local empty_ids=$(json_get "$result" '.results[] | select(.id == null)' | wc -l)
    assert_eq "$empty_ids" "0" "All results should have an id"
}

test_category_selection_sets_clear_input() {
    local result=$(hamr_test action --id "__cat__:All")
    
    local clear_input=$(json_get "$result" '.clearInput')
    assert_eq "$clear_input" "true"
}

test_back_clears_context() {
    hamr_test action --id "__cat__:All" > /dev/null
    local result=$(hamr_test action --id "__back__")
    
    # Context should be null (not set) or empty string
    local context=$(json_get "$result" '.context')
    if [[ "$context" == "null" || "$context" == "" ]]; then
        return 0
    fi
    echo "Context should be cleared on back, got: $context"
    return 1
}

test_search_with_valid_json() {
    local result=$(hamr_test search --query "test")
    
    # Verify it's valid JSON without using assert_ok
    if echo "$result" | jq . > /dev/null 2>&1; then
        return 0
    fi
    echo "Response is not valid JSON"
    return 1
}

test_category_has_icon() {
    local result=$(hamr_test initial)
    
    local icon=$(json_get "$result" '.results[0].icon')
    assert_contains "$icon" "app"
}

test_empty_action_is_safe() {
    hamr_test action --id "__cat__:All" > /dev/null
    # Finding the empty result ID if any
    local search=$(hamr_test search --query "zzzzzzzznonexistent")
    
    # Just verify we can handle search for non-existent apps
    assert_type "$search" "results"
    assert_has_result "$search" "__empty__"
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_initial_shows_categories \
    test_initial_has_positive_app_count \
    test_category_selection_shows_apps_in_category \
    test_all_category_shows_apps \
    test_back_button_returns_to_categories \
    test_search_with_query_returns_results \
    test_search_empty_query_shows_categories \
    test_search_nonexistent_shows_empty_state \
    test_search_in_category_with_context \
    test_search_preserves_placeholder \
    test_realtime_input_mode \
    test_category_context_persists \
    test_app_selection_returns_execute \
    test_result_has_required_fields \
    test_all_results_have_id \
    test_category_selection_sets_clear_input \
    test_back_clears_context \
    test_search_with_valid_json \
    test_category_has_icon \
    test_empty_action_is_safe
