use mux_types::Cell;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Print(char),
    Control(u8),
    Csi(CsiParams),
    Osc(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CsiParams {
    pub params: Vec<i64>,
    pub intermediates: Vec<u8>,
    pub final_byte: u8,
}

pub struct Parser {
    // state implementation (e.g. vte state machine)
}

impl Parser {
    pub fn new() -> Self {
        todo!()
    }

    pub fn process_byte(&mut self, byte: u8) -> Option<Action> {
        todo!()
    }
}
--- END FILE ---
