//! # mux-grid
//!
//! Chunked COW terminal grid with dirty-line tracking.
//!
//! ## Module Organization (GPT multi-file pattern)
//! - [`line`]: Single grid line with Arc-based COW.
//! - [`chunk`]: 64-line chunks with scrollback management.
//! - [`grid`]: The ChunkedGrid combining active screen + scrollback.
//! - [`snapshot`]: Immutable grid snapshot for rendering pipeline.

#![forbid(unsafe_code)]

pub mod line;
pub mod chunk;
pub mod grid;
pub mod snapshot;

pub use line::Line;
pub use chunk::Chunk;
pub use grid::ChunkedGrid;
pub use snapshot::GridSnapshot;
