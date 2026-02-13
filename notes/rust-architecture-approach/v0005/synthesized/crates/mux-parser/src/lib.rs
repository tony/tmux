//! Paul Williams VT state machine parser for tmux behavioral compatibility.
//!
//! Implements the 14-state VT parser described in "A parser for DEC's
//! ANSI-compatible video terminals." The parser processes arbitrary byte
//! streams and emits structured `VtAction` events.
//!
//! The parser never panics on any input (fuzz-verified via proptest).

#![forbid(unsafe_code)]

pub mod csi;
pub mod input;
pub mod state_machine;

pub use csi::CsiParams;
pub use input::{VtAction, VtParser};
pub use state_machine::{ParserState, TransitionAction};
