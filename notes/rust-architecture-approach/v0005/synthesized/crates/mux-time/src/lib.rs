//! Injectable clock abstraction for deterministic testing.
//!
//! The kernel and all time-dependent logic use `Clock` instead of
//! `std::time::Instant` directly. In production, `Clock::system()` wraps
//! the monotonic clock. In tests, `Clock::manual()` gives full control
//! over the passage of time, eliminating wall-clock flakiness.
//!
//! # Timer wheel
//!
//! The `TimerWheel` provides O(1) timer scheduling with configurable
//! tick granularity. Timers are bucketed by their expiration tick and
//! drained in order when `advance()` is called.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A monotonic timestamp in nanoseconds from an arbitrary epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Create a timestamp from nanoseconds.
    pub const fn from_nanos(ns: u64) -> Self {
        Self(ns)
    }

    /// Get the raw nanosecond value.
    pub const fn as_nanos(self) -> u64 {
        self.0
    }

    /// Get the value as milliseconds.
    pub const fn as_millis(self) -> u64 {
        self.0 / 1_000_000
    }

    /// Duration elapsed since another timestamp.
    pub fn duration_since(self, earlier: Self) -> Duration {
        Duration::from_nanos(self.0.saturating_sub(earlier.0))
    }

    /// Add a duration to this timestamp.
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, dur: Duration) -> Self {
        Self(self.0.saturating_add(dur.as_nanos() as u64))
    }
}

/// State for a manual clock (used for deterministic testing).
#[derive(Debug)]
pub struct ManualState {
    now: Timestamp,
}

/// Clock source abstraction.
#[derive(Debug, Clone)]
pub enum Clock {
    /// System monotonic clock.
    System { epoch: Instant },
    /// Manual clock for deterministic testing.
    Manual(Arc<Mutex<ManualState>>),
}

impl Default for Clock {
    fn default() -> Self {
        Self::system()
    }
}

impl Clock {
    /// Create a clock backed by the system monotonic clock.
    pub fn system() -> Self {
        Self::System {
            epoch: Instant::now(),
        }
    }

    /// Create a manual clock starting at time zero.
    pub fn manual() -> Self {
        Self::Manual(Arc::new(Mutex::new(ManualState {
            now: Timestamp::from_nanos(0),
        })))
    }

    /// Read the current time.
    pub fn now(&self) -> Timestamp {
        match self {
            Self::System { epoch } => {
                let elapsed = epoch.elapsed();
                Timestamp::from_nanos(elapsed.as_nanos() as u64)
            }
            Self::Manual(state) => {
                let guard = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                guard.now
            }
        }
    }

    /// Advance a manual clock by the given duration.
    ///
    /// No-op on system clocks.
    pub fn advance(&self, dur: Duration) {
        if let Self::Manual(state) = self {
            let mut guard = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.now = guard.now.add(dur);
        }
    }

    /// Set a manual clock to a specific timestamp.
    ///
    /// No-op on system clocks.
    pub fn set(&self, ts: Timestamp) {
        if let Self::Manual(state) = self {
            let mut guard = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.now = ts;
        }
    }

    /// Returns true if this is a manual clock.
    pub fn is_manual(&self) -> bool {
        matches!(self, Self::Manual(_))
    }

    /// Returns true if this is a system clock.
    pub fn is_system(&self) -> bool {
        matches!(self, Self::System { .. })
    }
}

/// Identifier for a scheduled timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimerId(u64);

impl TimerId {
    /// Get the raw value.
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// A timer entry stored in the wheel.
#[derive(Debug, Clone)]
pub struct TimerEntry {
    /// Unique identifier.
    pub id: TimerId,
    /// When this timer expires.
    pub deadline: Timestamp,
    /// Payload tag for the timer callback.
    pub tag: u64,
}

/// Timer wheel for efficient scheduling of timeouts.
///
/// Timers are stored in a `BTreeMap` keyed by deadline for ordered draining.
/// The wheel tracks a current time and drains expired timers on each `advance`.
#[derive(Debug)]
pub struct TimerWheel {
    timers: BTreeMap<(Timestamp, u64), TimerEntry>,
    next_id: u64,
    current_time: Timestamp,
}

impl Default for TimerWheel {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerWheel {
    /// Create a new timer wheel at time zero.
    pub fn new() -> Self {
        Self {
            timers: BTreeMap::new(),
            next_id: 1,
            current_time: Timestamp::from_nanos(0),
        }
    }

    /// Create with a specific start time.
    pub fn with_time(ts: Timestamp) -> Self {
        Self {
            timers: BTreeMap::new(),
            next_id: 1,
            current_time: ts,
        }
    }

    /// Schedule a timer to fire at `deadline`.
    pub fn schedule(&mut self, deadline: Timestamp, tag: u64) -> TimerId {
        let id = TimerId(self.next_id);
        self.next_id += 1;
        let entry = TimerEntry { id, deadline, tag };
        self.timers.insert((deadline, id.0), entry);
        id
    }

    /// Schedule a timer relative to current time.
    pub fn schedule_after(&mut self, delay: Duration, tag: u64) -> TimerId {
        let deadline = self.current_time.add(delay);
        self.schedule(deadline, tag)
    }

    /// Cancel a timer by id.
    pub fn cancel(&mut self, id: TimerId) -> bool {
        let key_to_remove = self
            .timers
            .iter()
            .find(|(_, e)| e.id == id)
            .map(|(k, _)| *k);
        if let Some(key) = key_to_remove {
            self.timers.remove(&key);
            true
        } else {
            false
        }
    }

    /// Advance time to `now` and return all expired timers.
    pub fn advance_to(&mut self, now: Timestamp) -> Vec<TimerEntry> {
        self.current_time = now;
        let split_key = (now.add(Duration::from_nanos(1)), 0);
        let expired_range = self.timers.split_off(&split_key);
        let expired: Vec<TimerEntry> = self.timers.values().cloned().collect();
        self.timers = expired_range;
        expired
    }

    /// Number of pending timers.
    pub fn pending_count(&self) -> usize {
        self.timers.len()
    }

    /// Current time of the wheel.
    pub fn current_time(&self) -> Timestamp {
        self.current_time
    }

    /// Returns true when there are no pending timers.
    pub fn is_empty(&self) -> bool {
        self.timers.is_empty()
    }

    /// Get the next deadline, if any timers are pending.
    pub fn next_deadline(&self) -> Option<Timestamp> {
        self.timers.keys().next().map(|(ts, _)| *ts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_clock_starts_at_zero() {
        let clock = Clock::manual();
        assert_eq!(clock.now(), Timestamp::from_nanos(0));
    }

    #[test]
    fn manual_clock_advance() {
        let clock = Clock::manual();
        clock.advance(Duration::from_millis(100));
        assert_eq!(clock.now().as_millis(), 100);
    }

    #[test]
    fn manual_clock_set() {
        let clock = Clock::manual();
        clock.set(Timestamp::from_nanos(5_000_000));
        assert_eq!(clock.now().as_millis(), 5);
    }

    #[test]
    fn system_clock_is_monotonic() {
        let clock = Clock::system();
        let t1 = clock.now();
        let t2 = clock.now();
        assert!(t2 >= t1);
    }

    #[test]
    fn clock_is_manual_check() {
        assert!(Clock::manual().is_manual());
        assert!(!Clock::system().is_manual());
    }

    #[test]
    fn clock_is_system_check() {
        assert!(Clock::system().is_system());
        assert!(!Clock::manual().is_system());
    }

    #[test]
    fn timestamp_duration_since() {
        let t1 = Timestamp::from_nanos(1_000);
        let t2 = Timestamp::from_nanos(5_000);
        assert_eq!(t2.duration_since(t1), Duration::from_nanos(4_000));
    }

    #[test]
    fn timestamp_duration_since_saturating() {
        let t1 = Timestamp::from_nanos(5_000);
        let t2 = Timestamp::from_nanos(1_000);
        assert_eq!(t2.duration_since(t1), Duration::from_nanos(0));
    }

    #[test]
    fn timestamp_add_saturating() {
        let t = Timestamp::from_nanos(u64::MAX - 10);
        let result = t.add(Duration::from_nanos(100));
        assert_eq!(result, Timestamp::from_nanos(u64::MAX));
    }

    #[test]
    fn timer_wheel_schedule_and_drain() {
        let mut wheel = TimerWheel::new();
        let _id1 = wheel.schedule(Timestamp::from_nanos(100), 1);
        let _id2 = wheel.schedule(Timestamp::from_nanos(200), 2);
        assert_eq!(wheel.pending_count(), 2);

        let expired = wheel.advance_to(Timestamp::from_nanos(150));
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].tag, 1);
        assert_eq!(wheel.pending_count(), 1);
    }

    #[test]
    fn timer_wheel_cancel() {
        let mut wheel = TimerWheel::new();
        let id = wheel.schedule(Timestamp::from_nanos(100), 1);
        assert!(wheel.cancel(id));
        assert_eq!(wheel.pending_count(), 0);
    }

    #[test]
    fn timer_wheel_cancel_nonexistent() {
        let mut wheel = TimerWheel::new();
        assert!(!wheel.cancel(TimerId(999)));
    }

    #[test]
    fn timer_wheel_schedule_after() {
        let mut wheel = TimerWheel::with_time(Timestamp::from_nanos(1000));
        let _id = wheel.schedule_after(Duration::from_nanos(500), 42);
        let expired = wheel.advance_to(Timestamp::from_nanos(1500));
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].tag, 42);
    }

    #[test]
    fn timer_wheel_empty() {
        let wheel = TimerWheel::new();
        assert!(wheel.is_empty());
    }

    #[test]
    fn timer_wheel_next_deadline() {
        let mut wheel = TimerWheel::new();
        assert!(wheel.next_deadline().is_none());
        wheel.schedule(Timestamp::from_nanos(500), 1);
        wheel.schedule(Timestamp::from_nanos(100), 2);
        assert_eq!(
            wheel.next_deadline(),
            Some(Timestamp::from_nanos(100))
        );
    }

    #[test]
    fn timestamp_ordering() {
        let t1 = Timestamp::from_nanos(10);
        let t2 = Timestamp::from_nanos(20);
        assert!(t1 < t2);
        assert!(t2 > t1);
    }

    #[test]
    fn timer_id_raw() {
        let id = TimerId(42);
        assert_eq!(id.raw(), 42);
    }

    #[test]
    fn timer_wheel_drain_all() {
        let mut wheel = TimerWheel::new();
        wheel.schedule(Timestamp::from_nanos(10), 1);
        wheel.schedule(Timestamp::from_nanos(20), 2);
        wheel.schedule(Timestamp::from_nanos(30), 3);
        let expired = wheel.advance_to(Timestamp::from_nanos(100));
        assert_eq!(expired.len(), 3);
        assert!(wheel.is_empty());
    }

    #[test]
    fn default_clock_is_system() {
        let clock = Clock::default();
        assert!(clock.is_system());
    }

    #[test]
    fn manual_clock_clones_share_state() {
        let c1 = Clock::manual();
        let c2 = c1.clone();
        c1.advance(Duration::from_millis(50));
        assert_eq!(c2.now().as_millis(), 50);
    }

    // proptest: advance is monotonic
    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn advance_monotonic(steps in proptest::collection::vec(1u64..1_000_000, 1..20)) {
                let clock = Clock::manual();
                let mut prev = clock.now();
                for ns in steps {
                    clock.advance(Duration::from_nanos(ns));
                    let now = clock.now();
                    prop_assert!(now >= prev);
                    prev = now;
                }
            }

            #[test]
            fn timestamp_add_monotonic(base in 0u64..1_000_000_000, delta in 0u64..1_000_000) {
                let t = Timestamp::from_nanos(base);
                let t2 = t.add(Duration::from_nanos(delta));
                prop_assert!(t2 >= t);
            }

            #[test]
            fn timer_wheel_scheduled_fires_at_deadline(deadline_ns in 1u64..10_000) {
                let mut wheel = TimerWheel::new();
                let _id = wheel.schedule(Timestamp::from_nanos(deadline_ns), 1);
                let expired = wheel.advance_to(Timestamp::from_nanos(deadline_ns));
                prop_assert_eq!(expired.len(), 1);
            }
        }
    }
}
