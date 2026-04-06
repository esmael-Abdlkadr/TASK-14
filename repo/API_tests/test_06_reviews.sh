#!/usr/bin/env bash
# ============================================================
# Test Suite: Review Workspace & Scorecards
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
source "$SCRIPT_DIR/.tokens" 2>/dev/null || true

echo ""
echo "=== Test Suite: Reviews ==="
echo ""

# ── 1. Create scorecard ──────────────────────────────────────

RESP=$(do_post "/api/reviews/scorecards" '{"name":"Inspection Scorecard","target_type":"InspectionSubmission","passing_score":3.0,"dimensions":[{"name":"Completeness","weight":2.0,"comment_required":false,"comment_required_below":3},{"name":"Accuracy","weight":3.0,"comment_required":false},{"name":"Timeliness","weight":1.0,"comment_required":false}]}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Create scorecard" 201 "$STATUS" "$BODY"
assert_json_contains "Scorecard has name" "$BODY" "Inspection Scorecard"

SCORECARD_ID=$(echo "$BODY" | python3 -c "import sys,json; print(json.load(sys.stdin)['scorecard']['id'])" 2>/dev/null || echo "")

# ── 2. List scorecards ───────────────────────────────────────

RESP=$(do_get "/api/reviews/scorecards" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "List scorecards" 200 "$STATUS" ""

# ── 3. Get scorecard detail ──────────────────────────────────

if [ -n "$SCORECARD_ID" ]; then
    RESP=$(do_get "/api/reviews/scorecards/$SCORECARD_ID" "$ADMIN_TOKEN")
    STATUS=$(extract_status "$RESP")
    BODY=$(extract_body "$RESP")
    assert_status "Get scorecard detail" 200 "$STATUS" "$BODY"
    assert_json_contains "Has dimensions" "$BODY" "dimensions"
    assert_json_contains "Has consistency_rules" "$BODY" "consistency_rules"
fi

# ── 4. Create scorecard as inspector (forbidden) ─────────────

RESP=$(do_post "/api/reviews/scorecards" '{"name":"Test","target_type":"InspectionSubmission"}' "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Inspector cannot create scorecard" 403 "$STATUS" ""

# ── 5. Get review queue (empty) ──────────────────────────────

RESP=$(do_get "/api/reviews/queue" "$REVIEWER_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Reviewer queue returns 200" 200 "$STATUS" "$BODY"
assert_json_contains "Queue has assignments array" "$BODY" "assignments"

# ── 6. Declare COI ───────────────────────────────────────────

RESP=$(do_post "/api/reviews/coi" '{"conflict_type":"department","department":"sanitation","description":"Same department"}' "$REVIEWER_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Declare COI" 201 "$STATUS" ""

# ── 7. List COI ──────────────────────────────────────────────

RESP=$(do_get "/api/reviews/coi" "$REVIEWER_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "List COI" 200 "$STATUS" "$BODY"
assert_json_contains "COI has department" "$BODY" "department"

# ── 8. Get nonexistent review ────────────────────────────────

RESP=$(do_get "/api/reviews/00000000-0000-0000-0000-000000000000" "$REVIEWER_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Get nonexistent review returns 404" 404 "$STATUS" ""

echo "SCORECARD_ID=$SCORECARD_ID" >> "$SCRIPT_DIR/.tokens"

print_summary "Reviews"
exit $FAILED
