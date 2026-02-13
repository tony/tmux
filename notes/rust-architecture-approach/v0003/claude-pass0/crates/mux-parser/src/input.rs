//! VT parser entry point.
//!
//! The `VtParser` processes raw bytes and emits `VtAction`s. It uses the
//! state machine from `state_machine.rs` and CSI parameters from `csi.rs`.

use crate::csi::CsiParams;
use crate::state_machine::{TransitionAction, VtState};

/// Actions emitted by the VT parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VtAction {
    /// Print a character at the cursor position.
    Print(char),
    /// Execute a C0 control character (BEL, BS, CR, LF, HT, etc.).
    Execute(u8),
    /// A complete CSI sequence has been dispatched.
    CsiDispatch(CsiParams),
    /// An ESC sequence has been dispatched.
    EscDispatch {
        /// Intermediate bytes collected.
        intermediates: Vec<u8>,
        /// Final byte of the escape sequence.
        final_byte: u8,
    },
    /// OSC string complete.
    OscDispatch(Vec<u8>),
    /// DCS hook (start of passthrough).
    DcsHook(CsiParams),
    /// DCS data byte.
    DcsPut(u8),
    /// DCS unhook (end of passthrough).
    DcsUnhook,
}

/// Byte-at-a-time VT terminal parser.
///
/// Feed bytes via `advance()` and collect the resulting `VtAction`s.
#[derive(Debug)]
pub struct VtParser {
    /// Current parser state.
    state: VtState,
    /// CSI parameter accumulator.
    csi_params: CsiParams,
    /// ESC intermediate bytes.
    esc_intermediates: Vec<u8>,
    /// OSC string buffer.
    osc_buffer: Vec<u8>,
    /// UTF-8 accumulation buffer.
    utf8_buf: [u8; 4],
    /// Number of UTF-8 bytes accumulated.
    utf8_len: u8,
    /// Expected UTF-8 sequence length.
    utf8_expected: u8,
}

impl VtParser {
    /// Create a new parser in the Ground state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: VtState::Ground,
            csi_params: CsiParams::new(),
            esc_intermediates: Vec::new(),
            osc_buffer: Vec::new(),
            utf8_buf: [0; 4],
            utf8_len: 0,
            utf8_expected: 0,
        }
    }

    /// Current parser state.
    #[must_use]
    pub const fn state(&self) -> VtState {
        self.state
    }

    /// Process a single byte and return any actions produced.
    pub fn advance(&mut self, byte: u8) -> Vec<VtAction> {
        let mut actions = Vec::new();

        // Handle UTF-8 multi-byte sequences in Ground state.
        if self.state == VtState::Ground && self.utf8_expected > 0 {
            if byte & 0xC0 == 0x80 {
                self.utf8_buf[self.utf8_len as usize] = byte;
                self.utf8_len += 1;
                if self.utf8_len == self.utf8_expected {
                    let s = core::str::from_utf8(&self.utf8_buf[..self.utf8_len as usize]);
                    if let Ok(s) = s {
                        if let Some(c) = s.chars().next() {
                            actions.push(VtAction::Print(c));
                        }
                    }
                    self.utf8_len = 0;
                    self.utf8_expected = 0;
                }
                return actions;
            }
            // Invalid continuation: reset UTF-8 state and fall through.
            self.utf8_len = 0;
            self.utf8_expected = 0;
        }

        // Check for UTF-8 lead byte in Ground state.
        if self.state == VtState::Ground && byte >= 0xC0 {
            let expected = if byte < 0xE0 {
                2
            } else if byte < 0xF0 {
                3
            } else {
                4
            };
            self.utf8_buf[0] = byte;
            self.utf8_len = 1;
            self.utf8_expected = expected;
            return actions;
        }

        let (next_state, action) = self.state.transition(byte);

        match action {
            TransitionAction::Print => {
                actions.push(VtAction::Print(byte as char));
            }
            TransitionAction::Execute => {
                actions.push(VtAction::Execute(byte));
            }
            TransitionAction::Collect => {
                if self.state.is_csi() || next_state.is_csi() {
                    self.csi_params.push_collect_byte(byte);
                } else if next_state == VtState::EscapeIntermediate
                    || self.state == VtState::Escape
                {
                    self.esc_intermediates.push(byte);
                }
            }
            TransitionAction::Param => {
                self.csi_params.push_param_byte(byte);
            }
            TransitionAction::CsiDispatch => {
                self.csi_params.final_byte = byte;
                actions.push(VtAction::CsiDispatch(self.csi_params.clone()));
                self.csi_params.reset();
            }
            TransitionAction::EscDispatch => {
                actions.push(VtAction::EscDispatch {
                    intermediates: self.esc_intermediates.clone(),
                    final_byte: byte,
                });
                self.esc_intermediates.clear();
            }
            TransitionAction::OscPut => {
                self.osc_buffer.push(byte);
            }
            TransitionAction::OscEnd => {
                actions.push(VtAction::OscDispatch(self.osc_buffer.clone()));
                self.osc_buffer.clear();
            }
            TransitionAction::DcsHook => {
                self.csi_params.final_byte = byte;
                actions.push(VtAction::DcsHook(self.csi_params.clone()));
                self.csi_params.reset();
            }
            TransitionAction::DcsPut => {
                actions.push(VtAction::DcsPut(byte));
            }
            TransitionAction::DcsUnhook => {
                actions.push(VtAction::DcsUnhook);
            }
            TransitionAction::None => {}
        }

        // Reset accumulators on state transitions as needed.
        if next_state == VtState::CsiEntry && self.state != VtState::CsiEntry {
            self.csi_params.reset();
        }
        if next_state == VtState::Escape && self.state != VtState::Escape {
            self.esc_intermediates.clear();
        }

        self.state = next_state;
        actions
    }

    /// Process a slice of bytes, returning all actions.
    pub fn advance_all(&mut self, data: &[u8]) -> Vec<VtAction> {
        let mut actions = Vec::new();
        for &byte in data {
            actions.extend(self.advance(byte));
        }
        actions
    }

    /// Reset the parser to the Ground state.
    pub fn reset(&mut self) {
        self.state = VtState::Ground;
        self.csi_params.reset();
        self.esc_intermediates.clear();
        self.osc_buffer.clear();
        self.utf8_len = 0;
        self.utf8_expected = 0;
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
    fn parse_ascii_text() {
        let mut parser = VtParser::new();
        let actions = parser.advance_all(b"Hello");
        assert_eq!(actions.len(), 5);
        assert_eq!(actions[0], VtAction::Print('H'));
        assert_eq!(actions[4], VtAction::Print('o'));
    }

    #[test]
    fn parse_newline() {
        let mut parser = VtParser::new();
        let actions = parser.advance_all(b"\n");
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], VtAction::Execute(0x0A));
    }

    #[test]
    fn parse_sgr_sequence() {
        let mut parser = VtParser::new();
        // ESC [ 1 ; 3 1 m = bold + red foreground
        let actions = parser.advance_all(b"\x1b[1;31m");
        let csi_actions: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, VtAction::CsiDispatch(_)))
            .collect();
        assert_eq!(csi_actions.len(), 1);
        if let VtAction::CsiDispatch(params) = &csi_actions[0] {
            assert_eq!(params.final_byte, b'm');
            assert_eq!(params.get(0, 0), 1);
            assert_eq!(params.get(1, 0), 31);
        }
    }

    #[test]
    fn parse_cursor_position() {
        let mut parser = VtParser::new();
        // ESC [ 10 ; 20 H = CUP(10, 20)
        let actions = parser.advance_all(b"\x1b[10;20H");
        let csi_actions: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, VtAction::CsiDispatch(_)))
            .collect();
        assert_eq!(csi_actions.len(), 1);
        if let VtAction::CsiDispatch(params) = &csi_actions[0] {
            assert_eq!(params.final_byte, b'H');
            assert_eq!(params.get(0, 1), 10);
            assert_eq!(params.get(1, 1), 20);
        }
    }

    #[test]
    fn parse_private_mode_set() {
        let mut parser = VtParser::new();
        // ESC [ ? 1049 h = DECSM (alternate screen)
        let actions = parser.advance_all(b"\x1b[?1049h");
        let csi_actions: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, VtAction::CsiDispatch(_)))
            .collect();
        assert_eq!(csi_actions.len(), 1);
        if let VtAction::CsiDispatch(params) = &csi_actions[0] {
            assert!(params.is_private());
            assert_eq!(params.final_byte, b'h');
            assert_eq!(params.get(0, 0), 1049);
        }
    }

    #[test]
    fn parse_osc_title() {
        let mut parser = VtParser::new();
        // ESC ] 0 ; t i t l e BEL
        let actions = parser.advance_all(b"\x1b]0;title\x07");
        let osc_actions: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, VtAction::OscDispatch(_)))
            .collect();
        assert_eq!(osc_actions.len(), 1);
        if let VtAction::OscDispatch(data) = &osc_actions[0] {
            assert_eq!(data, b"0;title");
        }
    }

    #[test]
    fn parse_escape_dispatch() {
        let mut parser = VtParser::new();
        // ESC 7 = DECSC (save cursor)
        let actions = parser.advance_all(b"\x1b7");
        assert!(actions
            .iter()
            .any(|a| matches!(a, VtAction::EscDispatch { final_byte: b'7', .. })));
    }

    #[test]
    fn parser_returns_to_ground() {
        let mut parser = VtParser::new();
        parser.advance_all(b"\x1b[31m");
        assert_eq!(parser.state(), VtState::Ground);
    }

    #[test]
    fn reset_clears_state() {
        let mut parser = VtParser::new();
        parser.advance_all(b"\x1b[");
        assert_ne!(parser.state(), VtState::Ground);
        parser.reset();
        assert_eq!(parser.state(), VtState::Ground);
    }

    #[test]
    fn multiple_csi_sequences() {
        let mut parser = VtParser::new();
        let actions = parser.advance_all(b"\x1b[1m\x1b[31m");
        let csi_count = actions
            .iter()
            .filter(|a| matches!(a, VtAction::CsiDispatch(_)))
            .count();
        assert_eq!(csi_count, 2);
    }

    #[test]
    fn cancel_aborts_sequence() {
        let mut parser = VtParser::new();
        // Start CSI then CAN (0x18)
        let actions = parser.advance_all(b"\x1b[\x18");
        assert_eq!(parser.state(), VtState::Ground);
        // The CAN produces an Execute action
        assert!(actions.iter().any(|a| matches!(a, VtAction::Execute(0x18))));
    }
}
