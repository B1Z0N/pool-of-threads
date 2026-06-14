//! Loom model-checking tests for the parking/shutdown coordination.
//!
//! Loom exhaustively explores every possible thread interleaving. If a
//! bug requires a specific ordering that only happens 0.001% of the time,
//! Loom finds it deterministically.
//!
//! These tests model the core synchronization pattern used in the pool:
//! Mutex + Condvar parking with an AtomicBool shutdown flag. The full
//! pool can't be Loom-tested because crossbeam uses real synchronization
//! primitives that Loom can't intercept.
//!
//! Run with: RUSTFLAGS="--cfg loom" cargo test --test loom_parking --features loom
#![allow(unexpected_cfgs)]

#[cfg(loom)]
use loom::sync::Arc;
#[cfg(loom)]
use loom::sync::atomic::{AtomicBool, Ordering};

#[cfg(loom)]
mod loom_tests {
    use super::*;
    use loom::sync::atomic::{AtomicBool as LoomAtomicBool, Ordering as LoomOrdering};
    use loom::sync::{Arc as LoomArc, Condvar, Mutex};
    use loom::thread;

    /// Models the park/wake pattern: workers park on a condvar when no work
    /// is available, and wake when the shutdown flag is set.
    ///
    /// Scenario: 2 workers, 1 shutdown signal. Verifies that:
    /// - All workers eventually observe shutdown=true
    /// - No worker parks forever after shutdown is signaled
    /// - The condvar notification reaches parked workers
    #[test]
    fn all_workers_wake_on_shutdown() {
        loom::model(|| {
            let shutdown = LoomArc::new(LoomAtomicBool::new(false));
            let parking = LoomArc::new((Mutex::new(()), Condvar::new()));
            let woken = LoomArc::new(LoomAtomicBool::new(false));

            // Worker 1
            let s1 = shutdown.clone();
            let p1 = parking.clone();
            let w1 = woken.clone();
            let worker1 = thread::spawn(move || {
                let (lock, cvar) = &*p1;
                let guard = lock.lock().unwrap();

                if !s1.load(LoomOrdering::Acquire) {
                    // Park — should be woken by shutdown notification.
                    let _guard = cvar.wait(guard).unwrap();
                }
                w1.store(true, LoomOrdering::Release);
            });

            // Worker 2
            let s2 = shutdown.clone();
            let p2 = parking.clone();
            let w2 = woken.clone();
            let worker2 = thread::spawn(move || {
                let (lock, cvar) = &*p2;
                let guard = lock.lock().unwrap();

                if !s2.load(LoomOrdering::Acquire) {
                    let _guard = cvar.wait(guard).unwrap();
                }
                w2.store(true, LoomOrdering::Release);
            });

            // Shutdown signaler
            let s3 = shutdown.clone();
            let p3 = parking.clone();
            thread::spawn(move || {
                let (lock, cvar) = &*p3;
                let _guard = lock.lock().unwrap();
                s3.store(true, LoomOrdering::Release);
                cvar.notify_all();
            });

            worker1.join().unwrap();
            worker2.join().unwrap();

            assert!(woken.load(LoomOrdering::Acquire));
        });
    }

    /// Models the lost-wakeup scenario: a worker checks "all queues empty"
    /// then spawn happens (push + notify), then worker parks. The condvar
    /// pattern must prevent the worker from missing the wakeup.
    ///
    /// This is the classic condvar race:
    ///   1. Worker: lock, check condition → false, about to wait
    ///   2. Spawner: push work, lock, notify_one, unlock
    ///   3. Worker: enter wait — misses the notification
    ///
    /// Loom explores both orderings: spawn-before-wait (correct) and
    /// spawn-during-wait-check (must still wake).
    #[test]
    fn no_lost_wakeup_when_work_arrives_during_check() {
        loom::model(|| {
            let work_available = LoomArc::new(LoomAtomicBool::new(false));
            let parking = LoomArc::new((Mutex::new(()), Condvar::new()));
            let task_ran = LoomArc::new(LoomAtomicBool::new(false));

            // Worker: checks condition, parks if no work.
            let w = work_available.clone();
            let p = parking.clone();
            let r = task_ran.clone();
            let worker = thread::spawn(move || {
                let (lock, cvar) = &*p;
                let guard = lock.lock().unwrap();

                if !w.load(LoomOrdering::Acquire) {
                    let _guard = cvar.wait(guard).unwrap();
                }
                // If we reach here, we either saw work_available=true
                // or were woken. In the real pool, we'd pop the task.
                r.store(true, LoomOrdering::Release);
            });

            // Spawner: pushes work, then notifies.
            let w2 = work_available.clone();
            let p2 = parking.clone();
            thread::spawn(move || {
                w2.store(true, LoomOrdering::Release);
                let (lock, cvar) = &*p2;
                let _guard = lock.lock().unwrap();
                cvar.notify_one();
            });

            worker.join().unwrap();
            assert!(task_ran.load(LoomOrdering::Acquire));
        });
    }

    /// Models the shutdown drain pattern: shutdown is signaled while
    /// work is still in the queue. Workers must not park — they must
    /// drain remaining work before exiting.
    #[test]
    fn workers_drain_remaining_work_after_shutdown() {
        loom::model(|| {
            let shutdown = LoomArc::new(LoomAtomicBool::new(false));
            let work_remaining = LoomArc::new(LoomAtomicBool::new(true));
            let drained = LoomArc::new(LoomAtomicBool::new(false));
            let parking = LoomArc::new((Mutex::new(()), Condvar::new()));

            // Worker: sees shutdown, but work remains → continue.
            let s = shutdown.clone();
            let w = work_remaining.clone();
            let d = drained.clone();
            let p = parking.clone();
            let worker = thread::spawn(move || {
                let (lock, cvar) = &*p;
                let guard = lock.lock().unwrap();

                if s.load(LoomOrdering::Acquire) {
                    if w.load(LoomOrdering::Acquire) {
                        // Work still present — must process it, not park.
                        // Simulate processing the work.
                        w.store(false, LoomOrdering::Release);
                        drop(guard);
                        d.store(true, LoomOrdering::Release);
                        return;
                    }
                }
                // Only park if no work AND no shutdown.
                let _guard = cvar.wait(guard).unwrap();
            });

            // Shutdown signaler — signals while work exists.
            let s2 = shutdown.clone();
            let p2 = parking.clone();
            thread::spawn(move || {
                s2.store(true, LoomOrdering::Release);
                let (lock, cvar) = &*p2;
                let _guard = lock.lock().unwrap();
                cvar.notify_all();
            });

            worker.join().unwrap();
            assert!(drained.load(LoomOrdering::Acquire));
        });
    }
}
