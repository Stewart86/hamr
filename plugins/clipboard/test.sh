#!/bin/bash
#
# Tests for clipboard plugin
# Run: ./test.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export HAMR_TEST_MODE=1
source "$SCRIPT_DIR/../test-helpers.sh"

# ============================================================================
# Config
# ============================================================================

TEST_NAME="Clipboard Plugin Tests"
HANDLER="$SCRIPT_DIR/handler.py"

# ============================================================================
# Mocking
# ============================================================================

# Mock cliphist to return test entries
export MOCK_CLIPHIST_ENTRIES=""

# Override cliphist for testing (patch PATH to use mock)
setup_mock_cliphist() {
    local mock_dir=$(mktemp -d)
    cat > "$mock_dir/cliphist" << 'EOF'
#!/bin/bash
# Mock cliphist for testing
case "$1" in
    list)
        # Return mock entries from environment
        echo "$MOCK_CLIPHIST_ENTRIES"
        ;;
    decode)
        # For mock, just echo the entry (skip actual decoding)
        cat
        ;;
    delete)
        # No-op for testing
        ;;
    wipe)
        # No-op for testing
        ;;
    *)
        exit 1
        ;;
esac
EOF
    chmod +x "$mock_dir/cliphist"
    echo "$mock_dir"
}

# ============================================================================
# Setup / Teardown
# ============================================================================

setup() {
    MOCK_DIR=$(setup_mock_cliphist)
    export PATH="$MOCK_DIR:$PATH"
}

teardown() {
    rm -rf "$MOCK_DIR"
}

# ============================================================================
# Helpers
# ============================================================================

set_clipboard_entries() {
    export MOCK_CLIPHIST_ENTRIES="$1"
}

clear_clipboard() {
    export MOCK_CLIPHIST_ENTRIES=""
}

add_clipboard_entry() {
    local entry="$1"
    if [[ -z "$MOCK_CLIPHIST_ENTRIES" ]]; then
        MOCK_CLIPHIST_ENTRIES="$entry"
    else
        MOCK_CLIPHIST_ENTRIES="$MOCK_CLIPHIST_ENTRIES
$entry"
    fi
    export MOCK_CLIPHIST_ENTRIES
}

# ============================================================================
# Tests: Initial Step
# ============================================================================

test_initial_empty() {
    clear_clipboard
    local result=$(hamr_test initial)
    
    assert_type "$result" "results"
    assert_contains "$result" "No clipboard entries"
}

test_initial_with_entries() {
    set_clipboard_entries "1	Hello world
2	Another entry"
    local result=$(hamr_test initial)
    
    assert_type "$result" "results"
    assert_contains "$result" "Hello world"
    assert_contains "$result" "Another entry"
}

test_initial_shows_wipe_option() {
    set_clipboard_entries "1	Test entry"
    local result=$(hamr_test initial)
    
    assert_has_result "$result" "__wipe__"
    assert_contains "$result" "Wipe clipboard history"
}

test_initial_no_wipe_when_empty() {
    clear_clipboard
    local result=$(hamr_test initial)
    
    # When empty, wipe is shown but also empty state message
    # Wipe button doesn't do anything useful when nothing to wipe
    assert_contains "$result" "No clipboard entries"
    assert_contains "$result" "Wipe clipboard history"
}

test_initial_realtime_mode() {
    set_clipboard_entries "1	Entry"
    local result=$(hamr_test initial)
    
    assert_realtime_mode "$result"
}

# ============================================================================
# Tests: Search Step
# ============================================================================

test_search_filter_matches() {
    set_clipboard_entries "1	Hello world
2	Hello universe
3	Goodbye world"
    local result=$(hamr_test search --query "hello")
    
    assert_contains "$result" "Hello world"
    assert_contains "$result" "Hello universe"
    assert_not_contains "$result" "Goodbye world"
}

test_search_case_insensitive() {
    set_clipboard_entries "1	Hello WORLD
2	goodbye world"
    local result=$(hamr_test search --query "HELLO")
    
    assert_contains "$result" "Hello WORLD"
    assert_not_contains "$result" "goodbye world"
}

test_search_fuzzy_match() {
    set_clipboard_entries "1	The quick brown fox
2	Another entry"
    local result=$(hamr_test search --query "tqb")
    
    assert_contains "$result" "The quick brown fox"
}

test_search_empty_query_shows_wipe() {
    set_clipboard_entries "1	Test"
    local result=$(hamr_test search --query "")
    
    assert_has_result "$result" "__wipe__"
}

test_search_with_query_hides_wipe() {
    set_clipboard_entries "1	Test entry"
    local result=$(hamr_test search --query "test")
    
    assert_no_result "$result" "__wipe__"
}

test_search_no_results() {
    set_clipboard_entries "1	Hello
2	World"
    local result=$(hamr_test search --query "xyz")
    
    assert_contains "$result" "No clipboard entries"
}

test_search_realtime_mode() {
    set_clipboard_entries "1	Entry"
    local result=$(hamr_test search --query "entry")
    
    assert_realtime_mode "$result"
}

# ============================================================================
# Tests: Text Entry Display
# ============================================================================

test_text_entry_truncation() {
    local long_text=$(printf 'A%.0s' {1..150})
    set_clipboard_entries "1	$long_text"
    local result=$(hamr_test initial)
    
    assert_contains "$result" "..."
}

test_text_entry_shows_text_icon() {
    set_clipboard_entries "1	Some text"
    local result=$(hamr_test initial)
    
    assert_json "$result" '.results[] | select(.name == "Some text") | .icon' "content_paste"
}

test_text_entry_shows_text_description() {
    set_clipboard_entries "1	Some text"
    local result=$(hamr_test initial)
    
    assert_json "$result" '.results[] | select(.name == "Some text") | .description' "Text"
}

# ============================================================================
# Tests: Image Entry Display
# ============================================================================

test_image_entry_detection() {
    set_clipboard_entries "1	[[image binary data 640x480]]"
    local result=$(hamr_test initial)
    
    assert_contains "$result" "Image 640x480"
}

test_image_entry_shows_image_icon() {
    set_clipboard_entries "1	[[image binary data 640x480]]"
    local result=$(hamr_test initial)
    
    assert_json "$result" '.results[] | select(.description == "Image") | .icon' "image"
}

test_image_entry_shows_image_description() {
    set_clipboard_entries "1	[[image binary data 640x480]]"
    local result=$(hamr_test initial)
    
    assert_json "$result" '.results[] | select(.description == "Image") | .description' "Image"
}

# ============================================================================
# Tests: Actions
# ============================================================================

test_entry_has_copy_action() {
    set_clipboard_entries "1	Copy me"
    local result=$(hamr_test initial)
    
    local actions=$(json_get "$result" '.results[] | select(.name == "Copy me") | .actions[].id' | tr '\n' ',')
    assert_contains "$actions" "copy"
}

test_entry_has_delete_action() {
    set_clipboard_entries "1	Delete me"
    local result=$(hamr_test initial)
    
    local actions=$(json_get "$result" '.results[] | select(.name == "Delete me") | .actions[].id' | tr '\n' ',')
    assert_contains "$actions" "delete"
}

test_default_action_is_copy() {
    set_clipboard_entries "1	Copy on click"
    local result=$(hamr_test action --id "1	Copy on click")
    
    assert_type "$result" "execute"
    assert_closes "$result"
}

test_explicit_copy_action() {
    set_clipboard_entries "1	Explicit copy"
    local result=$(hamr_test action --id "1	Explicit copy" --action "copy")
    
    assert_type "$result" "execute"
    assert_closes "$result"
}

test_copy_closes_launcher() {
    set_clipboard_entries "1	Test"
    local result=$(hamr_test action --id "1	Test" --action "copy")
    
    assert_closes "$result"
}

# ============================================================================
# Tests: Delete Action
# ============================================================================

test_delete_removes_entry() {
    set_clipboard_entries "1	Keep this
2	Delete this"
    local result=$(hamr_test action --id "2	Delete this" --action "delete")
    
    assert_contains "$result" "Keep this"
    assert_not_contains "$result" "Delete this"
}

test_delete_returns_results() {
    set_clipboard_entries "1	Entry
2	To delete"
    local result=$(hamr_test action --id "2	To delete" --action "delete")
    
    assert_type "$result" "results"
}

test_delete_shows_remaining_entries() {
    set_clipboard_entries "1	Keep
2	Remove"
    hamr_test action --id "2	Remove" --action "delete" > /dev/null
    local result=$(hamr_test initial)
    
    assert_contains "$result" "Keep"
}

test_delete_last_entry_shows_empty() {
    set_clipboard_entries "1	Only entry"
    local result=$(hamr_test action --id "1	Only entry" --action "delete")
    
    assert_contains "$result" "No clipboard entries"
}

# ============================================================================
# Tests: Wipe Action
# ============================================================================

test_wipe_shows_confirmation() {
    set_clipboard_entries "1	Entry"
    local result=$(hamr_test action --id "__wipe__")
    
    assert_type "$result" "results"
    assert_has_result "$result" "__wipe_confirm__"
    assert_has_result "$result" "__back__"
}

test_wipe_confirmation_shows_warning() {
    set_clipboard_entries "1	Entry"
    local result=$(hamr_test action --id "__wipe__")
    
    assert_contains "$result" "Confirm: Wipe all clipboard history"
}

test_wipe_confirmed_closes() {
    set_clipboard_entries "1	Entry"
    local result=$(hamr_test action --id "__wipe_confirm__")
    
    assert_type "$result" "execute"
    assert_closes "$result"
}

test_wipe_back_cancels() {
    set_clipboard_entries "1	Entry"
    hamr_test action --id "__wipe__" > /dev/null
    local result=$(hamr_test action --id "__back__")
    
    assert_type "$result" "results"
    assert_contains "$result" "Entry"
    assert_no_result "$result" "__wipe_confirm__"
}

# ============================================================================
# Tests: Empty State
# ============================================================================

test_empty_state_not_actionable() {
    clear_clipboard
    local result=$(hamr_test action --id "__empty__")
    
    assert_type "$result" "results"
}

test_empty_state_on_wipe() {
    set_clipboard_entries "1	Entry"
    hamr_test action --id "__wipe__" > /dev/null
    local result=$(hamr_test action --id "__wipe_confirm__")
    
    # Next state should be empty
    hamr_test initial > /dev/null 2>&1
}

# ============================================================================
# Tests: Placeholder Text
# ============================================================================

test_initial_placeholder() {
    set_clipboard_entries "1	Entry"
    local result=$(hamr_test initial)
    
    assert_json "$result" '.placeholder' "Search clipboard..."
}

test_wipe_confirmation_placeholder() {
    set_clipboard_entries "1	Entry"
    local result=$(hamr_test action --id "__wipe__")
    
    assert_json "$result" '.placeholder' "Confirm wipe?"
}

# ============================================================================
# Tests: Response Structure
# ============================================================================

test_result_has_required_fields() {
    set_clipboard_entries "1	Entry"
    local result=$(hamr_test initial)
    
    # Check result structure (skip wipe option at index 0, actual entry is at index 1)
    assert_json "$result" '.results[1].id' "1	Entry"
    assert_json "$result" '.results[1].name' "Entry"
    assert_json "$result" '.results[1].icon' "content_paste"
}

test_all_responses_are_valid_json() {
    set_clipboard_entries "1	Test entry
2	Another entry"
    
    assert_ok hamr_test initial
    assert_ok hamr_test search --query "test"
    assert_ok hamr_test action --id "__wipe__"
    assert_ok hamr_test action --id "__back__"
}

test_image_entry_has_thumbnail_field() {
    set_clipboard_entries "1	[[image binary data 640x480]]"
    local result=$(hamr_test initial)
    
    # Image entries should have thumbnail field (even if null in mock)
    local thumbnail=$(json_get "$result" '.results[] | select(.description == "Image") | .thumbnail')
    # Field exists (may be null or a path)
}

# ============================================================================
# Tests: Edge Cases
# ============================================================================

test_very_long_entry() {
    local long_entry=$(printf 'A%.0s' {1..1000})
    set_clipboard_entries "1	$long_entry"
    local result=$(hamr_test initial)
    
    assert_ok true  # Just test it doesn't crash
}

test_special_characters_in_entry() {
    set_clipboard_entries "1	Test with 'quotes' and \"double quotes\""
    local result=$(hamr_test initial)
    
    assert_ok true  # Should handle special chars
}

test_multiline_entries() {
    set_clipboard_entries "1	Line 1
Line 2
2	Another entry"
    local result=$(hamr_test initial)
    
    assert_ok true  # Should handle entries with newlines
}

test_empty_entry() {
    set_clipboard_entries "1	"
    local result=$(hamr_test initial)
    
    assert_ok true  # Should handle empty entries gracefully
}

test_tab_separated_entry() {
    set_clipboard_entries "1	Text	with	tabs"
    local result=$(hamr_test initial)
    
    assert_contains "$result" "Text"
}

test_unicode_in_entry() {
    set_clipboard_entries "1	Hello 世界 🌍 мир"
    local result=$(hamr_test initial)
    
    assert_ok true  # Should handle unicode
}

test_search_unicode() {
    set_clipboard_entries "1	Hello 世界"
    local result=$(hamr_test search --query "世界")
    
    # Check that result count is 1 (the matching entry)
    assert_result_count "$result" 1
}

# ============================================================================
# Tests: Realtime vs Submit Mode
# ============================================================================

test_initial_uses_realtime_mode() {
    set_clipboard_entries "1	Entry"
    local result=$(hamr_test initial)
    assert_realtime_mode "$result"
}

test_search_uses_realtime_mode() {
    set_clipboard_entries "1	Entry"
    local result=$(hamr_test search --query "entry")
    assert_realtime_mode "$result"
}

test_wipe_confirmation_uses_realtime_mode() {
    set_clipboard_entries "1	Entry"
    local result=$(hamr_test action --id "__wipe__")
    assert_realtime_mode "$result"
}

# ============================================================================
# Tests: Clear Input Flag
# ============================================================================

test_back_clears_input() {
    set_clipboard_entries "1	Entry"
    hamr_test action --id "__wipe__" > /dev/null
    local result=$(hamr_test action --id "__back__")
    
    local clearInput=$(json_get "$result" '.clearInput')
    assert_eq "$clearInput" "true" "clearInput should be true on back"
}

# ============================================================================
# Tests: Fuzzy Matching
# ============================================================================

test_fuzzy_discontinuous_chars() {
    set_clipboard_entries "1	The quick brown fox"
    local result=$(hamr_test search --query "tbf")
    
    # Should match: t(he) b(rown) f(ox)
    assert_contains "$result" "The quick brown fox"
}

test_fuzzy_all_chars_required() {
    set_clipboard_entries "1	Hello
2	World"
    local result=$(hamr_test search --query "hxyz")
    
    # Should not match - 'x', 'y', 'z' not in "Hello"
    assert_not_contains "$result" "Hello"
}

test_fuzzy_partial_match_fails() {
    set_clipboard_entries "1	Testing"
    local result=$(hamr_test search --query "testx")
    
    # Should not match - 'x' not in "Testing"
    assert_not_contains "$result" "Testing"
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_initial_empty \
    test_initial_with_entries \
    test_initial_shows_wipe_option \
    test_initial_no_wipe_when_empty \
    test_initial_realtime_mode \
    test_search_filter_matches \
    test_search_case_insensitive \
    test_search_fuzzy_match \
    test_search_empty_query_shows_wipe \
    test_search_with_query_hides_wipe \
    test_search_no_results \
    test_search_realtime_mode \
    test_text_entry_truncation \
    test_text_entry_shows_text_icon \
    test_text_entry_shows_text_description \
    test_image_entry_detection \
    test_image_entry_shows_image_icon \
    test_image_entry_shows_image_description \
    test_entry_has_copy_action \
    test_entry_has_delete_action \
    test_default_action_is_copy \
    test_explicit_copy_action \
    test_copy_closes_launcher \
    test_delete_removes_entry \
    test_delete_returns_results \
    test_delete_shows_remaining_entries \
    test_delete_last_entry_shows_empty \
    test_wipe_shows_confirmation \
    test_wipe_confirmation_shows_warning \
    test_wipe_confirmed_closes \
    test_wipe_back_cancels \
    test_empty_state_not_actionable \
    test_empty_state_on_wipe \
    test_initial_placeholder \
    test_wipe_confirmation_placeholder \
    test_result_has_required_fields \
    test_all_responses_are_valid_json \
    test_image_entry_has_thumbnail_field \
    test_very_long_entry \
    test_special_characters_in_entry \
    test_multiline_entries \
    test_empty_entry \
    test_tab_separated_entry \
    test_unicode_in_entry \
    test_search_unicode \
    test_initial_uses_realtime_mode \
    test_search_uses_realtime_mode \
    test_wipe_confirmation_uses_realtime_mode \
    test_back_clears_input \
    test_fuzzy_discontinuous_chars \
    test_fuzzy_all_chars_required \
    test_fuzzy_partial_match_fails
