//! Work-stealing effectiveness benchmarks.
//!
//! Answers: "Does the pool distribute imbalanced work across workers?"
//!
//! These are the most revealing benchmarks for a work-stealing scheduler.
//! All tasks flow through the global injector — workers pull batches via
//! `steal_batch_and_pop`. The key question: when some tasks are much heavier
//! than others, does the pool finish faster than a single thread?
//!
//! NOTE: true worker-to-worker stealing (one worker stealing from another's
//! local queue) is only indirectly tested here, since all `spawn()` calls
//! go to the global injector. Workers populate their local queues via batch
//! steals from the injector.

use criterion::{BenchmarkId, Criterion, black_box};
use pool_of_threads::ThreadPool;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// 80% light tasks (~1 µs) + 20% heavy tasks (~1 ms).
/// A pool should distribute heavy tasks across workers. Compares pool vs
/// sequential execution. Speedup >1 means the pool is helping.
fn bench_imbalanced_workload(c: &mut Criterion) {
    let workers = std::thread::available_parallelism().unwrap().get();
    let n_light = 8_000u64;
    let n_heavy = 2_000u64;
    let total = n_light + n_heavy;
    let mut group = c.benchmark_group("work_stealing/imbalanced");

    // Pool execution
    group.bench_function("pool", |b| {
        b.iter_custom(|iters| {
            let mut elapsed = std::time::Duration::ZERO;
            for _ in 0..iters {
                let pool = ThreadPool::new(workers);
                let counter = Arc::new(AtomicU64::new(0));
                let t0 = Instant::now();

                // Submit heavy tasks first so they sit deeper in the injector,
                // then flood with light tasks. Workers pull batches from the
                // top — light tasks get processed immediately, heavy tasks
                // are distributed as workers pull deeper.
                for _ in 0..n_heavy {
                    let c = counter.clone();
                    pool.spawn(move || {
                        black_box(pool_of_threads::bench_cpu_work(1_000_000)); // ~1 ms
                        c.fetch_add(1, Ordering::Relaxed);
                    });
                }
                for _ in 0..n_light {
                    let c = counter.clone();
                    pool.spawn(move || {
                        black_box(pool_of_threads::bench_cpu_work(1_000)); // ~1 µs
                        c.fetch_add(1, Ordering::Relaxed);
                    });
                }
                drop(pool);
                elapsed += t0.elapsed();
                assert_eq!(counter.load(Ordering::Relaxed), total);
            }
            elapsed
        });
    });

    // Sequential baseline
    group.bench_function("sequential", |b| {
        b.iter_custom(|iters| {
            let mut elapsed = std::time::Duration::ZERO;
            for _ in 0..iters {
                let t0 = Instant::now();
                for _ in 0..n_heavy {
                    black_box(pool_of_threads::bench_cpu_work(1_000_000));
                }
                for _ in 0..n_light {
                    black_box(pool_of_threads::bench_cpu_work(1_000));
                }
                elapsed += t0.elapsed();
            }
            elapsed
        });
    });
    group.finish();
}

/// Saturate all workers with heavy tasks (~10 ms each), then submit light
/// tasks. Measures whether the injector-steal path keeps the pool making
/// progress while workers are occupied — the light tasks must eventually
/// complete, and they should finish well before all heavy tasks are done
/// (because workers pull batches that may include a light task).
fn bench_saturated_workers_then_load(c: &mut Criterion) {
    let workers = std::thread::available_parallelism().unwrap().get();
    let mut group = c.benchmark_group("work_stealing/saturated");

    for n_extra in [100u64, 1_000] {
        group.bench_with_input(
            BenchmarkId::new("extra_tasks", n_extra),
            &n_extra,
            |b, &n_extra| {
                b.iter_custom(|iters| {
                    let mut elapsed = std::time::Duration::ZERO;
                    for _ in 0..iters {
                        let pool = ThreadPool::new(workers);
                        let counter = Arc::new(AtomicU64::new(0));

                        // Saturate every worker with a heavy task.
                        for _ in 0..workers {
                            let c = counter.clone();
                            pool.spawn(move || {
                                black_box(pool_of_threads::bench_cpu_work(10_000_000)); // ~10 ms
                                c.fetch_add(1, Ordering::Relaxed);
                            });
                        }

                        // Small delay so workers grab the heavy tasks.
                        std::thread::sleep(std::time::Duration::from_millis(5));

                        let t0 = Instant::now();
                        // Submit light tasks — injector-steal path should
                        // pick them up while heavy tasks are still running.
                        for _ in 0..n_extra {
                            let c = counter.clone();
                            pool.spawn(move || {
                                black_box(pool_of_threads::bench_cpu_work(1_000));
                                c.fetch_add(1, Ordering::Relaxed);
                            });
                        }

                        // Wait for all tasks. Acquire ordering prevents
                        // the compiler from hoisting the load out of the
                        // spin loop (Relaxed + spin_loop is unsound —
                        // spin_loop is nomem, so the compiler may assume
                        // memory doesn't change across it).
                        let expected = workers as u64 + n_extra;
                        while counter.load(Ordering::Acquire) < expected {
                            std::hint::spin_loop();
                        }
                        elapsed += t0.elapsed();
                        drop(pool);
                    }
                    elapsed
                });
            },
        );
    }
    group.finish();
}

criterion::criterion_group!(benches, bench_imbalanced_workload, bench_saturated_workers_then_load,);
criterion::criterion_main!(benches);
