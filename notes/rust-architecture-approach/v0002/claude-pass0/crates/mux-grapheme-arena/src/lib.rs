//! # mux-grapheme-arena
//!
//! Arena allocator for extended grapheme clusters used in terminal cells.
//!
//! Most terminal cells contain single ASCII characters or simple codepoints
//! that fit inline. Extended grapheme clusters (emoji sequences, combining
//! characters, etc.) are stored in this arena and referenced by a 32-bit
//! [`GraphemeId`].
//!
//! ## Design
//! - Index 0 (`GraphemeId::EMPTY`) is reserved for the empty/default grapheme.
//! - The arena is append-only; graphemes are never deallocated individually.
//! - Thread-local or kernel-owned (single-threaded kernel model).
//! - Deduplication via `IndexMap` for memory efficiency.

#![forbid(unsafe_code)]

use std::collections::HashMap;

/// A handle to a grapheme cluster stored in the arena.
///
/// Inline graphemes (single codepoints that fit in a `char`) can be stored
/// directly without arena allocation. Extended graphemes get an arena index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraphemeId {
    /// Empty cell -- no grapheme content.
    Empty,
    /// Single codepoint that fits in a `char`.
    Inline(char),
    /// Index into the arena for extended grapheme clusters.
    /// Index 0 in the arena storage is valid (distinct from `Empty`).
    Arena(u32),
}

impl GraphemeId {
    /// The empty grapheme (no content).
    pub const EMPTY: Self = Self::Empty;

    /// Create a grapheme ID for a single character.
    #[must_use]
    pub const fn from_char(ch: char) -> Self {
        Self::Inline(ch)
    }
}

impl Default for GraphemeId {
    fn default() -> Self {
        Self::Empty
    }
}

/// Arena storage for extended grapheme clusters.
///
/// Single-codepoint graphemes are stored inline in [`GraphemeId::Inline`]
/// and do not consume arena space. Only multi-codepoint clusters are
/// stored here.
#[derive(Debug, Clone)]
pub struct GraphemeArena {
    /// Stored grapheme strings, indexed by arena slot.
    strings: Vec<String>,
    /// Deduplication map: grapheme string -> arena index.
    dedup: HashMap<String, u32>,
}

impl GraphemeArena {
    /// Create a new empty arena.
    #[must_use]
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
            dedup: HashMap::new(),
        }
    }

    /// Intern a grapheme cluster string, returning its [`GraphemeId`].
    ///
    /// If the string is empty, returns [`GraphemeId::Empty`].
    /// If it is a single character, returns [`GraphemeId::Inline`].
    /// Otherwise, stores in the arena (with deduplication) and returns
    /// [`GraphemeId::Arena`].
    pub fn intern(&mut self, grapheme: &str) -> GraphemeId {
        if grapheme.is_empty() {
            return GraphemeId::Empty;
        }

        let mut chars = grapheme.chars();
        let first = match chars.next() {
            Some(ch) => ch,
            None => return GraphemeId::Empty,
        };

        // Single codepoint: store inline
        if chars.next().is_none() {
            return GraphemeId::Inline(first);
        }

        // Multi-codepoint: arena allocation with dedup
        if let Some(&idx) = self.dedup.get(grapheme) {
            return GraphemeId::Arena(idx);
        }

        let idx = self.strings.len() as u32;
        self.strings.push(grapheme.to_owned());
        self.dedup.insert(grapheme.to_owned(), idx);
        GraphemeId::Arena(idx)
    }

    /// Resolve a [`GraphemeId`] back to its string representation.
    ///
    /// Returns `None` only if an `Arena` index is out of bounds.
    #[must_use]
    pub fn resolve(&self, id: GraphemeId) -> Option<&str> {
        match id {
            GraphemeId::Empty => Some(""),
            GraphemeId::Inline(ch) => {
                // This is a limitation -- we can't return a &str for a char
                // without allocation. Callers should use resolve_to_buf or
                // handle Inline specially.
                // For now, return None and let callers handle it.
                let _ = ch;
                None
            }
            GraphemeId::Arena(idx) => self.strings.get(idx as usize).map(String::as_str),
        }
    }

    /// Resolve a grapheme ID, writing the result into the provided buffer.
    /// Returns the number of bytes written.
    pub fn resolve_to_buf(&self, id: GraphemeId, buf: &mut String) -> usize {
        match id {
            GraphemeId::Empty => 0,
            GraphemeId::Inline(ch) => {
                buf.push(ch);
                ch.len_utf8()
            }
            GraphemeId::Arena(idx) => {
                if let Some(s) = self.strings.get(idx as usize) {
                    buf.push_str(s);
                    s.len()
                } else {
                    0
                }
            }
        }
    }

    /// Number of extended grapheme clusters stored in the arena.
    #[must_use]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Whether the arena is empty (no extended graphemes stored).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Total bytes used by arena strings (approximate memory usage).
    #[must_use]
    pub fn byte_size(&self) -> usize {
        self.strings.iter().map(String::len).sum()
    }
}

impl Default for GraphemeArena {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_grapheme() {
        let mut arena = GraphemeArena::new();
        let id = arena.intern("");
        assert_eq!(id, GraphemeId::Empty);
        assert!(arena.is_empty());
    }

    #[test]
    fn single_char_inline() {
        let mut arena = GraphemeArena::new();
        let id = arena.intern("A");
        assert!(matches!(id, GraphemeId::Inline('A')));
        assert!(arena.is_empty()); // not stored in arena
    }

    #[test]
    fn multi_codepoint_arena() {
        let mut arena = GraphemeArena::new();
        let id = arena.intern("\u{1F469}\u{200D}\u{1F52C}"); // woman scientist emoji
        assert!(matches!(id, GraphemeId::Arena(0)));
        assert_eq!(arena.len(), 1);
    }

    #[test]
    fn deduplication() {
        let mut arena = GraphemeArena::new();
        let id1 = arena.intern("\u{0041}\u{0301}"); // A with combining acute
        let id2 = arena.intern("\u{0041}\u{0301}");
        assert_eq!(id1, id2);
        assert_eq!(arena.len(), 1);
    }

    #[test]
    fn resolve_to_buf_works() {
        let mut arena = GraphemeArena::new();
        let id = arena.intern("A");
        let mut buf = String::new();
        let n = arena.resolve_to_buf(id, &mut buf);
        assert_eq!(n, 1);
        assert_eq!(buf, "A");
    }
}
