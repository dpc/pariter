use crate::ThreadPool;

use crate::ParallelMap;

trait FilterMapFn<Item>: FnMut(Item) -> Option<Item> + Send + 'static {
    fn clone_box<'a>(&self) -> Box<dyn 'a + FilterMapFn<Item>>
    where
        Self: 'a;
}

impl<F, Item> FilterMapFn<Item> for F
where
    F: FnMut(Item) -> Option<Item> + Clone + Send + 'static,
{
    fn clone_box<'a>(&self) -> Box<dyn 'a + FilterMapFn<Item>>
    where
        Self: 'a,
    {
        Box::new(self.clone())
    }
}

impl<'a, Item: 'a> Clone for Box<dyn 'a + FilterMapFn<Item>> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

/// Like [`std::iter::Filter`] but multi-threaded
pub struct ParallelFilter<I, TP>
where
    TP: ThreadPool,
    I: Iterator,
{
    // the iterator we wrapped
    iter: ParallelMap<I, Option<I::Item>, Box<dyn FilterMapFn<I::Item>>, TP>,
}

impl<I, TP> ParallelFilter<I, TP>
where
    TP: ThreadPool,
    I: Iterator,
{
    pub fn new<F>(iter: I, thread_pool: TP, mut f: F) -> ParallelFilter<I, TP>
    where
        F: FnMut(&I::Item) -> bool + Send + 'static + Clone,
        I::Item: Send + 'static,
    {
        ParallelFilter {
            iter: ParallelMap::new(
                iter,
                thread_pool,
                Box::new(move |item| if (f)(&item) { Some(item) } else { None })
                    as Box<dyn FilterMapFn<I::Item> + Send + 'static>,
            ),
        }
    }
    /// Start the background workers eagerly, without waiting for a first [`Iterator::next`] call.
    ///
    /// See [`ParallelMap::started`].
    pub fn started(self) -> Self
    where
        I::Item: Send + 'static,
    {
        Self {
            iter: self.iter.started(),
        }
    }

    /// Use a custom thread pool
    ///
    /// See [`ParallelMap::within`].
    pub fn within<TP2>(self, thread_pool: TP2) -> ParallelFilter<I, TP2>
    where
        TP2: ThreadPool,
    {
        ParallelFilter {
            iter: self.iter.within(thread_pool),
        }
    }

    /// Set number of threads to use manually.
    ///
    /// See [`ParallelMap::threads`].
    pub fn threads(self, num_threads: usize) -> Self {
        Self {
            iter: self.iter.threads(num_threads),
        }
    }

    /// Set max number of items in flight
    ///
    /// See [`ParallelMap::max_in_flight`].
    pub fn max_in_flight(self, max_in_flight: usize) -> Self {
        Self {
            iter: self.iter.max_in_flight(max_in_flight),
        }
    }
}

impl<I, TP> Iterator for ParallelFilter<I, TP>
where
    I: Iterator,
    I::Item: Send + 'static,
    TP: ThreadPool,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some(Some(item)) => return Some(item),
                Some(None) => continue,
                None => return None,
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
