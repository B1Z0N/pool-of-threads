# pool-of-threads — quality-of-life shortcuts
#
# Run `just` with no arguments to list all available commands.
# Run `just <command>` to execute a specific target.

default:
    @just --list

# ── benchmarks ────────────────────────────────────────────────────────

# Save a baseline snapshot (call before making changes)
bench-baseline name="main":
    cargo bench --bench throughput --bench overhead --bench scalability --bench work_stealing -- --save-baseline {{name}}
    @echo "Baseline '{{name}}' saved. Make changes, then run: just bench-diff"

# Compare current benchmarks against a saved baseline
bench-diff name="main":
    cargo bench --bench throughput --bench overhead --bench scalability --bench work_stealing -- --baseline {{name}}

# Compare a specific benchmark against a baseline
bench-diff-bench group bench name="main":
    cargo bench --bench {{group}} -- {{bench}} --baseline {{name}}

# Run all benchmarks (no comparison, just collect numbers)
bench-all:
    cargo bench

# Run benchmarks without running them (check they compile)
bench-check:
    cargo bench --no-run

# ── noise characterization ─────────────────────────────────────────────

# Characterize benchmark noise — runs all benchmarks, computes
# coefficient of variation per benchmark, outputs recommendations.
# Set ITERATIONS env var to control runs (default: 5).
bench-noise:
    @bash scripts/noise-characterize.sh

# ── testing ────────────────────────────────────────────────────────────

# Run all tests (unit + integration + doctests)
test-all:
    cargo test

# Run only integration tests
test-integration:
    cargo test --test concurrent_submission --test leak --test starvation \
               --test stress --test work_stealing

# Run proptest (randomized property-based tests, 1024 cases each)
test-proptest:
    PROPTEST_CASES=1024 cargo test --test proptest

# Run perturbation tests (yield injection)
test-perturbation:
    cargo test --test perturbation

# Run soak tests for N seconds (default: 10)
test-soak secs="10":
    SOAK_SECS={{secs}} cargo test --test soak -- --ignored --nocapture

# ── sanitizers ──────────────────────────────────────────────────────────

# Run tests under ThreadSanitizer (requires nightly)
tsan *args:
    bash scripts/tsan.sh {{args}}

# ── lint ────────────────────────────────────────────────────────────────

# Run clippy (strict: warnings are errors)
lint:
    cargo clippy --all-targets -- -D warnings

# Check formatting
fmt:
    cargo fmt --all -- --check

# Auto-format all code
fmt-fix:
    cargo fmt --all

# ── full CI run (everything you'd run before pushing) ───────────────────

ci: lint fmt test-all test-proptest test-perturbation bench-check
    @echo ""
    @echo "✓ All CI checks passed"
    @echo "  Optional before pushing:"
    @echo "    just bench-baseline   # save baseline for future comparison"
    @echo "    just tsan             # run ThreadSanitizer (requires nightly)"
    @echo "    just test-soak 60     # 60-second soak test"

# ── benchmark regression ───────────────────────────────────────────────

# Compare PR branch against main using critcmp (for CI or manual use)
bench-regression base="main" head="HEAD":
    @bash scripts/bench-regression.sh {{base}} {{head}}

# ── benchmark trend tracking ────────────────────────────────────────────

# Record current benchmark results to benchmark-history.jsonl
bench-record:
    python3 scripts/bench-record.py
    @echo "History updated. View with: just bench-history"

# View benchmark trends across commits
bench-history *args:
    python3 scripts/bench-history.py {{args}}

# Show only regressions exceeding tolerance
bench-history-alerts:
    python3 scripts/bench-history.py --alerts
