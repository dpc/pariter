mod simple;

pub use simple::{TotalTimeProfiler, TotalTimeStats};

/// An interface to profile iterator consumption/prodution performance
///
/// In real applications utilizing pipelining it's important
/// to measure the production and consumption rates of each
/// pipeline step, to help identify current bottleneck.
///
/// [`ProfileEgress`] and [`ProfileIngress`] pinky-promise to
///  alway call `start` first, and then call corresponding `end`
///  before calling `start` again. The final `end` is not guaranteed
///  to be called (eg. if the inner iterator panicked), and in
///  particular the [`ProfileEgress`] will usually call the last
///  `start` with no-corresponding `end`, since it's impossible to
///  predict if the next iterator chain step will call `next()`
///  again to pull for the next item.
///
///  See [`TotalTimeProfiler`] for simple starting built-in implementation.
pub trait Profiler {
    fn start(&mut self);
    fn end(&mut self);
}

/// Profiles the time spent waiting for the downstream
/// iterator step to consume the previous returned item
/// and ask for the next one (or in other words, the time
/// spent NOT spent waiting on `next()` of the inner iterator.
pub struct ProfileEgress<I, P> {
    inner: I,
    profiler: P,
    first_returned: bool,
}

/// Profiles the time spent waiting for the upstream
/// iterator step to produce the next returned item
/// (or in other words, the time it took to call `next()`).
pub struct ProfileIngress<I, P> {
    inner: I,
    profiler: P,
}

impl<I, P> ProfileEgress<I, P> {
    pub fn new(inner: I, profiler: P) -> Self {
        Self {
            inner,
            profiler,
            first_returned: false,
        }
    }
}

impl<I, P> ProfileIngress<I, P> {
    pub fn new(inner: I, profiler: P) -> Self {
        Self { inner, profiler }
    }
}

impl<I, P> Iterator for ProfileEgress<I, P>
where
    I: Iterator,
    P: Profiler,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.first_returned {
            self.profiler.end();
        } else {
            // might as well switch it before actually pulling
            self.first_returned = true;
        }

        let item = self.inner.next();

        self.profiler.start();

        item
    }
}

impl<I, P> Iterator for ProfileIngress<I, P>
where
    I: Iterator,
    P: Profiler,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.profiler.start();

        let item = self.inner.next();

        self.profiler.end();
        item
    }
}
