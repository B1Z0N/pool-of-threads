//! Scalability benchmarks: how does throughput change as workers increase?
//!
//! Answers: "Does adding threads help, and by how much?"
//! Fixed workload, vary workers from 1 to available_parallelism.
//! Expect near-linear speedup for CPU-bound work; sub-linear reveals
//! contention in the injector, parking mutex, or crossbeam internals.

use criterion::{BenchmarkId, Criterion, black_box};
use pool_of_threads::ThreadPool;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Fixed workload (10K tasks × 10µs each), vary workers.
/// Reports throughput in estimated tasks/second so the curve is easy to read.
fn bench_worker_scaling(c: &mut Criterion) {
    let max_workers = std::thread::available_parallelism().unwrap().get();
    let n = 10_000u64;
    let work_iters = 10_000u64; // ~10 µs per task on modern hardware
    let mut group = c.benchmark_group("scalability/workers");

    for workers in [1, 2, 4, max_workers].iter().copied() {
        // Skip impossible configs.
        if workers > max_workers {
            continue;
        }
        group.bench_with_input(
            BenchmarkId::new("tasks_per_sec", workers),
            &workers,
            |b, &workers| {
                b.iter_custom(|iters| {
                    let mut elapsed = std::time::Duration::ZERO;
                    for _ in 0..iters {
                        let pool = ThreadPool::new(workers);
                        let counter = Arc::new(AtomicU64::new(0));
                        let t0 = Instant::now();
                        for _ in 0..n {
                            let c = counter.clone();
                            pool.spawn(move || {
                                black_box(pool_of_threads::bench_cpu_work(work_iters));
                                c.fetch_add(1, Ordering::Relaxed);
                            });
                        }
                        drop(pool);
                        elapsed += t0.elapsed();
                        assert_eq!(counter.load(Ordering::Relaxed), n);
                    }
                    elapsed
                });
            },
        );
    }
    group.finish();
}

/// Fixed 8 workers, vary submitters from 1 to 16.
/// Each submitter pushes tasks in a tight loop. Measures whether the
/// injector and parking mutex become bottlenecks under concurrent submission.
fn bench_submitter_scaling(c: &mut Criterion) {
    let workers = std::thread::available_parallelism().unwrap().get();
    let tasks_per_submitter = 2_500u64;
    let mut group = c.benchmark_group("scalability/submitters");

    for n_submitters in [1u64, 2, 4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::new("tasks_per_sec", n_submitters),
            &n_submitters,
            |b, &n_submitters| {
                b.iter_custom(|iters| {
                    let mut elapsed = std::time::Duration::ZERO;
                    for _ in 0..iters {
                        let pool = Arc::new(ThreadPool::new(workers));
                        let counter = Arc::new(AtomicU64::new(0));
                        let t0 = Instant::now();

                        let mut handles = vec![];
                        for _ in 0..n_submitters {
                            let p = pool.clone();
                            let c = counter.clone();
                            handles.push(std::thread::spawn(move || {
                                for _ in 0..tasks_per_submitter {
                                    let c = c.clone();
                                    p.spawn(move || {
                                        c.fetch_add(1, Ordering::Relaxed);
                                    });
                                }
                            }));
                        }
                        for h in handles {
                            h.join().unwrap();
                        }
                        drop(pool);
                        elapsed += t0.elapsed();
                        assert_eq!(
                            counter.load(Ordering::Relaxed),
                            n_submitters * tasks_per_submitter
                        );
                    }
                    elapsed
                });
            },
        );
    }
    group.finish();
}

criterion::criterion_group!(benches, bench_worker_scaling, bench_submitter_scaling);
criterion::criterion_main!(benches);
