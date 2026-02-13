//! Property-based tests for mux-time.

use proptest::prelude::*;
use mux_time::{Clock, Timestamp, TimerWheel, TimerId};
use std::time::Duration;

proptest! {
    #[test]
    fn manual_clock_advance_monotonic(steps in proptest::collection::vec(1..1000u64, 1..20)) {
        let clock = Clock::manual();
        let mut prev = clock.now();
        for ms in &steps {
            clock.advance(Duration::from_millis(*ms));
            let now = clock.now();
            prop_assert!(now >= prev);
            prev = now;
        }
    }

    #[test]
    fn timestamp_add_duration_monotonic(base_ns in 0..100_000_000_000u64, add_ns in 0..100_000_000_000u64) {
        let base = Timestamp::from_nanos(base_ns);
        let added = base.add(Duration::from_nanos(add_ns));
        prop_assert!(added >= base);
    }

    #[test]
    fn timestamp_duration_since_symmetric(a_ns in 0..50_000_000_000u64, b_ns in 0..50_000_000_000u64) {
        let a = Timestamp::from_nanos(a_ns);
        let b = Timestamp::from_nanos(b_ns);
        if a_ns >= b_ns {
            prop_assert_eq!(a.duration_since(b), Duration::from_nanos(a_ns - b_ns));
        } else {
            prop_assert_eq!(b.duration_since(a), Duration::from_nanos(b_ns - a_ns));
        }
    }

    #[test]
    fn timer_wheel_schedule_and_advance(
        timers in proptest::collection::vec((1..1000u64, 1..1000u64), 1..20),
    ) {
        let mut wheel = TimerWheel::new();
        for (tag, ms) in &timers {
            let deadline = Timestamp::from_nanos(0).add(Duration::from_millis(*ms));
            wheel.schedule(deadline, *tag);
        }
        let far_future = Timestamp::from_nanos(0).add(Duration::from_secs(10));
        let expired = wheel.advance_to(far_future);
        // All timers should have expired
        prop_assert_eq!(expired.len(), timers.len());
    }

    #[test]
    fn timer_wheel_no_premature_expire(ms in 100..10_000u64) {
        let mut wheel = TimerWheel::new();
        let deadline = Timestamp::from_nanos(0).add(Duration::from_millis(ms));
        wheel.schedule(deadline, 1);
        let before = Timestamp::from_nanos(0).add(Duration::from_millis(ms - 1));
        let expired = wheel.advance_to(before);
        prop_assert!(expired.is_empty());
    }

    #[test]
    fn timer_wheel_cancel_prevents_expire(tag in 0..100u64) {
        let mut wheel = TimerWheel::new();
        let deadline = Timestamp::from_nanos(0).add(Duration::from_millis(100));
        let id = wheel.schedule(deadline, tag);
        wheel.cancel(id);
        let expired = wheel.advance_to(Timestamp::from_nanos(0).add(Duration::from_secs(10)));
        prop_assert!(expired.is_empty());
    }

    #[test]
    fn timestamp_ordering_consistent(a_ns in 0..100_000_000_000u64, b_ns in 0..100_000_000_000u64) {
        let a = Timestamp::from_nanos(a_ns);
        let b = Timestamp::from_nanos(b_ns);
        prop_assert_eq!(a.cmp(&b), a_ns.cmp(&b_ns));
    }
}

#[test]
fn timer_wheel_multiple_same_deadline() {
    let mut wheel = TimerWheel::new();
    let deadline = Timestamp::from_nanos(500_000_000); // 500ms
    wheel.schedule(deadline, 1);
    wheel.schedule(deadline, 2);
    wheel.schedule(deadline, 3);
    let expired = wheel.advance_to(deadline);
    assert_eq!(expired.len(), 3);
}

#[test]
fn timer_wheel_expire_partial() {
    let mut wheel = TimerWheel::new();
    wheel.schedule(Timestamp::from_nanos(100_000_000), 1); // 100ms
    wheel.schedule(Timestamp::from_nanos(200_000_000), 2); // 200ms
    wheel.schedule(Timestamp::from_nanos(300_000_000), 3); // 300ms
    let expired = wheel.advance_to(Timestamp::from_nanos(150_000_000)); // 150ms
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].tag, 1);
}

#[test]
fn timer_wheel_empty_expire() {
    let mut wheel = TimerWheel::new();
    let expired = wheel.advance_to(Timestamp::from_nanos(1_000_000_000));
    assert!(expired.is_empty());
}

#[test]
fn clock_manual_starts_at_zero() {
    let clock = Clock::manual();
    assert_eq!(clock.now(), Timestamp::from_nanos(0));
}

#[test]
fn clock_advance_accumulates() {
    let clock = Clock::manual();
    clock.advance(Duration::from_millis(100));
    clock.advance(Duration::from_millis(200));
    assert_eq!(clock.now().as_millis(), 300);
}
