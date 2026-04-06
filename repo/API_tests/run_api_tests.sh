#!/usr/bin/env bash
# ============================================================
# CivicSort - API Integration Test Runner
# Requires: running backend at CIVICSORT_API_URL (default: http://localhost:8080)
# Requires: curl, python3
# ============================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BASE_URL="${CIVICSORT_API_URL:-http://localhost:8080}"

echo "============================================================"
echo "  CivicSort API Integration Test Suite"
echo "  $(date '+%Y-%m-%d %H:%M:%S')"
echo "  Target: $BASE_URL"
echo "============================================================"

# Clean up token file from previous runs
rm -f "$SCRIPT_DIR/.tokens"

# Check connectivity
echo ""
echo "Checking backend connectivity..."
if ! curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/health" | grep -q "200"; then
    echo "ERROR: Cannot reach backend at $BASE_URL"
    echo "Make sure the backend is running (docker compose up)"
    exit 1
fi
echo "Backend is reachable."
echo ""

TOTAL_SUITES=0
PASSED_SUITES=0
FAILED_SUITES=0
SUITE_RESULTS=""

GRAND_TOTAL=0
GRAND_PASSED=0
GRAND_FAILED=0

# Run each test suite in order
for test_script in "$SCRIPT_DIR"/test_*.sh; do
    TOTAL_SUITES=$((TOTAL_SUITES + 1))
    suite_name=$(basename "$test_script" .sh)

    set +e
    output=$(bash "$test_script" 2>&1)
    exit_code=$?
    set -e

    echo "$output"

    # Parse individual counts from output
    suite_total=$(echo "$output" | grep -oP 'Total:\s+\K\d+' | tail -1 || echo "0")
    suite_passed=$(echo "$output" | grep -oP 'Passed:\s+\K\d+' | tail -1 || echo "0")
    suite_failed=$(echo "$output" | grep -oP 'Failed:\s+\K\d+' | tail -1 || echo "0")

    GRAND_TOTAL=$((GRAND_TOTAL + ${suite_total:-0}))
    GRAND_PASSED=$((GRAND_PASSED + ${suite_passed:-0}))
    GRAND_FAILED=$((GRAND_FAILED + ${suite_failed:-0}))

    if [ "$exit_code" -eq 0 ]; then
        PASSED_SUITES=$((PASSED_SUITES + 1))
        SUITE_RESULTS="${SUITE_RESULTS}\n  PASS  $suite_name (${suite_passed:-0}/${suite_total:-0})"
    else
        FAILED_SUITES=$((FAILED_SUITES + 1))
        SUITE_RESULTS="${SUITE_RESULTS}\n  FAIL  $suite_name (${suite_passed:-0}/${suite_total:-0})"
    fi
done

# Cleanup
rm -f "$SCRIPT_DIR/.tokens"

echo ""
echo "============================================================"
echo "  API Test Grand Summary"
echo "============================================================"
echo ""
echo "  Test Suites: $TOTAL_SUITES total, $PASSED_SUITES passed, $FAILED_SUITES failed"
echo "  Test Cases:  $GRAND_TOTAL total, $GRAND_PASSED passed, $GRAND_FAILED failed"
echo ""
echo "  Suite Results:"
echo -e "$SUITE_RESULTS"
echo ""

if [ "$GRAND_FAILED" -eq 0 ]; then
    echo "STATUS: ALL API TESTS PASSED"
else
    echo "STATUS: SOME API TESTS FAILED"
fi
echo "============================================================"

exit $GRAND_FAILED
