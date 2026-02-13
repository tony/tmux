//! Integration tests: CRDT convergence verification.

use mux_crdt::{VectorClock, LwwRegister};

#[test]
fn three_way_merge_convergence() {
    let mut a = VectorClock::new();
    a.increment(1);
    let mut b = VectorClock::new();
    b.increment(2);
    let mut c = VectorClock::new();
    c.increment(3);

    let ab = a.merge(&b);
    let abc = ab.merge(&c);

    let bc = b.merge(&c);
    let bca = bc.merge(&a);

    assert_eq!(abc, bca);
}

#[test]
fn lww_convergence_any_order() {
    let r1 = LwwRegister::new("old", 1, 1);
    let r2 = LwwRegister::new("mid", 2, 2);
    let r3 = LwwRegister::new("new", 3, 3);

    let m1 = r1.merge(&r2).merge(&r3);
    let m2 = r3.merge(&r1).merge(&r2);
    let m3 = r2.merge(&r3).merge(&r1);

    assert_eq!(m1.value, "new");
    assert_eq!(m2.value, "new");
    assert_eq!(m3.value, "new");
}

#[test]
fn concurrent_increments_merge() {
    let mut replicas: Vec<VectorClock> = vec![VectorClock::new(); 5];
    for (i, vc) in replicas.iter_mut().enumerate() {
        for _ in 0..10 {
            vc.increment(i as u64);
        }
    }

    let merged = replicas.iter().fold(VectorClock::new(), |acc, vc| acc.merge(vc));
    for i in 0..5 {
        assert_eq!(merged.get(i as u64), 10);
    }
}

#[test]
fn dominance_after_merge() {
    let mut a = VectorClock::new();
    a.increment(1);
    let mut b = VectorClock::new();
    b.increment(2);

    let merged = a.merge(&b);
    assert!(merged.dominates(&a));
    assert!(merged.dominates(&b));
}
