//! # mux-parser
//!
//! VT parser state machine using static byte classification.

#![forbid(unsafe_code)]

pub mod byte_class;

pub use byte_class::{classify, classify_match, classify_oracle, ByteClass, CLASS_TABLE};

/// Parser state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParserState {
    Ground,
    Escape,
    EscapeIntermediate,
    CsiEntry,
    CsiParam,
    CsiIntermediate,
    CsiIgnore,
    DcsEntry,
    DcsParam,
    DcsIntermediate,
    DcsPassthrough,
    DcsIgnore,
    OscString,
    SosPmApcString,
}

/// Effect-free parser action record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    None,
    Print(char),
    Execute(u8),
    CsiDispatch {
        params: Vec<u16>,
        intermediates: Vec<u8>,
        final_byte: u8,
    },
    EscDispatch {
        intermediates: Vec<u8>,
        final_byte: u8,
    },
    DcsHook {
        params: Vec<u16>,
        intermediates: Vec<u8>,
        final_byte: u8,
    },
    DcsPut(u8),
    DcsUnhook,
    OscStart,
    OscPut(u8),
    OscEnd,
    Collect(u8),
    Param(u8),
    Clear,
}

/// Stateful VT parser.
#[derive(Debug, Clone)]
pub struct Parser {
    state: ParserState,
    params: Vec<u16>,
    intermediates: Vec<u8>,
    current_param: u16,
    osc_data: Vec<u8>,
}

impl Parser {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: ParserState::Ground,
            params: Vec::new(),
            intermediates: Vec::new(),
            current_param: 0,
            osc_data: Vec::new(),
        }
    }

    #[must_use]
    pub fn state(&self) -> ParserState {
        self.state
    }

    /// INV-027: pure transition in `(state, byte) -> action + next state` form.
    pub fn step(&mut self, byte: u8) -> Action {
        let class = classify(byte);

        match (self.state, class) {
            (ParserState::Ground, ByteClass::Printable) => Action::Print(byte as char),
            (ParserState::Ground, ByteClass::Control) => Action::Execute(byte),
            (ParserState::Ground, ByteClass::Escape) => {
                self.state = ParserState::Escape;
                self.params.clear();
                self.intermediates.clear();
                self.current_param = 0;
                Action::Clear
            }
            (ParserState::Ground, ByteClass::CsiEntry) => {
                self.state = ParserState::CsiEntry;
                self.params.clear();
                self.intermediates.clear();
                self.current_param = 0;
                Action::Clear
            }
            (ParserState::Ground, ByteClass::DcsEntry) => {
                self.state = ParserState::DcsEntry;
                self.params.clear();
                self.intermediates.clear();
                self.current_param = 0;
                Action::Clear
            }
            (ParserState::Ground, ByteClass::OscEntry) => {
                self.state = ParserState::OscString;
                self.osc_data.clear();
                Action::OscStart
            }
            (ParserState::Ground, ByteClass::Utf8Lead) => Action::Print(byte as char),

            (ParserState::Escape, ByteClass::Printable) => match byte {
                b'[' => {
                    self.state = ParserState::CsiEntry;
                    Action::Clear
                }
                b']' => {
                    self.state = ParserState::OscString;
                    self.osc_data.clear();
                    Action::OscStart
                }
                b'P' => {
                    self.state = ParserState::DcsEntry;
                    Action::Clear
                }
                _ => {
                    self.state = ParserState::Ground;
                    Action::EscDispatch {
                        intermediates: self.intermediates.clone(),
                        final_byte: byte,
                    }
                }
            },
            (ParserState::Escape, ByteClass::Control) => Action::Execute(byte),
            (ParserState::Escape, _) => {
                self.state = ParserState::Ground;
                Action::None
            }

            (ParserState::CsiEntry, ByteClass::Printable)
            | (ParserState::CsiParam, ByteClass::Printable) => match byte {
                b'0'..=b'9' => {
                    self.state = ParserState::CsiParam;
                    self.current_param = self
                        .current_param
                        .saturating_mul(10)
                        .saturating_add((byte - b'0') as u16);
                    Action::Param(byte)
                }
                b';' => {
                    self.state = ParserState::CsiParam;
                    self.params.push(self.current_param);
                    self.current_param = 0;
                    Action::Param(byte)
                }
                b' '..=b'/' => {
                    self.state = ParserState::CsiIntermediate;
                    self.intermediates.push(byte);
                    Action::Collect(byte)
                }
                b'@'..=b'~' => {
                    self.params.push(self.current_param);
                    self.state = ParserState::Ground;
                    Action::CsiDispatch {
                        params: self.params.clone(),
                        intermediates: self.intermediates.clone(),
                        final_byte: byte,
                    }
                }
                _ => Action::None,
            },
            (ParserState::CsiEntry, ByteClass::Control)
            | (ParserState::CsiParam, ByteClass::Control) => Action::Execute(byte),

            (ParserState::CsiIntermediate, ByteClass::Printable) => {
                if (b'@'..=b'~').contains(&byte) {
                    self.params.push(self.current_param);
                    self.state = ParserState::Ground;
                    Action::CsiDispatch {
                        params: self.params.clone(),
                        intermediates: self.intermediates.clone(),
                        final_byte: byte,
                    }
                } else {
                    self.intermediates.push(byte);
                    Action::Collect(byte)
                }
            }

            (ParserState::OscString, ByteClass::Printable) => {
                self.osc_data.push(byte);
                Action::OscPut(byte)
            }
            (ParserState::OscString, ByteClass::Control) => match byte {
                0x07 => {
                    self.state = ParserState::Ground;
                    Action::OscEnd
                }
                _ => Action::Execute(byte),
            },
            (ParserState::OscString, ByteClass::Escape) => {
                self.state = ParserState::Ground;
                Action::OscEnd
            }

            (ParserState::DcsEntry, ByteClass::Printable) => match byte {
                b'0'..=b'9' | b';' => {
                    self.state = ParserState::DcsParam;
                    Action::Param(byte)
                }
                b'@'..=b'~' => {
                    self.state = ParserState::DcsPassthrough;
                    Action::DcsHook {
                        params: self.params.clone(),
                        intermediates: self.intermediates.clone(),
                        final_byte: byte,
                    }
                }
                _ => {
                    self.state = ParserState::DcsPassthrough;
                    Action::DcsHook {
                        params: Vec::new(),
                        intermediates: Vec::new(),
                        final_byte: byte,
                    }
                }
            },
            (ParserState::DcsPassthrough, ByteClass::Printable) => Action::DcsPut(byte),
            (ParserState::DcsPassthrough, ByteClass::Escape) => {
                self.state = ParserState::Ground;
                Action::DcsUnhook
            }
            (ParserState::DcsPassthrough, ByteClass::Control) => {
                if byte == 0x1C {
                    self.state = ParserState::Ground;
                    Action::DcsUnhook
                } else {
                    Action::DcsPut(byte)
                }
            }

            (_, ByteClass::Escape) => {
                self.state = ParserState::Escape;
                self.params.clear();
                self.intermediates.clear();
                self.current_param = 0;
                Action::Clear
            }
            _ => Action::None,
        }
    }

    pub fn reset(&mut self) {
        self.state = ParserState::Ground;
        self.params.clear();
        self.intermediates.clear();
        self.current_param = 0;
        self.osc_data.clear();
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn representative(class: ByteClass) -> u8 {
        match class {
            ByteClass::Printable => b'A',
            ByteClass::Control => 0x01,
            ByteClass::Escape => 0x1B,
            ByteClass::CsiEntry => 0x9B,
            ByteClass::DcsEntry => 0x90,
            ByteClass::OscEntry => 0x9D,
            ByteClass::Utf8Lead => 0xC2,
        }
    }

    #[test]
    fn parser_ground_printable() {
        let mut p = Parser::new();
        let action = p.step(b'A');
        assert_eq!(action, Action::Print('A'));
        assert_eq!(p.state(), ParserState::Ground);
    }

    #[test]
    fn parser_csi_sequence_dispatches() {
        let mut p = Parser::new();
        p.step(0x1B);
        p.step(b'[');
        p.step(b'1');
        p.step(b';');
        p.step(b'2');
        let action = p.step(b'H');
        match action {
            Action::CsiDispatch {
                params, final_byte, ..
            } => {
                assert_eq!(params, vec![1, 2]);
                assert_eq!(final_byte, b'H');
            }
            other => panic!("expected CsiDispatch, got {other:?}"),
        }
        assert_eq!(p.state(), ParserState::Ground);
    }

    #[test]
    fn parser_osc_sequence_terminates_with_bel() {
        let mut p = Parser::new();
        p.step(0x9D);
        p.step(b'1');
        p.step(b';');
        let action = p.step(0x07);
        assert_eq!(action, Action::OscEnd);
        assert_eq!(p.state(), ParserState::Ground);
    }

    #[test]
    fn parser_reset_returns_to_ground() {
        let mut p = Parser::new();
        p.step(0x1B);
        assert_ne!(p.state(), ParserState::Ground);
        p.reset();
        assert_eq!(p.state(), ParserState::Ground);
    }

    #[test]
    fn parser_step_is_deterministic() {
        let bytes = [0x1B, b'[', b'1', b';', b'2', b'H'];

        let mut a = Parser::new();
        let mut b = Parser::new();

        let actions_a: Vec<Action> = bytes.into_iter().map(|x| a.step(x)).collect();
        let actions_b: Vec<Action> = bytes.into_iter().map(|x| b.step(x)).collect();

        assert_eq!(actions_a, actions_b);
        assert_eq!(a.state(), b.state());
    }

    #[test]
    fn parser_totality_for_representative_state_class_pairs() {
        let states = [
            ParserState::Ground,
            ParserState::Escape,
            ParserState::EscapeIntermediate,
            ParserState::CsiEntry,
            ParserState::CsiParam,
            ParserState::CsiIntermediate,
            ParserState::CsiIgnore,
            ParserState::DcsEntry,
            ParserState::DcsParam,
            ParserState::DcsIntermediate,
            ParserState::DcsPassthrough,
            ParserState::DcsIgnore,
            ParserState::OscString,
            ParserState::SosPmApcString,
        ];
        let classes = [
            ByteClass::Printable,
            ByteClass::Control,
            ByteClass::Escape,
            ByteClass::CsiEntry,
            ByteClass::DcsEntry,
            ByteClass::OscEntry,
            ByteClass::Utf8Lead,
        ];

        for state in states {
            for class in classes {
                let mut parser = Parser::new();
                parser.state = state;
                let _ = parser.step(representative(class));
            }
        }
    }

    #[test]
    fn parser_dcs_passthrough_unhooks_on_st_control() {
        let mut parser = Parser::new();
        parser.state = ParserState::DcsPassthrough;
        let action = parser.step(0x1C);
        assert_eq!(action, Action::DcsUnhook);
        assert_eq!(parser.state(), ParserState::Ground);
    }

    #[test]
    fn parser_escape_from_non_osc_state_clears_to_escape() {
        let mut parser = Parser::new();
        parser.state = ParserState::CsiParam;
        let action = parser.step(0x1B);
        assert_eq!(action, Action::Clear);
        assert_eq!(parser.state(), ParserState::Escape);
    }
}
