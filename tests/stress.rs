//! Integration test: correctness at scale.
//!
//! 10 000 tasks submitted from 8 threads across 8 workers.
//! Each task adds its index to an atomic sum. If any task is
//! lost, duplicated, or reordered in a way that breaks the
//! counter, the final sum won't match.

use pool_of_threads::ThreadPool;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

// NOTE: all tests below use Ordering::Relaxed on the AtomicU64 counter.
// This is safe because the final `load(Relaxed)` happens *after* `drop(pool)`,
// which joins every worker thread. `thread::spawn` / `JoinHandle::join` form a
// happens-before edge — the joining thread sees all side-effects from the
// joined thread, including every `fetch_add` the worker performed.

#[test]
fn stress_many_tasks_from_many_submitters() {
    let pool = Arc::new(ThreadPool::new(8));
    let total = Arc::new(AtomicU64::new(0));

    let n_submitters = 8u64;
    let tasks_per = 1250u64;

    let mut handles = vec![];
    for submitter_id in 0..n_submitters {
        let p = pool.clone();
        let t = total.clone();
        handles.push(std::thread::spawn(move || {
            for i in 0..tasks_per {
                let t = t.clone();
                let value = submitter_id * tasks_per + i;
                p.spawn(move || {
                    t.fetch_add(value, Ordering::Relaxed);
                });
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
    drop(pool);

    // Sum of all values 0..(n_submitters * tasks_per)
    let n = n_submitters * tasks_per;
    let expected: u64 = n * (n - 1) / 2;
    let actual = total.load(Ordering::Relaxed);
    assert_eq!(actual, expected, "stress test: counter mismatch");
}

#[test]
fn large_number_of_small_tasks() {
    // Many tiny tasks — stresses the queue and wake paths.
    let pool = ThreadPool::new(4);
    let counter = Arc::new(AtomicU64::new(0));
    let n = 20_000u64;

    for i in 0..n {
        let c = counter.clone();
        pool.spawn(move || {
            c.fetch_add(i, Ordering::Relaxed);
        });
    }
    drop(pool);

    let expected = n * (n - 1) / 2;
    assert_eq!(counter.load(Ordering::Relaxed), expected);
}
