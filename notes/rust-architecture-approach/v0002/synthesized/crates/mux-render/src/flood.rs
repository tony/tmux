//! Flood fairness model: per-pane row quota with stride rotation.
//!
//! When a pane floods output (e.g., `cat /dev/urandom | xxd`), the renderer
//! must not let that pane starve other panes of render cycles. This module
//! implements a per-pane row quota with stride-based rotation to ensure
//! fair rendering across all visible panes.
//!
//! ## Algorithm (merged from GPT's synthesis)
//!
//! 1. Each render cycle has a total row budget (e.g., terminal height * 2).
//! 2. Each pane receives a quota = budget / num_visible_panes.
//! 3. Panes are rendered in stride-rotated order: the first pane to render
//!    rotates each cycle, preventing positional bias.
//! 4. If a pane's dirty rows exceed its quota, excess rows are deferred
//!    to the next cycle.

use mux_types::PaneId;

/// Per-pane render quota tracking.
#[derive(Debug, Clone)]
pub struct PaneQuota {
    pub pane_id: PaneId,
    /// Rows consumed this cycle.
    pub rows_consumed: u32,
    /// Rows deferred from previous cycles.
    pub deferred_rows: u32,
}

/// The flood fairness scheduler.
#[derive(Debug)]
pub struct FloodScheduler {
    /// Total row budget per render cycle.
    budget: u32,
    /// Current stride offset (rotates each cycle).
    stride_offset: usize,
    /// Per-pane quota state.
    quotas: Vec<PaneQuota>,
}

impl FloodScheduler {
    /// Create a new scheduler with the given row budget.
    #[must_use]
    pub fn new(budget: u32) -> Self {
        Self {
            budget,
            stride_offset: 0,
            quotas: Vec::new(),
        }
    }

    /// Set the visible panes for this cycle.
    pub fn set_panes(&mut self, pane_ids: &[PaneId]) {
        // Preserve deferred_rows for panes that are still visible
        let mut new_quotas = Vec::with_capacity(pane_ids.len());
        for &id in pane_ids {
            let deferred = self.quotas.iter()
                .find(|q| q.pane_id == id)
                .map_or(0, |q| q.deferred_rows);
            new_quotas.push(PaneQuota {
                pane_id: id,
                rows_consumed: 0,
                deferred_rows: deferred,
            });
        }
        self.quotas = new_quotas;
    }

    /// Get the row quota for a pane in this cycle.
    #[must_use]
    pub fn quota_for(&self, _pane_id: PaneId) -> u32 {
        let n = self.quotas.len().max(1) as u32;
        self.budget / n
    }

    /// Get the pane rendering order for this cycle (stride-rotated).
    #[must_use]
    pub fn render_order(&self) -> Vec<PaneId> {
        if self.quotas.is_empty() {
            return Vec::new();
        }
        let n = self.quotas.len();
        let mut order = Vec::with_capacity(n);
        for i in 0..n {
            let idx = (i + self.stride_offset) % n;
            order.push(self.quotas[idx].pane_id);
        }
        order
    }

    /// Record that a pane consumed some rows.
    pub fn consume(&mut self, pane_id: PaneId, rows: u32) {
        if let Some(q) = self.quotas.iter_mut().find(|q| q.pane_id == pane_id) {
            q.rows_consumed += rows;
        }
    }

    /// Defer excess rows for panes that exceeded quota.
    pub fn finalize_cycle(&mut self) {
        let per_pane_quota = self.quota_for(PaneId(0));
        for q in &mut self.quotas {
            if q.rows_consumed > per_pane_quota {
                q.deferred_rows += q.rows_consumed - per_pane_quota;
            } else {
                q.deferred_rows = q.deferred_rows.saturating_sub(per_pane_quota - q.rows_consumed);
            }
            q.rows_consumed = 0;
        }
        // Advance stride for next cycle
        self.stride_offset = (self.stride_offset + 1) % self.quotas.len().max(1);
    }

    /// Current stride offset.
    #[must_use]
    pub const fn stride_offset(&self) -> usize {
        self.stride_offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_quota_distribution() {
        let sched = FloodScheduler::new(100);
        let panes = vec![PaneId(1), PaneId(2), PaneId(3), PaneId(4)];
        let mut s = sched;
        s.set_panes(&panes);
        assert_eq!(s.quota_for(PaneId(1)), 25);
    }

    #[test]
    fn stride_rotation() {
        let mut s = FloodScheduler::new(100);
        let panes = vec![PaneId(1), PaneId(2), PaneId(3)];
        s.set_panes(&panes);

        let order1 = s.render_order();
        assert_eq!(order1, vec![PaneId(1), PaneId(2), PaneId(3)]);

        s.finalize_cycle();
        let order2 = s.render_order();
        assert_eq!(order2, vec![PaneId(2), PaneId(3), PaneId(1)]);

        s.finalize_cycle();
        let order3 = s.render_order();
        assert_eq!(order3, vec![PaneId(3), PaneId(1), PaneId(2)]);
    }

    #[test]
    fn deferred_rows_tracking() {
        let mut s = FloodScheduler::new(100);
        s.set_panes(&[PaneId(1), PaneId(2)]);
        // Pane 1 consumes way more than its quota
        s.consume(PaneId(1), 80); // quota is 50
        s.consume(PaneId(2), 10);
        s.finalize_cycle();
        // Pane 1 should have deferred rows
        let q1 = s.quotas.iter().find(|q| q.pane_id == PaneId(1));
        assert!(q1.is_some_and(|q| q.deferred_rows > 0));
    }

    #[test]
    fn empty_panes() {
        let s = FloodScheduler::new(100);
        assert!(s.render_order().is_empty());
    }

    #[test]
    fn single_pane_gets_full_budget() {
        let mut s = FloodScheduler::new(48);
        s.set_panes(&[PaneId(1)]);
        assert_eq!(s.quota_for(PaneId(1)), 48);
    }
}
