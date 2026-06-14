use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

use crossbeam::deque::{
    self as cbdq,
    Steal::{self},
};

use crate::Task;

/// A work-stealing thread pool.
///
/// Workers own local FIFO queues. Tasks submitted via
/// [`spawn`](ThreadPool::spawn) land on a shared injector. Workers
/// check their local queue first, then the injector, then steal from
/// siblings (round-robin, starting from a random victim). Idle workers
/// park on a shared condvar until work arrives or shutdown is signaled.
///
/// The pool drains all remaining tasks and joins every worker thread
/// when dropped.
pub struct ThreadPool {
    queue: Arc<cbdq::Injector<Task>>,
    shutdown: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
    parking: Arc<(Mutex<()>, Condvar)>,
}

impl ThreadPool {
    /// Creates a thread pool with `n` worker threads.
    ///
    /// Each worker gets a local FIFO queue and a stealer handle for
    /// every sibling. Workers start running immediately in a loop:
    /// pop, steal, park.
    ///
    /// # Panics
    ///
    /// Panics if `n == 0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pool_of_threads::ThreadPool;
    ///
    /// let pool = ThreadPool::new(4);
    /// ```
    pub fn new(n: usize) -> Self {
        assert!(n > 0, "thread pool must have at least one worker");

        let queue = Arc::new(cbdq::Injector::<Task>::new());
        let shutdown = Arc::new(AtomicBool::new(false));

        let mut workers = Vec::with_capacity(n);
        let mut stealers = Vec::with_capacity(n);
        for _ in 0..n {
            let worker = cbdq::Worker::new_fifo();
            stealers.push(worker.stealer());
            workers.push(worker);
        }
        let stealers: Arc<[cbdq::Stealer<Task>]> = stealers.into();
        let parking = Arc::new((Mutex::new(()), Condvar::new()));

        let threads = workers
            .into_iter()
            .enumerate()
            .map(|(id, worker)| {
                let queue = queue.clone();
                let shutdown = shutdown.clone();
                let stealers = stealers.clone();
                let parking = parking.clone();

                thread::spawn(move || {
                    let mut steal_start = rand::random_range(0..n);
                    'worker: loop {
                        steal_start = (steal_start + 1) % n;
                        if let Some(task) = worker.pop() {
                            task.run();
                            continue;
                        }

                        match queue.steal_batch_and_pop(&worker) {
                            Steal::Success(task) => {
                                {
                                    let (lock, cvar) = &*parking;
                                    let _guard = lock.lock().unwrap();
                                    cvar.notify_all();
                                }
                                task.run();
                                continue;
                            }
                            Steal::Retry => continue,
                            Steal::Empty => {}
                        }

                        if n > 1 {
                            for offset in 0..n {
                                let i = (steal_start + offset) % n;
                                if i == id {
                                    continue;
                                }

                                match stealers[i].steal() {
                                    Steal::Success(task) => {
                                        task.run();
                                        continue 'worker;
                                    }
                                    Steal::Retry => continue 'worker,
                                    Steal::Empty => {}
                                }
                            }
                        }

                        if shutdown.load(Ordering::Acquire) {
                            if queue.is_empty() && stealers.iter().all(|s| s.is_empty()) {
                                break;
                            }
                            std::thread::yield_now();
                            continue;
                        }

                        let (lock, cvar) = &*parking;
                        let mut guard = lock.lock().unwrap();

                        // Single-worker pool: no stealers, skip the scan.
                        let all_empty = n == 1 || stealers.iter().all(|s| s.is_empty());
                        if queue.is_empty() && all_empty && !shutdown.load(Ordering::Acquire) {
                            guard = cvar.wait(guard).unwrap();
                        }
                    }
                })
            })
            .collect();
        Self { queue, shutdown, threads, parking }
    }

    /// Submits a closure for execution on the pool.
    ///
    /// The closure is pushed onto the global injector queue and one
    /// parked worker (if any) is notified. The closure always runs on
    /// a worker thread, never on the caller's thread.
    ///
    /// # Examples
    ///
    /// ```
    /// use pool_of_threads::ThreadPool;
    /// use std::sync::atomic::{AtomicBool, Ordering};
    /// use std::sync::Arc;
    ///
    /// let pool = ThreadPool::new(4);
    /// let done = Arc::new(AtomicBool::new(false));
    /// let d = done.clone();
    /// pool.spawn(move || d.store(true, Ordering::SeqCst));
    /// drop(pool);
    /// assert!(done.load(Ordering::SeqCst));
    /// ```
    pub fn spawn(&self, task: impl FnOnce() + Send + 'static) {
        let task = Task::new(task);
        self.queue.push(task);

        let (lock, cvar) = &*self.parking;
        {
            let _guard = lock.lock().unwrap();
            cvar.notify_one();
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        let (lock, cvar) = &*self.parking;
        {
            let _guard = lock.lock().unwrap();
            self.shutdown.store(true, Ordering::Release);
            cvar.notify_all();
        }

        // Drain and join workers. Task panics are caught by the worker
        // thread (std::thread::spawn isolates them) and surface here as
        // Err(...). We ignore them — the pool should shut down cleanly
        // even if individual tasks panicked.
        #[allow(unused_must_use)]
        for thread in self.threads.drain(..) {
            thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::thread;
    use std::time::{Duration, Instant};

    // ── existence ───────────────────────────────────────────────────

    #[test]
    fn create_pool_with_n_threads() {
        let _pool = ThreadPool::new(4);
    }

    // ── basic execution ─────────────────────────────────────────────

    #[test]
    fn spawn_runs_one_task() {
        let pool = ThreadPool::new(4);
        let flag = Arc::new(AtomicBool::new(false));

        let f = flag.clone();
        pool.spawn(move || f.store(true, Ordering::SeqCst));
        drop(pool);

        assert!(flag.load(Ordering::SeqCst), "task was not executed");
    }

    #[test]
    fn spawn_runs_many_tasks() {
        let pool = ThreadPool::new(4);
        let counter = Arc::new(AtomicUsize::new(0));
        let n = 100;

        for i in 0..n {
            let c = counter.clone();
            pool.spawn(move || {
                c.fetch_add(i, Ordering::SeqCst);
            });
        }
        drop(pool);

        // sum 0..100 = 4950
        assert_eq!(counter.load(Ordering::SeqCst), n * (n - 1) / 2);
    }

    #[test]
    fn tasks_run_on_worker_not_caller() {
        let pool = ThreadPool::new(4);
        let caller_tid = thread::current().id();
        let different = Arc::new(AtomicBool::new(false));

        let d = different.clone();
        pool.spawn(move || {
            if thread::current().id() != caller_tid {
                d.store(true, Ordering::SeqCst);
            }
        });
        drop(pool);

        assert!(different.load(Ordering::SeqCst), "task ran on calling thread instead of a worker");
    }

    // ── concurrent submission ───────────────────────────────────────

    #[test]
    fn concurrent_spawn_from_multiple_threads() {
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
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        drop(pool);

        assert_eq!(counter.load(Ordering::SeqCst), n_submitters * tasks_per);
    }

    // ── shutdown semantics ──────────────────────────────────────────

    #[test]
    fn shutdown_waits_for_pending_tasks() {
        let pool = ThreadPool::new(4);
        let counter = Arc::new(AtomicUsize::new(0));
        let n = 20;

        for _ in 0..n {
            let c = counter.clone();
            pool.spawn(move || {
                thread::sleep(Duration::from_millis(5));
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        drop(pool);

        assert_eq!(counter.load(Ordering::SeqCst), n);
    }

    // ── drop safety ─────────────────────────────────────────────────

    #[test]
    fn drop_without_shutdown_does_not_hang() {
        let pool = ThreadPool::new(4);
        let flag = Arc::new(AtomicBool::new(false));
        let f = flag.clone();
        pool.spawn(move || {
            f.store(true, Ordering::SeqCst);
        });
        drop(pool);
    }

    // ── edge cases ──────────────────────────────────────────────────

    #[test]
    #[should_panic(expected = "at least one worker")]
    fn zero_threads_panics() {
        ThreadPool::new(0);
    }

    #[test]
    fn single_worker_completes_all_tasks() {
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
    }

    // ── parking / wake behaviour ────────────────────────────────────

    #[test]
    fn parked_worker_wakes_for_new_task() {
        let pool = ThreadPool::new(2);
        let flag = Arc::new(AtomicBool::new(false));

        // Let workers drain and park
        thread::sleep(Duration::from_millis(50));

        // Submit — must wake a parked worker
        let f = flag.clone();
        let start = Instant::now();
        pool.spawn(move || f.store(true, Ordering::SeqCst));
        drop(pool);

        assert!(flag.load(Ordering::SeqCst));
        // If workers were genuinely parked, wake should be near-instant.
        // 500ms is generous for native runs; TSan instrumentation can
        // slow things down, so we allow up to 10s.
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_secs(10),
            "task took {:?} — workers may not be parking/waking correctly",
            elapsed
        );
    }

    // ── panic propagation ───────────────────────────────────────────

    #[test]
    fn panic_in_task_is_isolated() {
        let pool = ThreadPool::new(2);

        // This task panics, but the worker thread survives.
        pool.spawn(|| panic!("task B panics — worker should survive"));

        // This task must still run after the panic.
        let ran = Arc::new(AtomicBool::new(false));
        let r = ran.clone();
        pool.spawn(move || r.store(true, Ordering::SeqCst));
        drop(pool);

        assert!(ran.load(Ordering::SeqCst), "task after panic did not run");
    }

    // ── compile-time contracts ──────────────────────────────────────

    #[allow(dead_code)]
    fn assert_send_sync()
    where
        ThreadPool: Send + Sync,
    {
    }

    #[test]
    fn pool_is_send_and_sync() {
        // This test "runs" the where clause — if ThreadPool didn't
        // implement Send + Sync, the code wouldn't compile.
        assert_send_sync();
    }
}
