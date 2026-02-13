use mux_types::{PaneId, SessionId, WindowId};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Server,
    Session(SessionId),
    Window(WindowId),
    Pane(PaneId),
}

#[derive(Debug, Error)]
pub enum TargetError {
    #[error("invalid target: {0}")]
    Invalid(String),
}

pub fn parse_target(raw: &str) -> Result<Target, TargetError> {
    if raw == "server" {
        return Ok(Target::Server);
    }

    let mut parts = raw.split(':');
    let ty = parts
        .next()
        .ok_or_else(|| TargetError::Invalid(raw.to_owned()))?;
    let id = parts
        .next()
        .ok_or_else(|| TargetError::Invalid(raw.to_owned()))?
        .parse::<u64>()
        .map_err(|_| TargetError::Invalid(raw.to_owned()))?;

    match ty {
        "session" => Ok(Target::Session(SessionId(id))),
        "window" => Ok(Target::Window(WindowId(id))),
        "pane" => Ok(Target::Pane(PaneId(id))),
        _ => Err(TargetError::Invalid(raw.to_owned())),
    }
}
