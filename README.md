# pool-of-threads [![CI](https://github.com/B1Z0N/pool-of-threads/actions/workflows/ci.yml/badge.svg)](https://github.com/B1Z0N/pool-of-threads/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A work-stealing thread pool scheduler built from scratch in Rust.
Educational project — understand how concurrent schedulers work by
building one.

## How it works

- **Injector queue** — global FIFO where `spawn()` pushes tasks
- **Per-worker queues** — each thread has a local FIFO (crossbeam-deque)
- **Work-stealing** — idle workers steal from sibling queues (round-robin
  starting from a random victim)
- **Parking** — workers park on a shared condvar when all queues are
  empty, wake on new work or shutdown
- **Shutdown** — `Drop` sets an atomic flag, wakes all parked workers,
  drains remaining tasks, and joins every thread

## Project structure

```
src/
  main.rs       — demo binary
  lib.rs        — public API, module declarations
  pool.rs       — ThreadPool: spawn, work-stealing loop, parking, shutdown
  task.rs       — type-erased closure wrapper
  metrics.rs    — cache-line-padded per-worker counters (false-sharing safe)
benches/
  throughput.rs — Criterion benchmarks
tests/
  work_stealing.rs  — barrier-orchestrated proof that stealing happens
  concurrent_submission.rs  — concurrent spawn + drop stress
  stress.rs         — 10k+ tasks, 8 submitters, counter verification
  starvation.rs     — injector flood + heavy/light coexistence
  leak.rs           — pool lifecycle smoke test + native leak tool docs
scripts/
  bench-regression.py    — criterion diff against baseline
  bench-record.py        — append results to benchmark-history.jsonl
  bench-history.py       — view trend data
  noise-characterize.py  — characterize benchmark variance
.tsan_suppressions  — crossbeam false-positive suppressions for TSan
.justfile            — just command shortcuts (just --list)
```

## Build and test

```bash
just --help

just build
just test
just bench
just run       # demo binary — shows scaling across thread counts
```

### CI (pre-push and GitHub Actions)

`just ci` runs on every push via a pre-push hook. It catches the common
stuff before you wait for CI:

| Step | What |
|------|------|
| `cargo clippy -- -D warnings` | Lints are errors |
| `cargo fmt -- --check` | Consistent formatting |
| `cargo test` | Full test suite (40 tests) |
| `PROPTEST_CASES=1024 cargo test --test proptest` | Property-based tests |
| `cargo test --test perturbation` | Yield-injection tests |
| `cargo bench --no-run` | Benchmarks compile |

GitHub Actions adds the heavier checks:

| Job | What | Notes |
|-----|------|-------|
| test (ubuntu, macos) × (stable, beta) | 4 matrix builds | |
| lint | clippy + fmt | |
| tsan | ThreadSanitizer | Uses `.tsan_suppressions` for crossbeam false positives |
| asan | AddressSanitizer | |
| leak-check | macOS `leaks` tool | |
| bench-regression | Criterion diff vs main | PR only, 5× tolerance for shared-runner variance |
| bench-baseline | Save criterion baseline | main push only |
| soak | 60-second stress | nightly schedule only |

### TSan suppressions

ThreadSanitizer reports false-positive data races inside crossbeam's
epoch-based garbage collector and deque buffer. These are design-level
false positives — crossbeam uses custom synchronization (epoch counters,
lap counters, atomic CAS) that TSan cannot model. See `.tsan_suppressions`
and the [crossbeam issue](https://github.com/crossbeam-rs/crossbeam/issues/589).

To run TSan locally: `just tsan` (requires nightly).

## Usage

```rust
use pool_of_threads::ThreadPool;

let pool = ThreadPool::new(4);
pool.spawn(|| { /* work */ });
drop(pool); // waits for all tasks
```

See [`lib.rs`](src/lib.rs) for architecture details and more examples.

## License

MIT
