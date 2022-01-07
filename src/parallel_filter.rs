use crate::{ParallelMap, Scope};

trait FilterMapFn<Item>: FnMut(Item) -> Option<Item> + Send {
    fn clone_box<'a>(&self) -> Box<dyn 'a + FilterMapFn<Item> + Send>
    where
        Self: 'a;
}

impl<F, Item> FilterMapFn<Item> for F
where
    F: FnMut(Item) -> Option<Item> + Clone + Send,
{
    fn clone_box<'a>(&self) -> Box<dyn 'a + FilterMapFn<Item> + Send>
    where
        Self: 'a,
    {
        Box::new(self.clone())
    }
}

impl<'a, Item: 'a> Clone for Box<dyn 'a + FilterMapFn<Item> + Send> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

/// Like [`std::iter::Filter`] but multi-threaded
pub struct ParallelFilter<'env, 'scope, I>
where
    I: Iterator,
{
    // the iterator we wrapped
    iter:
        ParallelMap<'env, 'scope, I, Option<I::Item>, Box<dyn FilterMapFn<I::Item> + Send + 'env>>,
}

impl<I> ParallelFilter<'static, 'static, I>
where
    I: Iterator,
{
    pub fn new<F>(iter: I, mut f: F) -> ParallelFilter<'static, 'static, I>
    where
        F: FnMut(&I::Item) -> bool + Send + 'static + Clone,
        I::Item: Send + 'static,
    {
        ParallelFilter {
            iter: ParallelMap::new(
                iter,
                Box::new(move |item| if (f)(&item) { Some(item) } else { None })
                    as Box<dyn FilterMapFn<I::Item> + Send + 'static>,
            ),
        }
    }
}

impl<'env, 'scope, I> ParallelFilter<'env, 'scope, I>
where
    I: Iterator + 'env + 'scope,
    I::Item: Send + 'env + 'scope,
    'env: 'scope,
{
    pub fn new_scoped<F>(
        iter: I,
        scope: &'scope Scope<'env>,
        mut f: F,
    ) -> ParallelFilter<'env, 'scope, I>
    where
        F: FnMut(&I::Item) -> bool + Send + 'env + 'scope + Clone,
    {
        ParallelFilter {
            iter: ParallelMap::new_scoped(
                iter,
                scope,
                Box::new(move |item| if (f)(&item) { Some(item) } else { None })
                    as Box<dyn FilterMapFn<I::Item> + Send + 'env>,
            ),
        }
    }
    /// Start the background workers eagerly, without waiting for a first [`Iterator::next`] call.
    ///
    /// See [`ParallelMap::started`].
    pub fn started(self) -> Self {
        Self {
            iter: self.iter.started(),
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

impl<'env, 'scope, I> Iterator for ParallelFilter<'env, 'scope, I>
where
    I: Iterator + 'env + 'scope,
    I::Item: Send + 'env + 'scope,
    'env: 'scope,
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
