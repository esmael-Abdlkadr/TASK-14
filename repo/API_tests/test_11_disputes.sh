#!/usr/bin/env bash
# ============================================================
# Test Suite: Disputed Classification Flow
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
source "$SCRIPT_DIR/.tokens" 2>/dev/null || true

echo ""
echo "=== Test Suite: Disputed Classifications ==="
echo ""

# ── 1. Create a dispute as inspector ─────────────────────────

# First we need a KB entry ID
RESP=$(do_get "/api/kb/search?q=Plastic" "$ADMIN_TOKEN")
BODY=$(extract_body "$RESP")
KB_ID=$(echo "$BODY" | python3 -c "import sys,json; r=json.load(sys.stdin)['results']; print(r[0]['entry_id'] if r else '')" 2>/dev/null || echo "")

if [ -n "$KB_ID" ]; then
    RESP=$(do_post "/api/disputes" "{\"kb_entry_id\":\"$KB_ID\",\"reason\":\"This item should be composted not recycled\",\"proposed_category\":\"compost\"}" "$INSPECTOR_TOKEN")
    STATUS=$(extract_status "$RESP")
    BODY=$(extract_body "$RESP")
    assert_status "Inspector creates dispute" 201 "$STATUS" "$BODY"
    assert_json_contains "Dispute has reason" "$BODY" "reason"

    DISPUTE_ID=$(echo "$BODY" | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])" 2>/dev/null || echo "")
else
    TOTAL=$((TOTAL + 1)); FAILED=$((FAILED + 1))
    echo "  FAIL  No KB entry found for dispute test"
    DISPUTE_ID=""
fi

# ── 2. List disputes ─────────────────────────────────────────

RESP=$(do_get "/api/disputes" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Admin lists disputes" 200 "$STATUS" ""

# ── 3. Get dispute detail ────────────────────────────────────

if [ -n "$DISPUTE_ID" ]; then
    RESP=$(do_get "/api/disputes/$DISPUTE_ID" "$ADMIN_TOKEN")
    STATUS=$(extract_status "$RESP")
    BODY=$(extract_body "$RESP")
    assert_status "Get dispute detail" 200 "$STATUS" "$BODY"
    assert_json_contains "Dispute has kb_entry_name" "$BODY" "kb_entry_name"
fi

# ── 4. Resolve dispute ───────────────────────────────────────

if [ -n "$DISPUTE_ID" ]; then
    RESP=$(do_put "/api/disputes/$DISPUTE_ID/resolve" '{"status":"Resolved","resolution_notes":"Verified - item stays as recyclable"}' "$ADMIN_TOKEN")
    STATUS=$(extract_status "$RESP")
    assert_status "Resolve dispute" 200 "$STATUS" ""
fi

# ── 5. Inspector cannot list disputes (forbidden for inspectors) ──

RESP=$(do_get "/api/disputes" "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Inspector cannot list disputes" 403 "$STATUS" ""

# ── 6. Create dispute without auth fails ─────────────────────

RESP=$(do_post "/api/disputes" '{"kb_entry_id":"00000000-0000-0000-0000-000000000000","reason":"test"}' "")
STATUS=$(extract_status "$RESP")
assert_status "Dispute without auth returns 401" 401 "$STATUS" ""

# ── 7. Cross-user dispute detail denied ──────────────────────
# Create a new dispute as admin, try to view as inspector (who didn't create it)

RESP=$(do_get "/api/auth/session" "$ADMIN_TOKEN")
ADMIN_ID=$(echo "$(extract_body "$RESP")" | python3 -c "import sys,json; print(json.load(sys.stdin)['user_id'])" 2>/dev/null || echo "")

if [ -n "$KB_ID" ] && [ -n "$ADMIN_ID" ]; then
    RESP=$(do_post "/api/disputes" "{\"kb_entry_id\":\"$KB_ID\",\"reason\":\"Admin created dispute\"}" "$ADMIN_TOKEN")
    ADMIN_DISPUTE_ID=$(echo "$(extract_body "$RESP")" | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])" 2>/dev/null || echo "")

    if [ -n "$ADMIN_DISPUTE_ID" ]; then
        # Inspector trying to access admin's dispute should be denied
        RESP=$(do_get "/api/disputes/$ADMIN_DISPUTE_ID" "$INSPECTOR_TOKEN")
        STATUS=$(extract_status "$RESP")
        assert_status "Inspector cannot view other user dispute detail" 403 "$STATUS" ""
    fi
fi

echo "DISPUTE_ID=$DISPUTE_ID" >> "$SCRIPT_DIR/.tokens"

print_summary "Disputed Classifications"
exit $FAILED
