//! The VT parser entry point.

use crate::action::VtAction;
use crate::state::VtState;

/// VT parser implementing the Paul Williams state machine.
///
/// Feed bytes via [`VtParser::feed`] and collect emitted [`VtAction`]s.
/// The parser is Sans-IO: it produces actions without performing
/// any terminal mutations itself.
#[derive(Debug)]
pub struct VtParser {
    state: VtState,
    params: Vec<u16>,
    intermediates: Vec<u8>,
    osc_buf: Vec<u8>,
    current_param: u16,
    has_param: bool,
}

impl VtParser {
    /// Create a new parser in the Ground state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: VtState::Ground,
            params: Vec::with_capacity(16),
            intermediates: Vec::with_capacity(4),
            osc_buf: Vec::new(),
            current_param: 0,
            has_param: false,
        }
    }

    /// Feed a byte slice and return all emitted actions.
    pub fn feed(&mut self, data: &[u8]) -> Vec<VtAction> {
        let mut actions = Vec::new();
        for &byte in data {
            self.advance(byte, &mut actions);
        }
        actions
    }

    /// Current parser state.
    #[must_use]
    pub const fn state(&self) -> VtState {
        self.state
    }

    /// Reset the parser to the Ground state.
    pub fn reset(&mut self) {
        self.state = VtState::Ground;
        self.params.clear();
        self.intermediates.clear();
        self.osc_buf.clear();
        self.current_param = 0;
        self.has_param = false;
    }

    /// Process one byte through the state machine.
    fn advance(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        // C0 control characters are handled anywhere except within strings
        if byte < 0x20 && byte != 0x1B && self.state == VtState::Ground {
            actions.push(VtAction::Execute(byte));
            return;
        }

        match self.state {
            VtState::Ground => self.ground(byte, actions),
            VtState::Escape => self.escape(byte, actions),
            VtState::EscapeIntermediate => self.escape_intermediate(byte, actions),
            VtState::CsiEntry => self.csi_entry(byte, actions),
            VtState::CsiParam => self.csi_param(byte, actions),
            VtState::CsiIntermediate => self.csi_intermediate(byte, actions),
            VtState::CsiIgnore => self.csi_ignore(byte),
            VtState::OscString => self.osc_string(byte, actions),
            VtState::DcsEntry => self.dcs_entry(byte, actions),
            VtState::DcsParam => self.dcs_param(byte, actions),
            VtState::DcsIntermediate => self.dcs_intermediate(byte, actions),
            VtState::DcsPassthrough => self.dcs_passthrough(byte, actions),
            VtState::DcsIgnore => self.dcs_ignore(byte),
            VtState::SosPmApcString => self.sos_pm_apc(byte),
        }
    }

    fn ground(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match byte {
            0x1B => {
                self.state = VtState::Escape;
                self.clear();
            }
            0x20..=0x7E => {
                if let Some(ch) = char::from_u32(u32::from(byte)) {
                    actions.push(VtAction::Print(ch));
                }
            }
            0x80..=0x8F | 0x91..=0x97 | 0x99 | 0x9A => {
                actions.push(VtAction::Execute(byte));
            }
            0x90 => {
                self.state = VtState::DcsEntry;
                self.clear();
            }
            0x9B => {
                self.state = VtState::CsiEntry;
                self.clear();
            }
            0x9C => {} // ST -- no-op in ground
            0x9D => {
                self.state = VtState::OscString;
                self.osc_buf.clear();
            }
            0x9E | 0x9F => {
                self.state = VtState::SosPmApcString;
            }
            0xA0..=0xFF => {
                // UTF-8 continuation bytes or high Latin-1 -- simplified
                if let Some(ch) = char::from_u32(u32::from(byte)) {
                    actions.push(VtAction::Print(ch));
                }
            }
            _ => {}
        }
    }

    fn escape(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match byte {
            0x20..=0x2F => {
                self.intermediates.push(byte);
                self.state = VtState::EscapeIntermediate;
            }
            b'[' => {
                self.state = VtState::CsiEntry;
                self.clear();
            }
            b']' => {
                self.state = VtState::OscString;
                self.osc_buf.clear();
            }
            b'P' => {
                self.state = VtState::DcsEntry;
                self.clear();
            }
            b'X' | b'^' | b'_' => {
                self.state = VtState::SosPmApcString;
            }
            0x30..=0x7E => {
                actions.push(VtAction::EscDispatch {
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                });
                self.state = VtState::Ground;
            }
            0x1B => {
                // ESC ESC -- stay in escape, re-clear
                self.clear();
            }
            _ => {
                self.state = VtState::Ground;
            }
        }
    }

    fn escape_intermediate(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match byte {
            0x20..=0x2F => {
                self.intermediates.push(byte);
            }
            0x30..=0x7E => {
                actions.push(VtAction::EscDispatch {
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                });
                self.state = VtState::Ground;
            }
            _ => {
                self.state = VtState::Ground;
            }
        }
    }

    fn csi_entry(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match byte {
            0x30..=0x39 | b';' => {
                self.state = VtState::CsiParam;
                self.csi_param(byte, actions);
            }
            0x3C..=0x3F => {
                // Private-mode marker (e.g., '?' in CSI ? 25 h)
                self.intermediates.push(byte);
                self.state = VtState::CsiParam;
            }
            0x20..=0x2F => {
                self.intermediates.push(byte);
                self.state = VtState::CsiIntermediate;
            }
            0x40..=0x7E => {
                self.flush_param();
                actions.push(VtAction::CsiDispatch {
                    params: self.params.clone(),
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                });
                self.state = VtState::Ground;
            }
            0x1B => {
                self.state = VtState::Escape;
                self.clear();
            }
            _ => {
                self.state = VtState::CsiIgnore;
            }
        }
    }

    fn csi_param(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match byte {
            0x30..=0x39 => {
                self.current_param = self.current_param.saturating_mul(10)
                    .saturating_add(u16::from(byte - 0x30));
                self.has_param = true;
            }
            b';' => {
                self.flush_param();
            }
            0x20..=0x2F => {
                self.flush_param();
                self.intermediates.push(byte);
                self.state = VtState::CsiIntermediate;
            }
            0x40..=0x7E => {
                self.flush_param();
                actions.push(VtAction::CsiDispatch {
                    params: self.params.clone(),
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                });
                self.state = VtState::Ground;
            }
            0x1B => {
                self.state = VtState::Escape;
                self.clear();
            }
            _ => {
                self.state = VtState::CsiIgnore;
            }
        }
    }

    fn csi_intermediate(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match byte {
            0x20..=0x2F => {
                self.intermediates.push(byte);
            }
            0x40..=0x7E => {
                actions.push(VtAction::CsiDispatch {
                    params: self.params.clone(),
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                });
                self.state = VtState::Ground;
            }
            _ => {
                self.state = VtState::CsiIgnore;
            }
        }
    }

    fn csi_ignore(&mut self, byte: u8) {
        match byte {
            0x40..=0x7E => {
                self.state = VtState::Ground;
            }
            0x1B => {
                self.state = VtState::Escape;
                self.clear();
            }
            _ => {}
        }
    }

    fn osc_string(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match byte {
            0x07 | 0x9C => {
                // BEL or ST terminates OSC
                actions.push(VtAction::OscDispatch(self.osc_buf.clone()));
                self.state = VtState::Ground;
            }
            0x1B => {
                // Could be ESC \ (ST) -- simplified: treat as terminator
                actions.push(VtAction::OscDispatch(self.osc_buf.clone()));
                self.state = VtState::Ground;
            }
            _ => {
                self.osc_buf.push(byte);
            }
        }
    }

    fn dcs_entry(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match byte {
            0x30..=0x39 | b';' => {
                self.state = VtState::DcsParam;
                self.dcs_param(byte, actions);
            }
            0x20..=0x2F => {
                self.intermediates.push(byte);
                self.state = VtState::DcsIntermediate;
            }
            0x40..=0x7E => {
                self.flush_param();
                actions.push(VtAction::DcsHook {
                    params: self.params.clone(),
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                });
                self.state = VtState::DcsPassthrough;
            }
            _ => {
                self.state = VtState::DcsIgnore;
            }
        }
    }

    fn dcs_param(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match byte {
            0x30..=0x39 => {
                self.current_param = self.current_param.saturating_mul(10)
                    .saturating_add(u16::from(byte - 0x30));
                self.has_param = true;
            }
            b';' => {
                self.flush_param();
            }
            0x20..=0x2F => {
                self.flush_param();
                self.intermediates.push(byte);
                self.state = VtState::DcsIntermediate;
            }
            0x40..=0x7E => {
                self.flush_param();
                actions.push(VtAction::DcsHook {
                    params: self.params.clone(),
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                });
                self.state = VtState::DcsPassthrough;
            }
            _ => {
                self.state = VtState::DcsIgnore;
            }
        }
    }

    fn dcs_intermediate(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match byte {
            0x20..=0x2F => {
                self.intermediates.push(byte);
            }
            0x40..=0x7E => {
                actions.push(VtAction::DcsHook {
                    params: self.params.clone(),
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                });
                self.state = VtState::DcsPassthrough;
            }
            _ => {
                self.state = VtState::DcsIgnore;
            }
        }
    }

    fn dcs_passthrough(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match byte {
            0x9C => {
                actions.push(VtAction::DcsUnhook);
                self.state = VtState::Ground;
            }
            0x1B => {
                actions.push(VtAction::DcsUnhook);
                self.state = VtState::Escape;
                self.clear();
            }
            _ => {
                actions.push(VtAction::DcsPut(byte));
            }
        }
    }

    fn dcs_ignore(&mut self, byte: u8) {
        match byte {
            0x9C => {
                self.state = VtState::Ground;
            }
            0x1B => {
                self.state = VtState::Escape;
                self.clear();
            }
            _ => {}
        }
    }

    fn sos_pm_apc(&mut self, byte: u8) {
        match byte {
            0x9C => {
                self.state = VtState::Ground;
            }
            0x1B => {
                self.state = VtState::Escape;
                self.clear();
            }
            _ => {
                // consume and discard
            }
        }
    }

    fn clear(&mut self) {
        self.params.clear();
        self.intermediates.clear();
        self.current_param = 0;
        self.has_param = false;
    }

    fn flush_param(&mut self) {
        if self.has_param {
            self.params.push(self.current_param);
        } else {
            self.params.push(0);
        }
        self.current_param = 0;
        self.has_param = false;
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
    use crate::action::VtAction;

    #[test]
    fn parse_simple_text() {
        let mut p = VtParser::new();
        let actions = p.feed(b"Hello");
        assert_eq!(actions.len(), 5);
        assert!(matches!(actions[0], VtAction::Print('H')));
        assert!(matches!(actions[4], VtAction::Print('o')));
    }

    #[test]
    fn parse_csi_sgr() {
        let mut p = VtParser::new();
        let actions = p.feed(b"\x1b[1;31m");
        let csi = actions.iter().find(|a| matches!(a, VtAction::CsiDispatch { .. }));
        assert!(csi.is_some());
        if let Some(VtAction::CsiDispatch { params, final_byte, .. }) = csi {
            assert_eq!(*final_byte, b'm');
            assert_eq!(params, &[1, 31]);
        }
    }

    #[test]
    fn parse_csi_cursor_move() {
        let mut p = VtParser::new();
        let actions = p.feed(b"\x1b[10;20H");
        let csi = actions.iter().find(|a| matches!(a, VtAction::CsiDispatch { .. }));
        assert!(csi.is_some());
        if let Some(VtAction::CsiDispatch { params, final_byte, .. }) = csi {
            assert_eq!(*final_byte, b'H');
            assert_eq!(params, &[10, 20]);
        }
    }

    #[test]
    fn parse_osc_title() {
        let mut p = VtParser::new();
        let actions = p.feed(b"\x1b]0;My Title\x07");
        let osc = actions.iter().find(|a| matches!(a, VtAction::OscDispatch(_)));
        assert!(osc.is_some());
        if let Some(VtAction::OscDispatch(data)) = osc {
            assert_eq!(std::str::from_utf8(data).unwrap_or(""), "0;My Title");
        }
    }

    #[test]
    fn parse_esc_dispatch() {
        let mut p = VtParser::new();
        let actions = p.feed(b"\x1b7"); // DECSC
        let esc = actions.iter().find(|a| matches!(a, VtAction::EscDispatch { .. }));
        assert!(esc.is_some());
    }

    #[test]
    fn parse_c0_control() {
        let mut p = VtParser::new();
        let actions = p.feed(b"\x07"); // BEL
        assert!(matches!(actions.first(), Some(VtAction::Execute(0x07))));
    }

    #[test]
    fn parse_cr_lf() {
        let mut p = VtParser::new();
        let actions = p.feed(b"\r\n");
        assert_eq!(actions.len(), 2);
        assert!(matches!(actions[0], VtAction::Execute(0x0D)));
        assert!(matches!(actions[1], VtAction::Execute(0x0A)));
    }

    #[test]
    fn parser_reset() {
        let mut p = VtParser::new();
        p.feed(b"\x1b[");
        assert_ne!(p.state(), VtState::Ground);
        p.reset();
        assert_eq!(p.state(), VtState::Ground);
    }

    #[test]
    fn parse_private_mode() {
        let mut p = VtParser::new();
        let actions = p.feed(b"\x1b[?25h");
        let csi = actions.iter().find(|a| matches!(a, VtAction::CsiDispatch { .. }));
        assert!(csi.is_some());
        if let Some(VtAction::CsiDispatch { final_byte, intermediates, .. }) = csi {
            assert_eq!(*final_byte, b'h');
            assert_eq!(intermediates, &[b'?']);
        }
    }

    #[test]
    fn parse_dcs_sequence() {
        let mut p = VtParser::new();
        let actions = p.feed(b"\x1bPq#0;2;0;0;0#1;2;100;100;0~\x1b\\");
        let hook = actions.iter().any(|a| matches!(a, VtAction::DcsHook { .. }));
        assert!(hook);
    }
}
