#!/usr/bin/env python3
"""
Benchmark regression checker.

Compares current benchmarks against a saved criterion baseline and
reports which benchmarks regressed beyond their configured tolerance
(from .benchmarks.toml). Exits non-zero if any regression exceeds threshold.

Intended for CI:
    On main push:  cargo bench -- --save-baseline main
    On PR:         ./scripts/bench-regression.py main

The script:
    1. Runs cargo bench -- --baseline <name>
    2. Parses criterion output for regression percentages
    3. Loads tolerance rules from .benchmarks.toml
    4. Reports failures for benchmarks that regressed too much

Usage:
    ./scripts/bench-regression.py <baseline-name>
    ./scripts/bench-regression.py main --verbose
"""

import re
import subprocess
import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent

# ── parse criterion change output ────────────────────────────────────

CHANGE_RE = re.compile(
    r"^(\S+)\s*\n\s+time:\s+\[[^\]]+\]\s*\n\s+change:\s+\["
    r"([+-][0-9.]+)%\s+([+-][0-9.]+)%\s+([+-][0-9.]+)%"
    r"\]",
    re.MULTILINE,
)

# ── tolerance config ─────────────────────────────────────────────────

def load_tolerances() -> tuple[float, list[tuple[str, float, str]]]:
    """Parse .benchmarks.toml using Python's TOML parser.

    Tries tomllib (3.11+ stdlib), then tomli (pip install tomli).
    Returns (default_pct, [(pattern, pct, note), ...]).
    First matching rule wins — patterns are matched as substrings.
    """
    try:
        import tomllib
    except ImportError:
        try:
            import tomli as tomllib
        except ImportError:
            sys.exit(
                "ERROR: tomllib (Python 3.11+) not available.\n"
                "       Install the backport: pip3 install tomli"
            )

    config_path = PROJECT_ROOT / ".benchmarks.toml"
    if not config_path.exists():
        return 3.0, []

    with open(config_path, "rb") as f:
        config = tomllib.load(f)

    default = float(config.get("default", {}).get("max_regression_pct", 3.0))

    rules: list[tuple[str, float, str]] = []
    for rule in config.get("rules", []):
        pattern = rule["pattern"]
        pct = float(rule["max_regression_pct"])
        note = rule.get("note", "")
        rules.append((pattern, pct, note))

    return default, rules


def tolerance_for(benchmark: str, default: float, rules: list) -> float:
    """Find the tolerance for a benchmark name. First rule matching wins."""
    for pattern, pct, _note in rules:
        if pattern in benchmark:
            return pct
    return default


# ── main ─────────────────────────────────────────────────────────────

def main():
    if len(sys.argv) < 2:
        print("Usage: bench-regression.py <baseline-name> [--verbose]", file=sys.stderr)
        sys.exit(2)

    baseline = sys.argv[1]
    verbose = "--verbose" in sys.argv

    # Load tolerances
    default_pct, rules = load_tolerances()
    if verbose:
        print(f"Default tolerance: ±{default_pct}%")
        for pattern, pct, note in rules:
            print(f"  Rule: '{pattern}' → ±{pct}%  ({note})")
        print()

    # Run benchmarks against baseline
    print(f"==> Running benchmarks against baseline '{baseline}'...\n")
    result = subprocess.run(
        [
            "cargo", "bench",
            "--bench", "throughput",
            "--bench", "overhead",
            "--bench", "scalability",
            "--bench", "work_stealing",
            "--", "--baseline", baseline,
        ],
        cwd=PROJECT_ROOT,
        capture_output=True,
        text=True,
        timeout=600,
    )

    output = result.stdout + result.stderr

    # Parse changes
    changes = CHANGE_RE.finditer(output)
    benchmarks_found = 0
    regressions: list[tuple[str, float, float, float]] = []  # (name, lower, est, upper)
    failures: list[tuple[str, float, float, str]] = []  # (name, change_pct, tolerance, reason)

    for match in changes:
        name = match.group(1)
        lower = float(match.group(2))
        est = float(match.group(3))
        upper = float(match.group(4))
        benchmarks_found += 1

        tolerance = tolerance_for(name, default_pct, rules)

        # Only the upper bound matters for regression (we care if it got slower).
        # The lower bound matters for improvements.
        if upper > tolerance:
            failures.append(
                (
                    name,
                    upper,
                    tolerance,
                    f"regressed {upper:+.1f}% (limit: {tolerance:.0f}%)",
                )
            )
        elif upper > 0:
            regressions.append((name, lower, est, upper))
        elif verbose:
            print(f"  {name}: {lower:+.1f}% / {est:+.1f}% / {upper:+.1f}%  "
                  f"(limit: ±{tolerance:.0f}%) ✓")

    # Report regressions within tolerance (informational)
    if regressions and verbose:
        print("\n    Regressions within tolerance:")
        for name, lower, est, upper in regressions:
            tol = tolerance_for(name, default_pct, rules)
            print(f"      {name}: {lower:+.1f}% / {est:+.1f}% / {upper:+.1f}%  "
                  f"(limit: ±{tol:.0f}%)")

    # Summarize
    print(f"\n    Benchmarks compared: {benchmarks_found}")

    if failures:
        print(f"\n    ══ REGRESSIONS EXCEEDING TOLERANCE ══")
        for name, change_pct, tolerance, reason in failures:
            print(f"    ✗ {name}")
            print(f"      {reason}")
        print(f"\n    {len(failures)} benchmark(s) regressed beyond tolerance!")
        print(f"    Review the changes or update .benchmarks.toml if the regression is expected.")
        sys.exit(1)
    elif not changes:
        # No change lines means criterion couldn't find the baseline or
        # the benchmark names don't match. Report but don't fail (could
        # be the first run on a new machine).
        print("\n    WARNING: No benchmark change data found.")
        print(f"    This may mean the baseline '{baseline}' doesn't exist or")
        print("    benchmark names changed. Baselines are machine-specific.")
        print(f"    Run: just bench-baseline {baseline}")
    else:
        print("    ✓ All benchmarks within tolerance.")

    if result.returncode != 0:
        # Criterion returned non-zero for a reason other than regressions
        # (e.g., compilation failure). Propagate the exit code.
        sys.exit(result.returncode)


if __name__ == "__main__":
    main()
