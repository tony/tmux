//! # mux-parser
//!
//! VT terminal parser implementing Paul Williams' state machine.
//! Own parser for tmux behavioral compatibility -- crossterm/vt100/termwiz
//! are explicitly not used.
//!
//! ## Module Organization
//! - [`state_machine`]: The 14 VT parser states and transition logic.
//! - [`csi`]: CSI sequence parameter parsing and dispatch.
//! - [`input`]: The VtParser entry point and VtAction output.

#![forbid(unsafe_code)]

pub mod state_machine;
pub mod csi;
pub mod input;

pub use input::{VtParser, VtAction};
pub use state_machine::VtState;
pub use csi::CsiParams;
