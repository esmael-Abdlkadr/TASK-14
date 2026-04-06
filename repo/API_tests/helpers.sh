#!/usr/bin/env bash
# ============================================================
# CivicSort API Test Helpers
# ============================================================

BASE_URL="${CIVICSORT_API_URL:-http://localhost:8080}"

TOTAL=0
PASSED=0
FAILED=0
FAILURES=""

# Colors (if terminal supports it)
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# ── Test assertion helpers ───────────────────────────────────

assert_status() {
    local test_name="$1"
    local expected="$2"
    local actual="$3"
    local body="$4"

    TOTAL=$((TOTAL + 1))

    if [ "$actual" -eq "$expected" ]; then
        PASSED=$((PASSED + 1))
        echo -e "  ${GREEN}PASS${NC}  $test_name (HTTP $actual)"
    else
        FAILED=$((FAILED + 1))
        FAILURES="${FAILURES}\n  FAIL: $test_name - expected $expected, got $actual"
        echo -e "  ${RED}FAIL${NC}  $test_name - expected HTTP $expected, got $actual"
        if [ -n "$body" ]; then
            echo "        Response: $(echo "$body" | head -c 200)"
        fi
    fi
}

assert_json_field() {
    local test_name="$1"
    local json="$2"
    local field="$3"
    local expected="$4"

    TOTAL=$((TOTAL + 1))

    local actual
    actual=$(echo "$json" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d${field})" 2>/dev/null || echo "__PARSE_ERROR__")

    if [ "$actual" = "$expected" ]; then
        PASSED=$((PASSED + 1))
        echo -e "  ${GREEN}PASS${NC}  $test_name (${field}=${actual})"
    else
        FAILED=$((FAILED + 1))
        FAILURES="${FAILURES}\n  FAIL: $test_name - ${field} expected '$expected', got '$actual'"
        echo -e "  ${RED}FAIL${NC}  $test_name - ${field} expected '$expected', got '$actual'"
    fi
}

assert_json_contains() {
    local test_name="$1"
    local json="$2"
    local substring="$3"

    TOTAL=$((TOTAL + 1))

    if echo "$json" | grep -q "$substring"; then
        PASSED=$((PASSED + 1))
        echo -e "  ${GREEN}PASS${NC}  $test_name (contains '$substring')"
    else
        FAILED=$((FAILED + 1))
        FAILURES="${FAILURES}\n  FAIL: $test_name - response does not contain '$substring'"
        echo -e "  ${RED}FAIL${NC}  $test_name - response does not contain '$substring'"
    fi
}

# ── HTTP helpers ─────────────────────────────────────────────

do_get() {
    local path="$1"
    local token="$2"
    local headers=""
    if [ -n "$token" ]; then
        headers="-H \"Authorization: Bearer $token\""
    fi
    eval curl -s -w "\n%{http_code}" "$BASE_URL$path" $headers 2>/dev/null
}

do_post() {
    local path="$1"
    local data="$2"
    local token="$3"
    local headers="-H \"Content-Type: application/json\""
    if [ -n "$token" ]; then
        headers="$headers -H \"Authorization: Bearer $token\""
    fi
    eval curl -s -w "\n%{http_code}" -X POST "$BASE_URL$path" $headers -d "'$data'" 2>/dev/null
}

do_put() {
    local path="$1"
    local data="$2"
    local token="$3"
    local headers="-H \"Content-Type: application/json\""
    if [ -n "$token" ]; then
        headers="$headers -H \"Authorization: Bearer $token\""
    fi
    eval curl -s -w "\n%{http_code}" -X PUT "$BASE_URL$path" $headers -d "'$data'" 2>/dev/null
}

do_delete() {
    local path="$1"
    local token="$2"
    local headers=""
    if [ -n "$token" ]; then
        headers="-H \"Authorization: Bearer $token\""
    fi
    eval curl -s -w "\n%{http_code}" -X DELETE "$BASE_URL$path" $headers 2>/dev/null
}

# Extract HTTP status code (last line) and body (everything else)
extract_status() {
    echo "$1" | tail -1
}

extract_body() {
    echo "$1" | sed '$d'
}

# ── Summary output ───────────────────────────────────────────

print_summary() {
    local suite_name="$1"
    echo ""
    echo "------------------------------------------------------------"
    echo "  $suite_name Summary"
    echo "------------------------------------------------------------"
    echo "  Total:   $TOTAL"
    echo -e "  Passed:  ${GREEN}$PASSED${NC}"
    echo -e "  Failed:  ${RED}$FAILED${NC}"
    if [ $FAILED -gt 0 ]; then
        echo ""
        echo "  Failed tests:"
        echo -e "$FAILURES"
    fi
    echo "------------------------------------------------------------"
}
