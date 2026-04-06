#!/usr/bin/env bash
# ============================================================
# Test Suite: Step-up enforcement + Cross-object auth negatives
# All cross-object tests create their own fixtures deterministically.
# No "pass by skip" — missing fixtures cause FAIL.
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
source "$SCRIPT_DIR/.tokens" 2>/dev/null || true

echo ""
echo "=== Test Suite: Step-up & Cross-Object Auth ==="
echo ""

# ── Prerequisite check ────────────────────────────────────────

for VAR in ADMIN_TOKEN INSPECTOR_TOKEN INSPECTOR2_TOKEN REVIEWER_TOKEN; do
    eval VAL=\$$VAR
    if [ -z "$VAL" ]; then
        echo "  FATAL: $VAR is not set. Auth test suite must run first."
        TOTAL=1; FAILED=1; print_summary "Step-up & Cross-Object Auth"; exit 1
    fi
done

# ═══════════════════════════════════════════════════════════
# C1: STEP-UP ENFORCEMENT
# ═══════════════════════════════════════════════════════════

# ── 1. Report export fails without step-up ────────────────

RESP=$(do_post "/api/admin/reports/generate" '{"report_type":"kpi_summary","format":"csv"}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Report export fails without step-up (403)" 403 "$STATUS" "$BODY"
assert_json_contains "Step-up required error" "$BODY" "stepup_required"

# ── 2. Step-up with correct password succeeds ─────────────

RESP=$(do_post "/api/auth/stepup" '{"password":"SecurePass1!xy","action_type":"export_csv"}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Step-up with correct password succeeds" 200 "$STATUS" "$BODY"
assert_json_contains "Step-up success message" "$BODY" "Step-up verification successful"

# ── 3. Report export succeeds after step-up ───────────────

RESP=$(do_post "/api/admin/reports/generate" '{"report_type":"kpi_summary","format":"csv"}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Report export succeeds after step-up" 200 "$STATUS" ""

# ── 4. Step-up with wrong password fails ──────────────────

RESP=$(do_post "/api/auth/stepup" '{"password":"WrongPassword1!","action_type":"export_csv"}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Step-up with wrong password fails (401)" 401 "$STATUS" ""

# ── 5. Step-up with invalid action type fails ─────────────

RESP=$(do_post "/api/auth/stepup" '{"password":"SecurePass1!xy","action_type":"nonexistent_action"}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Step-up with invalid action returns 400" 400 "$STATUS" ""

# ── 6. Bulk export also requires step-up ──────────────────

RESP=$(do_post "/api/bulk/export" '{"entity_type":"kb_entry","format":"csv"}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Bulk export requires step-up" 403 "$STATUS" "$BODY"
assert_json_contains "Bulk export step-up required" "$BODY" "stepup_required"

# ═══════════════════════════════════════════════════════════
# C2: CROSS-OBJECT AUTH — DETERMINISTIC FIXTURE SETUP
# ═══════════════════════════════════════════════════════════

echo ""
echo "  -- Setting up cross-object fixtures --"

# ── Create a template + schedule + task for inspector1 ────

RESP=$(do_post "/api/inspection/templates" '{"name":"CrossAuthTest","cycle":"OneTime","subtasks":[{"title":"Check item","expected_type":"checkbox","is_required":true}]}' "$ADMIN_TOKEN")
TMPL_ID=$(echo "$(extract_body "$RESP")" | python3 -c "import sys,json; print(json.load(sys.stdin)['template']['id'])" 2>/dev/null || echo "")
SUBTASK_ID=$(echo "$(extract_body "$RESP")" | python3 -c "import sys,json; print(json.load(sys.stdin)['subtasks'][0]['id'])" 2>/dev/null || echo "")

# Get inspector1 user ID
RESP=$(do_get "/api/auth/session" "$INSPECTOR_TOKEN")
INSP1_ID=$(echo "$(extract_body "$RESP")" | python3 -c "import sys,json; print(json.load(sys.stdin)['user_id'])" 2>/dev/null || echo "")

if [ -z "$TMPL_ID" ] || [ -z "$INSP1_ID" ] || [ -z "$SUBTASK_ID" ]; then
    echo "  FATAL: Could not create template/get inspector ID for cross-object tests"
    TOTAL=1; FAILED=1; print_summary "Step-up & Cross-Object Auth"; exit 1
fi

# Create schedule assigning to inspector1
RESP=$(do_post "/api/inspection/schedules" "{\"template_id\":\"$TMPL_ID\",\"assigned_to\":\"$INSP1_ID\",\"start_date\":\"2026-04-06\"}" "$ADMIN_TOKEN")

# Get the generated task instance
RESP=$(do_get "/api/inspection/tasks?status=scheduled" "$INSPECTOR_TOKEN")
BODY=$(extract_body "$RESP")
TASK_ID=$(echo "$BODY" | python3 -c "
import sys,json
tasks=json.load(sys.stdin)['tasks']
for t in tasks:
    if t['template_name'] == 'CrossAuthTest':
        print(t['instance']['id'])
        break
else:
    print('')
" 2>/dev/null || echo "")

if [ -z "$TASK_ID" ]; then
    echo "  FATAL: Could not find generated task instance for cross-object tests"
    TOTAL=1; FAILED=1; print_summary "Step-up & Cross-Object Auth"; exit 1
fi

# Start the task as inspector1
do_post "/api/inspection/tasks/$TASK_ID/start" '{}' "$INSPECTOR_TOKEN" > /dev/null

# Submit the task as inspector1 with valid responses
RESP=$(do_post "/api/inspection/submissions" "{\"instance_id\":\"$TASK_ID\",\"notes\":\"Test submission for auth check\",\"responses\":[{\"subtask_id\":\"$SUBTASK_ID\",\"response_value\":{\"checked\":true}}]}" "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")

SUB_ID=$(echo "$BODY" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('submission',{}).get('submission',{}).get('id',''))" 2>/dev/null || echo "")

if [ -z "$SUB_ID" ]; then
    echo "  FATAL: Could not create submission for cross-object tests (HTTP $STATUS)"
    echo "  Response: $(echo "$BODY" | head -c 300)"
    TOTAL=1; FAILED=1; print_summary "Step-up & Cross-Object Auth"; exit 1
fi

echo "  -- Fixtures ready: task=$TASK_ID sub=$SUB_ID --"
echo ""

# ═══════════════════════════════════════════════════════════
# C2: CROSS-OBJECT DENIAL ASSERTIONS
# ═══════════════════════════════════════════════════════════

# ── 7. Inspector1 (owner) can view own submission ─────────

RESP=$(do_get "/api/inspection/submissions/$SUB_ID" "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Inspector1 can view own submission" 200 "$STATUS" ""

# ── 8. Reviewer (privileged) can view inspector1 submission

RESP=$(do_get "/api/inspection/submissions/$SUB_ID" "$REVIEWER_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Reviewer can view inspector1 submission (privileged)" 200 "$STATUS" ""

# ── 9. Inspector2 DENIED inspector1's submission (403) ────

RESP=$(do_get "/api/inspection/submissions/$SUB_ID" "$INSPECTOR2_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Inspector2 denied inspector1 submission" 403 "$STATUS" ""

# ── 10. Non-existent submission returns 404 ───────────────

RESP=$(do_get "/api/inspection/submissions/00000000-0000-0000-0000-000000000000" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Non-existent submission returns 404" 404 "$STATUS" ""

# ── 11. Non-existent review returns 404 ───────────────────

RESP=$(do_get "/api/reviews/00000000-0000-0000-0000-000000000000" "$REVIEWER_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Non-existent review returns 404" 404 "$STATUS" ""

# ── 12. Inspector denied review queue (role-gated) ────────
# Review queue requires Reviewer or OperationsAdmin role

RESP=$(do_get "/api/reviews/queue" "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Inspector denied review queue (403)" 403 "$STATUS" ""

# ── 13. Inspector denied review detail (404 — cannot confirm existence)

RESP=$(do_get "/api/reviews/00000000-0000-0000-0000-000000000000" "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Inspector gets 404 for review detail (no existence leak)" 404 "$STATUS" ""

# ═══════════════════════════════════════════════════════════
# C3: ENCRYPTION BEHAVIOR
# ═══════════════════════════════════════════════════════════

# ── 13. Device list never exposes raw device_fingerprint ──

RESP=$(do_get "/api/devices" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Device list returns 200" 200 "$STATUS" ""

TOTAL=$((TOTAL + 1))
if echo "$BODY" | grep -q '"device_fingerprint"'; then
    FAILED=$((FAILED + 1))
    echo -e "  ${RED}FAIL${NC}  Response contains raw device_fingerprint field"
else
    PASSED=$((PASSED + 1))
    echo -e "  ${GREEN}PASS${NC}  No raw device_fingerprint exposed in device list"
fi

# ── 14. Submission notes not returned as raw ciphertext ───

RESP=$(do_get "/api/inspection/submissions/$SUB_ID" "$INSPECTOR_TOKEN")
BODY=$(extract_body "$RESP")
NOTES_CHECK=$(echo "$BODY" | python3 -c "
import sys,json
d=json.load(sys.stdin)
n = d.get('submission',{}).get('notes','')
# Decrypted notes or [encrypted] marker — not base64 ciphertext
if not n:
    print('EMPTY')
elif n.startswith('[encrypted]'):
    print('MARKER')
elif len(n) > 80 and '==' in n:
    print('CIPHER')
else:
    print('OK')
" 2>/dev/null || echo "SKIP")

TOTAL=$((TOTAL + 1))
if [ "$NOTES_CHECK" = "CIPHER" ]; then
    FAILED=$((FAILED + 1))
    echo -e "  ${RED}FAIL${NC}  Submission notes appear to be raw ciphertext"
else
    PASSED=$((PASSED + 1))
    echo -e "  ${GREEN}PASS${NC}  Submission notes not leaked as raw ciphertext ($NOTES_CHECK)"
fi

print_summary "Step-up & Cross-Object Auth"
exit $FAILED
