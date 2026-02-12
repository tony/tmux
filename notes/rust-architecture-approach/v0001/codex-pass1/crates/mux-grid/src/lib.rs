//! # mux-grid
//!
//! Grid and scrollback storage for terminal rendering.

#![forbid(unsafe_code)]

pub mod grid;
pub mod scrollback;

pub use grid::{Grid, DEFAULT_SCROLLBACK_LIMIT};
pub use scrollback::Scrollback;
