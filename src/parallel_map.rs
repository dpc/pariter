use super::{DropIndicator, ThreadPool};

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering::SeqCst},
        Arc,
    },
};

struct ParallelMapInner<I, O, JH> {
    tx: Option<crossbeam_channel::Sender<(usize, I)>>,
    rx: crossbeam_channel::Receiver<(usize, O)>,
    _thread_handle: Vec<JH>,
}

/// Like [`std::iter::Map`] but multi-threaded
pub struct ParallelMap<I, O, F, TP>
where
    TP: ThreadPool,
    I: Iterator,
{
    // the iterator we wrapped
    iter: I,
    // is `iter` exhausted
    iter_done: bool,
    // thread pool to use
    thread_pool: TP,
    // number of worker threads to use
    num_threads: usize,
    // max number of items in flight
    max_in_flight: usize,
    /// the id of the work we are going to send next
    next_tx_i: usize,
    /// the id of response we are waiting for
    next_rx_i: usize,
    /// did any worker thread failed us
    worker_panicked: Arc<AtomicBool>,
    /// responses we received before we needed them
    out_of_order: HashMap<usize, O>,
    /// the function this map applies
    f: F,
    // stuff we created when we started workers
    inner: Option<ParallelMapInner<I::Item, O, TP::JoinHandle>>,
}

impl<I, O, F, TP> ParallelMap<I, O, F, TP>
where
    TP: ThreadPool,
    I: Iterator,
{
    pub fn new(iter: I, thread_pool: TP, f: F) -> Self {
        Self {
            iter,
            iter_done: false,
            worker_panicked: Arc::new(AtomicBool::new(false)),
            thread_pool,
            f,
            num_threads: 0,
            max_in_flight: 0,
            out_of_order: HashMap::new(),
            next_tx_i: 0,
            next_rx_i: 0,
            inner: None,
        }
    }

    /// Use a custom thread pool
    ///
    /// # Panics
    ///
    /// Changing thread-pool, after `started` was already called
    /// is not supported and will panic.
    pub fn within<TP2>(self, thread_pool: TP2) -> ParallelMap<I, O, F, TP2>
    where
        TP2: ThreadPool,
    {
        if self.inner.is_some() {
            panic!("Already started. Must call `within` before `started`.");
        }

        ParallelMap {
            iter: self.iter,
            iter_done: self.iter_done,
            thread_pool,
            num_threads: self.num_threads,
            max_in_flight: self.max_in_flight,
            worker_panicked: self.worker_panicked,
            next_rx_i: self.next_rx_i,
            next_tx_i: self.next_tx_i,
            out_of_order: self.out_of_order,
            f: self.f,
            inner: None,
        }
    }

    /// Set number of threads to use manually.
    ///
    /// Default or `0` means autodection.
    pub fn threads(self, num_threads: usize) -> Self {
        Self {
            num_threads: num_threads,
            ..self
        }
    }

    /// Set max number of items in flight
    ///
    /// Default or `0` means twice as many as number of threads.
    ///
    /// Larger values might waste some memory, smaller might lead
    /// to worker threads being under-utilizied.
    pub fn max_in_flight(self, max_in_flight: usize) -> Self {
        Self {
            max_in_flight: max_in_flight,
            ..self
        }
    }
}

impl<I, O, F, TP> ParallelMap<I, O, F, TP>
where
    I: Iterator,
    I::Item: Send + 'static,
    O: Send + 'static,
    TP: ThreadPool,
    F: FnMut(I::Item) -> O,
    F: Clone + Send + 'static,
{
    /// Start the background workers eagerly, without waiting for a first [`Iterator::next`] call.
    ///
    /// Normally, like any other good Rust iterator,
    /// [`ParallelMap`] will not perform any work until it
    /// is polled for an item.
    ///
    /// Note: After the first element was requested, the
    /// whole point of a parallel processing is to handle
    /// them ahead of time, so multiple items will be pulled from the
    /// inner iterator without waiting.
    pub fn started(mut self) -> Self {
        self.ensure_started();
        self
    }

    fn ensure_started(&mut self) {
        if self.inner.is_none() {
            if self.num_threads == 0 {
                self.num_threads = num_cpus::get();
            }
            if self.num_threads == 0 {
                panic!("Could not detect number of threads");
            }

            if self.max_in_flight == 0 {
                self.max_in_flight = 2 * self.num_threads;
            }

            // Note: we have enought capacity on both ends to hold all items
            // in progress, though the actual amount of items in flight is controlled
            // by `pump_tx`.
            let (in_tx, in_rx) = crossbeam_channel::bounded(self.max_in_flight);
            let (out_tx, out_rx) = crossbeam_channel::bounded(self.max_in_flight);
            self.inner = Some(ParallelMapInner {
                tx: Some(in_tx),
                rx: out_rx,
                _thread_handle: (0..self.num_threads)
                    .map(|_| {
                        let in_rx = in_rx.clone();
                        let out_tx = out_tx.clone();
                        let mut f = self.f.clone();
                        let drop_indicator = DropIndicator::new(self.worker_panicked.clone());
                        self.thread_pool.submit({
                            move || {
                                for (i, item) in in_rx.into_iter() {
                                    // we ignore send failures, if the receiver is gone
                                    // we just throw the work away
                                    let _ = out_tx.send((i, (f)(item)));
                                }
                                drop_indicator.cancel();
                            }
                        })
                    })
                    .collect(),
            });
            self.pump_tx();
        }
    }

    /// Fill the worker incoming queue with work
    fn pump_tx(&mut self) {
        if self.iter_done {
            return;
        }

        while self.next_tx_i < self.next_rx_i + self.max_in_flight {
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

impl<I, O, F, TP> Iterator for ParallelMap<I, O, F, TP>
where
    I: Iterator,
    I::Item: Send + 'static,
    O: Send + 'static,
    TP: ThreadPool,
    F: FnMut(I::Item) -> O,
    F: Clone + Send + 'static,
{
    type Item = O;

    fn next(&mut self) -> Option<Self::Item> {
        self.ensure_started();

        loop {
            // inner iterator is done, and all work sent was already received back
            if self.next_rx_i == self.next_tx_i && self.iter_done {
                return None;
            }

            // check if we didn't receive this item out of order
            if let Some(item) = self.out_of_order.remove(&self.next_rx_i) {
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
                        self.out_of_order.insert(item_i, item);
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
