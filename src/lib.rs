#![doc = include_str!("../README.md")]

use std::sync::{
    atomic::{AtomicBool, Ordering::SeqCst},
    Arc,
};

mod parallel_map;
pub use self::parallel_map::ParallelMap;

mod readahead;
pub use self::readahead::Readahead;

mod parallel_filter;
pub use self::parallel_filter::ParallelFilter;

use lazy_static::lazy_static;

type DefaultThreadPool = rusty_pool::ThreadPool;

lazy_static! {
    static ref DEFAULT_POOL: rusty_pool::ThreadPool = rusty_pool::ThreadPool::new(
        num_cpus::get(),
        rusty_pool::MAX_SIZE,
        std::time::Duration::from_secs(1)
    );
}

/// An interface for any thread pool to be used with parallel iterator
pub trait ThreadPool {
    /// A thread-handle
    ///
    /// ParallelMap and other iterators will hold on to it, until they drop
    type JoinHandle;

    /// Submit a function to the thread pool
    fn submit<F>(&mut self, f: F) -> Self::JoinHandle
    where
        F: FnOnce() + Send + 'static;
}

impl ThreadPool for rusty_pool::ThreadPool {
    type JoinHandle = rusty_pool::JoinHandle<()>;

    fn submit<F>(&mut self, f: F) -> Self::JoinHandle
    where
        F: FnOnce() + Send + 'static,
    {
        self.evaluate(f)
    }
}

/// Extension trait for [`std::iter::Iterator`] bringing parallel operations
///
/// # TODO
///
/// * `parallel_for_each`
/// * `parallel_flat_map`
/// * possibly others
///
/// PRs welcome
pub trait IteratorExt {
    /// Run `map` function in parallel on multiple threads
    ///
    /// Results will be returned in order.
    ///
    /// No worker threads will be started and no items will be pulled unless [`ParallelMap::started`]
    /// was called, or until first time [`ParallelMap`] is pulled for elements with [`ParallelMap::next`].
    /// In that respect, `ParallelMap` behaves like every other iterator and is lazy.
    ///
    /// Default built-in thread pool will be used unless [`ParallelMap::threads`] is used.
    fn parallel_map<F, O>(self, f: F) -> ParallelMap<Self, O, F, DefaultThreadPool>
    where
        Self: Sized,
        Self: Iterator,
        F: FnMut(Self::Item) -> O;

    /// Run `filter` function in parallel on multiple threads
    ///
    /// A wrapper around [`IteratorExt::parallel_map`] really, so it has similiar properties.
    fn parallel_filter<F>(self, f: F) -> ParallelFilter<Self, DefaultThreadPool>
    where
        Self: Sized,
        Self: Iterator + Send,
        F: FnMut(&Self::Item) -> bool + Send + 'static + Clone,
        Self::Item: Send + 'static;

    // Run the current iterator in another thread and return elements
    // through a buffered channel.
    //
    // `buffer_size` defines the size of the output channel connecting
    // current and the inner thread.
    //
    // It's a common mistake to use large channel sizes needlessly
    // in hopes of achieving higher performance. The only benefit
    // large buffer size value provides is smooting out the variance
    // of the inner iterator. The cost - wasting memory. In normal
    // circumstances `0` is recommended.
    fn readahead(self, buffer_size: usize) -> Readahead<Self, DefaultThreadPool>
    where
        Self: Iterator,
        Self: Sized,
        Self: Send + 'static,
        Self::Item: Send + 'static;
}

impl<I> IteratorExt for I
where
    I: Iterator,
{
    fn parallel_map<F, O>(self, f: F) -> ParallelMap<Self, O, F, DefaultThreadPool>
    where
        Self: Sized,
        Self: Iterator,
        F: FnMut(I::Item) -> O,
    {
        ParallelMap::new(self, DEFAULT_POOL.clone(), f)
    }

    fn readahead(self, buffer_size: usize) -> Readahead<Self, DefaultThreadPool>
    where
        Self: Iterator,
        Self: Sized,
        Self: Send + 'static,
        <Self as Iterator>::Item: Send + 'static,
    {
        Readahead::new(self, buffer_size, DEFAULT_POOL.clone())
    }

    fn parallel_filter<F>(self, f: F) -> ParallelFilter<Self, DefaultThreadPool>
    where
        Self: Sized,
        Self: Iterator + Send,
        F: FnMut(&I::Item) -> bool + Send + 'static + Clone,
        <Self as Iterator>::Item: Send + 'static,
    {
        ParallelFilter::new(self, DEFAULT_POOL.clone(), f)
    }
}

struct DropIndicator {
    canceled: bool,
    indicator: Arc<AtomicBool>,
}

impl DropIndicator {
    fn new(indicator: Arc<AtomicBool>) -> Self {
        Self {
            canceled: false,
            indicator,
        }
    }

    fn cancel(mut self) {
        self.canceled = true;
    }
}

impl Drop for DropIndicator {
    fn drop(&mut self) {
        if !self.canceled {
            self.indicator.store(true, SeqCst);
        }
    }
}

#[cfg(test)]
mod tests;
