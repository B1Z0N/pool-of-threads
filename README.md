# pool-of-threads [![CI](https://github.com/B1Z0N/pool-of-threads/actions/workflows/ci.yml/badge.svg)](https://github.com/B1Z0N/pool-of-threads/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Learning Rust by building a work-stealing thread pool. Also figuring out how schedulers work under the hood.

## Project structure

```
src/
  main.rs        — CLI entry point
  lib.rs         — Public API, module declarations
  pool.rs        — ThreadPool: spawn, shutdown, park
  worker.rs      — Worker thread loop, work-stealing
  task.rs        — Type-erased task / closure wrapper
  metrics.rs     — Optional counters
benches/
  throughput.rs  — Criterion benchmarks
```

## Build

```bash
cargo build
cargo test
cargo bench
```

## License

MIT
