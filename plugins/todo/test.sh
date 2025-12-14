#!/bin/bash
#
# Tests for todo plugin
# Run: ./test.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export HAMR_TEST_MODE=1
source "$SCRIPT_DIR/../test-helpers.sh"

# ============================================================================
# Config
# ============================================================================

TEST_NAME="Todo Plugin Tests"
HANDLER="$SCRIPT_DIR/handler.py"

# Todo file location (same as handler.py)
STATE_DIR="${XDG_STATE_HOME:-$HOME/.local/state}"
TODO_FILE="$STATE_DIR/quickshell/user/todo.json"
BACKUP_FILE="/tmp/todo-test-backup-$$.json"

# ============================================================================
# Setup / Teardown
# ============================================================================

setup() {
    # Backup existing todos
    if [[ -f "$TODO_FILE" ]]; then
        cp "$TODO_FILE" "$BACKUP_FILE"
    else
        echo "[]" > "$BACKUP_FILE"
    fi
}

teardown() {
    # Restore original todos
    mkdir -p "$(dirname "$TODO_FILE")"
    cp "$BACKUP_FILE" "$TODO_FILE"
    rm -f "$BACKUP_FILE"
}

before_each() {
    # Reset to backup state before each test
    mkdir -p "$(dirname "$TODO_FILE")"
    cp "$BACKUP_FILE" "$TODO_FILE"
}

# ============================================================================
# Helpers
# ============================================================================

set_todos() {
    mkdir -p "$(dirname "$TODO_FILE")"
    echo "$1" > "$TODO_FILE"
}

clear_todos() {
    set_todos "[]"
}

get_todo_file() {
    cat "$TODO_FILE"
}

# ============================================================================
# Tests
# ============================================================================

test_initial_empty() {
    clear_todos
    local result=$(hamr_test initial)
    
    assert_type "$result" "results"
    assert_has_result "$result" "__add__"
    assert_has_result "$result" "__empty__"
}

test_initial_with_todos() {
    set_todos '[{"content": "Task 1", "done": false}, {"content": "Task 2", "done": true}]'
    local result=$(hamr_test initial)
    
    assert_result_count "$result" 3  # add + 2 todos
    assert_has_result "$result" "todo:0"
    assert_has_result "$result" "todo:1"
    assert_json "$result" '.results[] | select(.id == "todo:0") | .description' "Pending"
    assert_json "$result" '.results[] | select(.id == "todo:1") | .description' "Done"
}

test_search_filters() {
    set_todos '[{"content": "Buy milk", "done": false}, {"content": "Call mom", "done": false}]'
    local result=$(hamr_test search --query "milk")
    
    assert_result_count "$result" 2  # add option + 1 match
    assert_contains "$result" "Buy milk"
    assert_not_contains "$result" "Call mom"
}

test_search_shows_add_option() {
    clear_todos
    local result=$(hamr_test search --query "New task")
    
    assert_json "$result" '.results[0].name' "Add: New task"
}

test_add_via_search() {
    clear_todos
    local search_result=$(hamr_test search --query "My new task")
    local add_id=$(json_get "$search_result" '.results[0].id')
    
    local result=$(hamr_test action --id "$add_id")
    
    assert_contains "$result" "My new task"
    assert_contains "$(get_todo_file)" "My new task"
}

test_add_mode_entry() {
    clear_todos
    local result=$(hamr_test action --id "__add__")
    
    assert_submit_mode "$result"
    assert_json "$result" '.context' "__add_mode__"
}

test_add_mode_submit() {
    clear_todos
    hamr_test action --id "__add__" > /dev/null
    local result=$(hamr_test search --query "Task from add mode" --context "__add_mode__")
    
    assert_contains "$result" "Task from add mode"
    assert_contains "$(get_todo_file)" "Task from add mode"
}

test_toggle_default_action() {
    set_todos '[{"content": "Toggle me", "done": false}]'
    local result=$(hamr_test action --id "todo:0")
    
    assert_json "$result" '.results[] | select(.id == "todo:0") | .description' "Done"
    assert_eq "$(json_get "$(get_todo_file)" '.[0].done')" "true"
}

test_toggle_action_button() {
    set_todos '[{"content": "Toggle me", "done": false}]'
    local result=$(hamr_test action --id "todo:0" --action "toggle")
    
    assert_json "$result" '.results[] | select(.id == "todo:0") | .description' "Done"
}

test_delete() {
    set_todos '[{"content": "Delete me", "done": false}, {"content": "Keep me", "done": false}]'
    local result=$(hamr_test action --id "todo:0" --action "delete")
    
    assert_not_contains "$result" "Delete me"
    assert_contains "$result" "Keep me"
    assert_eq "$(json_get "$(get_todo_file)" 'length')" "1"
}

test_edit_mode_entry() {
    set_todos '[{"content": "Edit me", "done": false}]'
    local result=$(hamr_test action --id "todo:0" --action "edit")
    
    assert_submit_mode "$result"
    assert_json "$result" '.context' "__edit__:0"
}

test_edit_submit() {
    set_todos '[{"content": "Old content", "done": false}]'
    hamr_test action --id "todo:0" --action "edit" > /dev/null
    local result=$(hamr_test search --query "New content" --context "__edit__:0")
    
    assert_contains "$result" "New content"
    assert_not_contains "$result" "Old content"
    assert_eq "$(json_get "$(get_todo_file)" '.[0].content')" "New content"
}

test_back_from_add_mode() {
    set_todos '[{"content": "Existing", "done": false}]'
    hamr_test action --id "__add__" > /dev/null
    local result=$(hamr_test action --id "__back__")
    
    assert_realtime_mode "$result"
    assert_contains "$result" "Existing"
}

test_back_from_edit_mode() {
    set_todos '[{"content": "Original", "done": false}]'
    hamr_test action --id "todo:0" --action "edit" > /dev/null
    local result=$(hamr_test action --id "__back__")
    
    assert_contains "$result" "Original"
    assert_eq "$(json_get "$(get_todo_file)" '.[0].content')" "Original"
}

test_todo_has_actions() {
    set_todos '[{"content": "Task", "done": false}]'
    local result=$(hamr_test initial)
    local actions=$(json_get "$result" '.results[] | select(.id == "todo:0") | .actions[].id' | tr '\n' ',')
    
    assert_contains "$actions" "toggle"
    assert_contains "$actions" "edit"
    assert_contains "$actions" "delete"
}

test_empty_state_not_actionable() {
    clear_todos
    local result=$(hamr_test action --id "__empty__")
    
    assert_type "$result" "results"
}

test_all_responses_valid() {
    set_todos '[{"content": "Test", "done": false}]'
    
    assert_ok hamr_test initial
    assert_ok hamr_test search --query "test"
    assert_ok hamr_test action --id "__add__"
    assert_ok hamr_test action --id "todo:0"
    assert_ok hamr_test action --id "todo:0" --action "edit"
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_initial_empty \
    test_initial_with_todos \
    test_search_filters \
    test_search_shows_add_option \
    test_add_via_search \
    test_add_mode_entry \
    test_add_mode_submit \
    test_toggle_default_action \
    test_toggle_action_button \
    test_delete \
    test_edit_mode_entry \
    test_edit_submit \
    test_back_from_add_mode \
    test_back_from_edit_mode \
    test_todo_has_actions \
    test_empty_state_not_actionable \
    test_all_responses_valid
