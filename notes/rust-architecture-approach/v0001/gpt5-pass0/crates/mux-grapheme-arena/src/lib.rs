#![forbid(unsafe_code)]

use std::collections::HashMap;

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

pub const EXT_BITS: u16 = 14;
pub const MAX_EXT_INDEX: u16 = (1 << EXT_BITS) - 1;
pub const RESERVED_NONE: u16 = 0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArenaError {
    Overflow,
    InvalidIndex(u16),
}

impl std::fmt::Display for ArenaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Overflow => f.write_str("grapheme arena overflow"),
            Self::InvalidIndex(i) => write!(f, "invalid grapheme index: {i}"),
        }
    }
}

impl std::error::Error for ArenaError {}

/// Canonical grapheme extension storage (S92, INV-030).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphemeArena {
    entries: Vec<CompactString>,
    #[serde(skip)]
    reverse: HashMap<CompactString, u16>,
}

impl PartialEq for GraphemeArena {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl Eq for GraphemeArena {}

impl GraphemeArena {
    pub fn new() -> Self {
        let mut entries = Vec::new();
        entries.push(CompactString::new(""));
        Self {
            entries,
            reverse: HashMap::new(),
        }
    }

    pub fn with_entries(entries: Vec<CompactString>) -> Result<Self, ArenaError> {
        if entries.is_empty() || entries.len() > (MAX_EXT_INDEX as usize + 1) {
            return Err(ArenaError::Overflow);
        }
        let mut arena = Self {
            entries,
            reverse: HashMap::new(),
        };
        arena.rebuild_index();
        Ok(arena)
    }

    pub fn insert(&mut self, grapheme: impl Into<CompactString>) -> Result<u16, ArenaError> {
        let grapheme = grapheme.into();
        if grapheme.is_empty() {
            return Ok(RESERVED_NONE);
        }
        if let Some(index) = self.reverse.get(&grapheme).copied() {
            return Ok(index);
        }
        if self.entries.len() > MAX_EXT_INDEX as usize {
            return Err(ArenaError::Overflow);
        }
        let index = self.entries.len() as u16;
        self.entries.push(grapheme.clone());
        self.reverse.insert(grapheme, index);
        Ok(index)
    }

    pub fn get(&self, index: u16) -> Result<Option<&str>, ArenaError> {
        if index == RESERVED_NONE {
            return Ok(None);
        }
        self.entries
            .get(index as usize)
            .map(|s| Some(s.as_str()))
            .ok_or(ArenaError::InvalidIndex(index))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.len() <= 1
    }

    pub fn rebuild_index(&mut self) {
        self.reverse.clear();
        for (i, entry) in self.entries.iter().enumerate().skip(1) {
            self.reverse.insert(entry.clone(), i as u16);
        }
    }

    pub fn as_entries(&self) -> &[CompactString] {
        &self.entries
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
    fn zero_index_reserved() {
        let arena = GraphemeArena::new();
        assert_eq!(arena.get(0).unwrap(), None);
    }

    #[test]
    fn insert_deduplicates() {
        let mut arena = GraphemeArena::new();
        let a = arena.insert("👩‍💻").unwrap();
        let b = arena.insert("👩‍💻").unwrap();
        assert_eq!(a, b);
        assert_eq!(arena.len(), 2);
    }

    #[test]
    fn invalid_index_errors() {
        let arena = GraphemeArena::new();
        assert!(matches!(arena.get(42), Err(ArenaError::InvalidIndex(42))));
    }

    #[test]
    fn rebuild_index_restores_lookup() {
        let mut arena = GraphemeArena::new();
        let idx = arena.insert("ab").unwrap();
        arena.rebuild_index();
        assert_eq!(arena.insert("ab").unwrap(), idx);
    }
}
