#!/bin/bash
# Calculate plugin tests

export HAMR_TEST_MODE=1
source "$(dirname "$0")/../test-helpers.sh"

TEST_NAME="Calculate Plugin Tests"
HANDLER="$(dirname "$0")/handler.py"

test_initial_shows_prompt() {
    local result=$(hamr_test initial)
    assert_type "$result" "prompt"
}

test_match_basic_math() {
    local result=$(hamr_test raw --input '{"step": "match", "query": "2+2"}')
    assert_type "$result" "match"
    assert_json "$result" '.result.name' "4"
}

test_match_sqrt() {
    local result=$(hamr_test raw --input '{"step": "match", "query": "sqrt(16)"}')
    assert_type "$result" "match"
    assert_json "$result" '.result.name' "4"
}

test_match_temperature() {
    local result=$(hamr_test raw --input '{"step": "match", "query": "10c"}')
    assert_type "$result" "match"
    # Result should contain fahrenheit conversion (use json_get to decode unicode)
    local name=$(json_get "$result" '.result.name')
    assert_contains "$name" "°F"
}

test_match_percentage() {
    local result=$(hamr_test raw --input '{"step": "match", "query": "20% of 32"}')
    assert_type "$result" "match"
    assert_json "$result" '.result.name' "6.4"
}

test_search_shows_result() {
    local result=$(hamr_test search --query "2+2")
    assert_type "$result" "results"
    assert_has_result "$result" "calc_result"
}

test_action_copies_result() {
    local result=$(hamr_test raw --input '{"step": "action", "query": "2+2", "selected": {"id": "calc_result"}}')
    assert_type "$result" "execute"
    assert_contains "$result" "wl-copy"
}

test_initial_shows_prompt_when_no_history() {
    # In test mode, history is always empty
    local result=$(hamr_test initial)
    assert_type "$result" "prompt"
}

test_history_item_action() {
    # Test that history items can be copied
    local result=$(hamr_test raw --input '{"step": "action", "selected": {"id": "history:2+2"}}')
    assert_type "$result" "execute"
    assert_contains "$result" "wl-copy"
}

test_clear_history_action() {
    local result=$(hamr_test raw --input '{"step": "action", "selected": {"id": "__plugin__"}, "action": "clear_history"}')
    assert_type "$result" "prompt"
    assert_contains "$result" "cleared"
}

run_tests \
    test_initial_shows_prompt \
    test_match_basic_math \
    test_match_sqrt \
    test_match_temperature \
    test_match_percentage \
    test_search_shows_result \
    test_action_copies_result \
    test_initial_shows_prompt_when_no_history \
    test_history_item_action \
    test_clear_history_action
