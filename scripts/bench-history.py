#!/usr/bin/env python3
"""
View benchmark trend history.

Reads benchmark-history.jsonl and displays per-benchmark trends:
point estimates over time, regression detection between consecutive
commits, and min/max range.

Usage:
    ./scripts/bench-history.py                  # full history
    ./scripts/bench-history.py --last 5         # last 5 entries
    ./scripts/bench-history.py --bench park_wake  # filter by benchmark name
    ./scripts/bench-history.py --alerts          # only show regressions > threshold
"""

import json
import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
HISTORY_FILE = PROJECT_ROOT / "benchmark-history.jsonl"

# Tolerance from .benchmarks.toml (same values, duplicated for standalone use)
DEFAULT_TOLERANCE = 4.0
TOLERANCES = [
    ("throughput", 2.0),
    ("sequential", 2.0),
    ("scalability/workers", 4.0),
    ("scalability/submitters", 5.0),
    ("work_stealing/imbalanced", 6.0),
    ("overhead/per_task", 8.0),
    ("work_stealing/saturated", 10.0),
    ("overhead/park_wake", 10.0),
]


def tolerance_for(name: str) -> float:
    for pattern, pct in TOLERANCES:
        if pattern in name:
            return pct
    return DEFAULT_TOLERANCE


def format_ns(ns: float) -> str:
    if ns >= 1_000_000_000:
        return f"{ns / 1_000_000_000:.3f}s"
    elif ns >= 1_000_000:
        return f"{ns / 1_000_000:.2f}ms"
    elif ns >= 1_000:
        return f"{ns / 1_000:.2f}µs"
    else:
        return f"{ns:.1f}ns"


def load_history() -> list[dict]:
    if not HISTORY_FILE.exists():
        print("No benchmark history found. Run: scripts/bench-record.py")
        return []
    entries = []
    with open(HISTORY_FILE) as f:
        for line in f:
            line = line.strip()
            if line:
                entries.append(json.loads(line))
    return entries


def main():
    args = sys.argv[1:]
    last_n = None
    filter_bench = None
    alerts_only = False

    i = 0
    while i < len(args):
        if args[i] == "--last" and i + 1 < len(args):
            last_n = int(args[i + 1])
            i += 2
        elif args[i] == "--bench" and i + 1 < len(args):
            filter_bench = args[i + 1]
            i += 2
        elif args[i] == "--alerts":
            alerts_only = True
            i += 1
        else:
            i += 1

    entries = load_history()
    if not entries:
        return

    if last_n:
        entries = entries[-last_n:]

    # Collect all benchmark names
    all_benchmarks: set[str] = set()
    for e in entries:
        all_benchmarks.update(e["benchmarks"].keys())

    if filter_bench:
        all_benchmarks = {b for b in all_benchmarks if filter_bench in b}

    alerts: list[str] = []

    for bench_name in sorted(all_benchmarks):
        # Extract time series for this benchmark
        series: list[tuple[str, float, float, float]] = []  # (commit, est, lower, upper)
        for e in entries:
            if bench_name in e["benchmarks"]:
                b = e["benchmarks"][bench_name]
                series.append((e["commit"], b["estimate_ns"], b["lower_ns"], b["upper_ns"]))

        if len(series) < 2:
            # Show single-entry benchmarks too (no trend yet)
            if not alerts_only:
                commit, est, lower, upper = series[0]
                print(f"\n── {bench_name}  (1 entry, no trend yet)")
                print(f"  {commit:>8}  {format_ns(est):>10}")
            continue

        estimates = [s[1] for s in series]
        min_est = min(estimates)
        max_est = max(estimates)
        spread_pct = ((max_est - min_est) / min_est * 100) if min_est > 0 else 0
        tol = tolerance_for(bench_name)

        # Check for regressions between consecutive commits
        regressions = []
        for j in range(1, len(series)):
            prev = series[j - 1][1]
            curr = series[j][1]
            if prev > 0:
                change = (curr - prev) / prev * 100
                if change > tol:
                    regressions.append(
                        f"  {series[j-1][0]} → {series[j][0]}: {change:+.1f}% "
                        f"({format_ns(prev)} → {format_ns(curr)}) ⚠ exceeds {tol:.0f}%"
                    )

        if alerts_only and not regressions:
            continue

        if not alerts_only:
            print(f"\n── {bench_name}  (tolerance: ±{tol:.0f}%, spread: {spread_pct:.1f}%)")
            for commit, est, lower, upper in series:
                bar_len = int((est - min_est) / (max_est - min_est) * 40) if max_est > min_est else 20
                bar = "█" * bar_len
                print(f"  {commit:>8}  {format_ns(est):>10}  {bar}")

        for r in regressions:
            alerts.append(f"{bench_name}:\n{r}")

    if alerts:
        print(f"\n{'═' * 70}")
        print(f"  REGRESSIONS ({len(alerts)}):")
        for a in alerts:
            print(a)
        print(f"{'═' * 70}")


if __name__ == "__main__":
    main()
