#!/usr/bin/env bash
# ============================================================
# Test Suite: Knowledge Base
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
source "$SCRIPT_DIR/.tokens" 2>/dev/null || true

echo ""
echo "=== Test Suite: Knowledge Base ==="
echo ""

# ── 1. Create category ───────────────────────────────────────

RESP=$(do_post "/api/kb/categories" '{"name":"Recyclables","description":"Items that can be recycled"}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Create category" 201 "$STATUS" "$BODY"
assert_json_contains "Category name returned" "$BODY" "Recyclables"

# ── 2. List categories ───────────────────────────────────────

RESP=$(do_get "/api/kb/categories" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "List categories" 200 "$STATUS" "$BODY"
assert_json_contains "Categories contains Recyclables" "$BODY" "Recyclables"

# ── 3. Create KB entry ───────────────────────────────────────

RESP=$(do_post "/api/kb/entries" '{"item_name":"Plastic Bottle","disposal_category":"recyclable","disposal_instructions":"Rinse and place in blue bin","region":"north","aliases":[{"alias":"water bottle"},{"alias":"plastik bottle","alias_type":"misspelling"}]}' "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Create KB entry" 201 "$STATUS" "$BODY"
assert_json_contains "Entry has item_name" "$BODY" "Plastic Bottle"

ENTRY_ID=$(echo "$BODY" | python3 -c "import sys,json; print(json.load(sys.stdin)['entry']['id'])" 2>/dev/null || echo "")

# ── 4. Get KB entry ──────────────────────────────────────────

if [ -n "$ENTRY_ID" ]; then
    RESP=$(do_get "/api/kb/entries/$ENTRY_ID" "$ADMIN_TOKEN")
    STATUS=$(extract_status "$RESP")
    BODY=$(extract_body "$RESP")
    assert_status "Get KB entry by ID" 200 "$STATUS" "$BODY"
    assert_json_contains "Entry details returned" "$BODY" "Plastic Bottle"
fi

# ── 5. Update KB entry (creates new version) ─────────────────

if [ -n "$ENTRY_ID" ]; then
    RESP=$(do_put "/api/kb/entries/$ENTRY_ID" '{"disposal_category":"recyclable","disposal_instructions":"Rinse thoroughly, remove cap, place in blue bin","change_summary":"Added cap removal instruction"}' "$ADMIN_TOKEN")
    STATUS=$(extract_status "$RESP")
    BODY=$(extract_body "$RESP")
    assert_status "Update KB entry creates new version" 200 "$STATUS" "$BODY"
    assert_json_contains "Version incremented" "$BODY" "version"
fi

# ── 6. Get version history ───────────────────────────────────

if [ -n "$ENTRY_ID" ]; then
    RESP=$(do_get "/api/kb/entries/$ENTRY_ID/versions" "$ADMIN_TOKEN")
    STATUS=$(extract_status "$RESP")
    BODY=$(extract_body "$RESP")
    assert_status "Get version history" 200 "$STATUS" "$BODY"
    assert_json_contains "History has versions array" "$BODY" "versions"
fi

# ── 7. Search - exact match ──────────────────────────────────

RESP=$(do_get "/api/kb/search?q=Plastic%20Bottle" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Search exact match" 200 "$STATUS" "$BODY"
assert_json_contains "Search finds Plastic Bottle" "$BODY" "Plastic Bottle"

# ── 8. Search - empty query ──────────────────────────────────

RESP=$(do_get "/api/kb/search?q=" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Search empty query returns 400" 400 "$STATUS" ""

# ── 9. Search - no results ───────────────────────────────────

RESP=$(do_get "/api/kb/search?q=xyznonexistent" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
BODY=$(extract_body "$RESP")
assert_status "Search no results returns 200" 200 "$STATUS" "$BODY"

# ── 10. Create entry as inspector (forbidden) ────────────────

RESP=$(do_post "/api/kb/entries" '{"item_name":"Test","disposal_category":"test","disposal_instructions":"test"}' "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Inspector cannot create KB entry" 403 "$STATUS" ""

# ── 11. Search as inspector (allowed) ────────────────────────

RESP=$(do_get "/api/kb/search?q=Plastic" "$INSPECTOR_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Inspector can search KB" 200 "$STATUS" ""

# ── 12. Get nonexistent entry ────────────────────────────────

RESP=$(do_get "/api/kb/entries/00000000-0000-0000-0000-000000000000" "$ADMIN_TOKEN")
STATUS=$(extract_status "$RESP")
assert_status "Get nonexistent entry returns 404" 404 "$STATUS" ""

# Save entry ID for other tests
echo "KB_ENTRY_ID=$ENTRY_ID" >> "$SCRIPT_DIR/.tokens"

print_summary "Knowledge Base"
exit $FAILED
