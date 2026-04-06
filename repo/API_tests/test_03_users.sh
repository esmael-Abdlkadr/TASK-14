#!/usr/bin/env bash
# ============================================================
# Test Suite: User Management
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
source "$SCRIPT_DIR/.tokens" 2>/dev/null || true

echo ""
echo "=== Test Suite: User Management ==="
echo ""

# ── 1. List users as admin ───────────────────────────────────

RESP=$(do_get "/api/users" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "List users as admin" 200 "$STATUS" "$BODY"
assert_json_contains "User list contains testadmin" "$BODY" "testadmin"

# ── 2. List users as inspector (forbidden) ───────────────────

RESP=$(do_get "/api/users" "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "List users as inspector returns 403" 403 "$STATUS" ""

# ── 3. Get self as inspector ─────────────────────────────────

# First get inspector's user_id
RESP=$(do_get "/api/auth/session" "$INSPECTOR_TOKEN")
BODY=$(extract_body "$RESP")
INSPECTOR_ID=$(echo "$BODY" | python3 -c "import sys,json; print(json.load(sys.stdin)['user_id'])" 2>/dev/null || echo "")

if [ -n "$INSPECTOR_ID" ]; then
    RESP=$(do_get "/api/users/$INSPECTOR_ID" "$INSPECTOR_TOKEN")
    STATUS=$(extract_status "$RESP")
    assert_status "Inspector can view own profile" 200 "$STATUS" ""
fi

# ── 4. Get other user as inspector (forbidden) ───────────────

RESP=$(do_get "/api/auth/session" "$ADMIN_TOKEN")
BODY=$(extract_body "$RESP")
ADMIN_ID=$(echo "$BODY" | python3 -c "import sys,json; print(json.load(sys.stdin)['user_id'])" 2>/dev/null || echo "")

if [ -n "$ADMIN_ID" ] && [ -n "$INSPECTOR_TOKEN" ]; then
    RESP=$(do_get "/api/users/$ADMIN_ID" "$INSPECTOR_TOKEN")
    STATUS=$(extract_status "$RESP")
    assert_status "Inspector cannot view other user" 403 "$STATUS" ""
fi

# ── 5. List users without auth ───────────────────────────────

RESP=$(do_get "/api/users" "")
STATUS=$(extract_status "$RESP")
assert_status "List users without auth returns 401" 401 "$STATUS" ""

print_summary "User Management"
exit $FAILED
