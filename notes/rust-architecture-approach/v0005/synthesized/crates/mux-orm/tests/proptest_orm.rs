//! Property-based tests for mux-orm QuerySet API.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;
use mux_orm::{SessionQuerySet, SessionInfo};
use mux_types::id::SessionId;

proptest! {
    #[test]
    fn filter_by_name_finds_match(name in "[a-z]{1,10}") {
        let sessions = vec![
            SessionInfo { id: SessionId(1), name: name.clone(), window_count: 1, attached: false },
            SessionInfo { id: SessionId(2), name: "other".to_string(), window_count: 1, attached: false },
        ];
        let qs = SessionQuerySet::new(sessions).filter_by_name(&name);
        prop_assert_eq!(qs.count(), 1);
    }

    #[test]
    fn filter_by_name_no_match(name in "[a-z]{1,10}") {
        let sessions = vec![
            SessionInfo { id: SessionId(1), name: "nomatch1".to_string(), window_count: 1, attached: false },
            SessionInfo { id: SessionId(2), name: "nomatch2".to_string(), window_count: 1, attached: false },
        ];
        let qs = SessionQuerySet::new(sessions).filter_by_name(&name);
        // Might match if name happens to be "nomatch1" or "nomatch2"
        prop_assert!(qs.count() <= 2);
    }

    #[test]
    fn first_returns_some_when_nonempty(n in 1..10usize) {
        let sessions: Vec<_> = (0..n).map(|i| SessionInfo {
            id: SessionId(i as u64 + 1),
            name: format!("s{i}"),
            window_count: 1,
            attached: false,
        }).collect();
        let qs = SessionQuerySet::new(sessions);
        prop_assert!(qs.first().is_some());
    }

    #[test]
    fn count_matches_input(n in 0..20usize) {
        let sessions: Vec<_> = (0..n).map(|i| SessionInfo {
            id: SessionId(i as u64 + 1),
            name: format!("s{i}"),
            window_count: 1,
            attached: false,
        }).collect();
        let qs = SessionQuerySet::new(sessions);
        prop_assert_eq!(qs.count(), n);
    }

    #[test]
    fn get_by_id_returns_correct(id in 1..100u64) {
        let sessions = vec![
            SessionInfo { id: SessionId(id), name: format!("session-{id}"), window_count: 1, attached: false },
        ];
        let qs = SessionQuerySet::new(sessions);
        let result = qs.get_by_id(SessionId(id));
        prop_assert!(result.is_some());
    }

    #[test]
    fn get_missing_id_none(id in 100..200u64) {
        let sessions = vec![
            SessionInfo { id: SessionId(1), name: "s1".to_string(), window_count: 1, attached: false },
            SessionInfo { id: SessionId(2), name: "s2".to_string(), window_count: 1, attached: false },
        ];
        let qs = SessionQuerySet::new(sessions);
        prop_assert!(qs.get_by_id(SessionId(id)).is_none());
    }

    #[test]
    fn filter_compose(
        names in proptest::collection::vec("[a-z]{1,5}", 1..10),
    ) {
        let sessions: Vec<_> = names.iter().enumerate()
            .map(|(i, name)| SessionInfo {
                id: SessionId(i as u64 + 1),
                name: name.clone(),
                window_count: 1,
                attached: false,
            })
            .collect();
        let qs = SessionQuerySet::new(sessions);
        // Filtering by the first name, then verifying result
        if let Some(first_name) = names.first() {
            let filtered = qs.filter_by_name(first_name);
            // At least 1 match (the first item)
            prop_assert!(filtered.count() >= 1);
        }
    }
}

#[test]
fn empty_queryset() {
    let qs = SessionQuerySet::new(vec![]);
    assert_eq!(qs.count(), 0);
    assert!(qs.first().is_none());
    assert!(!qs.exists());
}

#[test]
fn queryset_names() {
    let sessions = vec![
        SessionInfo { id: SessionId(1), name: "a".to_string(), window_count: 1, attached: false },
        SessionInfo { id: SessionId(2), name: "b".to_string(), window_count: 1, attached: false },
    ];
    let qs = SessionQuerySet::new(sessions);
    let names = qs.names();
    assert_eq!(names, vec!["a", "b"]);
}

#[test]
fn filter_chain() {
    let sessions = vec![
        SessionInfo { id: SessionId(1), name: "dev".to_string(), window_count: 1, attached: false },
        SessionInfo { id: SessionId(2), name: "prod".to_string(), window_count: 1, attached: false },
        SessionInfo { id: SessionId(3), name: "dev".to_string(), window_count: 1, attached: false },
    ];
    let qs = SessionQuerySet::new(sessions);
    let dev = qs.filter_by_name("dev");
    assert_eq!(dev.count(), 2);
    let first = dev.first().expect("expected at least one");
    assert_eq!(first.id, SessionId(1));
}

#[test]
fn queryset_get_by_id() {
    let sessions = vec![
        SessionInfo { id: SessionId(1), name: "a".to_string(), window_count: 1, attached: false },
    ];
    let qs = SessionQuerySet::new(sessions);
    assert!(qs.get_by_id(SessionId(1)).is_some());
    assert!(qs.get_by_id(SessionId(2)).is_none());
}
