use criterion::{black_box, Criterion};

pub fn bench_throughput(_c: &mut Criterion) {
    // Will be populated during implementation
}

criterion::criterion_group!(benches, bench_throughput);
criterion::criterion_main!(benches);
