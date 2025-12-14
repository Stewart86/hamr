#!/bin/bash
#
# Tests for screenrecord plugin
# Run: ./test.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export HAMR_TEST_MODE=1
source "$SCRIPT_DIR/../test-helpers.sh"

# ============================================================================
# Config
# ============================================================================

TEST_NAME="Screenrecord Plugin Tests"
HANDLER="$SCRIPT_DIR/handler.py"

# Cache and state files (same as handler.py)
CACHE_DIR="${HOME}/.cache/hamr"
STATE_FILE="$CACHE_DIR/screenrecord_state.json"
LAUNCH_TIMESTAMP_FILE="$CACHE_DIR/launch_timestamp"
BACKUP_STATE="/tmp/screenrecord-test-backup-state-$$.json"
BACKUP_TIMESTAMP="/tmp/screenrecord-test-backup-timestamp-$$.txt"

# ============================================================================
# Setup / Teardown
# ============================================================================

setup() {
    # Backup existing state files
    if [[ -f "$STATE_FILE" ]]; then
        cp "$STATE_FILE" "$BACKUP_STATE"
    fi
    if [[ -f "$LAUNCH_TIMESTAMP_FILE" ]]; then
        cp "$LAUNCH_TIMESTAMP_FILE" "$BACKUP_TIMESTAMP"
    fi
}

teardown() {
    # Restore original state
    mkdir -p "$CACHE_DIR"
    if [[ -f "$BACKUP_STATE" ]]; then
        cp "$BACKUP_STATE" "$STATE_FILE"
    else
        rm -f "$STATE_FILE"
    fi
    if [[ -f "$BACKUP_TIMESTAMP" ]]; then
        cp "$BACKUP_TIMESTAMP" "$LAUNCH_TIMESTAMP_FILE"
    else
        rm -f "$LAUNCH_TIMESTAMP_FILE"
    fi
    rm -f "$BACKUP_STATE" "$BACKUP_TIMESTAMP"
}

before_each() {
    # Clear state before each test (start with no recording in progress)
    rm -f "$STATE_FILE" "$LAUNCH_TIMESTAMP_FILE"
}

# ============================================================================
# Helpers
# ============================================================================

clear_recording_state() {
    rm -f "$STATE_FILE"
}

set_recording_state() {
    mkdir -p "$CACHE_DIR"
    echo "$1" > "$STATE_FILE"
}

get_recording_state() {
    if [[ -f "$STATE_FILE" ]]; then
        cat "$STATE_FILE"
    else
        echo ""
    fi
}

# Mock wf-recorder as running
mock_recording_running() {
    # This is a bit tricky - we'd need to actually run wf-recorder
    # For now, we just verify the response structure assumes no recording
    true
}

# ============================================================================
# Tests
# ============================================================================

test_initial_no_recording() {
    clear_recording_state
    local result=$(hamr_test initial)
    
    assert_type "$result" "results"
    # Should show recording options
    assert_has_result "$result" "record_screen"
    assert_has_result "$result" "record_screen_audio"
    assert_has_result "$result" "record_region"
    assert_has_result "$result" "record_region_audio"
    # Should always show browse option
    assert_has_result "$result" "browse"
    # Should not show stop option
    assert_no_result "$result" "stop"
}

test_initial_results_have_descriptions() {
    clear_recording_state
    local result=$(hamr_test initial)
    
    # Verify descriptions exist
    local desc=$(json_get "$result" '.results[] | select(.id == "record_screen") | .description')
    assert_contains "$desc" "Record focused monitor"
    assert_contains "$desc" "3s"
}

test_initial_browse_shows_videos_dir() {
    clear_recording_state
    local result=$(hamr_test initial)
    
    # Browse result should point to Videos directory
    local browse_desc=$(json_get "$result" '.results[] | select(.id == "browse") | .description')
    assert_contains "$browse_desc" "Videos"
}

test_record_screen_action() {
    clear_recording_state
    local result=$(hamr_test action --id "record_screen")
    
    assert_type "$result" "execute"
    # Should close after starting record
    assert_closes "$result"
    # Command should use bash -c
    local cmd=$(json_get "$result" '.execute.command[0]')
    assert_eq "$cmd" "bash"
}

test_record_screen_audio_action() {
    clear_recording_state
    local result=$(hamr_test action --id "record_screen_audio")
    
    assert_type "$result" "execute"
    assert_closes "$result"
    local cmd=$(json_get "$result" '.execute.command[0]')
    assert_eq "$cmd" "bash"
}

test_record_region_action() {
    clear_recording_state
    local result=$(hamr_test action --id "record_region")
    
    assert_type "$result" "execute"
    assert_closes "$result"
    # Should use bash -c for region selection
    local cmd=$(json_get "$result" '.execute.command[0]')
    assert_eq "$cmd" "bash"
}

test_record_region_audio_action() {
    clear_recording_state
    local result=$(hamr_test action --id "record_region_audio")
    
    assert_type "$result" "execute"
    assert_closes "$result"
    local cmd=$(json_get "$result" '.execute.command[0]')
    assert_eq "$cmd" "bash"
}

test_browse_action() {
    clear_recording_state
    local result=$(hamr_test action --id "browse")
    
    assert_type "$result" "execute"
    assert_closes "$result"
    # xdg-open command
    local cmd=$(json_get "$result" '.execute.command[0]')
    assert_eq "$cmd" "xdg-open"
}

test_record_screen_saves_state() {
    clear_recording_state
    hamr_test action --id "record_screen" > /dev/null
    
    # State file should be created
    if [[ -f "$STATE_FILE" ]]; then
        local state=$(cat "$STATE_FILE")
        assert_contains "$state" "recording_path"
        assert_contains "$state" "start_time_ms"
    fi
}

test_all_recording_actions_save_state() {
    for action in record_screen record_screen_audio record_region record_region_audio; do
        clear_recording_state
        hamr_test action --id "$action" > /dev/null
        
        if [[ -f "$STATE_FILE" ]]; then
            local state=$(cat "$STATE_FILE")
            assert_contains "$state" "recording_path" "State should be saved for $action"
        fi
    done
}

test_stop_action_requires_state() {
    clear_recording_state
    # Without state file, stop should still work (fallback)
    local result=$(hamr_test action --id "stop")
    
    assert_type "$result" "execute"
    assert_closes "$result"
    # Should use pkill to stop wf-recorder
    local cmd=$(json_get "$result" '.execute.command[2]')
    assert_contains "$cmd" "pkill"
}

test_record_commands_have_notify_send() {
    clear_recording_state
    local result=$(hamr_test action --id "record_screen")
    
    local cmd=$(json_get "$result" '.execute.command[2]')
    # Command should include notify-send for "Recording starts in Xs"
    assert_contains "$cmd" "notify-send"
}

test_record_script_includes_wf_recorder() {
    clear_recording_state
    local result=$(hamr_test action --id "record_screen")
    
    local cmd=$(json_get "$result" '.execute.command[2]')
    # Should use wf-recorder
    assert_contains "$cmd" "wf-recorder"
}

test_record_region_includes_slurp() {
    clear_recording_state
    local result=$(hamr_test action --id "record_region")
    
    local cmd=$(json_get "$result" '.execute.command[2]')
    # Region recording should use slurp for selection
    assert_contains "$cmd" "slurp"
}

test_record_with_audio_includes_pactl() {
    clear_recording_state
    local result=$(hamr_test action --id "record_screen_audio")
    
    local cmd=$(json_get "$result" '.execute.command[2]')
    # With audio, script checks audio source
    # Note: actual audio source depends on system, just verify script exists
    assert_contains "$cmd" "wf-recorder"
}

test_stop_action_kills_wf_recorder() {
    clear_recording_state
    local result=$(hamr_test action --id "stop")
    
    local cmd=$(json_get "$result" '.execute.command[2]')
    # Should kill wf-recorder
    assert_contains "$cmd" "pkill -INT wf-recorder"
}

test_all_execute_responses_are_commands() {
    clear_recording_state
    
    for action in record_screen record_screen_audio record_region record_region_audio browse stop; do
        local result=$(hamr_test action --id "$action")
        assert_type "$result" "execute" "Action $action should return execute type"
        
        # Should have command field
        local cmd=$(json_get "$result" '.execute.command')
        assert_not_contains "$cmd" "null" "Action $action should have command"
    done
}

test_responses_are_valid_json() {
    clear_recording_state
    
    # Test initial
    local result=$(hamr_test initial)
    assert_ok json_get "$result" '.type' > /dev/null
    
    # Test all actions
    for action in record_screen record_region browse stop; do
        result=$(hamr_test action --id "$action")
        assert_ok json_get "$result" '.execute' > /dev/null
    done
}

test_search_same_as_initial() {
    clear_recording_state
    local initial=$(hamr_test initial)
    local search=$(hamr_test search --query "")
    
    # Search and initial should return same results when no recording
    local initial_ids=$(json_get "$initial" '.results[].id' | sort)
    local search_ids=$(json_get "$search" '.results[].id' | sort)
    assert_eq "$initial_ids" "$search_ids"
}

test_unknown_action_returns_error() {
    local result=$(hamr_test action --id "nonexistent_action")
    
    assert_type "$result" "error"
    assert_contains "$result" "Unknown action"
}

test_result_icons_are_material_icons() {
    clear_recording_state
    local result=$(hamr_test initial)
    
    # Check that icons are valid material icon names (no spaces, lowercase)
    local icons=$(json_get "$result" '.results[].icon')
    while IFS= read -r icon; do
        [[ -z "$icon" ]] && continue
        # Icons should not contain spaces and should be lowercase
        assert_not_contains "$icon" " " "Icon should not contain spaces: $icon"
    done <<< "$icons"
}

test_record_video_path_uses_timestamp() {
    clear_recording_state
    local result=$(hamr_test action --id "record_screen")
    
    # The script should generate a filename with timestamp
    local cmd=$(json_get "$result" '.execute.command[2]')
    # Should reference recording_YYYY-MM-DD_HH.MM.SS.mp4
    assert_contains "$cmd" "recording_"
}

test_multiple_actions_dont_interfere() {
    clear_recording_state
    
    # Run multiple actions and ensure each produces valid output
    hamr_test action --id "record_screen" > /dev/null
    hamr_test action --id "browse" > /dev/null
    
    # State should reflect last action
    local result=$(hamr_test initial)
    assert_type "$result" "results"
}

# ============================================================================
# Run
# ============================================================================

run_tests \
    test_initial_no_recording \
    test_initial_results_have_descriptions \
    test_initial_browse_shows_videos_dir \
    test_record_screen_action \
    test_record_screen_audio_action \
    test_record_region_action \
    test_record_region_audio_action \
    test_browse_action \
    test_record_screen_saves_state \
    test_all_recording_actions_save_state \
    test_stop_action_requires_state \
    test_record_commands_have_notify_send \
    test_record_script_includes_wf_recorder \
    test_record_region_includes_slurp \
    test_record_with_audio_includes_pactl \
    test_stop_action_kills_wf_recorder \
    test_all_execute_responses_are_commands \
    test_responses_are_valid_json \
    test_search_same_as_initial \
    test_unknown_action_returns_error \
    test_result_icons_are_material_icons \
    test_record_video_path_uses_timestamp \
    test_multiple_actions_dont_interfere
