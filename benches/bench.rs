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
    for fib_size in [10, 11, 12] {
        let mut group = c.benchmark_group(format!("fibonacci({})", fib_size));

        for num_elements in [1, 100, 1_000, 10_000, 100_000] {
            let sample = sample_vec(num_elements);

            group.throughput(criterion::Throughput::Elements(num_elements));

            group.bench_with_input(
                BenchmarkId::new("map", num_elements),
                &num_elements,
                |b, _i| {
                    b.iter_batched(
                        || sample.clone(),
                        move |v| {
                            v.into_iter()
                                .map(move |i| black_box(fibonacci(black_box(fib_size)) + i))
                                .collect::<Vec<_>>()
                        },
                        BatchSize::SmallInput,
                    )
                },
            );

            for num_threads in [1, 2, 4] {
                group.bench_with_input(
                    BenchmarkId::new(format!("parallel_map-t{}", num_threads), num_elements),
                    &num_elements,
                    |b, _sample_size| {
                        b.iter_batched(
                            || sample.clone(),
                            move |v| {
                                v.into_iter()
                                    .parallel_map_custom(
                                        |o| o.threads(num_threads),
                                        move |i| black_box(fibonacci(black_box(fib_size)) + i),
                                    )
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
