#!/usr/bin/env bash
# Wrapper: runs the noise characterization Python script.
# Usage: ./scripts/noise-characterize.sh
#        ITERATIONS=10 ./scripts/noise-characterize.sh
set -euo pipefail
cd "$(dirname "$0")/.."
exec python3 scripts/noise-characterize.py
