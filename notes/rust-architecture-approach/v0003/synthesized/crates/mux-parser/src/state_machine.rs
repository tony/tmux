//! VT parser state machine -- 14 states per Paul Williams' model.
//!
//! Reference: tmux input.c state machine (input_state_table).

/// Parser states matching the Paul Williams VT parser model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VtState {
    /// Ground state -- normal character processing.
    Ground,
    /// Escape sequence started (after ESC).
    Escape,
    /// Escape intermediate (ESC + intermediate byte).
    EscapeIntermediate,
    /// CSI entry (after ESC [).
    CsiEntry,
    /// CSI parameter accumulation.
    CsiParam,
    /// CSI intermediate byte processing.
    CsiIntermediate,
    /// CSI ignore (invalid sequence, consume until final byte).
    CsiIgnore,
    /// DCS entry (Device Control String).
    DcsEntry,
    /// DCS parameter accumulation.
    DcsParam,
    /// DCS intermediate.
    DcsIntermediate,
    /// DCS passthrough (payload delivery).
    DcsPassthrough,
    /// DCS ignore.
    DcsIgnore,
    /// OSC string (Operating System Command).
    OscString,
    /// SOS/PM/APC string.
    SosPmApcString,
}

impl VtState {
    /// Whether this state is a ground state where normal chars print.
    #[must_use]
    pub const fn is_ground(&self) -> bool {
        matches!(self, Self::Ground)
    }

    /// Whether this state is part of a CSI sequence.
    #[must_use]
    pub const fn is_csi(&self) -> bool {
        matches!(
            self,
            Self::CsiEntry | Self::CsiParam | Self::CsiIntermediate | Self::CsiIgnore
        )
    }

    /// Whether this state is part of a DCS sequence.
    #[must_use]
    pub const fn is_dcs(&self) -> bool {
        matches!(
            self,
            Self::DcsEntry
                | Self::DcsParam
                | Self::DcsIntermediate
                | Self::DcsPassthrough
                | Self::DcsIgnore
        )
    }
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
    fn default_is_ground() {
        assert_eq!(VtState::default(), VtState::Ground);
    }

    #[test]
    fn ground_detection() {
        assert!(VtState::Ground.is_ground());
        assert!(!VtState::CsiEntry.is_ground());
    }

    #[test]
    fn csi_detection() {
        assert!(VtState::CsiEntry.is_csi());
        assert!(VtState::CsiParam.is_csi());
        assert!(VtState::CsiIntermediate.is_csi());
        assert!(VtState::CsiIgnore.is_csi());
        assert!(!VtState::Ground.is_csi());
    }

    #[test]
    fn dcs_detection() {
        assert!(VtState::DcsEntry.is_dcs());
        assert!(VtState::DcsPassthrough.is_dcs());
        assert!(!VtState::Ground.is_dcs());
    }

    #[test]
    fn all_states_exist() {
        // Verify all 14 states can be constructed
        let states = [
            VtState::Ground,
            VtState::Escape,
            VtState::EscapeIntermediate,
            VtState::CsiEntry,
            VtState::CsiParam,
            VtState::CsiIntermediate,
            VtState::CsiIgnore,
            VtState::DcsEntry,
            VtState::DcsParam,
            VtState::DcsIntermediate,
            VtState::DcsPassthrough,
            VtState::DcsIgnore,
            VtState::OscString,
            VtState::SosPmApcString,
        ];
        assert_eq!(states.len(), 14);
    }
}
