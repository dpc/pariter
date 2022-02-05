use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
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

pub fn map_fibonacci(c: &mut Criterion) {
    for fib_size in [17, 18, 19] {
        for sample_size in [10, 100, 1_000, 10_000] {
            let sample = sample_vec(sample_size);

            let mut group = c.benchmark_group(format!(
                "map-fibonacci; vec-size: {}; fib({});",
                sample_size, fib_size
            ));
            group.throughput(criterion::Throughput::Elements(sample_size));
            group.bench_with_input(BenchmarkId::new("single-fib", 1), &1, |b, _i| {
                b.iter(|| black_box(fibonacci(black_box(fib_size))));
            });

            group.bench_with_input(BenchmarkId::new("map", 1), &1, |b, _i| {
                b.iter_batched(
                    || sample.clone(),
                    move |v| {
                        v.into_iter()
                            .map(move |i| black_box(fibonacci(black_box(fib_size)) + i))
                            .collect::<Vec<_>>()
                    },
                    BatchSize::SmallInput,
                )
            });

            for threads in [1, 2, 4, 8] {
                group.bench_with_input(
                    BenchmarkId::new("parallel_map", threads),
                    &threads,
                    |b, threads| {
                        b.iter_batched(
                            || sample.clone(),
                            move |v| {
                                v.into_iter()
                                    .parallel_map_custom()
                                    .threads(*threads)
                                    .with(move |i| black_box(fibonacci(black_box(fib_size)) + i))
                                    .collect::<Vec<_>>()
                            },
                            BatchSize::SmallInput,
                        )
                    },
                );
            }
        }
    }
}

criterion_group!(benches, map_fibonacci);
criterion_main!(benches);
