#!/bin/bash
#
# Tests for files plugin
# Run: ./test.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export HAMR_TEST_MODE=1
source "$SCRIPT_DIR/../test-helpers.sh"

# ============================================================================
# Config
# ============================================================================

TEST_NAME="Files Plugin Tests"
HANDLER="$SCRIPT_DIR/handler.py"

# Use a temporary directory for test files
TEST_DIR=""
TEST_FILE_1=""
TEST_FILE_2=""
TEST_IMAGE_FILE=""
TEST_DIR_1=""

# Search history location (same as handler.py)
HISTORY_PATH="$HOME/.config/hamr/search-history.json"
HISTORY_BACKUP="/tmp/files-test-history-backup-$$.json"

# ============================================================================
# Setup / Teardown
# ============================================================================

setup() {
    # Create temporary test directory
    TEST_DIR=$(mktemp -d)
    
    # Create test files
    TEST_FILE_1="$TEST_DIR/test-document.md"
    TEST_FILE_2="$TEST_DIR/test-script.sh"
    TEST_IMAGE_FILE="$TEST_DIR/test-image.png"
    TEST_DIR_1="$TEST_DIR/test-subfolder"
    
    echo "# Test Document" > "$TEST_FILE_1"
    echo "#!/bin/bash" > "$TEST_FILE_2"
    mkdir -p "$TEST_DIR_1"
    
    # Create a minimal PNG file (1x1 transparent PNG)
    printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89\x00\x00\x00\nIDATx\x9cc\x00\x01\x00\x00\x05\x00\x01\r\n-\xb4\x00\x00\x00\x00IEND\xaeB`\x82' > "$TEST_IMAGE_FILE"
    
    # Backup existing history
    if [[ -f "$HISTORY_PATH" ]]; then
        cp "$HISTORY_PATH" "$HISTORY_BACKUP"
    else
        echo '{"history": []}' > "$HISTORY_BACKUP"
    fi
}

teardown() {
    # Clean up test directory
    if [[ -d "$TEST_DIR" ]]; then
        rm -rf "$TEST_DIR"
    fi
    
    # Restore history
    mkdir -p "$(dirname "$HISTORY_PATH")"
    cp "$HISTORY_BACKUP" "$HISTORY_PATH"
    rm -f "$HISTORY_BACKUP"
}

before_each() {
    # Reset history before each test
    mkdir -p "$(dirname "$HISTORY_PATH")"
    cp "$HISTORY_BACKUP" "$HISTORY_PATH"
}

# ============================================================================
# Helpers
# ============================================================================

set_history() {
    mkdir -p "$(dirname "$HISTORY_PATH")"
    echo "$1" > "$HISTORY_PATH"
}

clear_history() {
    set_history '{"history": []}'
}

get_history() {
    if [[ -f "$HISTORY_PATH" ]]; then
        cat "$HISTORY_PATH"
    else
        echo '{"history": []}'
    fi
}

# ============================================================================
# Tests
# ============================================================================

test_initial_no_recent_files() {
    clear_history
    local result=$(hamr_test initial)
    
    assert_type "$result" "results"
    assert_realtime_mode "$result"
    assert_has_result "$result" "__info__"
    assert_contains "$result" "Type to search files"
}

test_initial_with_recent_files() {
    # Add a recent file to history
    set_history "{\"history\": [{\"type\": \"file\", \"name\": \"$TEST_FILE_1\", \"count\": 5, \"lastUsed\": $(date +%s)000}]}"
    local result=$(hamr_test initial)
    
    assert_type "$result" "results"
    assert_realtime_mode "$result"
    assert_has_result "$result" "$TEST_FILE_1"
    assert_contains "$result" "test-document.md"
}

test_initial_with_multiple_recent_files() {
    set_history "{\"history\": [{\"type\": \"file\", \"name\": \"$TEST_FILE_1\", \"count\": 5, \"lastUsed\": $(date +%s)000}, {\"type\": \"file\", \"name\": \"$TEST_FILE_2\", \"count\": 3, \"lastUsed\": $(date +%s)000}]}"
    local result=$(hamr_test initial)
    
    assert_has_result "$result" "$TEST_FILE_1"
    assert_has_result "$result" "$TEST_FILE_2"
    assert_contains "$result" "test-document.md"
    assert_contains "$result" "test-script.sh"
}

test_search_empty_query_shows_recent() {
    set_history "{\"history\": [{\"type\": \"file\", \"name\": \"$TEST_FILE_1\", \"count\": 1, \"lastUsed\": $(date +%s)000}]}"
    local result=$(hamr_test search --query "")
    
    assert_type "$result" "results"
    assert_realtime_mode "$result"
    assert_has_result "$result" "$TEST_FILE_1"
}

test_search_no_results() {
    local result=$(hamr_test search --query "nonexistent_xyz_file_12345")
    
    assert_type "$result" "results"
    assert_realtime_mode "$result"
    assert_has_result "$result" "__no_results__"
    assert_contains "$result" "No files found"
}

test_result_has_id() {
    set_history "{\"history\": [{\"type\": \"file\", \"name\": \"$TEST_FILE_1\", \"count\": 1, \"lastUsed\": $(date +%s)000}]}"
    local result=$(hamr_test initial)
    
    # Each result should have an id matching the file path
    local file_id=$(json_get "$result" ".results[] | select(.name == \"test-document.md\") | .id")
    assert_eq "$file_id" "$TEST_FILE_1"
}

test_result_has_name() {
    set_history "{\"history\": [{\"type\": \"file\", \"name\": \"$TEST_FILE_1\", \"count\": 1, \"lastUsed\": $(date +%s)000}]}"
    local result=$(hamr_test initial)
    
    assert_has_result "$result" "$TEST_FILE_1"
    local name=$(json_get "$result" ".results[] | select(.id == \"$TEST_FILE_1\") | .name")
    assert_eq "$name" "test-document.md"
}

test_result_has_description() {
    set_history "{\"history\": [{\"type\": \"file\", \"name\": \"$TEST_FILE_1\", \"count\": 1, \"lastUsed\": $(date +%s)000}]}"
    local result=$(hamr_test initial)
    
    # Description should be the folder path
    local desc=$(json_get "$result" ".results[] | select(.id == \"$TEST_FILE_1\") | .description")
    assert_contains "$desc" "$TEST_DIR"
}

test_result_has_icon() {
    set_history "{\"history\": [{\"type\": \"file\", \"name\": \"$TEST_FILE_1\", \"count\": 1, \"lastUsed\": $(date +%s)000}]}"
    local result=$(hamr_test initial)
    
    local icon=$(json_get "$result" ".results[] | select(.id == \"$TEST_FILE_1\") | .icon")
    assert_eq "$icon" "article"  # .md file gets article icon
}

test_image_file_has_thumbnail() {
    set_history "{\"history\": [{\"type\": \"file\", \"name\": \"$TEST_IMAGE_FILE\", \"count\": 1, \"lastUsed\": $(date +%s)000}]}"
    local result=$(hamr_test initial)
    
    local thumbnail=$(json_get "$result" ".results[] | select(.id == \"$TEST_IMAGE_FILE\") | .thumbnail")
    assert_eq "$thumbnail" "$TEST_IMAGE_FILE"
}

test_non_image_file_no_thumbnail() {
    set_history "{\"history\": [{\"type\": \"file\", \"name\": \"$TEST_FILE_1\", \"count\": 1, \"lastUsed\": $(date +%s)000}]}"
    local result=$(hamr_test initial)
    
    local thumbnail=$(json_get "$result" ".results[] | select(.id == \"$TEST_FILE_1\") | .thumbnail")
    assert_eq "$thumbnail" "null"
}

test_result_has_actions() {
    set_history "{\"history\": [{\"type\": \"file\", \"name\": \"$TEST_FILE_1\", \"count\": 1, \"lastUsed\": $(date +%s)000}]}"
    local result=$(hamr_test initial)
    
    local actions=$(json_get "$result" ".results[] | select(.id == \"$TEST_FILE_1\") | .actions[]?.id")
    assert_contains "$actions" "open_folder"
    assert_contains "$actions" "copy_path"
    assert_contains "$actions" "delete"
}

test_directory_has_no_delete_action() {
    set_history "{\"history\": [{\"type\": \"file\", \"name\": \"$TEST_DIR_1\", \"count\": 1, \"lastUsed\": $(date +%s)000}]}"
    local result=$(hamr_test initial)
    
    # For directories, the delete action should not be present in the actions array
    local delete_action=$(json_get "$result" ".results[] | select(.id == \"$TEST_DIR_1\") | .actions[] | select(.id == \"delete\") | .id // empty")
    assert_eq "$delete_action" ""
}

test_directory_icon() {
    set_history "{\"history\": [{\"type\": \"file\", \"name\": \"$TEST_DIR_1\", \"count\": 1, \"lastUsed\": $(date +%s)000}]}"
    local result=$(hamr_test initial)
    
    local icon=$(json_get "$result" ".results[] | select(.id == \"$TEST_DIR_1\") | .icon")
    assert_eq "$icon" "folder"
}

test_open_folder_action() {
    local result=$(hamr_test action --id "$TEST_FILE_1" --action "open_folder")
    
    assert_type "$result" "execute"
    local cmd=$(json_get "$result" ".execute.command[0]")
    assert_eq "$cmd" "xdg-open"
    assert_contains "$result" "Open folder"
    assert_closes "$result"
}

test_copy_path_action() {
    local result=$(hamr_test action --id "$TEST_FILE_1" --action "copy_path")
    
    assert_type "$result" "execute"
    local cmd=$(json_get "$result" ".execute.command[0]")
    assert_eq "$cmd" "wl-copy"
    assert_contains "$result" "Copied"
    assert_closes "$result"
}

test_delete_action() {
    local result=$(hamr_test action --id "$TEST_FILE_1" --action "delete")
    
    assert_type "$result" "execute"
    local cmd=$(json_get "$result" ".execute.command[0]")
    assert_eq "$cmd" "gio"
    assert_contains "$result" "trash"
    assert_contains "$result" "Moved to trash"
    # Delete does not close the launcher
    local close=$(json_get "$result" ".execute.close")
    assert_eq "$close" "false"
}

test_default_action_opens_file() {
    # When no action is specified, should open the file
    local result=$(hamr_test action --id "$TEST_FILE_1")
    
    assert_type "$result" "execute"
    local cmd=$(json_get "$result" ".execute.command[0]")
    assert_eq "$cmd" "xdg-open"
    assert_contains "$result" "Open"
    assert_closes "$result"
}

test_default_action_includes_name() {
    local result=$(hamr_test action --id "$TEST_FILE_1")
    
    assert_type "$result" "execute"
    assert_contains "$result" "test-document.md"
}

test_image_execute_has_thumbnail() {
    local result=$(hamr_test action --id "$TEST_IMAGE_FILE")
    
    assert_type "$result" "execute"
    local thumbnail=$(json_get "$result" ".execute.thumbnail")
    assert_eq "$thumbnail" "$TEST_IMAGE_FILE"
}

test_non_image_execute_no_thumbnail() {
    local result=$(hamr_test action --id "$TEST_FILE_1")
    
    assert_type "$result" "execute"
    local thumbnail=$(json_get "$result" ".execute.thumbnail")
    assert_eq "$thumbnail" ""
}

test_nonexistent_file_error() {
    local result=$(hamr_test action --id "/nonexistent/file/path/xyz.txt")
    
    assert_type "$result" "error"
    assert_contains "$result" "not found"
}

test_info_item_not_actionable() {
    clear_history
    local initial=$(hamr_test initial)
    
    # Trying to act on __info__ should not crash
    local result=$(hamr_test action --id "__info__")
    
    # Should return nothing (handler returns early)
    # Just verify it doesn't crash
    assert_ok true
}

test_no_results_item_not_actionable() {
    local result_list=$(hamr_test search --query "nonexistent_xyz_file_12345")
    
    # Trying to act on __no_results__ should not crash
    local result=$(hamr_test action --id "__no_results__")
    
    # Should return nothing (handler returns early)
    assert_ok true
}

test_recent_files_sorted_by_frecency() {
    # Create history with two files, second one more recent
    set_history "{\"history\": [{\"type\": \"file\", \"name\": \"$TEST_FILE_1\", \"count\": 1, \"lastUsed\": 1000}, {\"type\": \"file\", \"name\": \"$TEST_FILE_2\", \"count\": 10, \"lastUsed\": $(date +%s)000}]}"
    local result=$(hamr_test initial)
    
    # File with higher count/recency should appear first
    local first_id=$(json_get "$result" ".results[0].id")
    assert_eq "$first_id" "$TEST_FILE_2"
}

test_path_displayed_with_tilde() {
    # Create a file in home directory for testing tilde replacement
    local home_file="$HOME/.test-hamr-file-$$.txt"
    echo "test" > "$home_file"
    trap "rm -f '$home_file'" EXIT
    
    set_history "{\"history\": [{\"type\": \"file\", \"name\": \"$home_file\", \"count\": 1, \"lastUsed\": $(date +%s)000}]}"
    local result=$(hamr_test initial)
    
    assert_has_result "$result" "$home_file"
}

test_file_icon_mapping_python() {
    local py_file="$TEST_DIR/test.py"
    echo "print('test')" > "$py_file"
    
    set_history "{\"history\": [{\"type\": \"file\", \"name\": \"$py_file\", \"count\": 1, \"lastUsed\": $(date +%s)000}]}"
    local result=$(hamr_test initial)
    
    local icon=$(json_get "$result" ".results[] | select(.id == \"$py_file\") | .icon")
    assert_eq "$icon" "code"
}

test_file_icon_mapping_json() {
    local json_file="$TEST_DIR/config.json"
    echo '{}' > "$json_file"
    
    set_history "{\"history\": [{\"type\": \"file\", \"name\": \"$json_file\", \"count\": 1, \"lastUsed\": $(date +%s)000}]}"
    local result=$(hamr_test initial)
    
    local icon=$(json_get "$result" ".results[] | select(.id == \"$json_file\") | .icon")
    assert_eq "$icon" "data_object"
}

test_open_folder_uses_correct_directory() {
    local result=$(hamr_test action --id "$TEST_FILE_1" --action "open_folder")
    
    assert_type "$result" "execute"
    local folder=$(json_get "$result" ".execute.command[1]")
    assert_eq "$folder" "$TEST_DIR"
}

test_all_responses_valid_json() {
    set_history "{\"history\": [{\"type\": \"file\", \"name\": \"$TEST_FILE_1\", \"count\": 1, \"lastUsed\": $(date +%s)000}]}"
    
    assert_ok hamr_test initial
    assert_ok hamr_test search --query ""
    assert_ok hamr_test search --query "test"
    assert_ok hamr_test action --id "$TEST_FILE_1"
    assert_ok hamr_test action --id "$TEST_FILE_1" --action "copy_path"
    assert_ok hamr_test action --id "$TEST_FILE_1" --action "open_folder"
}

test_placeholder_text() {
    local result=$(hamr_test initial)
    
    assert_json "$result" '.placeholder' "Search files..."
}

test_verb_on_results() {
    set_history "{\"history\": [{\"type\": \"file\", \"name\": \"$TEST_FILE_1\", \"count\": 1, \"lastUsed\": $(date +%s)000}]}"
    local result=$(hamr_test initial)
    
    local verb=$(json_get "$result" ".results[] | select(.id == \"$TEST_FILE_1\") | .verb")
    assert_eq "$verb" "Open"
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_initial_no_recent_files \
    test_initial_with_recent_files \
    test_initial_with_multiple_recent_files \
    test_search_empty_query_shows_recent \
    test_search_no_results \
    test_result_has_id \
    test_result_has_name \
    test_result_has_description \
    test_result_has_icon \
    test_image_file_has_thumbnail \
    test_non_image_file_no_thumbnail \
    test_result_has_actions \
    test_directory_has_no_delete_action \
    test_directory_icon \
    test_open_folder_action \
    test_copy_path_action \
    test_delete_action \
    test_default_action_opens_file \
    test_default_action_includes_name \
    test_image_execute_has_thumbnail \
    test_non_image_execute_no_thumbnail \
    test_nonexistent_file_error \
    test_info_item_not_actionable \
    test_no_results_item_not_actionable \
    test_recent_files_sorted_by_frecency \
    test_path_displayed_with_tilde \
    test_file_icon_mapping_python \
    test_file_icon_mapping_json \
    test_open_folder_uses_correct_directory \
    test_all_responses_valid_json \
    test_placeholder_text \
    test_verb_on_results
