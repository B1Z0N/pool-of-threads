//! Perturbation injection tests: deliberate scheduling interference.
//!
//! The idea: take the core invariants and insert `thread::yield_now()`
//! at decision points. If any yield causes a test to fail, the code has
//! an implicit scheduling assumption that's not guaranteed.
//!
//! These are the poor-man's Loom — each yield point forces the OS to
//! reschedule threads, exposing bugs that only manifest under specific
//! interleavings. A test that passes 100/100 without yields but fails
//! 1/100 with yields has a real race condition.

use pool_of_threads::ThreadPool;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

/// Rapid pool create/destroy with yields between each cycle.
/// Exposes any race between thread startup and teardown.
#[test]
fn rapid_pool_lifecycle_with_yields() {
    for cycle in 0..200 {
        let pool = ThreadPool::new(2);
        for _ in 0..50 {
            pool.spawn(|| {});
        }
        drop(pool);
        // Yield after drop — if the pool left any dangling state,
        // the next cycle might trip over it.
        thread::yield_now();
        if cycle % 50 == 0 {
            eprintln!("  perturbation lifecycle: cycle {cycle}/200");
        }
    }
}

/// Spawn from many threads, yield between every spawn call.
/// Forces the injector to handle interleaved pushes.
#[test]
fn concurrent_spawn_with_yield_between_every_call() {
    for _ in 0..100 {
        let pool = Arc::new(ThreadPool::new(4));
        let counter = Arc::new(AtomicUsize::new(0));
        let n_submitters = 4;
        let tasks_per = 25;
        let mut handles = vec![];
        for _ in 0..n_submitters {
            let p = pool.clone();
            let c = counter.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..tasks_per {
                    let c2 = c.clone();
                    p.spawn(move || {
                        c2.fetch_add(1, Ordering::SeqCst);
                    });
                    thread::yield_now();
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        drop(pool);
        assert_eq!(counter.load(Ordering::SeqCst), n_submitters * tasks_per);
    }
}

/// Submit tasks, yield, then drop. Exercises the boundary between
/// "pool still accepting work" and "pool draining."
#[test]
fn submit_then_yield_then_drop() {
    for _ in 0..100 {
        let pool = ThreadPool::new(2);
        let counter = Arc::new(AtomicUsize::new(0));
        for _ in 0..20 {
            let c = counter.clone();
            pool.spawn(move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        // Yield before drop — workers may have grabbed some tasks
        // already, some may still be in the injector.
        thread::yield_now();
        thread::yield_now();
        drop(pool);
        assert_eq!(counter.load(Ordering::SeqCst), 20);
    }
}

/// Submit tasks after a yield — workers may have parked by now.
/// Verifies park→wake path under scheduling perturbation.
#[test]
fn delayed_submission_after_yield() {
    for _ in 0..50 {
        let pool = ThreadPool::new(2);
        // Let workers drain and possibly park.
        thread::yield_now();
        thread::sleep(Duration::from_micros(100));
        thread::yield_now();

        let done = Arc::new(AtomicBool::new(false));
        let d = done.clone();
        pool.spawn(move || {
            d.store(true, Ordering::SeqCst);
        });

        thread::yield_now();
        drop(pool);
        assert!(done.load(Ordering::SeqCst), "task did not run after yield delay");
    }
}

/// Saturate all workers, then submit more tasks with yields between.
/// Tests injector-steal path under scheduling interference.
#[test]
fn saturated_then_submit_with_yields() {
    for _ in 0..20 {
        let pool = ThreadPool::new(2);
        let counter = Arc::new(AtomicUsize::new(0));

        // Block both workers.
        for _ in 0..2 {
            pool.spawn(|| {
                thread::sleep(Duration::from_millis(10));
            });
        }
        thread::sleep(Duration::from_millis(1));

        // Submit tasks with yields — injector path must pick them up.
        for i in 0..50 {
            let c = counter.clone();
            pool.spawn(move || {
                c.fetch_add(i, Ordering::SeqCst);
            });
            if i % 5 == 0 {
                thread::yield_now();
            }
        }
        drop(pool);
        let n: usize = 50;
        assert_eq!(counter.load(Ordering::SeqCst), n * (n - 1) / 2);
    }
}

/// Alternating spawn and yield, single worker.
/// Most likely to expose ordering bugs in the simplest configuration.
#[test]
fn single_worker_alternating_spawn_yield() {
    for _ in 0..50 {
        let pool = ThreadPool::new(1);
        let counter = Arc::new(AtomicUsize::new(0));
        for i in 0..30 {
            let c = counter.clone();
            pool.spawn(move || {
                c.fetch_add(i, Ordering::SeqCst);
            });
            thread::yield_now();
        }
        drop(pool);
        let n: usize = 30;
        assert_eq!(counter.load(Ordering::SeqCst), n * (n - 1) / 2);
    }
}
