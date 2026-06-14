//! Overhead microbenchmarks: scheduling cost and park/wake latency.
//!
//! Answers:
//! - "What is the per-task scheduling overhead?"
//! - "How fast does an idle worker respond to new work?"
//!
//! These are the asymptotes — no real workload can be faster than these numbers.

use criterion::{BenchmarkId, Criterion, black_box};
use pool_of_threads::ThreadPool;
use std::hint;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

/// Measures pure scheduling overhead: submit N empty tasks, divide total time
/// by N. Compares 1-worker vs multi-worker pools to isolate coordination cost.
fn bench_per_task_overhead(c: &mut Criterion) {
    let workers = std::thread::available_parallelism().unwrap().get();
    let mut group = c.benchmark_group("overhead/per_task");

    for &n in &[1_000u64, 10_000, 100_000] {
        // Multi-worker pool
        group.bench_with_input(BenchmarkId::new("pool_multi", n), &n, |b, &n| {
            b.iter_custom(|iters| {
                let mut elapsed = std::time::Duration::ZERO;
                for _ in 0..iters {
                    let pool = ThreadPool::new(workers);
                    let counter = Arc::new(AtomicU64::new(0));
                    let t0 = Instant::now();
                    for _ in 0..n {
                        let c = counter.clone();
                        pool.spawn(move || {
                            c.fetch_add(1, Ordering::Relaxed);
                        });
                    }
                    drop(pool);
                    elapsed += t0.elapsed();
                    assert_eq!(counter.load(Ordering::Relaxed), n);
                }
                elapsed
            });
        });

        // Single-worker pool — isolates coordination overhead
        group.bench_with_input(BenchmarkId::new("pool_single", n), &n, |b, &n| {
            b.iter_custom(|iters| {
                let mut elapsed = std::time::Duration::ZERO;
                for _ in 0..iters {
                    let pool = ThreadPool::new(1);
                    let counter = Arc::new(AtomicU64::new(0));
                    let t0 = Instant::now();
                    for _ in 0..n {
                        let c = counter.clone();
                        pool.spawn(move || {
                            c.fetch_add(1, Ordering::Relaxed);
                        });
                    }
                    drop(pool);
                    elapsed += t0.elapsed();
                    assert_eq!(counter.load(Ordering::Relaxed), n);
                }
                elapsed
            });
        });

        // Sequential baseline — no pool at all
        group.bench_with_input(BenchmarkId::new("sequential", n), &n, |b, &n| {
            b.iter_custom(|iters| {
                let mut elapsed = std::time::Duration::ZERO;
                for _ in 0..iters {
                    let t0 = Instant::now();
                    for _ in 0..n {
                        black_box(());
                    }
                    elapsed += t0.elapsed();
                }
                elapsed
            });
        });
    }
    group.finish();
}

/// Measures wake-from-park latency: submit a single task to an idle pool
/// (all workers parked on condvar) and measure wall-clock time from spawn
/// to task execution start.
fn bench_park_wake_latency(c: &mut Criterion) {
    let workers = std::thread::available_parallelism().unwrap().get();
    let mut group = c.benchmark_group("overhead/park_wake");

    group.bench_function("latency", |b| {
        let pool = ThreadPool::new(workers);
        // Separate ready flag + value to avoid the zero-means-not-ready
        // ambiguity. Acquire/Release ordering prevents the compiler from
        // hoisting the spin-wait load out of the loop (Relaxed + spin_loop
        // is UB-prone on ARM: spin_loop is nomem, so the compiler can
        // assume memory doesn't change across it).
        let ready: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let latency_ns: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));

        b.iter_custom(|iters| {
            let mut total_latency_ns: u64 = 0;
            for _ in 0..iters {
                let t_spawn = Instant::now();
                let r = ready.clone();
                let l = latency_ns.clone();
                pool.spawn(move || {
                    let task_start = Instant::now();
                    let delta_ns = task_start.duration_since(t_spawn).as_nanos() as u64;
                    l.store(delta_ns, Ordering::Relaxed);
                    r.store(true, Ordering::Release);
                });

                // Acquire pairs with the task's Release store on the ready flag,
                // forming a happens-before edge. The latency value (Relaxed) is
                // visible because it was stored before the Release.
                while !ready.load(Ordering::Acquire) {
                    hint::spin_loop();
                }
                total_latency_ns += latency_ns.load(Ordering::Relaxed);
                ready.store(false, Ordering::Release);
                latency_ns.store(0, Ordering::Relaxed);

                // Brief pause to let workers park again.
                // On a machine with workers+1 cores, one worker stays
                // unparked briefly. This is fine — criterion warmup
                // iterations absorb this.
            }
            std::time::Duration::from_nanos(total_latency_ns)
        });
    });
    group.finish();
}

criterion::criterion_group!(benches, bench_per_task_overhead, bench_park_wake_latency);
criterion::criterion_main!(benches);
