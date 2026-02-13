use mux_types::{SessionId, WindowId, PaneId};

// The high-level SDK surface area

pub struct Server {
    // Handle to kernel
}

pub struct Session {
    pub id: SessionId,
}

pub struct Window {
    pub id: WindowId,
}

pub struct Pane {
    pub id: PaneId,
}

impl Server {
    pub fn new() -> anyhow::Result<Self> {
        todo!()
    }
    
    pub fn sessions(&self) -> Vec<Session> {
        todo!()
    }
    
    pub fn new_session(&self, name: &str) -> anyhow::Result<Session> {
        todo!()
    }
}

impl Session {
    pub fn windows(&self) -> Vec<Window> {
        todo!()
    }
    
    pub fn active_window(&self) -> anyhow::Result<Window> {
        todo!()
    }
}
--- END FILE ---
