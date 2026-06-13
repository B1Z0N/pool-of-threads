//! Integration test: pool lifecycle smoke test.
//!
//! Creates and destroys pools in a loop with tasks that allocate.
//! If the pool catastrophically leaks task closures, worker state,
//! or queue buffers, the process RSS will grow without bound.
//!
//! The Rust test below catches only catastrophic leaks (OOM, thread
//! exhaustion). For precise leak detection, run under a native tool:
//!
//! ## macOS (Apple Silicon / x86)
//!
//! ```bash
//! cargo build --release
//! leaks --atExit -- ./target/release/pool-of-threads
//! # Or, for one-shot:
//! MallocStackLogging=1 cargo run --release &
//! leaks $(pgrep pool-of-threads)
//! kill %1
//! ```
//!
//! ## Linux
//!
//! ```bash
//! cargo build --release
//! valgrind --leak-check=full --show-leak-kinds=all \
//!   ./target/release/pool-of-threads
//! ```

use pool_of_threads::ThreadPool;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn many_pool_lifecycles_no_catastrophic_leak() {
    // 50 pools, each spawning 100 tasks that heap-allocate 4 KiB.
    // If there's a catastrophic leak in the pool machinery, this will
    // balloon RSS and may trigger OOM in constrained CI.
    for cycle in 0..50 {
        let pool = ThreadPool::new(4);
        for _ in 0..100 {
            pool.spawn(|| {
                let _buf: Vec<u8> = vec![0; 4096];
                // _buf dropped here — memory freed.
            });
        }
        drop(pool);

        // Progress indicator on stderr (visible with -- --nocapture).
        if cycle % 10 == 0 {
            eprintln!("  lifecycle smoke test: cycle {cycle}/50");
        }
    }
}

#[test]
fn all_tasks_complete_under_allocation_load() {
    // 1000 tasks, each allocating 1 KiB and contributing its allocation
    // size to a counter. Verifies correctness under allocation pressure:
    // every task runs and contributes exactly the expected amount.
    let pool = ThreadPool::new(4);
    let counter = Arc::new(AtomicUsize::new(0));
    let n = 1000;

    for _ in 0..n {
        let c = counter.clone();
        pool.spawn(move || {
            // Allocate, use, drop.
            let v: Vec<u8> = vec![0; 1024];
            c.fetch_add(v.len(), Ordering::SeqCst);
        });
    }
    drop(pool);

    assert_eq!(counter.load(Ordering::SeqCst), n * 1024);
}
