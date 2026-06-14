//! `pool-of-threads` — A work-stealing thread pool scheduler.
//!
//! ## Architecture
//!
//! Each worker thread owns a local FIFO queue
//! ([`crossbeam::deque::Worker`]). Incoming tasks land on a shared
//! injector queue. Workers pop from their local queue first; when idle,
//! they try the global injector, then attempt to steal from sibling
//! workers (round-robin starting from a random victim). When every queue
//! is empty, workers park on a shared condvar and wake when new work
//! arrives or at shutdown.
//!
//! ## Example
//!
//! ```
//! use pool_of_threads::ThreadPool;
//! use std::sync::atomic::{AtomicUsize, Ordering};
//! use std::sync::Arc;
//!
//! let pool = ThreadPool::new(4);
//! let counter = Arc::new(AtomicUsize::new(0));
//!
//! for i in 0..100 {
//!     let c = counter.clone();
//!     pool.spawn(move || { c.fetch_add(i, Ordering::SeqCst); });
//! }
//!
//! drop(pool); // waits for all tasks
//! assert_eq!(counter.load(Ordering::SeqCst), 4950);
//! ```
//!
//! ## How it works
//!
//! - **`spawn(f)`** pushes `f` onto the global injector queue and
//!   notifies one parked worker.
//! - **Worker loop**: pop local FIFO → steal batch from injector →
//!   steal from a random sibling → park on condvar.
//! - **Shutdown**: `Drop` sets an atomic flag, notifies all parked
//!   workers, drains remaining tasks, and joins every thread.

pub mod pool;
pub mod task;

pub use pool::ThreadPool;
pub use task::Task;

/// Deterministic CPU-bound work for benchmarks.
///
/// Runs an LCG for `iterations` steps. On modern hardware:
/// - 1_000 iterations ≈ 1 µs
/// - 1_000_000 iterations ≈ 1 ms
///
/// Not part of the public API — only exposed so benchmarks can use it
/// without duplicating the implementation across separate benchmark binaries.
#[doc(hidden)]
pub fn bench_cpu_work(iterations: u64) -> u64 {
    let mut x: u64 = 0;
    for _ in 0..iterations {
        x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
    }
    x
}
