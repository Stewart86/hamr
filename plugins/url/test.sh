#!/usr/bin/env bash
# Test URL plugin

export HAMR_TEST_MODE=1
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../test-helpers.sh"

HANDLER="$SCRIPT_DIR/handler.py"
TEST_NAME="URL Plugin Tests"

# ============================================================================
# Tests
# ============================================================================

test_match_simple_domain() {
    local result=$(hamr_test match --query "github.com")
    assert_type "$result" "match"
    assert_json "$result" '.result.name' "https://github.com"
}

test_match_with_path() {
    local result=$(hamr_test match --query "docs.google.com/doc/123")
    assert_type "$result" "match"
    assert_json "$result" '.result.name' "https://docs.google.com/doc/123"
}

test_match_subdomain() {
    local result=$(hamr_test match --query "api.example.co.uk")
    assert_type "$result" "match"
    assert_json "$result" '.result.name' "https://api.example.co.uk"
}

test_copy_action() {
    local result=$(hamr_test raw --input '{"step": "action", "query": "github.com", "action": "copy"}')
    assert_type "$result" "execute"
    assert_json "$result" '.execute.command[0]' "wl-copy"
    assert_closes "$result"
}

test_open_action() {
    local result=$(hamr_test raw --input '{"step": "action", "query": "github.com"}')
    assert_type "$result" "execute"
    assert_json "$result" '.execute.command[0]' "xdg-open"
    assert_closes "$result"
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_match_simple_domain \
    test_match_with_path \
    test_match_subdomain \
    test_copy_action \
    test_open_action
