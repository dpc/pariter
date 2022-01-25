use dpc_pariter::IteratorExt;

use std::time;

/// A simple [`readahead::Profiler`] implementation
///
/// This implementation will reports on `stderr` that
/// a given side of a pipeline is blocked, waiting for
/// the other side.
///
/// In a real code this might be logging using `log`,
/// capturing metrics, etc.
struct SimpleProfiler {
    name: String,
    start: time::Instant,
    duration: time::Duration,
}

impl SimpleProfiler {
    fn new(name: &str) -> Self {
        Self {
            start: time::Instant::now(),
            duration: time::Duration::default(),
            name: name.to_string(),
        }
    }
}

impl dpc_pariter::Profiler for SimpleProfiler {
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

        let min_duration_to_report = time::Duration::from_secs(10);
        if min_duration_to_report <= self.duration {
            self.duration -= min_duration_to_report;
            eprintln!("{} is blocked waiting", self.name);
        }
    }
}

fn main() {
    (0..22)
        .profile_egress(SimpleProfiler::new("sending"))
        .readahead(0)
        .profile_ingress(SimpleProfiler::new("receiving"))
        .for_each(|i| {
            println!("{i}");
            // make consuming values slow
            std::thread::sleep(time::Duration::from_secs(1));
        });

    dpc_pariter::scope(|scope| {
        (0..22)
            .map(|i| {
                // make producting values slow
                std::thread::sleep(time::Duration::from_secs(1));
                i
            })
            .readahead_scoped_profiled(
                scope,
                0,
                SimpleProfiler::new("sending2"),
                SimpleProfiler::new("receiving2"),
            )
            .for_each(|i| {
                println!("{i}");
            })
    })
    .expect("thread paniced");
}
