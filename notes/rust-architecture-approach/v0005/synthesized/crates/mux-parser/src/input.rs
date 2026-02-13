//! VT parser entry point with UTF-8 handling.
//!
//! The `VtParser` processes byte streams and emits `VtAction` events.
//! It handles UTF-8 multi-byte sequences in the Ground state.

use crate::csi::CsiParams;
use crate::state_machine::{transition, ParserState, TransitionAction};

/// Actions emitted by the VT parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VtAction {
    /// Print a character at the cursor position.
    Print(char),
    /// Execute a C0 control character (BEL, BS, CR, LF, etc.).
    Execute(u8),
    /// CSI sequence complete.
    CsiDispatch(CsiParams),
    /// ESC sequence complete (intermediate + final byte).
    EscDispatch {
        intermediates: Vec<u8>,
        final_byte: u8,
    },
    /// OSC string complete.
    OscDispatch(Vec<u8>),
    /// DCS sequence started.
    DcsHook(CsiParams),
    /// DCS data byte.
    DcsPut(u8),
    /// DCS sequence complete.
    DcsUnhook,
}

/// VT parser state.
#[derive(Debug, Clone)]
pub struct VtParser {
    state: ParserState,
    csi_params: CsiParams,
    osc_data: Vec<u8>,
    intermediates: Vec<u8>,
    // UTF-8 state
    utf8_buf: [u8; 4],
    utf8_len: usize,
    utf8_expected: usize,
}

impl Default for VtParser {
    fn default() -> Self {
        Self::new()
    }
}

impl VtParser {
    /// Create a new parser in the Ground state.
    pub fn new() -> Self {
        Self {
            state: ParserState::Ground,
            csi_params: CsiParams::new(),
            osc_data: Vec::new(),
            intermediates: Vec::new(),
            utf8_buf: [0; 4],
            utf8_len: 0,
            utf8_expected: 0,
        }
    }

    /// Current parser state.
    pub fn state(&self) -> ParserState {
        self.state
    }

    /// Process a single byte and return any resulting actions.
    pub fn process_byte(&mut self, byte: u8) -> Vec<VtAction> {
        let mut actions = Vec::new();

        // Handle UTF-8 continuation in Ground state
        if self.utf8_expected > 0 {
            if byte & 0xC0 == 0x80 {
                // Valid continuation byte
                self.utf8_buf[self.utf8_len] = byte;
                self.utf8_len += 1;
                if self.utf8_len == self.utf8_expected {
                    // Sequence complete
                    if let Ok(s) = std::str::from_utf8(&self.utf8_buf[..self.utf8_len]) {
                        if let Some(c) = s.chars().next() {
                            actions.push(VtAction::Print(c));
                        }
                    }
                    self.utf8_expected = 0;
                    self.utf8_len = 0;
                }
                return actions;
            }
            // Invalid continuation: reset and reprocess
            self.utf8_expected = 0;
            self.utf8_len = 0;
        }

        let (next_state, action) = transition(self.state, byte);

        // Handle state entry/exit actions
        match action {
            TransitionAction::Print => {
                if self.state == ParserState::Ground && byte >= 0xC0 {
                    // UTF-8 lead byte
                    let expected = match byte {
                        0xC0..=0xDF => 2,
                        0xE0..=0xEF => 3,
                        0xF0..=0xF7 => 4,
                        _ => 1,
                    };
                    if expected > 1 {
                        self.utf8_buf[0] = byte;
                        self.utf8_len = 1;
                        self.utf8_expected = expected;
                        self.state = next_state;
                        return actions;
                    }
                }
                actions.push(VtAction::Print(byte as char));
            }
            TransitionAction::Execute => {
                actions.push(VtAction::Execute(byte));
            }
            TransitionAction::Clear => {
                self.csi_params.clear();
                self.intermediates.clear();
            }
            TransitionAction::Collect => {
                self.intermediates.push(byte);
                // Also check for private marker
                if matches!(byte, b'?' | b'>' | b'<' | b'=')
                    && self.csi_params.private_marker.is_none()
                {
                    self.csi_params.private_marker = Some(byte);
                }
            }
            TransitionAction::Param => {
                self.csi_params.process_param_byte(byte);
            }
            TransitionAction::CsiDispatch => {
                self.csi_params.final_byte = byte;
                self.csi_params.intermediates = self.intermediates.clone();
                actions.push(VtAction::CsiDispatch(self.csi_params.clone()));
            }
            TransitionAction::EscDispatch => {
                actions.push(VtAction::EscDispatch {
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                });
            }
            TransitionAction::OscStart => {
                self.osc_data.clear();
            }
            TransitionAction::OscPut => {
                self.osc_data.push(byte);
            }
            TransitionAction::OscEnd => {
                actions.push(VtAction::OscDispatch(self.osc_data.clone()));
                self.osc_data.clear();
            }
            TransitionAction::Hook => {
                self.csi_params.final_byte = byte;
                actions.push(VtAction::DcsHook(self.csi_params.clone()));
            }
            TransitionAction::Put => {
                actions.push(VtAction::DcsPut(byte));
            }
            TransitionAction::Unhook => {
                actions.push(VtAction::DcsUnhook);
            }
            TransitionAction::None => {}
        }

        self.state = next_state;
        actions
    }

    /// Process a slice of bytes, collecting all actions.
    pub fn process_bytes(&mut self, data: &[u8]) -> Vec<VtAction> {
        let mut all_actions = Vec::new();
        for &byte in data {
            all_actions.extend(self.process_byte(byte));
        }
        all_actions
    }

    /// Reset the parser to initial state.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ascii_text() {
        let mut p = VtParser::new();
        let actions = p.process_bytes(b"AB");
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0], VtAction::Print('A'));
        assert_eq!(actions[1], VtAction::Print('B'));
    }

    #[test]
    fn parse_newline() {
        let mut p = VtParser::new();
        let actions = p.process_bytes(b"\n");
        assert_eq!(actions, vec![VtAction::Execute(b'\n')]);
    }

    #[test]
    fn parse_cr_lf() {
        let mut p = VtParser::new();
        let actions = p.process_bytes(b"\r\n");
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0], VtAction::Execute(b'\r'));
        assert_eq!(actions[1], VtAction::Execute(b'\n'));
    }

    #[test]
    fn parse_csi_sgr() {
        let mut p = VtParser::new();
        let actions = p.process_bytes(b"\x1b[1;31m");
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            VtAction::CsiDispatch(params) => {
                assert_eq!(params.get(0, 0), 1);
                assert_eq!(params.get(1, 0), 31);
                assert_eq!(params.final_byte, b'm');
            }
            _ => std::process::abort(),
        }
    }

    #[test]
    fn parse_csi_cursor_up() {
        let mut p = VtParser::new();
        let actions = p.process_bytes(b"\x1b[5A");
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            VtAction::CsiDispatch(params) => {
                assert_eq!(params.get(0, 1), 5);
                assert_eq!(params.final_byte, b'A');
            }
            _ => std::process::abort(),
        }
    }

    #[test]
    fn parse_csi_private_mode() {
        let mut p = VtParser::new();
        let actions = p.process_bytes(b"\x1b[?25h");
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            VtAction::CsiDispatch(params) => {
                assert!(params.is_private());
                assert_eq!(params.get(0, 0), 25);
                assert_eq!(params.final_byte, b'h');
            }
            _ => std::process::abort(),
        }
    }

    #[test]
    fn parse_osc() {
        let mut p = VtParser::new();
        let actions = p.process_bytes(b"\x1b]0;title\x07");
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            VtAction::OscDispatch(data) => {
                assert_eq!(data, b"0;title");
            }
            _ => std::process::abort(),
        }
    }

    #[test]
    fn parse_esc_dispatch() {
        let mut p = VtParser::new();
        let actions = p.process_bytes(b"\x1b7"); // Save cursor
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            VtAction::EscDispatch { final_byte, .. } => {
                assert_eq!(*final_byte, b'7');
            }
            _ => std::process::abort(),
        }
    }

    #[test]
    fn parse_bell() {
        let mut p = VtParser::new();
        let actions = p.process_bytes(b"\x07");
        assert_eq!(actions, vec![VtAction::Execute(0x07)]);
    }

    #[test]
    fn parse_backspace() {
        let mut p = VtParser::new();
        let actions = p.process_bytes(b"\x08");
        assert_eq!(actions, vec![VtAction::Execute(0x08)]);
    }

    #[test]
    fn csi_default_param() {
        let mut p = VtParser::new();
        let actions = p.process_bytes(b"\x1b[A"); // No param
        match &actions[0] {
            VtAction::CsiDispatch(params) => {
                assert_eq!(params.get(0, 1), 1); // Default to 1
            }
            _ => std::process::abort(),
        }
    }

    #[test]
    fn parser_reset() {
        let mut p = VtParser::new();
        p.process_bytes(b"\x1b[");
        assert_ne!(p.state(), ParserState::Ground);
        p.reset();
        assert_eq!(p.state(), ParserState::Ground);
    }

    #[test]
    fn tab_character() {
        let mut p = VtParser::new();
        let actions = p.process_bytes(b"\t");
        assert_eq!(actions, vec![VtAction::Execute(b'\t')]);
    }

    #[test]
    fn cancel_interrupts_csi() {
        let mut p = VtParser::new();
        let actions = p.process_bytes(b"\x1b[1\x18");
        // CAN (0x18) should cancel CSI and execute
        let has_execute = actions.iter().any(|a| matches!(a, VtAction::Execute(0x18)));
        assert!(has_execute);
    }

    #[test]
    fn dcs_passthrough() {
        let mut p = VtParser::new();
        let actions = p.process_bytes(b"\x1bPq\x9c");
        let has_hook = actions.iter().any(|a| matches!(a, VtAction::DcsHook(_)));
        let has_unhook = actions.iter().any(|a| matches!(a, VtAction::DcsUnhook));
        assert!(has_hook);
        assert!(has_unhook);
    }

    #[test]
    fn multiple_csi_sequences() {
        let mut p = VtParser::new();
        let actions = p.process_bytes(b"\x1b[1m\x1b[31m");
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn del_ignored_in_ground() {
        let mut p = VtParser::new();
        let actions = p.process_bytes(b"\x7f");
        assert!(actions.is_empty());
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            // INV: parser never panics on arbitrary input
            #[test]
            fn fuzz_safety(data in proptest::collection::vec(0u8..=255, 0..200)) {
                let mut p = VtParser::new();
                let _ = p.process_bytes(&data);
                // If we get here, no panic occurred
            }

            #[test]
            fn csi_roundtrip(param1 in 0u16..100, param2 in 0u16..100) {
                let mut p = VtParser::new();
                let seq = format!("\x1b[{param1};{param2}m");
                let actions = p.process_bytes(seq.as_bytes());
                prop_assert_eq!(actions.len(), 1);
                if let VtAction::CsiDispatch(params) = &actions[0] {
                    if param1 > 0 {
                        prop_assert_eq!(params.get(0, 0), param1);
                    }
                    if param2 > 0 {
                        prop_assert_eq!(params.get(1, 0), param2);
                    }
                }
            }

            #[test]
            fn state_always_valid(data in proptest::collection::vec(0u8..=255, 1..50)) {
                let mut p = VtParser::new();
                for &b in &data {
                    let _ = p.process_byte(b);
                    // State should always be a valid enum variant
                    let _ = format!("{:?}", p.state());
                }
            }
        }
    }
}
