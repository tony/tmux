//! # mux-target
//!
//! Parser for tmux's target syntax: `session:window.pane`

#![forbid(unsafe_code)]

/// A parsed target specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub session: Option<SessionTarget>,
    pub window: Option<WindowTarget>,
    pub pane: Option<PaneTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionTarget { Id(u64), ExactName(String), Name(String), Current, Last }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowTarget { Id(u64), Index(i32), Name(String), Current, Last, Next, Previous, Start, End }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaneTarget { Id(u64), Index(u32), Current, Last, Next, Previous, Top, Bottom, Left, Right, UpOf, DownOf }

/// Parse a target string.
pub fn parse_target(input: &str) -> Result<Target, TargetError> {
    if input.is_empty() {
        return Ok(Target { session: None, window: None, pane: None });
    }
    let (session_part, rest) = if let Some(pos) = input.find(':') {
        (Some(&input[..pos]), &input[pos + 1..])
    } else {
        (None, input)
    };
    let (window_part, pane_part) = if let Some(pos) = rest.find('.') {
        (if rest[..pos].is_empty() { None } else { Some(&rest[..pos]) }, Some(&rest[pos + 1..]))
    } else {
        (if rest.is_empty() { None } else { Some(rest) }, None)
    };
    let session = session_part.map(parse_session_target).transpose()?;
    let window = window_part.map(parse_window_target).transpose()?;
    let pane = pane_part.map(parse_pane_target).transpose()?;
    Ok(Target { session, window, pane })
}

fn parse_session_target(s: &str) -> Result<SessionTarget, TargetError> {
    if s.is_empty() { return Ok(SessionTarget::Current); }
    if let Some(id) = s.strip_prefix('$') { return Ok(SessionTarget::Id(id.parse().map_err(|_| TargetError::InvalidSessionId(s.to_owned()))?)); }
    if let Some(name) = s.strip_prefix('=') { return Ok(SessionTarget::ExactName(name.to_owned())); }
    if s == "!" || s == "{last}" { return Ok(SessionTarget::Last); }
    Ok(SessionTarget::Name(s.to_owned()))
}

fn parse_window_target(s: &str) -> Result<WindowTarget, TargetError> {
    if s.is_empty() { return Ok(WindowTarget::Current); }
    if let Some(id) = s.strip_prefix('@') { return Ok(WindowTarget::Id(id.parse().map_err(|_| TargetError::InvalidWindowId(s.to_owned()))?)); }
    match s {
        "!" | "{last}" => Ok(WindowTarget::Last), "+" | "{next}" => Ok(WindowTarget::Next),
        "-" | "{previous}" => Ok(WindowTarget::Previous), "^" | "{start}" => Ok(WindowTarget::Start),
        "$" | "{end}" => Ok(WindowTarget::End),
        _ => if let Ok(idx) = s.parse::<i32>() { Ok(WindowTarget::Index(idx)) } else { Ok(WindowTarget::Name(s.to_owned())) }
    }
}

fn parse_pane_target(s: &str) -> Result<PaneTarget, TargetError> {
    if s.is_empty() { return Ok(PaneTarget::Current); }
    if let Some(id) = s.strip_prefix('%') { return Ok(PaneTarget::Id(id.parse().map_err(|_| TargetError::InvalidPaneId(s.to_owned()))?)); }
    match s {
        "!" | "{last}" => Ok(PaneTarget::Last), "+" | "{next}" => Ok(PaneTarget::Next),
        "-" | "{previous}" => Ok(PaneTarget::Previous), "{top}" => Ok(PaneTarget::Top),
        "{bottom}" => Ok(PaneTarget::Bottom), "{left}" => Ok(PaneTarget::Left),
        "{right}" => Ok(PaneTarget::Right), "{up-of}" => Ok(PaneTarget::UpOf), "{down-of}" => Ok(PaneTarget::DownOf),
        _ => if let Ok(idx) = s.parse::<u32>() { Ok(PaneTarget::Index(idx)) } else { Err(TargetError::InvalidPaneSpec(s.to_owned())) }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TargetError {
    #[error("invalid session ID: {0}")] InvalidSessionId(String),
    #[error("invalid window ID: {0}")] InvalidWindowId(String),
    #[error("invalid pane ID: {0}")] InvalidPaneId(String),
    #[error("invalid pane specifier: {0}")] InvalidPaneSpec(String),
    #[error("malformed target: {0}")] Malformed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_session_colon_window() {
        let t = parse_target("mysess:1").ok();
        assert!(matches!(t.as_ref().and_then(|t| t.session.as_ref()), Some(SessionTarget::Name(n)) if n == "mysess"));
        assert!(matches!(t.as_ref().and_then(|t| t.window.as_ref()), Some(WindowTarget::Index(1))));
    }

    #[test]
    fn parse_pane_id() {
        let t = parse_target(":.%5").ok();
        assert!(matches!(t.as_ref().and_then(|t| t.pane.as_ref()), Some(PaneTarget::Id(5))));
    }

    #[test]
    fn parse_special_tokens() {
        assert!(matches!(parse_target("!:+.-"), Ok(Target { session: Some(SessionTarget::Last), window: Some(WindowTarget::Next), pane: Some(PaneTarget::Previous) })));
    }

    #[test]
    fn parse_empty_target() {
        let t = parse_target("");
        assert!(t.is_ok());
    }
}
