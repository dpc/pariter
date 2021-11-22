use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use dpc_pariter::IteratorExt;

#[inline]
fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

pub fn sample_vec(len: u64) -> Vec<u64> {
    (0..len).collect()
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let fib_size = 18;
    for sample_size in [10, 100, 1_000, 10_000] {
        let sample = sample_vec(sample_size);
        c.bench_function(
            &format!("map; size: {}; fib: {}", sample_size, fib_size),
            |b| {
                b.iter_batched(
                    || sample.clone(),
                    move |v| {
                        v.into_iter()
                            .map(move |i| black_box(fibonacci(black_box(fib_size)) + i))
                            .collect::<Vec<_>>()
                    },
                    BatchSize::LargeInput,
                )
            },
        );

        for threads in [1, 2, 4, 8] {
            c.bench_function(
                &format!(
                    "parallel_map; size: {}; fib: {}; threads: {}",
                    sample_size, fib_size, threads
                ),
                |b| {
                    b.iter_batched(
                        || sample.clone(),
                        move |v| {
                            v.into_iter()
                                .parallel_map(move |i| {
                                    black_box(fibonacci(black_box(fib_size)) + i)
                                })
                                .threads(threads)
                                .collect::<Vec<_>>()
                        },
                        BatchSize::LargeInput,
                    )
                },
            );
        }
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
