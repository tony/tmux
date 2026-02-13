use std::os::unix::io::RawFd;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PtyError {
    #[error("Fork failed")]
    ForkFailed,
    #[error("Exec failed")]
    ExecFailed,
}

pub struct Pty {
    pub master: RawFd,
    pub slave: RawFd,
    pub pid: i32,
}

impl Pty {
    pub fn spawn(cmd: &str, args: &[&str], env: &[(&str, &str)], cols: u16, rows: u16) -> Result<Self, PtyError> {
        // Implementation using forkpty or openpty + fork
        todo!()
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), PtyError> {
        todo!()
    }

    pub fn close(&mut self) {
        todo!()
    }
}
--- END FILE ---
