#!/usr/bin/env python3
"""
Noise characterization for pool-of-threads benchmarks.

Runs each benchmark N times, collects per-run estimates AND confidence
interval bounds. Computes cross-run statistics: CV of the point estimate
and trend of CI width (widening CI = growing system noise).

Outputs a formatted table and a JSON file for programmatic consumption.

Usage:
    ITERATIONS=3  ./scripts/noise-characterize.sh    # default (CI-optimized)
    ITERATIONS=10 ./scripts/noise-characterize.sh    # thorough
"""

import json
import os
import re
import statistics
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
ITERATIONS = int(os.environ.get("ITERATIONS", "3"))

# ── parse criterion output ──────────────────────────────────────────

TIME_RE = re.compile(
    r"^(\S+)\s*\n\s+time:\s+\[([0-9.]+)\s*(µ?s|ms|ns)\s+"
    r"([0-9.]+)\s*(µ?s|ms|ns)\s+"
    r"([0-9.]+)\s*(µ?s|ms|ns)\]",
    re.MULTILINE,
)


def parse_duration(value: str, unit: str) -> float:
    """Parse ('14.374', 'µs') → nanoseconds."""
    v = float(value)
    if unit == "ns":
        return v
    elif unit == "µs":
        return v * 1_000
    elif unit == "ms":
        return v * 1_000_000
    elif unit == "s":
        return v * 1_000_000_000
    else:
        raise ValueError(f"unknown unit: {unit}")


@dataclass
class RunResult:
    """One benchmark run: point estimate + confidence interval bounds (ns)."""
    lower_ns: float
    estimate_ns: float
    upper_ns: float

    @property
    def ci_width_ns(self) -> float:
        return self.upper_ns - self.lower_ns

    @property
    def ci_width_pct(self) -> float:
        """CI half-width as percentage of estimate."""
        if self.estimate_ns == 0:
            return 0.0
        return (self.ci_width_ns / 2) / self.estimate_ns * 100


def extract_benchmarks(output: str) -> dict[str, RunResult]:
    """Extract benchmark name → RunResult from criterion output."""
    results: dict[str, RunResult] = {}
    for match in TIME_RE.finditer(output):
        name = match.group(1)
        lower = parse_duration(match.group(2), match.group(3))
        estimate = parse_duration(match.group(4), match.group(5))
        upper = parse_duration(match.group(6), match.group(7))
        results[name] = RunResult(lower_ns=lower, estimate_ns=estimate, upper_ns=upper)
    return results


# ── statistics ─────────────────────────────────────────────────────

def stability_category(cv_pct: float) -> tuple[str, float]:
    """Classify benchmark stability and recommend CI regression threshold."""
    if cv_pct < 1.0:
        return ("★ stable", 2.0)
    elif cv_pct < 2.0:
        return ("● good", 4.0)
    elif cv_pct < 5.0:
        return ("◆ moderate", 8.0)
    elif cv_pct < 10.0:
        return ("▲ noisy", 15.0)
    else:
        return ("✗ unstable", 25.0)


def ci_width_trend(ci_widths_pct: list[float]) -> str:
    """Check if CI width is growing (indicates rising system noise)."""
    if len(ci_widths_pct) < 2:
        return "—"
    first_half = statistics.mean(ci_widths_pct[: len(ci_widths_pct) // 2])
    second_half = statistics.mean(ci_widths_pct[len(ci_widths_pct) // 2:])
    if first_half == 0:
        return "—"
    change = (second_half - first_half) / first_half * 100
    if change > 50:
        return f"⚠ widening ({change:+.0f}%)"
    elif change < -50:
        return f"↓ narrowing ({change:+.0f}%)"
    else:
        return f"stable ({change:+.0f}%)"


def format_ns(ns: float) -> str:
    """Human-readable duration from nanoseconds."""
    if ns >= 1_000_000_000:
        return f"{ns / 1_000_000_000:.3f} s"
    elif ns >= 1_000_000:
        return f"{ns / 1_000_000:.3f} ms"
    elif ns >= 1_000:
        return f"{ns / 1_000:.3f} µs"
    else:
        return f"{ns:.1f} ns"


# ── main ────────────────────────────────────────────────────────────

def main():
    print(f"==> Noise characterization: {ITERATIONS} runs per benchmark\n")

    # Collect per-run results for each benchmark
    all_runs: dict[str, list[RunResult]] = {}

    for run_num in range(1, ITERATIONS + 1):
        print(f"    Run {run_num}/{ITERATIONS}...", end=" ", flush=True)
        result = subprocess.run(
            [
                "cargo", "bench",
                "--bench", "throughput",
                "--bench", "overhead",
                "--bench", "scalability",
                "--bench", "work_stealing",
            ],
            cwd=PROJECT_ROOT,
            capture_output=True,
            text=True,
            timeout=900,
        )
        if result.returncode != 0:
            print(f"FAILED (exit {result.returncode})")
            print(result.stderr[-1000:])
            sys.exit(1)

        benchmarks = extract_benchmarks(result.stdout)
        if not benchmarks:
            print("WARNING: no benchmarks parsed")
            continue

        for name, run_result in benchmarks.items():
            all_runs.setdefault(name, []).append(run_result)

        print(f"({len(benchmarks)} benchmarks)")

    if not all_runs:
        print("ERROR: no benchmark data collected")
        sys.exit(1)

    # Compute statistics
    header = (
        f"{'Benchmark':<55} {'N':>3} {'Estimate':>12} {'CV':>7} "
        f"{'CI width':>9} {'CI trend':<22} {'Category':<12} {'Limit':>5}"
    )
    print(f"\n{header}")
    print("-" * len(header))

    stats: list[dict] = []
    warnings: list[str] = []

    for name in sorted(all_runs.keys()):
        runs = all_runs[name]
        if len(runs) < 2:
            continue

        estimates = [r.estimate_ns for r in runs]
        ci_widths_pct = [r.ci_width_pct for r in runs]

        mean_est = statistics.mean(estimates)
        stdev_est = statistics.stdev(estimates) if len(estimates) > 1 else 0.0
        cv_pct = (stdev_est / mean_est * 100) if mean_est > 0 else 0.0

        mean_ci_width = statistics.mean(ci_widths_pct)
        ci_trend = ci_width_trend(ci_widths_pct)

        cat, ci_limit = stability_category(cv_pct)

        print(
            f"{name:<55} {len(runs):>3} {format_ns(mean_est):>12} "
            f"{cv_pct:>6.2f}% {mean_ci_width:>8.2f}% {ci_trend:<22} "
            f"{cat:<12} {ci_limit:>4.0f}%"
        )

        # Flag potential issues
        if "⚠" in ci_trend:
            warnings.append(f"{name}: CI width is widening — system noise may be increasing")

        stats.append({
            "benchmark": name,
            "runs": len(runs),
            "mean_estimate_ns": mean_est,
            "stdev_estimate_ns": stdev_est,
            "cv_pct": round(cv_pct, 2),
            "mean_ci_width_pct": round(mean_ci_width, 2),
            "ci_width_trend": ci_trend,
            "stability": cat.strip(),
            "ci_regression_limit_pct": ci_limit,
        })

    # Write JSON
    json_path = PROJECT_ROOT / "target" / "benchmark-noise.json"
    json_path.parent.mkdir(parents=True, exist_ok=True)
    json_path.write_text(json.dumps(stats, indent=2))
    print(f"\n    JSON report: {json_path}")

    # Summary
    categories: dict[str, int] = {}
    for s in stats:
        cat = s["stability"]
        categories[cat] = categories.get(cat, 0) + 1

    print("\n    Summary by stability:")
    for cat in ["★ stable", "● good", "◆ moderate", "▲ noisy", "✗ unstable"]:
        if cat in categories:
            print(f"      {cat}: {categories[cat]}")

    stable_count = categories.get("★ stable", 0) + categories.get("● good", 0)
    total = len(stats)
    print(f"\n    {stable_count}/{total} benchmarks stable enough for CI regression detection.")

    if warnings:
        print(f"\n    ⚠ Warnings ({len(warnings)}):")
        for w in warnings:
            print(f"      {w}")

    if ITERATIONS < 3:
        print("\n    Note: <3 iterations → CI trend not meaningful. Use ITERATIONS=5+ for reliable trends.")


if __name__ == "__main__":
    main()
