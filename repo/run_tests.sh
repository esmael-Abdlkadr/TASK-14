#!/usr/bin/env bash
# ============================================================
# CivicSort - Master Test Runner
#
# Usage:
#   ./run_tests.sh              # Run all tests (unit + API)
#   ./run_tests.sh unit         # Run unit tests only
#   ./run_tests.sh api          # Run API tests only
#
# For API tests, the backend must be running:
#   docker compose up -d
#   ./run_tests.sh
#
# Environment:
#   CIVICSORT_API_URL  - Backend URL (default: http://localhost:8080)
# ============================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MODE="${1:-all}"

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║          CivicSort Test Verification Plan               ║"
echo "║          $(date '+%Y-%m-%d %H:%M:%S')                        ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

UNIT_PASSED=0
UNIT_FAILED=0
UNIT_TOTAL=0
API_PASSED=0
API_FAILED=0
API_TOTAL=0

# ── Unit Tests ───────────────────────────────────────────────

run_unit_tests() {
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  PHASE 1: Unit Tests (cargo test)"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""

    cd "$SCRIPT_DIR"

    RESULT_FILE=$(mktemp)
    set +e
    cargo test --lib 2>&1 | tee "$RESULT_FILE"
    UNIT_EXIT=$?
    set -e

    if grep -q "^test result:" "$RESULT_FILE"; then
        SUMMARY=$(grep "^test result:" "$RESULT_FILE" | tail -1)
        UNIT_PASSED=$(echo "$SUMMARY" | grep -oP '\d+ passed' | grep -oP '\d+' || echo "0")
        UNIT_FAILED=$(echo "$SUMMARY" | grep -oP '\d+ failed' | grep -oP '\d+' || echo "0")
        UNIT_TOTAL=$((UNIT_PASSED + UNIT_FAILED))
    fi

    rm -f "$RESULT_FILE"

    echo ""
    if [ "$UNIT_EXIT" -eq 0 ]; then
        echo "  Unit Tests: PASSED ($UNIT_PASSED/$UNIT_TOTAL)"
    else
        echo "  Unit Tests: FAILED ($UNIT_PASSED passed, $UNIT_FAILED failed)"
    fi
    echo ""

    return $UNIT_EXIT
}

# ── API Tests ────────────────────────────────────────────────

run_api_tests() {
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  PHASE 2: API Integration Tests (curl)"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""

    BASE_URL="${CIVICSORT_API_URL:-http://localhost:8080}"

    # Check connectivity first
    if ! curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/health" 2>/dev/null | grep -q "200"; then
        echo "  WARNING: Backend not reachable at $BASE_URL"
        echo "  Skipping API tests. Start the backend first:"
        echo "    docker compose up -d"
        echo ""
        return 0
    fi

    RESULT_FILE=$(mktemp)
    set +e
    bash "$SCRIPT_DIR/API_tests/run_api_tests.sh" 2>&1 | tee "$RESULT_FILE"
    API_EXIT=$?
    set -e

    API_TOTAL=$(grep "Test Cases:" "$RESULT_FILE" | grep -oP '\d+ total' | grep -oP '\d+' || echo "0")
    API_PASSED=$(grep "Test Cases:" "$RESULT_FILE" | grep -oP '\d+ passed' | grep -oP '\d+' || echo "0")
    API_FAILED=$(grep "Test Cases:" "$RESULT_FILE" | grep -oP '\d+ failed' | grep -oP '\d+' || echo "0")

    rm -f "$RESULT_FILE"
    return ${API_EXIT:-0}
}

# ── Execute ──────────────────────────────────────────────────

UNIT_EXIT=0
API_EXIT=0

case "$MODE" in
    unit)
        run_unit_tests
        UNIT_EXIT=$?
        ;;
    api)
        run_api_tests
        API_EXIT=$?
        ;;
    all|*)
        run_unit_tests
        UNIT_EXIT=$?
        echo ""
        run_api_tests
        API_EXIT=$?
        ;;
esac

# ── Final Summary ────────────────────────────────────────────

TOTAL=$((UNIT_TOTAL + API_TOTAL))
PASSED=$((UNIT_PASSED + API_PASSED))
FAILED=$((UNIT_FAILED + API_FAILED))

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║              FINAL TEST RESULTS SUMMARY                 ║"
echo "╠══════════════════════════════════════════════════════════╣"
echo "║                                                          ║"
if [ "$UNIT_TOTAL" -gt 0 ]; then
printf "║  Unit Tests:    %3d passed / %3d total                   ║\n" "$UNIT_PASSED" "$UNIT_TOTAL"
fi
if [ "$API_TOTAL" -gt 0 ]; then
printf "║  API Tests:     %3d passed / %3d total                   ║\n" "$API_PASSED" "$API_TOTAL"
fi
echo "║  ─────────────────────────────────                       ║"
printf "║  TOTAL:         %3d passed / %3d total                   ║\n" "$PASSED" "$TOTAL"
echo "║                                                          ║"

OVERALL_EXIT=$((UNIT_EXIT + API_EXIT))
if [ "$OVERALL_EXIT" -eq 0 ]; then
echo "║  STATUS: ✓ ALL TESTS PASSED                             ║"
else
echo "║  STATUS: ✗ SOME TESTS FAILED                            ║"
fi

echo "║                                                          ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

exit $OVERALL_EXIT
