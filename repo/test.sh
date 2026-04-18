#!/usr/bin/env bash
# Convenience alias: same as ./run_tests.sh
exec "$(cd "$(dirname "$0")" && pwd)/run_tests.sh" "$@"
