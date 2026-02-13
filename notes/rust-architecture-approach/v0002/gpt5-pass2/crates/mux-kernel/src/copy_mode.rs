use mux_types::PaneId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyKeymap {
    Vi,
    Emacs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionKind {
    Char,
    Word,
    Line,
    Rect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyModeState {
    pub pane: PaneId,
    pub keymap: CopyKeymap,
    pub selection_kind: SelectionKind,
    pub anchor: (u16, u16),
    pub cursor: (u16, u16),
    pub search_query: Option<String>,
}

impl CopyModeState {
    #[must_use]
    pub fn new(pane: PaneId) -> Self {
        Self {
            pane,
            keymap: CopyKeymap::Vi,
            selection_kind: SelectionKind::Char,
            anchor: (0, 0),
            cursor: (0, 0),
            search_query: None,
        }
    }

    pub fn set_keymap(&mut self, k: CopyKeymap) {
        self.keymap = k;
    }

    pub fn set_selection_kind(&mut self, k: SelectionKind) {
        self.selection_kind = k;
    }

    pub fn begin_search(&mut self, query: String) {
        self.search_query = Some(query);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_copy_mode_defaults() {
        let s = CopyModeState::new(PaneId(1));
        assert_eq!(s.keymap, CopyKeymap::Vi);
    }

    #[test]
    fn keymap_switch_works() {
        let mut s = CopyModeState::new(PaneId(1));
        s.set_keymap(CopyKeymap::Emacs);
        assert_eq!(s.keymap, CopyKeymap::Emacs);
    }

    #[test]
    fn supports_all_selection_kinds() {
        let kinds = [SelectionKind::Char, SelectionKind::Word, SelectionKind::Line, SelectionKind::Rect];
        assert_eq!(kinds.len(), 4);
    }

    #[test]
    fn search_query_is_stored() {
        let mut s = CopyModeState::new(PaneId(5));
        s.begin_search("needle".into());
        assert_eq!(s.search_query.as_deref(), Some("needle"));
    }
}
