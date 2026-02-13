use std::collections::HashMap;

#[derive(Default)]
pub struct GraphemeArena {
    storage: Vec<String>,
    lookup: HashMap<String, usize>,
}

impl GraphemeArena {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, s: &str) -> usize {
        if let Some(&idx) = self.lookup.get(s) {
            return idx;
        }
        let idx = self.storage.len();
        self.storage.push(s.to_string());
        self.lookup.insert(s.to_string(), idx);
        idx
    }

    pub fn get(&self, idx: usize) -> Option<&str> {
        self.storage.get(idx).map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_new() {
        let mut arena = GraphemeArena::new();
        let idx = arena.intern("a");
        assert_eq!(idx, 0);
        assert_eq!(arena.get(0), Some("a"));
    }

    #[test]
    fn test_intern_existing() {
        let mut arena = GraphemeArena::new();
        let idx1 = arena.intern("abc");
        let idx2 = arena.intern("abc");
        assert_eq!(idx1, idx2);
        assert_eq!(arena.storage.len(), 1);
    }

    #[test]
    fn test_intern_multiple() {
        let mut arena = GraphemeArena::new();
        arena.intern("a");
        arena.intern("b");
        assert_eq!(arena.storage.len(), 2);
    }

    #[test]
    fn test_get_invalid() {
        let arena = GraphemeArena::new();
        assert_eq!(arena.get(999), None);
    }

    #[test]
    fn test_intern_empty() {
        let mut arena = GraphemeArena::new();
        let idx = arena.intern("");
        assert_eq!(arena.get(idx), Some(""));
    }
}
