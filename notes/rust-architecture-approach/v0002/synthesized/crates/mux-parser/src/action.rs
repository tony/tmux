//! VT parser output actions.

/// Actions emitted by the VT parser for each processed byte sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VtAction {
    /// Print a character to the current cursor position.
    Print(char),
    /// Execute a C0 control character (0x00-0x1F excl. ESC).
    Execute(u8),
    /// CSI dispatch: final char, intermediate chars, params.
    CsiDispatch { params: Vec<u16>, intermediates: Vec<u8>, final_byte: u8 },
    /// ESC dispatch: intermediate chars, final byte.
    EscDispatch { intermediates: Vec<u8>, final_byte: u8 },
    /// OSC dispatch: full string payload.
    OscDispatch(Vec<u8>),
    /// DCS hook: params, intermediates, final byte.
    DcsHook { params: Vec<u16>, intermediates: Vec<u8>, final_byte: u8 },
    /// DCS data bytes (passthrough).
    DcsPut(u8),
    /// DCS unhook (sequence complete).
    DcsUnhook,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_action() {
        let a = VtAction::Print('A');
        assert!(matches!(a, VtAction::Print('A')));
    }

    #[test]
    fn csi_dispatch() {
        let a = VtAction::CsiDispatch {
            params: vec![1, 0],
            intermediates: vec![],
            final_byte: b'm',
        };
        if let VtAction::CsiDispatch { final_byte, .. } = a {
            assert_eq!(final_byte, b'm');
        }
    }
}
