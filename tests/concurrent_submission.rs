//! Integration test: heavy concurrent submission correctness.
//!
//! Multiple threads call `spawn()` in a tight loop, then stop, then the pool
//! is dropped. The test verifies that (a) the process doesn't hang, and (b)
//! every task that was pushed to a queue actually runs.
//!
//! NOTE: this does not test a genuine concurrent-spawn+drain race — Rust's
//! ownership model prevents one thread from dropping the pool while another
//! holds a reference. The spawners finish first, then the pool drains.

use pool_of_threads::ThreadPool;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

#[test]
fn heavy_concurrent_submission_completes_all_tasks() {
    // Run the full scenario multiple times — concurrent bugs are probabilistic.
    for _ in 0..10 {
        let pool = Arc::new(ThreadPool::new(4));
        let counter = Arc::new(AtomicUsize::new(0));
        let spawned = Arc::new(AtomicUsize::new(0));
        let live = Arc::new(AtomicBool::new(true));

        // Spawner threads: push tasks as fast as possible.
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
                // Push a final batch after the flag drops.
                for _ in 0..100 {
                    let c = c.clone();
                    p.spawn(move || {
                        c.fetch_add(1, Ordering::SeqCst);
                    });
                    s.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }

        // Let tasks pile up.
        thread::sleep(Duration::from_millis(5));
        live.store(false, Ordering::SeqCst);

        // Wait for spawner threads to finish.
        for h in spawners {
            h.join().unwrap();
        }

        // Now drop the pool — this triggers the real shutdown+drain.
        drop(pool);

        let total_spawned = spawned.load(Ordering::SeqCst);
        let total_ran = counter.load(Ordering::SeqCst);
        assert!(total_ran > 0, "no tasks executed");
        assert_eq!(
            total_ran, total_spawned,
            "spawned {total_spawned} tasks but only {total_ran} ran — tasks were lost"
        );
    }
}
