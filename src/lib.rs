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

use std::thread::Scope;

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
    fn parallel_map<F, O>(self, f: F) -> ParallelMap<Self, O>
    where
        Self: Sized,
        Self: Iterator,
        F: 'static + Send + Clone,
        Self::Item: Send + 'static,
        F: FnMut(Self::Item) -> O,
        O: Send + 'static,
    {
        ParallelMapBuilder::new(self).with(f)
    }

    /// See [`IteratorExt::parallel_map`]
    fn parallel_map_custom<F, O, OF>(self, of: OF, f: F) -> ParallelMap<Self, O>
    where
        Self: Sized,
        Self: Iterator,
        F: 'static + Send + Clone,
        F: FnMut(Self::Item) -> O,
        Self::Item: Send + 'static,
        O: Send + 'static,
        OF: FnOnce(ParallelMapBuilder<Self>) -> ParallelMapBuilder<Self>,
    {
        of(ParallelMapBuilder::new(self)).with(f)
    }

    /// A version of [`parallel_map`] supporting iterating over
    /// borrowed values.
    ///
    /// See [`IteratorExt::parallel_map`]
    fn parallel_map_scoped<'env, 'scope, F, O>(
        self,
        scope: &'scope Scope<'scope, 'env>,
        f: F,
    ) -> ParallelMap<Self, O>
    where
        Self: Sized,
        Self: Iterator,
        F: 'env + Send + Clone,
        Self::Item: Send + 'env,
        F: FnMut(Self::Item) -> O,
        O: Send + 'env,
    {
        ParallelMapBuilder::new(self).with_scoped(scope, f)
    }

    /// See [`IteratorExt::parallel_map_scoped`]
    fn parallel_map_scoped_custom<'env, 'scope, F, O, OF>(
        self,
        scope: &'scope Scope<'scope, 'env>,
        of: OF,
        f: F,
    ) -> ParallelMap<Self, O>
    where
        Self: Sized,
        Self: Iterator,
        F: 'env + Send + Clone,
        Self::Item: Send + 'env,
        F: FnMut(Self::Item) -> O,
        O: Send + 'env,
        OF: FnOnce(ParallelMapBuilder<Self>) -> ParallelMapBuilder<Self>,
    {
        of(ParallelMapBuilder::new(self)).with_scoped(scope, f)
    }

    /// Run `filter` function in parallel on multiple threads
    ///
    /// A wrapper around [`IteratorExt::parallel_map`] really, so it has similiar properties.
    fn parallel_filter<F>(self, f: F) -> ParallelFilter<Self>
    where
        Self: Sized,
        Self: Iterator,
        F: 'static + Send + Clone,
        Self::Item: Send + 'static,
        F: FnMut(&Self::Item) -> bool,
    {
        ParallelFilterBuilder::new(self).with(f)
    }

    /// See [`IteratorExt::parallel_filter`]
    fn parallel_filter_custom<F, OF>(self, of: OF, f: F) -> ParallelFilter<Self>
    where
        Self: Sized,
        Self: Iterator,
        F: 'static + Send + Clone,
        Self::Item: Send + 'static,
        F: FnMut(&Self::Item) -> bool,
        OF: FnOnce(ParallelFilterBuilder<Self>) -> ParallelFilterBuilder<Self>,
    {
        of(ParallelFilterBuilder::new(self)).with(f)
    }

    /// See [`IteratorExt::parallel_filter`]
    fn parallel_filter_scoped<'env, 'scope, F>(
        self,
        scope: &'scope Scope<'scope, 'env>,
        f: F,
    ) -> ParallelFilter<Self>
    where
        Self: Sized,
        Self: Iterator,
        F: 'env + Send + Clone,
        Self::Item: Send + 'env,
        F: FnMut(&Self::Item) -> bool,
    {
        ParallelFilterBuilder::new(self).with_scoped(scope, f)
    }

    /// See [`IteratorExt::parallel_filter`]
    fn parallel_filter_scoped_custom<'env, 'scope, F, OF>(
        self,
        scope: &'scope Scope<'scope, 'env>,
        of: OF,
        f: F,
    ) -> ParallelFilter<Self>
    where
        Self: Sized,
        Self: Iterator,
        F: 'env + Send + Clone,
        Self::Item: Send + 'env,
        F: FnMut(&Self::Item) -> bool,
        OF: FnOnce(ParallelFilterBuilder<Self>) -> ParallelFilterBuilder<Self>,
    {
        of(ParallelFilterBuilder::new(self)).with_scoped(scope, f)
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
        Self::Item: Send + 'static,
    {
        ReadaheadBuilder::new(self).with()
    }

    fn readahead_custom<OF>(self, of: OF) -> Readahead<Self>
    where
        Self: Iterator,
        Self: Sized + Send + 'static,
        Self::Item: Send + 'static,
        OF: FnOnce(ReadaheadBuilder<Self>) -> ReadaheadBuilder<Self>,
    {
        of(ReadaheadBuilder::new(self)).with()
    }

    /// Scoped version of [`IteratorExt::readahead`]
    ///
    /// Use when you want to process in parallel items that contain
    /// borrowed references.
    ///
    /// See [`scope`].
    fn readahead_scoped<'env, 'scope>(self, scope: &'scope Scope<'scope, 'env>) -> Readahead<Self>
    where
        Self: Sized + Send,
        Self: Iterator + 'scope + 'env,
        Self::Item: Send + 'env + 'scope + Send,
    {
        ReadaheadBuilder::new(self).with_scoped(scope)
    }

    fn readahead_scoped_custom<'env, 'scope, OF>(
        self,
        scope: &'scope Scope<'scope, 'env>,
        of: OF,
    ) -> Readahead<Self>
    where
        Self: Sized + Send,
        Self: Iterator + 'scope + 'env,
        Self::Item: Send + 'env + 'scope + Send,
        OF: FnOnce(ReadaheadBuilder<Self>) -> ReadaheadBuilder<Self>,
    {
        of(ReadaheadBuilder::new(self)).with_scoped(scope)
    }

    /// Profile the time it takes downstream iterator step to consume the returned items.
    ///
    /// See [`ProfileEgress`] and [`profile::Profiler`].
    fn profile_egress<P: profile::Profiler>(self, profiler: P) -> ProfileEgress<Self, P>
    where
        Self: Iterator,
        Self: Sized,
    {
        ProfileEgress::new(self, profiler)
    }

    /// Profile the time it takes upstream iterator step to produce the returned items.
    ///
    /// See [`ProfileIngress`] and [`profile::Profiler`].
    fn profile_ingress<P: profile::Profiler>(self, profiler: P) -> ProfileIngress<Self, P>
    where
        Self: Iterator,
        Self: Sized,
    {
        ProfileIngress::new(self, profiler)
    }

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
        TxP: Send + 'static,
    {
        self.profile_egress(tx_profiler)
            .readahead()
            .profile_ingress(rx_profiler)
    }

    /// Profiled version of [`IteratorExt::readahead_scoped`]
    ///
    /// Literally `.profile_egress(tx_profiler).readahead_scoped(scope, n).profile_ingress(rx_profiler)`
    ///
    /// See [`Profiler`] for more info.
    fn readahead_scoped_profiled<'env, 'scope, TxP: profile::Profiler, RxP: profile::Profiler>(
        self,
        scope: &'scope Scope<'scope, 'env>,
        tx_profiler: TxP,
        rx_profiler: RxP,
    ) -> ProfileIngress<Readahead<ProfileEgress<Self, TxP>>, RxP>
    where
        Self: Sized + Send,
        Self: Iterator + 'scope + 'env,
        Self::Item: Send + 'env + 'scope + Send,
        TxP: Send + 'static,
    {
        self.profile_egress(tx_profiler)
            .readahead_scoped(scope)
            .profile_ingress(rx_profiler)
    }
}

impl<I> IteratorExt for I where I: Iterator {}

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
