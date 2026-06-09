# Copilot Instructions — Pool of Threads

This file configures GitHub Copilot's behavior in this repository.

## What this project is

An educational Rust project building a work-stealing thread pool scheduler from scratch.
The codebase covers per-worker task queues, a global fallback queue, stealing between
idle and busy workers, efficient thread parking, and graceful shutdown. Every piece is
built to be understood, not just to work.

## How Copilot should help

- Complete patterns that fit the concurrent, ownership-heavy style already established in
  the file — don't introduce styles that clash with the surrounding code.
- When completing unsafe blocks, always include an inline safety comment explaining the
  invariant being upheld. Do not emit bare `unsafe` with no justification.
- Suggest test cases that exercise concurrent behavior: multiple threads, races at
  shutdown, queue-empty edge cases.
- Prefer explicit, readable code over clever one-liners — this is a learning codebase and
  clarity matters more than brevity.
- Respect the memory ordering already used in the file. Do not silently upgrade to
  `SeqCst` without a comment explaining why a weaker ordering is insufficient.
