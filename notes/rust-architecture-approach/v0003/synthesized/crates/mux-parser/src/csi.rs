//! CSI sequence parameter parsing and dispatch.

/// Parsed CSI parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CsiParams {
    /// Parameter values (semicolon-delimited). Empty = default (0).
    params: Vec<u16>,
    /// Intermediate bytes (between params and final byte).
    intermediates: Vec<u8>,
}

impl CsiParams {
    /// Create new empty params.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a parameter value.
    pub fn push_param(&mut self, value: u16) {
        self.params.push(value);
    }

    /// Push an intermediate byte.
    pub fn push_intermediate(&mut self, byte: u8) {
        self.intermediates.push(byte);
    }

    /// Get a parameter by index, with a default value.
    #[must_use]
    pub fn get(&self, idx: usize, default: u16) -> u16 {
        self.params.get(idx).copied().unwrap_or(default)
    }

    /// Number of parameters.
    #[must_use]
    pub fn len(&self) -> usize {
        self.params.len()
    }

    /// Whether there are no parameters.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    /// Get intermediate bytes.
    #[must_use]
    pub fn intermediates(&self) -> &[u8] {
        &self.intermediates
    }

    /// Parse CSI parameters from a byte slice (e.g., "1;2;3").
    #[must_use]
    pub fn parse(data: &[u8]) -> Self {
        let mut params = Self::new();
        let mut current: u16 = 0;
        let mut has_digit = false;

        for &b in data {
            match b {
                b'0'..=b'9' => {
                    current = current.saturating_mul(10).saturating_add(u16::from(b - b'0'));
                    has_digit = true;
                }
                b';' => {
                    params.push_param(if has_digit { current } else { 0 });
                    current = 0;
                    has_digit = false;
                }
                0x20..=0x2F => {
                    params.push_intermediate(b);
                }
                _ => {}
            }
        }
        if has_digit {
            params.push_param(current);
        }

        params
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
    fn parse_single() {
        let p = CsiParams::parse(b"5");
        assert_eq!(p.get(0, 0), 5);
    }

    #[test]
    fn parse_multiple() {
        let p = CsiParams::parse(b"1;2;3");
        assert_eq!(p.len(), 3);
        assert_eq!(p.get(0, 0), 1);
        assert_eq!(p.get(1, 0), 2);
        assert_eq!(p.get(2, 0), 3);
    }

    #[test]
    fn parse_default_value() {
        let p = CsiParams::parse(b"");
        assert_eq!(p.get(0, 1), 1); // default
    }

    #[test]
    fn parse_with_empty_params() {
        let p = CsiParams::parse(b";5");
        assert_eq!(p.get(0, 1), 0); // empty before semicolon -> 0
        assert_eq!(p.get(1, 0), 5);
    }

    #[test]
    fn parse_large_value_saturates() {
        let p = CsiParams::parse(b"99999");
        assert!(p.get(0, 0) <= u16::MAX);
    }
}
