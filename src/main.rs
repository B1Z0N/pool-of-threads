use pool_of_threads::ThreadPool;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

fn main() {
    let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);

    let task_count = 2_000;
    let work_per_task = 500;

    println!("pool-of-threads — work-stealing thread pool\n");
    println!("  Running {task_count} tasks ({} inner iterations each)\n", work_per_task,);

    for workers in [1, 2, 4, cpus] {
        let pool = ThreadPool::new(workers);
        let sum = Arc::new(AtomicU64::new(0));

        let start = Instant::now();
        for i in 0..task_count {
            let s = sum.clone();
            pool.spawn(move || {
                let mut x = 0u64;
                for j in 0..work_per_task {
                    x = x.wrapping_add((j * j) as u64);
                }
                s.fetch_add(i as u64, Ordering::Relaxed);
            });
        }
        drop(pool);
        let elapsed = start.elapsed();

        let expected: u64 = (0..task_count as u64).sum();
        let actual = sum.load(Ordering::Relaxed);
        let ok = if actual == expected { "✓" } else { "✗" };

        println!("  {workers:>2} workers  |  {elapsed:.2?}  |  sum={actual}  {ok}",);
    }
}
