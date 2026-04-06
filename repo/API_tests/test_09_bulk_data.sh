#!/usr/bin/env bash
# ============================================================
# Test Suite: Bulk Data Management & Deduplication
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
source "$SCRIPT_DIR/.tokens" 2>/dev/null || true

echo ""
echo "=== Test Suite: Bulk Data ==="
echo ""

# ── 1. Start import job ──────────────────────────────────────

RESP=$(do_post "/api/bulk/import" '{"name":"Test KB Import","entity_type":"kb_entry","rows":[{"item_name":"Glass Jar","disposal_category":"recyclable","disposal_instructions":"Rinse and recycle"},{"item_name":"Styrofoam Cup","disposal_category":"landfill","disposal_instructions":"Place in grey bin"},{"item_name":"","disposal_category":"","disposal_instructions":""}]}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Start import job" 201 "$STATUS" "$BODY"
assert_json_contains "Import has job" "$BODY" "job"

IMPORT_JOB_ID=$(echo "$BODY" | python3 -c "import sys,json; print(json.load(sys.stdin)['job']['id'])" 2>/dev/null || echo "")
ERRORS=$(echo "$BODY" | python3 -c "import sys,json; print(json.load(sys.stdin)['errors_found'])" 2>/dev/null || echo "0")
assert_json_contains "Detects validation errors" "$BODY" "errors_found"

# ── 2. List import jobs ──────────────────────────────────────

RESP=$(do_get "/api/bulk/import" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "List import jobs" 200 "$STATUS" ""

# ── 3. Get import job detail ─────────────────────────────────

if [ -n "$IMPORT_JOB_ID" ]; then
    RESP=$(do_get "/api/bulk/import/$IMPORT_JOB_ID" "$ADMIN_TOKEN")
    STATUS=$(extract_status "$RESP")
    BODY=$(extract_body "$RESP")
    assert_status "Get import job detail" 200 "$STATUS" "$BODY"
    assert_json_contains "Has rows" "$BODY" "rows"
fi

# ── 4. Execute import ────────────────────────────────────────

if [ -n "$IMPORT_JOB_ID" ]; then
    RESP=$(do_post "/api/bulk/import/$IMPORT_JOB_ID/execute" '{}' "$ADMIN_TOKEN")
    STATUS=$(extract_status "$RESP")
    BODY=$(extract_body "$RESP")
    assert_status "Execute import" 200 "$STATUS" "$BODY"
    assert_json_contains "Import completed" "$BODY" "completed"
fi

# ── 5. Get change history ────────────────────────────────────

RESP=$(do_get "/api/bulk/changes" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Get change history" 200 "$STATUS" "$BODY"
assert_json_contains "Has changes array" "$BODY" "changes"

# ── 6. Get change history filtered ───────────────────────────

RESP=$(do_get "/api/bulk/changes?entity_type=kb_entry" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Filter changes by entity_type" 200 "$STATUS" ""

# ── 7. List duplicates ───────────────────────────────────────

RESP=$(do_get "/api/bulk/duplicates" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "List duplicates" 200 "$STATUS" ""

# ── 8. List merge requests ───────────────────────────────────

RESP=$(do_get "/api/bulk/merges" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "List merge requests" 200 "$STATUS" ""

# ── 9. Import as inspector (forbidden) ───────────────────────

RESP=$(do_post "/api/bulk/import" '{"name":"test","entity_type":"kb_entry","rows":[]}' "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Inspector cannot import" 403 "$STATUS" ""

# ── 10. Import with empty rows ───────────────────────────────

RESP=$(do_post "/api/bulk/import" '{"name":"Empty","entity_type":"kb_entry","rows":[]}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Import with empty rows returns 400" 400 "$STATUS" ""

# ── 11. Reviewer cannot access bulk duplicates (auth fix) ─────

RESP=$(do_get "/api/bulk/duplicates" "$REVIEWER_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Reviewer cannot list duplicates" 403 "$STATUS" ""

# ── 12. Reviewer cannot access merge requests ────────────────

RESP=$(do_get "/api/bulk/merges" "$REVIEWER_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Reviewer cannot list merges" 403 "$STATUS" ""

# ── 13. Inspector cannot access change history ───────────────

RESP=$(do_get "/api/bulk/changes" "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Inspector cannot view change history" 403 "$STATUS" ""

print_summary "Bulk Data"
exit $FAILED
