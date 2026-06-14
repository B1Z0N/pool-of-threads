//! Soak tests: long-running, high-volume stress under continuous load.
//!
//! Concurrent bugs are probabilistic — a test that passes in 20ms may
//! deadlock at 30 seconds under sustained pressure. These tests run
//! continuously, checking invariants on every cycle. They are gated
//! behind `#[ignore]` so they don't run on every `cargo test`.
//!
//! Run them explicitly:
//!   cargo test --test soak -- --ignored --nocapture
//!
//! Control duration via env var:
//!   SOAK_SECS=10 cargo test --test soak -- --ignored --nocapture

use pool_of_threads::ThreadPool;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

fn soak_duration() -> Duration {
    let secs: u64 = std::env::var("SOAK_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(5);
    Duration::from_secs(secs)
}

/// Continuous concurrent submission: N threads push tasks as fast as
/// possible while workers drain. Checks every 100ms that tasks aren't
/// being lost, duplicated, or stuck.
#[test]
#[ignore]
fn continuous_concurrent_submission_under_sustained_load() {
    let deadline = Instant::now() + soak_duration();
    let cycle = Arc::new(AtomicU64::new(0));

    while Instant::now() < deadline {
        let pool = Arc::new(ThreadPool::new(4));
        let counter = Arc::new(AtomicUsize::new(0));
        let spawned = Arc::new(AtomicUsize::new(0));
        let live = Arc::new(AtomicBool::new(true));

        // Spawner threads.
        let mut spawners = vec![];
        for _ in 0..4 {
            let p = pool.clone();
            let c = counter.clone();
            let s = spawned.clone();
            let l = live.clone();
            spawners.push(thread::spawn(move || {
                while l.load(Ordering::SeqCst) {
                    let c = c.clone();
                    p.spawn(move || {
                        c.fetch_add(1, Ordering::SeqCst);
                    });
                    s.fetch_add(1, Ordering::SeqCst);
                    thread::yield_now();
                }
                // Final batch.
                for _ in 0..50 {
                    let c = c.clone();
                    p.spawn(move || {
                        c.fetch_add(1, Ordering::SeqCst);
                    });
                    s.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }

        // Let tasks pile up for a bit.
        thread::sleep(Duration::from_millis(5));
        live.store(false, Ordering::SeqCst);

        for h in spawners {
            h.join().unwrap();
        }
        drop(pool);

        let total_spawned = spawned.load(Ordering::SeqCst);
        let total_ran = counter.load(Ordering::SeqCst);
        assert!(total_ran > 0, "soak cycle {}: no tasks executed", cycle.load(Ordering::Relaxed));
        assert_eq!(
            total_ran,
            total_spawned,
            "soak cycle {}: spawned {total_spawned} but only {total_ran} ran",
            cycle.load(Ordering::Relaxed)
        );

        let c = cycle.fetch_add(1, Ordering::Relaxed) + 1;
        if c.is_multiple_of(20) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            eprintln!("  soak: cycle {c}, {total_ran} tasks, {:.0?} remaining", remaining);
        }
    }
}

/// Create and destroy pools in a tight loop with allocated tasks.
/// Catches leaks that take many iterations to surface.
#[test]
#[ignore]
fn continuous_pool_lifecycle_with_allocation() {
    let deadline = Instant::now() + soak_duration();
    let mut cycle: u64 = 0;

    while Instant::now() < deadline {
        let pool = ThreadPool::new(4);
        let counter = Arc::new(AtomicUsize::new(0));
        for _ in 0..200 {
            let c = counter.clone();
            pool.spawn(move || {
                let _buf: Vec<u8> = vec![0; 1024];
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        drop(pool);
        assert_eq!(counter.load(Ordering::SeqCst), 200);

        cycle += 1;
        if cycle.is_multiple_of(100) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            eprintln!("  soak lifecycle: cycle {cycle}, {:.0?} remaining", remaining);
        }
    }
}

/// Single worker, continuous submission — the simplest path, run long.
/// If there's a bug in the non-stealing code path, this finds it.
#[test]
#[ignore]
fn continuous_single_worker_stress() {
    let deadline = Instant::now() + soak_duration();
    let mut cycle: u64 = 0;

    while Instant::now() < deadline {
        let pool = ThreadPool::new(1);
        let counter = Arc::new(AtomicUsize::new(0));
        let n = 500;
        for i in 0..n {
            let c = counter.clone();
            pool.spawn(move || {
                c.fetch_add(i, Ordering::SeqCst);
            });
        }
        drop(pool);
        assert_eq!(counter.load(Ordering::SeqCst), n * (n - 1) / 2);

        cycle += 1;
        if cycle.is_multiple_of(200) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            eprintln!("  soak single-worker: cycle {cycle}, {:.0?} remaining", remaining);
        }
    }
}
