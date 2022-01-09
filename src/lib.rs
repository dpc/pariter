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

pub use crossbeam::{scope, thread::Scope};

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
    fn parallel_map<F, O>(self, f: F) -> ParallelMap<'static, 'static, Self, O, F>
    where
        Self: Sized,
        Self: Iterator,
        O: 'static,
        F: FnMut(Self::Item) -> O + Send + 'static;

    /// Scoped version of [`IteratorExt::parallel_map`]
    ///
    /// Use when you want to process in parallel items that contain
    /// borrowed references.
    ///
    /// See [`scope`].
    fn parallel_map_scoped<'env, 'scope, F, O>(
        self,
        scope: &'scope Scope<'env>,
        f: F,
    ) -> ParallelMap<'env, 'scope, Self, O, F>
    where
        Self: Sized,
        Self: Iterator + 'scope + 'env,
        F: FnMut(Self::Item) -> O + 'scope + 'env + Send;

    /// Run `filter` function in parallel on multiple threads
    ///
    /// A wrapper around [`IteratorExt::parallel_map`] really, so it has similiar properties.
    fn parallel_filter<F>(self, f: F) -> ParallelFilter<'static, 'static, Self>
    where
        Self: Sized,
        Self: Iterator + Send,
        F: FnMut(&Self::Item) -> bool + Send + 'static + Clone,
        Self::Item: Send + 'static;

    /// Scoped version of [`IteratorExt::parallel_filter`]
    ///
    /// Use when you want to process in parallel items that contain
    /// borrowed references.
    ///
    /// See [`scope`].
    fn parallel_filter_scoped<'env, 'scope, F>(
        self,
        scope: &'scope Scope<'env>,
        f: F,
    ) -> ParallelFilter<'env, 'scope, Self>
    where
        Self: Sized,
        Self: Iterator + Send + 'scope + 'env,
        F: FnMut(&Self::Item) -> bool + Send + 'env + 'scope + Clone,
        Self::Item: Send + 'env + 'scope;

    /// Run the current iterator in another thread and return elements
    /// through a buffered channel.
    ///
    /// `buffer_size` defines the size of the output channel connecting
    /// current and the inner thread.
    //
    /// It's a common mistake to use large channel sizes needlessly
    /// in hopes of achieving higher performance. The only benefit
    /// large buffer size value provides is smooting out the variance
    /// of the inner iterator returning items. The cost - wasting memory. In normal
    /// circumstances `0` is recommended.
    fn readahead(self, buffer_size: usize) -> Readahead<'static, 'static, Self>
    where
        Self: Iterator,
        Self: Sized,
        Self: Send + 'static,
        Self::Item: Send + 'static;

    /// Scoped version of [`IteratorExt::readahead`]
    ///
    /// Use when you want to process in parallel items that contain
    /// borrowed references.
    ///
    /// See [`scope`].
    fn readahead_scoped<'env, 'scope>(
        self,
        scope: &'scope Scope<'env>,
        buffer_size: usize,
    ) -> Readahead<'env, 'scope, Self>
    where
        Self: Sized + Send,
        Self: Iterator + 'scope + 'env,
        Self::Item: Send + 'env + 'scope + Send;
}

impl<I> IteratorExt for I
where
    I: Iterator,
{
    fn parallel_map<F, O>(self, f: F) -> ParallelMap<'static, 'static, Self, O, F>
    where
        Self: Sized,
        Self: Iterator,
        O: 'static,
        F: FnMut(I::Item) -> O + Send + 'static,
    {
        ParallelMap::new(self, f)
    }

    fn parallel_map_scoped<'env, 'scope, F, O>(
        self,
        scope: &'scope Scope<'env>,
        f: F,
    ) -> ParallelMap<'env, 'scope, Self, O, F>
    where
        Self: Sized,
        Self: Iterator + 'env + 'scope,
        F: FnMut(I::Item) -> O + 'env + 'scope + Send,
    {
        ParallelMap::new_scoped(self, scope, f)
    }

    fn parallel_filter<F>(self, f: F) -> ParallelFilter<'static, 'static, Self>
    where
        Self: Sized,
        Self: Iterator + Send,
        F: FnMut(&I::Item) -> bool + Send + 'static + Clone,
        <Self as Iterator>::Item: Send + 'static,
    {
        ParallelFilter::new(self, f)
    }

    fn parallel_filter_scoped<'env, 'scope, F>(
        self,
        scope: &'scope Scope<'env>,
        f: F,
    ) -> ParallelFilter<'env, 'scope, Self>
    where
        Self: Sized,
        Self: Iterator + Send + 'env + 'scope,
        F: FnMut(&I::Item) -> bool + Send + 'env + 'scope + Clone,
        <Self as Iterator>::Item: Send + 'env + 'scope,
    {
        ParallelFilter::new_scoped(self, scope, f)
    }

    fn readahead(self, buffer_size: usize) -> Readahead<'static, 'static, Self>
    where
        Self: Iterator,
        Self: Sized,
        Self: Send + 'static,
        <Self as Iterator>::Item: Send + 'static,
    {
        Readahead::new(self, buffer_size)
    }

    fn readahead_scoped<'env, 'scope>(
        self,
        scope: &'scope Scope<'env>,
        buffer_size: usize,
    ) -> Readahead<'env, 'scope, Self>
    where
        Self: Sized + Send,
        Self: Iterator + 'env + 'scope,
        <Self as Iterator>::Item: Send + 'env + 'scope,
    {
        Readahead::new_scoped(scope, self, buffer_size)
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
