use std::sync::Arc;

use crate::Cell;

/// Immutable-by-default line with Arc-backed COW semantics (S96, INV-032).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    cells: Arc<Vec<Cell>>,
}

impl Line {
    pub fn new(cells: Vec<Cell>) -> Self {
        Self { cells: Arc::new(cells) }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn as_slice(&self) -> &[Cell] {
        self.cells.as_slice()
    }

    pub fn as_arc(&self) -> Arc<Vec<Cell>> {
        Arc::clone(&self.cells)
    }

    pub fn get(&self, idx: usize) -> Option<&Cell> {
        self.cells.get(idx)
    }

    pub fn push_mut(&mut self, cell: Cell) {
        Arc::make_mut(&mut self.cells).push(cell);
    }

    pub fn set_mut(&mut self, idx: usize, cell: Cell) {
        if let Some(slot) = Arc::make_mut(&mut self.cells).get_mut(idx) {
            *slot = cell;
        }
    }

    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.cells)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cell;

    #[test]
    fn line_cow_clone_shares_storage() {
        let line = Line::new(vec![Cell::from_char('a')]);
        let line2 = line.clone();
        assert_eq!(line.strong_count(), 2);
        assert_eq!(line2.strong_count(), 2);
    }

    #[test]
    fn line_mutation_detaches_storage() {
        let mut line = Line::new(vec![Cell::from_char('a')]);
        let line2 = line.clone();
        line.set_mut(0, Cell::from_char('b'));
        assert_eq!(line.get(0).unwrap().grapheme, "b");
        assert_eq!(line2.get(0).unwrap().grapheme, "a");
        assert_eq!(line.strong_count(), 1);
        assert_eq!(line2.strong_count(), 1);
    }

    #[test]
    fn push_updates_length() {
        let mut line = Line::empty();
        line.push_mut(Cell::from_char('x'));
        line.push_mut(Cell::from_char('y'));
        assert_eq!(line.len(), 2);
    }
}
