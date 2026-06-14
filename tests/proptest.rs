//! Property-based tests: "for all valid inputs, invariants hold."
//!
//! Uses proptest to generate random configurations — worker count, task count,
//! submission patterns — and verifies that no combination causes deadlocks,
//! lost tasks, or incorrect results. Catches edge cases no one thinks to write:
//! 0 tasks, 1 worker, max workers, etc.

use pool_of_threads::ThreadPool;
use proptest::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

proptest! {
    /// All spawned tasks must complete and increment the counter exactly once.
    /// Covers the core correctness invariant for any (workers, tasks) pair.
    #[test]
    fn all_tasks_complete_for_any_worker_and_task_count(
        workers in 1usize..=16,
        tasks in 0usize..=2000,
    ) {
        let pool = ThreadPool::new(workers);
        let counter = Arc::new(AtomicUsize::new(0));
        for _ in 0..tasks {
            let c = counter.clone();
            pool.spawn(move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        drop(pool);
        prop_assert_eq!(counter.load(Ordering::SeqCst), tasks);
    }

    /// Tasks that capture unique indices must sum correctly —
    /// verifies that no task is duplicated or dropped.
    #[test]
    fn task_indices_sum_correctly(
        workers in 1usize..=8,
        tasks in 1usize..=500,
    ) {
        let pool = ThreadPool::new(workers);
        let sum = Arc::new(AtomicUsize::new(0));
        let n = tasks; // usize from proptest
        for i in 0..n {
            let s = sum.clone();
            pool.spawn(move || {
                s.fetch_add(i, Ordering::SeqCst);
            });
        }
        drop(pool);
        let expected = n * (n - 1) / 2;
        prop_assert_eq!(sum.load(Ordering::SeqCst), expected);
    }

    /// Concurrent submission from multiple caller threads must not lose tasks.
    #[test]
    fn concurrent_submitters_dont_lose_tasks(
        workers in 1usize..=8,
        submitters in 1usize..=8,
        tasks_per_submitter in 1usize..=100,
    ) {
        let pool = Arc::new(ThreadPool::new(workers));
        let counter = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];
        for _ in 0..submitters {
            let p = pool.clone();
            let c = counter.clone();
            handles.push(std::thread::spawn(move || {
                for _ in 0..tasks_per_submitter {
                    let c2 = c.clone();
                    p.spawn(move || {
                        c2.fetch_add(1, Ordering::SeqCst);
                    });
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        drop(pool);
        prop_assert_eq!(
            counter.load(Ordering::SeqCst),
            submitters * tasks_per_submitter
        );
    }

    /// Single worker must complete all tasks — verifies the no-stealing path.
    #[test]
    fn single_worker_completes_any_task_count(
        tasks in 0usize..=2000,
    ) {
        let pool = ThreadPool::new(1);
        let counter = Arc::new(AtomicUsize::new(0));
        for i in 0..tasks {
            let c = counter.clone();
            pool.spawn(move || {
                c.fetch_add(i, Ordering::SeqCst);
            });
        }
        drop(pool);
        let n = tasks;
        let expected = if n == 0 { 0 } else { n * (n - 1) / 2 };
        prop_assert_eq!(counter.load(Ordering::SeqCst), expected);
    }

    /// Pool with exactly 2 workers — the minimum for stealing to be possible.
    /// This is the simplest case where work-stealing machinery is exercised.
    #[test]
    fn two_workers_complete_all_tasks(
        tasks in 1usize..=1000,
    ) {
        let pool = ThreadPool::new(2);
        let counter = Arc::new(AtomicUsize::new(0));
        for _ in 0..tasks {
            let c = counter.clone();
            pool.spawn(move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        drop(pool);
        prop_assert_eq!(counter.load(Ordering::SeqCst), tasks);
    }
}
