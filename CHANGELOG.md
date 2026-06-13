# Changelog

All notable changes to Pool of Threads will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Project scaffold: Cargo.toml, CI, justfile, AI agent configs

- `Task` type — type-erased closure wrapper with `new` and `run`
- `ThreadPool` — work-stealing thread pool with configurable worker count
  - Global injector queue (`crossbeam::deque::Injector`)
  - Per-worker local FIFO queues
  - Work-stealing with round-robin victim selection
  - Condvar-based parking when all queues are empty
  - Graceful shutdown on `Drop` (drains remaining tasks, joins all workers)
- Demo binary (`main.rs`) — stress test showing throughput scaling across
  thread counts
- Doc-tests on all public API items
- Test suite (10 tests): basic execution, concurrent submission, shutdown
  semantics, drop safety
