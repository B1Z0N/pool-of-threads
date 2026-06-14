#!/usr/bin/env bash
# Benchmark regression check — for CI or manual use.
#
# Compares current benchmarks against a saved baseline and fails if any
# benchmark regresses beyond its configured tolerance (.benchmarks.toml).
#
# Usage:
#   ./scripts/bench-regression.sh main          # compare against 'main' baseline
#   ./scripts/bench-regression.sh main --verbose

set -euo pipefail
cd "$(dirname "$0")/.."
exec python3 scripts/bench-regression.py "$@"
