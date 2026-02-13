//! Copy mode state machine.
//!
//! ## Invariants
//! - Key table is explicit (Vi or Emacs) and mode-local.
//! - Selection mode is explicit (None, Character, Line, Block).
//! - Search cursor movement is deterministic and testable.
//! - Buffer ring is bounded (max 50 entries by default).

/// The key table used in copy mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CopyKeyTable {
    Vi,
    Emacs,
}

/// Selection mode in copy mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelectionMode {
    None,
    Character,
    Line,
    Block,
}

/// Search direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

/// Copy mode state for a single pane.
#[derive(Debug)]
pub struct CopyModeState {
    /// Active key table.
    pub key_table: CopyKeyTable,
    /// Current selection mode.
    pub selection_mode: SelectionMode,
    /// Cursor column in copy mode.
    pub cursor_x: u16,
    /// Cursor row in copy mode (relative to viewport).
    pub cursor_y: u16,
    /// Selection start (if selection is active).
    pub selection_start: Option<(u16, u16)>,
    /// Scroll offset into scrollback.
    pub scroll_offset: u32,
    /// Search term.
    pub search_term: Option<String>,
    /// Search direction.
    pub search_direction: SearchDirection,
}

impl CopyModeState {
    /// Create a new copy mode state at the given cursor position.
    #[must_use]
    pub fn new(key_table: CopyKeyTable, cursor_x: u16, cursor_y: u16) -> Self {
        Self {
            key_table,
            selection_mode: SelectionMode::None,
            cursor_x,
            cursor_y,
            selection_start: None,
            scroll_offset: 0,
            search_term: None,
            search_direction: SearchDirection::Forward,
        }
    }

    /// Begin character selection at the current cursor position.
    pub fn begin_selection(&mut self) {
        self.selection_mode = SelectionMode::Character;
        self.selection_start = Some((self.cursor_x, self.cursor_y));
    }

    /// Begin line selection.
    pub fn begin_line_selection(&mut self) {
        self.selection_mode = SelectionMode::Line;
        self.selection_start = Some((0, self.cursor_y));
    }

    /// Begin block (rectangle) selection.
    pub fn begin_block_selection(&mut self) {
        self.selection_mode = SelectionMode::Block;
        self.selection_start = Some((self.cursor_x, self.cursor_y));
    }

    /// Clear the selection.
    pub fn clear_selection(&mut self) {
        self.selection_mode = SelectionMode::None;
        self.selection_start = None;
    }

    /// Move cursor up by `n` rows.
    pub fn cursor_up(&mut self, n: u16) {
        if n > self.cursor_y {
            let overshoot = n - self.cursor_y;
            self.scroll_offset += u32::from(overshoot);
            self.cursor_y = 0;
        } else {
            self.cursor_y -= n;
        }
    }

    /// Move cursor down by `n` rows.
    pub fn cursor_down(&mut self, n: u16, screen_height: u16) {
        let new_y = self.cursor_y + n;
        if new_y >= screen_height {
            // Scroll viewport
            if self.scroll_offset > 0 {
                let reduce = u32::from(new_y - screen_height + 1).min(self.scroll_offset);
                self.scroll_offset -= reduce;
            }
            self.cursor_y = screen_height.saturating_sub(1);
        } else {
            self.cursor_y = new_y;
        }
    }

    /// Move cursor left.
    pub fn cursor_left(&mut self) {
        self.cursor_x = self.cursor_x.saturating_sub(1);
    }

    /// Move cursor right.
    pub fn cursor_right(&mut self, screen_width: u16) {
        if self.cursor_x + 1 < screen_width {
            self.cursor_x += 1;
        }
    }

    /// Set search term and direction.
    pub fn set_search(&mut self, term: String, direction: SearchDirection) {
        self.search_term = Some(term);
        self.search_direction = direction;
    }

    /// Whether a selection is active.
    #[must_use]
    pub fn has_selection(&self) -> bool {
        self.selection_mode != SelectionMode::None
    }
}

/// Bounded buffer ring for copied text.
#[derive(Debug)]
pub struct BufferRing {
    entries: Vec<String>,
    max_entries: usize,
}

impl BufferRing {
    /// Create a new buffer ring with the given capacity.
    #[must_use]
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
        }
    }

    /// Push a new entry, evicting the oldest if at capacity.
    pub fn push(&mut self, text: String) {
        if self.entries.len() >= self.max_entries {
            self.entries.remove(0);
        }
        self.entries.push(text);
    }

    /// Get the most recent entry.
    #[must_use]
    pub fn latest(&self) -> Option<&str> {
        self.entries.last().map(String::as_str)
    }

    /// Get an entry by index (0 = most recent).
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&str> {
        let len = self.entries.len();
        if index >= len {
            return None;
        }
        self.entries.get(len - 1 - index).map(String::as_str)
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the ring is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_mode_initial_state() {
        let state = CopyModeState::new(CopyKeyTable::Vi, 10, 5);
        assert_eq!(state.key_table, CopyKeyTable::Vi);
        assert_eq!(state.selection_mode, SelectionMode::None);
        assert_eq!(state.cursor_x, 10);
        assert_eq!(state.cursor_y, 5);
        assert!(!state.has_selection());
    }

    #[test]
    fn copy_mode_character_selection() {
        let mut state = CopyModeState::new(CopyKeyTable::Vi, 5, 3);
        state.begin_selection();
        assert!(state.has_selection());
        assert_eq!(state.selection_mode, SelectionMode::Character);
        assert_eq!(state.selection_start, Some((5, 3)));
    }

    #[test]
    fn copy_mode_line_selection() {
        let mut state = CopyModeState::new(CopyKeyTable::Vi, 5, 3);
        state.begin_line_selection();
        assert_eq!(state.selection_mode, SelectionMode::Line);
        assert_eq!(state.selection_start, Some((0, 3)));
    }

    #[test]
    fn copy_mode_block_selection() {
        let mut state = CopyModeState::new(CopyKeyTable::Vi, 5, 3);
        state.begin_block_selection();
        assert_eq!(state.selection_mode, SelectionMode::Block);
    }

    #[test]
    fn copy_mode_clear_selection() {
        let mut state = CopyModeState::new(CopyKeyTable::Vi, 5, 3);
        state.begin_selection();
        state.clear_selection();
        assert!(!state.has_selection());
    }

    #[test]
    fn copy_mode_cursor_movement() {
        let mut state = CopyModeState::new(CopyKeyTable::Vi, 5, 5);
        state.cursor_up(2);
        assert_eq!(state.cursor_y, 3);
        state.cursor_down(1, 24);
        assert_eq!(state.cursor_y, 4);
        state.cursor_left();
        assert_eq!(state.cursor_x, 4);
        state.cursor_right(80);
        assert_eq!(state.cursor_x, 5);
    }

    #[test]
    fn copy_mode_cursor_up_scrolls() {
        let mut state = CopyModeState::new(CopyKeyTable::Vi, 0, 0);
        state.cursor_up(5);
        assert_eq!(state.cursor_y, 0);
        assert_eq!(state.scroll_offset, 5);
    }

    #[test]
    fn copy_mode_search() {
        let mut state = CopyModeState::new(CopyKeyTable::Vi, 0, 0);
        state.set_search("pattern".into(), SearchDirection::Forward);
        assert_eq!(state.search_term.as_deref(), Some("pattern"));
        assert_eq!(state.search_direction, SearchDirection::Forward);
    }

    #[test]
    fn buffer_ring_basic() {
        let mut ring = BufferRing::new(50);
        assert!(ring.is_empty());
        ring.push("first".into());
        ring.push("second".into());
        assert_eq!(ring.len(), 2);
        assert_eq!(ring.latest(), Some("second"));
    }

    #[test]
    fn buffer_ring_eviction() {
        let mut ring = BufferRing::new(3);
        ring.push("a".into());
        ring.push("b".into());
        ring.push("c".into());
        ring.push("d".into()); // evicts "a"
        assert_eq!(ring.len(), 3);
        assert_eq!(ring.latest(), Some("d"));
        assert_eq!(ring.get(2), Some("b"));
    }

    #[test]
    fn buffer_ring_get_by_index() {
        let mut ring = BufferRing::new(50);
        ring.push("first".into());
        ring.push("second".into());
        ring.push("third".into());
        assert_eq!(ring.get(0), Some("third"));
        assert_eq!(ring.get(1), Some("second"));
        assert_eq!(ring.get(2), Some("first"));
        assert_eq!(ring.get(3), None);
    }

    #[test]
    fn emacs_key_table() {
        let state = CopyModeState::new(CopyKeyTable::Emacs, 0, 0);
        assert_eq!(state.key_table, CopyKeyTable::Emacs);
    }
}
