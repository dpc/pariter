# Parallel iterator processing library for Rust

See [`IteratorExt`](https://docs.rs/dpc-pariter/latest/dpc_pariter/trait.IteratorExt.html)
for supported operations.

## Notable features

* drop-in replacement for standard iterators(*)
  * preserves order
  * lazy, somewhat like single-threaded iterators
  * panic propagation
* support for iterating over borrowed values using scoped threads
* backpressure control

## When to use and alternatives

This library is a good general purpose solution to adding multi-threaded
processing to an existing iterator-based code. When you have
a chain of iterator steps, and would like to process one or some
of them in parallel to speed things up, this library goes a long way
to make it as close to a drop-in replacement as possible in all aspects.

The implementation is based on spawning thread-pools of worker threads
and sending them work using channels, then receiving and sorting the
results to turn them into a normal iterator again.

Sending iterator items through channels is fast, but not free.
Make sure to parallelize operations that are heavy enough to justify
overhead of sending data through channels. E.g.
operations involving IO or some CPU-heavy computation.

When you have a lot items **already stored in a collection**,
that you want to "roll over and perform some mass computation"
you probably want to use `rayon` instead. It's a library optimized
for parallelizing processing of whole chunks of larger set of data.
Because of that [converting `rayon`'s iterators back to ordered
sequencial iterator is non-trivial](https://github.com/rayon-rs/rayon/issues/210).

There are alternative libraries somewhat like this, but I did not
find any that I'd like API and/or implementation wise, so I wrote
my own.

## Usage

 Adding new ones based
on the existing code should be relatively easy, so PRs are welcome.

In short, if you have:

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

## Iterating over borrowed values

Hitting a `borrowed value does not live long enough` error? Looks like you
are iterating over values containing borrowed references. Sending them over
to different threads for processing could lead to memory unsafety issues.
But no problem, we got you covered.

First, if the values you are iterating over can be cheaply cloned, just try
adding `.cloned()` and turning them into owned values.

If you can't, you can use scoped-threads API from [`crossbeam`] crate:


```rust
use dpc_pariter::{IteratorExt, scope};
# fn step_a(x: &usize) -> usize {
#   *x * 7
# }
#
# fn filter_b(x: &usize) -> bool {
#   x % 2 == 0
# }
#
# fn step_c(x: usize) -> usize {
#   x + 1
# }
let v : Vec<_> = (0..10).collect();

scope(|scope| {
  assert_eq!(
    v
      .iter() // iterating over `&usize` now, `parallel_map` will not work
      .parallel_map_scoped(scope, step_a)
      .filter(filter_b)
      .map(step_c).collect::<Vec<_>>(),
      vec![1, 15, 29, 43, 57]
  );
});

// or:

assert_eq!(
  scope(|scope| {
  v
    .iter()
    .parallel_map_scoped(scope, step_a)
    .filter(filter_b)
    .map(step_c).collect::<Vec<_>>()}).expect("handle errors properly in production code"),
    vec![1, 15, 29, 43, 57]
);
```

The additional `scope` argument comes from [`crossbeam::thread::scope`] and is
there to enforce memory-safety. Just wrap your iterator chain in a `scope`
wrapper that does not outlive the borrowed value, and everything will work smoothly.


## Status & plans

I keep needing this exact functionality, so I've cleaned up my
ad-hoc code, put it in a proper library. I'm usually very busy,
so if you want something added, please submit a PR.

I'm open to share/transfer ownership & maintenance into reputable hands.
