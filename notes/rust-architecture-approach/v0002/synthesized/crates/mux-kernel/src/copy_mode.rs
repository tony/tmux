//! Copy mode state machine (Vi and Emacs bindings).

/// Copy mode key table variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyModeKeyTable {
    Vi,
    Emacs,
}

/// Selection mode within copy mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    None,
    Character,
    Line,
    Block,
}

/// Copy mode state for a single pane.
#[derive(Debug)]
pub struct CopyModeState {
    pub key_table: CopyModeKeyTable,
    pub selection: SelectionMode,
    pub cursor_x: u32,
    pub cursor_y: u32,
    pub scroll_offset: u32,
    pub search_term: Option<String>,
    pub search_direction: SearchDirection,
    /// Copy buffer ring (bounded).
    pub buffers: Vec<String>,
    pub max_buffers: usize,
}

/// Search direction in copy mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

impl CopyModeState {
    /// Create a new copy mode state.
    #[must_use]
    pub fn new(key_table: CopyModeKeyTable) -> Self {
        Self {
            key_table,
            selection: SelectionMode::None,
            cursor_x: 0,
            cursor_y: 0,
            scroll_offset: 0,
            search_term: None,
            search_direction: SearchDirection::Forward,
            buffers: Vec::new(),
            max_buffers: 50,
        }
    }

    /// Begin character selection at current cursor.
    pub fn begin_selection(&mut self) {
        self.selection = SelectionMode::Character;
    }

    /// Switch to line selection.
    pub fn select_line(&mut self) {
        self.selection = SelectionMode::Line;
    }

    /// Switch to block (rectangle) selection.
    pub fn select_block(&mut self) {
        self.selection = SelectionMode::Block;
    }

    /// Cancel selection.
    pub fn cancel_selection(&mut self) {
        self.selection = SelectionMode::None;
    }

    /// Copy the current selection to the buffer ring.
    pub fn copy_selection(&mut self, text: String) {
        if self.buffers.len() >= self.max_buffers {
            self.buffers.remove(0);
        }
        self.buffers.push(text);
        self.selection = SelectionMode::None;
    }

    /// Get the most recent buffer content.
    #[must_use]
    pub fn last_buffer(&self) -> Option<&str> {
        self.buffers.last().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_copy_mode() {
        let cm = CopyModeState::new(CopyModeKeyTable::Vi);
        assert_eq!(cm.key_table, CopyModeKeyTable::Vi);
        assert_eq!(cm.selection, SelectionMode::None);
    }

    #[test]
    fn selection_cycle() {
        let mut cm = CopyModeState::new(CopyModeKeyTable::Vi);
        cm.begin_selection();
        assert_eq!(cm.selection, SelectionMode::Character);
        cm.select_line();
        assert_eq!(cm.selection, SelectionMode::Line);
        cm.select_block();
        assert_eq!(cm.selection, SelectionMode::Block);
        cm.cancel_selection();
        assert_eq!(cm.selection, SelectionMode::None);
    }

    #[test]
    fn copy_to_buffer_ring() {
        let mut cm = CopyModeState::new(CopyModeKeyTable::Emacs);
        cm.copy_selection("hello".into());
        assert_eq!(cm.last_buffer(), Some("hello"));
        cm.copy_selection("world".into());
        assert_eq!(cm.last_buffer(), Some("world"));
        assert_eq!(cm.buffers.len(), 2);
    }

    #[test]
    fn buffer_ring_bounded() {
        let mut cm = CopyModeState::new(CopyModeKeyTable::Vi);
        cm.max_buffers = 3;
        for i in 0..5 {
            cm.copy_selection(format!("buf{i}"));
        }
        assert_eq!(cm.buffers.len(), 3);
        assert_eq!(cm.last_buffer(), Some("buf4"));
    }
}
