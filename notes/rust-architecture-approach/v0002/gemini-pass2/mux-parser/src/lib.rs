pub mod byte_class;

use mux_types::{Cell, Key};

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Print(char),
    Execute(u8),
    CsiDispatch { params: Vec<i64>, intermediates: Vec<u8>, ignore: bool, action: char },
    OscDispatch { params: Vec<Vec<u8>>, bell_terminated: bool },
    // ...
}

pub struct Parser {
    state: State,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Ground,
    Escape,
    CsiEntry,
    CsiParam,
    // ...
}

impl Parser {
    pub fn new() -> Self {
        Self { state: State::Ground }
    }

    pub fn process(&mut self, byte: u8) -> Option<Action> {
        // Very simplified stub
        match byte {
            0x1B => { self.state = State::Escape; None }
            c if c >= 0x20 && c <= 0x7E => Some(Action::Print(c as char)),
            _ => None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print() {
        let mut p = Parser::new();
        let action = p.process(b'A');
        assert_eq!(action, Some(Action::Print('A')));
    }

    #[test]
    fn test_escape() {
        let mut p = Parser::new();
        let action = p.process(0x1B);
        assert!(action.is_none());
        // Internal state should be Escape, but it's private.
        // If we process a char now, it might be consumed depending on logic.
        // In stub, next char prints.
        let action2 = p.process(b'A');
        assert_eq!(action2, Some(Action::Print('A'))); 
    }

    #[test]
    fn test_multiple_chars() {
        let mut p = Parser::new();
        assert_eq!(p.process(b'H'), Some(Action::Print('H')));
        assert_eq!(p.process(b'i'), Some(Action::Print('i')));
    }
}
