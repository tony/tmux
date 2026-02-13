use slotmap::new_key_type;

new_key_type! { pub struct GraphemeId; }

pub struct GraphemeArena {
    // Implementation for storing complex unicode strings
    // to avoid storing String in every Cell
}

impl GraphemeArena {
    pub fn new() -> Self {
        todo!()
    }

    pub fn insert(&mut self, s: &str) -> GraphemeId {
        todo!()
    }

    pub fn get(&self, id: GraphemeId) -> Option<&str> {
        todo!()
    }
}
--- END FILE ---
