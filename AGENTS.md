# AGENTS.md — Pool of Threads

Project context for AI coding assistants (Cursor, Copilot, Aider, etc.).

## What this project is

An educational Rust project that builds a work-stealing thread pool scheduler from
scratch. The goal is to deeply understand how concurrent schedulers work: how work is
distributed across threads, how idle threads find tasks without contention, how threads
park and wake efficiently, and how to shut everything down cleanly. There is no
shortcuts — every primitive is built and reasoned about, not imported and trusted.

## How you can help

- **Design guidance** — when the next piece of the scheduler needs to be wired together,
  help think through the ownership model, the threading contract, and the failure modes
  before any code is written.

- **Concurrency correctness** — flag data races, missed memory ordering constraints,
  incorrect assumptions about thread visibility, or unsafe blocks that lack a convincing
  safety argument.

- **Explaining tradeoffs** — this is a learning project. When there are multiple ways to
  solve something (different queue strategies, parking strategies, shutdown protocols),
  explain the tradeoffs rather than picking one silently.

- **Test scaffolding** — concurrent code is notoriously hard to test. Suggest test shapes
  that surface real races: stress tests, ordering checks, shutdown-under-load scenarios.

- **Incremental progress** — the project aims to be done in a week. Help scope tasks to
  be completable in a single session, and flag when something risks becoming a rabbit
  hole.

## What to avoid

- Reaching for external abstractions that would hide the learning — the whole point is to
  build and understand these primitives directly.
- Suggesting optimizations before correctness is established.
- Silently working around a misunderstanding in the existing code — surface it instead.
