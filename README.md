# pool-of-threads

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
