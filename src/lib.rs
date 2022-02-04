#![doc = include_str!("../README.md")]
use std::sync::{
    atomic::{AtomicBool, Ordering::SeqCst},
    Arc,
};

mod parallel_map;
pub use self::parallel_map::{ParallelMap, ParallelMapBuilder};

mod readahead;
pub use self::readahead::{Readahead, ReadaheadBuilder};

mod parallel_filter;
pub use self::parallel_filter::{ParallelFilter, ParallelFilterBuilder};

pub mod profile;
pub use self::profile::{
    ProfileEgress, ProfileIngress, Profiler, TotalTimeProfiler, TotalTimeStats,
};

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
    /// No items will be pulled until first time [`ParallelMap`] is pulled for elements with [`ParallelMap::next`].
    /// In that respect, `ParallelMap` behaves like every other iterator and is lazy.
    fn parallel_map_custom(self) -> ParallelMapBuilder<Self>
    where
        Self: Sized,
        Self: Iterator;

    /// See [`IteratorExt::parallel_map_custom`]
    fn parallel_map<F, O>(self, f: F) -> ParallelMap<Self, O>
    where
        Self: Sized,
        Self: Iterator + 'static,
        F: 'static + Send + Clone,
        Self::Item: Send + 'static,
        F: FnMut(Self::Item) -> O,
        O: Send + 'static,
    {
        self.parallel_map_custom().with(f)
    }

    /// See [`IteratorExt::parallel_map_custom`]
    fn parallel_map_scoped<'env, 'scope, F, O>(
        self,
        scope: &'scope Scope<'env>,
        f: F,
    ) -> ParallelMap<Self, O>
    where
        Self: Sized,
        Self: Iterator + 'env,
        F: 'env + Send + Clone,
        Self::Item: Send + 'env,
        F: FnMut(Self::Item) -> O,
        O: Send + 'env,
    {
        self.parallel_map_custom().with_scoped(scope, f)
    }

    /// Run `filter` function in parallel on multiple threads
    ///
    /// A wrapper around [`IteratorExt::parallel_map`] really, so it has similiar properties.
    fn parallel_filter_custom(self) -> ParallelFilterBuilder<Self>
    where
        Self: Sized,
        Self: Iterator;

    /// See [`IteratorExt::parallel_filter_custom`]
    fn parallel_filter<F>(self, f: F) -> ParallelFilter<Self>
    where
        Self: Sized,
        Self: Iterator + 'static,
        F: 'static + Send + Clone,
        Self::Item: Send + 'static,
        F: FnMut(&Self::Item) -> bool,
    {
        self.parallel_filter_custom().with(f)
    }

    /// See [`IteratorExt::parallel_filter_custom`]
    fn parallel_filter_scoped<'env, 'scope, F>(
        self,
        scope: &'scope Scope<'env>,
        f: F,
    ) -> ParallelFilter<Self>
    where
        Self: Sized,
        Self: Iterator + 'env,
        F: 'env + Send + Clone,
        Self::Item: Send + 'env,
        F: FnMut(&Self::Item) -> bool,
    {
        self.parallel_filter_custom().with_scoped(scope, f)
    }

    /// Run the current iterator in another thread and return elements
    /// through a buffered channel.
    ///
    /// `buffer_size` defines the size of the output channel connecting
    /// current and the inner thread.
    //
    /// It's a common mistake to use large channel sizes needlessly
    /// in hopes of achieving higher performance. The only benefit
    /// large buffer size value provides is smooting out the variance
    /// of the inner iterator returning items. The cost - wasting memory.
    /// In normal circumstances `0` is recommended (the default).
    fn readahead(self) -> Readahead<Self>
    where
        Self: Iterator + Send + 'static,
        Self: Sized,
        Self::Item: Send + 'static;

    /// Scoped version of [`IteratorExt::readahead`]
    ///
    /// Use when you want to process in parallel items that contain
    /// borrowed references.
    ///
    /// See [`scope`].
    fn readahead_scoped<'env, 'scope>(self, scope: &'scope Scope<'env>) -> Readahead<Self>
    where
        Self: Sized + Send,
        Self: Iterator + 'scope + 'env,
        Self::Item: Send + 'env + 'scope + Send;

    fn readahead_custom(self) -> ReadaheadBuilder<Self>
    where
        Self: Iterator,
        Self: Sized + Send,
        Self::Item: Send;

    /// Profile the time it takes downstream iterator step to consume the returned items.
    ///
    /// See [`ProfileEgress`] and [`profile::Profiler`].
    fn profile_egress<P: profile::Profiler>(self, profiler: P) -> ProfileEgress<Self, P>
    where
        Self: Iterator,
        Self: Sized;

    /// Profile the time it takes upstream iterator step to produce the returned items.
    ///
    /// See [`ProfileIngress`] and [`profile::Profiler`].
    fn profile_ingress<P: profile::Profiler>(self, profiler: P) -> ProfileIngress<Self, P>
    where
        Self: Iterator,
        Self: Sized;

    /// Profiled version of [`IteratorExt::readahead`]
    ///
    /// Literally `.profile_egress(tx_profiler).readahead(n).profile_ingress(rx_profiler)`
    ///
    /// See [`Profiler`] for more info.
    fn readahead_profiled<TxP: profile::Profiler, RxP: profile::Profiler>(
        self,
        tx_profiler: TxP,
        rx_profiler: RxP,
    ) -> ProfileIngress<Readahead<ProfileEgress<Self, TxP>>, RxP>
    where
        Self: Iterator,
        Self: Sized,
        Self: Send + 'static,
        Self::Item: Send + 'static,
        TxP: Send + 'static;

    /// Profiled version of [`IteratorExt::readahead_scoped`]
    ///
    /// Literally `.profile_egress(tx_profiler).readahead_scoped(scope, n).profile_ingress(rx_profiler)`
    ///
    /// See [`Profiler`] for more info.
    fn readahead_scoped_profiled<'env, 'scope, TxP: profile::Profiler, RxP: profile::Profiler>(
        self,
        scope: &'scope Scope<'env>,
        tx_profiler: TxP,
        rx_profiler: RxP,
    ) -> ProfileIngress<Readahead<ProfileEgress<Self, TxP>>, RxP>
    where
        Self: Sized + Send,
        Self: Iterator + 'scope + 'env,
        Self::Item: Send + 'env + 'scope + Send,
        TxP: Send + 'static;
}

impl<I> IteratorExt for I
where
    I: Iterator,
{
    fn parallel_map_custom(self) -> ParallelMapBuilder<Self>
    where
        Self: Sized,
        Self: Iterator,
    {
        ParallelMapBuilder::new(self)
    }

    fn parallel_filter_custom(self) -> ParallelFilterBuilder<Self>
    where
        Self: Sized,
        Self: Iterator,
    {
        ParallelFilterBuilder::new(self)
    }

    fn readahead(self) -> Readahead<Self>
    where
        Self: Iterator + Send + 'static,
        Self: Sized,
        <Self as Iterator>::Item: Send + 'static,
    {
        ReadaheadBuilder::new(self).with()
    }

    fn readahead_scoped<'env, 'scope>(self, scope: &'scope Scope<'env>) -> Readahead<Self>
    where
        Self: Sized + Send,
        Self: Iterator + 'env + 'scope,
        <Self as Iterator>::Item: Send + 'env + 'scope,
    {
        ReadaheadBuilder::new(self).with_scoped(scope)
    }

    fn readahead_custom(self) -> ReadaheadBuilder<Self>
    where
        Self: Iterator + Send,
        Self: Sized,
        <Self as Iterator>::Item: Send,
    {
        ReadaheadBuilder::new(self)
    }

    fn profile_egress<P: profile::Profiler>(self, profiler: P) -> ProfileEgress<Self, P>
    where
        Self: Iterator,
        Self: Sized,
    {
        ProfileEgress::new(self, profiler)
    }

    fn profile_ingress<P: profile::Profiler>(self, profiler: P) -> ProfileIngress<Self, P>
    where
        Self: Iterator,
        Self: Sized,
    {
        ProfileIngress::new(self, profiler)
    }

    fn readahead_profiled<TxP: profile::Profiler, RxP: profile::Profiler>(
        self,
        tx_profiler: TxP,
        rx_profiler: RxP,
    ) -> ProfileIngress<Readahead<ProfileEgress<Self, TxP>>, RxP>
    where
        Self: Iterator,
        Self: Sized,
        Self: Send + 'static,
        <Self as Iterator>::Item: Send + 'static,
        TxP: Send + 'static,
    {
        self.profile_egress(tx_profiler)
            .readahead()
            .profile_ingress(rx_profiler)
    }

    fn readahead_scoped_profiled<'env, 'scope, TxP: profile::Profiler, RxP: profile::Profiler>(
        self,
        scope: &'scope Scope<'env>,
        tx_profiler: TxP,
        rx_profiler: RxP,
    ) -> ProfileIngress<Readahead<ProfileEgress<Self, TxP>>, RxP>
    where
        Self: Iterator,
        Self: Sized + Send,
        Self: Iterator + 'scope + 'env,
        <Self as Iterator>::Item: Send + 'env + 'scope + Send,

        TxP: Send + 'static,
    {
        self.profile_egress(tx_profiler)
            .readahead_scoped(scope)
            .profile_ingress(rx_profiler)
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
