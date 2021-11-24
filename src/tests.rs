use super::IteratorExt;
use quickcheck_macros::quickcheck;

#[quickcheck]
fn map_vs_map_parallel(v: Vec<usize>, threads: usize) -> bool {
    let m: Vec<_> = v.clone().into_iter().map(|x| x / 2).collect();
    let mp: Vec<_> = v
        .clone()
        .into_iter()
        .parallel_map(|x| x / 2)
        .threads(threads % 32)
        .collect();

    m == mp
}

#[quickcheck]
fn iter_vs_readhead(v: Vec<usize>, out: usize) -> bool {
    let m: Vec<_> = v.clone().into_iter().map(|x| x / 2).collect();
    let mp: Vec<_> = v
        .clone()
        .into_iter()
        .readahead(out % 32)
        .map(|x| x / 2)
        .collect();

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
#[test]
#[should_panic]
fn panic_always_1() {
    (0..10).parallel_map(|_| panic!("foo")).threads(1).count();
}

#[test]
#[should_panic]
fn panic_always_8() {
    (0..10).parallel_map(|_| panic!("foo")).threads(8).count();
}

#[test]
#[should_panic]
fn panic_once_1() {
    (0..10)
        .parallel_map(|i| {
            if i == 5 {
                panic!("foo");
            } else {
                i
            }
        })
        .threads(1)
        .count();
}

#[test]
#[should_panic]
fn panic_once_8() {
    (0..10)
        .parallel_map(|i| {
            if i == 5 {
                panic!("foo");
            } else {
                i
            }
        })
        .threads(8)
        .count();
}

#[test]
#[should_panic]
fn panic_after_a_point_1() {
    (0..10)
        .parallel_map(|i| {
            if i > 5 {
                panic!("foo");
            } else {
                i
            }
        })
        .threads(1)
        .count();
}

#[test]
#[should_panic]
fn panic_after_a_point_8() {
    (0..10)
        .parallel_map(|i| {
            if i > 5 {
                panic!("foo");
            } else {
                i
            }
        })
        .threads(8)
        .count();
}

#[test]
#[should_panic]
fn panic_before_a_point_1() {
    (0..10)
        .parallel_map(|i| {
            if i < 5 {
                panic!("foo");
            } else {
                i
            }
        })
        .threads(1)
        .count();
}

#[test]
#[should_panic]
fn panic_before_a_point_8() {
    (0..10)
        .parallel_map(|i| {
            if i < 5 {
                panic!("foo");
            } else {
                i
            }
        })
        .threads(8)
        .count();
}
