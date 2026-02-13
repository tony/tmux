use std::fs;
use std::path::PathBuf;

use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum TestSupportError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct TestSocket {
    pub path: PathBuf,
}

impl TestSocket {
    #[must_use]
    pub fn unique(prefix: &str) -> Self {
        let pid = std::process::id();
        let path = PathBuf::from(format!("/tmp/{prefix}-{pid}-{}.sock", Uuid::new_v4()));
        Self { path }
    }
}

#[derive(Debug)]
pub struct TestGuard {
    socket: TestSocket,
}

impl TestGuard {
    #[must_use]
    pub fn new(prefix: &str) -> Self {
        Self {
            socket: TestSocket::unique(prefix),
        }
    }

    #[must_use]
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket.path
    }
}

impl Drop for TestGuard {
    fn drop(&mut self) {
        if self.socket.path.exists() {
            let _ = fs::remove_file(&self.socket.path);
        }
    }
}
