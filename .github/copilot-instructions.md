# Copilot Instructions — Pool of Threads

This file configures GitHub Copilot's behavior in this repository.

- Use Rust edition 2024 conventions.
- Prefer `parking_lot` over `std::sync` for Mutex, RwLock, Condvar.
- Use `crossbeam-deque` for Chase-Lev work-stealing queues.
- All `unsafe` blocks must include a `// SAFETY:` comment.
- Minimize dependencies — only crossbeam, parking_lot, once_cell.
- No async/await, no Tokio.
- Generate doc comments for all public items.
- Prefer `Ordering::Acquire/Release` for atomic operations.
- Tests should be in `#[cfg(test)] mod tests` blocks within each source file.
- Follow the project structure in AGENTS.md.
