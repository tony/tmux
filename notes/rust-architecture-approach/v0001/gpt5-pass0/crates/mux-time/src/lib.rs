#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Unified deterministic time source (S97, INV-033).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterministicTimeSource {
    wall_millis: u64,
    lamport: u64,
    vector: BTreeMap<String, u64>,
}

impl DeterministicTimeSource {
    pub fn new(start_wall_millis: u64) -> Self {
        Self {
            wall_millis: start_wall_millis,
            lamport: 0,
            vector: BTreeMap::new(),
        }
    }

    pub fn now_wall_millis(&self) -> u64 {
        self.wall_millis
    }

    pub fn lamport(&self) -> u64 {
        self.lamport
    }

    pub fn vector(&self) -> &BTreeMap<String, u64> {
        &self.vector
    }

    pub fn tick_wall(&mut self, delta_millis: u64) {
        self.wall_millis = self.wall_millis.saturating_add(delta_millis);
    }

    pub fn event(&mut self, node: &str) {
        self.lamport = self.lamport.saturating_add(1);
        let counter = self.vector.entry(node.to_string()).or_insert(0);
        *counter = counter.saturating_add(1);
    }

    pub fn merge(&mut self, other_lamport: u64, other_vector: &BTreeMap<String, u64>) {
        self.lamport = self.lamport.max(other_lamport).saturating_add(1);
        for (node, counter) in other_vector {
            let local = self.vector.entry(node.clone()).or_insert(0);
            *local = (*local).max(*counter);
        }
    }
}

impl Default for DeterministicTimeSource {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wall_tick_is_deterministic() {
        let mut ts = DeterministicTimeSource::new(100);
        ts.tick_wall(7);
        ts.tick_wall(3);
        assert_eq!(ts.now_wall_millis(), 110);
    }

    #[test]
    fn lamport_and_vector_advance() {
        let mut ts = DeterministicTimeSource::default();
        ts.event("a");
        ts.event("a");
        assert_eq!(ts.lamport(), 2);
        assert_eq!(ts.vector().get("a"), Some(&2));
    }

    #[test]
    fn merge_is_monotonic() {
        let mut a = DeterministicTimeSource::default();
        let mut b = DeterministicTimeSource::default();
        a.event("a");
        b.event("b");
        b.event("b");
        a.merge(b.lamport(), b.vector());
        assert!(a.lamport() >= 3);
        assert_eq!(a.vector().get("b"), Some(&2));
    }

    #[test]
    fn vector_is_sorted_map() {
        let mut ts = DeterministicTimeSource::default();
        ts.event("z");
        ts.event("a");
        let keys = ts.vector().keys().cloned().collect::<Vec<_>>();
        assert_eq!(keys, vec!["a".to_string(), "z".to_string()]);
    }
}
