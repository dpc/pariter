use std::time;

/// A stats handle passed to handlers by [`TotalTimeProfiler`].
#[derive(Debug)]
pub struct TotalTimeStats {
    current: time::Duration,
    total: time::Duration,
}

/// Something that can react to [`TotalTimeProfilerStats`] tracked by [`TotalTimeProfiler`].
pub trait Reporter {
    fn handle_stats(&mut self, stats: &mut TotalTimeStats);
}

impl<F> Reporter for F
where
    F: for<'a> Fn(&'a mut TotalTimeStats),
{
    fn handle_stats(&mut self, stats: &mut TotalTimeStats) {
        (self as &mut F)(stats);
    }
}

/// A simple basic profiler implementation which tracks
/// the accumulative time and calls a handler function
/// with it.
///
/// ## Example
///
/// ```rust
/// use pariter::{IteratorExt, TotalTimeProfiler};
///
/// std::thread::scope(|scope| {
///     (0..22)
///         .readahead_scoped_profiled(
///             scope,
///             TotalTimeProfiler::periodically_millis(10_000, || eprintln!("Blocked on sending")),
///             TotalTimeProfiler::periodically_millis(10_000, || eprintln!("Blocked on receving")),
///         )
///         .for_each(|i| {
///             println!("{i}");
///         })
/// });
/// ```
#[derive(Debug)]
pub struct TotalTimeProfiler<Reporter> {
    reporter: Reporter,
    start: time::Instant,
    stats: TotalTimeStats,
}

impl<F> TotalTimeProfiler<F>
where
    F: for<'a> Fn(&'a mut TotalTimeStats),
{
    /// Create a [`TotalTimeProfiler`] with any handle
    ///
    /// ## Example
    ///
    /// ```rust
    /// use pariter::{IteratorExt, TotalTimeProfiler};
    ///
    /// let profiler = TotalTimeProfiler::new(|stats| eprintln!("accumulative sending time so far: {}", stats.total().as_millis()));
    /// ```
    pub fn new(f: F) -> Self {
        Self {
            stats: TotalTimeStats {
                current: time::Duration::default(),
                total: time::Duration::default(),
            },

            start: time::Instant::now(),
            reporter: f,
        }
    }
}

impl<F> TotalTimeProfiler<PeriodicReporter<F>>
where
    F: Fn(),
{
    pub fn periodically_millis(millis: u64, f: F) -> Self {
        Self::periodically(time::Duration::from_millis(millis), f)
    }

    pub fn periodically(period: time::Duration, f: F) -> Self {
        Self {
            stats: TotalTimeStats {
                current: time::Duration::default(),
                total: time::Duration::default(),
            },

            start: time::Instant::now(),
            reporter: PeriodicReporter::new_millis(period, f),
        }
    }
}

/// Reporter calling a function every time the total accumulated time
/// being tracked crosses certain threshold
///
/// Use [`TotalTimeProfiler::periodically_millis`] instead
pub struct PeriodicReporter<F> {
    threshold: time::Duration,
    f: F,
}

impl<F> PeriodicReporter<F>
where
    F: Fn(),
{
    fn new_millis(threshold: time::Duration, f: F) -> Self {
        Self { threshold, f }
    }
}

impl<F> Reporter for PeriodicReporter<F>
where
    F: Fn(),
{
    fn handle_stats(&mut self, stats: &mut TotalTimeStats) {
        stats.periodically(self.threshold, || (self.f)());
    }
}

impl TotalTimeStats {
    fn periodically(&mut self, period: time::Duration, f: impl FnOnce()) {
        if self.total >= period {
            self.total -= period;
            (f)();
        }
    }

    /// Get total accumulated time
    pub fn total(&self) -> time::Duration {
        self.total
    }

    /// Get mutable reference to total accumulated time
    ///
    /// Your free to adjust it.
    pub fn total_mut(&mut self) -> &mut time::Duration {
        &mut self.total
    }
}
impl<Reporter> crate::Profiler for TotalTimeProfiler<Reporter>
where
    Reporter: self::Reporter,
{
    fn start(&mut self) {
        self.start = time::Instant::now();
    }

    fn end(&mut self) {
        self.stats.current = time::Instant::now()
            .duration_since(self.start)
            // Even with absolutely no delay waiting for
            // the other side of the channel a send/recv will take some time.
            // Substract some tiny value to account for it, to prevent
            // rare but spurious and confusing messages.
            .saturating_sub(time::Duration::from_micros(1));

        self.stats.total = self.stats.total.saturating_add(self.stats.current);

        let Self {
            ref mut reporter,
            ref mut stats,
            start: _,
        } = *self;

        reporter.handle_stats(stats);
    }
}
