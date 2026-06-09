//! `pool-of-threads` — A work-stealing thread pool scheduler.
//!
//! ## Architecture (planned)
//!
//! - **Workers** — OS threads with per-worker task queues (crossbeam-deque).
//! - **Global queue** — fallback for initial task submission.
//! - **Work-stealing** — idle workers steal from sibling queues (random victim).
//! - **Parking** — workers park on a condvar when all queues are empty.
//! - **Shutdown** — graceful drain + join.
//!
//! ## Usage (planned)
//!
//! ```rust,ignore
//! let pool = ThreadPool::new(num_cpus::get());
//! pool.spawn(|| { /* work */ });
//! pool.shutdown();
//! ```

pub mod pool;
pub mod task;

pub use pool::ThreadPool;
pub use task::Task;
