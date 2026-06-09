use std::{sync::{Arc, Mutex, mpsc}, thread::{self, JoinHandle}};

use crate::Task;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Task>>,
}

struct Worker {
    id: usize,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Task>>>) -> Self {
        let thread = thread::spawn(move || {
            loop {
                let message = receiver.lock().unwrap().recv();

                match message {
                    Ok(task) => task.run(),
                    Err(_) => {
                        println!("Worker {} disconnected. Shutting down.", id);
                        break;
                    }
                }
            }
        });
        Self { id, thread: Some(thread) }
    }
}

impl ThreadPool {
    pub fn new(n: usize) -> Self {
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let workers = (0..n)
            .map(|id| {
                Worker::new(id, receiver.clone())
            }).collect();
        Self {
            workers,
            sender: Some(sender),
        }
    }

    pub fn spawn(&self, task: impl FnOnce() + Send + 'static) {
        let task = Task::new(task);
        if let Err(_) = self.sender.as_ref().unwrap().send(task) {
            println!("Pool couldn't schedule a task.");
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        let sender = self.sender.take();
        drop(sender);
        for worker in &mut self.workers {
            worker.thread.take().unwrap().join();
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
        let pool = ThreadPool::new(4);
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

        assert!(
            different.load(Ordering::SeqCst),
            "task ran on calling thread instead of a worker"
        );
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
