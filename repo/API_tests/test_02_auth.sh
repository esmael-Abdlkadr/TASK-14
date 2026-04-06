#!/usr/bin/env bash
# ============================================================
# Test Suite: Authentication & User Management
# Tests registration lockdown, login, session, step-up flows
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"

echo ""
echo "=== Test Suite: Authentication ==="
echo ""

# ── 1. Bootstrap register (first user, CIVICSORT_BOOTSTRAP_ADMIN=1) ──
# The test environment must have CIVICSORT_BOOTSTRAP_ADMIN=1 and empty DB.
# First register creates admin without auth.

RESP=$(do_post "/api/auth/register" '{"username":"testadmin","password":"SecurePass1!xy","role":"OperationsAdmin"}' "")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Bootstrap admin register" 201 "$STATUS" "$BODY"
assert_json_contains "Register returns username" "$BODY" "testadmin"

# ── 2. Anonymous register now denied (DB has users) ──────────

RESP=$(do_post "/api/auth/register" '{"username":"anon_attempt","password":"SecurePass1!xy","role":"FieldInspector"}' "")
STATUS=$(extract_status "$RESP")
assert_status "Anonymous register denied after bootstrap" 401 "$STATUS" ""

# ── 3. Login admin ────────────────────────────────────────────

RESP=$(do_post "/api/auth/login" '{"username":"testadmin","password":"SecurePass1!xy"}' "")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Login with valid credentials" 200 "$STATUS" "$BODY"
assert_json_contains "Login returns session_token" "$BODY" "session_token"

ADMIN_TOKEN=$(echo "$BODY" | python3 -c "import sys,json; print(json.load(sys.stdin)['session_token'])" 2>/dev/null || echo "")

# ── 4. Admin-authenticated register: inspector ───────────────

RESP=$(do_post "/api/auth/register" '{"username":"inspector1","password":"InspectorPass1!","role":"FieldInspector"}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Admin registers inspector" 201 "$STATUS" ""

# ── 5. Admin-authenticated register: reviewer ────────────────

RESP=$(do_post "/api/auth/register" '{"username":"reviewer1","password":"ReviewerPass1!","role":"Reviewer"}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Admin registers reviewer" 201 "$STATUS" ""

# ── 6. Admin-authenticated register: manager ─────────────────

RESP=$(do_post "/api/auth/register" '{"username":"manager1","password":"ManagerPass1!xx","role":"DepartmentManager"}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Admin registers manager" 201 "$STATUS" ""

# ── 7. Inspector cannot register users ────────────────────────

RESP=$(do_post "/api/auth/login" '{"username":"inspector1","password":"InspectorPass1!"}' "")
INSPECTOR_TOKEN_TEMP=$(echo "$(extract_body "$RESP")" | python3 -c "import sys,json; print(json.load(sys.stdin)['session_token'])" 2>/dev/null || echo "")

RESP=$(do_post "/api/auth/register" '{"username":"unauthorized","password":"SecurePass1!xy","role":"FieldInspector"}' "$INSPECTOR_TOKEN_TEMP")
STATUS=$(extract_status "$RESP")
assert_status "Inspector cannot register users" 403 "$STATUS" ""

# ── 8. Register - password too short ─────────────────────────

RESP=$(do_post "/api/auth/register" '{"username":"shortpw","password":"Short1!","role":"FieldInspector"}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Register with short password returns 400" 400 "$STATUS" ""

# ── 9. Register - missing password requirements ──────────────

RESP=$(do_post "/api/auth/register" '{"username":"nouppercase","password":"alllowercase1!xx","role":"FieldInspector"}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Register without uppercase returns 400" 400 "$STATUS" ""

# ── 10. Login - wrong password ───────────────────────────────

RESP=$(do_post "/api/auth/login" '{"username":"testadmin","password":"WrongPassword1!"}' "")
STATUS=$(extract_status "$RESP")
assert_status "Login with wrong password returns 401" 401 "$STATUS" ""

# ── 11. Login - nonexistent user ─────────────────────────────

RESP=$(do_post "/api/auth/login" '{"username":"nonexistent","password":"Whatever1!xxxx"}' "")
STATUS=$(extract_status "$RESP")
assert_status "Login nonexistent user returns 401" 401 "$STATUS" ""

# ── 12. Session check - valid token ──────────────────────────

if [ -n "$ADMIN_TOKEN" ]; then
    RESP=$(do_get "/api/auth/session" "$ADMIN_TOKEN")
    STATUS=$(extract_status "$RESP")
    BODY=$(extract_body "$RESP")
    assert_status "GET session with valid token" 200 "$STATUS" "$BODY"
    assert_json_contains "Session returns username" "$BODY" "testadmin"
fi

# ── 13. Session check - invalid token ────────────────────────

RESP=$(do_get "/api/auth/session" "invalid-token-value")
STATUS=$(extract_status "$RESP")
assert_status "GET session with invalid token returns 401" 401 "$STATUS" ""

# ── 14. Logout + session invalidation ────────────────────────

if [ -n "$ADMIN_TOKEN" ]; then
    RESP=$(do_post "/api/auth/logout" '{}' "$ADMIN_TOKEN")
    STATUS=$(extract_status "$RESP")
    assert_status "Logout returns 200" 200 "$STATUS" ""

    RESP=$(do_get "/api/auth/session" "$ADMIN_TOKEN")
    STATUS=$(extract_status "$RESP")
    assert_status "Session invalid after logout" 401 "$STATUS" ""
fi

# ── 15. Missing body fields ──────────────────────────────────

RESP=$(do_post "/api/auth/login" '{}' "")
STATUS=$(extract_status "$RESP")
assert_status "Login with empty body returns 400" 400 "$STATUS" ""

# ── 16. Register second inspector for cross-user tests ───────

RESP=$(do_post "/api/auth/login" '{"username":"testadmin","password":"SecurePass1!xy"}' "")
TEMP_ADMIN=$(echo "$(extract_body "$RESP")" | python3 -c "import sys,json; print(json.load(sys.stdin)['session_token'])" 2>/dev/null || echo "")

RESP=$(do_post "/api/auth/register" '{"username":"inspector2","password":"InspectorTwo1!x","role":"FieldInspector"}' "$TEMP_ADMIN")
STATUS=$(extract_status "$RESP")
assert_status "Register second inspector" 201 "$STATUS" ""

# ── Save tokens for other test suites ────────────────────────

RESP=$(do_post "/api/auth/login" '{"username":"testadmin","password":"SecurePass1!xy"}' "")
ADMIN_TOKEN=$(echo "$(extract_body "$RESP")" | python3 -c "import sys,json; print(json.load(sys.stdin)['session_token'])" 2>/dev/null || echo "")

RESP=$(do_post "/api/auth/login" '{"username":"inspector1","password":"InspectorPass1!"}' "")
INSPECTOR_TOKEN=$(echo "$(extract_body "$RESP")" | python3 -c "import sys,json; print(json.load(sys.stdin)['session_token'])" 2>/dev/null || echo "")

RESP=$(do_post "/api/auth/login" '{"username":"reviewer1","password":"ReviewerPass1!"}' "")
REVIEWER_TOKEN=$(echo "$(extract_body "$RESP")" | python3 -c "import sys,json; print(json.load(sys.stdin)['session_token'])" 2>/dev/null || echo "")

RESP=$(do_post "/api/auth/login" '{"username":"manager1","password":"ManagerPass1!xx"}' "")
MANAGER_TOKEN=$(echo "$(extract_body "$RESP")" | python3 -c "import sys,json; print(json.load(sys.stdin)['session_token'])" 2>/dev/null || echo "")

RESP=$(do_post "/api/auth/login" '{"username":"inspector2","password":"InspectorTwo1!x"}' "")
INSPECTOR2_TOKEN=$(echo "$(extract_body "$RESP")" | python3 -c "import sys,json; print(json.load(sys.stdin)['session_token'])" 2>/dev/null || echo "")

echo "ADMIN_TOKEN=$ADMIN_TOKEN" > "$SCRIPT_DIR/.tokens"
echo "INSPECTOR_TOKEN=$INSPECTOR_TOKEN" >> "$SCRIPT_DIR/.tokens"
echo "INSPECTOR2_TOKEN=$INSPECTOR2_TOKEN" >> "$SCRIPT_DIR/.tokens"
echo "REVIEWER_TOKEN=$REVIEWER_TOKEN" >> "$SCRIPT_DIR/.tokens"
echo "MANAGER_TOKEN=$MANAGER_TOKEN" >> "$SCRIPT_DIR/.tokens"

print_summary "Authentication"
exit $FAILED
