use std::path::PathBuf;

use mux_types::Size;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct SpawnSpec {
    pub shell: PathBuf,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    pub size: Size,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildEvent {
    Exited(i32),
    Signaled(i32),
}

#[derive(Debug, Error)]
pub enum PtyError {
    #[error("pty backend not configured")]
    BackendUnavailable,
    #[error("io error: {0}")]
    Io(String),
}

pub trait PtyHandle: Send {
    fn pid(&self) -> u32;
    fn write_input(&mut self, bytes: &[u8]) -> Result<(), PtyError>;
    fn resize(&mut self, size: Size) -> Result<(), PtyError>;
    fn poll_output(&mut self) -> Result<Vec<u8>, PtyError>;
    fn terminate(&mut self) -> Result<(), PtyError>;
}

pub trait PtyBackend: Send + Sync {
    fn spawn(&self, spec: &SpawnSpec) -> Result<Box<dyn PtyHandle>, PtyError>;
}
