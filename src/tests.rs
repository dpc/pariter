use crate::TotalTimeProfiler;

use super::IteratorExt;
use quickcheck_macros::quickcheck;

#[quickcheck]
fn map_vs_map_parallel(v: Vec<usize>, threads: usize, max_in_flight: usize) -> bool {
    let m: Vec<_> = v.clone().into_iter().map(|x| x / 2).collect();
    let mp: Vec<_> = v
        .clone()
        .into_iter()
        .parallel_map_custom()
        .threads(threads % 32)
        .buffer_size(max_in_flight % 128)
        .with(|x| x / 2)
        .collect();

    m == mp
}

#[quickcheck]
fn map_vs_map_parallel_double(v: Vec<usize>, threads: usize, max_in_flight: usize) -> bool {
    let m: Vec<_> = v.clone().into_iter().map(|x| x / 2).collect();
    let mp: Vec<_> = v
        .clone()
        .into_iter()
        .parallel_map_custom()
        .threads(threads % 32)
        .buffer_size(max_in_flight % 128)
        .with(|x| x / 2)
        .parallel_map_custom()
        .with(|x| x)
        .collect();

    m == mp
}

#[quickcheck]
fn map_vs_map_parallel_scoped_double(v: Vec<usize>, threads: usize, max_in_flight: usize) -> bool {
    let m: Vec<usize> = v.iter().map(|x| x / 2).collect();
    let mp: Vec<usize> = super::scope(|s| {
        v.iter()
            .parallel_map_custom()
            .threads(threads % 32)
            .with_scoped(s, |x| x / 2)
            .parallel_map_custom()
            .buffer_size(max_in_flight % 128)
            .with_scoped(s, |x| x)
            .collect()
    })
    .expect("failed");

    m == mp
}

#[quickcheck]
fn map_vs_map_parallel_with_readahead(v: Vec<usize>, threads: usize) -> bool {
    let m: Vec<_> = v.clone().into_iter().map(|x| x / 2).collect();
    let mp: Vec<_> = v
        .clone()
        .into_iter()
        .parallel_map_custom()
        .threads(threads % 32)
        .with(|x| x / 2)
        .readahead()
        .parallel_map_custom()
        .threads(threads % 32)
        .with(|x| x)
        .readahead()
        .parallel_map_custom()
        .threads(threads % 32)
        .with(|x| x)
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
    let mp: Vec<usize> = super::scope(|s| {
        v.iter()
            .parallel_map_custom()
            .threads(threads % 32)
            .with_scoped(s, |x| x / 2)
            .readahead_scoped(s)
            .parallel_map_custom()
            .buffer_size(max_in_flight % 128)
            .with_scoped(s, |x| x)
            .readahead_scoped(s)
            .parallel_map_custom()
            .with_scoped(s, |x| x)
            .readahead_scoped(s)
            .parallel_map_custom()
            .threads(threads % 32)
            .with_scoped(s, |x| x)
            .collect()
    })
    .expect("failed");

    m == mp
}

#[quickcheck]
fn check_profile_compiles(v: Vec<usize>) -> bool {
    let m: Vec<usize> = v.iter().map(|x| x / 2).collect();
    let mp: Vec<usize> = super::scope(|s| {
        v.iter()
            .parallel_map_custom()
            .with_scoped(s, |x| x / 2)
            .profile_egress(TotalTimeProfiler::periodically_millis(10_000, || {
                eprintln!("Blocked on sending")
            }))
            .profile_ingress(TotalTimeProfiler::periodically_millis(10_000, || {
                eprintln!("Blocked on sending")
            }))
            .readahead_scoped(s)
            .parallel_map_custom()
            .with_scoped(s, |x| x)
            .readahead_scoped(s)
            .profile_egress(TotalTimeProfiler::periodically_millis(10_000, || {
                eprintln!("Blocked on sending")
            }))
            .profile_ingress(TotalTimeProfiler::periodically_millis(10_000, || {
                eprintln!("Blocked on sending")
            }))
            .parallel_map_custom()
            .with_scoped(s, |x| x)
            .readahead_scoped(s)
            .parallel_map_custom()
            .with_scoped(s, |x| x)
            .collect()
    })
    .expect("failed");

    m == mp
}

#[quickcheck]
fn iter_vs_readhead(v: Vec<usize>, out: usize) -> bool {
    let m: Vec<_> = v.clone().into_iter().map(|x| x / 2).collect();
    let mp: Vec<_> = v
        .clone()
        .into_iter()
        .readahead_custom()
        .buffer_size(out % 32)
        .with()
        .map(|x| x / 2)
        .collect();

    m == mp
}

#[quickcheck]
fn iter_vs_readhead_scoped(v: Vec<usize>, out: usize) -> bool {
    let m: Vec<_> = v.iter().map(|x| x / 2).collect();
    let mp: Vec<_> = super::scope(|s| {
        v.iter()
            .readahead_custom()
            .buffer_size(out % 32)
            .with_scoped(s)
            .map(|x| x / 2)
            .collect()
    })
    .expect("failed");

    m == mp
}

#[quickcheck]
fn filter_vs_parallel_filter(v: Vec<usize>) -> bool {
    let m: Vec<_> = v.clone().into_iter().filter(|x| x % 2 == 0).collect();
    let mp: Vec<_> = v
        .clone()
        .into_iter()
        .parallel_filter_custom()
        .with(|x| x % 2 == 0)
        .collect();

    m == mp
}

#[quickcheck]
fn filter_vs_parallel_filter_scoped(v: Vec<usize>) -> bool {
    let m: Vec<_> = v.iter().filter(|x| *x % 2 == 0).collect();
    let mp: Vec<_> = super::scope(|s| {
        v.iter()
            .parallel_filter_custom()
            .with_scoped(s, |x| *x % 2 == 0)
            .collect()
    })
    .expect("failed");

    m == mp
}

#[test]
#[should_panic]
fn panic_always_1() {
    (0..10)
        .parallel_map_custom()
        .threads(1)
        .with(|_| panic!("foo"))
        .count();
}

#[test]
#[should_panic]
fn panic_always_8() {
    (0..10)
        .parallel_map_custom()
        .threads(8)
        .with(|_| panic!("foo"))
        .count();
}

#[test]
#[should_panic]
fn panic_once_1() {
    (0..10)
        .parallel_map_custom()
        .threads(1)
        .with(|i| {
            if i == 5 {
                panic!("foo");
            } else {
                i
            }
        })
        .count();
}

#[test]
#[should_panic]
fn panic_once_8() {
    (0..10)
        .parallel_map_custom()
        .threads(8)
        .with(|i| {
            if i == 5 {
                panic!("foo");
            } else {
                i
            }
        })
        .count();
}

#[test]
#[should_panic]
fn panic_after_a_point_1() {
    (0..10)
        .parallel_map_custom()
        .threads(1)
        .with(|i| {
            if i > 5 {
                panic!("foo");
            } else {
                i
            }
        })
        .count();
}

#[test]
#[should_panic]
fn panic_after_a_point_8() {
    (0..10)
        .parallel_map_custom()
        .threads(8)
        .with(|i| {
            if i > 5 {
                panic!("foo");
            } else {
                i
            }
        })
        .count();
}

#[test]
#[should_panic]
fn panic_before_a_point_1() {
    (0..10)
        .parallel_map_custom()
        .threads(1)
        .with(|i| {
            if i < 5 {
                panic!("foo");
            } else {
                i
            }
        })
        .count();
}

#[test]
#[should_panic]
fn panic_before_a_point_8() {
    (0..10)
        .parallel_map_custom()
        .threads(8)
        .with(|i| {
            if i < 5 {
                panic!("foo");
            } else {
                i
            }
        })
        .count();
}
