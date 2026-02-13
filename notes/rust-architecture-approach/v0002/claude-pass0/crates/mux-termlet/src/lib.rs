//! # mux-termlet
//!
//! Lightweight testing pods -- a PTY + VT emulator + shell in one struct.
//!
//! Termlets are the SDK-first testing primitive for TermForge.
//! They can be spawned, resized, interacted with, and snapshot-tested
//! from Rust, Python (PyO3), and Node.js (napi-rs).
//!
//! ```rust,ignore
//! let termlet = Termlet::builder()
//!     .size(80, 24)
//!     .shell("/bin/bash")
//!     .env("TERM", "xterm-256color")
//!     .build()?;
//!
//! termlet.send_keys("echo hello\n")?;
//! termlet.wait_for("hello", Duration::from_secs(5))?;
//! let snapshot = termlet.capture()?;
//! insta::assert_snapshot!(snapshot);
//! ```

#![forbid(unsafe_code)]

use std::time::Duration;

use mux_grid::ChunkedGrid;
use mux_grapheme_arena::GraphemeArena;
use mux_snapshot::TextSnapshot;
use mux_types::Size;

/// Builder for creating Termlets.
#[derive(Debug, Clone)]
pub struct TermletBuilder {
    cols: u32,
    rows: u32,
    shell: String,
    env: Vec<(String, String)>,
    scrollback: u32,
}

impl TermletBuilder {
    /// Create a new builder with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cols: 80,
            rows: 24,
            shell: "/bin/sh".into(),
            env: vec![("TERM".into(), "xterm-256color".into())],
            scrollback: 10_000,
        }
    }

    /// Set terminal size.
    #[must_use]
    pub const fn size(mut self, cols: u32, rows: u32) -> Self {
        self.cols = cols;
        self.rows = rows;
        self
    }

    /// Set the shell program.
    #[must_use]
    pub fn shell(mut self, shell: &str) -> Self {
        self.shell = shell.into();
        self
    }

    /// Add an environment variable.
    #[must_use]
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Set scrollback limit.
    #[must_use]
    pub const fn scrollback(mut self, limit: u32) -> Self {
        self.scrollback = limit;
        self
    }

    /// Build and start the Termlet.
    ///
    /// # Errors
    /// Returns error if PTY allocation or process spawning fails.
    pub fn build(self) -> Result<Termlet, TermletError> {
        let grid = ChunkedGrid::new(self.cols, self.rows, self.scrollback);
        let arena = GraphemeArena::new();

        Ok(Termlet {
            grid,
            arena,
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
    fn default() -> Self {
        Self::new()
    }
}

/// A lightweight testing pod.
///
/// Contains a PTY, VT parser, grid, and grapheme arena.
/// Designed for automated testing of terminal interactions.
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
    /// Create a builder.
    #[must_use]
    pub fn builder() -> TermletBuilder {
        TermletBuilder::new()
    }

    /// Get the terminal size.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.size
    }

    /// Send keystrokes to the terminal.
    ///
    /// # Errors
    /// Returns error if the PTY write fails.
    pub fn send_keys(&mut self, _keys: &str) -> Result<(), TermletError> {
        // Write keys to master FD
        // Full implementation requires PTY to be started
        todo!("send_keys: write to PTY master FD")
    }

    /// Wait for a string pattern to appear in the terminal output.
    ///
    /// # Errors
    /// Returns error if timeout is reached before the pattern appears.
    pub fn wait_for(&mut self, _pattern: &str, _timeout: Duration) -> Result<(), TermletError> {
        // Poll PTY output, feed through parser, check grid for pattern
        todo!("wait_for: poll PTY + check grid content")
    }

    /// Capture the current terminal content as a text snapshot.
    #[must_use]
    pub fn capture(&self) -> TextSnapshot {
        let snapshot = self.grid.snapshot(0);
        mux_snapshot::extract_text(&snapshot, &self.arena)
    }

    /// Feed raw bytes through the VT parser into the grid.
    /// Useful for testing without a real PTY.
    pub fn feed_bytes(&mut self, data: &[u8]) {
        let actions = self.parser.feed(data);
        for action in actions {
            // Simplified: just handle Print for now
            if let mux_parser::VtAction::Print(ch) = action {
                let id = self.arena.intern(&ch.to_string());
                let cell = mux_types::Cell {
                    grapheme: id,
                    width: 1,
                    ..mux_types::Cell::empty()
                };
                // Would need cursor tracking -- stub for now
                let _ = cell;
            }
        }
    }

    /// Resize the terminal.
    pub fn resize(&mut self, cols: u32, rows: u32) {
        self.size = Size::new(cols, rows);
        self.grid.reflow(cols);
    }

    /// Whether the child process is still running.
    #[must_use]
    pub const fn is_alive(&self) -> bool {
        self.pid.is_some()
    }
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

/// Errors from Termlet operations.
#[derive(Debug, thiserror::Error)]
pub enum TermletError {
    #[error("PTY error: {0}")]
    Pty(String),
    #[error("timeout waiting for pattern: {0}")]
    Timeout(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("termlet not started")]
    NotStarted,
    #[error("termlet already stopped")]
    AlreadyStopped,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_defaults() {
        let builder = TermletBuilder::new();
        assert_eq!(builder.cols, 80);
        assert_eq!(builder.rows, 24);
    }

    #[test]
    fn build_termlet() {
        let termlet = Termlet::builder()
            .size(120, 40)
            .shell("/bin/bash")
            .build();
        assert!(termlet.is_ok());
        let termlet = termlet.unwrap_or_else(|_| unreachable!());
        assert_eq!(termlet.size().cols, 120);
        assert_eq!(termlet.size().rows, 40);
    }

    #[test]
    fn capture_empty_termlet() {
        let termlet = Termlet::builder().build().unwrap_or_else(|_| unreachable!());
        let snap = termlet.capture();
        assert_eq!(snap.lines.len(), 24);
        assert!(snap.lines.iter().all(|l| l.is_empty()));
    }
}
