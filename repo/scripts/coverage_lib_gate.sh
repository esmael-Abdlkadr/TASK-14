#!/usr/bin/env bash
# Lib coverage gate (≥90% line coverage). Scope: pure/unit-heavy modules only.
# HTTP integration: cargo test --test http_api (see tests/) with a live backend.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

TARPAULIN=(cargo tarpaulin --lib --timeout 300 --out Stdout --fail-under 90
  --include-files src/errors.rs
  --include-files src/models/user.rs
  --include-files src/auth/password.rs
  --include-files src/messaging/template_engine.rs
)

if command -v cargo-tarpaulin >/dev/null 2>&1; then
  echo "Running: ${TARPAULIN[*]}"
  exec "${TARPAULIN[@]}"
fi

if command -v docker >/dev/null 2>&1; then
  echo "Running coverage via Docker (installs cargo-tarpaulin)..."
  docker run --rm -v "$ROOT:/app" -w /app rust:1.88-bookworm bash -lc \
    "export PATH=\"/usr/local/cargo/bin:\$PATH\" && cargo install cargo-tarpaulin --locked -q && ${TARPAULIN[*]}"
  exit $?
fi

echo "Install cargo-tarpaulin or Docker to run this script." >&2
exit 1
