use dpc_pariter::IteratorExt;

use std::time;

/// Custom [`readahead::Profiler`] implementation
///
/// This implementation will reports on `stderr` that
/// a given side of a pipeline is blocked, waiting for
/// the other side.
///
/// In a real code this might be logging using `log`,
/// capturing metrics, etc.
struct StderrMsgProfiler {
    name: String,
    start: time::Instant,
    duration: time::Duration,
}

impl StderrMsgProfiler {
    fn new(name: &str) -> Self {
        Self {
            start: time::Instant::now(),
            duration: time::Duration::default(),
            name: name.to_string(),
        }
    }
}

impl dpc_pariter::Profiler for StderrMsgProfiler {
    fn start(&mut self) {
        self.start = time::Instant::now();
    }

    fn end(&mut self) {
        self.duration += time::Instant::now()
            .duration_since(self.start)
            // Even with absolutely no delay waiting for
            // the other side of the channel a send/recv will take some time.
            // Substract some tiny value to account for it, to prevent
            // rare but spurious and confusing messages.
            .saturating_sub(time::Duration::from_millis(1));

        let min_duration_to_report = time::Duration::from_millis(100);
        if min_duration_to_report <= self.duration {
            eprintln!(
                "{} was blocked waiting for {}ms",
                self.name,
                self.duration.as_millis()
            );
            self.duration -= min_duration_to_report;
        }
    }
}

fn main() {
    dpc_pariter::scope(|scope| {
        (0..22)
            .map(|i| {
                // make producting values slow
                std::thread::sleep(time::Duration::from_millis(10));
                i
            })
            .readahead_scoped_profiled(
                scope,
                dpc_pariter::TotalTimeProfiler::periodically_millis(2_000, || {
                    eprintln!("Blocked on sending")
                }),
                dpc_pariter::TotalTimeProfiler::new(|stat| {
                    eprintln!(
                        "Sending receiving wait time: {}ms",
                        stat.total().as_millis()
                    )
                }),
            )
            .for_each(|i| {
                println!("{i}");
            })
    })
    .expect("thread panicked");

    (0..22)
        .profile_egress(StderrMsgProfiler::new("sending"))
        .readahead()
        .profile_ingress(StderrMsgProfiler::new("receiving"))
        .for_each(|i| {
            println!("{i}");
            // make consuming values slow
            std::thread::sleep(time::Duration::from_millis(10));
        });

    dpc_pariter::scope(|scope| {
        (0..22)
            .map(|i| {
                // make producting values slow
                std::thread::sleep(time::Duration::from_millis(10));
                i
            })
            .readahead_scoped_profiled(
                scope,
                StderrMsgProfiler::new("sending2"),
                StderrMsgProfiler::new("receiving2"),
            )
            .for_each(|i| {
                println!("{i}");
            })
    })
    .expect("thread panicked");
}
