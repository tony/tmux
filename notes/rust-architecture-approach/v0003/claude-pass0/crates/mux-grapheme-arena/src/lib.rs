//! # mux-grapheme-arena
//!
//! Arena allocator for extended grapheme clusters. Single-char ASCII graphemes
//! are stored inline to avoid allocation overhead. Multi-codepoint clusters
//! are interned into a shared arena for deduplication.
//!
//! L0 leaf crate -- no internal dependencies.

#![forbid(unsafe_code)]

use unicode_segmentation::UnicodeSegmentation;

/// Handle into the grapheme arena. Inline variants avoid heap allocation
/// for common single-character cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraphemeId {
    /// No grapheme (cleared / default cell).
    Empty,
    /// Single ASCII character stored inline.
    Inline(char),
    /// Index into the arena's interned string table.
    Interned(u32),
}

impl GraphemeId {
    /// Construct an inline grapheme from a single char.
    #[must_use]
    pub const fn from_char(c: char) -> Self {
        Self::Inline(c)
    }

    /// Returns `true` if this grapheme is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

/// Arena for interning extended grapheme clusters.
///
/// Deduplicates multi-codepoint graphemes so identical clusters share storage.
/// The arena is append-only and never frees entries during the session lifetime.
#[derive(Debug, Clone)]
pub struct GraphemeArena {
    /// Interned strings, indexed by the `Interned(u32)` variant.
    strings: Vec<String>,
    /// Simple deduplication map: (string -> index).
    dedup: std::collections::HashMap<String, u32>,
}

impl GraphemeArena {
    /// Create a new empty arena.
    #[must_use]
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
            dedup: std::collections::HashMap::new(),
        }
    }

    /// Intern a grapheme cluster string. Returns an `Inline` id for single chars,
    /// or an `Interned` id for multi-codepoint clusters.
    ///
    /// # Errors
    ///
    /// Returns `None` if the arena is full (u32::MAX entries). In practice this
    /// limit is unreachable.
    pub fn intern(&mut self, grapheme: &str) -> Option<GraphemeId> {
        if grapheme.is_empty() {
            return Some(GraphemeId::Empty);
        }

        let mut chars = grapheme.chars();
        let first = chars.next()?;

        // Single char: store inline.
        if chars.next().is_none() {
            return Some(GraphemeId::Inline(first));
        }

        // Multi-codepoint: deduplicate in arena.
        if let Some(&idx) = self.dedup.get(grapheme) {
            return Some(GraphemeId::Interned(idx));
        }

        let idx = u32::try_from(self.strings.len()).ok()?;
        self.strings.push(grapheme.to_owned());
        self.dedup.insert(grapheme.to_owned(), idx);
        Some(GraphemeId::Interned(idx))
    }

    /// Resolve a grapheme id back to its string representation.
    #[must_use]
    pub fn resolve(&self, id: GraphemeId) -> Option<&str> {
        match id {
            GraphemeId::Empty => Some(""),
            GraphemeId::Inline(c) => {
                // We cannot return a reference to a stack-local buffer,
                // so inline chars must be resolved by the caller via the char.
                // This method is primarily for Interned lookups.
                // For Inline, callers should use `resolve_to_string`.
                None
            }
            GraphemeId::Interned(idx) => self.strings.get(idx as usize).map(String::as_str),
        }
    }

    /// Resolve any grapheme id to an owned String.
    #[must_use]
    pub fn resolve_to_string(&self, id: GraphemeId) -> String {
        match id {
            GraphemeId::Empty => String::new(),
            GraphemeId::Inline(c) => c.to_string(),
            GraphemeId::Interned(idx) => self
                .strings
                .get(idx as usize)
                .cloned()
                .unwrap_or_default(),
        }
    }

    /// Number of interned (non-inline) entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Whether the arena has any interned entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Segment a string into grapheme clusters and intern each one.
    pub fn intern_all(&mut self, text: &str) -> Vec<GraphemeId> {
        text.graphemes(true)
            .filter_map(|g| self.intern(g))
            .collect()
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
        let id = arena.intern("").unwrap_or(GraphemeId::Empty);
        assert_eq!(id, GraphemeId::Empty);
        assert!(id.is_empty());
    }

    #[test]
    fn inline_ascii() {
        let mut arena = GraphemeArena::new();
        let id = arena.intern("A").unwrap_or(GraphemeId::Empty);
        assert_eq!(id, GraphemeId::Inline('A'));
        assert!(arena.is_empty()); // No interning needed
    }

    #[test]
    fn inline_unicode_single_char() {
        let mut arena = GraphemeArena::new();
        let id = arena.intern("\u{1F600}").unwrap_or(GraphemeId::Empty);
        assert_eq!(id, GraphemeId::Inline('\u{1F600}'));
    }

    #[test]
    fn interned_multi_codepoint() {
        let mut arena = GraphemeArena::new();
        // Family emoji: multi-codepoint grapheme cluster
        let cluster = "e\u{0301}"; // e + combining acute accent
        let id = arena.intern(cluster).unwrap_or(GraphemeId::Empty);
        assert!(matches!(id, GraphemeId::Interned(0)));
        assert_eq!(arena.len(), 1);
    }

    #[test]
    fn deduplication() {
        let mut arena = GraphemeArena::new();
        let cluster = "e\u{0301}";
        let id1 = arena.intern(cluster).unwrap_or(GraphemeId::Empty);
        let id2 = arena.intern(cluster).unwrap_or(GraphemeId::Empty);
        assert_eq!(id1, id2);
        assert_eq!(arena.len(), 1);
    }

    #[test]
    fn resolve_interned() {
        let mut arena = GraphemeArena::new();
        let cluster = "e\u{0301}";
        let id = arena.intern(cluster).unwrap_or(GraphemeId::Empty);
        assert_eq!(arena.resolve(id), Some(cluster));
    }

    #[test]
    fn resolve_to_string_all_variants() {
        let mut arena = GraphemeArena::new();
        assert_eq!(arena.resolve_to_string(GraphemeId::Empty), "");
        assert_eq!(arena.resolve_to_string(GraphemeId::Inline('X')), "X");

        let cluster = "e\u{0301}";
        let id = arena.intern(cluster).unwrap_or(GraphemeId::Empty);
        assert_eq!(arena.resolve_to_string(id), cluster);
    }

    #[test]
    fn intern_all_segments() {
        let mut arena = GraphemeArena::new();
        let ids = arena.intern_all("ABC");
        assert_eq!(ids.len(), 3);
        assert_eq!(ids[0], GraphemeId::Inline('A'));
        assert_eq!(ids[1], GraphemeId::Inline('B'));
        assert_eq!(ids[2], GraphemeId::Inline('C'));
    }

    #[test]
    fn from_char_constructor() {
        let id = GraphemeId::from_char('Z');
        assert_eq!(id, GraphemeId::Inline('Z'));
        assert!(!id.is_empty());
    }

    #[test]
    fn default_arena_is_empty() {
        let arena = GraphemeArena::default();
        assert!(arena.is_empty());
        assert_eq!(arena.len(), 0);
    }

    #[test]
    fn multiple_distinct_interned() {
        let mut arena = GraphemeArena::new();
        let id1 = arena.intern("e\u{0301}").unwrap_or(GraphemeId::Empty);
        let id2 = arena.intern("a\u{0308}").unwrap_or(GraphemeId::Empty);
        assert_ne!(id1, id2);
        assert_eq!(arena.len(), 2);
    }
}
