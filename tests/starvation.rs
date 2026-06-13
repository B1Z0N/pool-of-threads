//! Integration test: starvation resistance under load.
//!
//! Verifies that the injector path doesn't indefinitely starve tasks
//! when the pool is under heavy pressure. These tests exercise throughput,
//! not a formal fairness guarantee — the injector is a Chase-Lev deque
//! where workers pull batches, so a task submitted during a flood will
//! eventually get picked up as long as workers drain faster than submitters
//! push. With enough submitters, starvation is still possible.

use pool_of_threads::ThreadPool;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn marked_task_completes_during_injector_flood() {
    let pool = Arc::new(ThreadPool::new(4));
    let done = Arc::new(AtomicBool::new(false));
    let flood = Arc::new(AtomicBool::new(true));

    // Two threads continuously push empty tasks onto the injector.
    // With 4 workers, this leaves at least 2 workers available to
    // drain the injector — the marked task should be picked up
    // within milliseconds. Three flooders + 4 workers only leaves
    // 1 free worker and can make the 1-second assertion flaky in CI.
    let mut flooders = vec![];
    for _ in 0..2 {
        let p = pool.clone();
        let f = flood.clone();
        flooders.push(thread::spawn(move || {
            while f.load(Ordering::SeqCst) {
                p.spawn(|| {});
            }
        }));
    }

    // Give the flood a head start so the injector is saturated.
    thread::sleep(Duration::from_millis(10));

    // Submit one marked task amid the storm.
    let d = done.clone();
    let start = Instant::now();
    pool.spawn(move || d.store(true, Ordering::SeqCst));

    // Poll with a hard deadline.
    let deadline = start + Duration::from_secs(5);
    while !done.load(Ordering::SeqCst) {
        if Instant::now() > deadline {
            flood.store(false, Ordering::SeqCst);
            for h in flooders {
                h.join().unwrap();
            }
            drop(pool);
            panic!("marked task did not complete within 5 s — injector path may be stuck");
        }
        thread::sleep(Duration::from_millis(1));
    }

    flood.store(false, Ordering::SeqCst);
    for h in flooders {
        h.join().unwrap();
    }
    drop(pool);

    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(1),
        "marked task took {elapsed:.2?} — may be starving behind flood tasks"
    );
}

#[test]
fn light_task_completes_while_all_workers_busy() {
    // Saturate every worker with a heavy task, then submit a light one.
    // The light task must complete — it should be picked up as soon as
    // the first heavy task finishes.
    let pool = ThreadPool::new(4);
    let light_done = Arc::new(AtomicBool::new(false));

    // 4 heavy tasks for 4 workers — no free workers.
    for _ in 0..4 {
        pool.spawn(|| {
            thread::sleep(Duration::from_millis(200));
        });
    }

    // Small delay so workers grab the heavy tasks first.
    thread::sleep(Duration::from_millis(10));

    // Submit a light task — must wait for one heavy task to finish.
    let ld = light_done.clone();
    let start = Instant::now();
    pool.spawn(move || ld.store(true, Ordering::SeqCst));

    drop(pool);

    assert!(light_done.load(Ordering::SeqCst));
    // First heavy task finishes at ~200ms. With scheduling overhead,
    // 500ms is very generous.
    assert!(
        start.elapsed() < Duration::from_millis(500),
        "light task starved by heavy tasks: {:?}",
        start.elapsed()
    );
}
