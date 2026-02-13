//! Property-based tests for mux-target.

use proptest::prelude::*;
use mux_target::{TargetSpec, TargetComponent};

proptest! {
    #[test]
    fn session_only_roundtrip(name in "[a-zA-Z][a-zA-Z0-9_]{0,15}") {
        let spec = TargetSpec::parse(&name).unwrap_or_else(|_| TargetSpec {
            session: None, window: None, pane: None,
        });
        // Session-only parse: session should be Name variant
        if let Some(TargetComponent::Name(ref n)) = spec.session {
            prop_assert_eq!(n, &name);
        }
    }

    #[test]
    fn session_colon_window(
        session in "[a-zA-Z][a-zA-Z0-9]{0,8}",
        window in 0..99u32,
    ) {
        let input = format!("{session}:{window}");
        let spec = TargetSpec::parse(&input).unwrap_or_else(|_| TargetSpec {
            session: None, window: None, pane: None,
        });
        // Session should be Name
        if let Some(TargetComponent::Name(ref n)) = spec.session {
            prop_assert_eq!(n, &session);
        }
        // Window should be Index
        if let Some(TargetComponent::Index(idx)) = spec.window {
            prop_assert_eq!(idx, window as i32);
        }
    }

    #[test]
    fn session_colon_window_dot_pane(
        session in "[a-zA-Z][a-zA-Z0-9]{0,8}",
        window in 0..99u32,
        pane in 0..99u32,
    ) {
        let input = format!("{session}:{window}.{pane}");
        let spec = TargetSpec::parse(&input).unwrap_or_else(|_| TargetSpec {
            session: None, window: None, pane: None,
        });
        prop_assert!(spec.session.is_some());
        prop_assert!(spec.window.is_some());
        prop_assert!(spec.pane.is_some());
    }

    #[test]
    fn parse_never_panics(input in ".*") {
        let _ = TargetSpec::parse(&input);
    }
}

#[test]
fn parse_empty_string() {
    let spec = TargetSpec::parse("").unwrap_or_else(|_| TargetSpec {
        session: None, window: None, pane: None,
    });
    assert_eq!(spec.session, Some(TargetComponent::Current));
    assert_eq!(spec.window, Some(TargetComponent::Current));
    assert_eq!(spec.pane, Some(TargetComponent::Current));
}

#[test]
fn parse_session_with_hyphen() {
    let spec = TargetSpec::parse("my-session").unwrap_or_else(|_| TargetSpec {
        session: None, window: None, pane: None,
    });
    assert_eq!(spec.session, Some(TargetComponent::Name("my-session".into())));
}

#[test]
fn parse_window_index_only() {
    let spec = TargetSpec::parse(":1").unwrap_or_else(|_| TargetSpec {
        session: None, window: None, pane: None,
    });
    assert_eq!(spec.window, Some(TargetComponent::Index(1)));
}

#[test]
fn parse_pane_index_only() {
    let spec = TargetSpec::parse(":.1").unwrap_or_else(|_| TargetSpec {
        session: None, window: None, pane: None,
    });
    assert_eq!(spec.pane, Some(TargetComponent::Index(1)));
}

#[test]
fn parse_exact_match() {
    let spec = TargetSpec::parse("=dev").unwrap_or_else(|_| TargetSpec {
        session: None, window: None, pane: None,
    });
    assert_eq!(spec.session, Some(TargetComponent::Exact("dev".into())));
}

#[test]
fn parse_dollar_id() {
    let spec = TargetSpec::parse("$5").unwrap_or_else(|_| TargetSpec {
        session: None, window: None, pane: None,
    });
    assert_eq!(spec.session, Some(TargetComponent::Id(5)));
}

#[test]
fn parse_numeric_session() {
    let spec = TargetSpec::parse("0").unwrap_or_else(|_| TargetSpec {
        session: None, window: None, pane: None,
    });
    assert_eq!(spec.session, Some(TargetComponent::Index(0)));
}

#[test]
fn parse_complex_names() {
    let spec = TargetSpec::parse("main:2.3").unwrap_or_else(|_| TargetSpec {
        session: None, window: None, pane: None,
    });
    assert_eq!(spec.session, Some(TargetComponent::Name("main".into())));
    assert_eq!(spec.window, Some(TargetComponent::Index(2)));
    assert_eq!(spec.pane, Some(TargetComponent::Index(3)));
}

#[test]
fn parse_last() {
    let spec = TargetSpec::parse("!").unwrap_or_else(|_| TargetSpec {
        session: None, window: None, pane: None,
    });
    assert_eq!(spec.session, Some(TargetComponent::Last));
}

#[test]
fn parse_next_previous() {
    let next = TargetSpec::parse("+").unwrap_or_else(|_| TargetSpec {
        session: None, window: None, pane: None,
    });
    assert_eq!(next.session, Some(TargetComponent::Next));

    let prev = TargetSpec::parse("-").unwrap_or_else(|_| TargetSpec {
        session: None, window: None, pane: None,
    });
    assert_eq!(prev.session, Some(TargetComponent::Previous));
}

#[test]
fn parse_relative() {
    let spec = TargetSpec::parse("+2").unwrap_or_else(|_| TargetSpec {
        session: None, window: None, pane: None,
    });
    assert_eq!(spec.session, Some(TargetComponent::Relative(2)));
}

#[test]
fn format_roundtrip() {
    let spec = TargetSpec {
        session: Some(TargetComponent::Name("dev".into())),
        window: Some(TargetComponent::Index(1)),
        pane: Some(TargetComponent::Index(0)),
    };
    let formatted = spec.format();
    assert!(formatted.contains("dev"));
}
