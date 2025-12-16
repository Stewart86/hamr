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
    # Add is now in pluginActions
    assert_contains "$result" "pluginActions"
    assert_json "$result" '.pluginActions[0].id' "add"
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
    # Add is in pluginActions, not results
    assert_contains "$result" "pluginActions"
    assert_result_count "$result" 2
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

test_search_shows_add_in_plugin_actions() {
    clear_quicklinks
    local result=$(hamr_test search --query "NewLink")
    
    # Add is in pluginActions
    assert_contains "$result" "pluginActions"
    assert_json "$result" '.pluginActions[0].id' "add"
}

test_search_realtime_mode() {
    clear_quicklinks
    local result=$(hamr_test search --query "test")
    
    assert_realtime_mode "$result"
}

# ============================================================================
# Tests - Add Quicklink (Form API)
# ============================================================================

test_add_shows_form() {
    clear_quicklinks
    local result=$(hamr_test action --id "__plugin__" --action "add")
    
    assert_type "$result" "form"
    assert_json "$result" '.context' "__add__"
    assert_json "$result" '.form.title' "Add New Quicklink"
}

test_add_form_has_name_field() {
    clear_quicklinks
    local result=$(hamr_test action --id "__plugin__" --action "add")
    
    local name_field=$(json_get "$result" '.form.fields[] | select(.id == "name")')
    assert_contains "$name_field" '"type": "text"'
    assert_contains "$name_field" '"required": true'
}

test_add_form_has_url_field() {
    clear_quicklinks
    local result=$(hamr_test action --id "__plugin__" --action "add")
    
    local url_field=$(json_get "$result" '.form.fields[] | select(.id == "url")')
    assert_contains "$url_field" '"type": "text"'
    assert_contains "$url_field" '"required": true'
    assert_contains "$url_field" "{query}"
}

test_add_form_has_icon_field() {
    clear_quicklinks
    local result=$(hamr_test action --id "__plugin__" --action "add")
    
    local icon_field=$(json_get "$result" '.form.fields[] | select(.id == "icon")')
    assert_contains "$icon_field" '"type": "text"'
}

test_add_form_submission_saves_quicklink() {
    clear_quicklinks
    hamr_test action --id "__plugin__" --action "add" > /dev/null
    local result=$(hamr_test form --data '{"name": "MyLink", "url": "https://example.com", "icon": "link"}' --context "__add__")
    
    assert_type "$result" "results"
    local file=$(get_quicklinks_file)
    assert_contains "$file" "MyLink"
    assert_contains "$file" "https://example.com"
}

test_add_form_requires_name() {
    clear_quicklinks
    hamr_test action --id "__plugin__" --action "add" > /dev/null
    local result=$(hamr_test form --data '{"name": "", "url": "https://example.com"}' --context "__add__")
    
    assert_type "$result" "error"
    assert_contains "$result" "Name is required"
}

test_add_form_requires_url() {
    clear_quicklinks
    hamr_test action --id "__plugin__" --action "add" > /dev/null
    local result=$(hamr_test form --data '{"name": "MyLink", "url": ""}' --context "__add__")
    
    assert_type "$result" "error"
    assert_contains "$result" "URL is required"
}

test_add_form_name_already_exists() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com", "icon": "search"}]}'
    hamr_test action --id "__plugin__" --action "add" > /dev/null
    local result=$(hamr_test form --data '{"name": "Google", "url": "https://newurl.com"}' --context "__add__")
    
    assert_type "$result" "error"
    assert_contains "$result" "already exists"
}

test_add_form_adds_https_prefix() {
    clear_quicklinks
    hamr_test action --id "__plugin__" --action "add" > /dev/null
    hamr_test form --data '{"name": "MyLink", "url": "example.com"}' --context "__add__" > /dev/null
    
    local url=$(json_get "$(get_quicklinks_file)" '.quicklinks[0].url')
    assert_eq "$url" "https://example.com"
}

test_add_form_preserves_http() {
    clear_quicklinks
    hamr_test action --id "__plugin__" --action "add" > /dev/null
    hamr_test form --data '{"name": "MyLink", "url": "http://example.com"}' --context "__add__" > /dev/null
    
    local url=$(json_get "$(get_quicklinks_file)" '.quicklinks[0].url')
    assert_eq "$url" "http://example.com"
}

test_add_form_returns_to_list() {
    clear_quicklinks
    hamr_test action --id "__plugin__" --action "add" > /dev/null
    local result=$(hamr_test form --data '{"name": "MyLink", "url": "https://example.com"}' --context "__add__")
    
    assert_type "$result" "results"
    assert_has_result "$result" "MyLink"
    assert_json "$result" '.context' ""
}

test_add_form_cancel_returns_to_list() {
    clear_quicklinks
    hamr_test action --id "__plugin__" --action "add" > /dev/null
    local result=$(hamr_test action --id "__form_cancel__")
    
    assert_type "$result" "results"
    assert_contains "$result" "pluginActions"
}

test_add_form_clears_input() {
    clear_quicklinks
    hamr_test action --id "__plugin__" --action "add" > /dev/null
    local result=$(hamr_test form --data '{"name": "MyLink", "url": "https://example.com"}' --context "__add__")
    
    assert_json "$result" '.clearInput' "true"
}

test_add_form_default_icon() {
    clear_quicklinks
    hamr_test action --id "__plugin__" --action "add" > /dev/null
    hamr_test form --data '{"name": "MyLink", "url": "https://example.com", "icon": ""}' --context "__add__" > /dev/null
    
    local icon=$(json_get "$(get_quicklinks_file)" '.quicklinks[0].icon')
    assert_eq "$icon" "link"
}

test_add_form_custom_icon() {
    clear_quicklinks
    hamr_test action --id "__plugin__" --action "add" > /dev/null
    hamr_test form --data '{"name": "MyLink", "url": "https://example.com", "icon": "star"}' --context "__add__" > /dev/null
    
    local icon=$(json_get "$(get_quicklinks_file)" '.quicklinks[0].icon')
    assert_eq "$icon" "star"
}

# ============================================================================
# Tests - Edit Quicklink (Form API)
# ============================================================================

test_edit_shows_form() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com", "icon": "search"}]}'
    local result=$(hamr_test action --id "Google" --action "edit")
    
    assert_type "$result" "form"
    assert_json "$result" '.context' "__edit__:Google"
    assert_contains "$result" "Edit Quicklink: Google"
}

test_edit_form_prefills_url() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com", "icon": "search"}]}'
    local result=$(hamr_test action --id "Google" --action "edit")
    
    local url_default=$(json_get "$result" '.form.fields[] | select(.id == "url") | .default')
    assert_eq "$url_default" "https://google.com"
}

test_edit_form_prefills_icon() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com", "icon": "search"}]}'
    local result=$(hamr_test action --id "Google" --action "edit")
    
    local icon_default=$(json_get "$result" '.form.fields[] | select(.id == "icon") | .default')
    assert_eq "$icon_default" "search"
}

test_edit_form_saves_new_url() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com", "icon": "search"}]}'
    hamr_test action --id "Google" --action "edit" > /dev/null
    local result=$(hamr_test form --data '{"url": "https://google.com/new", "icon": "search"}' --context "__edit__:Google")
    
    assert_type "$result" "results"
    local url=$(json_get "$(get_quicklinks_file)" '.quicklinks[0].url')
    assert_eq "$url" "https://google.com/new"
}

test_edit_form_saves_new_icon() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com", "icon": "search"}]}'
    hamr_test action --id "Google" --action "edit" > /dev/null
    hamr_test form --data '{"url": "https://google.com", "icon": "star"}' --context "__edit__:Google" > /dev/null
    
    local icon=$(json_get "$(get_quicklinks_file)" '.quicklinks[0].icon')
    assert_eq "$icon" "star"
}

test_edit_form_adds_https_prefix() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com", "icon": "search"}]}'
    hamr_test action --id "Google" --action "edit" > /dev/null
    hamr_test form --data '{"url": "newdomain.com", "icon": "search"}' --context "__edit__:Google" > /dev/null
    
    local url=$(json_get "$(get_quicklinks_file)" '.quicklinks[0].url')
    assert_eq "$url" "https://newdomain.com"
}

test_edit_form_requires_url() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com", "icon": "search"}]}'
    hamr_test action --id "Google" --action "edit" > /dev/null
    local result=$(hamr_test form --data '{"url": "", "icon": "search"}' --context "__edit__:Google")
    
    assert_type "$result" "error"
    assert_contains "$result" "URL is required"
}

test_edit_form_returns_to_list() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com", "icon": "search"}]}'
    hamr_test action --id "Google" --action "edit" > /dev/null
    local result=$(hamr_test form --data '{"url": "https://google.com/new", "icon": "search"}' --context "__edit__:Google")
    
    assert_type "$result" "results"
    assert_has_result "$result" "Google"
}

test_edit_form_cancel() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com", "icon": "search"}]}'
    hamr_test action --id "Google" --action "edit" > /dev/null
    local result=$(hamr_test action --id "__form_cancel__")
    
    assert_type "$result" "results"
    # URL should remain unchanged
    local url=$(json_get "$(get_quicklinks_file)" '.quicklinks[0].url')
    assert_eq "$url" "https://google.com"
}

test_edit_form_clears_input() {
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com", "icon": "search"}]}'
    hamr_test action --id "Google" --action "edit" > /dev/null
    local result=$(hamr_test form --data '{"url": "https://google.com/new", "icon": "search"}' --context "__edit__:Google")
    
    assert_json "$result" '.clearInput' "true"
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
    assert_ok hamr_test action --id "__plugin__" --action "add"
    assert_ok hamr_test action --id "Google" --action "edit"
}

# ============================================================================
# Tests - Index (for main search integration)
# ============================================================================

test_index_returns_items() {
    set_quicklinks '{
        "quicklinks": [
            {"name": "GitHub", "url": "https://github.com", "icon": "code"},
            {"name": "Google", "url": "https://google.com?q={query}", "icon": "search"}
        ]
    }'
    local result=$(hamr_test index)
    
    assert_type "$result" "index"
    local count=$(json_get "$result" '.items | length')
    assert_eq "$count" "2"
}

test_index_item_without_query_has_execute_name() {
    # Items without {query} placeholder should have execute.name for history tracking
    set_quicklinks '{"quicklinks": [{"name": "GitHub", "url": "https://github.com", "icon": "code"}]}'
    local result=$(hamr_test index)
    
    local name=$(json_get "$result" '.items[0].execute.name')
    assert_contains "$name" "GitHub"
}

test_index_item_with_query_uses_entrypoint() {
    # Items with {query} placeholder should use entryPoint (no direct execute)
    set_quicklinks '{"quicklinks": [{"name": "Google", "url": "https://google.com?q={query}", "icon": "search"}]}'
    local result=$(hamr_test index)
    
    # Should have entryPoint instead of execute
    local has_entrypoint=$(json_get "$result" '.items[0].entryPoint != null')
    assert_eq "$has_entrypoint" "true"
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
    test_search_shows_add_in_plugin_actions \
    test_search_realtime_mode \
    test_add_shows_form \
    test_add_form_has_name_field \
    test_add_form_has_url_field \
    test_add_form_has_icon_field \
    test_add_form_submission_saves_quicklink \
    test_add_form_requires_name \
    test_add_form_requires_url \
    test_add_form_name_already_exists \
    test_add_form_adds_https_prefix \
    test_add_form_preserves_http \
    test_add_form_returns_to_list \
    test_add_form_cancel_returns_to_list \
    test_add_form_clears_input \
    test_add_form_default_icon \
    test_add_form_custom_icon \
    test_edit_shows_form \
    test_edit_form_prefills_url \
    test_edit_form_prefills_icon \
    test_edit_form_saves_new_url \
    test_edit_form_saves_new_icon \
    test_edit_form_adds_https_prefix \
    test_edit_form_requires_url \
    test_edit_form_returns_to_list \
    test_edit_form_cancel \
    test_edit_form_clears_input \
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
    test_custom_icon_preserved \
    test_default_icon_when_missing \
    test_execute_has_close_flag \
    test_execute_search_has_close_flag \
    test_execute_includes_name \
    test_execute_includes_icon \
    test_all_responses_valid \
    test_index_returns_items \
    test_index_item_without_query_has_execute_name \
    test_index_item_with_query_uses_entrypoint
