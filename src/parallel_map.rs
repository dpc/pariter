use crossbeam_channel::{Receiver, Sender};

use super::{DropIndicator, Scope};

use std::{
    cmp,
    sync::{
        atomic::{AtomicBool, Ordering::SeqCst},
        Arc,
    },
};

struct ParallelMapInner<I, O> {
    tx: Option<crossbeam_channel::Sender<(usize, I)>>,
    rx: crossbeam_channel::Receiver<(usize, O)>,
}

pub struct ParallelMapBuilder<I>
where
    I: Iterator,
{
    // the iterator we wrapped
    iter: I,
    // number of worker threads to use
    num_threads: Option<usize>,
    // max number of items in flight
    buffer_size: Option<usize>,
}

impl<I> ParallelMapBuilder<I>
where
    I: Iterator,
{
    pub fn new(iter: I) -> Self {
        Self {
            iter,
            num_threads: None,
            buffer_size: None,
        }
    }

    pub fn threads(self, num: usize) -> Self {
        Self {
            num_threads: Some(num),
            ..self
        }
    }
    pub fn buffer_size(self, num: usize) -> Self {
        Self {
            buffer_size: Some(num),
            ..self
        }
    }

    fn num_threads<T: Into<Option<usize>>>(num_threads: T) -> usize {
        let mut num = num_threads.into().unwrap_or(0);
        if num == 0 {
            num = num_cpus::get_physical();
        }
        if num == 0 {
            num = 1
        }
        num
    }

    fn with_common<O>(
        self,
    ) -> (
        ParallelMap<I, O>,
        Receiver<(usize, I::Item)>,
        Sender<(usize, O)>,
    )
    where
        I: Iterator,
    {
        let num_threads = Self::num_threads(self.num_threads);
        let buffer_size = cmp::max(1, self.buffer_size.unwrap_or(num_threads * 2));

        // Note: we have enought capacity on both ends to hold all items
        // in progress, though the actual amount of items in flight is controlled
        // by `pump_tx`.
        let (in_tx, in_rx) = crossbeam_channel::bounded(buffer_size);
        let (out_tx, out_rx) = crossbeam_channel::bounded(buffer_size);

        (
            ParallelMap {
                iter: self.iter,
                iter_done: false,
                worker_panicked: Arc::new(AtomicBool::new(false)),
                num_threads,
                buffer_size,
                out_of_order: Vec::new(),
                next_tx_i: 0,
                next_rx_i: 0,
                inner: Some(ParallelMapInner {
                    tx: Some(in_tx),
                    rx: out_rx,
                }),
            },
            in_rx,
            out_tx,
        )
    }

    pub fn with<F, O>(self, f: F) -> ParallelMap<I, O>
    where
        I: Iterator + 'static,
        F: 'static + Send + Clone,
        O: Send + 'static,
        I::Item: Send + 'static,
        F: FnMut(I::Item) -> O,
    {
        let (ret, in_rx, out_tx) = self.with_common();

        for _ in 0..ret.num_threads {
            let in_rx = in_rx.clone();
            let out_tx = out_tx.clone();
            let mut f = f.clone();
            let drop_indicator = DropIndicator::new(ret.worker_panicked.clone());

            std::thread::spawn(move || {
                for (i, item) in in_rx.into_iter() {
                    // we ignore send failures, if the receiver is gone
                    // we just throw the work away
                    let _ = out_tx.send((i, (f)(item)));
                }
                drop_indicator.cancel();
            });
        }

        ret
    }

    pub fn with_scoped<'env, 'scope, F, O>(
        self,
        scope: &'scope Scope<'env>,
        f: F,
    ) -> ParallelMap<I, O>
    where
        I: Iterator + 'env,
        F: 'env + Send + Clone,
        O: Send + 'env,
        I::Item: Send + 'env,
        F: FnMut(I::Item) -> O,
    {
        let (ret, in_rx, out_tx) = self.with_common();

        for _ in 0..ret.num_threads {
            let in_rx = in_rx.clone();
            let out_tx = out_tx.clone();
            let mut f = f.clone();
            let drop_indicator = DropIndicator::new(ret.worker_panicked.clone());

            scope.spawn(move |_scope| {
                for (i, item) in in_rx.into_iter() {
                    // we ignore send failures, if the receiver is gone
                    // we just throw the work away
                    let _ = out_tx.send((i, (f)(item)));
                }
                drop_indicator.cancel();
            });
        }

        ret
    }
}

/// Like [`std::iter::Map`] but multi-threaded
pub struct ParallelMap<I, O>
where
    I: Iterator,
{
    // the iterator we wrapped
    iter: I,
    // is `iter` exhausted
    iter_done: bool,
    // number of worker threads to use
    num_threads: usize,
    // max number of items in flight
    buffer_size: usize,
    /// the id of the work we are going to send next
    next_tx_i: usize,
    /// the id of response we are waiting for
    next_rx_i: usize,
    /// did any worker thread failed us
    worker_panicked: Arc<AtomicBool>,
    /// responses we received before we needed them
    out_of_order: Vec<(usize, O)>,
    // stuff we created when we started workers
    inner: Option<ParallelMapInner<I::Item, O>>,
}

impl<I, O> ParallelMap<I, O>
where
    I: Iterator,
    I::Item: Send,
    O: Send,
{
    /// Fill the worker incoming queue with work
    fn pump_tx(&mut self) {
        if self.iter_done {
            return;
        }

        while self.next_tx_i < self.next_rx_i + self.buffer_size {
            if let Some(item) = self.iter.next() {
                self.inner
                    .as_ref()
                    .expect("not started")
                    .tx
                    .as_ref()
                    .expect("inner-iterator exhausted")
                    .send((self.next_tx_i, item))
                    .expect("send failed");
                self.next_tx_i += 1;
            } else {
                self.iter_done = true;
                self.inner.as_mut().expect("not started").tx = None;
                break;
            }
        }
    }
}

impl<I, O> Iterator for ParallelMap<I, O>
where
    I: Iterator,
    I::Item: Send,
    O: Send,
{
    type Item = O;

    fn next(&mut self) -> Option<Self::Item> {
        self.pump_tx();

        loop {
            // inner iterator is done, and all work sent was already received back
            if self.next_rx_i == self.next_tx_i && self.iter_done {
                return None;
            }

            // check if we didn't receive this item out of order
            if let Some(index) = self
                .out_of_order
                .iter()
                .position(|(i, _)| (i == &self.next_rx_i))
            {
                let item = self.out_of_order.swap_remove(index).1;
                self.next_rx_i += 1;
                self.pump_tx();
                return Some(item);
            }

            // there are multiple ways to detect worker panics, but here we
            // use a timeout to periodically check atomic bool.
            match self
                .inner
                .as_ref()
                .expect("not started")
                .rx
                .recv_timeout(std::time::Duration::from_micros(100))
            {
                Ok((item_i, item)) => {
                    if item_i == self.next_rx_i {
                        self.next_rx_i += 1;
                        self.pump_tx();
                        return Some(item);
                    } else {
                        assert!(item_i > self.next_rx_i);
                        self.out_of_order.push((item_i, item));
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    if self.worker_panicked.load(SeqCst) {
                        panic!("parallel_map worker thread panicked: panic indicator set");
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    panic!("parallel_map worker thread panicked: channel disconnected");
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
