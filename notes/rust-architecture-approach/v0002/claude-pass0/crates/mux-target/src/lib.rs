//! # mux-target
//!
//! Parser for tmux's target syntax: `session:window.pane`
//!
//! Supports the full tmux target grammar including:
//! - Session names, IDs (`$N`), patterns, `=exact`
//! - Window names, IDs (`@N`), indices, special tokens (`{last}`, `+`, `-`)
//! - Pane IDs (`%N`), indices, special tokens (`{left}`, `{right}`, etc.)

#![forbid(unsafe_code)]

/// A parsed target specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// Session component.
    pub session: Option<SessionTarget>,
    /// Window component.
    pub window: Option<WindowTarget>,
    /// Pane component.
    pub pane: Option<PaneTarget>,
}

/// Session target specifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionTarget {
    /// Session ID: `$N`.
    Id(u64),
    /// Exact name match: `=name`.
    ExactName(String),
    /// Name pattern (fnmatch): `name`.
    Name(String),
    /// Current session.
    Current,
    /// Last session: `{last}` or `!`.
    Last,
}

/// Window target specifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowTarget {
    /// Window ID: `@N`.
    Id(u64),
    /// Window index number.
    Index(i32),
    /// Window name.
    Name(String),
    /// Current window.
    Current,
    /// Last window: `{last}` or `!`.
    Last,
    /// Next window: `{next}` or `+`.
    Next,
    /// Previous window: `{previous}` or `-`.
    Previous,
    /// Start window: `{start}` or `^`.
    Start,
    /// End window: `{end}` or `$`.
    End,
}

/// Pane target specifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaneTarget {
    /// Pane ID: `%N`.
    Id(u64),
    /// Pane index.
    Index(u32),
    /// Current pane.
    Current,
    /// Last pane: `{last}` or `!`.
    Last,
    /// Next pane: `{next}` or `+`.
    Next,
    /// Previous pane: `{previous}` or `-`.
    Previous,
    /// Top pane: `{top}`.
    Top,
    /// Bottom pane: `{bottom}`.
    Bottom,
    /// Left pane: `{left}`.
    Left,
    /// Right pane: `{right}`.
    Right,
    /// Up-of pane: `{up-of}`.
    UpOf,
    /// Down-of pane: `{down-of}`.
    DownOf,
}

/// Parse a target string into a [`Target`] structure.
///
/// The format is `[session:]window[.pane]`.
///
/// # Errors
/// Returns error if the target string is malformed.
pub fn parse_target(input: &str) -> Result<Target, TargetError> {
    if input.is_empty() {
        return Ok(Target {
            session: None,
            window: None,
            pane: None,
        });
    }

    let mut session = None;
    let mut window = None;
    let mut pane = None;

    // Split on ':' for session:window
    let (session_part, rest) = if let Some(colon_pos) = input.find(':') {
        let s = &input[..colon_pos];
        let r = &input[colon_pos + 1..];
        (Some(s), r)
    } else {
        (None, input)
    };

    // Split on '.' for window.pane
    let (window_part, pane_part) = if let Some(dot_pos) = rest.find('.') {
        let w = &rest[..dot_pos];
        let p = &rest[dot_pos + 1..];
        (if w.is_empty() { None } else { Some(w) }, Some(p))
    } else {
        (if rest.is_empty() { None } else { Some(rest) }, None)
    };

    if let Some(s) = session_part {
        session = Some(parse_session_target(s)?);
    }

    if let Some(w) = window_part {
        window = Some(parse_window_target(w)?);
    }

    if let Some(p) = pane_part {
        pane = Some(parse_pane_target(p)?);
    }

    Ok(Target { session, window, pane })
}

fn parse_session_target(s: &str) -> Result<SessionTarget, TargetError> {
    if s.is_empty() {
        return Ok(SessionTarget::Current);
    }
    if let Some(id) = s.strip_prefix('$') {
        let n = id
            .parse::<u64>()
            .map_err(|_| TargetError::InvalidSessionId(s.to_owned()))?;
        return Ok(SessionTarget::Id(n));
    }
    if let Some(name) = s.strip_prefix('=') {
        return Ok(SessionTarget::ExactName(name.to_owned()));
    }
    if s == "!" || s == "{last}" {
        return Ok(SessionTarget::Last);
    }
    Ok(SessionTarget::Name(s.to_owned()))
}

fn parse_window_target(s: &str) -> Result<WindowTarget, TargetError> {
    if s.is_empty() {
        return Ok(WindowTarget::Current);
    }
    if let Some(id) = s.strip_prefix('@') {
        let n = id
            .parse::<u64>()
            .map_err(|_| TargetError::InvalidWindowId(s.to_owned()))?;
        return Ok(WindowTarget::Id(n));
    }
    match s {
        "!" | "{last}" => Ok(WindowTarget::Last),
        "+" | "{next}" => Ok(WindowTarget::Next),
        "-" | "{previous}" => Ok(WindowTarget::Previous),
        "^" | "{start}" => Ok(WindowTarget::Start),
        "$" | "{end}" => Ok(WindowTarget::End),
        _ => {
            if let Ok(idx) = s.parse::<i32>() {
                Ok(WindowTarget::Index(idx))
            } else {
                Ok(WindowTarget::Name(s.to_owned()))
            }
        }
    }
}

fn parse_pane_target(s: &str) -> Result<PaneTarget, TargetError> {
    if s.is_empty() {
        return Ok(PaneTarget::Current);
    }
    if let Some(id) = s.strip_prefix('%') {
        let n = id
            .parse::<u64>()
            .map_err(|_| TargetError::InvalidPaneId(s.to_owned()))?;
        return Ok(PaneTarget::Id(n));
    }
    match s {
        "!" | "{last}" => Ok(PaneTarget::Last),
        "+" | "{next}" => Ok(PaneTarget::Next),
        "-" | "{previous}" => Ok(PaneTarget::Previous),
        "{top}" => Ok(PaneTarget::Top),
        "{bottom}" => Ok(PaneTarget::Bottom),
        "{left}" => Ok(PaneTarget::Left),
        "{right}" => Ok(PaneTarget::Right),
        "{up-of}" => Ok(PaneTarget::UpOf),
        "{down-of}" => Ok(PaneTarget::DownOf),
        _ => {
            if let Ok(idx) = s.parse::<u32>() {
                Ok(PaneTarget::Index(idx))
            } else {
                Err(TargetError::InvalidPaneSpec(s.to_owned()))
            }
        }
    }
}

/// Errors from target parsing.
#[derive(Debug, thiserror::Error)]
pub enum TargetError {
    #[error("invalid session ID: {0}")]
    InvalidSessionId(String),
    #[error("invalid window ID: {0}")]
    InvalidWindowId(String),
    #[error("invalid pane ID: {0}")]
    InvalidPaneId(String),
    #[error("invalid pane specifier: {0}")]
    InvalidPaneSpec(String),
    #[error("malformed target: {0}")]
    Malformed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_session_colon_window() {
        let t = parse_target("mysess:1").ok();
        assert!(matches!(
            t.as_ref().and_then(|t| t.session.as_ref()),
            Some(SessionTarget::Name(n)) if n == "mysess"
        ));
        assert!(matches!(
            t.as_ref().and_then(|t| t.window.as_ref()),
            Some(WindowTarget::Index(1))
        ));
    }

    #[test]
    fn parse_pane_id() {
        let t = parse_target(":.%5").ok();
        assert!(matches!(
            t.as_ref().and_then(|t| t.pane.as_ref()),
            Some(PaneTarget::Id(5))
        ));
    }

    #[test]
    fn parse_session_id() {
        let t = parse_target("$3:").ok();
        assert!(matches!(
            t.as_ref().and_then(|t| t.session.as_ref()),
            Some(SessionTarget::Id(3))
        ));
    }

    #[test]
    fn parse_special_tokens() {
        assert!(matches!(
            parse_target("!:+.-"),
            Ok(Target {
                session: Some(SessionTarget::Last),
                window: Some(WindowTarget::Next),
                pane: Some(PaneTarget::Previous),
            })
        ));
    }

    #[test]
    fn parse_empty_target() {
        let t = parse_target("");
        assert!(t.is_ok());
        let t = t.ok();
        assert!(t.as_ref().map_or(false, |t| t.session.is_none()));
    }
}
