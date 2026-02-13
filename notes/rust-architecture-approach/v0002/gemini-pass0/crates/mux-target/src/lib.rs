// Parses session:window.pane syntax

#[derive(Debug, PartialEq)]
pub struct Target {
    pub session: Option<String>,
    pub window: Option<String>,
    pub pane: Option<String>,
}

pub fn parse_target(s: &str) -> Option<Target> {
    todo!()
}
--- END FILE ---
