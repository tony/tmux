use mux_types::{SessionId, WindowId, PaneId};
use mux_grid::Grid;
use slotmap::SlotMap;

pub struct Kernel {
    sessions: SlotMap<SessionId, Session>,
    windows: SlotMap<WindowId, Window>,
    panes: SlotMap<PaneId, Pane>,
    // channels for IO/Render
}

struct Session {
    name: String,
    windows: Vec<WindowId>,
}

struct Window {
    name: String,
    panes: Vec<PaneId>,
    layout: Layout,
}

struct Pane {
    grid: Grid,
    pty_id: Option<u32>,
}

struct Layout {
    // TBD
}

impl Kernel {
    pub fn new() -> Self {
        todo!()
    }

    pub fn run(&mut self) {
        // Event loop
        todo!()
    }
    
    fn handle_msg(&mut self, msg: mux_proto::Message) {
        todo!()
    }
}
--- END FILE ---
