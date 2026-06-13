//! Integration test: pool makes progress when some workers are blocked.
//!
//! Strategy: block one or more workers on a barrier, flood the pool with
//! fast tasks, and verify the remaining workers pick them up. This exercises
//! the injector-steal path — when a free worker finds its local queue empty,
//! it pulls batches from the global injector via `steal_batch_and_pop`.
//!
//! NOTE: these tests do NOT prove cross-worker work-stealing (one worker
//! stealing from another's local queue). All tasks flow through the global
//! injector because `spawn()` pushes there directly. Proving true work-stealing
//! would require tasks to land in a worker's local queue (e.g. via batch
//! stealing), which is an internal detail not observable from the public API.

use pool_of_threads::ThreadPool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

#[test]
fn progress_continues_when_one_worker_is_blocked() {
    let pool = ThreadPool::new(2);
    let barrier = Arc::new(Barrier::new(2));
    let counter = Arc::new(AtomicUsize::new(0));

    // Task A: block one worker on a barrier.
    let b = barrier.clone();
    pool.spawn(move || {
        b.wait(); // parked here until test releases
    });

    // Give the worker time to grab task A and park on the barrier.
    thread::sleep(Duration::from_millis(20));

    // Flood with fast tasks. The free worker handles these via the
    // injector (not by stealing from the blocked worker's local queue,
    // which never received any of these tasks).
    let n = 500;
    for _ in 0..n {
        let c = counter.clone();
        pool.spawn(move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
    }

    // Give the free worker time to process via injector-steal.
    thread::sleep(Duration::from_millis(50));

    let mid = counter.load(Ordering::SeqCst);
    assert!(
        mid > 400,
        "only {mid}/{n} fast tasks completed — injector-steal path may not be working"
    );

    // Release the blocked worker so the pool can drain.
    barrier.wait();
    drop(pool);

    assert_eq!(counter.load(Ordering::SeqCst), n);
}

#[test]
fn remaining_workers_handle_load_when_others_blocked() {
    // With 4 workers, block 3 on a barrier. The 4th handles 100 tasks
    // from the injector. Verifies the injector-steal path works even
    // when most workers are unavailable.
    let pool = ThreadPool::new(4);
    let barrier = Arc::new(Barrier::new(4));

    // Block 3 workers on a barrier.
    for _ in 0..3 {
        let b = barrier.clone();
        pool.spawn(move || {
            b.wait();
        });
    }

    thread::sleep(Duration::from_millis(20));

    // The 4th worker should handle these from the injector.
    let counter = Arc::new(AtomicUsize::new(0));
    for _ in 0..100 {
        let c = counter.clone();
        pool.spawn(move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
    }

    thread::sleep(Duration::from_millis(30));
    let completed = counter.load(Ordering::SeqCst);
    assert!(
        completed > 50,
        "only {completed}/100 tasks ran — remaining worker should have grabbed them from injector"
    );

    barrier.wait();
    drop(pool);
    assert_eq!(counter.load(Ordering::SeqCst), 100);
}
