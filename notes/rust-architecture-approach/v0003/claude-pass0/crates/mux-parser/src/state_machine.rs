//! VT parser state machine.
//!
//! Implements the 14-state Paul Williams state machine for VT parsing.
//! This is a byte-at-a-time state machine that produces actions.
//! Reference: <https://vt100.net/emu/dec_ansi_parser>

/// The 14 states of the VT parser state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VtState {
    /// Normal text processing.
    Ground,
    /// ESC received.
    Escape,
    /// ESC + intermediate character(s).
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
    /// DCS data pass-through.
    DcsPassthrough,
    /// Malformed DCS.
    DcsIgnore,
    /// SOS/PM/APC string (consumed and discarded).
    SosPmApcString,
}

impl VtState {
    /// Whether this state processes printable characters.
    #[must_use]
    pub const fn is_ground(&self) -> bool {
        matches!(self, Self::Ground)
    }

    /// Whether this state is within a CSI sequence.
    #[must_use]
    pub const fn is_csi(&self) -> bool {
        matches!(
            self,
            Self::CsiEntry | Self::CsiParam | Self::CsiIntermediate | Self::CsiIgnore
        )
    }

    /// Whether this state is within a DCS sequence.
    #[must_use]
    pub const fn is_dcs(&self) -> bool {
        matches!(
            self,
            Self::DcsEntry | Self::DcsParam | Self::DcsIntermediate | Self::DcsPassthrough | Self::DcsIgnore
        )
    }

    /// Whether this state is within an OSC sequence.
    #[must_use]
    pub const fn is_osc(&self) -> bool {
        matches!(self, Self::OscString)
    }

    /// Determine the next state given a byte in the current state.
    /// Returns (next_state, action_kind).
    #[must_use]
    pub fn transition(self, byte: u8) -> (Self, TransitionAction) {
        // Anywhere transitions (C0 control characters that cause immediate state changes)
        match byte {
            0x1B => return (Self::Escape, TransitionAction::None),
            0x18 | 0x1A => return (Self::Ground, TransitionAction::Execute),
            _ => {}
        }

        match self {
            Self::Ground => match byte {
                0x00..=0x17 | 0x19 | 0x1C..=0x1F => (Self::Ground, TransitionAction::Execute),
                0x20..=0x7E => (Self::Ground, TransitionAction::Print),
                0x80..=0xFF => (Self::Ground, TransitionAction::Print), // UTF-8 handled upstream
                _ => (Self::Ground, TransitionAction::None),
            },
            Self::Escape => match byte {
                0x20..=0x2F => (Self::EscapeIntermediate, TransitionAction::Collect),
                0x30..=0x4F | 0x51..=0x57 | 0x59 | 0x5A | 0x5C | 0x60..=0x7E => {
                    (Self::Ground, TransitionAction::EscDispatch)
                }
                0x5B => (Self::CsiEntry, TransitionAction::None),
                0x5D => (Self::OscString, TransitionAction::None),
                0x50 => (Self::DcsEntry, TransitionAction::None),
                0x58 | 0x5E | 0x5F => (Self::SosPmApcString, TransitionAction::None),
                _ => (Self::Escape, TransitionAction::None),
            },
            Self::EscapeIntermediate => match byte {
                0x20..=0x2F => (Self::EscapeIntermediate, TransitionAction::Collect),
                0x30..=0x7E => (Self::Ground, TransitionAction::EscDispatch),
                _ => (Self::EscapeIntermediate, TransitionAction::None),
            },
            Self::CsiEntry => match byte {
                0x30..=0x39 | 0x3B => (Self::CsiParam, TransitionAction::Param),
                0x3C..=0x3F => (Self::CsiParam, TransitionAction::Collect),
                0x20..=0x2F => (Self::CsiIntermediate, TransitionAction::Collect),
                0x40..=0x7E => (Self::Ground, TransitionAction::CsiDispatch),
                _ => (Self::CsiEntry, TransitionAction::None),
            },
            Self::CsiParam => match byte {
                0x30..=0x39 | 0x3B => (Self::CsiParam, TransitionAction::Param),
                0x3C..=0x3F => (Self::CsiIgnore, TransitionAction::None),
                0x20..=0x2F => (Self::CsiIntermediate, TransitionAction::Collect),
                0x40..=0x7E => (Self::Ground, TransitionAction::CsiDispatch),
                _ => (Self::CsiParam, TransitionAction::None),
            },
            Self::CsiIntermediate => match byte {
                0x20..=0x2F => (Self::CsiIntermediate, TransitionAction::Collect),
                0x40..=0x7E => (Self::Ground, TransitionAction::CsiDispatch),
                0x30..=0x3F => (Self::CsiIgnore, TransitionAction::None),
                _ => (Self::CsiIntermediate, TransitionAction::None),
            },
            Self::CsiIgnore => match byte {
                0x40..=0x7E => (Self::Ground, TransitionAction::None),
                _ => (Self::CsiIgnore, TransitionAction::None),
            },
            Self::OscString => match byte {
                0x07 => (Self::Ground, TransitionAction::OscEnd),
                0x9C => (Self::Ground, TransitionAction::OscEnd),
                _ => (Self::OscString, TransitionAction::OscPut),
            },
            Self::DcsEntry => match byte {
                0x30..=0x39 | 0x3B => (Self::DcsParam, TransitionAction::Param),
                0x3C..=0x3F => (Self::DcsParam, TransitionAction::Collect),
                0x20..=0x2F => (Self::DcsIntermediate, TransitionAction::Collect),
                0x40..=0x7E => (Self::DcsPassthrough, TransitionAction::DcsHook),
                _ => (Self::DcsEntry, TransitionAction::None),
            },
            Self::DcsParam => match byte {
                0x30..=0x39 | 0x3B => (Self::DcsParam, TransitionAction::Param),
                0x20..=0x2F => (Self::DcsIntermediate, TransitionAction::Collect),
                0x40..=0x7E => (Self::DcsPassthrough, TransitionAction::DcsHook),
                0x3C..=0x3F => (Self::DcsIgnore, TransitionAction::None),
                _ => (Self::DcsParam, TransitionAction::None),
            },
            Self::DcsIntermediate => match byte {
                0x20..=0x2F => (Self::DcsIntermediate, TransitionAction::Collect),
                0x40..=0x7E => (Self::DcsPassthrough, TransitionAction::DcsHook),
                0x30..=0x3F => (Self::DcsIgnore, TransitionAction::None),
                _ => (Self::DcsIntermediate, TransitionAction::None),
            },
            Self::DcsPassthrough => match byte {
                0x9C => (Self::Ground, TransitionAction::DcsUnhook),
                _ => (Self::DcsPassthrough, TransitionAction::DcsPut),
            },
            Self::DcsIgnore => match byte {
                0x9C => (Self::Ground, TransitionAction::None),
                _ => (Self::DcsIgnore, TransitionAction::None),
            },
            Self::SosPmApcString => match byte {
                0x9C => (Self::Ground, TransitionAction::None),
                _ => (Self::SosPmApcString, TransitionAction::None),
            },
        }
    }
}

/// Action triggered during state transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionAction {
    /// No action.
    None,
    /// Print a character.
    Print,
    /// Execute a C0 control.
    Execute,
    /// Collect an intermediate or private marker byte.
    Collect,
    /// Process a CSI parameter digit/semicolon.
    Param,
    /// Dispatch a complete CSI sequence.
    CsiDispatch,
    /// Dispatch an ESC sequence.
    EscDispatch,
    /// Process an OSC data byte.
    OscPut,
    /// End of OSC string.
    OscEnd,
    /// DCS hook (start of passthrough).
    DcsHook,
    /// DCS data byte.
    DcsPut,
    /// DCS unhook (end of passthrough).
    DcsUnhook,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ground_printable() {
        let (state, action) = VtState::Ground.transition(b'A');
        assert_eq!(state, VtState::Ground);
        assert_eq!(action, TransitionAction::Print);
    }

    #[test]
    fn ground_c0_control() {
        let (state, action) = VtState::Ground.transition(0x07); // BEL
        assert_eq!(state, VtState::Ground);
        assert_eq!(action, TransitionAction::Execute);
    }

    #[test]
    fn escape_from_anywhere() {
        let (state, _) = VtState::Ground.transition(0x1B);
        assert_eq!(state, VtState::Escape);
    }

    #[test]
    fn csi_entry_from_escape() {
        let (state, _) = VtState::Escape.transition(b'[');
        assert_eq!(state, VtState::CsiEntry);
    }

    #[test]
    fn csi_param_accumulation() {
        let (state, action) = VtState::CsiEntry.transition(b'3');
        assert_eq!(state, VtState::CsiParam);
        assert_eq!(action, TransitionAction::Param);
    }

    #[test]
    fn csi_dispatch_on_final() {
        let (state, action) = VtState::CsiParam.transition(b'm');
        assert_eq!(state, VtState::Ground);
        assert_eq!(action, TransitionAction::CsiDispatch);
    }

    #[test]
    fn osc_from_escape() {
        let (state, _) = VtState::Escape.transition(b']');
        assert_eq!(state, VtState::OscString);
    }

    #[test]
    fn osc_terminated_by_bel() {
        let (state, action) = VtState::OscString.transition(0x07);
        assert_eq!(state, VtState::Ground);
        assert_eq!(action, TransitionAction::OscEnd);
    }

    #[test]
    fn dcs_from_escape() {
        let (state, _) = VtState::Escape.transition(b'P');
        assert_eq!(state, VtState::DcsEntry);
    }

    #[test]
    fn state_is_ground() {
        assert!(VtState::Ground.is_ground());
        assert!(!VtState::Escape.is_ground());
    }

    #[test]
    fn state_is_csi() {
        assert!(VtState::CsiEntry.is_csi());
        assert!(VtState::CsiParam.is_csi());
        assert!(!VtState::Ground.is_csi());
    }

    #[test]
    fn state_is_dcs() {
        assert!(VtState::DcsPassthrough.is_dcs());
        assert!(!VtState::OscString.is_dcs());
    }

    #[test]
    fn cancel_from_anywhere() {
        // CAN (0x18) should return to Ground
        let (state, action) = VtState::CsiParam.transition(0x18);
        assert_eq!(state, VtState::Ground);
        assert_eq!(action, TransitionAction::Execute);
    }
}
