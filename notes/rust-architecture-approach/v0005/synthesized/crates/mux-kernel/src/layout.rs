//! Layout engine using a binary tree model.
//!
//! Supports tmux's 5 built-in layout algorithms plus custom layouts.

use mux_types::id::PaneId;

/// Layout orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// A node in the layout binary tree.
#[derive(Debug, Clone)]
pub enum LayoutNode {
    /// A leaf pane with position and dimensions.
    Leaf {
        pane_id: PaneId,
        x: u16,
        y: u16,
        cols: u16,
        rows: u16,
    },
    /// A split container.
    Split {
        orientation: Orientation,
        x: u16,
        y: u16,
        cols: u16,
        rows: u16,
        children: Vec<LayoutNode>,
    },
}

impl LayoutNode {
    /// Get the dimensions of this node.
    pub fn size(&self) -> (u16, u16) {
        match self {
            Self::Leaf { cols, rows, .. } => (*cols, *rows),
            Self::Split { cols, rows, .. } => (*cols, *rows),
        }
    }

    /// Get the position of this node.
    pub fn position(&self) -> (u16, u16) {
        match self {
            Self::Leaf { x, y, .. } => (*x, *y),
            Self::Split { x, y, .. } => (*x, *y),
        }
    }

    /// Count leaf panes.
    pub fn pane_count(&self) -> usize {
        match self {
            Self::Leaf { .. } => 1,
            Self::Split { children, .. } => children.iter().map(Self::pane_count).sum(),
        }
    }

    /// Collect all leaf pane IDs.
    pub fn pane_ids(&self) -> Vec<PaneId> {
        match self {
            Self::Leaf { pane_id, .. } => vec![*pane_id],
            Self::Split { children, .. } => {
                children.iter().flat_map(Self::pane_ids).collect()
            }
        }
    }
}

/// Built-in layout algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutAlgorithm {
    EvenHorizontal,
    EvenVertical,
    MainHorizontal,
    MainVertical,
    Tiled,
}

/// Distribute `total` evenly among `count` items.
/// Returns a vector of sizes that sum exactly to `total`.
pub fn distribute_evenly(total: u16, count: usize) -> Vec<u16> {
    if count == 0 {
        return vec![];
    }
    let base = total / count as u16;
    let remainder = (total % count as u16) as usize;
    let mut sizes = vec![base; count];
    for item in sizes.iter_mut().take(remainder) {
        *item += 1;
    }
    sizes
}

/// Apply an even-horizontal layout.
pub fn even_horizontal(pane_ids: &[PaneId], cols: u16, rows: u16) -> LayoutNode {
    if pane_ids.len() <= 1 {
        return LayoutNode::Leaf {
            pane_id: pane_ids.first().copied().unwrap_or(PaneId(0)),
            x: 0, y: 0, cols, rows,
        };
    }
    let widths = distribute_evenly(cols, pane_ids.len());
    let mut children = Vec::new();
    let mut x = 0u16;
    for (i, &pid) in pane_ids.iter().enumerate() {
        children.push(LayoutNode::Leaf {
            pane_id: pid,
            x, y: 0,
            cols: widths[i],
            rows,
        });
        x += widths[i];
    }
    LayoutNode::Split {
        orientation: Orientation::Vertical,
        x: 0, y: 0, cols, rows,
        children,
    }
}

/// Apply an even-vertical layout.
pub fn even_vertical(pane_ids: &[PaneId], cols: u16, rows: u16) -> LayoutNode {
    if pane_ids.len() <= 1 {
        return LayoutNode::Leaf {
            pane_id: pane_ids.first().copied().unwrap_or(PaneId(0)),
            x: 0, y: 0, cols, rows,
        };
    }
    let heights = distribute_evenly(rows, pane_ids.len());
    let mut children = Vec::new();
    let mut y = 0u16;
    for (i, &pid) in pane_ids.iter().enumerate() {
        children.push(LayoutNode::Leaf {
            pane_id: pid,
            x: 0, y,
            cols,
            rows: heights[i],
        });
        y += heights[i];
    }
    LayoutNode::Split {
        orientation: Orientation::Horizontal,
        x: 0, y: 0, cols, rows,
        children,
    }
}

/// Validate a custom layout checksum (4 hex digits).
pub fn validate_layout_checksum(layout_str: &str) -> bool {
    if layout_str.len() < 5 {
        return false;
    }
    let checksum_str = &layout_str[..4];
    if let Ok(expected) = u16::from_str_radix(checksum_str, 16) {
        let body = &layout_str[5..]; // skip the comma after checksum
        let computed: u16 = body.bytes().fold(0u16, |acc, b| acc.wrapping_add(u16::from(b)));
        expected == computed
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distribute_evenly_exact() {
        let sizes = distribute_evenly(100, 4);
        assert_eq!(sizes, vec![25, 25, 25, 25]);
        assert_eq!(sizes.iter().map(|&s| u32::from(s)).sum::<u32>(), 100);
    }

    #[test]
    fn distribute_evenly_remainder() {
        let sizes = distribute_evenly(10, 3);
        assert_eq!(sizes.iter().map(|&s| u32::from(s)).sum::<u32>(), 10);
        assert_eq!(sizes[0], 4); // first gets extra
        assert_eq!(sizes[1], 3);
        assert_eq!(sizes[2], 3);
    }

    #[test]
    fn distribute_evenly_single() {
        let sizes = distribute_evenly(80, 1);
        assert_eq!(sizes, vec![80]);
    }

    #[test]
    fn distribute_evenly_zero() {
        let sizes = distribute_evenly(80, 0);
        assert!(sizes.is_empty());
    }

    #[test]
    fn even_horizontal_layout() {
        let panes = vec![PaneId(1), PaneId(2), PaneId(3)];
        let layout = even_horizontal(&panes, 120, 40);
        assert_eq!(layout.pane_count(), 3);
    }

    #[test]
    fn even_vertical_layout() {
        let panes = vec![PaneId(1), PaneId(2)];
        let layout = even_vertical(&panes, 80, 24);
        assert_eq!(layout.pane_count(), 2);
    }

    #[test]
    fn layout_pane_ids() {
        let panes = vec![PaneId(1), PaneId(2), PaneId(3)];
        let layout = even_horizontal(&panes, 120, 40);
        let ids = layout.pane_ids();
        assert_eq!(ids, panes);
    }

    #[test]
    fn single_pane_layout() {
        let panes = vec![PaneId(1)];
        let layout = even_horizontal(&panes, 80, 24);
        assert_eq!(layout.pane_count(), 1);
        assert_eq!(layout.size(), (80, 24));
    }

    #[test]
    fn layout_dimensions_sum() {
        let panes = vec![PaneId(1), PaneId(2), PaneId(3), PaneId(4)];
        let layout = even_horizontal(&panes, 100, 30);
        if let LayoutNode::Split { children, .. } = &layout {
            let total_width: u16 = children.iter().map(|c| c.size().0).sum();
            assert_eq!(total_width, 100);
        }
    }

    #[test]
    fn orientation_types() {
        assert_ne!(Orientation::Horizontal, Orientation::Vertical);
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn distribute_sums_to_total(total in 1u16..500, count in 1usize..20) {
                let sizes = distribute_evenly(total, count);
                let sum: u32 = sizes.iter().map(|&s| u32::from(s)).sum();
                prop_assert_eq!(sum, u32::from(total));
            }

            #[test]
            fn even_horizontal_sums(
                pane_count in 1usize..10,
                cols in 20u16..300,
                rows in 5u16..100,
            ) {
                let panes: Vec<PaneId> = (0..pane_count).map(|i| PaneId(i as u64 + 1)).collect();
                let layout = even_horizontal(&panes, cols, rows);
                if let LayoutNode::Split { children, .. } = &layout {
                    let total_width: u16 = children.iter().map(|c| c.size().0).sum();
                    prop_assert_eq!(total_width, cols);
                }
            }

            #[test]
            fn even_vertical_sums(
                pane_count in 1usize..10,
                cols in 20u16..300,
                rows in 5u16..100,
            ) {
                let panes: Vec<PaneId> = (0..pane_count).map(|i| PaneId(i as u64 + 1)).collect();
                let layout = even_vertical(&panes, cols, rows);
                if let LayoutNode::Split { children, .. } = &layout {
                    let total_height: u16 = children.iter().map(|c| c.size().1).sum();
                    prop_assert_eq!(total_height, rows);
                }
            }
        }
    }
}
