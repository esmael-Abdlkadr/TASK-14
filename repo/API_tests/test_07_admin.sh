#!/usr/bin/env bash
# ============================================================
# Test Suite: Admin Console & KPI Dashboards
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
source "$SCRIPT_DIR/.tokens" 2>/dev/null || true

echo ""
echo "=== Test Suite: Admin Console ==="
echo ""

# ── 1. Dashboard KPIs ────────────────────────────────────────

RESP=$(do_get "/api/admin/dashboard" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Get dashboard KPIs" 200 "$STATUS" "$BODY"
assert_json_contains "Has sorting_conversion_rate" "$BODY" "sorting_conversion_rate"
assert_json_contains "Has retention_30d" "$BODY" "retention_30d"
assert_json_contains "Has active_users" "$BODY" "active_users"

# ── 2. Dashboard as inspector (forbidden) ────────────────────

RESP=$(do_get "/api/admin/dashboard" "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Inspector cannot access dashboard" 403 "$STATUS" ""

# ── 3. User overview ─────────────────────────────────────────

RESP=$(do_get "/api/admin/overview/users" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "User overview" 200 "$STATUS" "$BODY"
assert_json_contains "Has total_users" "$BODY" "total_users"
assert_json_contains "Has by_role" "$BODY" "by_role"

# ── 4. Item overview ─────────────────────────────────────────

RESP=$(do_get "/api/admin/overview/items" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Item overview" 200 "$STATUS" ""

# ── 5. Work order overview ───────────────────────────────────

RESP=$(do_get "/api/admin/overview/workorders" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Work order overview" 200 "$STATUS" "$BODY"
assert_json_contains "Has completion_rate" "$BODY" "completion_rate"

# ── 6. Create campaign ───────────────────────────────────────

RESP=$(do_post "/api/admin/campaigns" '{"name":"Spring Cleanup Week","description":"Annual spring cleanup campaign","start_date":"2026-04-15","end_date":"2026-04-22","target_region":"north","target_audience":"All inspectors"}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Create campaign" 201 "$STATUS" "$BODY"
assert_json_contains "Campaign name" "$BODY" "Spring Cleanup Week"

# ── 7. Create campaign - end before start ─────────────────────

RESP=$(do_post "/api/admin/campaigns" '{"name":"Bad Dates","start_date":"2026-04-22","end_date":"2026-04-15"}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Campaign with bad dates returns 400" 400 "$STATUS" ""

# ── 8. List campaigns ────────────────────────────────────────

RESP=$(do_get "/api/admin/campaigns" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "List campaigns" 200 "$STATUS" ""

# ── 9. Create tag ────────────────────────────────────────────

RESP=$(do_post "/api/admin/tags" '{"name":"recycling","color":"#22c55e"}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Create tag" 201 "$STATUS" ""

# ── 10. List tags ────────────────────────────────────────────

RESP=$(do_get "/api/admin/tags" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "List tags" 200 "$STATUS" ""

# ── 11. Manager can access dashboard ─────────────────────────

RESP=$(do_get "/api/admin/dashboard" "$MANAGER_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Manager can access dashboard" 200 "$STATUS" ""

print_summary "Admin Console"
exit $FAILED
