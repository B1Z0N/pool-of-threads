# pool-of-threads &emsp; [![CI](https://github.com/B1Z0N/pool-of-threads/actions/workflows/ci.yml/badge.svg)](https://github.com/B1Z0N/pool-of-threads/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A work-stealing thread pool scheduler written from scratch in Rust.

Built as a learning project to deepen understanding of OS threads, lock-free data structures, atomics, and concurrent system design — no async, no Tokio, just raw primitives.

## Architecture (planned)

```
         ┌─────────────┐
         │  Global Q   │  (crossbeam injector — fallback submit)
         └──────┬──────┘
       ┌────────┼────────┐
       ▼        ▼        ▼
   ┌──────┐ ┌──────┐ ┌──────┐
   │ Wkr 0│ │ Wkr 1│ │ Wkr 2│  (OS threads, per-worker Chase-Lev deque)
   └──┬───┘ └──┬───┘ └──┬───┘
      │   steal │   steal │
      └─────────┼─────────┘
                ▼
         (random victim)
```

- **Workers** — OS threads with per-worker task queues (crossbeam-deque).
- **Work-stealing** — idle workers steal from random sibling queues.
- **Parking** — workers park on a condvar when all queues are empty. Spurious wakeup safe.
- **Shutdown** — atomic flag → unpark all → join handles. Graceful drain.

## Quick Start

```bash
# Build
cargo build

# Run tests
cargo test

# Run benchmarks
cargo bench

# CI preflight (fmt + clippy + test + bench-compile)
just ci
```

## Project Structure

```
src/
  main.rs        — CLI entry point
  lib.rs         — Public API, module declarations
  pool.rs        — ThreadPool: spawn, shutdown, park
  worker.rs      — Worker thread loop, work-stealing
  task.rs        — Type-erased task trait / closure wrapper
  metrics.rs     — Optional counters
benches/
  throughput.rs  — Criterion benchmarks
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| [crossbeam-deque](https://crates.io/crates/crossbeam) | Chase-Lev work-stealing queues |
| [parking_lot](https://crates.io/crates/parking_lot) | Fast Mutex, RwLock, Condvar |
| [criterion](https://crates.io/crates/criterion) | Statistical benchmarking |

No async runtime. No Tokio. Pure OS threads and atomics.

## License

MIT — see [LICENSE](LICENSE).
