//! Throughput benchmarks: tasks per second at various scales and granularities.
//!
//! Answers: "How many tasks can the pool process per second?"
//! Varies: task count (1K → 100K) and per-task CPU cost (trivial → 100 µs).

use criterion::{BenchmarkId, Criterion, black_box};
use pool_of_threads::ThreadPool;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

fn bench_throughput_by_task_count(c: &mut Criterion) {
    let workers = std::thread::available_parallelism().unwrap().get();
    let mut group = c.benchmark_group("throughput/task_count");

    for n in [1_000u64, 10_000, 100_000] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter_custom(|iters| {
                let mut elapsed = std::time::Duration::ZERO;
                for _ in 0..iters {
                    let pool = ThreadPool::new(workers);
                    let counter = Arc::new(AtomicU64::new(0));
                    let t0 = Instant::now();
                    for i in 0..n {
                        let c = counter.clone();
                        pool.spawn(move || {
                            c.fetch_add(i, Ordering::Relaxed);
                        });
                    }
                    drop(pool); // drains all tasks
                    elapsed += t0.elapsed();
                    // Correctness check: sum 0..(n-1)
                    assert_eq!(
                        counter.load(Ordering::Relaxed),
                        n * (n - 1) / 2,
                        "throughput counter mismatch at n={n}"
                    );
                }
                elapsed
            });
        });
    }
    group.finish();
}

fn bench_throughput_by_granularity(c: &mut Criterion) {
    let workers = std::thread::available_parallelism().unwrap().get();
    let n = 10_000u64;
    let mut group = c.benchmark_group("throughput/granularity");

    for work_iters in [0u64, 1_000, 10_000, 100_000] {
        let label = match work_iters {
            0 => "empty",
            1_000 => "1µs",
            10_000 => "10µs",
            100_000 => "100µs",
            _ => unreachable!(),
        };
        group.bench_with_input(BenchmarkId::new(label, n), &work_iters, |b, &work_iters| {
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
                    assert_eq!(
                        counter.load(Ordering::Relaxed),
                        n,
                        "granularity counter mismatch: {} != {n}",
                        counter.load(Ordering::Relaxed)
                    );
                }
                elapsed
            });
        });
    }
    group.finish();
}

criterion::criterion_group!(
    benches,
    bench_throughput_by_task_count,
    bench_throughput_by_granularity,
);
criterion::criterion_main!(benches);
