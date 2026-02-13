//! Per-pane flood fairness engine.
//!
//! Ensures each pane gets a fair share of render bandwidth by enforcing
//! a per-pane row quota with stride rotation.

/// Configuration for the flood fairness engine.
#[derive(Debug, Clone)]
pub struct FloodConfig {
    /// Maximum rows per pane per render cycle.
    pub row_quota: u16,
    /// Total render budget (rows across all panes).
    pub total_budget: u16,
}

impl FloodConfig {
    /// Default flood config.
    #[must_use]
    pub const fn default_config() -> Self {
        Self {
            row_quota: 24,
            total_budget: 256,
        }
    }
}

impl Default for FloodConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

/// Flood fairness state tracker.
#[derive(Debug)]
pub struct FloodState {
    config: FloodConfig,
    /// Current stride offset for rotation.
    stride_offset: usize,
    /// Per-pane row counters for the current cycle.
    pane_rows: Vec<u16>,
}

impl FloodState {
    /// Create a new flood state.
    #[must_use]
    pub fn new(config: FloodConfig) -> Self {
        Self {
            config,
            stride_offset: 0,
            pane_rows: Vec::new(),
        }
    }

    /// Begin a new render cycle, resetting counters.
    pub fn begin_cycle(&mut self, pane_count: usize) {
        self.pane_rows.clear();
        self.pane_rows.resize(pane_count, 0);
        self.stride_offset = (self.stride_offset + 1) % pane_count.max(1);
    }

    /// Check if a pane can render more rows in this cycle.
    #[must_use]
    pub fn can_render(&self, pane_idx: usize) -> bool {
        self.pane_rows
            .get(pane_idx)
            .map_or(false, |&count| count < self.config.row_quota)
    }

    /// Record that a pane rendered a row.
    pub fn record_row(&mut self, pane_idx: usize) {
        if let Some(count) = self.pane_rows.get_mut(pane_idx) {
            *count += 1;
        }
    }

    /// Current stride offset for round-robin fairness.
    #[must_use]
    pub const fn stride_offset(&self) -> usize {
        self.stride_offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = FloodConfig::default();
        assert_eq!(config.row_quota, 24);
        assert_eq!(config.total_budget, 256);
    }

    #[test]
    fn new_flood_state() {
        let state = FloodState::new(FloodConfig::default());
        assert_eq!(state.stride_offset(), 0);
    }

    #[test]
    fn can_render_initially() {
        let mut state = FloodState::new(FloodConfig::default());
        state.begin_cycle(3);
        assert!(state.can_render(0));
        assert!(state.can_render(1));
        assert!(state.can_render(2));
    }

    #[test]
    fn quota_exhaustion() {
        let config = FloodConfig {
            row_quota: 2,
            total_budget: 256,
        };
        let mut state = FloodState::new(config);
        state.begin_cycle(1);
        state.record_row(0);
        state.record_row(0);
        assert!(!state.can_render(0));
    }

    #[test]
    fn stride_rotates() {
        let mut state = FloodState::new(FloodConfig::default());
        state.begin_cycle(3);
        let s1 = state.stride_offset();
        state.begin_cycle(3);
        let s2 = state.stride_offset();
        assert_ne!(s1, s2);
    }

    #[test]
    fn out_of_bounds_pane_cannot_render() {
        let mut state = FloodState::new(FloodConfig::default());
        state.begin_cycle(2);
        assert!(!state.can_render(5));
    }

    #[test]
    fn single_pane_gets_full_budget() {
        let config = FloodConfig {
            row_quota: 100,
            total_budget: 100,
        };
        let mut state = FloodState::new(config);
        state.begin_cycle(1);
        // Single pane should get the entire budget
        for _ in 0..100 {
            assert!(state.can_render(0));
            state.record_row(0);
        }
        assert!(!state.can_render(0));
    }

    #[test]
    fn stride_cycles_through_all_panes() {
        let mut state = FloodState::new(FloodConfig::default());
        let mut seen = std::collections::HashSet::new();
        for _ in 0..3 {
            state.begin_cycle(3);
            seen.insert(state.stride_offset());
        }
        assert_eq!(seen.len(), 3);
    }
}
