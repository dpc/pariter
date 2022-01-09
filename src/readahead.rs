use crate::Scope;
use std::sync::{atomic::AtomicBool, Arc};

use crate::DropIndicator;

/// And iterator that provides parallelism
/// by running the inner iterator in another thread.
pub struct Readahead<'env, 'scope, I>
where
    I: Iterator,
{
    spawn_fn: Box<dyn Fn(Box<dyn FnOnce() + Send + 'env>) + 'scope>,

    iter: Option<I>,
    iter_size_hint: (usize, Option<usize>),
    buffer_size: usize,
    inner: Option<ReadaheadInner<I>>,
    worker_panicked: Arc<AtomicBool>,
}

impl<I> Readahead<'static, 'static, I>
where
    I: Iterator,
    I: Send + 'static,
    I::Item: Send + 'static,
{
    pub fn new(iter: I, buffer_size: usize) -> Self {
        Self {
            spawn_fn: Box::new(move |f: Box<dyn FnOnce() + Send + 'static>| {
                std::thread::spawn(move || f());
            }) as Box<dyn Fn(_) + 'static>,

            iter_size_hint: iter.size_hint(),
            iter: Some(iter),
            buffer_size,
            inner: None,
            worker_panicked: Arc::new(AtomicBool::new(false)),
        }
    }
}
impl<'env, 'scope, I> Readahead<'env, 'scope, I>
where
    I: Iterator,
    I: Send + 'env + 'scope,
    I::Item: Send + 'env + 'scope,
{
    pub fn new_scoped(scope: &'scope Scope<'env>, iter: I, buffer_size: usize) -> Self {
        Self {
            spawn_fn: Box::new(move |f: Box<dyn FnOnce() + Send + 'env>| {
                scope.spawn(move |_| f());
            }) as Box<(dyn Fn(_) + 'scope)>,

            iter_size_hint: iter.size_hint(),
            iter: Some(iter),
            buffer_size,
            inner: None,
            worker_panicked: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start the background worker eagerly, without waiting for a first [`Iterator::next`] call.
    pub fn started(mut self) -> Self {
        self.ensure_started();
        self
    }

    fn ensure_started(&mut self) {
        if self.inner.is_none() {
            let (tx, rx) = crossbeam_channel::bounded(self.buffer_size);

            let drop_indicator = DropIndicator::new(self.worker_panicked.clone());
            let mut iter = self.iter.take().expect("iter empty?!");
            (self.spawn_fn)(Box::new(move || {
                while let Some(i) = iter.next() {
                    // don't panic if the receiver disconnects
                    let _ = tx.send(i);
                }
                drop_indicator.cancel();
            }));
            self.inner = Some(ReadaheadInner { rx });
        }
    }
}

struct ReadaheadInner<I>
where
    I: Iterator,
{
    rx: crossbeam_channel::Receiver<I::Item>,
}

impl<'env, 'scope, I> Iterator for Readahead<'env, 'scope, I>
where
    I: Iterator + 'env + 'scope,
    I: Send + 'env + 'scope,
    I::Item: Send + 'env + 'scope,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.ensure_started();

        match self.inner.as_ref().expect("thread started").rx.recv() {
            Ok(i) => Some(i),
            Err(crossbeam_channel::RecvError) => {
                if self
                    .worker_panicked
                    .load(std::sync::atomic::Ordering::SeqCst)
                {
                    panic!("readahead worker thread panicked: panic indicator set");
                } else {
                    None
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter_size_hint
    }
}
