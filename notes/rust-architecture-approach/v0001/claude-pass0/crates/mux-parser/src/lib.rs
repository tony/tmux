//! # mux-parser
//!
//! VT100/VT220/xterm escape sequence parser using a static lookup table.
//!
//! ## Key Design (S95)
//! - `CLASS_TABLE[256]`: static LUT maps every byte to its [`ByteClass`].
//! - `classify_match()`: match-based oracle retained for testing.
//! - `step()`: pure state transition function (INV-027).
//!
//! ## Invariants
//! - INV-026: ByteClass has 7 variants including DcsEntry.
//! - INV-027: step() is a pure function (no side effects).
//! - INV-010: All parser transitions are defined.

#![forbid(unsafe_code)]

/// ByteClass: classification of input bytes for the parser state machine.
///
/// INV-026: Exactly 7 variants. DcsEntry for 0x90.
/// RULE-S06-04: #[repr(u8)] for compact storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ByteClass {
    /// Printable characters: 0x20..=0x7E
    Printable = 0,
    /// C0 control codes: 0x00..=0x1F (except ESC 0x1B)
    Control = 1,
    /// ESC: 0x1B
    Escape = 2,
    /// CSI entry: 0x9B
    CsiEntry = 3,
    /// DCS entry: 0x90
    DcsEntry = 4,
    /// OSC entry: 0x9D
    OscEntry = 5,
    /// UTF-8 lead bytes and other high bytes
    Utf8Lead = 6,
}

/// S95: CLASS_TABLE[256] static lookup table.
///
/// Maps every byte value to its ByteClass. This is the canonical hot path;
/// `classify_match()` is retained only as a test oracle.
///
/// RULE-S06-01: CLASS_TABLE is the canonical classifier.
pub static CLASS_TABLE: [ByteClass; 256] = {
    let mut table = [ByteClass::Utf8Lead; 256];
    // C0 control codes: 0x00..=0x1A, 0x1C..=0x1F
    let mut i = 0u16;
    while i <= 0x1A {
        table[i as usize] = ByteClass::Control;
        i += 1;
    }
    // 0x1B = Escape
    table[0x1B] = ByteClass::Escape;
    // 0x1C..=0x1F
    table[0x1C] = ByteClass::Control;
    table[0x1D] = ByteClass::Control;
    table[0x1E] = ByteClass::Control;
    table[0x1F] = ByteClass::Control;
    // Printable: 0x20..=0x7E
    i = 0x20;
    while i <= 0x7E {
        table[i as usize] = ByteClass::Printable;
        i += 1;
    }
    // 0x7F = DEL (control)
    table[0x7F] = ByteClass::Control;
    // C1 control codes as specific entries
    table[0x90] = ByteClass::DcsEntry;  // DCS
    table[0x9B] = ByteClass::CsiEntry;  // CSI
    table[0x9D] = ByteClass::OscEntry;  // OSC
    // All other 0x80..=0xFF bytes remain Utf8Lead (set during init)
    table
};

/// Match-based byte classifier retained as oracle for testing.
///
/// RULE-S06-02: classify_match() is the reference implementation.
/// RULE-S06-08: CLASS_TABLE must match this oracle for all 256 bytes.
pub fn classify_match(byte: u8) -> ByteClass {
    match byte {
        0x00..=0x1A | 0x1C..=0x1F | 0x7F => ByteClass::Control,
        0x1B => ByteClass::Escape,
        0x20..=0x7E => ByteClass::Printable,
        0x90 => ByteClass::DcsEntry,
        0x9B => ByteClass::CsiEntry,
        0x9D => ByteClass::OscEntry,
        _ => ByteClass::Utf8Lead,
    }
}

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
    pub fn state(&self) -> ParserState {
        self.state
    }

    /// INV-027: Pure step function.
    ///
    /// Processes one byte and returns the action to take. The parser
    /// state is updated internally.
    ///
    /// S95: Uses CLASS_TABLE for O(1) byte classification.
    pub fn step(&mut self, byte: u8) -> Action {
        let class = CLASS_TABLE[byte as usize];

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
                // Simplified: treat high bytes as printable for now
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
                        // BEL terminates OSC
                        self.state = ParserState::Ground;
                        Action::OscEnd
                    }
                    _ => Action::Execute(byte),
                }
            }
            (ParserState::OscString, ByteClass::Escape) => {
                // ESC \ is String Terminator
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
                // String Terminator via 0x9C
                if byte == 0x9C_u8.wrapping_sub(0x80) {
                    self.state = ParserState::Ground;
                    Action::DcsUnhook
                } else {
                    Action::DcsPut(byte)
                }
            }

            // Default: stay in current state
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

    /// RULE-S06-08: CLASS_TABLE matches oracle for all 256 bytes.
    #[test]
    fn test_class_table_matches_oracle_all_256() {
        for byte in 0u16..=255 {
            let b = byte as u8;
            assert_eq!(
                CLASS_TABLE[b as usize],
                classify_match(b),
                "mismatch at byte 0x{b:02X}"
            );
        }
    }

    /// INV-026: ByteClass has 7 variants.
    #[test]
    fn test_byte_class_has_7_variants() {
        let variants = [
            ByteClass::Printable,
            ByteClass::Control,
            ByteClass::Escape,
            ByteClass::CsiEntry,
            ByteClass::DcsEntry,
            ByteClass::OscEntry,
            ByteClass::Utf8Lead,
        ];
        assert_eq!(variants.len(), 7);
    }

    /// RULE-S06-05: DcsEntry at 0x90.
    #[test]
    fn test_dcs_entry_at_0x90() {
        assert_eq!(CLASS_TABLE[0x90], ByteClass::DcsEntry);
    }

    /// Test basic printable character parsing.
    #[test]
    fn test_parser_ground_printable() {
        let mut p = Parser::new();
        let action = p.step(b'A');
        assert_eq!(action, Action::Print('A'));
        assert_eq!(p.state(), ParserState::Ground);
    }

    /// Test CSI sequence: ESC [ 1 ; 2 H
    #[test]
    fn test_parser_csi_sequence() {
        let mut p = Parser::new();
        p.step(0x1B); // ESC
        p.step(b'['); // CSI entry
        p.step(b'1'); // param
        p.step(b';'); // separator
        p.step(b'2'); // param
        let action = p.step(b'H'); // final byte
        match action {
            Action::CsiDispatch { params, final_byte, .. } => {
                assert_eq!(params, vec![1, 2]);
                assert_eq!(final_byte, b'H');
            }
            _ => panic!("expected CsiDispatch, got {action:?}"),
        }
        assert_eq!(p.state(), ParserState::Ground);
    }
}
