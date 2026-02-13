//! # mux-time
//!
//! Injectable clock abstraction for the TermForge kernel. Using `Clock::manual()`
//! in tests eliminates all wall-clock dependencies and makes the kernel fully
//! deterministic.
//!
//! L0 leaf crate -- no internal dependencies.

#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A monotonic timestamp, either real or synthetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Create a timestamp from nanoseconds since an arbitrary epoch.
    #[must_use]
    pub const fn from_nanos(ns: u64) -> Self {
        Self(ns)
    }

    /// Returns the raw nanosecond value.
    #[must_use]
    pub const fn as_nanos(&self) -> u64 {
        self.0
    }

    /// Returns milliseconds since epoch.
    #[must_use]
    pub const fn as_millis(&self) -> u64 {
        self.0 / 1_000_000
    }

    /// Duration elapsed since another (earlier) timestamp.
    #[must_use]
    pub const fn duration_since(&self, earlier: Self) -> Duration {
        Duration::from_nanos(self.0.saturating_sub(earlier.0))
    }

    /// Add a duration to this timestamp.
    #[must_use]
    pub const fn add(self, duration: Duration) -> Self {
        Self(self.0.saturating_add(duration.as_nanos() as u64))
    }

    /// The zero timestamp.
    pub const ZERO: Self = Self(0);
}

/// Shared state for manual clock advancement.
#[derive(Debug)]
struct ManualState {
    current_ns: u64,
}

/// Injectable clock for deterministic testing.
///
/// Production code uses `Clock::system()` which delegates to `Instant::now()`.
/// Test code uses `Clock::manual()` which returns a controllable synthetic time.
#[derive(Debug, Clone)]
pub enum Clock {
    /// System clock backed by `std::time::Instant`.
    System {
        /// The instant captured at Clock creation, used as epoch reference.
        epoch: Instant,
    },
    /// Manual clock for deterministic tests.
    Manual {
        /// Shared mutable state for time advancement.
        state: Arc<Mutex<ManualState>>,
    },
}

impl Clock {
    /// Create a system clock that uses real monotonic time.
    #[must_use]
    pub fn system() -> Self {
        Self::System {
            epoch: Instant::now(),
        }
    }

    /// Create a manual clock starting at time zero.
    /// Use `advance()` to move time forward deterministically.
    #[must_use]
    pub fn manual() -> Self {
        Self::Manual {
            state: Arc::new(Mutex::new(ManualState { current_ns: 0 })),
        }
    }

    /// Read the current timestamp.
    #[must_use]
    pub fn now(&self) -> Timestamp {
        match self {
            Self::System { epoch } => {
                let elapsed = epoch.elapsed();
                Timestamp::from_nanos(elapsed.as_nanos() as u64)
            }
            Self::Manual { state } => {
                let guard = state.lock().unwrap_or_else(|e| e.into_inner());
                Timestamp::from_nanos(guard.current_ns)
            }
        }
    }

    /// Advance the manual clock by the given duration.
    /// No-op for system clocks.
    pub fn advance(&self, duration: Duration) {
        if let Self::Manual { state } = self {
            let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
            guard.current_ns = guard.current_ns.saturating_add(duration.as_nanos() as u64);
        }
    }

    /// Set the manual clock to an exact timestamp.
    /// No-op for system clocks.
    pub fn set(&self, timestamp: Timestamp) {
        if let Self::Manual { state } = self {
            let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
            guard.current_ns = timestamp.as_nanos();
        }
    }

    /// Returns `true` if this is a manual (test) clock.
    #[must_use]
    pub const fn is_manual(&self) -> bool {
        matches!(self, Self::Manual { .. })
    }

    /// Returns `true` if this is a system (real) clock.
    #[must_use]
    pub const fn is_system(&self) -> bool {
        matches!(self, Self::System { .. })
    }
}

/// A deadline that can be checked against a clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deadline {
    /// When the deadline expires.
    pub expires_at: Timestamp,
}

impl Deadline {
    /// Create a deadline that expires at the given absolute timestamp.
    #[must_use]
    pub const fn at(ts: Timestamp) -> Self {
        Self { expires_at: ts }
    }

    /// Create a deadline relative to a given base timestamp and duration.
    #[must_use]
    pub const fn after(base: Timestamp, duration: Duration) -> Self {
        Self {
            expires_at: base.add(duration),
        }
    }

    /// Check whether this deadline has expired.
    #[must_use]
    pub fn is_expired(&self, clock: &Clock) -> bool {
        clock.now() >= self.expires_at
    }

    /// Remaining time until expiry, or zero if already expired.
    #[must_use]
    pub fn remaining(&self, clock: &Clock) -> Duration {
        let now = clock.now();
        if now >= self.expires_at {
            Duration::ZERO
        } else {
            self.expires_at.duration_since(now)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_clock_starts_at_zero() {
        let clock = Clock::manual();
        assert_eq!(clock.now(), Timestamp::ZERO);
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
        clock.set(Timestamp::from_nanos(5_000_000_000));
        assert_eq!(clock.now().as_nanos(), 5_000_000_000);
    }

    #[test]
    fn manual_clock_multiple_advances() {
        let clock = Clock::manual();
        clock.advance(Duration::from_millis(50));
        clock.advance(Duration::from_millis(50));
        assert_eq!(clock.now().as_millis(), 100);
    }

    #[test]
    fn timestamp_duration_since() {
        let a = Timestamp::from_nanos(100);
        let b = Timestamp::from_nanos(300);
        assert_eq!(b.duration_since(a), Duration::from_nanos(200));
    }

    #[test]
    fn timestamp_duration_since_saturates() {
        let a = Timestamp::from_nanos(300);
        let b = Timestamp::from_nanos(100);
        assert_eq!(b.duration_since(a), Duration::ZERO);
    }

    #[test]
    fn timestamp_add() {
        let ts = Timestamp::from_nanos(100);
        let result = ts.add(Duration::from_nanos(50));
        assert_eq!(result.as_nanos(), 150);
    }

    #[test]
    fn system_clock_is_monotonic() {
        let clock = Clock::system();
        let t1 = clock.now();
        let t2 = clock.now();
        assert!(t2 >= t1);
    }

    #[test]
    fn clock_type_detection() {
        let manual = Clock::manual();
        assert!(manual.is_manual());
        assert!(!manual.is_system());

        let system = Clock::system();
        assert!(system.is_system());
        assert!(!system.is_manual());
    }

    #[test]
    fn deadline_not_expired_initially() {
        let clock = Clock::manual();
        let deadline = Deadline::after(clock.now(), Duration::from_millis(100));
        assert!(!deadline.is_expired(&clock));
    }

    #[test]
    fn deadline_expires_after_advance() {
        let clock = Clock::manual();
        let deadline = Deadline::after(clock.now(), Duration::from_millis(100));
        clock.advance(Duration::from_millis(100));
        assert!(deadline.is_expired(&clock));
    }

    #[test]
    fn deadline_remaining() {
        let clock = Clock::manual();
        let deadline = Deadline::after(clock.now(), Duration::from_millis(100));
        clock.advance(Duration::from_millis(30));
        assert_eq!(deadline.remaining(&clock), Duration::from_millis(70));
    }

    #[test]
    fn deadline_remaining_after_expiry() {
        let clock = Clock::manual();
        let deadline = Deadline::after(clock.now(), Duration::from_millis(50));
        clock.advance(Duration::from_millis(100));
        assert_eq!(deadline.remaining(&clock), Duration::ZERO);
    }

    #[test]
    fn cloned_manual_clock_shares_state() {
        let clock = Clock::manual();
        let clone = clock.clone();
        clock.advance(Duration::from_millis(42));
        assert_eq!(clone.now().as_millis(), 42);
    }

    #[test]
    fn timestamp_ordering() {
        let a = Timestamp::from_nanos(10);
        let b = Timestamp::from_nanos(20);
        assert!(a < b);
        assert!(b > a);
        assert_eq!(a, Timestamp::from_nanos(10));
    }
}
