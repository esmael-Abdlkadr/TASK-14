#!/usr/bin/env bash
# ============================================================
# Test Suite: Audit Log
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
source "$SCRIPT_DIR/.tokens" 2>/dev/null || true

echo ""
echo "=== Test Suite: Audit Log ==="
echo ""

# ── 1. Query audit log ───────────────────────────────────────

RESP=$(do_get "/api/audit" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Query audit log" 200 "$STATUS" "$BODY"
assert_json_contains "Has entries array" "$BODY" "entries"
assert_json_contains "Has total count" "$BODY" "total"

# ── 2. Audit log has entries from previous tests ─────────────

assert_json_contains "Audit captured user_registered" "$BODY" "user_registered"

# ── 3. Audit log as inspector (forbidden) ────────────────────

RESP=$(do_get "/api/audit" "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Inspector cannot view audit log" 403 "$STATUS" ""

# ── 4. Check audit chain integrity ───────────────────────────

RESP=$(do_get "/api/audit/integrity" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Check audit integrity" 200 "$STATUS" "$BODY"
assert_json_contains "Chain valid field present" "$BODY" "chain_valid"

# ── 5. Manager can view audit ────────────────────────────────

RESP=$(do_get "/api/audit" "$MANAGER_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Manager can view audit log" 200 "$STATUS" ""

# ── 6. Integrity check as inspector (forbidden) ──────────────

RESP=$(do_get "/api/audit/integrity" "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Inspector cannot check integrity" 403 "$STATUS" ""

print_summary "Audit Log"
exit $FAILED
