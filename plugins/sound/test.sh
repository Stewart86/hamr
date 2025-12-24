#!/bin/bash
export HAMR_TEST_MODE=1

source "$(dirname "$0")/../test-helpers.sh"

TEST_NAME="Sound Plugin Tests"
HANDLER="$(dirname "$0")/handler.py"

test_initial_shows_actions() {
    local result=$(hamr_test initial)
    assert_type "$result" "results"
    assert_has_result "$result" "vol-up"
    assert_has_result "$result" "vol-down"
    assert_has_result "$result" "mute-toggle"
}

test_initial_shows_volume() {
    local result=$(hamr_test initial)
    assert_contains "$result" "Volume:"
}

test_search_filters() {
    local result=$(hamr_test search --query "mute")
    assert_has_result "$result" "mute-toggle"
    assert_has_result "$result" "mic-mute-toggle"
}

test_action_vol_up() {
    local result=$(hamr_test action --id "vol-up")
    assert_type "$result" "execute"
}

test_action_mute_toggle() {
    local result=$(hamr_test action --id "mute-toggle")
    assert_type "$result" "execute"
}

test_plugin_action_vol_up() {
    local result=$(hamr_test action --id "__plugin__" --action "vol-up")
    assert_type "$result" "execute"
}

test_index_returns_items() {
    local result=$(hamr_test index)
    assert_type "$result" "index"
    assert_contains "$result" "sound:vol-up"
    assert_contains "$result" "sound:mute-toggle"
}

run_tests \
    test_initial_shows_actions \
    test_initial_shows_volume \
    test_search_filters \
    test_action_vol_up \
    test_action_mute_toggle \
    test_plugin_action_vol_up \
    test_index_returns_items
