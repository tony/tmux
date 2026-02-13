use chrono::{DateTime, Utc};

pub fn now() -> DateTime<Utc> {
    Utc::now()
}

pub struct StopWatch {
    start: DateTime<Utc>,
}

impl StopWatch {
    pub fn start() -> Self {
        Self { start: Utc::now() }
    }
}
--- END FILE ---
