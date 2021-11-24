use std::sync::{atomic::AtomicBool, Arc};

use crate::DropIndicator;

use super::ThreadPool;

/// And iterator that provides parallelism
/// by running the inner iterator in another thread.
pub struct Readahead<I, TP>
where
    I: Iterator,
    TP: ThreadPool,
{
    iter: Option<I>,
    iter_size_hint: (usize, Option<usize>),
    thread_pool: TP,
    buffer_size: usize,
    inner: Option<ReadaheadInner<I, TP::JoinHandle>>,
    worker_panicked: Arc<AtomicBool>,
}

impl<I, TP> Readahead<I, TP>
where
    I: Iterator,
    I: Send + 'static,
    TP: ThreadPool,
    I::Item: Send + 'static,
{
    pub fn new(iter: I, buffer_size: usize, thread_pool: TP) -> Self {
        Self {
            iter_size_hint: iter.size_hint(),
            iter: Some(iter),
            buffer_size,
            thread_pool,
            inner: None,
            worker_panicked: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Use a custom thread pool
    ///
    /// # Panics
    ///
    /// Changing thread-pool, after `started` was already called
    /// is not supported and will panic.
    pub fn within<TP2>(self, thread_pool: TP2) -> Readahead<I, TP2>
    where
        TP2: ThreadPool,
    {
        if self.inner.is_some() {
            panic!("Already started. Must call `within` before `started`.");
        }

        Readahead {
            iter: self.iter,
            iter_size_hint: self.iter_size_hint,
            thread_pool,
            inner: None,
            buffer_size: self.buffer_size,
            worker_panicked: self.worker_panicked,
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
            let _join = self.thread_pool.submit(move || {
                while let Some(i) = iter.next() {
                    // don't panic if the receiver disconnects
                    let _ = tx.send(i);
                }
                drop_indicator.cancel();
            });
            self.inner = Some(ReadaheadInner { rx, _join });
        }
    }
}

struct ReadaheadInner<I, JH>
where
    I: Iterator,
{
    rx: crossbeam_channel::Receiver<I::Item>,
    _join: JH,
}

impl<I, TP> Iterator for Readahead<I, TP>
where
    I: Iterator,
    TP: ThreadPool,
    I: Send + 'static,

    I::Item: Send + 'static,
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
