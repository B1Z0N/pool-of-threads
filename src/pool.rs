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

pub struct ThreadPool {
    queue: Arc<cbdq::Injector<Task>>,
    shutdown: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
    parking: Arc<(Mutex<()>, Condvar)>,
}

impl ThreadPool {
    pub fn new(n: usize) -> Self {
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

                        if queue.is_empty()
                            && stealers.iter().all(|s| s.is_empty())
                            && !shutdown.load(Ordering::Acquire)
                        {
                            guard = cvar.wait(guard).unwrap();
                        }
                    }
                })
            })
            .collect();
        Self { queue, shutdown, threads, parking }
    }

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

        for thread in self.threads.drain(..) {
            thread.join().unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

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
}
