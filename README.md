# pool-of-threads

Learning Rust by building a work-stealing thread pool. Also figuring out how schedulers work under the hood.

## Why

I'm refreshing Rust and wanted a project that forces me to use:
- threads and `std::thread`
- atomics (`AtomicBool`, `AtomicUsize`, ...)
- mutexes and condvars
- unsafe (Chase-Lev deque internals)

No async, no Tokio. Just `std` + `crossbeam` + `parking_lot`.

## Architecture (planned)

```
        ┌─────────────┐
        │  Global Q    │  (crossbeam injector — fallback submit)
        └──────┬──────┘
      ┌────────┼────────┐
      ▼        ▼        ▼
  ┌──────┐ ┌──────┐ ┌──────┐
  │ Wkr 0│ │ Wkr 1│ │ Wkr 2│  (OS threads, per-worker Chase-Lev deque)
  └──┬───┘ └──┬───┘ └──┬───┘
     │  steal  │  steal  │
     └─────────┼─────────┘
               ▼
        (random victim)
```

- Workers — OS threads with per-worker task queues
- Work-stealing — idle workers grab tasks from random siblings
- Parking — workers sleep on a condvar when queues are empty
- Shutdown — atomic flag, unpark all, join handles

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
