//! Session, Window, Pane, Client entity structs.

use mux_grid::ChunkedGrid;
use mux_grapheme_arena::GraphemeArena;
use mux_options::OptionTable;
use mux_types::{ClientId, PaneId, SessionId, Size, WindowId};

/// A terminal session containing windows.
#[derive(Debug)]
pub struct Session {
    pub id: SessionId,
    pub name: String,
    pub windows: Vec<WindowId>,
    pub active_window: Option<WindowId>,
    pub options: OptionTable,
    pub created: i64,
    pub attached_clients: Vec<ClientId>,
}

impl Session {
    pub fn new(id: SessionId, name: String, created: i64) -> Self {
        Self {
            id,
            name,
            windows: Vec::new(),
            active_window: None,
            options: OptionTable::new(),
            created,
            attached_clients: Vec::new(),
        }
    }
}

/// A window containing panes in a layout.
#[derive(Debug)]
pub struct Window {
    pub id: WindowId,
    pub session_id: SessionId,
    pub name: String,
    pub index: u32,
    pub panes: Vec<PaneId>,
    pub active_pane: Option<PaneId>,
    pub options: OptionTable,
    pub size: Size,
}

impl Window {
    pub fn new(id: WindowId, session_id: SessionId, name: String, index: u32, size: Size) -> Self {
        Self {
            id,
            session_id,
            name,
            index,
            panes: Vec::new(),
            active_pane: None,
            options: OptionTable::new(),
            size,
        }
    }
}

/// A terminal pane with grid, parser state, and process info.
#[derive(Debug)]
pub struct Pane {
    pub id: PaneId,
    pub window_id: WindowId,
    pub grid: ChunkedGrid,
    pub arena: GraphemeArena,
    pub parser: mux_parser::VtParser,
    pub size: Size,
    pub cursor_x: u32,
    pub cursor_y: u32,
    pub pid: Option<u32>,
    pub title: String,
    pub options: OptionTable,
}

impl Pane {
    pub fn new(id: PaneId, window_id: WindowId, size: Size, hlimit: u32) -> Self {
        Self {
            id,
            window_id,
            grid: ChunkedGrid::new(size.cols, size.rows, hlimit),
            arena: GraphemeArena::new(),
            parser: mux_parser::VtParser::new(),
            size,
            cursor_x: 0,
            cursor_y: 0,
            pid: None,
            title: String::new(),
            options: OptionTable::new(),
        }
    }
}

/// A connected client.
#[derive(Debug)]
pub struct Client {
    pub id: ClientId,
    pub session_id: Option<SessionId>,
    pub name: String,
    pub tty: String,
    pub size: Size,
}

impl Client {
    pub fn new(id: ClientId, name: String, size: Size) -> Self {
        Self {
            id,
            session_id: None,
            name,
            tty: String::new(),
            size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_new() {
        let s = Session::new(SessionId(1), "test".into(), 12345);
        assert_eq!(s.name, "test");
        assert!(s.windows.is_empty());
    }

    #[test]
    fn window_new() {
        let w = Window::new(WindowId(1), SessionId(1), "main".into(), 0, Size::new(80, 24));
        assert_eq!(w.name, "main");
        assert!(w.panes.is_empty());
    }

    #[test]
    fn pane_new() {
        let p = Pane::new(PaneId(1), WindowId(1), Size::new(80, 24), 10_000);
        assert_eq!(p.size, Size::new(80, 24));
        assert_eq!(p.cursor_x, 0);
    }

    #[test]
    fn client_new() {
        let c = Client::new(ClientId(1), "client0".into(), Size::new(80, 24));
        assert!(c.session_id.is_none());
    }
}
