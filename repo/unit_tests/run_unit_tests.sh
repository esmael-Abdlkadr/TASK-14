#!/usr/bin/env bash
# ============================================================
# CivicSort - Unit Test Runner
# Executes all Rust unit tests and outputs structured results
# ============================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "============================================================"
echo "  CivicSort Unit Test Suite"
echo "  $(date '+%Y-%m-%d %H:%M:%S')"
echo "============================================================"
echo ""

cd "$PROJECT_DIR"

# Run cargo test with output capture
echo "Running cargo test..."
echo ""

RESULT_FILE=$(mktemp)
set +e
cargo test --lib -- --format=pretty 2>&1 | tee "$RESULT_FILE"
TEST_EXIT=$?
set -e

echo ""
echo "============================================================"
echo "  Unit Test Results Summary"
echo "============================================================"

# Parse results
if grep -q "^test result:" "$RESULT_FILE"; then
    SUMMARY=$(grep "^test result:" "$RESULT_FILE" | tail -1)
    echo "$SUMMARY"

    PASSED=$(echo "$SUMMARY" | grep -oP '\d+ passed' | grep -oP '\d+')
    FAILED=$(echo "$SUMMARY" | grep -oP '\d+ failed' | grep -oP '\d+')
    IGNORED=$(echo "$SUMMARY" | grep -oP '\d+ ignored' | grep -oP '\d+')

    echo ""
    echo "  Passed:  ${PASSED:-0}"
    echo "  Failed:  ${FAILED:-0}"
    echo "  Ignored: ${IGNORED:-0}"
else
    echo "  ERROR: Could not parse test results"
fi

echo ""

# Show any failures in detail
if [ "$TEST_EXIT" -ne 0 ]; then
    echo "============================================================"
    echo "  FAILED TEST DETAILS"
    echo "============================================================"
    grep -A 10 "^---- .* stdout ----" "$RESULT_FILE" || true
    echo ""
fi

rm -f "$RESULT_FILE"

if [ "$TEST_EXIT" -eq 0 ]; then
    echo "STATUS: ALL UNIT TESTS PASSED"
else
    echo "STATUS: SOME UNIT TESTS FAILED (exit code: $TEST_EXIT)"
fi

echo "============================================================"
exit $TEST_EXIT
