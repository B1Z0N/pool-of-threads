# CLAUDE.md — Pool of Threads

Context for Claude Code sessions in this project.

## What this project is

An educational Rust project that builds a work-stealing thread pool scheduler entirely
from scratch. The goal is to deeply understand concurrent scheduling: how tasks move from
a submission queue onto per-worker queues, how idle workers steal tasks from busy ones
without bottlenecking on a single lock, how threads park and wake with minimal overhead,
and how a clean shutdown drains in-flight work and joins every thread safely.

This is a week-long focused project — scope decisions should favor learning and
completeness over perfection.

## How Claude can help

- **Planning the next step** — before starting a new module or wiring pieces together,
  talk through the design: what invariants need to hold, what the ownership story is,
  where concurrency bugs typically appear in this kind of code.

- **Reviewing safety** — when unsafe code is written or modified, Claude should audit the
  safety argument, not just accept the comment at face value. Point out if an invariant
  is incomplete or if the surrounding safe code can violate it.

- **Explaining concepts** — this is a learning project. Whenever a tradeoff is made
  (queue strategy, parking strategy, atomic ordering choice, shutdown protocol), explain
  what alternatives exist and what each costs.

- **Debugging concurrency** — when tests fail intermittently or a scenario is hard to
  reason about, help construct a minimal reproducer and reason through the happens-before
  relationships.

- **Keeping scope tight** — if a task is drifting into a rabbit hole, flag it. Suggest
  the smallest version of a thing that still teaches the lesson.

## What to avoid

- Suggesting external abstractions that would hide the learning — the whole point is to
  build and reason about the primitives directly.
- Applying optimizations before the code is correct and understood.
- Making silent choices: if there is a meaningful tradeoff, surface it.
