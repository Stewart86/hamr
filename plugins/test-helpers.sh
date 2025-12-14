#!/bin/bash
#
# Shared test helpers for Hamr plugin tests
#
# Usage in your test.sh:
#   source "$(dirname "$0")/../test-helpers.sh"
#   
#   test_initial() {
#       local result=$(hamr_test initial)
#       assert_type "$result" "results"
#   }
#   
#   run_tests test_initial test_search ...
#

set -e

# ============================================================================
# Setup
# ============================================================================

# Find test-harness relative to this file
TEST_HELPERS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HAMR_TEST="${HAMR_TEST:-$TEST_HELPERS_DIR/test-harness}"

# Must be set by the test file
HANDLER="${HANDLER:-}"

# ============================================================================
# HAMR_TEST_MODE Check
# ============================================================================
# Plugin authors MUST explicitly set HAMR_TEST_MODE=1 in their test.sh
# BEFORE sourcing this file. This ensures they've implemented mock data
# in their handler for external API calls.
#
# Example test.sh:
#   #!/bin/bash
#   export HAMR_TEST_MODE=1  # I confirm my handler supports test mode
#   source "$(dirname "$0")/../test-helpers.sh"
#   ...
#
if [[ "${HAMR_TEST_MODE:-}" != "1" ]]; then
    echo -e "\033[0;31mError:\033[0m HAMR_TEST_MODE=1 must be set BEFORE sourcing test-helpers.sh" >&2
    echo "" >&2
    echo "This ensures your handler implements mock data for external APIs." >&2
    echo "" >&2
    echo "Add this to the TOP of your test.sh (before the source line):" >&2
    echo "  export HAMR_TEST_MODE=1" >&2
    echo "" >&2
    echo "Then implement mock support in your handler.py:" >&2
    echo "  TEST_MODE = os.environ.get(\"HAMR_TEST_MODE\") == \"1\"" >&2
    echo "  if TEST_MODE:" >&2
    echo "      return mock_data  # Don't call real APIs" >&2
    echo "" >&2
    exit 1
fi

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# Test counters
_TESTS_RUN=0
_TESTS_PASSED=0
_TESTS_FAILED=0

# ============================================================================
# Core test runner
# ============================================================================

# Run handler with hamr-test
# Usage: hamr_test initial
#        hamr_test search --query "test"
#        hamr_test action --id "item" --action "edit"
hamr_test() {
    if [[ -z "$HANDLER" ]]; then
        echo "Error: HANDLER not set" >&2
        exit 1
    fi
    "$HAMR_TEST" "$HANDLER" "$@" 2>&1
}

# Run a single test case
# Usage: run_test "test name" test_function
run_test() {
    local name="$1"
    local func="$2"
    _TESTS_RUN=$((_TESTS_RUN + 1))
    
    echo -n "  $name... "
    
    # Run test in subshell to isolate failures
    local output
    if output=$("$func" 2>&1); then
        echo -e "${GREEN}PASS${NC}"
        _TESTS_PASSED=$((_TESTS_PASSED + 1))
    else
        echo -e "${RED}FAIL${NC}"
        if [[ -n "$output" ]]; then
            echo "$output" | sed 's/^/    /'
        fi
        _TESTS_FAILED=$((_TESTS_FAILED + 1))
    fi
}

# Run multiple tests and print summary
# Usage: run_tests test_func1 test_func2 ...
run_tests() {
    local test_name="${TEST_NAME:-Plugin Tests}"
    
    echo -e "${CYAN}$test_name${NC}"
    echo "$(printf '=%.0s' {1..40})"
    echo ""
    
    # Check dependencies
    if [[ ! -x "$HAMR_TEST" ]]; then
        echo -e "${RED}Error${NC}: hamr-test not found at $HAMR_TEST"
        exit 1
    fi
    
    if ! command -v jq &> /dev/null; then
        echo -e "${RED}Error${NC}: jq is required"
        exit 1
    fi
    
    if [[ -z "$HANDLER" ]]; then
        echo -e "${RED}Error${NC}: HANDLER not set"
        exit 1
    fi
    
    # Run setup if defined
    if declare -f setup &> /dev/null; then
        setup
    fi
    
    # Run each test
    for test_func in "$@"; do
        # Extract test name from function (test_foo_bar -> "foo bar")
        local name="${test_func#test_}"
        name="${name//_/ }"
        
        # Run per-test setup if defined
        if declare -f before_each &> /dev/null; then
            before_each
        fi
        
        run_test "$name" "$test_func"
        
        # Run per-test teardown if defined
        if declare -f after_each &> /dev/null; then
            after_each
        fi
    done
    
    # Run teardown if defined
    if declare -f teardown &> /dev/null; then
        teardown
    fi
    
    # Summary
    echo ""
    echo "$(printf '=%.0s' {1..40})"
    echo -e "Tests: $_TESTS_RUN | ${GREEN}Passed: $_TESTS_PASSED${NC} | ${RED}Failed: $_TESTS_FAILED${NC}"
    
    [[ $_TESTS_FAILED -eq 0 ]]
}

# ============================================================================
# Assertions
# ============================================================================

# Assert two values are equal
# Usage: assert_eq "$actual" "$expected" "message"
assert_eq() {
    local actual="$1"
    local expected="$2"
    local msg="${3:-Values should be equal}"
    
    if [[ "$actual" == "$expected" ]]; then
        return 0
    fi
    echo "$msg"
    echo "  Expected: $expected"
    echo "  Actual:   $actual"
    return 1
}

# Assert string contains substring
# Usage: assert_contains "$haystack" "$needle" "message"
assert_contains() {
    local haystack="$1"
    local needle="$2"
    local msg="${3:-Should contain substring}"
    
    if [[ "$haystack" == *"$needle"* ]]; then
        return 0
    fi
    echo "$msg"
    echo "  Expected to contain: $needle"
    echo "  In: ${haystack:0:200}..."
    return 1
}

# Assert string does not contain substring
# Usage: assert_not_contains "$haystack" "$needle" "message"
assert_not_contains() {
    local haystack="$1"
    local needle="$2"
    local msg="${3:-Should not contain substring}"
    
    if [[ "$haystack" != *"$needle"* ]]; then
        return 0
    fi
    echo "$msg"
    echo "  Should not contain: $needle"
    return 1
}

# Assert command succeeds (exit 0)
# Usage: assert_ok hamr_test initial
assert_ok() {
    if "$@" > /dev/null 2>&1; then
        return 0
    fi
    echo "Command failed: $*"
    return 1
}

# Assert command fails (exit non-zero)
# Usage: assert_fail hamr_test action --id "nonexistent"
assert_fail() {
    if ! "$@" > /dev/null 2>&1; then
        return 0
    fi
    echo "Command should have failed: $*"
    return 1
}

# ============================================================================
# JSON helpers (require jq)
# ============================================================================

# Get response type
# Usage: assert_type "$response" "results"
assert_type() {
    local response="$1"
    local expected="$2"
    local actual=$(echo "$response" | jq -r '.type')
    assert_eq "$actual" "$expected" "Response type should be '$expected'"
}

# Assert response has a result with given ID
# Usage: assert_has_result "$response" "item-id"
assert_has_result() {
    local response="$1"
    local id="$2"
    local found=$(echo "$response" | jq -r --arg id "$id" '.results[] | select(.id == $id) | .id')
    
    if [[ "$found" == "$id" ]]; then
        return 0
    fi
    echo "Result with id '$id' not found"
    echo "  Available IDs: $(echo "$response" | jq -r '.results[].id' | tr '\n' ' ')"
    return 1
}

# Assert response does not have a result with given ID
# Usage: assert_no_result "$response" "item-id"
assert_no_result() {
    local response="$1"
    local id="$2"
    local found=$(echo "$response" | jq -r --arg id "$id" '.results[] | select(.id == $id) | .id')
    
    if [[ -z "$found" ]]; then
        return 0
    fi
    echo "Result with id '$id' should not exist"
    return 1
}

# Get result count
# Usage: count=$(get_result_count "$response")
get_result_count() {
    echo "$1" | jq '.results | length'
}

# Assert result count
# Usage: assert_result_count "$response" 5
assert_result_count() {
    local response="$1"
    local expected="$2"
    local actual=$(get_result_count "$response")
    assert_eq "$actual" "$expected" "Result count should be $expected"
}

# Get field from response
# Usage: type=$(json_get "$response" '.type')
#        name=$(json_get "$response" '.results[0].name')
json_get() {
    echo "$1" | jq -r "$2"
}

# Assert JSON field value
# Usage: assert_json "$response" '.inputMode' "submit"
assert_json() {
    local response="$1"
    local path="$2"
    local expected="$3"
    local actual=$(json_get "$response" "$path")
    assert_eq "$actual" "$expected" "Field $path should be '$expected'"
}

# Assert response is in submit mode
# Usage: assert_submit_mode "$response"
assert_submit_mode() {
    assert_json "$1" '.inputMode' "submit"
}

# Assert response is in realtime mode
# Usage: assert_realtime_mode "$response"
assert_realtime_mode() {
    local mode=$(json_get "$1" '.inputMode')
    # Default is realtime, so null or "realtime" both count
    if [[ "$mode" == "realtime" || "$mode" == "null" ]]; then
        return 0
    fi
    echo "Expected realtime mode, got: $mode"
    return 1
}

# Assert execute response closes launcher
# Usage: assert_closes "$response"
assert_closes() {
    local response="$1"
    assert_type "$response" "execute" || return 1
    local close=$(json_get "$response" '.execute.close')
    if [[ "$close" == "true" ]]; then
        return 0
    fi
    echo "Expected execute.close to be true"
    return 1
}

# Assert execute response stays open
# Usage: assert_stays_open "$response"
assert_stays_open() {
    local response="$1"
    assert_type "$response" "execute" || return 1
    local close=$(json_get "$response" '.execute.close')
    if [[ "$close" == "false" ]]; then
        return 0
    fi
    echo "Expected execute.close to be false"
    return 1
}

# ============================================================================
# File helpers
# ============================================================================

# Create a temporary file that's cleaned up on exit
# Usage: tmpfile=$(make_temp)
make_temp() {
    local tmpfile=$(mktemp)
    trap "rm -f '$tmpfile'" EXIT
    echo "$tmpfile"
}

# Create a temporary directory that's cleaned up on exit
# Usage: tmpdir=$(make_temp_dir)
make_temp_dir() {
    local tmpdir=$(mktemp -d)
    trap "rm -rf '$tmpdir'" EXIT
    echo "$tmpdir"
}
