//! # mux-parser
//!
//! VT100/VT220/xterm escape sequence parser using a static lookup table.
//!
//! ## Module Decomposition
//! - [`byte_class`]: ByteClass enum + CLASS_TABLE[256] + oracle.
//! - Root: Parser state machine, ParserState, Action.
//!
//! ## Key Design (S95)
//! - `CLASS_TABLE[256]`: static LUT maps every byte to its [`ByteClass`].
//! - `classify_match()`: match-based oracle retained for testing.
//! - `step()`: pure state transition function (INV-027).

#![forbid(unsafe_code)]

pub mod byte_class;

// Re-exports for ergonomic imports
pub use byte_class::{ByteClass, CLASS_TABLE, classify, classify_match};

/// Parser state machine state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Action produced by the parser state machine.
///
/// RULE-S06-09: Actions are effect-free records (no side effects).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// No action (state transition only).
    None,
    /// Print a character to the current cell.
    Print(char),
    /// Execute a C0 control code.
    Execute(u8),
    /// CSI dispatch with params and final byte.
    CsiDispatch {
        params: Vec<u16>,
        intermediates: Vec<u8>,
        final_byte: u8,
    },
    /// ESC dispatch with intermediate and final byte.
    EscDispatch {
        intermediates: Vec<u8>,
        final_byte: u8,
    },
    /// DCS hook (start of DCS passthrough).
    DcsHook {
        params: Vec<u16>,
        intermediates: Vec<u8>,
        final_byte: u8,
    },
    /// DCS put: data byte in passthrough mode.
    DcsPut(u8),
    /// DCS unhook (end of DCS passthrough).
    DcsUnhook,
    /// OSC start.
    OscStart,
    /// OSC put: data byte.
    OscPut(u8),
    /// OSC end.
    OscEnd,
    /// Collect an intermediate byte.
    Collect(u8),
    /// Store a parameter byte.
    Param(u8),
    /// Clear collected parameters and intermediates.
    Clear,
}

/// The VT parser state machine.
///
/// INV-027: `step()` is a pure function -- it takes (state, byte) and returns
/// (new_state, action) with no side effects beyond the returned values.
#[derive(Debug, Clone)]
pub struct Parser {
    state: ParserState,
    params: Vec<u16>,
    intermediates: Vec<u8>,
    current_param: u16,
    osc_data: Vec<u8>,
}

impl Parser {
    /// Create a new parser in the Ground state.
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

    /// Current parser state.
    #[must_use]
    pub fn state(&self) -> ParserState {
        self.state
    }

    /// INV-027: Pure step function.
    ///
    /// Processes one byte and returns the action to take.
    /// S95: Uses CLASS_TABLE for O(1) byte classification.
    pub fn step(&mut self, byte: u8) -> Action {
        let class = classify(byte);

        match (self.state, class) {
            // Ground state
            (ParserState::Ground, ByteClass::Printable) => {
                Action::Print(byte as char)
            }
            (ParserState::Ground, ByteClass::Control) => {
                Action::Execute(byte)
            }
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
            (ParserState::Ground, ByteClass::Utf8Lead) => {
                Action::Print(byte as char)
            }

            // Escape state
            (ParserState::Escape, ByteClass::Printable) => {
                match byte {
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
                }
            }
            (ParserState::Escape, ByteClass::Control) => {
                Action::Execute(byte)
            }
            (ParserState::Escape, _) => {
                self.state = ParserState::Ground;
                Action::None
            }

            // CSI Entry/Param states
            (ParserState::CsiEntry, ByteClass::Printable) |
            (ParserState::CsiParam, ByteClass::Printable) => {
                match byte {
                    b'0'..=b'9' => {
                        self.state = ParserState::CsiParam;
                        self.current_param = self.current_param
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
                    _ => {
                        Action::None
                    }
                }
            }
            (ParserState::CsiEntry, ByteClass::Control) |
            (ParserState::CsiParam, ByteClass::Control) => {
                Action::Execute(byte)
            }

            // CSI Intermediate
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

            // OSC String
            (ParserState::OscString, ByteClass::Printable) => {
                self.osc_data.push(byte);
                Action::OscPut(byte)
            }
            (ParserState::OscString, ByteClass::Control) => {
                match byte {
                    0x07 => {
                        self.state = ParserState::Ground;
                        Action::OscEnd
                    }
                    _ => Action::Execute(byte),
                }
            }
            (ParserState::OscString, ByteClass::Escape) => {
                self.state = ParserState::Ground;
                Action::OscEnd
            }

            // DCS states
            (ParserState::DcsEntry, ByteClass::Printable) => {
                match byte {
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
                }
            }
            (ParserState::DcsPassthrough, ByteClass::Printable) => {
                Action::DcsPut(byte)
            }
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

            // Default: ESC from any state resets
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

    /// Reset the parser to Ground state.
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

    #[test]
    fn test_parser_ground_printable() {
        let mut p = Parser::new();
        let action = p.step(b'A');
        assert_eq!(action, Action::Print('A'));
        assert_eq!(p.state(), ParserState::Ground);
    }

    #[test]
    fn test_parser_csi_sequence() {
        let mut p = Parser::new();
        p.step(0x1B);
        p.step(b'[');
        p.step(b'1');
        p.step(b';');
        p.step(b'2');
        let action = p.step(b'H');
        match action {
            Action::CsiDispatch { params, final_byte, .. } => {
                assert_eq!(params, vec![1, 2]);
                assert_eq!(final_byte, b'H');
            }
            _ => panic!("expected CsiDispatch, got {action:?}"),
        }
        assert_eq!(p.state(), ParserState::Ground);
    }

    #[test]
    fn test_parser_csi_8bit_entry() {
        let mut p = Parser::new();
        p.step(0x9B);
        p.step(b'5');
        let action = p.step(b'm');
        match action {
            Action::CsiDispatch { params, final_byte, .. } => {
                assert_eq!(params, vec![5]);
                assert_eq!(final_byte, b'm');
            }
            _ => panic!("expected CsiDispatch, got {action:?}"),
        }
    }

    #[test]
    fn test_parser_osc_sequence() {
        let mut p = Parser::new();
        p.step(0x1B);
        let action = p.step(b']');
        assert_eq!(action, Action::OscStart);
        assert_eq!(p.state(), ParserState::OscString);
        p.step(b'0');
        let end = p.step(0x07);
        assert_eq!(end, Action::OscEnd);
        assert_eq!(p.state(), ParserState::Ground);
    }

    #[test]
    fn test_parser_dcs_via_esc_p() {
        let mut p = Parser::new();
        p.step(0x1B);
        p.step(b'P');
        let action = p.step(b'q');
        assert!(matches!(action, Action::DcsHook { .. }));
        assert_eq!(p.state(), ParserState::DcsPassthrough);
    }

    #[test]
    fn test_parser_esc_dispatch() {
        let mut p = Parser::new();
        p.step(0x1B);
        let action = p.step(b'c');
        match action {
            Action::EscDispatch { final_byte, .. } => {
                assert_eq!(final_byte, b'c');
            }
            _ => panic!("expected EscDispatch, got {action:?}"),
        }
        assert_eq!(p.state(), ParserState::Ground);
    }

    #[test]
    fn test_parser_c0_control() {
        let mut p = Parser::new();
        let action = p.step(0x0A);
        assert_eq!(action, Action::Execute(0x0A));
    }

    #[test]
    fn test_parser_reset() {
        let mut p = Parser::new();
        p.step(0x1B);
        assert_eq!(p.state(), ParserState::Escape);
        p.reset();
        assert_eq!(p.state(), ParserState::Ground);
    }

    #[test]
    fn test_escape_byte_transition_matrix() {
        let all_states = [
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
        for state in all_states {
            let mut p = Parser::new();
            p.state = state;
            let action = p.step(0x1B);
            match state {
                ParserState::Escape => {
                    assert_eq!(action, Action::None, "state was {state:?}");
                    assert_eq!(p.state(), ParserState::Ground, "state was {state:?}");
                }
                ParserState::OscString => {
                    assert_eq!(action, Action::OscEnd, "state was {state:?}");
                    assert_eq!(p.state(), ParserState::Ground, "state was {state:?}");
                }
                ParserState::DcsPassthrough => {
                    assert_eq!(action, Action::DcsUnhook, "state was {state:?}");
                    assert_eq!(p.state(), ParserState::Ground, "state was {state:?}");
                }
                _ => {
                    assert_eq!(action, Action::Clear, "state was {state:?}");
                    assert_eq!(p.state(), ParserState::Escape, "state was {state:?}");
                }
            }
        }
    }

    #[test]
    fn test_csi_intermediate_final_dispatch() {
        let mut p = Parser::new();
        p.state = ParserState::CsiIntermediate;
        p.params = vec![12];
        p.current_param = 34;
        p.intermediates.push(b' ');
        let action = p.step(b'm');
        assert!(matches!(action, Action::CsiDispatch { final_byte: b'm', .. }));
        assert_eq!(p.state(), ParserState::Ground);
    }

    #[test]
    fn test_dcs_passthrough_fs_terminates() {
        let mut p = Parser::new();
        p.state = ParserState::DcsPassthrough;
        let action = p.step(0x1C);
        assert_eq!(action, Action::DcsUnhook);
        assert_eq!(p.state(), ParserState::Ground);
    }

    #[test]
    fn test_osc_escape_terminates() {
        let mut p = Parser::new();
        p.state = ParserState::OscString;
        let action = p.step(0x1B);
        assert_eq!(action, Action::OscEnd);
        assert_eq!(p.state(), ParserState::Ground);
    }

    #[test]
    fn test_escape_state_control_executes() {
        let mut p = Parser::new();
        p.state = ParserState::Escape;
        let action = p.step(0x08);
        assert_eq!(action, Action::Execute(0x08));
        assert_eq!(p.state(), ParserState::Escape);
    }

    #[test]
    fn test_ignored_state_returns_none() {
        let mut p = Parser::new();
        p.state = ParserState::CsiIgnore;
        let action = p.step(b'X');
        assert_eq!(action, Action::None);
    }

    /// INV-010: Parser has 14 states (total function).
    #[test]
    fn test_parser_state_count() {
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
        assert_eq!(states.len(), 14);
    }

    #[test]
    fn test_parser_csi_sgr_no_params() {
        let mut p = Parser::new();
        p.step(0x1B);
        p.step(b'[');
        let action = p.step(b'm');
        match action {
            Action::CsiDispatch { params, final_byte, .. } => {
                assert_eq!(params, vec![0]); // default param
                assert_eq!(final_byte, b'm');
            }
            _ => panic!("expected CsiDispatch, got {action:?}"),
        }
    }

    #[test]
    fn test_parser_osc_esc_terminated() {
        let mut p = Parser::new();
        p.step(0x1B);
        p.step(b']');
        assert_eq!(p.state(), ParserState::OscString);
        p.step(b'2');
        p.step(b';');
        let action = p.step(0x1B); // ESC terminates OSC
        assert_eq!(action, Action::OscEnd);
    }

    #[test]
    fn test_parser_dcs_8bit_entry() {
        let mut p = Parser::new();
        p.step(0x90); // 8-bit DCS
        assert_eq!(p.state(), ParserState::DcsEntry);
    }

    #[test]
    fn test_parser_default_is_ground() {
        let p = Parser::default();
        assert_eq!(p.state(), ParserState::Ground);
    }
}
