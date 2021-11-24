# Parallel iterator processing library for Rust

I keep needing one, so I wrote it.

If you have:

```rust
# fn step_a(x: usize) -> usize {
#   x * 7
# }
# 
# fn filter_b(x: &usize) -> bool {
#   x % 2 == 0
# }
# 
# fn step_c(x: usize) -> usize {
#   x + 1
# }
assert_eq!(
  (0..10)
    .map(step_a)
    .filter(filter_b)
    .map(step_c).collect::<Vec<_>>(),
    vec![1, 15, 29, 43, 57]
);
```

You can change it to:

```rust
use dpc_pariter::IteratorExt;
# fn step_a(x: usize) -> usize {
#   x * 7
# }
# 
# fn filter_b(x: &usize) -> bool {
#   x % 2 == 0
# }
# 
# fn step_c(x: usize) -> usize {
#   x + 1
# }
assert_eq!(
  (0..10)
    .map(step_a)
    .filter(filter_b)
    .parallel_map(step_c).collect::<Vec<_>>(),
    vec![1, 15, 29, 43, 57]
);
```

and it will run faster (conditions apply), because
`step_c` will run in parallel on multiple-threads.

Or you can try even more features:

```rust
use dpc_pariter::IteratorExt;
# fn step_a(x: usize) -> usize {
#   x * 7
# }
# 
# fn filter_b(x: &usize) -> bool {
#   x % 2 == 0
# }
# 
# fn step_c(x: usize) -> usize {
#   x + 1
# }
assert_eq!(
  (0..10)
    .map(step_a)
    .readahead(0)
    .parallel_filter(filter_b)
    .parallel_map(step_c).collect::<Vec<_>>(),
    vec![1, 15, 29, 43, 57]
);
```



## Notable features

* order preserving
* lazy, somewhat like a normal iterators
* panic propagation
* backpressure control
* custom thread-pool support
* other knobs to control performance & resource utilization

## When to use and alternatives

This library is a good general purpose solution. When you have
a chain of iterator steps, and would like to process one or some
of them in parallel to speed it up, this should be a drop-in
replacement.

Sending iterator items through channels is fast, but not free.
Make sure to parallelize operations that are heavy enough to justify
overhead of sending data through channels. E.g.
operations involving IO or some CPU-heavy computation.

When you have a lot items **already stored in a collection**,
that you want to "roll over and perform some mass computation"
you probably want to use `rayon` instead. It's a library optimized
for parallelizing processing of whole chunks of larger set of data.
Because of that converting `rayon`'s iterators back to ordered
sequencial iterator is non-trivial.

There are alternative libraries somewhat like this, but I did not
find any that I'd like API and/or implementation wise, so I wrote
my own.

## Status & plans

I keep needing this exact functionality, so I've cleaned up my
ad-hoc code and put it in a proper library. I would
actually prefer if someone else did it. :D I might add
more features in the future, and I am happy to accept
PRs.

I'm open to share ownership & maintenance, or even "donate it".
