# CLAUDE.md — Pool of Threads

Context for Claude Code sessions in this project.

## Project

A work-stealing thread pool scheduler written from scratch in Rust. Educational project
to refresh Rust knowledge and learn concurrency primitives.

## Tech Stack

- **Rust edition 2024**, stable toolchain
- **crossbeam-deque** for work-stealing queues (Chase-Lev)
- **parking_lot** for fast mutexes and condvars
- **criterion** for benchmarks
- No async runtime — pure OS threads and atomics

## Build & Test

```bash
cargo build
cargo test
cargo test -- --nocapture          # show println output
cargo bench                         # criterion benchmarks
cargo lint                          # alias: clippy with -D warnings
cargo fmt -- --check                # check formatting
```

## Code Style

- `rustfmt.toml` is authoritative — run `cargo fmt` before committing
- Modules: one concept per file, re-exported via `lib.rs`
- Unsafe blocks: minimal, isolated, and commented with safety invariants
- Atomics: prefer `Ordering::Acquire/Release` over `SeqCst` unless needed
- Tests: inline `#[cfg(test)] mod tests` in each module

## Pitfalls

- macOS has no `pthread_spinlock`, use `parking_lot` instead of `std::sync`
- `crossbeam-deque` `Injector` and `Worker` have different push/pop semantics
- Thread parking must handle spurious wakeups
- Shutdown coordination: use atomic flags + barrier, not just a condvar
- `cargo build --release` on macOS may produce x86_64 binary on ARM if
  rustup was installed under Rosetta — verify with `file target/release/pool-of-threads`
