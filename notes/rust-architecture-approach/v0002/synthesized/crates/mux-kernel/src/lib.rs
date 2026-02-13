//! # mux-kernel
//!
//! Single-threaded Sans-IO kernel for TermForge.
//!
//! ## Module Organization (GPT multi-file pattern)
//! - [`entity`]: Session, Window, Pane, Client structs.
//! - [`layout`]: Binary-tree layout engine (7 built-in algorithms).
//! - [`event`]: KernelEvent / KernelEffect types.
//! - [`dispatch`]: Command dispatch table.
//! - [`copy_mode`]: Vi/Emacs copy mode state machine.
//! - [`kernel`]: The Kernel struct and process_event reducer.

#![forbid(unsafe_code)]

pub mod entity;
pub mod layout;
pub mod event;
pub mod dispatch;
pub mod copy_mode;
pub mod kernel;

pub use event::{KernelEvent, KernelEffect};
pub use kernel::Kernel;
