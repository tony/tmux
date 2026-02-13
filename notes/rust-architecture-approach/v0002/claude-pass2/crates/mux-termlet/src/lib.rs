//! # mux-termlet
//!
//! Lightweight testing pods -- PTY + VT emulator + shell in one struct.

#![forbid(unsafe_code)]

use mux_grid::ChunkedGrid;
use mux_grapheme_arena::GraphemeArena;
use mux_snapshot::TextSnapshot;
use mux_types::Size;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct TermletBuilder {
    cols: u32,
    rows: u32,
    shell: String,
    env: Vec<(String, String)>,
    scrollback: u32,
}

impl TermletBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cols: 80, rows: 24,
            shell: "/bin/sh".into(),
            env: vec![("TERM".into(), "xterm-256color".into())],
            scrollback: 10_000,
        }
    }

    #[must_use]
    pub const fn size(mut self, cols: u32, rows: u32) -> Self {
        self.cols = cols;
        self.rows = rows;
        self
    }

    #[must_use]
    pub fn shell(mut self, shell: &str) -> Self {
        self.shell = shell.into();
        self
    }

    #[must_use]
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    #[must_use]
    pub const fn scrollback(mut self, limit: u32) -> Self {
        self.scrollback = limit;
        self
    }

    pub fn build(self) -> Result<Termlet, TermletError> {
        Ok(Termlet {
            grid: ChunkedGrid::new(self.cols, self.rows, self.scrollback),
            arena: GraphemeArena::new(),
            parser: mux_parser::VtParser::new(),
            size: Size::new(self.cols, self.rows),
            shell: self.shell,
            env: self.env,
            pid: None,
            master_fd: None,
        })
    }
}

impl Default for TermletBuilder {
    fn default() -> Self { Self::new() }
}

pub struct Termlet {
    grid: ChunkedGrid,
    arena: GraphemeArena,
    parser: mux_parser::VtParser,
    size: Size,
    shell: String,
    env: Vec<(String, String)>,
    pid: Option<u32>,
    master_fd: Option<i32>,
}

impl Termlet {
    #[must_use]
    pub fn builder() -> TermletBuilder { TermletBuilder::new() }

    #[must_use]
    pub const fn size(&self) -> Size { self.size }

    pub fn send_keys(&mut self, _keys: &str) -> Result<(), TermletError> {
        todo!("send_keys")
    }

    pub fn wait_for(&mut self, _pattern: &str, _timeout: Duration) -> Result<(), TermletError> {
        todo!("wait_for")
    }

    #[must_use]
    pub fn capture(&self) -> TextSnapshot {
        let snapshot = self.grid.snapshot(0);
        mux_snapshot::extract_text(&snapshot, &self.arena)
    }

    pub fn feed_bytes(&mut self, data: &[u8]) {
        let actions = self.parser.feed(data);
        for action in actions {
            if let mux_parser::VtAction::Print(ch) = action {
                let id = self.arena.intern(&ch.to_string());
                let _ = mux_types::Cell { grapheme: id, width: 1, ..mux_types::Cell::empty() };
            }
        }
    }

    pub fn resize(&mut self, cols: u32, rows: u32) {
        self.size = Size::new(cols, rows);
        self.grid.reflow(cols);
    }

    #[must_use]
    pub const fn is_alive(&self) -> bool { self.pid.is_some() }
}

impl std::fmt::Debug for Termlet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Termlet")
            .field("size", &self.size)
            .field("shell", &self.shell)
            .field("pid", &self.pid)
            .finish()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TermletError {
    #[error("PTY error: {0}")]
    Pty(String),
    #[error("timeout: {0}")]
    Timeout(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not started")]
    NotStarted,
    #[error("already stopped")]
    AlreadyStopped,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_defaults() {
        let b = TermletBuilder::new();
        assert_eq!(b.cols, 80);
        assert_eq!(b.rows, 24);
    }

    #[test]
    fn build_termlet() {
        let t = Termlet::builder().size(120, 40).shell("/bin/bash").build();
        assert!(t.is_ok());
    }

    #[test]
    fn capture_empty() {
        let t = Termlet::builder().build().unwrap_or_else(|_| unreachable!());
        let snap = t.capture();
        assert_eq!(snap.lines.len(), 24);
    }
}
