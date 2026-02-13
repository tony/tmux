use mux_kernel::Session;
use mux_config::Config;

pub struct TermForge {
    sessions: Vec<Session>,
    config: Config,
}

impl TermForge {
    pub fn new(config: Config) -> Self {
        Self {
            sessions: Vec::new(),
            config,
        }
    }

    pub fn start(&self) {
        println!("Starting TermForge...");
    }
}
