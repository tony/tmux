//! Extended property-based tests for CRDT convergence.

use proptest::prelude::*;
use mux_crdt::{VectorClock, LwwRegister};

proptest! {
    #[test]
    fn vector_clock_merge_commutative(
        ops_a in proptest::collection::vec((0..5u64, 1..100u64), 1..10),
        ops_b in proptest::collection::vec((0..5u64, 1..100u64), 1..10),
    ) {
        let mut a = VectorClock::new();
        let mut b = VectorClock::new();
        for (node, count) in &ops_a {
            for _ in 0..*count {
                a.increment(*node);
            }
        }
        for (node, count) in &ops_b {
            for _ in 0..*count {
                b.increment(*node);
            }
        }
        let ab = a.merge(&b);
        let ba = b.merge(&a);
        prop_assert_eq!(ab, ba);
    }

    #[test]
    fn vector_clock_merge_idempotent(
        ops in proptest::collection::vec((0..5u64, 1..50u64), 1..10),
    ) {
        let mut vc = VectorClock::new();
        for (node, count) in &ops {
            for _ in 0..*count {
                vc.increment(*node);
            }
        }
        let merged = vc.merge(&vc);
        prop_assert_eq!(merged, vc);
    }

    #[test]
    fn vector_clock_merge_associative(
        ops_a in proptest::collection::vec((0..3u64, 1..20u64), 1..5),
        ops_b in proptest::collection::vec((0..3u64, 1..20u64), 1..5),
        ops_c in proptest::collection::vec((0..3u64, 1..20u64), 1..5),
    ) {
        let mut a = VectorClock::new();
        let mut b = VectorClock::new();
        let mut c = VectorClock::new();
        for (node, count) in &ops_a { for _ in 0..*count { a.increment(*node); } }
        for (node, count) in &ops_b { for _ in 0..*count { b.increment(*node); } }
        for (node, count) in &ops_c { for _ in 0..*count { c.increment(*node); } }
        let ab_c = a.merge(&b).merge(&c);
        let a_bc = a.merge(&b.merge(&c));
        prop_assert_eq!(ab_c, a_bc);
    }

    #[test]
    fn lww_register_last_write_wins(
        values in proptest::collection::vec(("[a-z]{1,10}", 1..1000u64), 2..10),
    ) {
        let mut reg = LwwRegister::new("initial".to_string(), 0, 0);
        let mut max_ts = 0u64;
        let mut expected = "initial".to_string();
        for (val, ts) in &values {
            reg.update(val.clone(), *ts, 0);
            // update() only replaces when timestamp is strictly greater
            // (same replica_id means no tie-break on equal timestamps)
            if *ts > max_ts {
                max_ts = *ts;
                expected = val.clone();
            }
        }
        prop_assert_eq!(&reg.value, &expected);
    }

    #[test]
    fn lww_register_merge_commutative(
        val_a in "[a-z]{1,5}", ts_a in 1..1000u64,
        val_b in "[a-z]{1,5}", ts_b in 1..1000u64,
    ) {
        let a = LwwRegister::new(val_a, ts_a, 1);
        let b = LwwRegister::new(val_b, ts_b, 2);
        let ab = a.merge(&b);
        let ba = b.merge(&a);
        prop_assert_eq!(&ab.value, &ba.value);
    }

    #[test]
    fn lww_register_merge_idempotent(val in "[a-z]{1,10}", ts in 1..1000u64) {
        let reg = LwwRegister::new(val, ts, 1);
        let merged = reg.merge(&reg);
        prop_assert_eq!(&merged.value, &reg.value);
    }

    #[test]
    fn vector_clock_increment_increases(node in 0..10u64) {
        let mut vc = VectorClock::new();
        let before = vc.get(node);
        vc.increment(node);
        let after = vc.get(node);
        prop_assert_eq!(after, before + 1);
    }

    #[test]
    fn vector_clock_dominates_after_increment(node in 0..10u64) {
        let mut a = VectorClock::new();
        let b = a.clone();
        a.increment(node);
        prop_assert!(a.dominates(&b));
    }
}
