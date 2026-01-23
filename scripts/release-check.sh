#!/usr/bin/env bash
# Release readiness check script
# Runs all validation commands and exits non-zero on failure
#
# Usage:
#   ./scripts/release-check.sh
#   ./scripts/release-check.sh --quick  # Skip slow tests

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

quick_mode=false
failed=0
passed=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --quick)
            quick_mode=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [--quick]"
            echo ""
            echo "Run all validation checks for release readiness."
            echo ""
            echo "Options:"
            echo "  --quick     Skip slow tests (cargo test)"
            echo "  -h, --help  Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Run a check and track pass/fail
run_check() {
    local name="$1"
    shift
    
    echo -e "${BLUE}[$name]${NC} Running..."
    
    if "$@" > /dev/null 2>&1; then
        echo -e "${GREEN}[$name]${NC} Passed"
        ((passed++)) || true
        return 0
    else
        echo -e "${RED}[$name]${NC} Failed"
        echo -e "${YELLOW}  Command: $*${NC}"
        ((failed++)) || true
        return 1
    fi
}

# Run a check with visible output on failure
run_check_verbose() {
    local name="$1"
    shift
    local tmpfile
    tmpfile=$(mktemp)
    
    echo -e "${BLUE}[$name]${NC} Running..."
    
    if "$@" > "$tmpfile" 2>&1; then
        echo -e "${GREEN}[$name]${NC} Passed"
        ((passed++)) || true
        rm -f "$tmpfile"
        return 0
    else
        echo -e "${RED}[$name]${NC} Failed"
        echo -e "${YELLOW}  Command: $*${NC}"
        echo -e "${YELLOW}  Output:${NC}"
        cat "$tmpfile" | head -50
        rm -f "$tmpfile"
        ((failed++)) || true
        return 1
    fi
}

cd "$PROJECT_ROOT"

echo ""
echo "====================================="
echo "  Hamr Release Readiness Check"
echo "====================================="
echo ""

# 1. Rust formatting
run_check_verbose "cargo fmt" cargo fmt --all -- --check || true

# 2. Clippy lints
run_check_verbose "cargo clippy" cargo clippy --all-targets -- -D warnings || true

# 3. Cargo tests (skip in quick mode)
if [[ "$quick_mode" == true ]]; then
    echo -e "${YELLOW}[cargo test]${NC} Skipped (quick mode)"
else
    run_check_verbose "cargo test" cargo test --all || true
fi

# 4. Cargo build (release mode for real release check)
run_check "cargo build" cargo build --release || true

# 5. MkDocs build
run_check_verbose "mkdocs build" mkdocs build --strict || true

# 6. Python syntax check (plugins)
if command -v python3 > /dev/null 2>&1; then
    run_check "python compile" python3 -m compileall -q plugins || true
else
    echo -e "${YELLOW}[python compile]${NC} Skipped (python3 not found)"
fi

# 7. Plugin checksums verification (if checksums.json exists)
if [[ -f "$PROJECT_ROOT/plugins/checksums.json" ]]; then
    run_check "plugin checksums" "$SCRIPT_DIR/generate-plugin-checksums.sh" --verify || true
else
    echo -e "${YELLOW}[plugin checksums]${NC} Skipped (no checksums.json)"
fi

echo ""
echo "====================================="
echo "  Summary"
echo "====================================="
echo ""
echo -e "Passed: ${GREEN}$passed${NC}"
echo -e "Failed: ${RED}$failed${NC}"
echo ""

if [[ $failed -gt 0 ]]; then
    echo -e "${RED}Release check failed. Fix issues before releasing.${NC}"
    exit 1
else
    echo -e "${GREEN}All checks passed. Ready for release!${NC}"
    exit 0
fi
