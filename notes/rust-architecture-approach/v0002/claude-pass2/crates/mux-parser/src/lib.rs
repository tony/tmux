//! # mux-parser
//!
//! VT terminal parser implementing Paul Williams' state machine.
//!
//! ## Module Organization (GPT multi-file pattern)
//! - [`state`]: State machine states and transitions.
//! - [`action`]: VtAction output variants.
//! - [`parser`]: The VtParser entry point.

#![forbid(unsafe_code)]

pub mod state;
pub mod action;
pub mod parser;

pub use action::VtAction;
pub use parser::VtParser;
pub use state::VtState;
