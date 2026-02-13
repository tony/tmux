//! # mux-types
//!
//! Core types for the TermForge terminal multiplexer.
//! L1 crate -- depends only on mux-grapheme-arena and mux-time.
//!
//! ## Module Organization
//! - [`cell`]: Terminal cell with grapheme, width, flags, attributes, colours.
//! - [`colour`]: Colour representation with pack/unpack for compact storage.
//! - [`attrs`]: Text attribute bitflags (bold, italic, underline variants, etc.).
//! - [`key`]: KeyCode, KeyModifiers, KeyEvent, MouseEvent.
//! - [`id`]: Strongly-typed entity IDs (SessionId, WindowId, PaneId, ClientId).
//! - [`geometry`]: Size, Rect, Position types.
//! - [`error`]: Core error type and Result alias.

#![forbid(unsafe_code)]

pub mod cell;
pub mod colour;
pub mod attrs;
pub mod key;
pub mod id;
pub mod geometry;
pub mod error;

// Re-exports for ergonomic imports
pub use cell::{Cell, CellFlags};
pub use colour::Colour;
pub use attrs::Attrs;
pub use key::{KeyCode, KeyModifiers, KeyEvent, MouseButton, MouseEventKind, MouseEvent};
pub use id::{SessionId, WindowId, PaneId, ClientId};
pub use geometry::{Size, Rect, Position};
pub use error::{TermForgeError, Result};
