#!/usr/bin/env bash
# Run tests under ThreadSanitizer.
#
# TSan detects data races at runtime — wrong Ordering, unprotected shared
# writes, races that manifest 0.001% of the time. Required: nightly Rust.
#
# Usage:
#   ./scripts/tsan.sh          # run all tests under TSan
#   ./scripts/tsan.sh test_name  # run a specific test

set -euo pipefail

HOST_TRIPLE=$(rustc -vV | grep host | cut -d' ' -f2)

echo "==> Host triple: $HOST_TRIPLE"
echo "==> Checking for nightly toolchain..."

if ! rustup toolchain list | grep -q nightly; then
    echo "==> Installing nightly toolchain..."
    rustup toolchain install nightly
fi

echo "==> Installing rust-src for nightly..."
rustup component add rust-src --toolchain nightly 2>/dev/null || true

echo "==> Running tests under ThreadSanitizer..."
echo "    (TSan may produce false positives from system libraries;"
echo "     focus on reports in pool_of_threads source files)"

RUSTFLAGS="-Z sanitizer=thread" \
    cargo +nightly test \
    -Z build-std \
    --target "$HOST_TRIPLE" \
    --lib --tests \
    "$@" \
    2>&1 | tee target/tsan_output.txt

echo ""
echo "==> TSan output saved to target/tsan_output.txt"
