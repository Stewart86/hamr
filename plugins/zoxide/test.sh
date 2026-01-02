#!/bin/bash
#
# Tests for zoxide plugin
# Run: ./test.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export HAMR_TEST_MODE=1
source "$SCRIPT_DIR/../test-helpers.sh"

# ============================================================================
# Config
# ============================================================================

TEST_NAME="Zoxide Plugin Tests"
HANDLER="$SCRIPT_DIR/handler.py"

# ============================================================================
# Tests
# ============================================================================

test_index_returns_items() {
    local result=$(hamr_test index)
    
    assert_type "$result" "index"
    
    local count=$(json_get "$result" '.items | length')
    assert_eq "$([ $count -gt 0 ] && echo 1 || echo 0)" "1" "Should have indexed items"
}

test_index_items_have_required_fields() {
    local result=$(hamr_test index)
    
    local first_id=$(json_get "$result" '.items[0].id')
    local first_name=$(json_get "$result" '.items[0].name')
    local first_desc=$(json_get "$result" '.items[0].description')
    
    assert_contains "$first_id" "zoxide:"
    assert_eq "$([ -n "$first_name" ] && echo 1 || echo 0)" "1" "Name should not be empty"
    assert_eq "$([ -n "$first_desc" ] && echo 1 || echo 0)" "1" "Description should not be empty"
}

test_index_items_have_execute() {
    local result=$(hamr_test index)
    
    local exec_cmd=$(json_get "$result" '.items[0].execute.command[0]')
    assert_eq "$([ -n "$exec_cmd" ] && echo 1 || echo 0)" "1" "Execute command should not be empty"
}

test_index_items_have_actions() {
    local result=$(hamr_test index)
    
    local action_count=$(json_get "$result" '.items[0].actions | length')
    assert_eq "$([ $action_count -ge 2 ] && echo 1 || echo 0)" "1" "Should have at least 2 actions"
}

test_index_items_have_files_action() {
    local result=$(hamr_test index)
    
    local files_action=$(json_get "$result" '.items[0].actions[] | select(.id == "files") | .name')
    assert_contains "$files_action" "Files"
}

test_index_items_have_copy_action() {
    local result=$(hamr_test index)
    
    local copy_action=$(json_get "$result" '.items[0].actions[] | select(.id == "copy") | .name')
    assert_contains "$copy_action" "Copy"
}

test_index_items_have_keywords() {
    local result=$(hamr_test index)
    
    local keywords=$(json_get "$result" '.items[0].keywords | length')
    assert_eq "$([ $keywords -gt 0 ] && echo 1 || echo 0)" "1" "Should have keywords from path"
}

test_index_items_have_icon() {
    local result=$(hamr_test index)
    
    local icon=$(json_get "$result" '.items[0].icon')
    assert_eq "$icon" "folder_special"
}

test_incremental_index_returns_mode() {
    local result=$(echo '{"step": "index", "mode": "incremental", "indexedIds": []}' | "$HANDLER")
    
    assert_type "$result" "index"
    assert_json "$result" '.mode' "incremental"
}

test_initial_step_returns_error() {
    local result=$(hamr_test initial)
    
    assert_type "$result" "error"
}

test_index_items_have_preview() {
    local result=$(hamr_test index)
    
    local preview_type=$(json_get "$result" '.items[0].preview.type')
    assert_eq "$preview_type" "text"
    
    local preview_content=$(json_get "$result" '.items[0].preview.content')
    assert_eq "$([ -n "$preview_content" ] && echo 1 || echo 0)" "1" "Preview content should not be empty"
}

test_index_items_have_preview_title() {
    local result=$(hamr_test index)
    
    local preview_title=$(json_get "$result" '.items[0].preview.title')
    assert_eq "$([ -n "$preview_title" ] && echo 1 || echo 0)" "1" "Preview title should not be empty"
}

test_index_valid_json() {
    local result=$(hamr_test index)
    
    if echo "$result" | jq . > /dev/null 2>&1; then
        return 0
    fi
    echo "Response is not valid JSON"
    return 1
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_index_returns_items \
    test_index_items_have_required_fields \
    test_index_items_have_execute \
    test_index_items_have_actions \
    test_index_items_have_files_action \
    test_index_items_have_copy_action \
    test_index_items_have_keywords \
    test_index_items_have_icon \
    test_incremental_index_returns_mode \
    test_initial_step_returns_error \
    test_index_items_have_preview \
    test_index_items_have_preview_title \
    test_index_valid_json
