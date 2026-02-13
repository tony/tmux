use mux_pty::Pty;
use mux_grid::Grid;
use mux_snapshot::GridSnapshot;
use std::time::Duration;

pub struct Termlet {
    pty: Pty,
    grid: Grid,
    // channels etc
}

pub struct TermletBuilder {
    cols: u16,
    rows: u16,
    cmd: String,
    env: Vec<(String, String)>,
}

impl TermletBuilder {
    pub fn new() -> Self {
        todo!()
    }
    
    pub fn size(mut self, cols: u16, rows: u16) -> Self {
        self.cols = cols;
        self.rows = rows;
        self
    }
    
    pub fn shell(mut self, shell: &str) -> Self {
        self.cmd = shell.to_string();
        self
    }
    
    pub fn env(mut self, key: &str, val: &str) -> Self {
        self.env.push((key.to_string(), val.to_string()));
        self
    }

    pub fn build(self) -> anyhow::Result<Termlet> {
        todo!()
    }
}

impl Termlet {
    pub fn builder() -> TermletBuilder {
        TermletBuilder::new()
    }

    pub fn send_keys(&self, keys: &str) -> anyhow::Result<()> {
        todo!()
    }

    pub fn wait_for(&self, text: &str, timeout: Duration) -> anyhow::Result<()> {
        todo!()
    }

    pub fn capture(&self) -> anyhow::Result<GridSnapshot> {
        todo!()
    }
}
--- END FILE ---
