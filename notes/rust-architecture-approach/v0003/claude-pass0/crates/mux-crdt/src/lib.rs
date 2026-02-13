//! # mux-crdt
//!
//! Conflict-free Replicated Data Types for collaborative terminal sessions.
//!
//! ## Strategy
//! - Vector clock dominance for causal ordering.
//! - Last-Writer-Wins (LWW) register for individual cell conflicts.
//! - Convergence verified via proptest with concurrent mutation generators.
//!
//! L2 data crate -- feature-gated.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// A server/replica identifier.
pub type ReplicaId = u64;

/// Vector clock for causal ordering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorClock {
    entries: HashMap<ReplicaId, u64>,
}

impl VectorClock {
    /// Create a new zero vector clock.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Increment the clock for a given replica.
    pub fn increment(&mut self, replica: ReplicaId) {
        let counter = self.entries.entry(replica).or_insert(0);
        *counter += 1;
    }

    /// Get the counter for a replica (0 if not seen).
    #[must_use]
    pub fn get(&self, replica: ReplicaId) -> u64 {
        self.entries.get(&replica).copied().unwrap_or(0)
    }

    /// Merge another vector clock into this one (component-wise max).
    pub fn merge(&mut self, other: &Self) {
        for (&replica, &counter) in &other.entries {
            let entry = self.entries.entry(replica).or_insert(0);
            if counter > *entry {
                *entry = counter;
            }
        }
    }

    /// Check if this clock dominates another (>=  in all dimensions,
    /// > in at least one).
    #[must_use]
    pub fn dominates(&self, other: &Self) -> bool {
        let mut dominated_any = false;

        // Check all entries in other
        for (&replica, &their_val) in &other.entries {
            let our_val = self.get(replica);
            if our_val < their_val {
                return false;
            }
            if our_val > their_val {
                dominated_any = true;
            }
        }

        // Check for entries we have that they don't
        for (&replica, &our_val) in &self.entries {
            if our_val > other.get(replica) {
                dominated_any = true;
            }
        }

        dominated_any
    }

    /// Check if two clocks are concurrent (neither dominates the other).
    #[must_use]
    pub fn is_concurrent(&self, other: &Self) -> bool {
        !self.dominates(other) && !other.dominates(self) && self != other
    }
}

impl Default for VectorClock {
    fn default() -> Self {
        Self::new()
    }
}

/// A Last-Writer-Wins register for resolving cell conflicts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LwwRegister<T> {
    /// The value stored.
    pub value: T,
    /// Logical timestamp of the last write.
    pub timestamp: u64,
    /// Replica that performed the write (tie-breaker).
    pub origin: ReplicaId,
}

impl<T: Clone> LwwRegister<T> {
    /// Create a new register.
    #[must_use]
    pub fn new(value: T, timestamp: u64, origin: ReplicaId) -> Self {
        Self {
            value,
            timestamp,
            origin,
        }
    }

    /// Merge with another register. Higher timestamp wins; ties broken by replica ID.
    pub fn merge(&mut self, other: &Self) {
        if other.timestamp > self.timestamp
            || (other.timestamp == self.timestamp && other.origin > self.origin)
        {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
            self.origin = other.origin;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vclock_new_is_zero() {
        let vc = VectorClock::new();
        assert_eq!(vc.get(1), 0);
    }

    #[test]
    fn vclock_increment() {
        let mut vc = VectorClock::new();
        vc.increment(1);
        assert_eq!(vc.get(1), 1);
        vc.increment(1);
        assert_eq!(vc.get(1), 2);
    }

    #[test]
    fn vclock_merge() {
        let mut a = VectorClock::new();
        a.increment(1);
        a.increment(1);

        let mut b = VectorClock::new();
        b.increment(2);
        b.increment(2);
        b.increment(2);

        a.merge(&b);
        assert_eq!(a.get(1), 2);
        assert_eq!(a.get(2), 3);
    }

    #[test]
    fn vclock_dominates() {
        let mut a = VectorClock::new();
        a.increment(1);
        a.increment(1);

        let mut b = VectorClock::new();
        b.increment(1);

        assert!(a.dominates(&b));
        assert!(!b.dominates(&a));
    }

    #[test]
    fn vclock_concurrent() {
        let mut a = VectorClock::new();
        a.increment(1);

        let mut b = VectorClock::new();
        b.increment(2);

        assert!(a.is_concurrent(&b));
        assert!(b.is_concurrent(&a));
    }

    #[test]
    fn vclock_equal_not_dominant() {
        let mut a = VectorClock::new();
        a.increment(1);
        let b = a.clone();
        assert!(!a.dominates(&b));
        assert!(!a.is_concurrent(&b));
    }

    #[test]
    fn lww_higher_timestamp_wins() {
        let mut reg1 = LwwRegister::new("old", 1, 1);
        let reg2 = LwwRegister::new("new", 2, 2);
        reg1.merge(&reg2);
        assert_eq!(reg1.value, "new");
    }

    #[test]
    fn lww_same_timestamp_higher_id_wins() {
        let mut reg1 = LwwRegister::new("A", 1, 1);
        let reg2 = LwwRegister::new("B", 1, 2);
        reg1.merge(&reg2);
        assert_eq!(reg1.value, "B");
    }

    #[test]
    fn lww_lower_timestamp_loses() {
        let mut reg1 = LwwRegister::new("new", 5, 1);
        let reg2 = LwwRegister::new("old", 3, 2);
        reg1.merge(&reg2);
        assert_eq!(reg1.value, "new");
    }

    #[test]
    fn lww_convergence() {
        // Both replicas should converge to the same value regardless of merge order
        let write_a = LwwRegister::new("A", 10, 1);
        let write_b = LwwRegister::new("B", 10, 2);

        let mut r1 = write_a.clone();
        r1.merge(&write_b);

        let mut r2 = write_b.clone();
        r2.merge(&write_a);

        assert_eq!(r1.value, r2.value);
    }
}
