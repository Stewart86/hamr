#!/bin/bash
#
# Tests for quicklinks plugin
# Run: ./test.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export HAMR_TEST_MODE=1
source "$SCRIPT_DIR/../test-helpers.sh"

# ============================================================================
# Config
# ============================================================================

TEST_NAME="Quicklinks Plugin Tests"
HANDLER="$SCRIPT_DIR/handler.py"

# Quicklinks file location (same as handler.py)
QUICKLINKS_FILE="$HOME/.config/hamr/quicklinks.json"
BACKUP_FILE="/tmp/quicklinks-test-backup-$$.json"

# ============================================================================
# Setup / Teardown
# ============================================================================

setup() {
    # Backup existing quicklinks
    if [[ -f "$QUICKLINKS_FILE" ]]; then
        cp "$QUICKLINKS_FILE" "$BACKUP_FILE"
    else
        echo '{"quicklinks": []}' > "$BACKUP_FILE"
    fi
}

teardown() {
    # Restore original quicklinks
    mkdir -p "$(dirname "$QUICKLINKS_FILE")"
    cp "$BACKUP_FILE" "$QUICKLINKS_FILE"
    rm -f "$BACKUP_FILE"
}

before_each() {
    # Reset to backup state before each test
    mkdir -p "$(dirname "$QUICKLINKS_FILE")"
    cp "$BACKUP_FILE" "$QUICKLINKS_FILE"
}

# ============================================================================
# Helpers
# ============================================================================

set_quicklinks() {
    mkdir -p "$(dirname "$QUICKLINKS_FILE")"
    echo "$1" > "$QUICKLINKS_FILE"
}

clear_quicklinks() {
    set_quicklinks '{"quicklinks": []}'
}

get_quicklinks_file() {
    cat "$QUICKLINKS_FILE"
}

# ============================================================================
# Tests - Initial State
# ============================================================================

test_initial_empty() {
    clear_quicklinks
    local result=$(hamr_test initial)
    
    assert_type "$result" "results"
    assert_realtime_mode "$result"
    assert_has_result "$result" "__add__"
    assert_json "$result" '.results | length' "1"
}

test_initial_with_quicklinks() {
    set_quicklinks '{
        "quicklinks": [
            {"name": "Google", "url": "https://google.com?q={query}", "icon": "search"},
            {"name": "GitHub", "url": "https://github.com", "icon": "code"}
        ]
    }'
    local result=$(hamr_test initial)
    
    assert_type "$result" "results"
    assert_has_result "$result" "Google"
    assert_has_result "$result" "GitHub"
    assert_has_result "$result" "__add__"
    assert_result_count "$result" 3
}

test_initial_has_actions() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com", "icon": "search"}]}'
    local result=$(hamr_test initial)
    
    local actions=$(json_get "$result" '.results[] | select(.id == "Google") | .actions')
    assert_contains "$actions" "edit"
    assert_contains "$actions" "delete"
}

# ============================================================================
# Tests - Search / Filter
# ============================================================================

test_search_filters_by_name() {
    set_quicklinks '{
        "quicklinks": [
            {"name": "Google", "url": "https://google.com", "icon": "search"},
            {"name": "GitHub", "url": "https://github.com", "icon": "code"},
            {"name": "GitLab", "url": "https://gitlab.com", "icon": "code"}
        ]
    }'
    local result=$(hamr_test search --query "git")
    
    assert_contains "$result" "GitHub"
    assert_contains "$result" "GitLab"
    assert_not_contains "$result" "Google"
}

test_search_filters_by_alias() {
    set_quicklinks '{
        "quicklinks": [
            {
                "name": "Google",
                "url": "https://google.com",
                "icon": "search",
                "aliases": ["goog", "search"]
            }
        ]
    }'
    local result=$(hamr_test search --query "goog")
    
    assert_contains "$result" "Google"
}

test_search_shows_add_option() {
    clear_quicklinks
    local result=$(hamr_test search --query "NewLink")
    
    assert_has_result "$result" "__add__"
}

test_search_realtime_mode() {
    clear_quicklinks
    local result=$(hamr_test search --query "test")
    
    assert_realtime_mode "$result"
}

# ============================================================================
# Tests - Add Quicklink
# ============================================================================

test_add_action_opens_add_mode() {
    clear_quicklinks
    local result=$(hamr_test action --id "__add__")
    
    assert_type "$result" "results"
    assert_submit_mode "$result"
    assert_json "$result" '.context' "__add__"
    assert_json "$result" '.placeholder' "Enter quicklink name (Enter to confirm)"
}

test_add_submit_name_shows_url_prompt() {
    clear_quicklinks
    hamr_test action --id "__add__" > /dev/null
    local result=$(hamr_test search --query "MyLink" --context "__add__")
    
    assert_submit_mode "$result"
    assert_json "$result" '.context' "__add_name__:MyLink"
    assert_json "$result" '.placeholder' "Enter URL for 'MyLink' (Enter to save)"
    assert_contains "$result" "Back"
}

test_add_duplicate_name_shows_error() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com", "icon": "search"}]}'
    hamr_test action --id "__add__" > /dev/null
    local result=$(hamr_test search --query "Google" --context "__add__")
    
    assert_contains "$result" "already exists"
    assert_has_result "$result" "__error__"
}

test_add_url_saves_quicklink() {
    clear_quicklinks
    hamr_test action --id "__add__" > /dev/null
    hamr_test search --query "MyLink" --context "__add__" > /dev/null
    local result=$(hamr_test search --query "https://example.com" --context "__add_name__:MyLink")
    
    assert_realtime_mode "$result"
    assert_contains "$(get_quicklinks_file)" "MyLink"
    assert_contains "$(get_quicklinks_file)" "https://example.com"
}

test_add_url_adds_https_prefix() {
    clear_quicklinks
    hamr_test action --id "__add__" > /dev/null
    hamr_test search --query "MyLink" --context "__add__" > /dev/null
    hamr_test search --query "example.com" --context "__add_name__:MyLink" > /dev/null
    
    local url=$(json_get "$(get_quicklinks_file)" '.quicklinks[0].url')
    assert_eq "$url" "https://example.com"
}

test_add_url_clears_input() {
    clear_quicklinks
    hamr_test action --id "__add__" > /dev/null
    hamr_test search --query "MyLink" --context "__add__" > /dev/null
    local result=$(hamr_test search --query "https://example.com" --context "__add_name__:MyLink")
    
    assert_json "$result" '.clearInput' "true"
}

test_add_back_returns_to_menu() {
    clear_quicklinks
    hamr_test action --id "__add__" > /dev/null
    local result=$(hamr_test action --id "__back__")
    
    assert_realtime_mode "$result"
    assert_json "$result" '.context' ""
}

# ============================================================================
# Tests - Edit Quicklink
# ============================================================================

test_edit_action_opens_edit_mode() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com", "icon": "search"}]}'
    local result=$(hamr_test action --id "Google" --action "edit")
    
    assert_type "$result" "results"
    assert_submit_mode "$result"
    assert_json "$result" '.context' "__edit__:Google"
    assert_contains "$result" "Current: https://google.com"
}

test_edit_submit_updates_url() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com", "icon": "search"}]}'
    hamr_test action --id "Google" --action "edit" > /dev/null
    local result=$(hamr_test search --query "https://google.com/new" --context "__edit__:Google")
    
    local url=$(json_get "$(get_quicklinks_file)" '.quicklinks[0].url')
    assert_eq "$url" "https://google.com/new"
}

test_edit_url_adds_https_prefix() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com", "icon": "search"}]}'
    hamr_test action --id "Google" --action "edit" > /dev/null
    hamr_test search --query "newdomain.com" --context "__edit__:Google" > /dev/null
    
    local url=$(json_get "$(get_quicklinks_file)" '.quicklinks[0].url')
    assert_eq "$url" "https://newdomain.com"
}

test_edit_url_clears_input() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com", "icon": "search"}]}'
    hamr_test action --id "Google" --action "edit" > /dev/null
    local result=$(hamr_test search --query "https://newurl.com" --context "__edit__:Google")
    
    assert_json "$result" '.clearInput' "true"
}

test_edit_back_cancels_changes() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com", "icon": "search"}]}'
    hamr_test action --id "Google" --action "edit" > /dev/null
    local result=$(hamr_test action --id "__back__")
    
    local url=$(json_get "$(get_quicklinks_file)" '.quicklinks[0].url')
    assert_eq "$url" "https://google.com"
}

# ============================================================================
# Tests - Delete Quicklink
# ============================================================================

test_delete_action_removes_quicklink() {
    set_quicklinks '{
        "quicklinks": [
            {"name": "Google", "url": "https://google.com", "icon": "search"},
            {"name": "GitHub", "url": "https://github.com", "icon": "code"}
        ]
    }'
    local result=$(hamr_test action --id "Google" --action "delete")
    
    assert_not_contains "$result" "Google"
    assert_contains "$result" "GitHub"
}

test_delete_updates_file() {
    set_quicklinks '{
        "quicklinks": [
            {"name": "Google", "url": "https://google.com", "icon": "search"},
            {"name": "GitHub", "url": "https://github.com", "icon": "code"}
        ]
    }'
    hamr_test action --id "Google" --action "delete" > /dev/null
    
    assert_eq "$(json_get "$(get_quicklinks_file)" '.quicklinks | length')" "1"
    assert_contains "$(get_quicklinks_file)" "GitHub"
}

test_delete_shows_remaining() {
    set_quicklinks '{
        "quicklinks": [
            {"name": "Google", "url": "https://google.com", "icon": "search"}
        ]
    }'
    local result=$(hamr_test action --id "Google" --action "delete")
    
    assert_type "$result" "results"
    assert_realtime_mode "$result"
}

# ============================================================================
# Tests - URL Expansion and Search
# ============================================================================

test_quicklink_with_query_placeholder() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com/search?q={query}", "icon": "search"}]}'
    local result=$(hamr_test initial)
    
    local verb=$(json_get "$result" '.results[] | select(.id == "Google") | .verb')
    assert_eq "$verb" "Search"
}

test_quicklink_without_query_placeholder() {
    set_quicklinks '{"quicklinks": [{"name": "GitHub", "url": "https://github.com", "icon": "code"}]}'
    local result=$(hamr_test initial)
    
    local verb=$(json_get "$result" '.results[] | select(.id == "GitHub") | .verb')
    assert_eq "$verb" "Open"
}

test_select_quicklink_with_query_enters_search_mode() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com?q={query}", "icon": "search"}]}'
    local result=$(hamr_test action --id "Google")
    
    assert_type "$result" "results"
    assert_submit_mode "$result"
    assert_json "$result" '.context' "__search__:Google"
}

test_search_mode_expands_query() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com?q={query}", "icon": "search"}]}'
    hamr_test action --id "Google" > /dev/null
    local result=$(hamr_test search --query "test query" --context "__search__:Google")
    
    assert_type "$result" "execute"
    assert_contains "$result" "google.com?q=test"
}

test_search_query_url_encoded() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com?q={query}", "icon": "search"}]}'
    hamr_test action --id "Google" > /dev/null
    local result=$(hamr_test search --query "hello world" --context "__search__:Google")
    
    assert_contains "$result" "hello%20world"
}

test_search_mode_shows_open_direct_option() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com?q={query}", "icon": "search"}]}'
    local result=$(hamr_test action --id "Google")
    
    assert_has_result "$result" "__open_direct__:Google"
}

test_open_direct_removes_query_placeholder() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com?q={query}", "icon": "search"}]}'
    hamr_test action --id "Google" > /dev/null
    local result=$(hamr_test action --id "__open_direct__:Google")
    
    assert_type "$result" "execute"
    assert_not_contains "$result" "{query}"
    assert_contains "$result" "google.com?q="
}

test_select_quicklink_without_query_opens_directly() {
    set_quicklinks '{"quicklinks": [{"name": "GitHub", "url": "https://github.com", "icon": "code"}]}'
    local result=$(hamr_test action --id "GitHub")
    
    assert_type "$result" "execute"
    assert_closes "$result"
}

# ============================================================================
# Tests - Aliases and Description
# ============================================================================

test_quicklink_with_aliases_shows_description() {
    set_quicklinks '{
        "quicklinks": [
            {
                "name": "Google",
                "url": "https://google.com",
                "icon": "search",
                "aliases": ["goog", "search"]
            }
        ]
    }'
    local result=$(hamr_test initial)
    
    local description=$(json_get "$result" '.results[] | select(.id == "Google") | .description')
    assert_eq "$description" "goog, search"
}

test_search_by_alias() {
    set_quicklinks '{
        "quicklinks": [
            {
                "name": "Google",
                "url": "https://google.com",
                "icon": "search",
                "aliases": ["goog", "search"]
            }
        ]
    }'
    local result=$(hamr_test search --query "search")
    
    assert_has_result "$result" "Google"
}

# ============================================================================
# Tests - Context Persistence
# ============================================================================

test_context_persists_during_add() {
    clear_quicklinks
    hamr_test action --id "__add__" > /dev/null
    hamr_test search --query "MyLink" --context "__add__" > /dev/null
    local result=$(hamr_test search --query "" --context "__add_name__:MyLink")
    
    assert_json "$result" '.context' "__add_name__:MyLink"
}

test_context_persists_during_edit() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com", "icon": "search"}]}'
    hamr_test action --id "Google" --action "edit" > /dev/null
    local result=$(hamr_test search --query "" --context "__edit__:Google")
    
    assert_json "$result" '.context' "__edit__:Google"
}

test_context_cleared_on_back() {
    clear_quicklinks
    hamr_test action --id "__add__" > /dev/null
    local result=$(hamr_test action --id "__back__")
    
    assert_json "$result" '.context' ""
}

test_context_cleared_on_save() {
    clear_quicklinks
    hamr_test action --id "__add__" > /dev/null
    hamr_test search --query "MyLink" --context "__add__" > /dev/null
    local result=$(hamr_test search --query "https://example.com" --context "__add_name__:MyLink")
    
    assert_json "$result" '.context' ""
}

# ============================================================================
# Tests - Icon and Metadata
# ============================================================================

test_custom_icon_preserved() {
    set_quicklinks '{"quicklinks": [{"name": "GitHub", "url": "https://github.com", "icon": "github"}]}'
    local result=$(hamr_test initial)
    
    local icon=$(json_get "$result" '.results[] | select(.id == "GitHub") | .icon')
    assert_eq "$icon" "github"
}

test_default_icon_when_missing() {
    set_quicklinks '{"quicklinks": [{"name": "Test", "url": "https://example.com"}]}'
    local result=$(hamr_test initial)
    
    local icon=$(json_get "$result" '.results[] | select(.id == "Test") | .icon')
    assert_eq "$icon" "link"
}

# ============================================================================
# Tests - Encoding and Special Characters
# ============================================================================

test_https_prefix_not_duplicated() {
    clear_quicklinks
    hamr_test action --id "__add__" > /dev/null
    hamr_test search --query "MyLink" --context "__add__" > /dev/null
    hamr_test search --query "https://example.com" --context "__add_name__:MyLink" > /dev/null
    
    local url=$(json_get "$(get_quicklinks_file)" '.quicklinks[0].url')
    assert_eq "$url" "https://example.com"
}

test_http_protocol_preserved() {
    clear_quicklinks
    hamr_test action --id "__add__" > /dev/null
    hamr_test search --query "MyLink" --context "__add__" > /dev/null
    hamr_test search --query "http://example.com" --context "__add_name__:MyLink" > /dev/null
    
    local url=$(json_get "$(get_quicklinks_file)" '.quicklinks[0].url')
    assert_eq "$url" "http://example.com"
}

# ============================================================================
# Tests - Execute Response Properties
# ============================================================================

test_execute_has_close_flag() {
    set_quicklinks '{"quicklinks": [{"name": "GitHub", "url": "https://github.com", "icon": "code"}]}'
    local result=$(hamr_test action --id "GitHub")
    
    assert_closes "$result"
}

test_execute_search_has_close_flag() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com?q={query}", "icon": "search"}]}'
    hamr_test action --id "Google" > /dev/null
    local result=$(hamr_test search --query "test" --context "__search__:Google")
    
    assert_closes "$result"
}

test_execute_includes_name() {
    set_quicklinks '{"quicklinks": [{"name": "GitHub", "url": "https://github.com", "icon": "code"}]}'
    local result=$(hamr_test action --id "GitHub")
    
    assert_contains "$result" "Open GitHub"
}

test_execute_includes_icon() {
    set_quicklinks '{"quicklinks": [{"name": "GitHub", "url": "https://github.com", "icon": "github"}]}'
    local result=$(hamr_test action --id "GitHub")
    
    local icon=$(json_get "$result" '.execute.icon')
    assert_eq "$icon" "github"
}

# ============================================================================
# Tests - All Responses Valid
# ============================================================================

test_all_responses_valid() {
    set_quicklinks '{
        "quicklinks": [
            {"name": "Google", "url": "https://google.com?q={query}", "icon": "search"},
            {"name": "GitHub", "url": "https://github.com", "icon": "code"}
        ]
    }'
    
    assert_ok hamr_test initial
    assert_ok hamr_test search --query "git"
    assert_ok hamr_test action --id "Google"
    assert_ok hamr_test action --id "GitHub"
    assert_ok hamr_test action --id "__add__"
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_initial_empty \
    test_initial_with_quicklinks \
    test_initial_has_actions \
    test_search_filters_by_name \
    test_search_filters_by_alias \
    test_search_shows_add_option \
    test_search_realtime_mode \
    test_add_action_opens_add_mode \
    test_add_submit_name_shows_url_prompt \
    test_add_duplicate_name_shows_error \
    test_add_url_saves_quicklink \
    test_add_url_adds_https_prefix \
    test_add_url_clears_input \
    test_add_back_returns_to_menu \
    test_edit_action_opens_edit_mode \
    test_edit_submit_updates_url \
    test_edit_url_adds_https_prefix \
    test_edit_url_clears_input \
    test_edit_back_cancels_changes \
    test_delete_action_removes_quicklink \
    test_delete_updates_file \
    test_delete_shows_remaining \
    test_quicklink_with_query_placeholder \
    test_quicklink_without_query_placeholder \
    test_select_quicklink_with_query_enters_search_mode \
    test_search_mode_expands_query \
    test_search_query_url_encoded \
    test_search_mode_shows_open_direct_option \
    test_open_direct_removes_query_placeholder \
    test_select_quicklink_without_query_opens_directly \
    test_quicklink_with_aliases_shows_description \
    test_search_by_alias \
    test_context_persists_during_add \
    test_context_persists_during_edit \
    test_context_cleared_on_back \
    test_context_cleared_on_save \
    test_custom_icon_preserved \
    test_default_icon_when_missing \
    test_https_prefix_not_duplicated \
    test_http_protocol_preserved \
    test_execute_has_close_flag \
    test_execute_search_has_close_flag \
    test_execute_includes_name \
    test_execute_includes_icon \
    test_all_responses_valid
