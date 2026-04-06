#!/usr/bin/env bash
# ============================================================
# Test Suite: Messaging & Notification Center
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
source "$SCRIPT_DIR/.tokens" 2>/dev/null || true

echo ""
echo "=== Test Suite: Messaging ==="
echo ""

# ── 1. Create notification template ──────────────────────────

RESP=$(do_post "/api/messaging/templates" '{"name":"inspection_overdue","channel":"InApp","body_template":"Hello {{user_name}}, your inspection {{task_name}} is overdue since {{due_date}}.","subject_template":"Overdue: {{task_name}}","variables":[{"var_name":"user_name","var_type":"string","is_required":true},{"var_name":"task_name","var_type":"string","is_required":true},{"var_name":"due_date","var_type":"date","is_required":true}]}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Create notification template" 201 "$STATUS" "$BODY"
assert_json_contains "Template name" "$BODY" "inspection_overdue"

NOTIF_TEMPLATE_ID=$(echo "$BODY" | python3 -c "import sys,json; print(json.load(sys.stdin)['template']['id'])" 2>/dev/null || echo "")

# ── 2. List templates ────────────────────────────────────────

RESP=$(do_get "/api/messaging/templates" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "List notification templates" 200 "$STATUS" ""

# ── 3. Get template detail ───────────────────────────────────

if [ -n "$NOTIF_TEMPLATE_ID" ]; then
    RESP=$(do_get "/api/messaging/templates/$NOTIF_TEMPLATE_ID" "$ADMIN_TOKEN")
    STATUS=$(extract_status "$RESP")
    BODY=$(extract_body "$RESP")
    assert_status "Get template detail" 200 "$STATUS" "$BODY"
    assert_json_contains "Has variables" "$BODY" "variables"
fi

# ── 4. Create trigger rule ───────────────────────────────────

if [ -n "$NOTIF_TEMPLATE_ID" ]; then
    RESP=$(do_post "/api/messaging/triggers" "{\"name\":\"overdue_notification\",\"event\":\"InspectionOverdue\",\"template_id\":\"$NOTIF_TEMPLATE_ID\",\"channel\":\"InApp\"}" "$ADMIN_TOKEN")
    STATUS=$(extract_status "$RESP")
    assert_status "Create trigger rule" 201 "$STATUS" ""
fi

# ── 5. List triggers ─────────────────────────────────────────

RESP=$(do_get "/api/messaging/triggers" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "List triggers" 200 "$STATUS" ""

# ── 6. Fire event ────────────────────────────────────────────

RESP=$(do_get "/api/auth/session" "$INSPECTOR_TOKEN")
INSPECTOR_ID=$(echo "$(extract_body "$RESP")" | python3 -c "import sys,json; print(json.load(sys.stdin)['user_id'])" 2>/dev/null || echo "")

if [ -n "$INSPECTOR_ID" ]; then
    RESP=$(do_post "/api/messaging/fire" "{\"event\":\"InspectionOverdue\",\"payload\":{\"user_name\":\"inspector1\",\"task_name\":\"Dumpster Check\",\"due_date\":\"2026-04-01\"},\"recipient_user_id\":\"$INSPECTOR_ID\"}" "$ADMIN_TOKEN")
    STATUS=$(extract_status "$RESP")
    BODY=$(extract_body "$RESP")
    assert_status "Fire event" 200 "$STATUS" "$BODY"
    assert_json_contains "Event result has rules_matched" "$BODY" "rules_matched"
fi

# ── 7. Get notifications inbox ───────────────────────────────

RESP=$(do_get "/api/messaging/notifications" "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Get notification inbox" 200 "$STATUS" "$BODY"
assert_json_contains "Has unread_count" "$BODY" "unread_count"

# ── 8. Get payload queue ─────────────────────────────────────

RESP=$(do_get "/api/messaging/payloads" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Get payload queue" 200 "$STATUS" ""

# ── 9. Template creation as inspector (forbidden) ────────────

RESP=$(do_post "/api/messaging/templates" '{"name":"test","channel":"InApp","body_template":"test"}' "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Inspector cannot create template" 403 "$STATUS" ""

# ── 11. Inspector cannot list templates (restricted) ─────────

RESP=$(do_get "/api/messaging/templates" "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Inspector cannot list templates" 403 "$STATUS" ""

# ── 12. Inspector cannot list triggers (restricted) ──────────

RESP=$(do_get "/api/messaging/triggers" "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Inspector cannot list triggers" 403 "$STATUS" ""

# ── 13. Inspector cannot fire events (restricted) ────────────

RESP=$(do_post "/api/messaging/fire" '{"event":"Custom","payload":{}}' "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Inspector cannot fire events" 403 "$STATUS" ""

# ── 14. Inspector cannot access payload queue ────────────────

RESP=$(do_get "/api/messaging/payloads" "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Inspector cannot access payloads" 403 "$STATUS" ""

# ── 10. Mark all notifications read ──────────────────────────

RESP=$(do_post "/api/messaging/notifications/read-all" '{}' "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Mark all read" 200 "$STATUS" ""

print_summary "Messaging"
exit $FAILED
