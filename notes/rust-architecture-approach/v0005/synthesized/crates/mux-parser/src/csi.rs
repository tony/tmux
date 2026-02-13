//! CSI parameter accumulation and parsing.
//!
//! CSI sequences support up to 16 parameters. Parameter values saturate
//! at u16::MAX to prevent overflow from malicious input.

/// Maximum number of CSI parameters.
pub const MAX_CSI_PARAMS: usize = 16;

/// Accumulated CSI parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsiParams {
    /// Parameter values.
    params: [u16; MAX_CSI_PARAMS],
    /// Number of parameters accumulated.
    count: usize,
    /// Private marker byte (e.g., '?' for DECSM).
    pub private_marker: Option<u8>,
    /// Intermediate bytes.
    pub intermediates: Vec<u8>,
    /// Final byte of the CSI sequence.
    pub final_byte: u8,
}

impl Default for CsiParams {
    fn default() -> Self {
        Self::new()
    }
}

impl CsiParams {
    /// Create empty CSI parameters.
    pub fn new() -> Self {
        Self {
            params: [0; MAX_CSI_PARAMS],
            count: 0,
            private_marker: None,
            intermediates: Vec::new(),
            final_byte: 0,
        }
    }

    /// Reset all parameters for reuse.
    pub fn clear(&mut self) {
        self.params = [0; MAX_CSI_PARAMS];
        self.count = 0;
        self.private_marker = None;
        self.intermediates.clear();
        self.final_byte = 0;
    }

    /// Add a digit to the current parameter (saturating at u16::MAX).
    pub fn add_digit(&mut self, digit: u8) {
        if self.count == 0 {
            self.count = 1;
        }
        let idx = self.count.saturating_sub(1);
        if idx < MAX_CSI_PARAMS {
            self.params[idx] = self.params[idx]
                .saturating_mul(10)
                .saturating_add(u16::from(digit));
        }
    }

    /// Move to the next parameter (semicolon separator).
    pub fn next_param(&mut self) {
        if self.count < MAX_CSI_PARAMS {
            self.count += 1;
        }
    }

    /// Process a CSI parameter byte (digit or semicolon).
    pub fn process_param_byte(&mut self, byte: u8) {
        match byte {
            b'0'..=b'9' => self.add_digit(byte - b'0'),
            b';' => self.next_param(),
            _ => {}
        }
    }

    /// Get a parameter value with a default if not present.
    pub fn get(&self, index: usize, default: u16) -> u16 {
        if index < self.count {
            let val = self.params[index];
            if val == 0 { default } else { val }
        } else {
            default
        }
    }

    /// Get a raw parameter value (0 if not present, not substituted with default).
    pub fn get_raw(&self, index: usize) -> u16 {
        if index < self.count {
            self.params[index]
        } else {
            0
        }
    }

    /// Number of parameters.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns true if no parameters were accumulated.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Returns true if this is a private-mode sequence (e.g., CSI ? Ps h).
    pub fn is_private(&self) -> bool {
        self.private_marker.is_some()
    }

    /// Iterate over parameter values.
    pub fn iter(&self) -> impl Iterator<Item = u16> + '_ {
        self.params[..self.count].iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_params() {
        let p = CsiParams::new();
        assert_eq!(p.len(), 0);
        assert!(p.is_empty());
    }

    #[test]
    fn single_param() {
        let mut p = CsiParams::new();
        p.add_digit(4);
        p.add_digit(2);
        assert_eq!(p.get(0, 1), 42);
    }

    #[test]
    fn multiple_params() {
        let mut p = CsiParams::new();
        // "1;2;3"
        p.add_digit(1);
        p.next_param();
        p.add_digit(2);
        p.next_param();
        p.add_digit(3);
        assert_eq!(p.len(), 3);
        assert_eq!(p.get(0, 0), 1);
        assert_eq!(p.get(1, 0), 2);
        assert_eq!(p.get(2, 0), 3);
    }

    #[test]
    fn default_for_missing() {
        let p = CsiParams::new();
        assert_eq!(p.get(0, 99), 99);
        assert_eq!(p.get(5, 42), 42);
    }

    #[test]
    fn default_for_zero_param() {
        let mut p = CsiParams::new();
        // Param explicitly 0 should use default
        p.next_param(); // count = 1, params[0] = 0
        assert_eq!(p.get(0, 1), 1);
    }

    #[test]
    fn saturates_at_u16_max() {
        let mut p = CsiParams::new();
        for _ in 0..10 {
            p.add_digit(9);
        }
        assert_eq!(p.get(0, 0), u16::MAX);
    }

    #[test]
    fn process_param_byte() {
        let mut p = CsiParams::new();
        for b in b"10;20;30".iter() {
            p.process_param_byte(*b);
        }
        assert_eq!(p.get(0, 0), 10);
        assert_eq!(p.get(1, 0), 20);
        assert_eq!(p.get(2, 0), 30);
    }

    #[test]
    fn max_params_cap() {
        let mut p = CsiParams::new();
        for _ in 0..20 {
            p.add_digit(1);
            p.next_param();
        }
        assert!(p.len() <= MAX_CSI_PARAMS);
    }

    #[test]
    fn private_marker() {
        let mut p = CsiParams::new();
        p.private_marker = Some(b'?');
        assert!(p.is_private());
    }

    #[test]
    fn clear_resets() {
        let mut p = CsiParams::new();
        p.add_digit(5);
        p.private_marker = Some(b'?');
        p.clear();
        assert!(p.is_empty());
        assert!(!p.is_private());
    }

    #[test]
    fn iter_params() {
        let mut p = CsiParams::new();
        p.process_param_byte(b'1');
        p.process_param_byte(b';');
        p.process_param_byte(b'2');
        let values: Vec<_> = p.iter().collect();
        assert_eq!(values, vec![1, 2]);
    }

    #[test]
    fn get_raw_returns_zero() {
        let p = CsiParams::new();
        assert_eq!(p.get_raw(0), 0);
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn param_never_exceeds_max(digits in proptest::collection::vec(0u8..10, 1..20)) {
                let mut p = CsiParams::new();
                for d in digits {
                    p.add_digit(d);
                }
                prop_assert!(p.get(0, 0) <= u16::MAX);
            }

            #[test]
            fn param_count_bounded(sep_count in 0usize..30) {
                let mut p = CsiParams::new();
                for _ in 0..sep_count {
                    p.add_digit(1);
                    p.next_param();
                }
                prop_assert!(p.len() <= MAX_CSI_PARAMS);
            }
        }
    }
}
