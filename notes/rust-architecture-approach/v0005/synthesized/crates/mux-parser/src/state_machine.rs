//! VT parser state machine with 14 states.

/// Parser states matching the Paul Williams VT state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParserState {
    /// Normal text processing.
    Ground,
    /// ESC received.
    Escape,
    /// ESC + intermediate characters.
    EscapeIntermediate,
    /// CSI (ESC [) received.
    CsiEntry,
    /// CSI parameter accumulation.
    CsiParam,
    /// CSI intermediate characters.
    CsiIntermediate,
    /// Malformed CSI, consuming to final byte.
    CsiIgnore,
    /// OSC (ESC ]) string data.
    OscString,
    /// DCS (ESC P) received.
    DcsEntry,
    /// DCS parameter accumulation.
    DcsParam,
    /// DCS intermediate characters.
    DcsIntermediate,
    /// DCS data bytes.
    DcsPassthrough,
    /// Malformed DCS.
    DcsIgnore,
    /// SOS/PM/APC string (consumed and discarded).
    SosPmApcString,
}

impl Default for ParserState {
    fn default() -> Self {
        Self::Ground
    }
}

/// Action to take during a state transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionAction {
    /// No action needed.
    None,
    /// Print character to terminal.
    Print,
    /// Execute C0 control character.
    Execute,
    /// Clear CSI parameters.
    Clear,
    /// Collect intermediate character.
    Collect,
    /// Add parameter digit.
    Param,
    /// Dispatch ESC sequence.
    EscDispatch,
    /// Dispatch CSI sequence.
    CsiDispatch,
    /// Start OSC string.
    OscStart,
    /// Add byte to OSC string.
    OscPut,
    /// End OSC string.
    OscEnd,
    /// Hook DCS.
    Hook,
    /// DCS data byte.
    Put,
    /// Unhook DCS.
    Unhook,
}

/// Determine the next state and action for a byte in the current state.
pub fn transition(state: ParserState, byte: u8) -> (ParserState, TransitionAction) {
    // Anywhere transitions (these take priority)
    match byte {
        0x18 | 0x1A => return (ParserState::Ground, TransitionAction::Execute),
        0x1B => return (ParserState::Escape, TransitionAction::None),
        _ => {}
    }

    match state {
        ParserState::Ground => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                (ParserState::Ground, TransitionAction::Execute)
            }
            0x20..=0x7E => (ParserState::Ground, TransitionAction::Print),
            0x7F => (ParserState::Ground, TransitionAction::None), // DEL ignored
            0x80..=0x8F | 0x91..=0x97 | 0x99 | 0x9A => {
                (ParserState::Ground, TransitionAction::Execute)
            }
            0x90 => (ParserState::DcsEntry, TransitionAction::Clear),
            0x98 | 0x9E | 0x9F => (ParserState::SosPmApcString, TransitionAction::None),
            0x9B => (ParserState::CsiEntry, TransitionAction::Clear),
            0x9C => (ParserState::Ground, TransitionAction::None), // ST
            0x9D => (ParserState::OscString, TransitionAction::OscStart),
            // UTF-8 lead and continuation bytes
            _ => (ParserState::Ground, TransitionAction::Print),
        },

        ParserState::Escape => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                (ParserState::Escape, TransitionAction::Execute)
            }
            0x20..=0x2F => (ParserState::EscapeIntermediate, TransitionAction::Collect),
            0x30..=0x4F | 0x51..=0x57 | 0x59 | 0x5A | 0x5C | 0x60..=0x7E => {
                (ParserState::Ground, TransitionAction::EscDispatch)
            }
            0x50 => (ParserState::DcsEntry, TransitionAction::Clear),
            0x58 | 0x5E | 0x5F => (ParserState::SosPmApcString, TransitionAction::None),
            0x5B => (ParserState::CsiEntry, TransitionAction::Clear),
            0x5D => (ParserState::OscString, TransitionAction::OscStart),
            0x7F => (ParserState::Escape, TransitionAction::None), // DEL ignored
            _ => (ParserState::Ground, TransitionAction::None),
        },

        ParserState::EscapeIntermediate => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                (ParserState::EscapeIntermediate, TransitionAction::Execute)
            }
            0x20..=0x2F => (ParserState::EscapeIntermediate, TransitionAction::Collect),
            0x30..=0x7E => (ParserState::Ground, TransitionAction::EscDispatch),
            0x7F => (ParserState::EscapeIntermediate, TransitionAction::None),
            _ => (ParserState::Ground, TransitionAction::None),
        },

        ParserState::CsiEntry => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                (ParserState::CsiEntry, TransitionAction::Execute)
            }
            0x20..=0x2F => (ParserState::CsiIntermediate, TransitionAction::Collect),
            0x30..=0x39 | 0x3B => (ParserState::CsiParam, TransitionAction::Param),
            0x3C..=0x3F => (ParserState::CsiParam, TransitionAction::Collect),
            0x40..=0x7E => (ParserState::Ground, TransitionAction::CsiDispatch),
            0x7F => (ParserState::CsiEntry, TransitionAction::None),
            _ => (ParserState::Ground, TransitionAction::None),
        },

        ParserState::CsiParam => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                (ParserState::CsiParam, TransitionAction::Execute)
            }
            0x20..=0x2F => (ParserState::CsiIntermediate, TransitionAction::Collect),
            0x30..=0x39 | 0x3B => (ParserState::CsiParam, TransitionAction::Param),
            0x3C..=0x3F => (ParserState::CsiIgnore, TransitionAction::None),
            0x40..=0x7E => (ParserState::Ground, TransitionAction::CsiDispatch),
            0x7F => (ParserState::CsiParam, TransitionAction::None),
            _ => (ParserState::Ground, TransitionAction::None),
        },

        ParserState::CsiIntermediate => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                (ParserState::CsiIntermediate, TransitionAction::Execute)
            }
            0x20..=0x2F => (ParserState::CsiIntermediate, TransitionAction::Collect),
            0x30..=0x3F => (ParserState::CsiIgnore, TransitionAction::None),
            0x40..=0x7E => (ParserState::Ground, TransitionAction::CsiDispatch),
            0x7F => (ParserState::CsiIntermediate, TransitionAction::None),
            _ => (ParserState::Ground, TransitionAction::None),
        },

        ParserState::CsiIgnore => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                (ParserState::CsiIgnore, TransitionAction::Execute)
            }
            0x20..=0x3F => (ParserState::CsiIgnore, TransitionAction::None),
            0x40..=0x7E => (ParserState::Ground, TransitionAction::None),
            0x7F => (ParserState::CsiIgnore, TransitionAction::None),
            _ => (ParserState::Ground, TransitionAction::None),
        },

        ParserState::OscString => match byte {
            0x07 => (ParserState::Ground, TransitionAction::OscEnd),
            0x9C => (ParserState::Ground, TransitionAction::OscEnd),
            0x00..=0x06 | 0x08..=0x17 | 0x19 | 0x1C..=0x1F => {
                (ParserState::OscString, TransitionAction::None)
            }
            _ => (ParserState::OscString, TransitionAction::OscPut),
        },

        ParserState::DcsEntry => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                (ParserState::DcsEntry, TransitionAction::None)
            }
            0x20..=0x2F => (ParserState::DcsIntermediate, TransitionAction::Collect),
            0x30..=0x39 | 0x3B => (ParserState::DcsParam, TransitionAction::Param),
            0x3C..=0x3F => (ParserState::DcsParam, TransitionAction::Collect),
            0x40..=0x7E => (ParserState::DcsPassthrough, TransitionAction::Hook),
            0x7F => (ParserState::DcsEntry, TransitionAction::None),
            _ => (ParserState::Ground, TransitionAction::None),
        },

        ParserState::DcsParam => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                (ParserState::DcsParam, TransitionAction::None)
            }
            0x20..=0x2F => (ParserState::DcsIntermediate, TransitionAction::Collect),
            0x30..=0x39 | 0x3B => (ParserState::DcsParam, TransitionAction::Param),
            0x3C..=0x3F => (ParserState::DcsIgnore, TransitionAction::None),
            0x40..=0x7E => (ParserState::DcsPassthrough, TransitionAction::Hook),
            0x7F => (ParserState::DcsParam, TransitionAction::None),
            _ => (ParserState::Ground, TransitionAction::None),
        },

        ParserState::DcsIntermediate => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                (ParserState::DcsIntermediate, TransitionAction::None)
            }
            0x20..=0x2F => (ParserState::DcsIntermediate, TransitionAction::Collect),
            0x30..=0x3F => (ParserState::DcsIgnore, TransitionAction::None),
            0x40..=0x7E => (ParserState::DcsPassthrough, TransitionAction::Hook),
            0x7F => (ParserState::DcsIntermediate, TransitionAction::None),
            _ => (ParserState::Ground, TransitionAction::None),
        },

        ParserState::DcsPassthrough => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F | 0x20..=0x7E => {
                (ParserState::DcsPassthrough, TransitionAction::Put)
            }
            0x7F => (ParserState::DcsPassthrough, TransitionAction::None),
            0x9C => (ParserState::Ground, TransitionAction::Unhook),
            _ => (ParserState::DcsPassthrough, TransitionAction::Put),
        },

        ParserState::DcsIgnore => match byte {
            0x9C => (ParserState::Ground, TransitionAction::None),
            _ => (ParserState::DcsIgnore, TransitionAction::None),
        },

        ParserState::SosPmApcString => match byte {
            0x9C => (ParserState::Ground, TransitionAction::None),
            _ => (ParserState::SosPmApcString, TransitionAction::None),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ground_printable() {
        let (state, action) = transition(ParserState::Ground, b'A');
        assert_eq!(state, ParserState::Ground);
        assert_eq!(action, TransitionAction::Print);
    }

    #[test]
    fn ground_esc_transitions() {
        let (state, _) = transition(ParserState::Ground, 0x1B);
        assert_eq!(state, ParserState::Escape);
    }

    #[test]
    fn escape_bracket_enters_csi() {
        let (state, action) = transition(ParserState::Escape, b'[');
        assert_eq!(state, ParserState::CsiEntry);
        assert_eq!(action, TransitionAction::Clear);
    }

    #[test]
    fn csi_digit_to_param() {
        let (state, action) = transition(ParserState::CsiEntry, b'0');
        assert_eq!(state, ParserState::CsiParam);
        assert_eq!(action, TransitionAction::Param);
    }

    #[test]
    fn csi_final_dispatches() {
        let (state, action) = transition(ParserState::CsiParam, b'm');
        assert_eq!(state, ParserState::Ground);
        assert_eq!(action, TransitionAction::CsiDispatch);
    }

    #[test]
    fn osc_start_from_escape() {
        let (state, action) = transition(ParserState::Escape, b']');
        assert_eq!(state, ParserState::OscString);
        assert_eq!(action, TransitionAction::OscStart);
    }

    #[test]
    fn osc_bel_terminates() {
        let (state, action) = transition(ParserState::OscString, 0x07);
        assert_eq!(state, ParserState::Ground);
        assert_eq!(action, TransitionAction::OscEnd);
    }

    #[test]
    fn dcs_entry_from_escape() {
        let (state, action) = transition(ParserState::Escape, b'P');
        assert_eq!(state, ParserState::DcsEntry);
        assert_eq!(action, TransitionAction::Clear);
    }

    #[test]
    fn c0_execute_in_ground() {
        let (state, action) = transition(ParserState::Ground, b'\n');
        assert_eq!(state, ParserState::Ground);
        assert_eq!(action, TransitionAction::Execute);
    }

    #[test]
    fn cancel_byte_returns_to_ground() {
        let (state, action) = transition(ParserState::CsiParam, 0x18);
        assert_eq!(state, ParserState::Ground);
        assert_eq!(action, TransitionAction::Execute);
    }

    #[test]
    fn default_state_is_ground() {
        assert_eq!(ParserState::default(), ParserState::Ground);
    }

    // Comprehensive state transition tests

    #[test]
    fn ground_all_printable_ascii() {
        for b in 0x20..=0x7E {
            let (state, action) = transition(ParserState::Ground, b);
            assert_eq!(state, ParserState::Ground);
            assert_eq!(action, TransitionAction::Print, "byte 0x{b:02X}");
        }
    }

    #[test]
    fn ground_del_ignored() {
        let (state, action) = transition(ParserState::Ground, 0x7F);
        assert_eq!(state, ParserState::Ground);
        assert_eq!(action, TransitionAction::None);
    }

    #[test]
    fn ground_c0_controls_execute() {
        for b in [0x00u8, 0x01, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F] {
            let (state, action) = transition(ParserState::Ground, b);
            assert_eq!(state, ParserState::Ground, "byte 0x{b:02X}");
            assert_eq!(action, TransitionAction::Execute, "byte 0x{b:02X}");
        }
    }

    #[test]
    fn ground_8bit_csi_entry() {
        let (state, action) = transition(ParserState::Ground, 0x9B);
        assert_eq!(state, ParserState::CsiEntry);
        assert_eq!(action, TransitionAction::Clear);
    }

    #[test]
    fn ground_8bit_osc_string() {
        let (state, action) = transition(ParserState::Ground, 0x9D);
        assert_eq!(state, ParserState::OscString);
        assert_eq!(action, TransitionAction::OscStart);
    }

    #[test]
    fn ground_8bit_dcs_entry() {
        let (state, action) = transition(ParserState::Ground, 0x90);
        assert_eq!(state, ParserState::DcsEntry);
        assert_eq!(action, TransitionAction::Clear);
    }

    #[test]
    fn ground_st_ignored() {
        let (state, action) = transition(ParserState::Ground, 0x9C);
        assert_eq!(state, ParserState::Ground);
        assert_eq!(action, TransitionAction::None);
    }

    #[test]
    fn escape_esc_dispatch_letters() {
        for b in b'0'..=b'O' {
            let (state, action) = transition(ParserState::Escape, b);
            assert_eq!(state, ParserState::Ground, "byte {b}");
            assert_eq!(action, TransitionAction::EscDispatch, "byte {b}");
        }
    }

    #[test]
    fn escape_del_ignored() {
        let (state, action) = transition(ParserState::Escape, 0x7F);
        assert_eq!(state, ParserState::Escape);
        assert_eq!(action, TransitionAction::None);
    }

    #[test]
    fn escape_intermediate_collect() {
        let (state, action) = transition(ParserState::Escape, b' ');
        assert_eq!(state, ParserState::EscapeIntermediate);
        assert_eq!(action, TransitionAction::Collect);
    }

    #[test]
    fn escape_intermediate_final_dispatch() {
        let (state, action) = transition(ParserState::EscapeIntermediate, b'B');
        assert_eq!(state, ParserState::Ground);
        assert_eq!(action, TransitionAction::EscDispatch);
    }

    #[test]
    fn csi_semicolon_is_param() {
        let (state, action) = transition(ParserState::CsiEntry, b';');
        assert_eq!(state, ParserState::CsiParam);
        assert_eq!(action, TransitionAction::Param);
    }

    #[test]
    fn csi_param_all_digits() {
        for b in b'0'..=b'9' {
            let (state, action) = transition(ParserState::CsiParam, b);
            assert_eq!(state, ParserState::CsiParam, "digit {b}");
            assert_eq!(action, TransitionAction::Param, "digit {b}");
        }
    }

    #[test]
    fn csi_entry_private_marker() {
        // ? is a private marker (0x3F)
        let (state, action) = transition(ParserState::CsiEntry, b'?');
        assert_eq!(state, ParserState::CsiParam);
        assert_eq!(action, TransitionAction::Collect);
    }

    #[test]
    fn csi_param_private_marker_goes_to_ignore() {
        let (state, action) = transition(ParserState::CsiParam, b'?');
        assert_eq!(state, ParserState::CsiIgnore);
    }

    #[test]
    fn csi_intermediate_to_csi_dispatch() {
        let (state, action) = transition(ParserState::CsiIntermediate, b'p');
        assert_eq!(state, ParserState::Ground);
        assert_eq!(action, TransitionAction::CsiDispatch);
    }

    #[test]
    fn csi_ignore_consumes_until_final() {
        let (state, _) = transition(ParserState::CsiIgnore, b'3');
        assert_eq!(state, ParserState::CsiIgnore);
        let (state, _) = transition(ParserState::CsiIgnore, b'm');
        assert_eq!(state, ParserState::Ground);
    }

    #[test]
    fn osc_string_data_collected() {
        let (state, action) = transition(ParserState::OscString, b'x');
        assert_eq!(state, ParserState::OscString);
        assert_eq!(action, TransitionAction::OscPut);
    }

    #[test]
    fn osc_st_terminates() {
        let (state, action) = transition(ParserState::OscString, 0x9C);
        assert_eq!(state, ParserState::Ground);
        assert_eq!(action, TransitionAction::OscEnd);
    }

    #[test]
    fn dcs_entry_digit_to_param() {
        let (state, action) = transition(ParserState::DcsEntry, b'5');
        assert_eq!(state, ParserState::DcsParam);
        assert_eq!(action, TransitionAction::Param);
    }

    #[test]
    fn dcs_entry_final_to_passthrough() {
        let (state, action) = transition(ParserState::DcsEntry, b'q');
        assert_eq!(state, ParserState::DcsPassthrough);
        assert_eq!(action, TransitionAction::Hook);
    }

    #[test]
    fn dcs_passthrough_data_put() {
        let (state, action) = transition(ParserState::DcsPassthrough, b'A');
        assert_eq!(state, ParserState::DcsPassthrough);
        assert_eq!(action, TransitionAction::Put);
    }

    #[test]
    fn dcs_passthrough_st_unhook() {
        let (state, action) = transition(ParserState::DcsPassthrough, 0x9C);
        assert_eq!(state, ParserState::Ground);
        assert_eq!(action, TransitionAction::Unhook);
    }

    #[test]
    fn dcs_ignore_st_exits() {
        let (state, _) = transition(ParserState::DcsIgnore, b'x');
        assert_eq!(state, ParserState::DcsIgnore);
        let (state, _) = transition(ParserState::DcsIgnore, 0x9C);
        assert_eq!(state, ParserState::Ground);
    }

    #[test]
    fn sos_pm_apc_string_st_exits() {
        let (state, _) = transition(ParserState::SosPmApcString, b'x');
        assert_eq!(state, ParserState::SosPmApcString);
        let (state, _) = transition(ParserState::SosPmApcString, 0x9C);
        assert_eq!(state, ParserState::Ground);
    }

    #[test]
    fn anywhere_esc_from_any_state() {
        let states = [
            ParserState::CsiParam,
            ParserState::OscString,
            ParserState::DcsPassthrough,
            ParserState::DcsIgnore,
            ParserState::SosPmApcString,
        ];
        for s in states {
            let (next, _) = transition(s, 0x1B);
            assert_eq!(next, ParserState::Escape, "from {s:?}");
        }
    }

    #[test]
    fn anywhere_cancel_from_any_state() {
        let states = [
            ParserState::CsiParam,
            ParserState::OscString,
            ParserState::Escape,
        ];
        for s in states {
            let (next, action) = transition(s, 0x18);
            assert_eq!(next, ParserState::Ground, "from {s:?}");
            assert_eq!(action, TransitionAction::Execute, "from {s:?}");
        }
    }

    #[test]
    fn utf8_lead_bytes_print_in_ground() {
        // 2-byte UTF-8 lead (0xC0..=0xDF)
        let (state, action) = transition(ParserState::Ground, 0xC3);
        assert_eq!(state, ParserState::Ground);
        assert_eq!(action, TransitionAction::Print);
    }

    #[test]
    fn csi_sgr_sequence_full() {
        // Simulate ESC [ 1 ; 3 1 m (bold + red fg)
        let (s1, _) = transition(ParserState::Ground, 0x1B); // ESC
        assert_eq!(s1, ParserState::Escape);
        let (s2, _) = transition(s1, b'['); // CSI
        assert_eq!(s2, ParserState::CsiEntry);
        let (s3, _) = transition(s2, b'1'); // param digit
        assert_eq!(s3, ParserState::CsiParam);
        let (s4, _) = transition(s3, b';'); // separator
        assert_eq!(s4, ParserState::CsiParam);
        let (s5, _) = transition(s4, b'3');
        assert_eq!(s5, ParserState::CsiParam);
        let (s6, _) = transition(s5, b'1');
        assert_eq!(s6, ParserState::CsiParam);
        let (s7, action) = transition(s6, b'm'); // final
        assert_eq!(s7, ParserState::Ground);
        assert_eq!(action, TransitionAction::CsiDispatch);
    }

    #[test]
    fn csi_entry_intermediate_to_intermediate() {
        let (state, action) = transition(ParserState::CsiEntry, b' ');
        assert_eq!(state, ParserState::CsiIntermediate);
        assert_eq!(action, TransitionAction::Collect);
    }

    #[test]
    fn csi_intermediate_param_to_ignore() {
        let (state, _) = transition(ParserState::CsiIntermediate, b'3');
        assert_eq!(state, ParserState::CsiIgnore);
    }
}
