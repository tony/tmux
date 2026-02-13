//! # mux-target
//!
//! Target resolution for tmux-style session:window.pane addressing.
//!
//! Parses target strings like "mysession:2.1" into structured target specs.
//!
//! L2 data crate.

#![forbid(unsafe_code)]

use thiserror::Error;

/// Target resolution errors.
#[derive(Debug, Error)]
pub enum TargetError {
    /// Invalid target string format.
    #[error("invalid target: {0}")]
    Invalid(String),
}

/// A parsed target specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetSpec {
    /// Session name or index.
    pub session: Option<String>,
    /// Window index or name.
    pub window: Option<String>,
    /// Pane index.
    pub pane: Option<String>,
}

impl TargetSpec {
    /// Parse a target string (e.g., "session:window.pane").
    pub fn parse(input: &str) -> Result<Self, TargetError> {
        if input.is_empty() {
            return Ok(Self {
                session: None,
                window: None,
                pane: None,
            });
        }

        let (session_window, pane) = if let Some(dot_pos) = input.rfind('.') {
            let pane_part = &input[dot_pos + 1..];
            let rest = &input[..dot_pos];
            (rest, Some(pane_part.to_owned()))
        } else {
            (input, None)
        };

        let (session, window) = if let Some(colon_pos) = session_window.find(':') {
            let s = &session_window[..colon_pos];
            let w = &session_window[colon_pos + 1..];
            (
                if s.is_empty() { None } else { Some(s.to_owned()) },
                if w.is_empty() { None } else { Some(w.to_owned()) },
            )
        } else {
            (
                if session_window.is_empty() {
                    None
                } else {
                    Some(session_window.to_owned())
                },
                None,
            )
        };

        Ok(Self {
            session,
            window,
            pane,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        let t = TargetSpec::parse("").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert!(t.session.is_none());
        assert!(t.window.is_none());
        assert!(t.pane.is_none());
    }

    #[test]
    fn parse_session_only() {
        let t = TargetSpec::parse("main").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(t.session.as_deref(), Some("main"));
        assert!(t.window.is_none());
    }

    #[test]
    fn parse_session_window() {
        let t = TargetSpec::parse("main:2").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(t.session.as_deref(), Some("main"));
        assert_eq!(t.window.as_deref(), Some("2"));
    }

    #[test]
    fn parse_full_target() {
        let t = TargetSpec::parse("main:2.1").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(t.session.as_deref(), Some("main"));
        assert_eq!(t.window.as_deref(), Some("2"));
        assert_eq!(t.pane.as_deref(), Some("1"));
    }

    #[test]
    fn parse_window_pane_only() {
        let t = TargetSpec::parse(":2.1").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert!(t.session.is_none());
        assert_eq!(t.window.as_deref(), Some("2"));
        assert_eq!(t.pane.as_deref(), Some("1"));
    }

    #[test]
    fn parse_pane_only() {
        let t = TargetSpec::parse(":.1").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(t.pane.as_deref(), Some("1"));
    }

    #[test]
    fn parse_named_window() {
        let t = TargetSpec::parse("main:editor").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(t.session.as_deref(), Some("main"));
        assert_eq!(t.window.as_deref(), Some("editor"));
    }

    #[test]
    fn target_spec_equality() {
        let a = TargetSpec::parse("main:2.1").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        let b = TargetSpec::parse("main:2.1").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(a, b);
    }

    #[test]
    fn target_spec_clone() {
        let t = TargetSpec::parse("dev:0.0").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        let t2 = t.clone();
        assert_eq!(t, t2);
    }

    #[test]
    fn parse_colon_only() {
        let t = TargetSpec::parse(":").unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert!(t.session.is_none());
        assert!(t.window.is_none());
    }

    #[test]
    fn parse_dot_separator() {
        let result = TargetSpec::parse("sess:win.1");
        assert!(result.is_ok());
        let t = result.unwrap_or_else(|_| TargetSpec {
            session: None,
            window: None,
            pane: None,
        });
        assert!(t.pane.is_some());
    }

    #[test]
    fn empty_string_parse() {
        let result = TargetSpec::parse("");
        // Empty string should parse to all-None or produce error
        if let Ok(t) = result {
            assert!(t.session.is_none());
        }
    }
}
