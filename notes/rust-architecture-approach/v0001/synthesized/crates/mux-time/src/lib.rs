//! # mux-time
//!
//! Deterministic time source combining wall clock, Lamport clock,
//! and vector clock for testable, reproducible causal ordering.
//!
//! ## Key Design (S97)
//! - `DeterministicTimeSource`: injectable time for testing.
//! - `MonotonicGuard`: ensures timestamps never go backward.
//! - Wall time + Lamport clock + vector clock in a single struct.
//! - INV-025: VectorClock entries are monotonically non-decreasing per node.
//! - INV-033: Deterministic replay under fixed clock.

#![forbid(unsafe_code)]

#[cfg(feature = "crdt")]
use std::collections::BTreeMap;

/// Ergonomic re-exports for downstream crates.
pub mod prelude {
    #[cfg(feature = "crdt")]
    pub use super::VectorClock;
    pub use super::{DeterministicTimeSource, LamportClock, MonotonicGuard};
}

/// A Lamport logical clock.
///
/// Lamport clocks provide a total ordering of events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LamportClock {
    counter: u64,
}

impl LamportClock {
    #[must_use]
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    #[must_use]
    pub fn now(&self) -> u64 {
        self.counter
    }

    pub fn tick(&mut self) -> u64 {
        self.counter += 1;
        self.counter
    }

    pub fn receive(&mut self, remote: u64) -> u64 {
        self.counter = self.counter.max(remote) + 1;
        self.counter
    }
}

impl Default for LamportClock {
    fn default() -> Self {
        Self::new()
    }
}

/// INV-025: VectorClock with monotonically non-decreasing entries per node.
///
/// Provides causal ordering for distributed events.
/// Feature-gated under `crdt` for CRDT collaboration.
#[cfg(feature = "crdt")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorClock {
    clocks: BTreeMap<u32, u64>,
}

#[cfg(feature = "crdt")]
impl VectorClock {
    #[must_use]
    pub fn new() -> Self {
        Self { clocks: BTreeMap::new() }
    }

    pub fn increment(&mut self, actor: u32) {
        let counter = self.clocks.entry(actor).or_insert(0);
        *counter += 1;
    }

    #[must_use]
    pub fn get(&self, actor: u32) -> u64 {
        self.clocks.get(&actor).copied().unwrap_or(0)
    }

    pub fn merge(&mut self, other: &VectorClock) {
        for (&actor, &count) in &other.clocks {
            let entry = self.clocks.entry(actor).or_insert(0);
            *entry = (*entry).max(count);
        }
    }

    #[must_use]
    pub fn dominates(&self, other: &VectorClock) -> bool {
        for (&actor, &count) in &other.clocks {
            if self.get(actor) < count {
                return false;
            }
        }
        true
    }

    #[must_use]
    pub fn concurrent_with(&self, other: &VectorClock) -> bool {
        !self.dominates(other) && !other.dominates(self)
    }

    #[must_use]
    pub fn actor_count(&self) -> usize {
        self.clocks.len()
    }

    /// True if the clock for the given actor has ever been incremented.
    #[must_use]
    pub fn has_actor(&self, actor: u32) -> bool {
        self.clocks.contains_key(&actor)
    }
}

#[cfg(feature = "crdt")]
impl Default for VectorClock {
    fn default() -> Self {
        Self::new()
    }
}

/// S97: DeterministicTimeSource combining wall clock, Lamport, and vector clock.
///
/// INV-033: Fixed seed -> deterministic timestamps.
#[derive(Debug, Clone)]
pub struct DeterministicTimeSource {
    wall_time_ms: u64,
    lamport: LamportClock,
    #[cfg(feature = "crdt")]
    vector: VectorClock,
    actor_id: u32,
    is_deterministic: bool,
}

impl DeterministicTimeSource {
    /// Create a production time source.
    #[must_use]
    pub fn new(actor_id: u32) -> Self {
        Self {
            wall_time_ms: Self::system_time_ms(),
            lamport: LamportClock::new(),
            #[cfg(feature = "crdt")]
            vector: VectorClock::new(),
            actor_id,
            is_deterministic: false,
        }
    }

    /// Create a deterministic time source for testing.
    #[must_use]
    pub fn deterministic(actor_id: u32, start_time_ms: u64) -> Self {
        Self {
            wall_time_ms: start_time_ms,
            lamport: LamportClock::new(),
            #[cfg(feature = "crdt")]
            vector: VectorClock::new(),
            actor_id,
            is_deterministic: true,
        }
    }

    #[must_use]
    pub fn wall_time_ms(&self) -> u64 {
        self.wall_time_ms
    }

    #[must_use]
    pub fn lamport_time(&self) -> u64 {
        self.lamport.now()
    }

    #[cfg(feature = "crdt")]
    #[must_use]
    pub fn vector_clock(&self) -> &VectorClock {
        &self.vector
    }

    #[must_use]
    pub fn actor_id(&self) -> u32 {
        self.actor_id
    }

    /// Record a local event: advance wall time and Lamport clock.
    pub fn tick(&mut self) -> u64 {
        if self.is_deterministic {
            self.wall_time_ms += 1;
        } else {
            self.wall_time_ms = Self::system_time_ms();
        }
        #[cfg(feature = "crdt")]
        self.vector.increment(self.actor_id);
        self.lamport.tick()
    }

    /// Process a remote event: merge clocks (CRDT-aware version).
    #[cfg(feature = "crdt")]
    pub fn receive(&mut self, remote_lamport: u64, remote_vector: &VectorClock) {
        self.lamport.receive(remote_lamport);
        self.vector.merge(remote_vector);
        self.vector.increment(self.actor_id);
    }

    /// Process a remote event: merge Lamport clock only (non-CRDT version).
    #[cfg(not(feature = "crdt"))]
    pub fn receive_lamport(&mut self, remote_lamport: u64) {
        self.lamport.receive(remote_lamport);
    }

    #[must_use]
    pub fn is_deterministic(&self) -> bool {
        self.is_deterministic
    }

    fn system_time_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

/// MonotonicGuard: wraps a time source and ensures timestamps never decrease.
#[derive(Debug, Clone)]
pub struct MonotonicGuard {
    last_time_ms: u64,
}

impl MonotonicGuard {
    #[must_use]
    pub fn new() -> Self {
        Self { last_time_ms: 0 }
    }

    /// Get monotonic time, never less than previous call.
    pub fn monotonic(&mut self, current_ms: u64) -> u64 {
        if current_ms > self.last_time_ms {
            self.last_time_ms = current_ms;
        }
        self.last_time_ms
    }

    #[must_use]
    pub fn last(&self) -> u64 {
        self.last_time_ms
    }
}

impl Default for MonotonicGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lamport_tick() {
        let mut lc = LamportClock::new();
        assert_eq!(lc.now(), 0);
        assert_eq!(lc.tick(), 1);
        assert_eq!(lc.tick(), 2);
    }

    #[test]
    fn test_lamport_receive() {
        let mut lc = LamportClock::new();
        lc.tick(); // counter = 1
        let ts = lc.receive(10); // max(1, 10) + 1 = 11
        assert_eq!(ts, 11);
    }

    #[test]
    fn test_lamport_receive_lower() {
        let mut lc = LamportClock::new();
        lc.tick(); // 1
        lc.tick(); // 2
        lc.tick(); // 3
        let ts = lc.receive(1); // max(3, 1) + 1 = 4
        assert_eq!(ts, 4);
    }

    /// INV-025: Vector clock monotonic per node.
    #[cfg(feature = "crdt")]
    #[test]
    fn test_vector_clock_monotonic() {
        let mut vc = VectorClock::new();
        vc.increment(1);
        vc.increment(1);
        assert_eq!(vc.get(1), 2);
        // Counter only goes up
        vc.increment(1);
        assert_eq!(vc.get(1), 3);
    }

    /// Vector clock merge is commutative.
    #[cfg(feature = "crdt")]
    #[test]
    fn test_vector_clock_merge_commutative() {
        let mut a = VectorClock::new();
        a.increment(1);
        let mut b = VectorClock::new();
        b.increment(2);

        let mut ab = a.clone();
        ab.merge(&b);
        let mut ba = b.clone();
        ba.merge(&a);
        assert_eq!(ab, ba);
    }

    /// Vector clock dominance.
    #[cfg(feature = "crdt")]
    #[test]
    fn test_vector_clock_dominates() {
        let mut a = VectorClock::new();
        a.increment(1);
        a.increment(1);
        let mut b = VectorClock::new();
        b.increment(1);
        assert!(a.dominates(&b));
        assert!(!b.dominates(&a));
    }

    /// Concurrent events detected.
    #[cfg(feature = "crdt")]
    #[test]
    fn test_vector_clock_concurrent() {
        let mut a = VectorClock::new();
        a.increment(1);
        let mut b = VectorClock::new();
        b.increment(2);
        assert!(a.concurrent_with(&b));
    }

    /// S97: Deterministic time source advances predictably.
    #[test]
    fn test_deterministic_time_source() {
        let mut ts = DeterministicTimeSource::deterministic(1, 1000);
        assert!(ts.is_deterministic());
        assert_eq!(ts.wall_time_ms(), 1000);

        ts.tick();
        assert_eq!(ts.wall_time_ms(), 1001);
        assert_eq!(ts.lamport_time(), 1);

        ts.tick();
        assert_eq!(ts.wall_time_ms(), 1002);
        assert_eq!(ts.lamport_time(), 2);
    }

    /// Deterministic time source tracks vector clock.
    #[cfg(feature = "crdt")]
    #[test]
    fn test_deterministic_time_vector_clock() {
        let mut ts = DeterministicTimeSource::deterministic(1, 0);
        ts.tick();
        assert_eq!(ts.vector_clock().get(1), 1);
        ts.tick();
        assert_eq!(ts.vector_clock().get(1), 2);
    }

    /// Receive merges remote clocks.
    #[cfg(feature = "crdt")]
    #[test]
    fn test_deterministic_time_receive() {
        let mut ts = DeterministicTimeSource::deterministic(1, 0);
        ts.tick();

        let mut remote_vc = VectorClock::new();
        remote_vc.increment(2);
        remote_vc.increment(2);
        remote_vc.increment(2);

        ts.receive(10, &remote_vc);
        assert_eq!(ts.lamport_time(), 11); // max(1, 10) + 1
        assert_eq!(ts.vector_clock().get(2), 3);
    }

    /// MonotonicGuard never goes backward.
    #[test]
    fn test_monotonic_guard() {
        let mut guard = MonotonicGuard::new();
        assert_eq!(guard.monotonic(100), 100);
        assert_eq!(guard.monotonic(50), 100); // does not go backward
        assert_eq!(guard.monotonic(200), 200);
        assert_eq!(guard.last(), 200);
    }

    /// MonotonicGuard starts at 0.
    #[test]
    fn test_monotonic_guard_initial() {
        let guard = MonotonicGuard::new();
        assert_eq!(guard.last(), 0);
    }

    /// VectorClock has_actor.
    #[cfg(feature = "crdt")]
    #[test]
    fn test_vector_clock_has_actor() {
        let mut vc = VectorClock::new();
        assert!(!vc.has_actor(1));
        vc.increment(1);
        assert!(vc.has_actor(1));
        assert!(!vc.has_actor(2));
    }
}
