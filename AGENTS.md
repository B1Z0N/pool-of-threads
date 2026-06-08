# AGENTS.md — Pool of Threads

Project context for AI coding assistants (Cursor, Copilot, Aider, etc.).

## Rules

1. No async/await. This is a sync-only project using OS threads.
2. No Tokio. Dependencies are crossbeam, parking_lot, once_cell only.
3. Every `unsafe` block gets a `// SAFETY:` comment explaining the invariant.
4. Lock-free data structures must have tests with loom or stress runs.
5. All public API is documented with doc-tests where possible.
6. Benchmarks go in `benches/` using criterion, harness = false.
7. CI (future): `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `cargo bench --no-run`.

## Project Map

```
src/
  main.rs        — CLI entry point (eventually: config + spawn + demo)
  lib.rs         — Public API, module declarations
  pool.rs        — ThreadPool struct, spawn/shutdown/park
  worker.rs      — Worker thread loop, work-stealing logic
  task.rs        — Task trait / type-erased closure wrapper
  metrics.rs     — Optional counters (tasks completed, steals, etc.)
benches/
  throughput.rs  — Criterion benchmarks
tests/
  integration.rs — Integration tests
```

## Notes

- This is a learning project. Prioritize readability and correctness over micro-optimizations.
- Use `std::thread::Builder` with stack sizes — not just `std::thread::spawn`.
- Shutdown: first set an atomic flag, then unpark all workers, then join handles.
