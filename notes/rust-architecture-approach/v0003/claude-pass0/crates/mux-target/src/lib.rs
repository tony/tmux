//! # mux-target
//!
//! Parser for tmux target syntax: `session:window.pane`.
//!
//! Supports forms like:
//! - `mysession` (session only)
//! - `mysession:1` (session + window)
//! - `mysession:1.2` (session + window + pane)
//! - `:1.2` (current session, window 1, pane 2)
//! - `%5` (pane ID)
//! - `@3` (window ID)
//! - `$2` (session ID)
//!
//! L2 data crate -- no internal dependencies.

#![forbid(unsafe_code)]

use thiserror::Error;

/// Errors from target parsing.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TargetError {
    /// Invalid target syntax.
    #[error("invalid target: {0}")]
    Invalid(String),
}

/// A parsed tmux target specifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// Session specifier (name, ID with $, or None for current).
    pub session: Option<TargetSpec>,
    /// Window specifier (index, ID with @, or None for current).
    pub window: Option<TargetSpec>,
    /// Pane specifier (index, ID with %, or None for current).
    pub pane: Option<TargetSpec>,
}

/// A single component of a target specifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetSpec {
    /// A name string.
    Name(String),
    /// A numeric index.
    Index(i32),
    /// An entity ID (prefixed with $, @, or %).
    Id(u64),
    /// Relative offset (+N or -N).
    Relative(i32),
    /// Special tokens: {last}, {next}, {previous}, etc.
    Special(String),
}

impl Target {
    /// Parse a target string.
    ///
    /// # Errors
    ///
    /// Returns `TargetError::Invalid` if the syntax is unrecognized.
    pub fn parse(input: &str) -> Result<Self, TargetError> {
        if input.is_empty() {
            return Ok(Self {
                session: None,
                window: None,
                pane: None,
            });
        }

        // Direct ID references
        if let Some(rest) = input.strip_prefix('%') {
            let id = rest
                .parse::<u64>()
                .map_err(|_| TargetError::Invalid(input.to_owned()))?;
            return Ok(Self {
                session: None,
                window: None,
                pane: Some(TargetSpec::Id(id)),
            });
        }
        if let Some(rest) = input.strip_prefix('@') {
            let id = rest
                .parse::<u64>()
                .map_err(|_| TargetError::Invalid(input.to_owned()))?;
            return Ok(Self {
                session: None,
                window: Some(TargetSpec::Id(id)),
                pane: None,
            });
        }
        if let Some(rest) = input.strip_prefix('$') {
            let id = rest
                .parse::<u64>()
                .map_err(|_| TargetError::Invalid(input.to_owned()))?;
            return Ok(Self {
                session: Some(TargetSpec::Id(id)),
                window: None,
                pane: None,
            });
        }

        // Parse session:window.pane
        let (session_part, rest) = if let Some(colon_pos) = input.find(':') {
            let s = &input[..colon_pos];
            let r = &input[colon_pos + 1..];
            (if s.is_empty() { None } else { Some(s) }, r)
        } else {
            // No colon: could be just a session name
            return Ok(Self {
                session: Some(parse_spec(input)),
                window: None,
                pane: None,
            });
        };

        let (window_part, pane_part) = if let Some(dot_pos) = rest.find('.') {
            let w = &rest[..dot_pos];
            let p = &rest[dot_pos + 1..];
            (
                if w.is_empty() { None } else { Some(w) },
                if p.is_empty() { None } else { Some(p) },
            )
        } else {
            (
                if rest.is_empty() { None } else { Some(rest) },
                None,
            )
        };

        Ok(Self {
            session: session_part.map(parse_spec),
            window: window_part.map(parse_spec),
            pane: pane_part.map(parse_spec),
        })
    }
}

/// Parse a single spec component (name, index, or special).
fn parse_spec(s: &str) -> TargetSpec {
    if s.starts_with('{') && s.ends_with('}') {
        return TargetSpec::Special(s[1..s.len() - 1].to_owned());
    }
    if let Some(rest) = s.strip_prefix('+') {
        if let Ok(n) = rest.parse::<i32>() {
            return TargetSpec::Relative(n);
        }
    }
    if let Some(rest) = s.strip_prefix('-') {
        if let Ok(n) = rest.parse::<i32>() {
            return TargetSpec::Relative(-n);
        }
    }
    if let Ok(n) = s.parse::<i32>() {
        return TargetSpec::Index(n);
    }
    TargetSpec::Name(s.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        let t = Target::parse("").unwrap_or_else(|_| Target {
            session: None,
            window: None,
            pane: None,
        });
        assert!(t.session.is_none());
    }

    #[test]
    fn parse_session_name() {
        let t = Target::parse("main").unwrap_or_else(|_| Target {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(t.session, Some(TargetSpec::Name("main".into())));
    }

    #[test]
    fn parse_session_window() {
        let t = Target::parse("main:1").unwrap_or_else(|_| Target {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(t.session, Some(TargetSpec::Name("main".into())));
        assert_eq!(t.window, Some(TargetSpec::Index(1)));
    }

    #[test]
    fn parse_full_target() {
        let t = Target::parse("main:1.2").unwrap_or_else(|_| Target {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(t.session, Some(TargetSpec::Name("main".into())));
        assert_eq!(t.window, Some(TargetSpec::Index(1)));
        assert_eq!(t.pane, Some(TargetSpec::Index(2)));
    }

    #[test]
    fn parse_pane_id() {
        let t = Target::parse("%5").unwrap_or_else(|_| Target {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(t.pane, Some(TargetSpec::Id(5)));
    }

    #[test]
    fn parse_window_id() {
        let t = Target::parse("@3").unwrap_or_else(|_| Target {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(t.window, Some(TargetSpec::Id(3)));
    }

    #[test]
    fn parse_session_id() {
        let t = Target::parse("$2").unwrap_or_else(|_| Target {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(t.session, Some(TargetSpec::Id(2)));
    }

    #[test]
    fn parse_current_session_window() {
        let t = Target::parse(":1.2").unwrap_or_else(|_| Target {
            session: None,
            window: None,
            pane: None,
        });
        assert!(t.session.is_none());
        assert_eq!(t.window, Some(TargetSpec::Index(1)));
        assert_eq!(t.pane, Some(TargetSpec::Index(2)));
    }

    #[test]
    fn parse_special_last() {
        let t = Target::parse("{last}").unwrap_or_else(|_| Target {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(t.session, Some(TargetSpec::Special("last".into())));
    }

    #[test]
    fn parse_relative() {
        let t = Target::parse("+1").unwrap_or_else(|_| Target {
            session: None,
            window: None,
            pane: None,
        });
        assert_eq!(t.session, Some(TargetSpec::Relative(1)));
    }

    #[test]
    fn parse_invalid_pane_id() {
        assert!(Target::parse("%abc").is_err());
    }
}
