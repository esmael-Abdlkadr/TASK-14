#!/usr/bin/env bash
# ============================================================
# Test Suite: Inspection Tasks & Scheduling
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
source "$SCRIPT_DIR/.tokens" 2>/dev/null || true

echo ""
echo "=== Test Suite: Inspection Tasks ==="
echo ""

# ── 1. Create task template ──────────────────────────────────

RESP=$(do_post "/api/inspection/templates" '{"name":"Weekly Dumpster Area","description":"Check dumpster area","group_name":"Weekly HOA Dumpster Area","cycle":"Weekly","time_window_start":"08:00","time_window_end":"18:00","allowed_misses":1,"miss_window_days":30,"makeup_allowed":true,"makeup_deadline_hours":48,"subtasks":[{"title":"Check signage","expected_type":"checkbox","is_required":true},{"title":"Check contamination","expected_type":"select","is_required":true,"options":{"choices":["clean","minor","major"]}},{"title":"Bin condition","expected_type":"text","is_required":true},{"title":"Temperature","expected_type":"number","is_required":false,"options":{"min":0,"max":120}}]}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Create task template" 201 "$STATUS" "$BODY"
assert_json_contains "Template has name" "$BODY" "Weekly Dumpster Area"

TEMPLATE_ID=$(echo "$BODY" | python3 -c "import sys,json; print(json.load(sys.stdin)['template']['id'])" 2>/dev/null || echo "")

# ── 2. List templates ────────────────────────────────────────

RESP=$(do_get "/api/inspection/templates" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "List templates" 200 "$STATUS" ""

# ── 3. Get template detail ───────────────────────────────────

if [ -n "$TEMPLATE_ID" ]; then
    RESP=$(do_get "/api/inspection/templates/$TEMPLATE_ID" "$ADMIN_TOKEN")
    STATUS=$(extract_status "$RESP")
    BODY=$(extract_body "$RESP")
    assert_status "Get template by ID" 200 "$STATUS" "$BODY"
    assert_json_contains "Template has subtasks" "$BODY" "subtasks"
fi

# ── 4. Create template as inspector (forbidden) ──────────────

RESP=$(do_post "/api/inspection/templates" '{"name":"Test","cycle":"Daily"}' "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Inspector cannot create template" 403 "$STATUS" ""

# ── 5. Create schedule ───────────────────────────────────────

# Get inspector's user ID
RESP=$(do_get "/api/auth/session" "$INSPECTOR_TOKEN")
INSPECTOR_ID=$(echo "$(extract_body "$RESP")" | python3 -c "import sys,json; print(json.load(sys.stdin)['user_id'])" 2>/dev/null || echo "")

if [ -n "$TEMPLATE_ID" ] && [ -n "$INSPECTOR_ID" ]; then
    RESP=$(do_post "/api/inspection/schedules" "{\"template_id\":\"$TEMPLATE_ID\",\"assigned_to\":\"$INSPECTOR_ID\",\"start_date\":\"2026-04-01\"}" "$ADMIN_TOKEN")
    STATUS=$(extract_status "$RESP")
    BODY=$(extract_body "$RESP")
    assert_status "Create schedule" 201 "$STATUS" "$BODY"
    assert_json_contains "Schedule generates instances" "$BODY" "instances_generated"
fi

# ── 6. List tasks as inspector ───────────────────────────────

RESP=$(do_get "/api/inspection/tasks" "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "List tasks as inspector" 200 "$STATUS" "$BODY"
assert_json_contains "Tasks response has tasks array" "$BODY" "tasks"

TASK_ID=$(echo "$BODY" | python3 -c "import sys,json; tasks=json.load(sys.stdin)['tasks']; print(tasks[0]['instance']['id'] if tasks else '')" 2>/dev/null || echo "")

# ── 7. Get task detail ───────────────────────────────────────

if [ -n "$TASK_ID" ]; then
    RESP=$(do_get "/api/inspection/tasks/$TASK_ID" "$INSPECTOR_TOKEN")
    STATUS=$(extract_status "$RESP")
    BODY=$(extract_body "$RESP")
    assert_status "Get task detail" 200 "$STATUS" "$BODY"
    assert_json_contains "Task has template_name" "$BODY" "template_name"
fi

# ── 8. Start task ────────────────────────────────────────────

if [ -n "$TASK_ID" ]; then
    RESP=$(do_post "/api/inspection/tasks/$TASK_ID/start" '{}' "$INSPECTOR_TOKEN")
    STATUS=$(extract_status "$RESP")
    assert_status "Start task" 200 "$STATUS" ""
fi

# ── 9. Submit task - missing required fields ─────────────────

if [ -n "$TASK_ID" ]; then
    RESP=$(do_post "/api/inspection/submissions" "{\"instance_id\":\"$TASK_ID\",\"responses\":[]}" "$INSPECTOR_TOKEN")
    STATUS=$(extract_status "$RESP")
    BODY=$(extract_body "$RESP")
    assert_status "Submit with missing fields returns 422" 422 "$STATUS" "$BODY"
    assert_json_contains "Validation returns errors" "$BODY" "errors"
fi

# ── 10. List reminders ───────────────────────────────────────

RESP=$(do_get "/api/inspection/reminders" "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "List reminders" 200 "$STATUS" ""

# ── 11. Filter tasks by status ───────────────────────────────

RESP=$(do_get "/api/inspection/tasks?status=in_progress" "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Filter tasks by status" 200 "$STATUS" ""

# ── 12. List tasks without auth ──────────────────────────────

RESP=$(do_get "/api/inspection/tasks" "")
STATUS=$(extract_status "$RESP")
assert_status "Tasks without auth returns 401" 401 "$STATUS" ""

echo "TEMPLATE_ID=$TEMPLATE_ID" >> "$SCRIPT_DIR/.tokens"
echo "TASK_ID=$TASK_ID" >> "$SCRIPT_DIR/.tokens"

print_summary "Inspection Tasks"
exit $FAILED
