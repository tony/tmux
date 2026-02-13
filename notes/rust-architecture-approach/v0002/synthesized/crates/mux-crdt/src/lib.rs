#![forbid(unsafe_code)]

#[cfg(feature = "crdt")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "crdt", derive(Serialize, Deserialize))]
pub struct VectorClock {
    inner: BTreeMap<u64, u64>,
}

impl VectorClock {
    #[must_use]
    pub fn new() -> Self {
        Self { inner: BTreeMap::new() }
    }

    pub fn tick(&mut self, node: u64) {
        *self.inner.entry(node).or_insert(0) += 1;
    }

    #[must_use]
    pub fn dominates(&self, other: &Self) -> bool {
        let mut strictly_greater = false;
        for (node, o) in &other.inner {
            let s = self.inner.get(node).copied().unwrap_or(0);
            if s < *o {
                return false;
            }
            if s > *o {
                strictly_greater = true;
            }
        }
        strictly_greater || self.inner.len() > other.inner.len()
    }

    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        let mut out = self.clone();
        for (n, v) in &other.inner {
            let e = out.inner.entry(*n).or_insert(0);
            if *e < *v {
                *e = *v;
            }
        }
        out
    }
}

impl Default for VectorClock {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LwwValue<T> {
    pub value: T,
    pub ts: u64,
    pub node: u64,
}

impl<T: Clone> LwwValue<T> {
    #[must_use]
    pub fn resolve(a: &Self, b: &Self) -> Self {
        if a.ts > b.ts {
            return a.clone();
        }
        if b.ts > a.ts {
            return b.clone();
        }
        if a.node <= b.node { a.clone() } else { b.clone() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_advances_clock() {
        let mut vc = VectorClock::new();
        vc.tick(1);
        assert!(vc.dominates(&VectorClock::new()));
    }

    #[test]
    fn merge_takes_maxima() {
        let mut a = VectorClock::new();
        let mut b = VectorClock::new();
        a.tick(1);
        b.tick(2);
        let m = a.merge(&b);
        assert!(m.dominates(&a));
        assert!(m.dominates(&b));
    }

    #[test]
    fn dominance_detects_order() {
        let mut a = VectorClock::new();
        let mut b = VectorClock::new();
        a.tick(1);
        a.tick(1);
        b.tick(1);
        assert!(a.dominates(&b));
    }

    #[test]
    fn dominance_false_for_incomparable() {
        let mut a = VectorClock::new();
        let mut b = VectorClock::new();
        a.tick(1);
        b.tick(2);
        assert!(!a.dominates(&b));
        assert!(!b.dominates(&a));
    }

    #[test]
    fn lww_uses_timestamp_first() {
        let a = LwwValue { value: "a", ts: 1, node: 2 };
        let b = LwwValue { value: "b", ts: 2, node: 1 };
        assert_eq!(LwwValue::resolve(&a, &b).value, "b");
    }

    #[test]
    fn lww_tie_breaks_by_node_id() {
        let a = LwwValue { value: "a", ts: 5, node: 1 };
        let b = LwwValue { value: "b", ts: 5, node: 7 };
        assert_eq!(LwwValue::resolve(&a, &b).value, "a");
    }
}
