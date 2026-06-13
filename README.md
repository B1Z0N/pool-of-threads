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
  shutdown_race.rs  — concurrent spawn + drop stress
  stress.rs         — 10k+ tasks, 8 submitters, counter verification
  starvation.rs     — injector flood + heavy/light coexistence
  leak.rs           — pool lifecycle smoke test + native leak tool docs
```

## Build

```bash
cargo build
cargo test
cargo bench
cargo run    # runs the demo — shows scaling across thread counts
```

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
