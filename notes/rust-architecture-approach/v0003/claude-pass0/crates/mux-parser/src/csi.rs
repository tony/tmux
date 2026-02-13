//! CSI sequence parameter parsing.
//!
//! Parses semicolon-delimited numeric parameters from CSI sequences.
//! For example, `ESC[1;31m` has params [1, 31] with final byte 'm' (SGR).

/// Maximum number of CSI parameters we track (matches tmux limit).
pub const MAX_CSI_PARAMS: usize = 16;

/// Parsed CSI parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsiParams {
    /// Parameter values (0 = default/unset).
    params: [u16; MAX_CSI_PARAMS],
    /// Number of parameters accumulated.
    len: usize,
    /// Private marker character (e.g., '?' for DECSM/DECRM).
    pub private_marker: Option<u8>,
    /// Intermediate characters collected.
    pub intermediates: Vec<u8>,
    /// The final byte that triggered dispatch.
    pub final_byte: u8,
}

impl CsiParams {
    /// Create empty CSI parameters.
    #[must_use]
    pub fn new() -> Self {
        Self {
            params: [0; MAX_CSI_PARAMS],
            len: 0,
            private_marker: None,
            intermediates: Vec::new(),
            final_byte: 0,
        }
    }

    /// Reset for reuse.
    pub fn reset(&mut self) {
        self.params = [0; MAX_CSI_PARAMS];
        self.len = 0;
        self.private_marker = None;
        self.intermediates.clear();
        self.final_byte = 0;
    }

    /// Process a parameter byte (digit or semicolon).
    pub fn push_param_byte(&mut self, byte: u8) {
        match byte {
            b'0'..=b'9' => {
                if self.len == 0 {
                    self.len = 1;
                }
                let idx = self.len - 1;
                if idx < MAX_CSI_PARAMS {
                    self.params[idx] = self.params[idx]
                        .saturating_mul(10)
                        .saturating_add(u16::from(byte - b'0'));
                }
            }
            b';' => {
                if self.len < MAX_CSI_PARAMS {
                    self.len += 1;
                }
            }
            _ => {}
        }
    }

    /// Process a collect byte (intermediate or private marker).
    pub fn push_collect_byte(&mut self, byte: u8) {
        match byte {
            b'?' | b'>' | b'<' | b'=' => {
                self.private_marker = Some(byte);
            }
            0x20..=0x2F => {
                self.intermediates.push(byte);
            }
            _ => {}
        }
    }

    /// Number of parameters.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether no parameters were given.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get parameter at index, with a default value if unset.
    #[must_use]
    pub fn get(&self, index: usize, default: u16) -> u16 {
        if index < self.len && self.params[index] != 0 {
            self.params[index]
        } else {
            default
        }
    }

    /// Get parameter at index, returning 0 if unset.
    #[must_use]
    pub fn get_raw(&self, index: usize) -> u16 {
        if index < self.len {
            self.params[index]
        } else {
            0
        }
    }

    /// Whether this is a private-mode sequence (e.g., DECSM: CSI ? Pm h).
    #[must_use]
    pub fn is_private(&self) -> bool {
        self.private_marker.is_some()
    }

    /// Iterate over all parameters.
    pub fn iter(&self) -> impl Iterator<Item = u16> + '_ {
        self.params[..self.len].iter().copied()
    }
}

impl Default for CsiParams {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_params() {
        let p = CsiParams::new();
        assert!(p.is_empty());
        assert_eq!(p.len(), 0);
    }

    #[test]
    fn single_param() {
        let mut p = CsiParams::new();
        for byte in b"31" {
            p.push_param_byte(*byte);
        }
        assert_eq!(p.len(), 1);
        assert_eq!(p.get(0, 0), 31);
    }

    #[test]
    fn multiple_params() {
        let mut p = CsiParams::new();
        for byte in b"1;31;42" {
            p.push_param_byte(*byte);
        }
        assert_eq!(p.len(), 3);
        assert_eq!(p.get(0, 0), 1);
        assert_eq!(p.get(1, 0), 31);
        assert_eq!(p.get(2, 0), 42);
    }

    #[test]
    fn default_param_value() {
        let p = CsiParams::new();
        assert_eq!(p.get(0, 1), 1); // default
    }

    #[test]
    fn private_marker() {
        let mut p = CsiParams::new();
        p.push_collect_byte(b'?');
        assert!(p.is_private());
        assert_eq!(p.private_marker, Some(b'?'));
    }

    #[test]
    fn intermediate_collection() {
        let mut p = CsiParams::new();
        p.push_collect_byte(b' ');
        assert_eq!(p.intermediates, vec![b' ']);
    }

    #[test]
    fn reset_clears() {
        let mut p = CsiParams::new();
        p.push_param_byte(b'5');
        p.push_collect_byte(b'?');
        p.reset();
        assert!(p.is_empty());
        assert!(!p.is_private());
    }

    #[test]
    fn max_params_saturate() {
        let mut p = CsiParams::new();
        for _ in 0..MAX_CSI_PARAMS + 5 {
            p.push_param_byte(b'1');
            p.push_param_byte(b';');
        }
        assert!(p.len() <= MAX_CSI_PARAMS);
    }

    #[test]
    fn param_overflow_saturates() {
        let mut p = CsiParams::new();
        // Push many digits -- should saturate instead of overflowing
        for _ in 0..10 {
            p.push_param_byte(b'9');
        }
        assert_eq!(p.get(0, 0), u16::MAX);
    }

    #[test]
    fn iter_params() {
        let mut p = CsiParams::new();
        for byte in b"10;20;30" {
            p.push_param_byte(*byte);
        }
        let values: Vec<u16> = p.iter().collect();
        assert_eq!(values, vec![10, 20, 30]);
    }
}
