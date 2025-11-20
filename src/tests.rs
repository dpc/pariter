use crate::TotalTimeProfiler;

use super::IteratorExt;
use quickcheck_macros::quickcheck;

#[quickcheck]
fn map_vs_map_parallel(v: Vec<usize>, threads: usize, max_in_flight: usize) -> bool {
    let m: Vec<_> = v.clone().into_iter().map(|x| x / 2).collect();
    let mp: Vec<_> = v
        .clone()
        .into_iter()
        .parallel_map_custom(
            |o| o.threads(threads % 32).buffer_size(max_in_flight % 128),
            |x| x / 2,
        )
        .collect();

    m == mp
}

#[quickcheck]
fn map_vs_map_parallel_double(v: Vec<usize>, threads: usize, max_in_flight: usize) -> bool {
    let m: Vec<_> = v.clone().into_iter().map(|x| x / 2).collect();
    let mp: Vec<_> = v
        .clone()
        .into_iter()
        .parallel_map_custom(
            |o| o.threads(threads % 32).buffer_size(max_in_flight % 128),
            |x| x / 2,
        )
        .parallel_map_custom(
            |o| o.threads(threads % 32).buffer_size(max_in_flight % 128),
            |x| x,
        )
        .collect();

    m == mp
}

#[quickcheck]
fn map_vs_map_parallel_scoped_double(v: Vec<usize>, threads: usize, max_in_flight: usize) -> bool {
    let m: Vec<usize> = v.iter().map(|x| x / 2).collect();
    let mp: Vec<usize> = std::thread::scope(|s| {
        v.iter()
            .parallel_map_scoped_custom(s, |o| o.threads(threads % 32), |x| x / 2)
            .parallel_map_scoped_custom(s, |o| o.buffer_size(max_in_flight % 128), |x| x)
            .collect()
    });

    m == mp
}

#[quickcheck]
fn map_vs_map_parallel_with_readahead(v: Vec<usize>, threads: usize) -> bool {
    let m: Vec<_> = v.clone().into_iter().map(|x| x / 2).collect();
    let mp: Vec<_> = v
        .clone()
        .into_iter()
        .parallel_map_custom(|o| o.threads(threads % 32), |x| x / 2)
        .readahead()
        .parallel_map_custom(|o| o.threads(threads % 32), |x| x)
        .readahead()
        .parallel_map_custom(|o| o.threads(threads % 32), |x| x)
        .collect();

    m == mp
}

#[quickcheck]
fn map_vs_map_parallel_scoped_with_readahead(
    v: Vec<usize>,
    threads: usize,
    max_in_flight: usize,
) -> bool {
    let m: Vec<usize> = v.iter().map(|x| x / 2).collect();
    let mp: Vec<usize> = std::thread::scope(|s| {
        v.iter()
            .parallel_map_scoped_custom(s, |o| o.threads(threads % 32), |x| x / 2)
            .readahead_scoped(s)
            .parallel_map_scoped_custom(s, |o| o.buffer_size(max_in_flight % 128), |x| x)
            .readahead_scoped(s)
            .parallel_map_scoped(s, |x| x)
            .readahead_scoped(s)
            .parallel_map_scoped_custom(s, |o| o.threads(threads % 32), |x| x)
            .collect()
    });

    m == mp
}

#[quickcheck]
fn check_profile_compiles(v: Vec<usize>) -> bool {
    let m: Vec<usize> = v.iter().map(|x| x / 2).collect();
    let mp: Vec<usize> = std::thread::scope(|s| {
        v.iter()
            .parallel_map_scoped(s, |x| x / 2)
            .profile_egress(TotalTimeProfiler::periodically_millis(10_000, || {
                eprintln!("Blocked on sending")
            }))
            .profile_ingress(TotalTimeProfiler::periodically_millis(10_000, || {
                eprintln!("Blocked on sending")
            }))
            .readahead_scoped(s)
            .parallel_map_scoped_custom(s, |o| o, |x| x)
            .readahead_scoped(s)
            .profile_egress(TotalTimeProfiler::periodically_millis(10_000, || {
                eprintln!("Blocked on sending")
            }))
            .profile_ingress(TotalTimeProfiler::periodically_millis(10_000, || {
                eprintln!("Blocked on sending")
            }))
            .parallel_map_scoped_custom(s, |o| o, |x| x)
            .readahead_scoped(s)
            .parallel_map_scoped_custom(s, |o| o, |x| x)
            .collect()
    });

    m == mp
}

#[quickcheck]
fn iter_vs_readhead(v: Vec<usize>, out: usize) -> bool {
    let m: Vec<_> = v.clone().into_iter().map(|x| x / 2).collect();
    let mp: Vec<_> = v
        .clone()
        .into_iter()
        .readahead_custom(|o| o.buffer_size(out % 32))
        .map(|x| x / 2)
        .collect();

    m == mp
}

#[quickcheck]
fn iter_vs_readhead_scoped(v: Vec<usize>, out: usize) -> bool {
    let m: Vec<_> = v.iter().map(|x| x / 2).collect();
    let mp: Vec<_> = std::thread::scope(|s| {
        v.iter()
            .readahead_scoped_custom(s, |o| o.buffer_size(out % 32))
            .map(|x| x / 2)
            .collect()
    });

    m == mp
}

#[quickcheck]
fn filter_vs_parallel_filter(v: Vec<usize>) -> bool {
    let m: Vec<_> = v.clone().into_iter().filter(|x| x % 2 == 0).collect();
    let mp: Vec<_> = v
        .clone()
        .into_iter()
        .parallel_filter(|x| x % 2 == 0)
        .collect();

    m == mp
}

#[quickcheck]
fn filter_vs_parallel_filter_scoped(v: Vec<usize>) -> bool {
    let m: Vec<_> = v.iter().filter(|x| *x % 2 == 0).collect();
    let mp: Vec<_> = std::thread::scope(|s| {
        v.iter()
            .parallel_filter_scoped(s, |x| *x % 2 == 0)
            .collect()
    });

    m == mp
}

#[test]
#[should_panic]
fn panic_always_1() {
    (0..10)
        .parallel_map_custom(|o| o.threads(1), |_| panic!("foo"))
        .count();
}

#[test]
#[should_panic]
fn panic_always_8() {
    (0..10)
        .parallel_map_custom(|o| o.threads(8), |_| panic!("foo"))
        .count();
}

#[test]
#[should_panic]
fn panic_once_1() {
    (0..10)
        .parallel_map_custom(
            |o| o.threads(1),
            |i| {
                if i == 5 {
                    panic!("foo");
                } else {
                    i
                }
            },
        )
        .count();
}

#[test]
#[should_panic]
fn panic_once_8() {
    (0..10)
        .parallel_map_custom(
            |o| o.threads(8),
            |i| {
                if i == 5 {
                    panic!("foo");
                } else {
                    i
                }
            },
        )
        .count();
}

#[test]
#[should_panic]
fn panic_after_a_point_1() {
    (0..10)
        .parallel_map_custom(
            |o| o.threads(1),
            |i| {
                if i > 5 {
                    panic!("foo");
                } else {
                    i
                }
            },
        )
        .count();
}

#[test]
#[should_panic]
fn panic_after_a_point_8() {
    (0..10)
        .parallel_map_custom(
            |o| o.threads(8),
            |i| {
                if i > 5 {
                    panic!("foo");
                } else {
                    i
                }
            },
        )
        .count();
}

#[test]
#[should_panic]
fn panic_before_a_point_1() {
    (0..10)
        .parallel_map_custom(
            |o| o.threads(1),
            |i| {
                if i < 5 {
                    panic!("foo");
                } else {
                    i
                }
            },
        )
        .count();
}

#[test]
#[should_panic]
fn panic_before_a_point_8() {
    (0..10)
        .parallel_map_custom(
            |o| o.threads(8),
            |i| {
                if i < 5 {
                    panic!("foo");
                } else {
                    i
                }
            },
        )
        .count();
}
