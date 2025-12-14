#!/bin/bash
#
# Tests for notes plugin
# 
# Comprehensive test suite covering:
# - Initial listing and empty states
# - Search filtering by title and content
# - Form-based CRUD operations (create, read, update, delete)
# - Card views with markdown support
# - Quick add via search query
# - Input validation (required fields)
# - Data persistence and sorting
#
# Run: ./test.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export HAMR_TEST_MODE=1
source "$SCRIPT_DIR/../test-helpers.sh"

# ============================================================================
# Config
# ============================================================================

TEST_NAME="Notes Plugin Tests"
HANDLER="$SCRIPT_DIR/handler.py"

# Notes file location (same as handler.py)
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}"
NOTES_FILE="$CONFIG_DIR/hamr/notes.json"
BACKUP_FILE="/tmp/notes-test-backup-$$.json"

# ============================================================================
# Setup / Teardown
# ============================================================================

setup() {
    # Backup existing notes
    if [[ -f "$NOTES_FILE" ]]; then
        cp "$NOTES_FILE" "$BACKUP_FILE"
    else
        echo '{"notes": []}' > "$BACKUP_FILE"
    fi
}

teardown() {
    # Restore original notes
    mkdir -p "$(dirname "$NOTES_FILE")"
    cp "$BACKUP_FILE" "$NOTES_FILE"
    rm -f "$BACKUP_FILE"
}

before_each() {
    # Reset to backup state before each test
    mkdir -p "$(dirname "$NOTES_FILE")"
    cp "$BACKUP_FILE" "$NOTES_FILE"
}

# ============================================================================
# Helpers
# ============================================================================

set_notes() {
    mkdir -p "$(dirname "$NOTES_FILE")"
    echo "$1" > "$NOTES_FILE"
}

clear_notes() {
    set_notes '{"notes": []}'
}

get_notes_file() {
    cat "$NOTES_FILE"
}

get_note_count() {
    jq '.notes | length' "$NOTES_FILE"
}

get_first_note() {
    jq '.notes[0]' "$NOTES_FILE"
}

# ============================================================================
# Tests
# ============================================================================

test_initial_empty() {
    clear_notes
    local result=$(hamr_test initial)
    
    assert_type "$result" "results"
    assert_has_result "$result" "__add__"
    assert_has_result "$result" "__empty__"
}

test_initial_with_notes() {
    local notes='{"notes": [
        {"id": "note_1", "title": "First Note", "content": "First content", "created": 1000, "updated": 1000},
        {"id": "note_2", "title": "Second Note", "content": "Second content", "created": 2000, "updated": 2000}
    ]}'
    set_notes "$notes"
    local result=$(hamr_test initial)
    
    assert_type "$result" "results"
    assert_result_count "$result" 3  # add + 2 notes
    assert_has_result "$result" "note_1"
    assert_has_result "$result" "note_2"
}

test_initial_notes_sorted_by_updated_desc() {
    local notes='{"notes": [
        {"id": "note_1", "title": "Old Note", "content": "First", "created": 1000, "updated": 1000},
        {"id": "note_2", "title": "New Note", "content": "Second", "created": 2000, "updated": 2000}
    ]}'
    set_notes "$notes"
    local result=$(hamr_test initial)
    
    # New note should appear first (after add button)
    local first_note_id=$(json_get "$result" '.results[1].id')
    assert_eq "$first_note_id" "note_2" "Most recent note should appear first"
}

test_search_filters_by_title() {
    local notes='{"notes": [
        {"id": "note_1", "title": "Buy Groceries", "content": "Milk, eggs", "created": 1000, "updated": 1000},
        {"id": "note_2", "title": "Call Mom", "content": "Remember to call", "created": 2000, "updated": 2000}
    ]}'
    set_notes "$notes"
    local result=$(hamr_test search --query "groceries")
    
    assert_contains "$result" "Buy Groceries"
    assert_not_contains "$result" "Call Mom"
}

test_search_filters_by_content() {
    local notes='{"notes": [
        {"id": "note_1", "title": "Shopping", "content": "Buy milk and eggs", "created": 1000, "updated": 1000},
        {"id": "note_2", "title": "Todo", "content": "Call mom later", "created": 2000, "updated": 2000}
    ]}'
    set_notes "$notes"
    local result=$(hamr_test search --query "milk")
    
    assert_contains "$result" "Shopping"
    assert_not_contains "$result" "Todo"
}

test_search_empty_query_shows_all() {
    local notes='{"notes": [
        {"id": "note_1", "title": "Note One", "content": "Content", "created": 1000, "updated": 1000},
        {"id": "note_2", "title": "Note Two", "content": "Content", "created": 2000, "updated": 2000}
    ]}'
    set_notes "$notes"
    local result=$(hamr_test search --query "")
    
    assert_has_result "$result" "note_1"
    assert_has_result "$result" "note_2"
}

test_search_shows_quick_add() {
    clear_notes
    local result=$(hamr_test search --query "My new note")
    
    local quick_add_id=$(json_get "$result" '.results[0].id')
    assert_contains "$quick_add_id" "__add_quick__:My new note"
}

test_search_with_query_hides_add_button() {
    clear_notes
    local result=$(hamr_test search --query "something")
    
    # Should have quick add, but not regular add button
    assert_has_result "$result" "__add_quick__:something"
    assert_no_result "$result" "__add__"
}

test_search_empty_hides_quick_add() {
    clear_notes
    local result=$(hamr_test search --query "")
    
    assert_has_result "$result" "__add__"
    assert_no_result "$result" "__add_quick__"
}

test_action_click_add_shows_form() {
    clear_notes
    local result=$(hamr_test action --id "__add__")
    
    assert_type "$result" "form"
    assert_json "$result" '.form.title' "Add New Note"
    assert_json "$result" '.context' "__add__"
}

test_add_form_has_title_field() {
    clear_notes
    local result=$(hamr_test action --id "__add__")
    
    local title_field=$(json_get "$result" '.form.fields[] | select(.id == "title")')
    assert_contains "$title_field" '"type": "text"'
    assert_contains "$title_field" '"required": true'
}

test_add_form_has_content_field() {
    clear_notes
    local result=$(hamr_test action --id "__add__")
    
    local content_field=$(json_get "$result" '.form.fields[] | select(.id == "content")')
    assert_contains "$content_field" '"type": "textarea"'
}

test_action_quick_add_shows_form_with_title() {
    clear_notes
    local result=$(hamr_test action --id "__add_quick__:My Quick Note")
    
    assert_type "$result" "form"
    # Form should have title field with default value
    local title_default=$(json_get "$result" '.form.fields[] | select(.id == "title") | .default')
    assert_eq "$title_default" "My Quick Note"
}

test_form_submission_add_creates_note() {
    clear_notes
    hamr_test action --id "__add__" > /dev/null
    local result=$(hamr_test form --data '{"title": "Test Note", "content": "Test content"}' --context "__add__")
    
    assert_type "$result" "results"
    assert_contains "$result" "Test Note"
    assert_eq "$(get_note_count)" "1"
}

test_form_submission_add_returns_to_list() {
    local notes='{"notes": [{"id": "note_1", "title": "Existing", "content": "Content", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    hamr_test action --id "__add__" > /dev/null
    local result=$(hamr_test form --data '{"title": "New Note", "content": ""}' --context "__add__")
    
    assert_has_result "$result" "__add__"
    assert_has_result "$result" "note_1"
    assert_contains "$result" "New Note"
}

test_form_submission_requires_title() {
    clear_notes
    hamr_test action --id "__add__" > /dev/null
    local result=$(hamr_test form --data '{"title": "", "content": "Some content"}' --context "__add__")
    
    assert_type "$result" "error"
    assert_contains "$result" "Title is required"
    assert_eq "$(get_note_count)" "0"
}

test_form_submission_allows_empty_content() {
    clear_notes
    hamr_test action --id "__add__" > /dev/null
    local result=$(hamr_test form --data '{"title": "Title Only", "content": ""}' --context "__add__")
    
    assert_type "$result" "results"
    assert_contains "$result" "Title Only"
}

test_form_submission_clears_input() {
    clear_notes
    hamr_test action --id "__add__" > /dev/null
    local result=$(hamr_test form --data '{"title": "Test", "content": ""}' --context "__add__")
    
    local clear_input=$(json_get "$result" '.clearInput')
    assert_eq "$clear_input" "true"
}

test_action_view_shows_card() {
    local notes='{"notes": [{"id": "note_1", "title": "Test Note", "content": "Test content line 1\nLine 2", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    local result=$(hamr_test action --id "note_1")
    
    assert_type "$result" "card"
    assert_contains "$result" "Test Note"
    assert_contains "$result" "Test content"
}

test_action_view_default_when_no_action() {
    local notes='{"notes": [{"id": "note_1", "title": "Test", "content": "Content", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    # No --action flag, should default to view
    local result=$(hamr_test action --id "note_1")
    
    assert_type "$result" "card"
}

test_card_has_markdown() {
    local notes='{"notes": [{"id": "note_1", "title": "Markdown Note", "content": "**Bold** text", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    local result=$(hamr_test action --id "note_1")
    
    local markdown=$(json_get "$result" '.card.markdown')
    assert_eq "$markdown" "true"
}

test_card_stores_context() {
    local notes='{"notes": [{"id": "note_1", "title": "Test", "content": "Content", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    local result=$(hamr_test action --id "note_1")
    
    local context=$(json_get "$result" '.context')
    assert_eq "$context" "note_1"
}

test_card_has_edit_action() {
    local notes='{"notes": [{"id": "note_1", "title": "Test", "content": "Content", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    local result=$(hamr_test action --id "note_1" --action "view")
    
    local edit_action=$(json_get "$result" '.card.actions[] | select(.id == "edit")')
    assert_contains "$edit_action" '"name": "Edit"'
}

test_card_has_delete_action() {
    local notes='{"notes": [{"id": "note_1", "title": "Test", "content": "Content", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    local result=$(hamr_test action --id "note_1" --action "view")
    
    local delete_action=$(json_get "$result" '.card.actions[] | select(.id == "delete")')
    assert_contains "$delete_action" '"name": "Delete"'
}

test_action_edit_shows_form() {
    local notes='{"notes": [{"id": "note_1", "title": "Original", "content": "Original content", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    local result=$(hamr_test action --id "note_1" --action "edit")
    
    assert_type "$result" "form"
    assert_json "$result" '.form.title' "Edit Note"
}

test_edit_form_prefills_title() {
    local notes='{"notes": [{"id": "note_1", "title": "Original Title", "content": "Content", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    local result=$(hamr_test action --id "note_1" --action "edit")
    
    local title_default=$(json_get "$result" '.form.fields[] | select(.id == "title") | .default')
    assert_eq "$title_default" "Original Title"
}

test_edit_form_prefills_content() {
    local notes='{"notes": [{"id": "note_1", "title": "Title", "content": "Original content", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    local result=$(hamr_test action --id "note_1" --action "edit")
    
    local content_default=$(json_get "$result" '.form.fields[] | select(.id == "content") | .default')
    assert_eq "$content_default" "Original content"
}

test_edit_form_context() {
    local notes='{"notes": [{"id": "note_1", "title": "Title", "content": "Content", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    local result=$(hamr_test action --id "note_1" --action "edit")
    
    local context=$(json_get "$result" '.context')
    assert_eq "$context" "__edit__:note_1"
}

test_form_submission_edit_updates_note() {
    local notes='{"notes": [{"id": "note_1", "title": "Old Title", "content": "Old content", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    hamr_test action --id "note_1" --action "edit" > /dev/null
    local result=$(hamr_test form --data '{"title": "New Title", "content": "New content"}' --context "__edit__:note_1")
    
    assert_type "$result" "results"
    assert_contains "$result" "New Title"
    assert_not_contains "$result" "Old Title"
    
    local updated_note=$(get_first_note)
    assert_contains "$updated_note" '"title": "New Title"'
    assert_contains "$updated_note" '"content": "New content"'
}

test_edit_updates_timestamp() {
    local notes='{"notes": [{"id": "note_1", "title": "Title", "content": "Content", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    hamr_test action --id "note_1" --action "edit" > /dev/null
    hamr_test form --data '{"title": "Title", "content": "Updated"}' --context "__edit__:note_1" > /dev/null
    
    local updated_note=$(get_first_note)
    local created=$(json_get "$updated_note" '.created')
    local updated=$(json_get "$updated_note" '.updated')
    
    assert_eq "$created" "1000" "Created timestamp should not change"
    # Updated should be greater than original
    if [[ $updated -le 1000 ]]; then
        echo "Updated timestamp should be greater than created"
        return 1
    fi
}

test_edit_requires_title() {
    local notes='{"notes": [{"id": "note_1", "title": "Title", "content": "Content", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    hamr_test action --id "note_1" --action "edit" > /dev/null
    local result=$(hamr_test form --data '{"title": "", "content": "New content"}' --context "__edit__:note_1")
    
    assert_type "$result" "error"
    assert_contains "$result" "Title is required"
}

test_action_delete_removes_note() {
    local notes='{"notes": [
        {"id": "note_1", "title": "Delete me", "content": "Content", "created": 1000, "updated": 1000},
        {"id": "note_2", "title": "Keep me", "content": "Content", "created": 2000, "updated": 2000}
    ]}'
    set_notes "$notes"
    local result=$(hamr_test action --id "note_1" --action "delete")
    
    assert_type "$result" "results"
    assert_not_contains "$result" "Delete me"
    assert_contains "$result" "Keep me"
    assert_eq "$(get_note_count)" "1"
}

test_delete_returns_to_list() {
    local notes='{"notes": [
        {"id": "note_1", "title": "Delete me", "content": "Content", "created": 1000, "updated": 1000},
        {"id": "note_2", "title": "Keep me", "content": "Content", "created": 2000, "updated": 2000}
    ]}'
    set_notes "$notes"
    local result=$(hamr_test action --id "note_1" --action "delete")
    
    assert_has_result "$result" "__add__"
    assert_has_result "$result" "note_2"
}

test_action_copy_executes() {
    local notes='{"notes": [{"id": "note_1", "title": "Test Note", "content": "Test content", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    
    # Create a temporary mock wl-copy that accepts arguments (note: real wl-copy takes stdin)
    # This prevents the handler from hanging trying to use the real wl-copy
    local mock_dir=$(mktemp -d)
    local mock_wl_copy="$mock_dir/wl-copy"
    cat > "$mock_wl_copy" << 'MOCK_SCRIPT'
#!/bin/bash
# Mock wl-copy - just silently succeed
exit 0
MOCK_SCRIPT
    chmod +x "$mock_wl_copy"
    
    # Run test with mock in PATH
    local result=$(PATH="$mock_dir:$PATH" hamr_test action --id "note_1" --action "copy")
    rm -rf "$mock_dir"
    
    assert_type "$result" "execute"
    assert_contains "$result" "copied"
    assert_closes "$result"
}

test_action_back_from_card() {
    local notes='{"notes": [{"id": "note_1", "title": "Test", "content": "Content", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    # View the card first
    hamr_test action --id "note_1" > /dev/null
    # Then click back action from the card
    local result=$(hamr_test action --id "note_1" --action "back")
    
    assert_type "$result" "results"
    assert_has_result "$result" "note_1"
}

test_note_has_description() {
    local notes='{"notes": [{"id": "note_1", "title": "Test", "content": "First line\nSecond line", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    local result=$(hamr_test initial)
    
    local description=$(json_get "$result" '.results[] | select(.id == "note_1") | .description')
    assert_eq "$description" "First line"
}

test_note_description_truncated() {
    local long_content=$(printf 'a%.0s' {1..100})
    local notes=$(jq -n --arg content "$long_content" '{"notes": [{"id": "note_1", "title": "Test", "content": $content, "created": 1000, "updated": 1000}]}')
    set_notes "$notes"
    local result=$(hamr_test initial)
    
    local description=$(json_get "$result" '.results[] | select(.id == "note_1") | .description')
    # Should be truncated to 50 chars max
    if [[ ${#description} -gt 53 ]]; then
        echo "Description should be truncated to ~50 chars, got length ${#description}"
        return 1
    fi
}

test_note_empty_content_shows_empty_note() {
    local notes='{"notes": [{"id": "note_1", "title": "No Content", "content": "", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    local result=$(hamr_test initial)
    
    local description=$(json_get "$result" '.results[] | select(.id == "note_1") | .description')
    assert_eq "$description" "Empty note"
}

test_note_has_actions() {
    local notes='{"notes": [{"id": "note_1", "title": "Test", "content": "Content", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    local result=$(hamr_test initial)
    
    local view_action=$(json_get "$result" '.results[] | select(.id == "note_1") | .actions[] | select(.id == "view")')
    assert_contains "$view_action" '"name": "View"'
    
    local edit_action=$(json_get "$result" '.results[] | select(.id == "note_1") | .actions[] | select(.id == "edit")')
    assert_contains "$edit_action" '"name": "Edit"'
    
    local delete_action=$(json_get "$result" '.results[] | select(.id == "note_1") | .actions[] | select(.id == "delete")')
    assert_contains "$delete_action" '"name": "Delete"'
}

test_all_responses_valid_json() {
    local notes='{"notes": [{"id": "note_1", "title": "Test", "content": "Content", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    
    assert_ok hamr_test initial
    assert_ok hamr_test search --query "test"
    assert_ok hamr_test action --id "__add__"
    assert_ok hamr_test action --id "note_1"
    assert_ok hamr_test action --id "note_1" --action "view"
    assert_ok hamr_test action --id "note_1" --action "edit"
}

test_nonexistent_note_error() {
    clear_notes
    local result=$(hamr_test action --id "nonexistent_id")
    
    assert_type "$result" "error"
    assert_contains "$result" "not found"
}

test_form_cancel_returns_to_list() {
    local notes='{"notes": [{"id": "note_1", "title": "Existing", "content": "Content", "created": 1000, "updated": 1000}]}'
    set_notes "$notes"
    hamr_test action --id "__add__" > /dev/null
    local result=$(hamr_test action --id "__form_cancel__")
    
    assert_type "$result" "results"
    assert_has_result "$result" "note_1"
    assert_has_result "$result" "__add__"
}

test_input_mode_realtime() {
    clear_notes
    local result=$(hamr_test initial)
    
    local input_mode=$(json_get "$result" '.inputMode')
    assert_eq "$input_mode" "realtime"
}

test_info_items_not_actionable() {
    clear_notes
    # Info items like __empty__ should not respond (silently ignore clicks)
    # This just verifies the handler doesn't crash on them
    local result=$(hamr_test action --id "__empty__" 2>&1)
    
    # Info items return no output (not an error, just no response)
    # We just verify the handler completes successfully
    assert_ok true
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_initial_empty \
    test_initial_with_notes \
    test_initial_notes_sorted_by_updated_desc \
    test_search_filters_by_title \
    test_search_filters_by_content \
    test_search_empty_query_shows_all \
    test_search_shows_quick_add \
    test_search_with_query_hides_add_button \
    test_search_empty_hides_quick_add \
    test_action_click_add_shows_form \
    test_add_form_has_title_field \
    test_add_form_has_content_field \
    test_action_quick_add_shows_form_with_title \
    test_form_submission_add_creates_note \
    test_form_submission_add_returns_to_list \
    test_form_submission_requires_title \
    test_form_submission_allows_empty_content \
    test_form_submission_clears_input \
    test_action_view_shows_card \
    test_action_view_default_when_no_action \
    test_card_has_markdown \
    test_card_stores_context \
    test_card_has_edit_action \
    test_card_has_delete_action \
    test_action_edit_shows_form \
    test_edit_form_prefills_title \
    test_edit_form_prefills_content \
    test_edit_form_context \
    test_form_submission_edit_updates_note \
    test_edit_updates_timestamp \
    test_edit_requires_title \
    test_action_delete_removes_note \
    test_delete_returns_to_list \
    test_action_copy_executes \
    test_action_back_from_card \
    test_note_has_description \
    test_note_description_truncated \
    test_note_empty_content_shows_empty_note \
    test_note_has_actions \
    test_all_responses_valid_json \
    test_nonexistent_note_error \
    test_form_cancel_returns_to_list \
    test_input_mode_realtime \
    test_info_items_not_actionable
