#!/bin/bash
#
# Tests for create-plugin handler
# Run: ./test.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export HAMR_TEST_MODE=1
source "$SCRIPT_DIR/../test-helpers.sh"

# ============================================================================
# Config
# ============================================================================

TEST_NAME="Create Plugin Tests"
HANDLER="$SCRIPT_DIR/handler.py"

# Session file location (same as handler.py)
SESSION_FILE="$HOME/.cache/hamr/create-plugin-session.json"
SESSION_BACKUP="/tmp/create-plugin-session-backup-$$.json"

# ============================================================================
# Setup / Teardown
# ============================================================================

setup() {
    # Backup existing session
    if [[ -f "$SESSION_FILE" ]]; then
        mkdir -p "$(dirname "$SESSION_BACKUP")"
        cp "$SESSION_FILE" "$SESSION_BACKUP"
    else
        echo "{}" > "$SESSION_BACKUP"
    fi
}

teardown() {
    # Restore original session
    if [[ -f "$SESSION_BACKUP" ]]; then
        mkdir -p "$(dirname "$SESSION_FILE")"
        cp "$SESSION_BACKUP" "$SESSION_FILE"
        rm -f "$SESSION_BACKUP"
    else
        rm -f "$SESSION_FILE"
    fi
}

before_each() {
    # Clear session before each test (start fresh)
    rm -f "$SESSION_FILE"
}

# ============================================================================
# Helpers
# ============================================================================

clear_session() {
    rm -f "$SESSION_FILE"
}

has_session() {
    [[ -f "$SESSION_FILE" ]]
}

get_session_file() {
    cat "$SESSION_FILE" 2>/dev/null || echo "{}"
}

# ============================================================================
# Tests
# ============================================================================

test_initial_first_time_shows_help_only() {
    clear_session
    local result=$(hamr_test initial)
    
    assert_type "$result" "results"
    assert_submit_mode "$result"
    # First time should only show "How plugins work"
    assert_has_result "$result" "help"
    assert_result_count "$result" 1
}

test_initial_with_existing_conversation_shows_options() {
    clear_session
    # Simulate existing conversation by directly creating session with messages
    mkdir -p "$(dirname "$SESSION_FILE")"
    echo '{"messages": [{"role": "user", "content": "test", "ts": 1234567890}], "state": "initial"}' > "$SESSION_FILE"
    
    local result=$(hamr_test initial)
    
    assert_type "$result" "results"
    assert_submit_mode "$result"
    # With conversation history, should show continue/new/help options
    assert_has_result "$result" "continue"
    assert_has_result "$result" "new"
    assert_has_result "$result" "help"
    assert_result_count "$result" 3
}

test_initial_has_submit_mode() {
    clear_session
    local result=$(hamr_test initial)
    
    assert_submit_mode "$result"
}

test_initial_has_correct_placeholder() {
    clear_session
    local result=$(hamr_test initial)
    
    local placeholder=$(json_get "$result" '.placeholder')
    assert_contains "$placeholder" "Enter"
}

test_action_help_shows_protocol_card() {
    clear_session
    local result=$(hamr_test action --id "help")
    
    assert_type "$result" "card"
    local title=$(json_get "$result" '.card.title')
    assert_contains "$title" "Plugin Protocol"
}

test_action_help_has_submit_mode() {
    clear_session
    local result=$(hamr_test action --id "help")
    
    assert_submit_mode "$result"
}

test_action_help_includes_protocol_info() {
    clear_session
    local result=$(hamr_test action --id "help")
    
    assert_contains "$result" "manifest.json"
    assert_contains "$result" "handler.py"
    assert_contains "$result" "results"
}

test_action_new_clears_session() {
    clear_session
    # Create a session first
    mkdir -p "$(dirname "$SESSION_FILE")"
    echo '{"messages": [{"role": "user", "content": "old", "ts": 1234567890}], "state": "initial"}' > "$SESSION_FILE"
    
    hamr_test action --id "new" > /dev/null
    
    # Session should be cleared
    local messages=$(json_get "$(get_session_file)" '.messages | length')
    assert_eq "$messages" "0"
}

test_action_new_returns_results() {
    clear_session
    local result=$(hamr_test action --id "new")
    
    assert_type "$result" "results"
    assert_submit_mode "$result"
    assert_json "$result" '.clearInput' "true"
}

test_action_new_has_empty_results() {
    clear_session
    mkdir -p "$(dirname "$SESSION_FILE")"
    echo '{"messages": [{"role": "user", "content": "old", "ts": 1234567890}], "state": "initial"}' > "$SESSION_FILE"
    
    local result=$(hamr_test action --id "new")
    
    local count=$(get_result_count "$result")
    assert_eq "$count" "0"
}

test_action_continue_shows_conversation_card() {
    clear_session
    mkdir -p "$(dirname "$SESSION_FILE")"
    echo '{"messages": [{"role": "user", "content": "test message", "ts": 1234567890}], "state": "initial"}' > "$SESSION_FILE"
    
    local result=$(hamr_test action --id "continue")
    
    assert_type "$result" "card"
    assert_submit_mode "$result"
}

test_action_continue_without_messages_shows_results() {
    clear_session
    mkdir -p "$(dirname "$SESSION_FILE")"
    echo '{"messages": [], "state": "initial"}' > "$SESSION_FILE"
    
    local result=$(hamr_test action --id "continue")
    
    assert_type "$result" "results"
    assert_submit_mode "$result"
}

test_action_continue_clears_input() {
    clear_session
    mkdir -p "$(dirname "$SESSION_FILE")"
    echo '{"messages": [{"role": "user", "content": "test", "ts": 1234567890}], "state": "initial"}' > "$SESSION_FILE"
    
    local result=$(hamr_test action --id "continue")
    
    assert_json "$result" '.clearInput' "true"
}

test_search_empty_query_shows_placeholder() {
    clear_session
    local result=$(hamr_test search --query "")
    
    local placeholder=$(json_get "$result" '.placeholder')
    assert_contains "$placeholder" "Enter"
}

test_search_empty_query_returns_results() {
    clear_session
    local result=$(hamr_test search --query "")
    
    assert_type "$result" "results"
    local placeholder=$(json_get "$result" '.placeholder')
    assert_contains "$placeholder" "Enter"
}

test_response_has_valid_json() {
    clear_session
    local result=$(hamr_test initial)
    
    # If we got here, JSON parsing succeeded in assertions above
    local parsed=$(echo "$result" | jq '.' 2>&1)
    assert_contains "$parsed" "type"
}

test_initial_results_have_required_fields() {
    clear_session
    local result=$(hamr_test initial)
    
    # Results should have at least one item with required fields
    assert_has_result "$result" "help"
    local item=$(json_get "$result" '.results[] | select(.id == "help")')
    assert_contains "$item" "\"name\""
    assert_contains "$item" "\"icon\""
}

test_action_help_has_markdown_enabled() {
    clear_session
    local result=$(hamr_test action --id "help")
    
    assert_json "$result" '.card.markdown' "true"
}

test_action_help_has_content() {
    clear_session
    local result=$(hamr_test action --id "help")
    
    local content=$(json_get "$result" '.card.content')
    # Should have substantial content about the plugin protocol
    assert_contains "$content" "Plugins are folders"
}

test_all_responses_are_valid_json() {
    clear_session
    
    # Test that all major paths return valid JSON
    assert_ok hamr_test initial
    assert_ok hamr_test action --id "help"
    assert_ok hamr_test action --id "new"
}

test_session_file_created_on_action() {
    clear_session
    # Action with help should work and may create/modify session
    hamr_test action --id "help" > /dev/null
    
    # Session file may or may not exist after help, but if it does, it should be valid JSON
    if has_session; then
        local session=$(get_session_file)
        assert_contains "$session" "{"
    fi
}

test_initial_placeholder_mentions_enter() {
    clear_session
    local result=$(hamr_test initial)
    
    local placeholder=$(json_get "$result" '.placeholder')
    assert_contains "$placeholder" "Enter"
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_initial_first_time_shows_help_only \
    test_initial_with_existing_conversation_shows_options \
    test_initial_has_submit_mode \
    test_initial_has_correct_placeholder \
    test_action_help_shows_protocol_card \
    test_action_help_has_submit_mode \
    test_action_help_includes_protocol_info \
    test_action_new_clears_session \
    test_action_new_returns_results \
    test_action_new_has_empty_results \
    test_action_continue_shows_conversation_card \
    test_action_continue_without_messages_shows_results \
    test_action_continue_clears_input \
    test_search_empty_query_shows_placeholder \
    test_search_empty_query_returns_results \
    test_response_has_valid_json \
    test_initial_results_have_required_fields \
    test_action_help_has_markdown_enabled \
    test_action_help_has_content \
    test_all_responses_are_valid_json \
    test_session_file_created_on_action \
    test_initial_placeholder_mentions_enter
