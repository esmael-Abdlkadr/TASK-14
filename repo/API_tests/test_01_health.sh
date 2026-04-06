#!/usr/bin/env bash
# ============================================================
# Test Suite: Health Check & Basic Connectivity
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"

echo ""
echo "=== Test Suite: Health Check ==="
echo ""

# ── 1. Health endpoint returns 200 ───────────────────────────

RESP=$(do_get "/api/health" "")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")

assert_status "GET /api/health returns 200" 200 "$STATUS" "$BODY"
assert_json_contains "Health response has status=healthy" "$BODY" "healthy"
assert_json_contains "Health response has service=civicsort" "$BODY" "civicsort"

# ── 2. Unknown endpoint returns 404 ──────────────────────────

RESP=$(do_get "/api/nonexistent" "")
STATUS=$(extract_status "$RESP")
assert_status "GET unknown path returns 404" 404 "$STATUS" ""

# ── 3. API base without auth returns 401 ─────────────────────

RESP=$(do_get "/api/auth/session" "")
STATUS=$(extract_status "$RESP")
assert_status "GET /api/auth/session without token returns 401" 401 "$STATUS" ""

print_summary "Health Check"
exit $FAILED
