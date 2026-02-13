//! Copy mode state machine with selection, search, and paste buffer ring.

use mux_types::geometry::Position;

/// Selection mode in copy mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    None,
    Character,
    Line,
    Block,
}

/// Key table for copy mode bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyKeyTable {
    Vi,
    Emacs,
}

/// Copy mode state for a pane.
#[derive(Debug, Clone)]
pub struct CopyModeState {
    pub active: bool,
    pub cursor: Position,
    pub anchor: Option<Position>,
    pub selection_mode: SelectionMode,
    pub key_table: CopyKeyTable,
    pub search_pattern: Option<String>,
    pub scroll_offset: u32,
}

impl Default for CopyModeState {
    fn default() -> Self {
        Self {
            active: false,
            cursor: Position::ORIGIN,
            anchor: None,
            selection_mode: SelectionMode::None,
            key_table: CopyKeyTable::Vi,
            search_pattern: None,
            scroll_offset: 0,
        }
    }
}

impl CopyModeState {
    pub fn enter(&mut self) {
        self.active = true;
        self.cursor = Position::ORIGIN;
        self.anchor = None;
        self.selection_mode = SelectionMode::None;
        self.scroll_offset = 0;
    }

    pub fn exit(&mut self) {
        self.active = false;
        self.anchor = None;
        self.selection_mode = SelectionMode::None;
        self.search_pattern = None;
    }

    pub fn begin_selection(&mut self) {
        self.anchor = Some(self.cursor);
        self.selection_mode = SelectionMode::Character;
    }

    pub fn has_selection(&self) -> bool {
        self.anchor.is_some() && self.selection_mode != SelectionMode::None
    }

    pub fn cursor_up(&mut self, n: u16) {
        self.cursor.y = self.cursor.y.saturating_sub(n);
    }

    pub fn cursor_down(&mut self, n: u16, max_y: u16) {
        self.cursor.y = (self.cursor.y + n).min(max_y);
    }

    pub fn cursor_left(&mut self, n: u16) {
        self.cursor.x = self.cursor.x.saturating_sub(n);
    }

    pub fn cursor_right(&mut self, n: u16, max_x: u16) {
        self.cursor.x = (self.cursor.x + n).min(max_x);
    }

    pub fn begin_search(&mut self, pattern: &str) {
        self.search_pattern = Some(pattern.to_owned());
    }

    pub fn selection_range(&self) -> Option<(Position, Position)> {
        self.anchor.map(|anchor| {
            if anchor.y < self.cursor.y
                || (anchor.y == self.cursor.y && anchor.x <= self.cursor.x)
            {
                (anchor, self.cursor)
            } else {
                (self.cursor, anchor)
            }
        })
    }
}

/// Bounded paste buffer ring.
#[derive(Debug, Clone)]
pub struct PasteBufferRing {
    buffers: Vec<String>,
    max_size: usize,
}

impl PasteBufferRing {
    pub fn new(max_size: usize) -> Self {
        Self {
            buffers: Vec::new(),
            max_size,
        }
    }

    pub fn push(&mut self, text: String) {
        if self.buffers.len() >= self.max_size {
            self.buffers.remove(0);
        }
        self.buffers.push(text);
    }

    pub fn latest(&self) -> Option<&str> {
        self.buffers.last().map(String::as_str)
    }

    pub fn get(&self, index: usize) -> Option<&str> {
        let len = self.buffers.len();
        if index < len {
            Some(&self.buffers[len - 1 - index])
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.buffers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }

    pub fn clear(&mut self) {
        self.buffers.clear();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn default_copy_mode_not_active() {
        let cm = CopyModeState::default();
        assert!(!cm.active);
        assert_eq!(cm.cursor, Position::ORIGIN);
        assert_eq!(cm.selection_mode, SelectionMode::None);
    }

    #[test]
    fn enter_copy_mode() {
        let mut cm = CopyModeState::default();
        cm.enter();
        assert!(cm.active);
        assert_eq!(cm.cursor, Position::ORIGIN);
    }

    #[test]
    fn exit_copy_mode() {
        let mut cm = CopyModeState::default();
        cm.enter();
        cm.exit();
        assert!(!cm.active);
        assert!(cm.anchor.is_none());
        assert_eq!(cm.selection_mode, SelectionMode::None);
    }

    #[test]
    fn begin_selection() {
        let mut cm = CopyModeState::default();
        cm.enter();
        cm.cursor = Position { x: 5, y: 3 };
        cm.begin_selection();
        assert!(cm.has_selection());
        assert_eq!(cm.anchor, Some(Position { x: 5, y: 3 }));
        assert_eq!(cm.selection_mode, SelectionMode::Character);
    }

    #[test]
    fn no_selection_without_begin() {
        let cm = CopyModeState::default();
        assert!(!cm.has_selection());
    }

    #[test]
    fn cursor_up() {
        let mut cm = CopyModeState::default();
        cm.enter();
        cm.cursor = Position { x: 0, y: 10 };
        cm.cursor_up(3);
        assert_eq!(cm.cursor.y, 7);
    }

    #[test]
    fn cursor_up_saturates_at_zero() {
        let mut cm = CopyModeState::default();
        cm.enter();
        cm.cursor = Position { x: 0, y: 2 };
        cm.cursor_up(5);
        assert_eq!(cm.cursor.y, 0);
    }

    #[test]
    fn cursor_down() {
        let mut cm = CopyModeState::default();
        cm.enter();
        cm.cursor_down(5, 23);
        assert_eq!(cm.cursor.y, 5);
    }

    #[test]
    fn cursor_down_clamps_at_max() {
        let mut cm = CopyModeState::default();
        cm.enter();
        cm.cursor_down(30, 23);
        assert_eq!(cm.cursor.y, 23);
    }

    #[test]
    fn cursor_left() {
        let mut cm = CopyModeState::default();
        cm.enter();
        cm.cursor = Position { x: 10, y: 0 };
        cm.cursor_left(3);
        assert_eq!(cm.cursor.x, 7);
    }

    #[test]
    fn cursor_left_saturates() {
        let mut cm = CopyModeState::default();
        cm.enter();
        cm.cursor = Position { x: 2, y: 0 };
        cm.cursor_left(5);
        assert_eq!(cm.cursor.x, 0);
    }

    #[test]
    fn cursor_right() {
        let mut cm = CopyModeState::default();
        cm.enter();
        cm.cursor_right(5, 79);
        assert_eq!(cm.cursor.x, 5);
    }

    #[test]
    fn cursor_right_clamps_at_max() {
        let mut cm = CopyModeState::default();
        cm.enter();
        cm.cursor_right(100, 79);
        assert_eq!(cm.cursor.x, 79);
    }

    #[test]
    fn selection_range_anchor_before_cursor() {
        let mut cm = CopyModeState::default();
        cm.enter();
        cm.cursor = Position { x: 2, y: 1 };
        cm.begin_selection();
        cm.cursor = Position { x: 10, y: 5 };
        let (start, end) = cm.selection_range().unwrap();
        assert_eq!(start, Position { x: 2, y: 1 });
        assert_eq!(end, Position { x: 10, y: 5 });
    }

    #[test]
    fn selection_range_cursor_before_anchor() {
        let mut cm = CopyModeState::default();
        cm.enter();
        cm.cursor = Position { x: 10, y: 5 };
        cm.begin_selection();
        cm.cursor = Position { x: 2, y: 1 };
        let (start, end) = cm.selection_range().unwrap();
        assert_eq!(start, Position { x: 2, y: 1 });
        assert_eq!(end, Position { x: 10, y: 5 });
    }

    #[test]
    fn selection_range_same_line() {
        let mut cm = CopyModeState::default();
        cm.enter();
        cm.cursor = Position { x: 5, y: 3 };
        cm.begin_selection();
        cm.cursor = Position { x: 10, y: 3 };
        let (start, end) = cm.selection_range().unwrap();
        assert_eq!(start.x, 5);
        assert_eq!(end.x, 10);
    }

    #[test]
    fn no_selection_range_without_anchor() {
        let cm = CopyModeState::default();
        assert!(cm.selection_range().is_none());
    }

    #[test]
    fn begin_search() {
        let mut cm = CopyModeState::default();
        cm.enter();
        cm.begin_search("hello");
        assert_eq!(cm.search_pattern.as_deref(), Some("hello"));
    }

    #[test]
    fn exit_clears_search() {
        let mut cm = CopyModeState::default();
        cm.enter();
        cm.begin_search("test");
        cm.exit();
        assert!(cm.search_pattern.is_none());
    }

    #[test]
    fn key_table_default_vi() {
        let cm = CopyModeState::default();
        assert_eq!(cm.key_table, CopyKeyTable::Vi);
    }

    #[test]
    fn scroll_offset_default_zero() {
        let cm = CopyModeState::default();
        assert_eq!(cm.scroll_offset, 0);
    }

    // PasteBufferRing tests

    #[test]
    fn empty_ring() {
        let ring = PasteBufferRing::new(10);
        assert!(ring.is_empty());
        assert_eq!(ring.len(), 0);
        assert!(ring.latest().is_none());
    }

    #[test]
    fn push_and_latest() {
        let mut ring = PasteBufferRing::new(10);
        ring.push("hello".to_owned());
        assert_eq!(ring.latest(), Some("hello"));
    }

    #[test]
    fn push_multiple_latest_is_last() {
        let mut ring = PasteBufferRing::new(10);
        ring.push("first".to_owned());
        ring.push("second".to_owned());
        ring.push("third".to_owned());
        assert_eq!(ring.latest(), Some("third"));
    }

    #[test]
    fn get_index_zero_is_latest() {
        let mut ring = PasteBufferRing::new(10);
        ring.push("a".to_owned());
        ring.push("b".to_owned());
        assert_eq!(ring.get(0), Some("b"));
        assert_eq!(ring.get(1), Some("a"));
    }

    #[test]
    fn get_out_of_bounds() {
        let mut ring = PasteBufferRing::new(10);
        ring.push("x".to_owned());
        assert!(ring.get(5).is_none());
    }

    #[test]
    fn ring_overflow_evicts_oldest() {
        let mut ring = PasteBufferRing::new(3);
        ring.push("a".to_owned());
        ring.push("b".to_owned());
        ring.push("c".to_owned());
        ring.push("d".to_owned());
        assert_eq!(ring.len(), 3);
        assert_eq!(ring.get(0), Some("d"));
        assert_eq!(ring.get(1), Some("c"));
        assert_eq!(ring.get(2), Some("b"));
    }

    #[test]
    fn ring_clear() {
        let mut ring = PasteBufferRing::new(10);
        ring.push("data".to_owned());
        ring.clear();
        assert!(ring.is_empty());
    }

    #[test]
    fn ring_size_one() {
        let mut ring = PasteBufferRing::new(1);
        ring.push("a".to_owned());
        ring.push("b".to_owned());
        assert_eq!(ring.len(), 1);
        assert_eq!(ring.latest(), Some("b"));
    }

    #[test]
    fn ring_len_tracks_insertions() {
        let mut ring = PasteBufferRing::new(50);
        for i in 0..25 {
            ring.push(format!("item-{i}"));
        }
        assert_eq!(ring.len(), 25);
    }
}
