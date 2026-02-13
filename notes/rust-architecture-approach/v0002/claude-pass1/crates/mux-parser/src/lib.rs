//! # mux-parser
//!
//! VT escape sequence parser based on the Paul Williams state machine.
//!
//! ## Design Decision (RULE-S33-87)
//! Own parser, not vt100 or termwiz (see architecture.md section 9).
//!
//! Reasons:
//! - Full control over DCS passthrough for tmux protocol compatibility
//! - Tight integration with GraphemeArena
//! - Zero-copy: operates on byte slices, produces VtAction enum values

#![forbid(unsafe_code)]

/// VT parser actions emitted when parsing escape sequences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VtAction {
    /// Print a character at the current cursor position.
    Print(char),
    /// Execute a C0 control character (0x00-0x1F).
    Execute(u8),
    /// CSI sequence dispatched with parameters and intermediates.
    CsiDispatch {
        params: Vec<u16>,
        intermediates: Vec<u8>,
        final_byte: u8,
    },
    /// OSC sequence (Operating System Command).
    OscDispatch {
        command: u16,
        data: String,
    },
    /// ESC sequence with intermediate bytes.
    EscDispatch {
        intermediates: Vec<u8>,
        final_byte: u8,
    },
    /// DCS hook: start of Device Control String.
    DcsHook {
        params: Vec<u16>,
        intermediates: Vec<u8>,
        final_byte: u8,
    },
    /// DCS data byte.
    DcsPut(u8),
    /// DCS unhook: end of Device Control String.
    DcsUnhook,
}

/// Parser state (Paul Williams state machine).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Ground,
    Escape,
    EscapeIntermediate,
    CsiEntry,
    CsiParam,
    CsiIntermediate,
    OscString,
    DcsEntry,
    DcsParam,
    DcsPassthrough,
}

/// A VT escape sequence parser.
///
/// Feed bytes via [`feed()`] and collect [`VtAction`] values.
/// The parser maintains state between calls (for streaming input).
#[derive(Debug)]
pub struct VtParser {
    state: State,
    params: Vec<u16>,
    current_param: u16,
    intermediates: Vec<u8>,
    osc_data: String,
}

impl VtParser {
    /// Create a new parser in the ground state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: State::Ground,
            params: Vec::new(),
            current_param: 0,
            intermediates: Vec::new(),
            osc_data: String::new(),
        }
    }

    /// Feed a slice of bytes through the parser.
    ///
    /// Returns a list of actions produced.
    pub fn feed(&mut self, data: &[u8]) -> Vec<VtAction> {
        let mut actions = Vec::new();
        for &byte in data {
            self.process_byte(byte, &mut actions);
        }
        actions
    }

    fn process_byte(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match self.state {
            State::Ground => self.ground(byte, actions),
            State::Escape => self.escape(byte, actions),
            State::EscapeIntermediate => self.escape_intermediate(byte, actions),
            State::CsiEntry => self.csi_entry(byte, actions),
            State::CsiParam => self.csi_param(byte, actions),
            State::CsiIntermediate => self.csi_intermediate(byte, actions),
            State::OscString => self.osc_string(byte, actions),
            State::DcsEntry => self.dcs_entry(byte, actions),
            State::DcsParam => self.dcs_param(byte, actions),
            State::DcsPassthrough => self.dcs_passthrough(byte, actions),
        }
    }

    fn ground(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match byte {
            0x00..=0x1F => {
                if byte == 0x1B {
                    self.state = State::Escape;
                } else {
                    actions.push(VtAction::Execute(byte));
                }
            }
            0x20..=0x7E => {
                actions.push(VtAction::Print(byte as char));
            }
            0x80..=0xFF => {
                // UTF-8 handling (simplified: treat as single byte for now)
                // Full implementation handles multi-byte UTF-8 sequences
                if byte >= 0xC0 {
                    actions.push(VtAction::Print(char::REPLACEMENT_CHARACTER));
                }
            }
            _ => {}
        }
    }

    fn escape(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match byte {
            0x5B => {
                // '[' -> CSI entry
                self.state = State::CsiEntry;
                self.params.clear();
                self.current_param = 0;
                self.intermediates.clear();
            }
            0x5D => {
                // ']' -> OSC string
                self.state = State::OscString;
                self.osc_data.clear();
            }
            0x50 => {
                // 'P' -> DCS entry
                self.state = State::DcsEntry;
                self.params.clear();
                self.current_param = 0;
                self.intermediates.clear();
            }
            0x20..=0x2F => {
                self.intermediates.push(byte);
                self.state = State::EscapeIntermediate;
            }
            0x30..=0x7E => {
                actions.push(VtAction::EscDispatch {
                    intermediates: std::mem::take(&mut self.intermediates),
                    final_byte: byte,
                });
                self.state = State::Ground;
            }
            _ => {
                self.state = State::Ground;
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
                    intermediates: std::mem::take(&mut self.intermediates),
                    final_byte: byte,
                });
                self.state = State::Ground;
            }
            _ => {
                self.state = State::Ground;
            }
        }
    }

    fn csi_entry(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match byte {
            0x30..=0x39 => {
                self.current_param = u16::from(byte - 0x30);
                self.state = State::CsiParam;
            }
            0x3B => {
                self.params.push(0);
                self.state = State::CsiParam;
            }
            0x3C..=0x3F => {
                // Private parameter prefix (<, =, >, ?)
                self.intermediates.push(byte);
                self.state = State::CsiParam;
            }
            0x40..=0x7E => {
                actions.push(VtAction::CsiDispatch {
                    params: std::mem::take(&mut self.params),
                    intermediates: std::mem::take(&mut self.intermediates),
                    final_byte: byte,
                });
                self.state = State::Ground;
            }
            _ => {
                self.state = State::Ground;
            }
        }
    }

    fn csi_param(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match byte {
            0x30..=0x39 => {
                self.current_param = self.current_param
                    .saturating_mul(10)
                    .saturating_add(u16::from(byte - 0x30));
            }
            0x3B => {
                self.params.push(self.current_param);
                self.current_param = 0;
            }
            0x20..=0x2F => {
                self.params.push(self.current_param);
                self.current_param = 0;
                self.intermediates.push(byte);
                self.state = State::CsiIntermediate;
            }
            0x40..=0x7E => {
                self.params.push(self.current_param);
                self.current_param = 0;
                actions.push(VtAction::CsiDispatch {
                    params: std::mem::take(&mut self.params),
                    intermediates: std::mem::take(&mut self.intermediates),
                    final_byte: byte,
                });
                self.state = State::Ground;
            }
            _ => {
                self.state = State::Ground;
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
                    params: std::mem::take(&mut self.params),
                    intermediates: std::mem::take(&mut self.intermediates),
                    final_byte: byte,
                });
                self.state = State::Ground;
            }
            _ => {
                self.state = State::Ground;
            }
        }
    }

    fn osc_string(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match byte {
            0x07 | 0x9C => {
                // ST (String Terminator)
                let command = self.osc_data
                    .split(';')
                    .next()
                    .and_then(|s| s.parse::<u16>().ok())
                    .unwrap_or(0);
                let data = self.osc_data
                    .split_once(';')
                    .map(|(_, d)| d.to_owned())
                    .unwrap_or_default();
                actions.push(VtAction::OscDispatch { command, data });
                self.osc_data.clear();
                self.state = State::Ground;
            }
            0x1B => {
                // ESC may be start of ESC \ (ST)
                // Simplified: treat as ST
                let command = self.osc_data
                    .split(';')
                    .next()
                    .and_then(|s| s.parse::<u16>().ok())
                    .unwrap_or(0);
                let data = self.osc_data
                    .split_once(';')
                    .map(|(_, d)| d.to_owned())
                    .unwrap_or_default();
                actions.push(VtAction::OscDispatch { command, data });
                self.osc_data.clear();
                self.state = State::Ground;
            }
            0x20..=0x7E => {
                self.osc_data.push(byte as char);
            }
            _ => {}
        }
    }

    fn dcs_entry(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match byte {
            0x30..=0x39 | 0x3B => {
                self.state = State::DcsParam;
                self.process_byte(byte, actions);
            }
            0x40..=0x7E => {
                actions.push(VtAction::DcsHook {
                    params: std::mem::take(&mut self.params),
                    intermediates: std::mem::take(&mut self.intermediates),
                    final_byte: byte,
                });
                self.state = State::DcsPassthrough;
            }
            _ => {
                self.state = State::Ground;
            }
        }
    }

    fn dcs_param(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match byte {
            0x30..=0x39 => {
                self.current_param = self.current_param
                    .saturating_mul(10)
                    .saturating_add(u16::from(byte - 0x30));
            }
            0x3B => {
                self.params.push(self.current_param);
                self.current_param = 0;
            }
            0x40..=0x7E => {
                self.params.push(self.current_param);
                self.current_param = 0;
                actions.push(VtAction::DcsHook {
                    params: std::mem::take(&mut self.params),
                    intermediates: std::mem::take(&mut self.intermediates),
                    final_byte: byte,
                });
                self.state = State::DcsPassthrough;
            }
            _ => {
                self.state = State::Ground;
            }
        }
    }

    fn dcs_passthrough(&mut self, byte: u8, actions: &mut Vec<VtAction>) {
        match byte {
            0x9C => {
                actions.push(VtAction::DcsUnhook);
                self.state = State::Ground;
            }
            0x1B => {
                // May be start of ESC \ (ST)
                actions.push(VtAction::DcsUnhook);
                self.state = State::Ground;
            }
            _ => {
                actions.push(VtAction::DcsPut(byte));
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
    fn parse_printable_ascii() {
        let mut parser = VtParser::new();
        let actions = parser.feed(b"hello");
        assert_eq!(actions.len(), 5);
        assert_eq!(actions[0], VtAction::Print('h'));
    }

    #[test]
    fn parse_newline() {
        let mut parser = VtParser::new();
        let actions = parser.feed(b"\n");
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], VtAction::Execute(0x0A));
    }

    #[test]
    fn parse_csi_cursor_up() {
        let mut parser = VtParser::new();
        let actions = parser.feed(b"\x1b[5A");
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], VtAction::CsiDispatch {
            params, final_byte: b'A', ..
        } if params == &[5]));
    }

    #[test]
    fn parse_csi_sgr() {
        let mut parser = VtParser::new();
        let actions = parser.feed(b"\x1b[1;31m");
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], VtAction::CsiDispatch {
            params, final_byte: b'm', ..
        } if params == &[1, 31]));
    }

    #[test]
    fn parse_osc_title() {
        let mut parser = VtParser::new();
        let actions = parser.feed(b"\x1b]0;My Title\x07");
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], VtAction::OscDispatch {
            command: 0, data
        } if data == "My Title"));
    }

    #[test]
    fn parse_esc_sequence() {
        let mut parser = VtParser::new();
        let actions = parser.feed(b"\x1b7"); // DECSC (save cursor)
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], VtAction::EscDispatch { final_byte: b'7', .. }));
    }

    #[test]
    fn parse_dcs_hook_unhook() {
        let mut parser = VtParser::new();
        let actions = parser.feed(b"\x1bPq\x1b\\");
        // Should produce DcsHook and DcsUnhook
        assert!(actions.iter().any(|a| matches!(a, VtAction::DcsHook { .. })));
        assert!(actions.iter().any(|a| matches!(a, VtAction::DcsUnhook)));
    }

    #[test]
    fn parse_mixed_content() {
        let mut parser = VtParser::new();
        let actions = parser.feed(b"A\x1b[1mB\x1b[0m");
        // Print A, CSI bold, Print B, CSI reset
        assert!(actions.len() >= 4);
        assert_eq!(actions[0], VtAction::Print('A'));
    }

    #[test]
    fn parser_state_persists_between_feeds() {
        let mut parser = VtParser::new();
        // Feed partial CSI
        let actions1 = parser.feed(b"\x1b[");
        assert!(actions1.is_empty());
        // Feed the rest
        let actions2 = parser.feed(b"5A");
        assert_eq!(actions2.len(), 1);
    }

    #[test]
    fn parse_private_mode_csi() {
        let mut parser = VtParser::new();
        let actions = parser.feed(b"\x1b[?25h"); // show cursor
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], VtAction::CsiDispatch {
            intermediates, final_byte: b'h', ..
        } if intermediates == &[b'?']));
    }
}
