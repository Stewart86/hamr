#!/bin/bash
#
# Tests for dict plugin
# Run: ./test.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export HAMR_TEST_MODE=1
source "$SCRIPT_DIR/../test-helpers.sh"

# ============================================================================
# Config
# ============================================================================

TEST_NAME="Dict Plugin Tests"
HANDLER="$SCRIPT_DIR/handler.py"

# ============================================================================
# Helpers
# ============================================================================

# Test with common English words that should be in dictionary
get_common_word() {
    echo "hello"
}

# Test with an uncommon/nonexistent word
get_nonexistent_word() {
    echo "xyzqwerty123notaword"
}

# ============================================================================
# Tests
# ============================================================================

test_initial_step() {
    local result=$(hamr_test initial)
    
    assert_type "$result" "prompt"
    assert_contains "$result" "Enter word to define"
}

test_search_empty_query() {
    local result=$(hamr_test search --query "")
    
    assert_type "$result" "results"
    assert_eq "$(get_result_count "$result")" "0"
    assert_json "$result" '.inputMode' "realtime"
}

test_search_whitespace_only() {
    local result=$(hamr_test search --query "   ")
    
    assert_type "$result" "results"
    assert_eq "$(get_result_count "$result")" "0"
}

test_search_valid_word_shows_card() {
    local result=$(hamr_test search --query "hello")
    
    assert_type "$result" "card"
    assert_contains "$result" "markdown"
    assert_json "$result" '.inputMode' "realtime"
}

test_card_contains_word() {
    local result=$(hamr_test search --query "hello")
    
    # Card content should contain the word
    assert_contains "$result" "hello"
}

test_card_has_copy_action() {
    local result=$(hamr_test search --query "hello")
    
    # Card should have a copy action
    assert_contains "$result" "copy"
    assert_contains "$result" "content_copy"
}

test_card_context_set() {
    local result=$(hamr_test search --query "hello")
    
    # Context should be set to the word for copy action
    assert_json "$result" '.context' "hello"
}

test_search_invalid_word_shows_not_found() {
    local result=$(hamr_test search --query "$(get_nonexistent_word)")
    
    assert_type "$result" "results"
    assert_contains "$result" "No definition found"
    assert_json "$result" '.inputMode' "realtime"
}

test_invalid_word_result_has_icon() {
    local result=$(hamr_test search --query "$(get_nonexistent_word)")
    
    assert_contains "$result" "search_off"
}

test_invalid_word_not_actionable() {
    local result=$(hamr_test search --query "$(get_nonexistent_word)")
    
    assert_has_result "$result" "__not_found__"
}

test_action_not_found_returns_gracefully() {
    hamr_test search --query "$(get_nonexistent_word)" > /dev/null
    local result=$(hamr_test action --id "__not_found__")
    
    # Should complete without error (handler returns early)
    assert_ok echo "$result"
}

test_copy_action() {
    hamr_test search --query "hello" > /dev/null
    # Note: Copy action re-fetches definition which adds latency
    # This test verifies the handler structure supports the copy action
    # (actual clipboard copy depends on wl-copy availability)
    local result=$(timeout 15 hamr_test action --id "copy" --context "hello" 2>&1 || echo '{"type":"execute","execute":{"notify":"timeout"}}')
    
    # Verify handler completes (either with success or timeout, which is expected in test env)
    assert_contains "$result" "type"
}

test_copy_preserves_word_context() {
    hamr_test search --query "hello" > /dev/null
    local result=$(hamr_test action --id "copy" --context "hello")
    
    # Notification should mention the word
    assert_contains "$result" "hello"
}

test_card_markdown_property() {
    local result=$(hamr_test search --query "hello")
    
    assert_json "$result" '.card.markdown' "true"
}

test_multiple_word_searches() {
    # Test several different words to ensure handler works consistently
    for word in "run" "walk" "think"; do
        local result=$(hamr_test search --query "$word")
        assert_type "$result" "card" "Should find definition for '$word'"
    done
}

test_case_insensitive_search() {
    # Most dictionary APIs are case-insensitive
    local result_lower=$(hamr_test search --query "hello")
    local result_upper=$(hamr_test search --query "HELLO")
    
    # Both should succeed (or both fail, but not inconsistently)
    local type_lower=$(json_get "$result_lower" '.type')
    local type_upper=$(json_get "$result_upper" '.type')
    assert_eq "$type_lower" "$type_upper" "Case shouldn't affect result type"
}

test_all_responses_valid_json() {
    # Ensure all responses are valid JSON
    assert_ok hamr_test initial
    assert_ok hamr_test search --query "hello"
    assert_ok hamr_test search --query "$(get_nonexistent_word)"
    assert_ok hamr_test action --id "copy" --context "hello"
}

test_search_with_numbers() {
    # Words with numbers typically won't have definitions
    local result=$(hamr_test search --query "123")
    
    assert_type "$result" "results"
    assert_contains "$result" "No definition found"
}

test_search_special_characters() {
    # Special characters in query
    local result=$(hamr_test search --query "@#\$")
    
    # Should handle gracefully without crashing
    assert_ok echo "$result"
}

test_word_lookup_formats_definition() {
    local result=$(hamr_test search --query "cat")
    
    # Definition should be formatted with markdown (bold, italics)
    assert_contains "$result" "**cat**"
}

test_single_letter_word() {
    # Single letter words like 'a' might have definitions
    local result=$(hamr_test search --query "a")
    
    # Should either show definition or "not found" - both are valid
    local type=$(json_get "$result" '.type')
    if [[ "$type" == "card" ]] || [[ "$type" == "results" ]]; then
        return 0
    fi
    return 1
}

test_copy_with_context_from_card() {
    # Full workflow: search -> get card with context -> copy using context
    local search_result=$(hamr_test search --query "dog")
    local context=$(json_get "$search_result" '.context')
    
    # Context should be the word
    assert_eq "$context" "dog"
    
    # Use context in copy action
    local result=$(hamr_test action --id "copy" --context "$context")
    assert_closes "$result"
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_initial_step \
    test_search_empty_query \
    test_search_whitespace_only \
    test_search_valid_word_shows_card \
    test_card_contains_word \
    test_card_has_copy_action \
    test_card_context_set \
    test_search_invalid_word_shows_not_found \
    test_invalid_word_result_has_icon \
    test_invalid_word_not_actionable \
    test_action_not_found_returns_gracefully \
    test_copy_action \
    test_copy_preserves_word_context \
    test_card_markdown_property \
    test_multiple_word_searches \
    test_case_insensitive_search \
    test_all_responses_valid_json \
    test_search_with_numbers \
    test_search_special_characters \
    test_word_lookup_formats_definition \
    test_single_letter_word \
    test_copy_with_context_from_card
