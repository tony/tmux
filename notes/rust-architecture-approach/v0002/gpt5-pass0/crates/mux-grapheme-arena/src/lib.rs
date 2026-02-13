use std::collections::HashMap;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GraphemeId(pub u32);

#[derive(Debug, Error)]
pub enum GraphemeArenaError {
    #[error("grapheme id {0:?} does not exist")]
    Missing(GraphemeId),
}

#[derive(Debug, Default)]
pub struct GraphemeArena {
    values: Vec<String>,
    index: HashMap<String, GraphemeId>,
}

impl GraphemeArena {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, grapheme: impl Into<String>) -> GraphemeId {
        let grapheme = grapheme.into();
        if let Some(id) = self.index.get(&grapheme).copied() {
            return id;
        }

        let id = GraphemeId(self.values.len() as u32);
        self.values.push(grapheme.clone());
        self.index.insert(grapheme, id);
        id
    }

    pub fn get(&self, id: GraphemeId) -> Result<&str, GraphemeArenaError> {
        self.values
            .get(id.0 as usize)
            .map(String::as_str)
            .ok_or(GraphemeArenaError::Missing(id))
    }
}
