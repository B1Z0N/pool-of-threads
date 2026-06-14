#!/usr/bin/env python3
"""
Record benchmark results for trend tracking.

Runs all criterion benchmarks, extracts point estimates and CI bounds,
and appends a timestamped entry to benchmark-history.jsonl.

Usage:
    ./scripts/bench-record.py                    # record current state
    ./scripts/bench-record.py --commit abc123    # tag with commit SHA

CI integration (on push to main):
    python3 scripts/bench-record.py --commit "$GITHUB_SHA"
    git add benchmark-history.jsonl
    git commit -m "bench: record $GITHUB_SHA"
    git push
"""

import json
import os
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
HISTORY_FILE = PROJECT_ROOT / "benchmark-history.jsonl"

TIME_RE = re.compile(
    r"^(\S+)\s*\n\s+time:\s+\[([0-9.]+)\s*(µ?s|ms|ns)\s+"
    r"([0-9.]+)\s*(µ?s|ms|ns)\s+"
    r"([0-9.]+)\s*(µ?s|ms|ns)\]",
    re.MULTILINE,
)


def parse_duration(value: str, unit: str) -> float:
    v = float(value)
    if unit == "ns":
        return v
    elif unit == "µs":
        return v * 1_000
    elif unit == "ms":
        return v * 1_000_000
    elif unit == "s":
        return v * 1_000_000_000
    raise ValueError(f"unknown unit: {unit}")


def get_commit_sha() -> str:
    try:
        return subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            capture_output=True, text=True, cwd=PROJECT_ROOT, timeout=5,
        ).stdout.strip()
    except Exception:
        return "unknown"


def main():
    commit = None
    args = sys.argv[1:]
    if len(args) >= 2 and args[0] == "--commit":
        commit = args[1]
    commit = commit or get_commit_sha()

    print(f"==> Recording benchmarks for commit {commit}...")

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
        print(f"ERROR: benchmarks failed (exit {result.returncode})")
        print(result.stderr[-1000:])
        sys.exit(1)

    benchmarks = {}
    for match in TIME_RE.finditer(result.stdout):
        name = match.group(1)
        lower = parse_duration(match.group(2), match.group(3))
        estimate = parse_duration(match.group(4), match.group(5))
        upper = parse_duration(match.group(6), match.group(7))
        benchmarks[name] = {
            "estimate_ns": estimate,
            "lower_ns": lower,
            "upper_ns": upper,
        }

    entry = {
        "commit": commit,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "benchmarks": benchmarks,
    }

    with open(HISTORY_FILE, "a") as f:
        f.write(json.dumps(entry) + "\n")

    print(f"    Recorded {len(benchmarks)} benchmarks → {HISTORY_FILE}")
    print(f"    Total entries in history: {sum(1 for _ in open(HISTORY_FILE))}")


if __name__ == "__main__":
    main()
