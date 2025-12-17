#!/bin/bash
export HAMR_TEST_MODE=1

source "$(dirname "$0")/../test-helpers.sh"

TEST_NAME="Settings Plugin Tests"
HANDLER="$(dirname "$0")/handler.py"

test_initial_shows_categories() {
    local result=$(hamr_test initial)
    assert_type "$result" "results"
    assert_has_result "$result" "category:apps"
    assert_has_result "$result" "category:search"
    assert_has_result "$result" "category:appearance"
    assert_has_result "$result" "category:sizes"
    assert_has_result "$result" "category:fonts"
    assert_has_result "$result" "category:paths"
}

test_search_filters_all_settings() {
    local result=$(hamr_test search --query "terminal")
    assert_type "$result" "results"
    assert_contains "$result" "terminal"
}

test_search_launcher_finds_appearance() {
    local result=$(hamr_test search --query "launcher")
    assert_type "$result" "results"
    assert_contains "$result" "launcherXRatio"
    assert_contains "$result" "launcherYRatio"
}

test_category_navigation() {
    local result=$(hamr_test action --id "category:appearance")
    assert_type "$result" "results"
    assert_has_result "$result" "setting:appearance.backgroundTransparency"
    assert_has_result "$result" "setting:appearance.launcherXRatio"
    assert_json "$result" '.context' "category:appearance"
}

test_search_within_category() {
    local result=$(hamr_test search --query "trans" --context "category:appearance")
    assert_type "$result" "results"
    assert_contains "$result" "backgroundTransparency"
    assert_contains "$result" "contentTransparency"
}

test_setting_edit_shows_form() {
    local result=$(hamr_test action --id "setting:appearance.backgroundTransparency" --context "category:appearance")
    assert_type "$result" "form"
    assert_contains "$result" "backgroundTransparency"
}

test_back_from_category() {
    local result=$(hamr_test action --id "__back__" --context "category:appearance")
    assert_type "$result" "results"
    assert_has_result "$result" "category:apps"
}

run_tests \
    test_initial_shows_categories \
    test_search_filters_all_settings \
    test_search_launcher_finds_appearance \
    test_category_navigation \
    test_search_within_category \
    test_setting_edit_shows_form \
    test_back_from_category
