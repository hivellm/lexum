//! Simple performance benchmark to test criterion setup

use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn simple_benchmark(c: &mut Criterion) {
    c.bench_function("simple_add", |b| {
        b.iter(|| {
            let x = black_box(1);
            let y = black_box(2);
            black_box(x + y)
        })
    });
}

criterion_group!(benches, simple_benchmark);
criterion_main!(benches);
