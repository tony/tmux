//! VtParser entry point -- feeds bytes, outputs VtActions.

use crate::state_machine::VtState;
use crate::csi::CsiParams;

/// Actions produced by the VT parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VtAction {
    /// Print a character.
    Print(char),
    /// Execute a C0 control code (0x00-0x1F).
    Execute(u8),
    /// CSI dispatch (final byte + params).
    CsiDispatch {
        /// Final byte of the CSI sequence.
        final_byte: u8,
        /// Parsed parameters.
        params: CsiParams,
    },
    /// ESC dispatch (final byte).
    EscDispatch(u8),
    /// OSC string completed.
    OscEnd(Vec<u8>),
    /// DCS passthrough data (graphics, etc).
    DcsData(Vec<u8>),
}

/// VT terminal parser.
#[derive(Debug)]
pub struct VtParser {
    state: VtState,
    /// Accumulated parameter bytes for current sequence.
    param_buf: Vec<u8>,
    /// Accumulated string data for OSC/DCS.
    string_buf: Vec<u8>,
}

impl VtParser {
    /// Create a new parser in the ground state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: VtState::Ground,
            param_buf: Vec::with_capacity(64),
            string_buf: Vec::with_capacity(256),
        }
    }

    /// Current parser state.
    #[must_use]
    pub const fn state(&self) -> VtState {
        self.state
    }

    /// Reset parser to ground state.
    pub fn reset(&mut self) {
        self.state = VtState::Ground;
        self.param_buf.clear();
        self.string_buf.clear();
    }

    /// Advance the parser by a single byte, returning any produced action.
    pub fn advance(&mut self, byte: u8) -> Option<VtAction> {
        match self.state {
            VtState::Ground => self.ground(byte),
            VtState::Escape => self.escape(byte),
            VtState::EscapeIntermediate => self.escape_intermediate(byte),
            VtState::CsiEntry => self.csi_entry(byte),
            VtState::CsiParam => self.csi_param(byte),
            VtState::CsiIntermediate | VtState::CsiIgnore => self.csi_collect(byte),
            VtState::OscString => self.osc_string(byte),
            VtState::DcsPassthrough => self.dcs_passthrough(byte),
            _ => {
                // For DCS entry/param/intermediate and SOS/PM/APC:
                // simplified -- consume until ST or ESC \\.
                self.consume_string(byte)
            }
        }
    }

    /// Process all bytes, collecting actions.
    pub fn advance_all(&mut self, data: &[u8]) -> Vec<VtAction> {
        let mut actions = Vec::new();
        for &byte in data {
            if let Some(action) = self.advance(byte) {
                actions.push(action);
            }
        }
        actions
    }

    fn ground(&mut self, byte: u8) -> Option<VtAction> {
        match byte {
            0x1B => {
                self.state = VtState::Escape;
                None
            }
            0x00..=0x1F => Some(VtAction::Execute(byte)),
            0x20..=0x7E => Some(VtAction::Print(byte as char)),
            0x80..=0xFF => {
                // UTF-8 high bytes -- simplified: treat as printable
                // A real implementation would decode multi-byte sequences.
                Some(VtAction::Print(char::REPLACEMENT_CHARACTER))
            }
            _ => None,
        }
    }

    fn escape(&mut self, byte: u8) -> Option<VtAction> {
        match byte {
            b'[' => {
                self.state = VtState::CsiEntry;
                self.param_buf.clear();
                None
            }
            b']' => {
                self.state = VtState::OscString;
                self.string_buf.clear();
                None
            }
            b'P' => {
                self.state = VtState::DcsPassthrough;
                self.string_buf.clear();
                None
            }
            0x20..=0x2F => {
                self.state = VtState::EscapeIntermediate;
                None
            }
            0x30..=0x7E => {
                self.state = VtState::Ground;
                Some(VtAction::EscDispatch(byte))
            }
            _ => {
                self.state = VtState::Ground;
                None
            }
        }
    }

    fn escape_intermediate(&mut self, byte: u8) -> Option<VtAction> {
        match byte {
            0x30..=0x7E => {
                self.state = VtState::Ground;
                Some(VtAction::EscDispatch(byte))
            }
            _ => None,
        }
    }

    fn csi_entry(&mut self, byte: u8) -> Option<VtAction> {
        match byte {
            b'0'..=b'9' | b';' => {
                self.state = VtState::CsiParam;
                self.param_buf.push(byte);
                None
            }
            0x40..=0x7E => {
                self.state = VtState::Ground;
                let params = CsiParams::parse(&self.param_buf);
                self.param_buf.clear();
                Some(VtAction::CsiDispatch {
                    final_byte: byte,
                    params,
                })
            }
            _ => {
                self.state = VtState::CsiIgnore;
                None
            }
        }
    }

    fn csi_param(&mut self, byte: u8) -> Option<VtAction> {
        match byte {
            b'0'..=b'9' | b';' => {
                self.param_buf.push(byte);
                None
            }
            0x20..=0x2F => {
                self.state = VtState::CsiIntermediate;
                self.param_buf.push(byte);
                None
            }
            0x40..=0x7E => {
                self.state = VtState::Ground;
                let params = CsiParams::parse(&self.param_buf);
                self.param_buf.clear();
                Some(VtAction::CsiDispatch {
                    final_byte: byte,
                    params,
                })
            }
            _ => {
                self.state = VtState::CsiIgnore;
                None
            }
        }
    }

    fn csi_collect(&mut self, byte: u8) -> Option<VtAction> {
        match byte {
            0x40..=0x7E => {
                self.state = VtState::Ground;
                let params = CsiParams::parse(&self.param_buf);
                self.param_buf.clear();
                Some(VtAction::CsiDispatch {
                    final_byte: byte,
                    params,
                })
            }
            _ => None,
        }
    }

    fn osc_string(&mut self, byte: u8) -> Option<VtAction> {
        match byte {
            0x07 | 0x9C => {
                // BEL or ST terminates OSC
                self.state = VtState::Ground;
                let data = std::mem::take(&mut self.string_buf);
                Some(VtAction::OscEnd(data))
            }
            0x1B => {
                // ESC might be start of ST (ESC \\)
                // Simplified: treat as end
                self.state = VtState::Ground;
                let data = std::mem::take(&mut self.string_buf);
                Some(VtAction::OscEnd(data))
            }
            _ => {
                self.string_buf.push(byte);
                None
            }
        }
    }

    fn dcs_passthrough(&mut self, byte: u8) -> Option<VtAction> {
        match byte {
            0x9C => {
                self.state = VtState::Ground;
                let data = std::mem::take(&mut self.string_buf);
                Some(VtAction::DcsData(data))
            }
            0x1B => {
                self.state = VtState::Ground;
                let data = std::mem::take(&mut self.string_buf);
                Some(VtAction::DcsData(data))
            }
            _ => {
                self.string_buf.push(byte);
                None
            }
        }
    }

    fn consume_string(&mut self, byte: u8) -> Option<VtAction> {
        match byte {
            0x9C | 0x1B | 0x07 => {
                self.state = VtState::Ground;
                self.string_buf.clear();
                None
            }
            _ => {
                self.string_buf.push(byte);
                None
            }
        }
    }
}

impl Default for VtParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plain_text() {
        let mut p = VtParser::new();
        let actions = p.advance_all(b"ABC");
        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0], VtAction::Print('A'));
        assert_eq!(actions[1], VtAction::Print('B'));
        assert_eq!(actions[2], VtAction::Print('C'));
    }

    #[test]
    fn parse_newline() {
        let mut p = VtParser::new();
        let actions = p.advance_all(b"\n");
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], VtAction::Execute(0x0A));
    }

    #[test]
    fn parse_csi_cursor_up() {
        let mut p = VtParser::new();
        let actions = p.advance_all(b"\x1B[5A");
        assert_eq!(actions.len(), 1);
        if let VtAction::CsiDispatch { final_byte, params } = &actions[0] {
            assert_eq!(*final_byte, b'A');
            assert_eq!(params.get(0, 1), 5);
        } else {
            // Use assertion on the variant
            assert!(false, "expected CsiDispatch");
        }
    }

    #[test]
    fn parse_csi_sgr() {
        let mut p = VtParser::new();
        let actions = p.advance_all(b"\x1B[1;31m");
        assert_eq!(actions.len(), 1);
        if let VtAction::CsiDispatch { final_byte, params } = &actions[0] {
            assert_eq!(*final_byte, b'm');
            assert_eq!(params.get(0, 0), 1);
            assert_eq!(params.get(1, 0), 31);
        } else {
            assert!(false, "expected CsiDispatch");
        }
    }

    #[test]
    fn parse_mixed_text_and_control() {
        let mut p = VtParser::new();
        let actions = p.advance_all(b"A\x1B[1mB");
        // Print(A), CsiDispatch(m), Print(B)
        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0], VtAction::Print('A'));
        assert_eq!(actions[2], VtAction::Print('B'));
    }

    #[test]
    fn parse_escape_dispatch() {
        let mut p = VtParser::new();
        let actions = p.advance_all(b"\x1Bc"); // RIS (reset)
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], VtAction::EscDispatch(b'c'));
    }

    #[test]
    fn parse_osc_string() {
        let mut p = VtParser::new();
        let actions = p.advance_all(b"\x1B]0;title\x07");
        assert_eq!(actions.len(), 1);
        if let VtAction::OscEnd(data) = &actions[0] {
            assert_eq!(data, b"0;title");
        } else {
            assert!(false, "expected OscEnd");
        }
    }

    #[test]
    fn parser_reset() {
        let mut p = VtParser::new();
        p.advance_all(b"\x1B[");
        assert!(p.state().is_csi());
        p.reset();
        assert!(p.state().is_ground());
    }

    #[test]
    fn parse_cr() {
        let mut p = VtParser::new();
        let actions = p.advance_all(b"\r");
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], VtAction::Execute(0x0D));
    }

    #[test]
    fn empty_csi() {
        let mut p = VtParser::new();
        let actions = p.advance_all(b"\x1B[m");
        assert_eq!(actions.len(), 1);
        if let VtAction::CsiDispatch { final_byte, params } = &actions[0] {
            assert_eq!(*final_byte, b'm');
            assert_eq!(params.get(0, 0), 0);
        } else {
            assert!(false, "expected CsiDispatch");
        }
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Parser fuzz: arbitrary byte sequences must never panic.
        #[test]
        fn parser_never_panics(data in proptest::collection::vec(any::<u8>(), 0..1024)) {
            let mut p = VtParser::new();
            let _actions = p.advance_all(&data);
            // If we get here, no panic occurred
        }
    }
}
