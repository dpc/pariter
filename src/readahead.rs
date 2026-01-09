use crossbeam_channel::Sender;

use crate::Scope;
use std::{
    marker::PhantomData,
    sync::{atomic::AtomicBool, Arc},
    thread,
};

use crate::DropIndicator;

pub struct ReadaheadBuilder<I>
where
    I: Iterator,
{
    // the iterator we wrapped
    iter: I,
    // max number of items in flight
    buffer_size: Option<usize>,
}

impl<I> ReadaheadBuilder<I>
where
    I: Iterator,
{
    pub fn new(iter: I) -> Self {
        Self {
            iter,
            buffer_size: None,
        }
    }

    pub fn buffer_size(self, num: usize) -> Self {
        Self {
            buffer_size: Some(num),
            ..self
        }
    }

    fn with_common(self) -> (Readahead<I>, Sender<I::Item>, I)
    where
        I: Iterator,
    {
        let buffer_size = self.buffer_size.unwrap_or(0);

        let (tx, rx) = crossbeam_channel::bounded(buffer_size);
        (
            Readahead {
                _iter_marker: PhantomData,
                iter_size_hint: self.iter.size_hint(),
                inner: Some(ReadaheadInner { rx }),
                worker_panicked: Arc::new(AtomicBool::new(false)),
            },
            tx,
            self.iter,
        )
    }

    pub fn with(self) -> Readahead<I>
    where
        I: Iterator + 'static + Send,
        I::Item: Send + 'static,
    {
        let (ret, tx, iter) = self.with_common();

        let drop_indicator = DropIndicator::new(ret.worker_panicked.clone());
        thread::spawn(move || {
            for i in iter {
                // don't panic if the receiver disconnects
                let _ = tx.send(i);
            }
            drop_indicator.cancel();
        });

        ret
    }

    pub fn with_scoped<'env, 'scope>(self, scope: &'scope Scope<'scope, 'env>) -> Readahead<I>
    where
        I: Iterator + 'env + Send,
        I::Item: Send + 'env,
    {
        let (ret, tx, iter) = self.with_common();

        let drop_indicator = DropIndicator::new(ret.worker_panicked.clone());
        scope.spawn(move || {
            for i in iter {
                // don't panic if the receiver disconnects
                let _ = tx.send(i);
            }
            drop_indicator.cancel();
        });

        ret
    }
}
/// And iterator that provides parallelism
/// by running the inner iterator in another thread.
pub struct Readahead<I>
where
    I: Iterator,
{
    _iter_marker: PhantomData<I>,
    iter_size_hint: (usize, Option<usize>),
    inner: Option<ReadaheadInner<I>>,
    worker_panicked: Arc<AtomicBool>,
}

struct ReadaheadInner<I>
where
    I: Iterator,
{
    rx: crossbeam_channel::Receiver<I::Item>,
}

impl<I> Iterator for Readahead<I>
where
    I: Iterator,
    I: Send,
    I::Item: Send,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
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
