use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct Timestamp(Instant);

impl Timestamp {
    pub fn now() -> Self {
        Self(Instant::now())
    }

    pub fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self(Instant::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now() {
        let t = Timestamp::now();
        assert!(t.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn test_default() {
        let t = Timestamp::default();
        assert!(t.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn test_ordering() {
        let t1 = Timestamp::now();
        std::thread::sleep(Duration::from_millis(1));
        let t2 = Timestamp::now();
        assert!(t2.0 > t1.0);
    }

    #[test]
    fn test_debug() {
        let t = Timestamp::now();
        assert!(format!("{:?}", t).contains("Timestamp"));
    }

    #[test]
    fn test_copy() {
        let t1 = Timestamp::now();
        let t2 = t1;
        assert_eq!(t1.0, t2.0);
    }
}
