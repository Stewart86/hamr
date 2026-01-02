#!/bin/bash
#
# Tests for snippet plugin
# Run: ./test.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export HAMR_TEST_MODE=1
source "$SCRIPT_DIR/../test-helpers.sh"

# ============================================================================
# Config
# ============================================================================

TEST_NAME="Snippet Plugin Tests"
HANDLER="$SCRIPT_DIR/handler.py"

# Snippets file location (same as handler.py)
SNIPPETS_PATH="$HOME/.config/hamr/snippets.json"
BACKUP_FILE="/tmp/snippet-test-backup-$$.json"

# ============================================================================
# Setup / Teardown
# ============================================================================

setup() {
    # Backup existing snippets
    if [[ -f "$SNIPPETS_PATH" ]]; then
        cp "$SNIPPETS_PATH" "$BACKUP_FILE"
    else
        echo '{"snippets": []}' > "$BACKUP_FILE"
    fi
}

teardown() {
    # Restore original snippets
    mkdir -p "$(dirname "$SNIPPETS_PATH")"
    cp "$BACKUP_FILE" "$SNIPPETS_PATH"
    rm -f "$BACKUP_FILE"
}

before_each() {
    # Reset to backup state before each test
    mkdir -p "$(dirname "$SNIPPETS_PATH")"
    cp "$BACKUP_FILE" "$SNIPPETS_PATH"
}

# ============================================================================
# Helpers
# ============================================================================

set_snippets() {
    mkdir -p "$(dirname "$SNIPPETS_PATH")"
    echo "$1" > "$SNIPPETS_PATH"
}

clear_snippets() {
    set_snippets '{"snippets": []}'
}

get_snippets_file() {
    cat "$SNIPPETS_PATH"
}

# ============================================================================
# Tests - Initial State
# ============================================================================

test_initial_empty() {
    clear_snippets
    local result=$(hamr_test initial)
    
    assert_type "$result" "results"
    # Add is now in pluginActions
    assert_contains "$result" "pluginActions"
    assert_json "$result" '.pluginActions[0].id' "add"
    assert_json "$result" '.inputMode' "realtime"
}

test_initial_with_snippets() {
    set_snippets '{"snippets": [{"key": "hello", "value": "Hello, World!"}, {"key": "date", "value": "2024-01-01"}]}'
    local result=$(hamr_test initial)
    
    assert_type "$result" "results"
    assert_result_count "$result" 2  # 2 snippets (add is in pluginActions)
    assert_has_result "$result" "hello"
    assert_has_result "$result" "date"
}

test_initial_shows_description() {
    set_snippets '{"snippets": [{"key": "email", "value": "user@example.com"}]}'
    local result=$(hamr_test initial)
    
    assert_contains "$result" "user@example.com"
}

test_initial_truncates_long_values() {
    local long_value=$(printf 'A%.0s' {1..100})
    set_snippets "{\"snippets\": [{\"key\": \"long\", \"value\": \"$long_value\"}]}"
    local result=$(hamr_test initial)
    
    assert_contains "$result" "..."
}

test_initial_replaces_newlines_in_description() {
    set_snippets '{"snippets": [{"key": "multi", "value": "Line 1\nLine 2\nLine 3"}]}'
    local result=$(hamr_test initial)
    
    # Newlines should be replaced with spaces in description
    assert_contains "$result" "Line 1"
    # The entire preview should be on one line with spaces replacing newlines
    assert_contains "$result" "Line 1 Line 2 Line 3"
}

test_initial_snippets_have_actions() {
    set_snippets '{"snippets": [{"key": "test", "value": "test value"}]}'
    local result=$(hamr_test initial)
    
    local actions=$(json_get "$result" '.results[] | select(.id == "test") | .actions[].id' | tr '\n' ',')
    assert_contains "$actions" "copy"
    assert_contains "$actions" "edit"
    assert_contains "$actions" "delete"
}

# ============================================================================
# Tests - Search
# ============================================================================

test_search_filters_by_key() {
    set_snippets '{"snippets": [{"key": "hello", "value": "hi"}, {"key": "world", "value": "earth"}]}'
    local result=$(hamr_test search --query "hello")
    
    assert_has_result "$result" "hello"
    assert_no_result "$result" "world"
}

test_search_filters_by_value_preview() {
    set_snippets '{"snippets": [{"key": "key1", "value": "find me"}, {"key": "key2", "value": "other"}]}'
    local result=$(hamr_test search --query "find")
    
    assert_has_result "$result" "key1"
    assert_no_result "$result" "key2"
}

test_search_fuzzy_matching() {
    set_snippets '{"snippets": [{"key": "helloworld", "value": "test"}]}'
    local result=$(hamr_test search --query "hlo")
    
    assert_has_result "$result" "helloworld"
}

test_search_shows_add_in_plugin_actions() {
    set_snippets '{"snippets": [{"key": "test", "value": "value"}]}'
    local result=$(hamr_test search --query "xyz")
    
    # Add is in pluginActions
    assert_contains "$result" "pluginActions"
    assert_json "$result" '.pluginActions[0].id' "add"
}

test_search_empty_query_shows_all() {
    set_snippets '{"snippets": [{"key": "a", "value": "1"}, {"key": "b", "value": "2"}]}'
    local result=$(hamr_test search --query "")
    
    assert_has_result "$result" "a"
    assert_has_result "$result" "b"
}

test_search_case_insensitive() {
    set_snippets '{"snippets": [{"key": "Hello", "value": "World"}]}'
    local result=$(hamr_test search --query "hello")
    
    assert_has_result "$result" "Hello"
}

# ============================================================================
# Tests - Add Snippet (Form API)
# ============================================================================

test_add_shows_form() {
    clear_snippets
    # Add is now triggered via __plugin__ id with "add" action
    local result=$(hamr_test action --id "__plugin__" --action "add")
    
    assert_type "$result" "form"
    assert_json "$result" '.context' "__add__"
    assert_json "$result" '.form.title' "Add New Snippet"
}

test_add_form_has_key_field() {
    clear_snippets
    local result=$(hamr_test action --id "__plugin__" --action "add")
    
    local key_field=$(json_get "$result" '.form.fields[] | select(.id == "key")')
    assert_contains "$key_field" '"type": "text"'
    assert_contains "$key_field" '"required": true'
}

test_add_form_has_value_field() {
    clear_snippets
    local result=$(hamr_test action --id "__plugin__" --action "add")
    
    local value_field=$(json_get "$result" '.form.fields[] | select(.id == "value")')
    assert_contains "$value_field" '"type": "textarea"'
    assert_contains "$value_field" '"required": true'
}

test_add_form_submission_saves_snippet() {
    clear_snippets
    hamr_test action --id "__plugin__" --action "add" > /dev/null
    local result=$(hamr_test form --data '{"key": "mykey", "value": "myvalue"}' --context "__add__")
    
    assert_type "$result" "results"
    local snippets=$(get_snippets_file)
    assert_contains "$snippets" "mykey"
    assert_contains "$snippets" "myvalue"
}

test_add_form_requires_key() {
    clear_snippets
    hamr_test action --id "__plugin__" --action "add" > /dev/null
    local result=$(hamr_test form --data '{"key": "", "value": "somevalue"}' --context "__add__")
    
    assert_type "$result" "error"
    assert_contains "$result" "Key is required"
}

test_add_form_requires_value() {
    clear_snippets
    hamr_test action --id "__plugin__" --action "add" > /dev/null
    local result=$(hamr_test form --data '{"key": "somekey", "value": ""}' --context "__add__")
    
    assert_type "$result" "error"
    assert_contains "$result" "Value is required"
}

test_add_form_key_already_exists() {
    set_snippets '{"snippets": [{"key": "taken", "value": "value"}]}'
    hamr_test action --id "__plugin__" --action "add" > /dev/null
    local result=$(hamr_test form --data '{"key": "taken", "value": "newvalue"}' --context "__add__")
    
    assert_type "$result" "error"
    assert_contains "$result" "already exists"
}

test_add_form_multiline_value() {
    clear_snippets
    hamr_test action --id "__plugin__" --action "add" > /dev/null
    local result=$(hamr_test form --data '{"key": "multi", "value": "Line 1\nLine 2"}' --context "__add__")
    
    assert_type "$result" "results"
    local value=$(json_get "$(get_snippets_file)" '.snippets[0].value')
    assert_contains "$value" "Line 1"
}

test_add_form_returns_to_list() {
    clear_snippets
    hamr_test action --id "__plugin__" --action "add" > /dev/null
    local result=$(hamr_test form --data '{"key": "newsnippet", "value": "content"}' --context "__add__")
    
    assert_type "$result" "results"
    assert_has_result "$result" "newsnippet"
}

test_add_form_cancel_returns_to_list() {
    clear_snippets
    hamr_test action --id "__plugin__" --action "add" > /dev/null
    local result=$(hamr_test action --id "__form_cancel__")
    
    assert_type "$result" "results"
    assert_contains "$result" "pluginActions"
}

# ============================================================================
# Tests - Edit Snippet (Form API)
# ============================================================================

test_edit_shows_form() {
    set_snippets '{"snippets": [{"key": "edit_me", "value": "old value"}]}'
    local result=$(hamr_test action --id "edit_me" --action "edit")
    
    assert_type "$result" "form"
    assert_json "$result" '.context' "__edit__:edit_me"
    assert_contains "$result" "Edit Snippet"
}

test_edit_form_prefills_value() {
    set_snippets '{"snippets": [{"key": "test", "value": "current value"}]}'
    local result=$(hamr_test action --id "test" --action "edit")
    
    local value_default=$(json_get "$result" '.form.fields[] | select(.id == "value") | .default')
    assert_eq "$value_default" "current value"
}

test_edit_form_saves_new_value() {
    set_snippets '{"snippets": [{"key": "edit_test", "value": "old"}]}'
    hamr_test action --id "edit_test" --action "edit" > /dev/null
    local result=$(hamr_test form --data '{"value": "new"}' --context "__edit__:edit_test")
    
    assert_type "$result" "results"
    local value=$(json_get "$(get_snippets_file)" '.snippets[0].value')
    assert_eq "$value" "new"
}

test_edit_form_multiline_value() {
    set_snippets '{"snippets": [{"key": "escape_test", "value": "old"}]}'
    hamr_test action --id "escape_test" --action "edit" > /dev/null
    local result=$(hamr_test form --data '{"value": "Line 1\nLine 2"}' --context "__edit__:escape_test")
    
    assert_type "$result" "results"
    local value=$(json_get "$(get_snippets_file)" '.snippets[0].value')
    assert_contains "$value" "Line 1"
}

test_edit_form_returns_to_list() {
    set_snippets '{"snippets": [{"key": "edited", "value": "old"}]}'
    hamr_test action --id "edited" --action "edit" > /dev/null
    local result=$(hamr_test form --data '{"value": "new value"}' --context "__edit__:edited")
    
    assert_type "$result" "results"
    assert_has_result "$result" "edited"
}

test_edit_form_cancel() {
    set_snippets '{"snippets": [{"key": "cancel_edit", "value": "original"}]}'
    hamr_test action --id "cancel_edit" --action "edit" > /dev/null
    local result=$(hamr_test action --id "__form_cancel__")
    
    assert_type "$result" "results"
    # Value should remain unchanged
    local value=$(json_get "$(get_snippets_file)" '.snippets[0].value')
    assert_eq "$value" "original"
}

test_edit_form_requires_value() {
    set_snippets '{"snippets": [{"key": "empty_edit", "value": "something"}]}'
    hamr_test action --id "empty_edit" --action "edit" > /dev/null
    local result=$(hamr_test form --data '{"value": ""}' --context "__edit__:empty_edit")
    
    assert_type "$result" "error"
    assert_contains "$result" "Value is required"
}

# ============================================================================
# Tests - Copy Snippet
# ============================================================================

test_copy_action_executes() {
    set_snippets '{"snippets": [{"key": "copy_me", "value": "content"}]}'
    local result=$(hamr_test action --id "copy_me" --action "copy")
    
    assert_type "$result" "execute"
    assert_contains "$result" "wl-copy"
}

test_copy_uses_snippet_value() {
    set_snippets '{"snippets": [{"key": "test_copy", "value": "my content"}]}'
    local result=$(hamr_test action --id "test_copy" --action "copy")
    
    assert_contains "$result" "my content"
}

test_copy_closes_launcher() {
    set_snippets '{"snippets": [{"key": "close_copy", "value": "value"}]}'
    local result=$(hamr_test action --id "close_copy" --action "copy")
    
    assert_closes "$result"
}

test_copy_includes_notification() {
    set_snippets '{"snippets": [{"key": "notify", "value": "value"}]}'
    local result=$(hamr_test action --id "notify" --action "copy")
    
    assert_contains "$result" "notify"
}

# ============================================================================
# Tests - Delete Snippet
# ============================================================================

test_delete_removes_snippet() {
    set_snippets '{"snippets": [{"key": "delete_me", "value": "value"}, {"key": "keep_me", "value": "value"}]}'
    hamr_test action --id "delete_me" --action "delete" > /dev/null
    
    local snippets=$(get_snippets_file)
    assert_contains "$snippets" "keep_me"
    assert_not_contains "$snippets" "delete_me"
}

test_delete_returns_to_list() {
    set_snippets '{"snippets": [{"key": "del1", "value": "v"}, {"key": "keep", "value": "v"}]}'
    local result=$(hamr_test action --id "del1" --action "delete")
    
    assert_type "$result" "results"
    assert_has_result "$result" "keep"
}

test_delete_clears_input() {
    set_snippets '{"snippets": [{"key": "d1", "value": "v"}, {"key": "d2", "value": "v"}]}'
    local result=$(hamr_test action --id "d1" --action "delete")
    
    assert_json "$result" '.clearInput' "true"
}

test_delete_single_snippet() {
    set_snippets '{"snippets": [{"key": "only", "value": "value"}]}'
    hamr_test action --id "only" --action "delete" > /dev/null
    
    local count=$(json_get "$(get_snippets_file)" '.snippets | length')
    assert_eq "$count" "0"
}

# ============================================================================
# Tests - Direct Selection (Insert)
# ============================================================================

test_select_snippet_for_insertion() {
    set_snippets '{"snippets": [{"key": "insert_me", "value": "inserted content"}]}'
    local result=$(hamr_test action --id "insert_me")
    
    assert_type "$result" "execute"
    assert_contains "$result" "ydotool"
}

test_insert_closes_launcher() {
    set_snippets '{"snippets": [{"key": "close_insert", "value": "value"}]}'
    local result=$(hamr_test action --id "close_insert")
    
    assert_closes "$result"
}

test_insert_with_delay() {
    set_snippets '{"snippets": [{"key": "delayed", "value": "value"}]}'
    local result=$(hamr_test action --id "delayed")
    
    assert_contains "$result" "sleep"
}

test_insert_nonexistent_snippet() {
    clear_snippets
    local result=$(hamr_test action --id "nonexistent")
    
    assert_type "$result" "error"
}

# ============================================================================
# Tests - Edge Cases
# ============================================================================

test_snippet_with_special_characters() {
    set_snippets '{"snippets": [{"key": "special", "value": "hello $@!#%"}]}'
    local result=$(hamr_test initial)
    
    assert_has_result "$result" "special"
}

test_snippet_with_quotes() {
    set_snippets '{"snippets": [{"key": "quotes", "value": "\"quoted text\""}]}'
    local result=$(hamr_test initial)
    
    assert_has_result "$result" "quotes"
}

test_snippet_with_unicode() {
    set_snippets '{"snippets": [{"key": "unicode", "value": "Hello 世界 🌍"}]}'
    local result=$(hamr_test initial)
    
    assert_has_result "$result" "unicode"
}

test_very_long_key_name() {
    local long_key=$(printf 'k%.0s' {1..100})
    set_snippets "{\"snippets\": [{\"key\": \"$long_key\", \"value\": \"value\"}]}"
    local result=$(hamr_test initial)
    
    assert_has_result "$result" "$long_key"
}

test_multiple_snippets_same_prefix() {
    set_snippets '{"snippets": [{"key": "pre1", "value": "v1"}, {"key": "pre2", "value": "v2"}, {"key": "prefix", "value": "v3"}]}'
    local result=$(hamr_test search --query "pre")
    
    assert_result_count "$result" 3  # All 3 snippets (add is in pluginActions)
}

# ============================================================================
# Tests - Validity
# ============================================================================

test_all_responses_valid_json() {
    set_snippets '{"snippets": [{"key": "test", "value": "value"}]}'
    
    assert_ok hamr_test initial
    assert_ok hamr_test search --query "test"
    assert_ok hamr_test action --id "__plugin__" --action "add"
    assert_ok hamr_test action --id "test" --action "copy"
    assert_ok hamr_test action --id "test" --action "edit"
    assert_ok hamr_test action --id "test" --action "delete"
}

test_response_has_type() {
    set_snippets '{"snippets": [{"key": "t", "value": "v"}]}'
    local result=$(hamr_test initial)
    
    local type=$(json_get "$result" '.type')
    if [[ -z "$type" || "$type" == "null" ]]; then
        echo "Response missing 'type' field"
        return 1
    fi
}

test_snippet_with_date_variable() {
    set_snippets '{"snippets": [{"key": "today", "value": "Date: {date}"}]}'
    local result=$(hamr_test action --id "today" --action "copy")
    
    assert_type "$result" "execute"
    # Should contain expanded date in YYYY-MM-DD format
    assert_contains "$result" "$(date +%Y-%m-%d)"
}

test_snippet_with_user_variable() {
    set_snippets '{"snippets": [{"key": "sig", "value": "By {user}"}]}'
    local result=$(hamr_test action --id "sig" --action "copy")
    
    assert_type "$result" "execute"
    assert_contains "$result" "$USER"
}

test_snippet_with_clipboard_variable() {
    set_snippets '{"snippets": [{"key": "paste", "value": "Pasted: {clipboard}"}]}'
    local result=$(hamr_test action --id "paste" --action "copy")
    
    assert_type "$result" "execute"
    # In test mode, clipboard returns "clipboard_content"
    assert_contains "$result" "clipboard_content"
}

test_snippet_with_multiple_variables() {
    set_snippets '{"snippets": [{"key": "full", "value": "{user} on {date}"}]}'
    local result=$(hamr_test action --id "full" --action "copy")
    
    assert_type "$result" "execute"
    assert_contains "$result" "$USER"
    assert_contains "$result" "$(date +%Y-%m-%d)"
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_initial_empty \
    test_initial_with_snippets \
    test_initial_shows_description \
    test_initial_truncates_long_values \
    test_initial_replaces_newlines_in_description \
    test_initial_snippets_have_actions \
    test_search_filters_by_key \
    test_search_filters_by_value_preview \
    test_search_fuzzy_matching \
    test_search_shows_add_in_plugin_actions \
    test_search_empty_query_shows_all \
    test_search_case_insensitive \
    test_add_shows_form \
    test_add_form_has_key_field \
    test_add_form_has_value_field \
    test_add_form_submission_saves_snippet \
    test_add_form_requires_key \
    test_add_form_requires_value \
    test_add_form_key_already_exists \
    test_add_form_multiline_value \
    test_add_form_returns_to_list \
    test_add_form_cancel_returns_to_list \
    test_edit_shows_form \
    test_edit_form_prefills_value \
    test_edit_form_saves_new_value \
    test_edit_form_multiline_value \
    test_edit_form_returns_to_list \
    test_edit_form_cancel \
    test_edit_form_requires_value \
    test_copy_action_executes \
    test_copy_uses_snippet_value \
    test_copy_closes_launcher \
    test_copy_includes_notification \
    test_delete_removes_snippet \
    test_delete_returns_to_list \
    test_delete_clears_input \
    test_delete_single_snippet \
    test_select_snippet_for_insertion \
    test_insert_closes_launcher \
    test_insert_with_delay \
    test_insert_nonexistent_snippet \
    test_snippet_with_special_characters \
    test_snippet_with_quotes \
    test_snippet_with_unicode \
    test_very_long_key_name \
    test_multiple_snippets_same_prefix \
    test_all_responses_valid_json \
    test_response_has_type \
    test_snippet_with_date_variable \
    test_snippet_with_user_variable \
    test_snippet_with_clipboard_variable \
    test_snippet_with_multiple_variables
