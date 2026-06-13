# Contributing to Pool of Threads

Thanks for your interest! This is a learning project focused on
implementing a work-stealing thread pool in Rust.

## Setup

```bash
# Requires Rust stable (edition 2024)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install just (optional task runner)
brew install just   # macOS
cargo install just  # anywhere

# Build and test
just build
just test
```

## Workflow

1. Fork the repo, create a feature branch
2. Run `just ci` before pushing — this runs fmt, lint, and tests
3. Open a PR against `main`
4. CI must be green before merge

## Code Style

- `cargo fmt` is non-negotiable — `rustfmt.toml` is the single source of truth
- `cargo clippy -- -D warnings` must pass clean
- Every `unsafe` block gets a `// SAFETY:` comment
- Ordering: prefer `Acquire`/`Release` over `SeqCst`

## Project Structure

```
src/
  main.rs        — CLI entry point
  lib.rs         — Public API + module declarations
  pool.rs        — ThreadPool: spawn, work-stealing loop, parking, shutdown
  task.rs        — Type-erased closure wrapper
benches/
  throughput.rs  — Criterion benchmarks
```
