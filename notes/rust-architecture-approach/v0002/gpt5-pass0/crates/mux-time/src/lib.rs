use std::time::{Duration, Instant, SystemTime};

pub trait Clock: Send + Sync + 'static {
    fn now_instant(&self) -> Instant;
    fn now_system(&self) -> SystemTime;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_instant(&self) -> Instant {
        Instant::now()
    }

    fn now_system(&self) -> SystemTime {
        SystemTime::now()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Deadline {
    pub start: Instant,
    pub timeout: Duration,
}

impl Deadline {
    #[must_use]
    pub fn new(clock: &dyn Clock, timeout: Duration) -> Self {
        Self {
            start: clock.now_instant(),
            timeout,
        }
    }

    #[must_use]
    pub fn is_expired(self, clock: &dyn Clock) -> bool {
        clock.now_instant().duration_since(self.start) >= self.timeout
    }
}
