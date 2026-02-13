//! Vi/Emacs copy mode state machine.

/// Key table for copy mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyModeKeyTable {
    Vi,
    Emacs,
}

/// Selection mode in copy mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    Character,
    Line,
    Block,
}

/// Copy mode state.
#[derive(Debug)]
pub struct CopyMode {
    key_table: CopyModeKeyTable,
    selection_mode: Option<SelectionMode>,
    cursor_x: u16,
    cursor_y: u16,
    /// Search pattern (if searching).
    search_pattern: Option<String>,
    /// Buffer ring for copy operations (bounded, max 50).
    buffer_ring: Vec<String>,
}

impl CopyMode {
    /// Maximum buffer ring size.
    pub const MAX_BUFFERS: usize = 50;

    /// Create a new copy mode with the given key table.
    #[must_use]
    pub fn new(key_table: CopyModeKeyTable) -> Self {
        Self {
            key_table,
            selection_mode: None,
            cursor_x: 0,
            cursor_y: 0,
            search_pattern: None,
            buffer_ring: Vec::new(),
        }
    }

    /// Start a selection.
    pub fn start_selection(&mut self, mode: SelectionMode) {
        self.selection_mode = Some(mode);
    }

    /// Cancel the selection.
    pub fn cancel_selection(&mut self) {
        self.selection_mode = None;
    }

    /// Current selection mode.
    #[must_use]
    pub const fn selection_mode(&self) -> Option<SelectionMode> {
        self.selection_mode
    }

    /// Move cursor.
    pub fn move_cursor(&mut self, x: u16, y: u16) {
        self.cursor_x = x;
        self.cursor_y = y;
    }

    /// Push a buffer onto the ring.
    pub fn push_buffer(&mut self, text: String) {
        if self.buffer_ring.len() >= Self::MAX_BUFFERS {
            self.buffer_ring.remove(0);
        }
        self.buffer_ring.push(text);
    }

    /// Number of buffers in the ring.
    #[must_use]
    pub fn buffer_count(&self) -> usize {
        self.buffer_ring.len()
    }

    /// Get the most recent buffer.
    #[must_use]
    pub fn latest_buffer(&self) -> Option<&str> {
        self.buffer_ring.last().map(String::as_str)
    }

    /// The active key table.
    #[must_use]
    pub const fn key_table(&self) -> CopyModeKeyTable {
        self.key_table
    }

    /// Set search pattern.
    pub fn set_search(&mut self, pattern: String) {
        self.search_pattern = Some(pattern);
    }

    /// Current search pattern.
    #[must_use]
    pub fn search_pattern(&self) -> Option<&str> {
        self.search_pattern.as_deref()
    }

    /// Cursor position.
    #[must_use]
    pub const fn cursor(&self) -> (u16, u16) {
        (self.cursor_x, self.cursor_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_copy_mode() {
        let cm = CopyMode::new(CopyModeKeyTable::Vi);
        assert_eq!(cm.key_table(), CopyModeKeyTable::Vi);
        assert!(cm.selection_mode().is_none());
        assert_eq!(cm.cursor(), (0, 0));
    }

    #[test]
    fn selection_lifecycle() {
        let mut cm = CopyMode::new(CopyModeKeyTable::Emacs);
        cm.start_selection(SelectionMode::Character);
        assert_eq!(cm.selection_mode(), Some(SelectionMode::Character));
        cm.cancel_selection();
        assert!(cm.selection_mode().is_none());
    }

    #[test]
    fn buffer_ring() {
        let mut cm = CopyMode::new(CopyModeKeyTable::Vi);
        cm.push_buffer("first".into());
        cm.push_buffer("second".into());
        assert_eq!(cm.buffer_count(), 2);
        assert_eq!(cm.latest_buffer(), Some("second"));
    }

    #[test]
    fn buffer_ring_bounded() {
        let mut cm = CopyMode::new(CopyModeKeyTable::Vi);
        for i in 0..=CopyMode::MAX_BUFFERS {
            cm.push_buffer(format!("buf-{i}"));
        }
        assert_eq!(cm.buffer_count(), CopyMode::MAX_BUFFERS);
    }

    #[test]
    fn cursor_movement() {
        let mut cm = CopyMode::new(CopyModeKeyTable::Vi);
        cm.move_cursor(10, 5);
        assert_eq!(cm.cursor(), (10, 5));
    }

    #[test]
    fn search_pattern() {
        let mut cm = CopyMode::new(CopyModeKeyTable::Vi);
        assert!(cm.search_pattern().is_none());
        cm.set_search("error".into());
        assert_eq!(cm.search_pattern(), Some("error"));
    }

    #[test]
    fn selection_modes() {
        assert_ne!(SelectionMode::Character, SelectionMode::Line);
        assert_ne!(SelectionMode::Line, SelectionMode::Block);
    }

    #[test]
    fn cursor_movement_independent_axes() {
        let mut cm = CopyMode::new(CopyModeKeyTable::Vi);
        cm.move_cursor(5, 0);
        assert_eq!(cm.cursor(), (5, 0));
        cm.move_cursor(5, 10);
        assert_eq!(cm.cursor(), (5, 10));
    }

    #[test]
    fn emacs_key_table() {
        let cm = CopyMode::new(CopyModeKeyTable::Emacs);
        assert_eq!(cm.key_table(), CopyModeKeyTable::Emacs);
    }

    #[test]
    fn buffer_ring_empty_latest() {
        let cm = CopyMode::new(CopyModeKeyTable::Vi);
        assert!(cm.latest_buffer().is_none());
    }
}
