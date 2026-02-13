//! VT parser states per Paul Williams' state machine.
//!
//! Reference: <https://vt100.net/emu/dec_ansi_parser>

/// States of the VT parser state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VtState {
    /// Normal text processing.
    Ground,
    /// ESC seen, waiting for next byte.
    Escape,
    /// ESC intermediate characters.
    EscapeIntermediate,
    /// CSI entry (after ESC [).
    CsiEntry,
    /// CSI parameter accumulation.
    CsiParam,
    /// CSI intermediate characters.
    CsiIntermediate,
    /// CSI ignoring rest of sequence.
    CsiIgnore,
    /// DCS entry (after ESC P or 0x90).
    DcsEntry,
    /// DCS parameter accumulation.
    DcsParam,
    /// DCS intermediate characters.
    DcsIntermediate,
    /// DCS passthrough data.
    DcsPassthrough,
    /// DCS ignoring rest of sequence.
    DcsIgnore,
    /// OSC string (after ESC ] or 0x9D).
    OscString,
    /// SOS/PM/APC string (ignored, consumed until ST).
    SosPmApcString,
}

impl Default for VtState {
    fn default() -> Self {
        Self::Ground
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_ground() {
        assert_eq!(VtState::default(), VtState::Ground);
    }
}
