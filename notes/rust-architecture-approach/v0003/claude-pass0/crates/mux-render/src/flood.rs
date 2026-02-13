//! Flood fairness model.
//!
//! When a pane floods output (e.g., `yes`, `cat /dev/urandom | xxd`), the
//! renderer must not spend all time on that one pane. This module implements
//! per-pane row quota with stride rotation.

use mux_types::PaneId;

/// Per-pane render budget tracking with stride rotation.
#[derive(Debug)]
pub struct FloodScheduler {
    /// Total row budget per render cycle.
    budget: u16,
    /// Rotation offset for fairness (increments each cycle).
    stride_offset: usize,
    /// Per-pane deferred row counts.
    deferred: Vec<(PaneId, u16)>,
}

impl FloodScheduler {
    /// Create a new flood scheduler with the given total budget.
    #[must_use]
    pub fn new(budget: u16) -> Self {
        Self {
            budget,
            stride_offset: 0,
            deferred: Vec::new(),
        }
    }

    /// Compute per-pane quotas for a render cycle.
    ///
    /// Returns (pane_id, row_quota) pairs in the rotated order for this cycle.
    #[must_use]
    pub fn compute_quotas(&self, visible_panes: &[PaneId]) -> Vec<(PaneId, u16)> {
        if visible_panes.is_empty() {
            return Vec::new();
        }

        let n = visible_panes.len();
        let per_pane = self.budget / n as u16;
        let remainder = self.budget % n as u16;

        let mut quotas = Vec::with_capacity(n);
        for i in 0..n {
            let rotated_idx = (i + self.stride_offset) % n;
            let extra = if (i as u16) < remainder { 1 } else { 0 };
            quotas.push((visible_panes[rotated_idx], per_pane + extra));
        }
        quotas
    }

    /// Advance the stride offset for the next render cycle.
    pub fn advance_stride(&mut self) {
        self.stride_offset += 1;
    }

    /// Record deferred rows for a pane (rows that exceeded quota).
    pub fn defer_rows(&mut self, pane_id: PaneId, count: u16) {
        if let Some(entry) = self.deferred.iter_mut().find(|(id, _)| *id == pane_id) {
            entry.1 = entry.1.saturating_add(count);
        } else {
            self.deferred.push((pane_id, count));
        }
    }

    /// Get the number of deferred rows for a pane.
    #[must_use]
    pub fn deferred_rows(&self, pane_id: PaneId) -> u16 {
        self.deferred
            .iter()
            .find(|(id, _)| *id == pane_id)
            .map(|(_, count)| *count)
            .unwrap_or(0)
    }

    /// Clear deferred rows for a pane (after they've been rendered).
    pub fn clear_deferred(&mut self, pane_id: PaneId) {
        self.deferred.retain(|(id, _)| *id != pane_id);
    }

    /// Total budget.
    #[must_use]
    pub const fn budget(&self) -> u16 {
        self.budget
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
    fn empty_panes() {
        let sched = FloodScheduler::new(48);
        let quotas = sched.compute_quotas(&[]);
        assert!(quotas.is_empty());
    }

    #[test]
    fn single_pane_gets_full_budget() {
        let sched = FloodScheduler::new(48);
        let panes = vec![PaneId::new(1)];
        let quotas = sched.compute_quotas(&panes);
        assert_eq!(quotas.len(), 1);
        assert_eq!(quotas[0].1, 48);
    }

    #[test]
    fn equal_distribution() {
        let sched = FloodScheduler::new(48);
        let panes = vec![PaneId::new(1), PaneId::new(2), PaneId::new(3)];
        let quotas = sched.compute_quotas(&panes);
        let total: u16 = quotas.iter().map(|(_, q)| q).sum();
        assert_eq!(total, 48);
    }

    #[test]
    fn stride_rotation() {
        let mut sched = FloodScheduler::new(48);
        let panes = vec![PaneId::new(1), PaneId::new(2), PaneId::new(3)];

        let q0 = sched.compute_quotas(&panes);
        sched.advance_stride();
        let q1 = sched.compute_quotas(&panes);

        // First pane in q0 should be different from first pane in q1
        assert_ne!(q0[0].0, q1[0].0);
    }

    #[test]
    fn stride_wraps_around() {
        let mut sched = FloodScheduler::new(48);
        let panes = vec![PaneId::new(1), PaneId::new(2), PaneId::new(3)];

        for _ in 0..3 {
            sched.advance_stride();
        }
        let q = sched.compute_quotas(&panes);
        // After 3 advances with 3 panes, should be back to original order
        assert_eq!(q[0].0, PaneId::new(1));
    }

    #[test]
    fn deferred_rows() {
        let mut sched = FloodScheduler::new(48);
        sched.defer_rows(PaneId::new(1), 10);
        assert_eq!(sched.deferred_rows(PaneId::new(1)), 10);
        sched.defer_rows(PaneId::new(1), 5);
        assert_eq!(sched.deferred_rows(PaneId::new(1)), 15);
    }

    #[test]
    fn clear_deferred() {
        let mut sched = FloodScheduler::new(48);
        sched.defer_rows(PaneId::new(1), 10);
        sched.clear_deferred(PaneId::new(1));
        assert_eq!(sched.deferred_rows(PaneId::new(1)), 0);
    }

    #[test]
    fn deferred_unknown_pane() {
        let sched = FloodScheduler::new(48);
        assert_eq!(sched.deferred_rows(PaneId::new(99)), 0);
    }
}
