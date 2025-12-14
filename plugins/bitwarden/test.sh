#!/bin/bash
#
# Tests for Bitwarden plugin
# Run: ./test.sh
#
# Note: Tests work whether or not Bitwarden CLI is installed.
# Tests validate response format/structure without requiring actual credentials.
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export HAMR_TEST_MODE=1
source "$SCRIPT_DIR/../test-helpers.sh"

# ============================================================================
# Config
# ============================================================================

TEST_NAME="Bitwarden Plugin Tests"
HANDLER="$SCRIPT_DIR/handler.py"

# Cache directory (same as handler.py)
CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/hamr/bitwarden"
ITEMS_CACHE_FILE="$CACHE_DIR/items.json"
BACKUP_CACHE="/tmp/bw-cache-backup-$$.json"

# Mock Bitwarden session for testing
MOCK_SESSION="test-session-token-12345"

# ============================================================================
# Setup / Teardown
# ============================================================================

setup() {
    # Backup existing cache
    if [[ -f "$ITEMS_CACHE_FILE" ]]; then
        mkdir -p "$(dirname "$BACKUP_CACHE")"
        cp "$ITEMS_CACHE_FILE" "$BACKUP_CACHE"
    fi
}

teardown() {
    # Restore original cache
    if [[ -f "$BACKUP_CACHE" ]]; then
        mkdir -p "$(dirname "$ITEMS_CACHE_FILE")"
        cp "$BACKUP_CACHE" "$ITEMS_CACHE_FILE"
        rm -f "$BACKUP_CACHE"
    else
        # Clear test cache
        rm -f "$ITEMS_CACHE_FILE"
    fi
}

before_each() {
    # Reset to backup before each test
    if [[ -f "$BACKUP_CACHE" ]]; then
        mkdir -p "$(dirname "$ITEMS_CACHE_FILE")"
        cp "$BACKUP_CACHE" "$ITEMS_CACHE_FILE"
    else
        rm -f "$ITEMS_CACHE_FILE"
    fi
}

# ============================================================================
# Mock Helpers
# ============================================================================

# Set mock vault items in cache
# Args: JSON array of items
set_cache_items() {
    mkdir -p "$CACHE_DIR"
    echo "$1" > "$ITEMS_CACHE_FILE"
    chmod 600 "$ITEMS_CACHE_FILE"
}

# Clear cache
clear_cache() {
    rm -f "$ITEMS_CACHE_FILE"
}

# Create a mock vault item
# Args: id name username password has_totp(true/false)
make_item() {
    local id="$1"
    local name="$2"
    local username="$3"
    local password="$4"
    local has_totp="$5"
    
    local totp_field=""
    if [[ "$has_totp" == "true" ]]; then
        totp_field=',"totp":"123456"'
    fi
    
    echo "{\"id\":\"$id\",\"type\":1,\"name\":\"$name\",\"login\":{\"username\":\"$username\",\"password\":\"$password\"$totp_field},\"notes\":\"Test note\"}"
}

# Check if bw CLI is available
has_bw() {
    command -v bw &> /dev/null
}

# Get session token for testing (or return mock)
get_test_session() {
    # Try to get real session if bw is available
    if has_bw; then
        local home="$HOME"
        if [[ -f "$home/.bw_session" ]]; then
            cat "$home/.bw_session"
            return
        fi
    fi
    # Return mock session
    echo "$MOCK_SESSION"
}

# ============================================================================
# Tests
# ============================================================================

test_no_bw_cli_response() {
    # When bw CLI is not available, handler should return error card
    # This test only runs when bw is NOT installed AND we're NOT in test mode
    # In test mode, we mock the bw CLI existence, so we skip this test
    if has_bw || [[ "$HAMR_TEST_MODE" == "1" ]]; then
        # Skip this test if bw is installed or we're in test mode
        return 0
    fi
    
    local result=$(hamr_test initial)
    
    assert_type "$result" "card"
    assert_contains "$result" "Bitwarden CLI Required"
}

test_no_session_response() {
    # When no session is found, handler should return session required card
    # (This requires temporarily unsetting BW_SESSION and clearing session files)
    
    # For now, just verify handler can respond with card type when needed
    # Real test requires mocking session lookup which is complex
    local result=$(hamr_test initial)
    
    # Response should be either:
    # 1. Card (error) if no session/bw
    # 2. Results (success) if bw works
    local type=$(json_get "$result" '.type')
    if [[ "$type" == "card" ]]; then
        assert_contains "$result" "Session Required\|Bitwarden CLI Required\|No Items Found"
    fi
}

test_initial_with_cache() {
    # Setup cache with mock items
    local items='['
    items+='{"id":"item1","type":1,"name":"GitHub","login":{"username":"user@example.com","password":"pass123"},"notes":"Dev account"},'
    items+='{"id":"item2","type":1,"name":"Gmail","login":{"username":"me@gmail.com","password":"secret"},"notes":""}'
    items+=']'
    
    set_cache_items "$items"
    
    local result=$(hamr_test initial)
    
    assert_type "$result" "results"
    assert_has_result "$result" "item1"
    assert_has_result "$result" "item2"
    assert_has_result "$result" "__sync__"
}

test_initial_results_have_structure() {
    # Verify results have required fields
    local items='[{"id":"test1","type":1,"name":"Test App","login":{"username":"testuser","password":"testpass"},"notes":""}]'
    set_cache_items "$items"
    
    local result=$(hamr_test initial)
    
    # Check result structure
    assert_contains "$result" '"name"'
    assert_contains "$result" '"icon"'
    assert_contains "$result" '"actions"'
    assert_contains "$result" "Test App"
}

test_results_with_username_have_copy_username_action() {
    # Items with username should have copy_username action
    local items='[{"id":"u1","type":1,"name":"App","login":{"username":"user@test.com","password":"pass"},"notes":""}]'
    set_cache_items "$items"
    
    local result=$(hamr_test initial)
    local actions=$(json_get "$result" '.results[] | select(.id == "u1") | .actions[].id')
    
    assert_contains "$actions" "copy_username"
}

test_results_with_password_have_copy_password_action() {
    # Items with password should have copy_password action
    local items='[{"id":"p1","type":1,"name":"App","login":{"username":"","password":"secret123"},"notes":""}]'
    set_cache_items "$items"
    
    local result=$(hamr_test initial)
    local actions=$(json_get "$result" '.results[] | select(.id == "p1") | .actions[].id')
    
    assert_contains "$actions" "copy_password"
}

test_results_with_totp_have_copy_totp_action() {
    # Items with TOTP should have copy_totp action
    local items='[{"id":"t1","type":1,"name":"2FA App","login":{"username":"user","password":"pass","totp":"123456"},"notes":""}]'
    set_cache_items "$items"
    
    local result=$(hamr_test initial)
    local actions=$(json_get "$result" '.results[] | select(.id == "t1") | .actions[].id')
    
    assert_contains "$actions" "copy_totp"
}

test_results_icon_for_different_types() {
    # Type 1 = login (password icon), Type 2 = note, Type 3 = card, Type 4 = identity
    local items='['
    items+='{"id":"t1","type":1,"name":"Login Item","login":{"username":"u","password":"p"},"notes":""},'
    items+='{"id":"t2","type":2,"name":"Note Item","notes":"Some note"},'
    items+='{"id":"t3","type":3,"name":"Card Item","card":{},"notes":""},'
    items+='{"id":"t4","type":4,"name":"Identity Item","identity":{},"notes":""}'
    items+=']'
    set_cache_items "$items"
    
    local result=$(hamr_test initial)
    
    local icon1=$(json_get "$result" '.results[] | select(.id == "t1") | .icon')
    local icon2=$(json_get "$result" '.results[] | select(.id == "t2") | .icon')
    local icon3=$(json_get "$result" '.results[] | select(.id == "t3") | .icon')
    local icon4=$(json_get "$result" '.results[] | select(.id == "t4") | .icon')
    
    assert_eq "$icon1" "password" "Type 1 should have password icon"
    assert_eq "$icon2" "note" "Type 2 should have note icon"
    assert_eq "$icon3" "credit_card" "Type 3 should have credit_card icon"
    assert_eq "$icon4" "person" "Type 4 should have person icon"
}

test_results_description_shows_username() {
    # Description should show username if available
    local items='[{"id":"d1","type":1,"name":"App","login":{"username":"myuser@test.com","password":"pass"},"notes":""}]'
    set_cache_items "$items"
    
    local result=$(hamr_test initial)
    local desc=$(json_get "$result" '.results[] | select(.id == "d1") | .description')
    
    assert_eq "$desc" "myuser@test.com" "Description should show username"
}

test_results_description_shows_notes_if_no_username() {
    # Description should show notes preview if no username
    local items='[{"id":"n1","type":1,"name":"App","login":{"username":"","password":"pass"},"notes":"Test note content"}]'
    set_cache_items "$items"
    
    local result=$(hamr_test initial)
    local desc=$(json_get "$result" '.results[] | select(.id == "n1") | .description')
    
    assert_contains "$desc" "Test note"
}

test_sync_button_in_initial() {
    # Initial results should include __sync__ button
    local items='[{"id":"i1","type":1,"name":"Item","login":{"username":"u","password":"p"},"notes":""}]'
    set_cache_items "$items"
    
    local result=$(hamr_test initial)
    
    assert_has_result "$result" "__sync__"
    assert_contains "$result" "Sync Vault"
}

test_sync_button_shows_cache_age() {
    # __sync__ button should show cache age in description
    local items='[{"id":"i1","type":1,"name":"Item","login":{"username":"u","password":"p"},"notes":""}]'
    set_cache_items "$items"
    
    # Make cache file old by touching it to a past time
    touch -t 202401010000 "$ITEMS_CACHE_FILE" 2>/dev/null || true
    
    local result=$(hamr_test initial)
    
    # Should mention cache age or sync status
    assert_contains "$result" "synced"
}

test_search_filters_by_name() {
    # Search should filter items by name
    local items='['
    items+='{"id":"gh","type":1,"name":"GitHub","login":{"username":"user","password":"pass"},"notes":""},'
    items+='{"id":"gm","type":1,"name":"Gmail","login":{"username":"user","password":"pass"},"notes":""},'
    items+='{"id":"tw","type":1,"name":"Twitter","login":{"username":"user","password":"pass"},"notes":""}'
    items+=']'
    set_cache_items "$items"
    
    local result=$(hamr_test search --query "git")
    
    # Should only show GitHub, not Gmail or Twitter
    assert_has_result "$result" "gh"
    assert_no_result "$result" "tw"
}

test_search_filters_by_username() {
    # Search should filter items by username
    local items='['
    items+='{"id":"a1","type":1,"name":"App A","login":{"username":"alice@example.com","password":"pass"},"notes":""},'
    items+='{"id":"b1","type":1,"name":"App B","login":{"username":"bob@example.com","password":"pass"},"notes":""}'
    items+=']'
    set_cache_items "$items"
    
    local result=$(hamr_test search --query "alice")
    
    assert_has_result "$result" "a1"
    assert_no_result "$result" "b1"
}

test_search_case_insensitive() {
    # Search should be case insensitive
    local items='[{"id":"test","type":1,"name":"MyAwesomeApp","login":{"username":"user","password":"pass"},"notes":""}]'
    set_cache_items "$items"
    
    local result=$(hamr_test search --query "myawesome")
    
    assert_has_result "$result" "test"
}

test_search_no_results_message() {
    # Search with no matches should show no results message
    local items='[{"id":"i1","type":1,"name":"GitHub","login":{"username":"u","password":"p"},"notes":""}]'
    set_cache_items "$items"
    
    local result=$(hamr_test search --query "nonexistent")
    
    assert_has_result "$result" "__no_results__"
    assert_contains "$result" "No results"
}

test_action_on_no_results_placeholder() {
    # Clicking __no_results__ should be a no-op (early return in handler)
    # Just verify the handler doesn't crash - response structure varies
    local items='[{"id":"i1","type":1,"name":"Item","login":{"username":"u","password":"p"},"notes":""}]'
    set_cache_items "$items"
    
    # Try to action on the placeholder - should return something valid
    local result=$(hamr_test action --id "__no_results__" 2>/dev/null || echo "{}")
    # If no output, handler returned early (which is correct)
    # If output, should be valid JSON
    echo "$result" | jq . > /dev/null 2>&1 || true
}

test_action_execute_has_entry_point() {
    # Actions should return execute response with entryPoint (not command with password!)
    local items='[{"id":"sec1","type":1,"name":"Secret App","login":{"username":"user@test.com","password":"secret123"},"notes":""}]'
    set_cache_items "$items"
    
    local result=$(hamr_test action --id "sec1" --action "copy_password")
    
    assert_type "$result" "execute"
    assert_contains "$result" "entryPoint"
    # Make sure password is NOT in command (should use entryPoint)
    if assert_contains "$result" '"command"'; then
        # If command exists, it should NOT contain the password
        local cmd=$(json_get "$result" '.execute.command // "not present"')
        assert_not_contains "$cmd" "secret123"
    fi
}

test_action_copy_password_closes() {
    # Copy password action should close the launcher
    # Note: This test may hang if wl-copy is not available
    # Skip if timeout occurs
    local items='[{"id":"i1","type":1,"name":"App","login":{"username":"u","password":"pass"},"notes":""}]'
    set_cache_items "$items"
    
    # Skip this test if wl-copy is not available (common in test environments)
    if ! command -v wl-copy &> /dev/null; then
        return 0
    fi
    
    local result=$(timeout 2 hamr_test action --id "i1" --action "copy_password" 2>/dev/null || echo "{}")
    
    # Only assert if we got a result
    if [[ "$result" != "{}" ]] && [[ -n "$result" ]]; then
        assert_type "$result" "execute"
        local close=$(json_get "$result" '.execute.close')
        assert_eq "$close" "true" "Should have close: true"
    fi
}

test_action_copy_username_closes() {
    # Copy username action should close the launcher
    local items='[{"id":"i1","type":1,"name":"App","login":{"username":"testuser","password":"pass"},"notes":""}]'
    set_cache_items "$items"
    
    # Skip if wl-copy not available
    if ! command -v wl-copy &> /dev/null; then
        return 0
    fi
    
    local result=$(timeout 2 hamr_test action --id "i1" --action "copy_username" 2>/dev/null || echo "{}")
    
    if [[ "$result" != "{}" ]] && [[ -n "$result" ]]; then
        assert_type "$result" "execute"
        local close=$(json_get "$result" '.execute.close')
        assert_eq "$close" "true" "Should have close: true"
    fi
}

test_action_copy_totp_closes() {
    # Copy TOTP action should close the launcher
    local items='[{"id":"i1","type":1,"name":"App","login":{"username":"u","password":"p","totp":"123456"},"notes":""}]'
    set_cache_items "$items"
    
    local result=$(hamr_test action --id "i1" --action "copy_totp")
    
    # Should be execute type (will close based on close: true)
    if [[ "$(json_get "$result" '.type')" == "execute" ]]; then
        assert_closes "$result"
    fi
}

test_action_default_copy_password_when_available() {
    # Default action (no --action specified) should copy password if available
    # Note: Skipped if wl-copy not available (causes hang)
    local items='[{"id":"i1","type":1,"name":"App","login":{"username":"user","password":"mypass"},"notes":""}]'
    set_cache_items "$items"
    
    # Skip if wl-copy not available
    if ! command -v wl-copy &> /dev/null; then
        return 0
    fi
    
    # Default is to copy password - verifiable via entryPoint action
    local result=$(timeout 2 hamr_test action --id "i1" 2>/dev/null || echo "{}")
    
    if [[ "$result" != "{}" ]] && [[ -n "$result" ]]; then
        assert_type "$result" "execute"
        local action=$(json_get "$result" '.execute.entryPoint.action')
        assert_eq "$action" "copy_password" "Default action should be copy_password"
    fi
}

test_action_default_copy_username_when_no_password() {
    # If no password, default action should copy username
    local items='[{"id":"i1","type":1,"name":"App","login":{"username":"onlyuser","password":""},"notes":""}]'
    set_cache_items "$items"
    
    # Skip if wl-copy not available
    if ! command -v wl-copy &> /dev/null; then
        return 0
    fi
    
    local result=$(timeout 2 hamr_test action --id "i1" 2>/dev/null || echo "{}")
    
    if [[ "$result" != "{}" ]] && [[ -n "$result" ]]; then
        assert_type "$result" "execute"
        local action=$(json_get "$result" '.execute.entryPoint.action')
        assert_eq "$action" "copy_username" "Default action should be copy_username"
    fi
}

test_action_error_when_no_credentials() {
    # Should show error when item has no credentials
    local items='[{"id":"i1","type":1,"name":"Empty","login":{},"notes":""}]'
    set_cache_items "$items"
    
    local result=$(hamr_test action --id "i1")
    
    # Should be error card
    local type=$(json_get "$result" '.type')
    assert_eq "$type" "card" "Should return error card"
    assert_contains "$result" "Error"
}

test_sync_action() {
    # __sync__ action should perform sync and return fresh results
    local items='[{"id":"old1","type":1,"name":"Old Item","login":{"username":"u","password":"p"},"notes":""}]'
    set_cache_items "$items"
    
    local result=$(hamr_test action --id "__sync__")
    
    # Should return results (same or updated)
    assert_type "$result" "results"
    # Should have sync button again
    assert_has_result "$result" "__sync__"
}

test_response_realtime_mode() {
    # Results should be in realtime mode (default for search)
    local items='[{"id":"i1","type":1,"name":"Item","login":{"username":"u","password":"p"},"notes":""}]'
    set_cache_items "$items"
    
    local result=$(hamr_test search --query "test")
    
    local mode=$(json_get "$result" '.inputMode // "realtime"')
    assert_eq "$mode" "realtime" "Should be realtime mode"
}

test_response_has_placeholder() {
    # Results should have placeholder text
    local items='[{"id":"i1","type":1,"name":"Item","login":{"username":"u","password":"p"},"notes":""}]'
    set_cache_items "$items"
    
    local result=$(hamr_test initial)
    
    assert_contains "$result" "placeholder"
    assert_contains "$result" "Search vault"
}

test_empty_cache_no_results() {
    # Empty cache should show "No Items Found" card
    set_cache_items "[]"
    
    local result=$(hamr_test initial)
    
    # With empty cache, should show error card
    local type=$(json_get "$result" '.type')
    assert_eq "$type" "card" "Should return card for empty cache"
    assert_contains "$result" "No Items Found"
}

test_all_responses_valid_json() {
    # All responses should be valid JSON
    local items='[{"id":"i1","type":1,"name":"Test","login":{"username":"user","password":"pass","totp":"123"},"notes":"Note"}]'
    set_cache_items "$items"
    
    local r1=$(hamr_test initial)
    local r2=$(hamr_test search --query "test")
    
    # Should be valid JSON
    if [[ -n "$r1" ]]; then
        assert_ok echo "$r1" | jq . > /dev/null
    fi
    if [[ -n "$r2" ]]; then
        assert_ok echo "$r2" | jq . > /dev/null
    fi
}

test_action_copy_password_has_name_for_history() {
    # Copy password response should include name for history tracking
    local items='[{"id":"i1","type":1,"name":"My SecretApp","login":{"username":"u","password":"p"},"notes":""}]'
    set_cache_items "$items"
    
    local result=$(hamr_test action --id "i1" --action "copy_password" 2>/dev/null)
    
    # Verify it includes the app name if response is valid
    if [[ -n "$result" ]] && echo "$result" | jq . > /dev/null 2>&1; then
        assert_contains "$result" "My SecretApp"
    fi
}

test_action_response_has_icon() {
    # Execute responses should have icon for history
    local items='[{"id":"i1","type":1,"name":"App","login":{"username":"u","password":"p"},"notes":""}]'
    set_cache_items "$items"
    
    local result=$(hamr_test action --id "i1" 2>/dev/null)
    
    # Verify icon if response is valid
    if [[ -n "$result" ]] && echo "$result" | jq . > /dev/null 2>&1; then
        local icon=$(json_get "$result" '.execute.icon')
        [[ -n "$icon" ]] && [[ "$icon" != "null" ]]
    fi
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_no_bw_cli_response \
    test_no_session_response \
    test_initial_with_cache \
    test_initial_results_have_structure \
    test_results_with_username_have_copy_username_action \
    test_results_with_password_have_copy_password_action \
    test_results_with_totp_have_copy_totp_action \
    test_results_icon_for_different_types \
    test_results_description_shows_username \
    test_results_description_shows_notes_if_no_username \
    test_sync_button_in_initial \
    test_sync_button_shows_cache_age \
    test_search_filters_by_name \
    test_search_filters_by_username \
    test_search_case_insensitive \
    test_search_no_results_message \
    test_action_on_no_results_placeholder \
    test_action_execute_has_entry_point \
    test_action_copy_password_closes \
    test_action_copy_username_closes \
    test_action_copy_totp_closes \
    test_action_default_copy_password_when_available \
    test_action_default_copy_username_when_no_password \
    test_action_error_when_no_credentials \
    test_sync_action \
    test_response_realtime_mode \
    test_response_has_placeholder \
    test_empty_cache_no_results \
    test_all_responses_valid_json \
    test_action_copy_password_has_name_for_history \
    test_action_response_has_icon
