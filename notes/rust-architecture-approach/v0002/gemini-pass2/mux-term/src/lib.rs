// mux-term: IO thread wrapper
use std::thread;

pub struct TerminalThread {
    handle: Option<thread::JoinHandle<()>>,
}

impl TerminalThread {
    pub fn start() -> Self {
        let handle = thread::spawn(|| {
            // blocking IO loop if needed
        });
        Self { handle: Some(handle) }
    }
}
