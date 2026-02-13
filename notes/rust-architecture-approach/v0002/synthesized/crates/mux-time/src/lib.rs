//! # mux-time
//!
//! Injectable time source for deterministic testing in TermForge.
//!
//! The kernel and all crates that need time MUST use [`Clock`] instead of
//! `std::time::Instant` or `SystemTime` directly.

#![forbid(unsafe_code)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// A monotonic timestamp in microseconds since an arbitrary epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MonoTime(u64);

impl MonoTime {
    /// Create from raw microseconds.
    #[must_use]
    pub const fn from_micros(us: u64) -> Self {
        Self(us)
    }

    /// Get raw microseconds.
    #[must_use]
    pub const fn as_micros(self) -> u64 {
        self.0
    }

    /// Duration since another `MonoTime`.
    #[must_use]
    pub const fn duration_since(self, earlier: Self) -> Duration {
        Duration::from_micros(self.0.saturating_sub(earlier.0))
    }

    /// Add a duration.
    #[must_use]
    pub const fn add(self, duration: Duration) -> Self {
        Self(self.0.saturating_add(duration.as_micros() as u64))
    }
}

/// Wall-clock timestamp (milliseconds since UNIX epoch).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WallTime(i64);

impl WallTime {
    /// Create from milliseconds since UNIX epoch.
    #[must_use]
    pub const fn from_millis(ms: i64) -> Self {
        Self(ms)
    }

    /// Get milliseconds since UNIX epoch.
    #[must_use]
    pub const fn as_millis(self) -> i64 {
        self.0
    }
}

/// Injectable time source.
///
/// Production code uses [`Clock::system()`], tests use [`Clock::manual()`].
#[derive(Debug)]
pub enum Clock {
    /// Real system clock.
    System { epoch: Instant },
    /// Manual clock for deterministic testing.
    Manual { mono: AtomicU64, wall: AtomicU64 },
}

impl Clock {
    /// Create a system clock that reads real time.
    #[must_use]
    pub fn system() -> Self {
        Self::System {
            epoch: Instant::now(),
        }
    }

    /// Create a manual clock starting at the given values.
    #[must_use]
    pub fn manual(initial_mono_us: u64, initial_wall_ms: u64) -> Self {
        Self::Manual {
            mono: AtomicU64::new(initial_mono_us),
            wall: AtomicU64::new(initial_wall_ms),
        }
    }

    /// Get current monotonic time.
    #[must_use]
    pub fn now_mono(&self) -> MonoTime {
        match self {
            Self::System { epoch } => {
                let elapsed = epoch.elapsed();
                MonoTime::from_micros(elapsed.as_micros() as u64)
            }
            Self::Manual { mono, .. } => MonoTime::from_micros(mono.load(Ordering::Relaxed)),
        }
    }

    /// Get current wall-clock time.
    #[must_use]
    pub fn now_wall(&self) -> WallTime {
        match self {
            Self::System { .. } => {
                let millis = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                WallTime::from_millis(millis)
            }
            Self::Manual { wall, .. } => {
                WallTime::from_millis(wall.load(Ordering::Relaxed) as i64)
            }
        }
    }

    /// Advance the manual clock.
    pub fn advance(&self, duration: Duration) {
        match self {
            Self::System { .. } => {}
            Self::Manual { mono, wall, .. } => {
                mono.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
                wall.fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
            }
        }
    }

    /// Set the manual clock to specific values.
    pub fn set(&self, mono_us: u64, wall_ms: u64) {
        match self {
            Self::System { .. } => {}
            Self::Manual { mono, wall } => {
                mono.store(mono_us, Ordering::Relaxed);
                wall.store(wall_ms, Ordering::Relaxed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_clock_deterministic() {
        let clock = Clock::manual(0, 1_000_000);
        assert_eq!(clock.now_mono().as_micros(), 0);
        clock.advance(Duration::from_millis(100));
        assert_eq!(clock.now_mono().as_micros(), 100_000);
    }

    #[test]
    fn system_clock_monotonic() {
        let clock = Clock::system();
        let t1 = clock.now_mono();
        let t2 = clock.now_mono();
        assert!(t2 >= t1);
    }

    #[test]
    fn mono_time_duration_since() {
        let a = MonoTime::from_micros(1000);
        let b = MonoTime::from_micros(3000);
        assert_eq!(b.duration_since(a), Duration::from_micros(2000));
    }

    #[test]
    fn wall_time_roundtrip() {
        let t = WallTime::from_millis(1_700_000_000_000);
        assert_eq!(t.as_millis(), 1_700_000_000_000);
    }

    #[test]
    fn manual_clock_set() {
        let clock = Clock::manual(0, 0);
        clock.set(5000, 10_000);
        assert_eq!(clock.now_mono().as_micros(), 5000);
        assert_eq!(clock.now_wall().as_millis(), 10_000);
    }

    #[test]
    fn mono_time_add() {
        let t = MonoTime::from_micros(1000);
        let t2 = t.add(Duration::from_micros(500));
        assert_eq!(t2.as_micros(), 1500);
    }
}
