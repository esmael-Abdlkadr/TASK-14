#!/usr/bin/env bash
# ============================================================
# CivicSort — single entrypoint: Docker build + compose + all tests
#
# Default (./run_tests.sh or ./test.sh):
#   1. docker compose build
#   2. docker compose up -d (db + backend + frontend)
#   3. Wait until GET /api/health returns 200
#   4. cargo test --lib + --doc inside Docker (unit / doctests; no API required)
#   5. cargo test --tests inside Docker — integration tests hit the compose API via host.docker.internal
#
# Modes:
#   ./run_tests.sh           all of the above (default)
#   ./run_tests.sh unit      unit + doctests only (Rust tests in Docker; does not start compose)
#   ./run_tests.sh api       stack + all integration tests only
#
# Optional environment:
#   SKIP_DOCKER=1            do not build/up; use API already at CIVICSORT_API_URL
#   CIVICSORT_API_URL        default http://127.0.0.1:8080 (IPv4; avoids macOS localhost→::1 issues)
#   CIVICSORT_STACK_WAIT     seconds to wait for health (default 180)
#   NO_COMPOSE_BUILD=1       skip "docker compose build" (faster when images exist)
#   RUN_CARGO_IN_DOCKER=0    use host cargo when installed (default: 1 — always run cargo test in Docker)
#   RUST_TEST_IMAGE          Rust toolchain image for tests (default rust:1.88-bookworm)
#   (Postgres is not published on the host by default; see docker-compose.db-host.yml)
# ============================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MODE="${1:-all}"

accumulate_cargo_summary() {
    local file="$1"
    local psum=0 fsum=0
    local line _p _f
    while IFS= read -r line; do
        _p=$(echo "$line" | sed -n 's/.* \([0-9][0-9]*\) passed.*/\1/p')
        _f=$(echo "$line" | sed -n 's/.* \([0-9][0-9]*\) failed.*/\1/p')
        if [ -n "$_p" ]; then psum=$((psum + _p)); fi
        if [ -n "$_f" ]; then fsum=$((fsum + _f)); fi
    done < <(grep '^test result:' "$file" 2>/dev/null || true)
    echo "$psum $fsum"
}

wait_for_backend() {
    local base="${CIVICSORT_API_URL:-http://127.0.0.1:8080}"
    local url="${base%/}/api/health"
    local max="${CIVICSORT_STACK_WAIT:-180}"
    local n=0
    echo ""
    echo "  [stack] Waiting for API: $url (timeout ${max}s)"
    while [ "$n" -lt "$max" ]; do
        local code
        code=$(curl -4s --connect-timeout 3 -o /dev/null -w "%{http_code}" "$url" 2>/dev/null || echo "000")
        if [ "$code" = "200" ]; then
            echo "  [stack] Backend is up (HTTP 200)."
            echo ""
            return 0
        fi
        sleep 2
        n=$((n + 2))
        if [ $((n % 20)) -eq 0 ]; then
            echo "  [stack] ... still waiting (${n}s, last HTTP ${code})"
        fi
    done
    echo "  [stack] ERROR: API did not become healthy in time."
    echo "  [stack] Check: docker compose -f $SCRIPT_DIR/docker-compose.yml logs backend"
    echo ""
    return 1
}

compose_stack_up() {
    if [ "${SKIP_DOCKER:-0}" = "1" ]; then
        echo ""
        echo "  [stack] SKIP_DOCKER=1 — not running compose (using existing services)"
        echo ""
        return 0
    fi

    if ! command -v docker >/dev/null 2>&1; then
        echo "  ERROR: docker not found. Install Docker or set SKIP_DOCKER=1 with API already running."
        return 1
    fi
    if ! docker compose version >/dev/null 2>&1; then
        echo "  ERROR: docker compose not available."
        return 1
    fi

    cd "$SCRIPT_DIR" || return 1

    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  STACK: docker compose build + up (from repo root)"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""

    if [ "${NO_COMPOSE_BUILD:-0}" != "1" ]; then
        docker compose build || return 1
    else
        echo "  [stack] NO_COMPOSE_BUILD=1 — skipping image build"
    fi

    docker compose up -d --remove-orphans || return 1

    wait_for_backend || return 1
    return 0
}

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║          CivicSort Test Verification Plan               ║"
echo "║          $(date '+%Y-%m-%d %H:%M:%S')                        ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

UNIT_PASSED=0
UNIT_FAILED=0
UNIT_TOTAL=0
API_PASSED=0
API_FAILED=0
API_TOTAL=0

run_unit_tests() {
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  PHASE 1: Rust unit + doctests (cargo in Docker by default)"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""

    cd "$SCRIPT_DIR"
    RESULT_FILE=$(mktemp)
    RUST_IMG="${RUST_TEST_IMAGE:-rust:1.88-bookworm}"

    if command -v docker >/dev/null 2>&1 && [ "${RUN_CARGO_IN_DOCKER:-1}" != "0" ]; then
        echo "  Running unit + doc tests inside Docker ($RUST_IMG)"
        set +e
        docker run --rm -v "$SCRIPT_DIR":/app -w /app "$RUST_IMG" bash -lc \
            'export PATH=/usr/local/cargo/bin:$PATH; cargo test --lib -- --nocapture && cargo test --doc -- --nocapture' \
            2>&1 | tee "$RESULT_FILE"
        UNIT_EXIT=$?
        set -e
    elif command -v cargo >/dev/null 2>&1; then
        set +e
        cargo test --lib -- --nocapture 2>&1 | tee "$RESULT_FILE"
        U1=$?
        cargo test --doc -- --nocapture 2>&1 | tee -a "$RESULT_FILE"
        U2=$?
        set -e
        UNIT_EXIT=0
        if [ "$U1" -ne 0 ] || [ "$U2" -ne 0 ]; then UNIT_EXIT=1; fi
    else
        echo "  ERROR: Docker is required for Rust tests (install Docker), or set RUN_CARGO_IN_DOCKER=0 with cargo on PATH."
        rm -f "$RESULT_FILE"
        return 1
    fi

    read -r UNIT_PASSED UNIT_FAILED < <(accumulate_cargo_summary "$RESULT_FILE")
    UNIT_TOTAL=$((UNIT_PASSED + UNIT_FAILED))

    rm -f "$RESULT_FILE"
    echo ""
    if [ "$UNIT_EXIT" -eq 0 ]; then
        echo "  Phase 1: PASSED ($UNIT_PASSED tests passed)"
    else
        echo "  Phase 1: FAILED ($UNIT_PASSED passed, $UNIT_FAILED failed)"
    fi
    echo ""
    return "$UNIT_EXIT"
}

run_api_tests() {
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  PHASE 2: ALL Rust HTTP integration tests (cargo test --tests)"
    echo "  Every #[test] under tests/ — real TCP to CIVICSORT_API_URL"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""

    BASE_URL="${CIVICSORT_API_URL:-http://127.0.0.1:8080}"
    if ! curl -4s --connect-timeout 3 -o /dev/null -w "%{http_code}" "${BASE_URL%/}/api/health" 2>/dev/null | grep -q "200"; then
        echo "  ERROR: Backend not reachable at $BASE_URL"
        echo "  Run without SKIP_DOCKER=1 so compose can start the stack, or start the API manually."
        echo ""
        return 1
    fi

    cd "$SCRIPT_DIR"
    RESULT_FILE=$(mktemp)
    RUST_IMG="${RUST_TEST_IMAGE:-rust:1.88-bookworm}"

    if command -v docker >/dev/null 2>&1 && [ "${RUN_CARGO_IN_DOCKER:-1}" != "0" ]; then
        API_URL_FOR_CONTAINER="${CIVICSORT_API_URL:-http://host.docker.internal:8080}"
        echo "  Running HTTP integration tests inside Docker ($RUST_IMG)"
        echo "  CIVICSORT_API_URL for container: $API_URL_FOR_CONTAINER"
        set +e
        docker run --rm \
            --add-host=host.docker.internal:host-gateway \
            -e "CIVICSORT_API_URL=$API_URL_FOR_CONTAINER" \
            -v "$SCRIPT_DIR":/app -w /app "$RUST_IMG" bash -lc \
            'export PATH=/usr/local/cargo/bin:$PATH; cargo test --tests -- --nocapture' \
            2>&1 | tee "$RESULT_FILE"
        API_EXIT=$?
        set -e
    elif command -v cargo >/dev/null 2>&1; then
        set +e
        cargo test --tests -- --nocapture 2>&1 | tee "$RESULT_FILE"
        API_EXIT=$?
        set -e
    else
        echo "  ERROR: Docker is required for Rust tests (install Docker), or set RUN_CARGO_IN_DOCKER=0 with cargo on PATH."
        rm -f "$RESULT_FILE"
        return 1
    fi

    read -r API_PASSED API_FAILED < <(accumulate_cargo_summary "$RESULT_FILE")
    API_TOTAL=$((API_PASSED + API_FAILED))
    rm -f "$RESULT_FILE"
    return ${API_EXIT:-0}
}

UNIT_EXIT=0
API_EXIT=0

case "$MODE" in
    unit)
        run_unit_tests
        UNIT_EXIT=$?
        ;;
    api)
        compose_stack_up || exit 1
        run_api_tests
        API_EXIT=$?
        ;;
    all|*)
        compose_stack_up || exit 1
        run_unit_tests
        UNIT_EXIT=$?
        echo ""
        run_api_tests
        API_EXIT=$?
        ;;
esac

TOTAL=$((UNIT_TOTAL + API_TOTAL))
PASSED=$((UNIT_PASSED + API_PASSED))
FAILED=$((UNIT_FAILED + API_FAILED))

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║              FINAL TEST RESULTS SUMMARY                 ║"
echo "╠══════════════════════════════════════════════════════════╣"
echo "║                                                          ║"
if [ "$UNIT_TOTAL" -gt 0 ]; then
    printf "║  Unit + doc:    %3d passed / %3d total                   ║\n" "$UNIT_PASSED" "$UNIT_TOTAL"
fi
if [ "$API_TOTAL" -gt 0 ]; then
    printf "║  HTTP (tests/): %3d passed / %3d total                   ║\n" "$API_PASSED" "$API_TOTAL"
fi
echo "║  ─────────────────────────────────                       ║"
printf "║  TOTAL:         %3d passed / %3d total                   ║\n" "$PASSED" "$TOTAL"
echo "║                                                          ║"

OVERALL_EXIT=$((UNIT_EXIT + API_EXIT))
if [ "$OVERALL_EXIT" -eq 0 ]; then
    echo "║  STATUS: ✓ ALL TESTS PASSED                             ║"
else
    echo "║  STATUS: ✗ SOME TESTS FAILED                            ║"
fi
echo "║                                                          ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

exit $OVERALL_EXIT
