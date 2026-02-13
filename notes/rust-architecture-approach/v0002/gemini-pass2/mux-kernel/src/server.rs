use crate::session::Session;

pub struct Server {
    pub sessions: Vec<Session>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
        }
    }
}
