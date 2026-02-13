use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use mux_snapshot::Snapshot;
use mux_types::Size;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TermletError {
    #[error("runtime is not configured")]
    MissingRuntime,
    #[error("operation failed: {0}")]
    Operation(String),
}

pub trait TermletRuntime: Send + Sync {
    fn send_keys(&self, input: &str) -> Result<(), TermletError>;
    fn wait_for(&self, needle: &str, timeout: Duration) -> Result<(), TermletError>;
    fn capture(&self) -> Result<Snapshot, TermletError>;
}

pub struct Termlet {
    pub size: Size,
    pub shell: PathBuf,
    pub env: HashMap<String, String>,
    runtime: Option<Box<dyn TermletRuntime>>,
}

impl Termlet {
    #[must_use]
    pub fn builder() -> TermletBuilder {
        TermletBuilder::default()
    }

    pub fn send_keys(&self, input: &str) -> Result<(), TermletError> {
        self.runtime
            .as_ref()
            .ok_or(TermletError::MissingRuntime)?
            .send_keys(input)
    }

    pub fn wait_for(&self, needle: &str, timeout: Duration) -> Result<(), TermletError> {
        self.runtime
            .as_ref()
            .ok_or(TermletError::MissingRuntime)?
            .wait_for(needle, timeout)
    }

    pub fn capture(&self) -> Result<Snapshot, TermletError> {
        self.runtime
            .as_ref()
            .ok_or(TermletError::MissingRuntime)?
            .capture()
    }
}

#[derive(Default)]
pub struct TermletBuilder {
    size: Option<Size>,
    shell: Option<PathBuf>,
    env: HashMap<String, String>,
    runtime: Option<Box<dyn TermletRuntime>>,
}

impl TermletBuilder {
    #[must_use]
    pub fn size(mut self, cols: u16, rows: u16) -> Self {
        self.size = Some(Size { cols, rows });
        self
    }

    #[must_use]
    pub fn shell(mut self, shell: impl Into<PathBuf>) -> Self {
        self.shell = Some(shell.into());
        self
    }

    #[must_use]
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    #[must_use]
    pub fn runtime(mut self, runtime: Box<dyn TermletRuntime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    pub fn build(self) -> Result<Termlet, TermletError> {
        Ok(Termlet {
            size: self.size.unwrap_or(Size { cols: 80, rows: 24 }),
            shell: self.shell.unwrap_or_else(|| PathBuf::from("/bin/sh")),
            env: self.env,
            runtime: self.runtime,
        })
    }
}
