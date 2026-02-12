//! # mux-grapheme-arena
//!
//! Arena allocator for extended grapheme clusters that don't fit in a single
//! Unicode scalar value (multi-codepoint graphemes like flags, skin-tone emoji, etc.).
//!
//! ## Key Design (S92)
//! - 14-bit extension index (0x0001..=0x3FFF), max 16,383 entries.
//! - INV-030: Index 0x0000 is reserved for "no extension".
//! - Deduplication: identical graphemes share the same index.
//! - Binary encode/decode for snapshot serialization.

#![forbid(unsafe_code)]

use std::collections::HashMap;

/// Maximum number of extended grapheme entries (14-bit index).
pub const MAX_ENTRIES: usize = 0x3FFF; // 16,383

/// Error type for GraphemeArena operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphemeError {
    /// The arena has reached its capacity limit.
    CapacityExceeded { limit: usize },
    /// The given index is out of range or refers to a freed slot.
    InvalidIndex(u16),
    /// The grapheme string is empty.
    EmptyGrapheme,
}

impl std::fmt::Display for GraphemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CapacityExceeded { limit } =>
                write!(f, "grapheme arena capacity exceeded (limit: {limit})"),
            Self::InvalidIndex(idx) =>
                write!(f, "invalid grapheme arena index: 0x{idx:04X}"),
            Self::EmptyGrapheme =>
                write!(f, "empty grapheme string"),
        }
    }
}

impl std::error::Error for GraphemeError {}

// INV-005: GraphemeError is Send + Sync.
const _: () = {
    fn assert_send_sync<T: Send + Sync + 'static>() {}
    fn check() { assert_send_sync::<GraphemeError>(); }
};

/// Arena for storing extended grapheme clusters.
///
/// S92: 14-bit ext index. INV-030: index 0 = no extension.
#[derive(Debug, Clone)]
pub struct GraphemeArena {
    /// Index -> grapheme string.
    entries: Vec<String>,
    /// Grapheme string -> index (for deduplication).
    dedup: HashMap<String, u16>,
}

impl GraphemeArena {
    /// Create a new empty arena.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            dedup: HashMap::new(),
        }
    }

    /// Number of entries in the arena.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True if the arena has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Insert a grapheme into the arena, returning its 14-bit index.
    ///
    /// If the grapheme already exists, returns the existing index (dedup).
    /// INV-030: Index 0x0000 is never returned (reserved for "no extension").
    #[must_use]
    pub fn insert(&mut self, grapheme: &str) -> Result<u16, GraphemeError> {
        if grapheme.is_empty() {
            return Err(GraphemeError::EmptyGrapheme);
        }

        if let Some(&idx) = self.dedup.get(grapheme) {
            return Ok(idx);
        }

        if self.entries.len() >= MAX_ENTRIES {
            return Err(GraphemeError::CapacityExceeded { limit: MAX_ENTRIES });
        }

        // Allocate new index (1-based, since 0 is reserved)
        let idx = (self.entries.len() as u16) + 1;
        self.entries.push(grapheme.to_string());
        self.dedup.insert(grapheme.to_string(), idx);
        Ok(idx)
    }

    /// Look up a grapheme by its index.
    ///
    /// INV-030: Index 0 returns Err (no extension).
    #[must_use]
    pub fn get(&self, index: u16) -> Result<&str, GraphemeError> {
        if index == 0 {
            return Err(GraphemeError::InvalidIndex(0));
        }
        let array_idx = (index - 1) as usize;
        self.entries
            .get(array_idx)
            .map(|s| s.as_str())
            .ok_or(GraphemeError::InvalidIndex(index))
    }

    /// Check if a grapheme already exists in the arena.
    #[must_use]
    pub fn contains(&self, grapheme: &str) -> bool {
        self.dedup.contains_key(grapheme)
    }

    /// Get the index of an existing grapheme, or None if not present.
    #[must_use]
    pub fn index_of(&self, grapheme: &str) -> Option<u16> {
        self.dedup.get(grapheme).copied()
    }

    /// Encode the arena to bytes for snapshot serialization.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let count = self.entries.len() as u16;
        buf.extend_from_slice(&count.to_le_bytes());
        for entry in &self.entries {
            let len = entry.len() as u16;
            buf.extend_from_slice(&len.to_le_bytes());
            buf.extend_from_slice(entry.as_bytes());
        }
        buf
    }

    /// Decode an arena from bytes.
    #[must_use]
    pub fn decode(data: &[u8]) -> Result<Self, GraphemeError> {
        if data.len() < 2 {
            return Ok(Self::new());
        }
        let count = u16::from_le_bytes([data[0], data[1]]) as usize;
        let mut arena = Self::new();
        let mut offset = 2;

        for _ in 0..count {
            if offset + 2 > data.len() {
                break;
            }
            let len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
            offset += 2;
            if offset + len > data.len() {
                break;
            }
            let grapheme = String::from_utf8_lossy(&data[offset..offset + len]).to_string();
            offset += len;
            let _ = arena.insert(&grapheme);
        }

        Ok(arena)
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
    fn test_insert_and_get() {
        let mut arena = GraphemeArena::new();
        let idx = arena.insert("\u{1F1FA}\u{1F1F8}").unwrap(); // US flag
        assert_eq!(idx, 1); // first index is 1
        assert_eq!(arena.get(idx).unwrap(), "\u{1F1FA}\u{1F1F8}");
    }

    /// INV-030: Index 0 means "no extension".
    #[test]
    fn test_index_zero_is_invalid() {
        let arena = GraphemeArena::new();
        assert!(arena.get(0).is_err());
    }

    /// Deduplication: same grapheme returns same index.
    #[test]
    fn test_dedup() {
        let mut arena = GraphemeArena::new();
        let idx1 = arena.insert("flag-\u{1F1FA}").unwrap();
        let idx2 = arena.insert("flag-\u{1F1FA}").unwrap();
        assert_eq!(idx1, idx2);
        assert_eq!(arena.len(), 1);
    }

    /// Empty grapheme is rejected.
    #[test]
    fn test_empty_grapheme_rejected() {
        let mut arena = GraphemeArena::new();
        assert!(matches!(
            arena.insert(""),
            Err(GraphemeError::EmptyGrapheme)
        ));
    }

    /// Encode/decode round-trip.
    #[test]
    fn test_encode_decode_round_trip() {
        let mut arena = GraphemeArena::new();
        arena.insert("hello").unwrap();
        arena.insert("\u{1F600}").unwrap(); // grinning face
        arena.insert("\u{0301}combined").unwrap();

        let encoded = arena.encode();
        let decoded = GraphemeArena::decode(&encoded).unwrap();

        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded.get(1).unwrap(), "hello");
        assert_eq!(decoded.get(2).unwrap(), "\u{1F600}");
        assert_eq!(decoded.get(3).unwrap(), "\u{0301}combined");
    }

    /// contains() and index_of() work.
    #[test]
    fn test_contains_and_index_of() {
        let mut arena = GraphemeArena::new();
        arena.insert("test").unwrap();
        assert!(arena.contains("test"));
        assert!(!arena.contains("nope"));
        assert_eq!(arena.index_of("test"), Some(1));
        assert_eq!(arena.index_of("nope"), None);
    }

    /// Out-of-bounds index returns error.
    #[test]
    fn test_out_of_bounds_index() {
        let arena = GraphemeArena::new();
        assert!(matches!(arena.get(99), Err(GraphemeError::InvalidIndex(99))));
    }

    /// Decode empty data.
    #[test]
    fn test_decode_empty() {
        let arena = GraphemeArena::decode(&[]).unwrap();
        assert!(arena.is_empty());
    }
}
