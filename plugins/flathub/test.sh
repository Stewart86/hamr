#!/bin/bash
#
# Tests for flathub plugin
# Run: ./test.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export HAMR_TEST_MODE=1
source "$SCRIPT_DIR/../test-helpers.sh"

# ============================================================================
# Config
# ============================================================================

TEST_NAME="Flathub Plugin Tests"
HANDLER="$SCRIPT_DIR/handler.py"

# ============================================================================
# Tests - Initial State (Installed Apps)
# ============================================================================

test_initial_shows_installed_apps() {
    local result=$(hamr_test initial)
    
    assert_type "$result" "results"
    assert_realtime_mode "$result"
    assert_contains "$result" "Firefox"
    assert_contains "$result" "VLC"
}

test_initial_has_placeholder() {
    local result=$(hamr_test initial)
    
    assert_json "$result" '.placeholder' "Search installed apps..."
}

test_initial_apps_have_actions() {
    local result=$(hamr_test initial)
    
    assert_contains "$result" "uninstall"
    assert_contains "$result" "open_web"
}

test_initial_apps_have_thumbnail() {
    local result=$(hamr_test initial)
    
    assert_contains "$result" "thumbnail"
    assert_contains "$result" "file://"
}

test_initial_has_plugin_actions() {
    local result=$(hamr_test initial)
    
    assert_contains "$result" "pluginActions"
    assert_contains "$result" "search_new"
    assert_contains "$result" "Install New"
}

# ============================================================================
# Tests - Search Installed Apps
# ============================================================================

test_search_filters_installed_apps() {
    local result=$(hamr_test search --query "fire")
    
    assert_type "$result" "results"
    assert_contains "$result" "Firefox"
}

test_search_no_match_shows_empty() {
    local result=$(hamr_test search --query "nonexistent")
    
    assert_type "$result" "results"
    assert_has_result "$result" "__empty__"
    assert_contains "$result" "No installed apps match"
}

# ============================================================================
# Tests - Plugin Action: Install New
# ============================================================================

test_plugin_action_search_new() {
    local result=$(hamr_test action --id "__plugin__" --action "search_new")
    
    assert_type "$result" "results"
    assert_contains "$result" "context"
    assert_contains "$result" "__search_new__"
    assert_contains "$result" "Search Flathub"
}

test_search_new_mode_too_short() {
    local result=$(echo '{"step": "search", "query": "a", "context": "__search_new__"}' | HAMR_TEST_MODE=1 python3 "$HANDLER")
    
    assert_type "$result" "results"
    assert_has_result "$result" "__prompt__"
    assert_contains "$result" "at least 2 characters"
}

test_search_new_mode_returns_flathub_results() {
    local result=$(echo '{"step": "search", "query": "firefox", "context": "__search_new__"}' | HAMR_TEST_MODE=1 python3 "$HANDLER")
    
    assert_type "$result" "results"
    assert_realtime_mode "$result"
    assert_contains "$result" "Firefox"
    # Flathub results have https thumbnails
    assert_contains "$result" "https://"
    # Flathub results have developer info
    assert_contains "$result" "Mozilla"
}

test_search_new_mode_result_has_verb() {
    local result=$(echo '{"step": "search", "query": "firefox", "context": "__search_new__"}' | HAMR_TEST_MODE=1 python3 "$HANDLER")
    
    local verb=$(json_get "$result" '.results[0].verb')
    [[ "$verb" == "Install" || "$verb" == "Open" ]]
}

test_search_new_mode_result_has_actions() {
    local result=$(echo '{"step": "search", "query": "firefox", "context": "__search_new__"}' | HAMR_TEST_MODE=1 python3 "$HANDLER")
    
    assert_contains "$result" "actions"
    assert_contains "$result" "open_web"
    assert_contains "$result" "install"
}

# ============================================================================
# Tests - Back Navigation
# ============================================================================

test_back_returns_to_installed() {
    local result=$(hamr_test action --id "__back__")
    
    assert_type "$result" "results"
    assert_contains "$result" "Firefox"
    assert_contains "$result" "VLC"
    assert_json "$result" '.navigationDepth' "0"
}

# ============================================================================
# Tests - Actions
# ============================================================================

test_action_prompt_no_output() {
    local result=$(hamr_test action --id "__prompt__" 2>&1)
    
    assert_contains "$result" "no output"
}

test_action_empty_no_output() {
    local result=$(hamr_test action --id "__empty__" 2>&1)
    
    assert_contains "$result" "no output"
}

test_action_default_install_returns_execute() {
    local result=$(echo '{"step": "action", "selected": {"id": "org.example.App", "name": "Example App"}}' | HAMR_TEST_MODE=1 python3 "$HANDLER")
    
    assert_type "$result" "execute"
    assert_closes "$result"
    assert_contains "$result" "flatpak install"
    assert_contains "$result" "notify-send"
}

test_action_install_returns_execute() {
    local result=$(hamr_test action --id "org.example.App" --action "install")
    
    assert_type "$result" "execute"
    assert_closes "$result"
    assert_contains "$result" "flatpak install"
    assert_contains "$result" "notify-send"
}

test_action_uninstall_returns_execute() {
    local result=$(echo '{"step": "action", "action": "uninstall", "selected": {"id": "org.example.App", "name": "Example App"}}' | HAMR_TEST_MODE=1 python3 "$HANDLER")
    
    assert_type "$result" "execute"
    assert_closes "$result"
    assert_contains "$result" "flatpak uninstall"
    assert_contains "$result" "notify-send"
}

test_action_open_web_returns_execute() {
    local result=$(hamr_test action --id "org.example.App" --action "open_web")
    
    assert_type "$result" "execute"
    assert_closes "$result"
    assert_contains "$result" "xdg-open"
    assert_contains "$result" "flathub.org/apps/org.example.App"
}

# ============================================================================
# Tests - All Responses Valid
# ============================================================================

test_all_responses_valid() {
    assert_ok hamr_test initial
    assert_ok hamr_test search --query "fire"
    assert_ok hamr_test action --id "__plugin__" --action "search_new"
    assert_ok hamr_test action --id "__back__"
    assert_ok hamr_test action --id "org.example.App"
    assert_ok hamr_test action --id "org.example.App" --action "install"
    assert_ok hamr_test action --id "org.example.App" --action "open_web"
    assert_ok hamr_test action --id "org.example.App" --action "uninstall"
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_initial_shows_installed_apps \
    test_initial_has_placeholder \
    test_initial_apps_have_actions \
    test_initial_apps_have_thumbnail \
    test_initial_has_plugin_actions \
    test_search_filters_installed_apps \
    test_search_no_match_shows_empty \
    test_plugin_action_search_new \
    test_search_new_mode_too_short \
    test_search_new_mode_returns_flathub_results \
    test_search_new_mode_result_has_verb \
    test_search_new_mode_result_has_actions \
    test_back_returns_to_installed \
    test_action_prompt_no_output \
    test_action_empty_no_output \
    test_action_default_install_returns_execute \
    test_action_install_returns_execute \
    test_action_uninstall_returns_execute \
    test_action_open_web_returns_execute \
    test_all_responses_valid
