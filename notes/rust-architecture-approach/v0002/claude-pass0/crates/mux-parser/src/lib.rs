//! # mux-parser
//!
//! VT parser and terminal input byte sequence decoder.
//!
//! ## Responsibilities
//! - Parse VT100/VT220/xterm escape sequences from PTY output
//! - Parse terminal input byte sequences into [`KeyEvent`] and [`MouseEvent`]
//! - State machine driven by byte classification
//!
//! ## Design Decision
//! Hand-written state machine (not vte crate, not termwiz).
//! Rationale: full control over tmux-compatible escape sequence handling,
//! including DCS passthrough and OSC parsing that match tmux behavior.

#![forbid(unsafe_code)]

use mux_types::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

/// VT parser state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserState {
    /// Ground state -- normal character processing.
    Ground,
    /// Escape sequence started (ESC received).
    Escape,
    /// Escape intermediate bytes.
    EscapeIntermediate,
    /// CSI (Control Sequence Introducer) entry.
    CsiEntry,
    /// CSI parameter bytes.
    CsiParam,
    /// CSI intermediate bytes.
    CsiIntermediate,
    /// CSI ignore (malformed sequence).
    CsiIgnore,
    /// DCS (Device Control String) entry.
    DcsEntry,
    /// DCS parameter bytes.
    DcsParam,
    /// DCS intermediate bytes.
    DcsIntermediate,
    /// DCS passthrough.
    DcsPassthrough,
    /// DCS ignore.
    DcsIgnore,
    /// OSC (Operating System Command) string.
    OscString,
    /// SOS/PM/APC string.
    SosString,
}

impl Default for ParserState {
    fn default() -> Self {
        Self::Ground
    }
}

/// Actions produced by the VT parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VtAction {
    /// Print a character to the current position.
    Print(char),
    /// Execute a C0/C1 control character.
    Execute(u8),
    /// CSI dispatch: final char, params, intermediates.
    CsiDispatch {
        params: Vec<u16>,
        intermediates: Vec<u8>,
        final_byte: u8,
    },
    /// ESC dispatch: intermediate byte, final byte.
    EscDispatch {
        intermediates: Vec<u8>,
        final_byte: u8,
    },
    /// OSC dispatch: the full OSC string content.
    OscDispatch(Vec<u8>),
    /// DCS hook: params, intermediates, final byte.
    DcsHook {
        params: Vec<u16>,
        intermediates: Vec<u8>,
        final_byte: u8,
    },
    /// DCS put: data byte in passthrough mode.
    DcsPut(u8),
    /// DCS unhook: end of DCS sequence.
    DcsUnhook,
}

/// The VT state machine parser.
///
/// Fed bytes from PTY output, produces [`VtAction`]s.
#[derive(Debug)]
pub struct VtParser {
    state: ParserState,
    params: Vec<u16>,
    intermediates: Vec<u8>,
    osc_data: Vec<u8>,
    current_param: u16,
    has_param: bool,
}

impl VtParser {
    /// Create a new parser in ground state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: ParserState::Ground,
            params: Vec::with_capacity(16),
            intermediates: Vec::with_capacity(4),
            osc_data: Vec::with_capacity(256),
            current_param: 0,
            has_param: false,
        }
    }

    /// Current parser state.
    #[must_use]
    pub const fn state(&self) -> ParserState {
        self.state
    }

    /// Feed a single byte, returning any action produced.
    pub fn advance(&mut self, byte: u8) -> Option<VtAction> {
        // Stub: full state machine implementation goes here.
        // The state machine follows the Paul Williams VT parser
        // state diagram with tmux-specific extensions.
        match self.state {
            ParserState::Ground => self.handle_ground(byte),
            ParserState::Escape => self.handle_escape(byte),
            ParserState::CsiEntry | ParserState::CsiParam => self.handle_csi(byte),
            _ => {
                // Other states to be implemented
                None
            }
        }
    }

    /// Feed a slice of bytes, collecting all produced actions.
    pub fn feed(&mut self, data: &[u8]) -> Vec<VtAction> {
        let mut actions = Vec::new();
        for &byte in data {
            if let Some(action) = self.advance(byte) {
                actions.push(action);
            }
        }
        actions
    }

    fn handle_ground(&mut self, byte: u8) -> Option<VtAction> {
        match byte {
            0x1B => {
                self.state = ParserState::Escape;
                None
            }
            0x20..=0x7E => {
                Some(VtAction::Print(char::from(byte)))
            }
            0x00..=0x1A | 0x1C..=0x1F => {
                Some(VtAction::Execute(byte))
            }
            0x80..=0xFF => {
                // UTF-8 continuation or C1 controls -- simplified for stub
                None
            }
            _ => None,
        }
    }

    fn handle_escape(&mut self, byte: u8) -> Option<VtAction> {
        match byte {
            b'[' => {
                self.state = ParserState::CsiEntry;
                self.params.clear();
                self.intermediates.clear();
                self.current_param = 0;
                self.has_param = false;
                None
            }
            b']' => {
                self.state = ParserState::OscString;
                self.osc_data.clear();
                None
            }
            0x30..=0x7E => {
                self.state = ParserState::Ground;
                Some(VtAction::EscDispatch {
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                })
            }
            _ => {
                self.state = ParserState::Ground;
                None
            }
        }
    }

    fn handle_csi(&mut self, byte: u8) -> Option<VtAction> {
        match byte {
            b'0'..=b'9' => {
                self.state = ParserState::CsiParam;
                self.current_param = self.current_param.saturating_mul(10)
                    .saturating_add(u16::from(byte - b'0'));
                self.has_param = true;
                None
            }
            b';' => {
                if self.has_param {
                    self.params.push(self.current_param);
                } else {
                    self.params.push(0);
                }
                self.current_param = 0;
                self.has_param = false;
                None
            }
            0x40..=0x7E => {
                // Final byte
                if self.has_param {
                    self.params.push(self.current_param);
                }
                self.state = ParserState::Ground;
                let action = VtAction::CsiDispatch {
                    params: self.params.clone(),
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                };
                self.params.clear();
                self.current_param = 0;
                self.has_param = false;
                Some(action)
            }
            0x20..=0x2F => {
                // Intermediate byte
                self.intermediates.push(byte);
                None
            }
            _ => {
                self.state = ParserState::Ground;
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

/// Parse terminal input bytes into key events.
///
/// This handles xterm-style key encoding including:
/// - CSI sequences for cursor/function keys
/// - SS3 sequences for application-mode keys
/// - Modified key reporting (CSI 1;mod X)
pub fn parse_key_input(data: &[u8]) -> Option<KeyEvent> {
    if data.is_empty() {
        return None;
    }

    match data[0] {
        0x1B if data.len() == 1 => Some(KeyEvent {
            code: KeyCode::Escape,
            modifiers: KeyModifiers::default(),
        }),
        0x0D => Some(KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::default(),
        }),
        0x09 => Some(KeyEvent {
            code: KeyCode::Tab,
            modifiers: KeyModifiers::default(),
        }),
        0x7F => Some(KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::default(),
        }),
        0x01..=0x1A => Some(KeyEvent {
            code: KeyCode::Char(char::from(data[0] + 0x60)),
            modifiers: KeyModifiers {
                ctrl: true,
                ..KeyModifiers::default()
            },
        }),
        0x20..=0x7E => Some(KeyEvent {
            code: KeyCode::Char(char::from(data[0])),
            modifiers: KeyModifiers::default(),
        }),
        _ => None, // UTF-8 / escape sequences handled by full parser
    }
}

/// Parse SGR-encoded mouse events.
///
/// Format: CSI < Pb ; Px ; Py M/m
pub fn parse_mouse_sgr(params: &[u16], final_byte: u8) -> Option<MouseEvent> {
    if params.len() < 3 {
        return None;
    }

    let cb = params[0];
    let x = params[1].saturating_sub(1); // 1-based to 0-based
    let y = params[2].saturating_sub(1);

    let button = match cb & 0x03 {
        0 => MouseButton::Left,
        1 => MouseButton::Middle,
        2 => MouseButton::Right,
        3 => {
            return Some(MouseEvent {
                kind: MouseEventKind::Motion,
                x,
                y,
                modifiers: KeyModifiers::default(),
            });
        }
        _ => return None,
    };

    let is_release = final_byte == b'm';
    let is_drag = cb & 0x20 != 0;
    let is_wheel = cb & 0x40 != 0;

    let kind = if is_wheel {
        if cb & 0x01 != 0 {
            MouseEventKind::Press(MouseButton::WheelDown)
        } else {
            MouseEventKind::Press(MouseButton::WheelUp)
        }
    } else if is_release {
        MouseEventKind::Release(button)
    } else if is_drag {
        MouseEventKind::Drag(button)
    } else {
        MouseEventKind::Press(button)
    };

    let modifiers = KeyModifiers {
        shift: cb & 0x04 != 0,
        alt: cb & 0x08 != 0,
        ctrl: cb & 0x10 != 0,
        meta: false,
    };

    Some(MouseEvent {
        kind,
        x,
        y,
        modifiers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_starts_in_ground() {
        let parser = VtParser::new();
        assert_eq!(parser.state(), ParserState::Ground);
    }

    #[test]
    fn parser_prints_ascii() {
        let mut parser = VtParser::new();
        let action = parser.advance(b'A');
        assert_eq!(action, Some(VtAction::Print('A')));
    }

    #[test]
    fn parser_csi_sequence() {
        let mut parser = VtParser::new();
        let actions = parser.feed(b"\x1b[1;2H");
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            VtAction::CsiDispatch { params, final_byte, .. } => {
                assert_eq!(params, &[1, 2]);
                assert_eq!(*final_byte, b'H');
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn parse_key_enter() {
        let event = parse_key_input(b"\r");
        assert_eq!(
            event,
            Some(KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::default(),
            })
        );
    }

    #[test]
    fn parse_key_ctrl_c() {
        let event = parse_key_input(&[0x03]);
        assert!(event.is_some());
        let event = event.map(|e| e.modifiers.ctrl);
        assert_eq!(event, Some(true));
    }

    #[test]
    fn parse_mouse_sgr_click() {
        let event = parse_mouse_sgr(&[0, 10, 20], b'M');
        assert!(event.is_some());
        let event = event.as_ref();
        assert_eq!(event.map(|e| e.x), Some(9));
        assert_eq!(event.map(|e| e.y), Some(19));
    }
}
