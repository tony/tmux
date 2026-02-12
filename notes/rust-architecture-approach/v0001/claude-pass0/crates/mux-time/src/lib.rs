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

use std::collections::BTreeMap;

/// A Lamport logical clock.
///
/// Lamport clocks provide a total ordering of events:
/// - Local event: increment.
/// - Send: increment, attach timestamp.
/// - Receive: max(local, received) + 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LamportClock {
    counter: u64,
}

impl LamportClock {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    /// Get the current timestamp.
    pub fn now(&self) -> u64 {
        self.counter
    }

    /// Increment for a local event.
    pub fn tick(&mut self) -> u64 {
        self.counter += 1;
        self.counter
    }

    /// Update on receiving a remote timestamp.
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
/// Provides causal ordering for distributed events (CRDT collaboration).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorClock {
    clocks: BTreeMap<u32, u64>,
}

impl VectorClock {
    pub fn new() -> Self {
        Self { clocks: BTreeMap::new() }
    }

    /// Increment the counter for the given actor.
    pub fn increment(&mut self, actor: u32) {
        let counter = self.clocks.entry(actor).or_insert(0);
        *counter += 1;
    }

    /// Get the counter for an actor (0 if unknown).
    pub fn get(&self, actor: u32) -> u64 {
        self.clocks.get(&actor).copied().unwrap_or(0)
    }

    /// Merge another vector clock (component-wise max).
    pub fn merge(&mut self, other: &VectorClock) {
        for (&actor, &count) in &other.clocks {
            let entry = self.clocks.entry(actor).or_insert(0);
            *entry = (*entry).max(count);
        }
    }

    /// True if self causally dominates other.
    pub fn dominates(&self, other: &VectorClock) -> bool {
        for (&actor, &count) in &other.clocks {
            if self.get(actor) < count {
                return false;
            }
        }
        true
    }

    /// True if events are concurrent (neither dominates).
    pub fn concurrent_with(&self, other: &VectorClock) -> bool {
        !self.dominates(other) && !other.dominates(self)
    }

    /// Number of known actors.
    pub fn actor_count(&self) -> usize {
        self.clocks.len()
    }
}

impl Default for VectorClock {
    fn default() -> Self {
        Self::new()
    }
}

/// S97: DeterministicTimeSource combining wall clock, Lamport, and vector clock.
///
/// In production, wall_time_ms comes from the system clock.
/// In tests, it is injected for deterministic replay (INV-033).
#[derive(Debug, Clone)]
pub struct DeterministicTimeSource {
    /// Wall clock time in milliseconds since epoch.
    wall_time_ms: u64,
    /// Lamport logical clock.
    lamport: LamportClock,
    /// Vector clock for this node.
    vector: VectorClock,
    /// This node's actor ID.
    actor_id: u32,
    /// True if using injected time (test mode).
    is_deterministic: bool,
}

impl DeterministicTimeSource {
    /// Create a production time source.
    pub fn new(actor_id: u32) -> Self {
        Self {
            wall_time_ms: Self::system_time_ms(),
            lamport: LamportClock::new(),
            vector: VectorClock::new(),
            actor_id,
            is_deterministic: false,
        }
    }

    /// Create a deterministic time source for testing.
    /// INV-033: Fixed seed -> deterministic timestamps.
    pub fn deterministic(actor_id: u32, start_time_ms: u64) -> Self {
        Self {
            wall_time_ms: start_time_ms,
            lamport: LamportClock::new(),
            vector: VectorClock::new(),
            actor_id,
            is_deterministic: true,
        }
    }

    /// Get the current wall time.
    pub fn wall_time_ms(&self) -> u64 {
        self.wall_time_ms
    }

    /// Get the current Lamport timestamp.
    pub fn lamport_time(&self) -> u64 {
        self.lamport.now()
    }

    /// Get the vector clock.
    pub fn vector_clock(&self) -> &VectorClock {
        &self.vector
    }

    /// Record a local event: advance wall time and Lamport clock.
    pub fn tick(&mut self) -> u64 {
        if self.is_deterministic {
            self.wall_time_ms += 1; // advance by 1ms in deterministic mode
        } else {
            self.wall_time_ms = Self::system_time_ms();
        }
        self.vector.increment(self.actor_id);
        self.lamport.tick()
    }

    /// Process a remote event: merge clocks.
    pub fn receive(&mut self, remote_lamport: u64, remote_vector: &VectorClock) {
        self.lamport.receive(remote_lamport);
        self.vector.merge(remote_vector);
        self.vector.increment(self.actor_id);
    }

    /// Whether this is deterministic (test) mode.
    pub fn is_deterministic(&self) -> bool {
        self.is_deterministic
    }

    /// System time helper.
    fn system_time_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

/// MonotonicGuard: wraps a time source and ensures timestamps never decrease.
///
/// If the wall clock goes backward (NTP adjustment, VM migration, etc.),
/// the guard holds at the last-seen timestamp.
#[derive(Debug, Clone)]
pub struct MonotonicGuard {
    last_time_ms: u64,
}

impl MonotonicGuard {
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

    /// INV-025: Vector clock monotonic per node.
    #[test]
    fn test_vector_clock_monotonic() {
        let mut vc = VectorClock::new();
        vc.increment(1);
        vc.increment(1);
        assert_eq!(vc.get(1), 2);
        // Counter only goes up
    }

    /// Vector clock merge is commutative.
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

    /// MonotonicGuard never goes backward.
    #[test]
    fn test_monotonic_guard() {
        let mut guard = MonotonicGuard::new();
        assert_eq!(guard.monotonic(100), 100);
        assert_eq!(guard.monotonic(50), 100); // does not go backward
        assert_eq!(guard.monotonic(200), 200);
    }
}
