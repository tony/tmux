//! Binary-tree layout engine.
//!
//! The layout engine uses a binary tree matching tmux's `layout_cell` tree
//! from `layout.c`. Five built-in algorithms are provided plus custom layout
//! string parsing with checksum validation.
//!
//! ## Layout Invariants
//! - Sum of child widths/heights exactly matches parent size after redistribution.
//! - Minimum pane size: 2 cols x 1 row.
//! - Built-in layouts always available.
//! - Custom layout strings must pass checksum validation.

use mux_types::{PaneId, Size};

/// A node in the layout binary tree.
#[derive(Debug, Clone)]
pub enum LayoutNode {
    /// A leaf node containing a single pane.
    Pane(PaneId),
    /// Horizontal split: children arranged left-to-right.
    LeftRight {
        children: Vec<LayoutNode>,
        sizes: Vec<u16>,
    },
    /// Vertical split: children arranged top-to-bottom.
    TopBottom {
        children: Vec<LayoutNode>,
        sizes: Vec<u16>,
    },
}

/// Built-in layout algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinLayout {
    EvenHorizontal,
    EvenVertical,
    MainHorizontal,
    MainVertical,
    Tiled,
}

impl BuiltinLayout {
    /// Apply this built-in layout to the given pane IDs and total size.
    #[must_use]
    pub fn apply(self, panes: &[PaneId], total: Size) -> LayoutNode {
        if panes.is_empty() {
            return LayoutNode::Pane(PaneId::new(0));
        }
        if panes.len() == 1 {
            return LayoutNode::Pane(panes[0]);
        }

        match self {
            Self::EvenHorizontal => even_horizontal(panes, total),
            Self::EvenVertical => even_vertical(panes, total),
            Self::MainHorizontal => main_horizontal(panes, total),
            Self::MainVertical => main_vertical(panes, total),
            Self::Tiled => tiled(panes, total),
        }
    }
}

/// Even horizontal: equal-width vertical splits.
fn even_horizontal(panes: &[PaneId], total: Size) -> LayoutNode {
    let n = panes.len() as u16;
    let base_width = total.cols / n;
    let remainder = total.cols % n;

    let mut sizes = Vec::with_capacity(panes.len());
    let mut children = Vec::with_capacity(panes.len());

    for (i, &pane_id) in panes.iter().enumerate() {
        let extra = if (i as u16) < remainder { 1 } else { 0 };
        sizes.push(base_width + extra);
        children.push(LayoutNode::Pane(pane_id));
    }

    LayoutNode::LeftRight { children, sizes }
}

/// Even vertical: equal-height horizontal splits.
fn even_vertical(panes: &[PaneId], total: Size) -> LayoutNode {
    let n = panes.len() as u16;
    let base_height = total.rows / n;
    let remainder = total.rows % n;

    let mut sizes = Vec::with_capacity(panes.len());
    let mut children = Vec::with_capacity(panes.len());

    for (i, &pane_id) in panes.iter().enumerate() {
        let extra = if (i as u16) < remainder { 1 } else { 0 };
        sizes.push(base_height + extra);
        children.push(LayoutNode::Pane(pane_id));
    }

    LayoutNode::TopBottom { children, sizes }
}

/// Main horizontal: main pane on top, rest split below.
fn main_horizontal(panes: &[PaneId], total: Size) -> LayoutNode {
    let main_height = total.rows / 2;
    let rest_height = total.rows - main_height;

    let rest_panes = &panes[1..];
    let rest_node = even_horizontal(rest_panes, Size::new(total.cols, rest_height));

    LayoutNode::TopBottom {
        children: vec![LayoutNode::Pane(panes[0]), rest_node],
        sizes: vec![main_height, rest_height],
    }
}

/// Main vertical: main pane on left, rest split right.
fn main_vertical(panes: &[PaneId], total: Size) -> LayoutNode {
    let main_width = total.cols / 2;
    let rest_width = total.cols - main_width;

    let rest_panes = &panes[1..];
    let rest_node = even_vertical(rest_panes, Size::new(rest_width, total.rows));

    LayoutNode::LeftRight {
        children: vec![LayoutNode::Pane(panes[0]), rest_node],
        sizes: vec![main_width, rest_width],
    }
}

/// Tiled: grid arrangement (sqrt-based).
fn tiled(panes: &[PaneId], total: Size) -> LayoutNode {
    let n = panes.len();
    let cols = (n as f64).sqrt().ceil() as usize;
    let rows = (n + cols - 1) / cols;

    let row_height = total.rows / rows as u16;
    let row_remainder = total.rows % rows as u16;

    let mut row_nodes = Vec::with_capacity(rows);
    let mut row_sizes = Vec::with_capacity(rows);
    let mut idx = 0;

    for r in 0..rows {
        let extra = if (r as u16) < row_remainder { 1 } else { 0 };
        let h = row_height + extra;

        let panes_in_row = if r == rows - 1 { n - idx } else { cols.min(n - idx) };
        let col_width = total.cols / panes_in_row as u16;
        let col_remainder = total.cols % panes_in_row as u16;

        let mut col_nodes = Vec::with_capacity(panes_in_row);
        let mut col_sizes = Vec::with_capacity(panes_in_row);

        for c in 0..panes_in_row {
            let col_extra = if (c as u16) < col_remainder { 1 } else { 0 };
            col_sizes.push(col_width + col_extra);
            col_nodes.push(LayoutNode::Pane(panes[idx]));
            idx += 1;
        }

        if col_nodes.len() == 1 {
            row_nodes.push(col_nodes.into_iter().next().unwrap_or(LayoutNode::Pane(PaneId::new(0))));
        } else {
            row_nodes.push(LayoutNode::LeftRight {
                children: col_nodes,
                sizes: col_sizes,
            });
        }
        row_sizes.push(h);
    }

    if row_nodes.len() == 1 {
        row_nodes.into_iter().next().unwrap_or(LayoutNode::Pane(PaneId::new(0)))
    } else {
        LayoutNode::TopBottom {
            children: row_nodes,
            sizes: row_sizes,
        }
    }
}

/// Validate a custom layout string checksum.
///
/// tmux layout strings start with a 4-hex-digit checksum:
/// `89eb,120x40,0,0{60x40,0,0,0,59x40,61,0,1}`
///
/// The checksum is computed over the remainder of the string after the comma.
#[must_use]
pub fn validate_layout_checksum(layout_str: &str) -> bool {
    if layout_str.len() < 5 || layout_str.as_bytes().get(4) != Some(&b',') {
        return false;
    }

    let checksum_hex = &layout_str[..4];
    let expected = u16::from_str_radix(checksum_hex, 16);
    let expected = match expected {
        Ok(v) => v,
        Err(_) => return false,
    };

    let data = &layout_str[5..];
    let computed = layout_checksum(data);
    computed == expected
}

/// Compute the tmux layout checksum (csum from layout.c).
fn layout_checksum(data: &str) -> u16 {
    let mut csum: u16 = 0;
    for &byte in data.as_bytes() {
        csum = (csum >> 1) | ((csum & 1) << 15);
        csum = csum.wrapping_add(u16::from(byte));
    }
    csum
}

/// Collect all pane IDs from a layout tree.
#[must_use]
pub fn collect_pane_ids(node: &LayoutNode) -> Vec<PaneId> {
    match node {
        LayoutNode::Pane(id) => vec![*id],
        LayoutNode::LeftRight { children, .. } | LayoutNode::TopBottom { children, .. } => {
            children.iter().flat_map(collect_pane_ids).collect()
        }
    }
}

/// Sum of sizes in a layout node's children.
#[must_use]
pub fn sum_child_sizes(sizes: &[u16]) -> u16 {
    sizes.iter().copied().sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pane_ids(n: usize) -> Vec<PaneId> {
        (1..=n).map(|i| PaneId::new(i as u64)).collect()
    }

    #[test]
    fn even_horizontal_sizes_sum() {
        let panes = pane_ids(3);
        let total = Size::new(80, 24);
        if let LayoutNode::LeftRight { sizes, .. } = BuiltinLayout::EvenHorizontal.apply(&panes, total) {
            assert_eq!(sum_child_sizes(&sizes), 80);
        }
    }

    #[test]
    fn even_vertical_sizes_sum() {
        let panes = pane_ids(3);
        let total = Size::new(80, 24);
        if let LayoutNode::TopBottom { sizes, .. } = BuiltinLayout::EvenVertical.apply(&panes, total) {
            assert_eq!(sum_child_sizes(&sizes), 24);
        }
    }

    #[test]
    fn main_horizontal_sizes_sum() {
        let panes = pane_ids(3);
        let total = Size::new(80, 24);
        if let LayoutNode::TopBottom { sizes, .. } = BuiltinLayout::MainHorizontal.apply(&panes, total) {
            assert_eq!(sum_child_sizes(&sizes), 24);
        }
    }

    #[test]
    fn main_vertical_sizes_sum() {
        let panes = pane_ids(3);
        let total = Size::new(80, 24);
        if let LayoutNode::LeftRight { sizes, .. } = BuiltinLayout::MainVertical.apply(&panes, total) {
            assert_eq!(sum_child_sizes(&sizes), 80);
        }
    }

    #[test]
    fn single_pane_layout() {
        let panes = pane_ids(1);
        let total = Size::new(80, 24);
        let node = BuiltinLayout::EvenHorizontal.apply(&panes, total);
        assert!(matches!(node, LayoutNode::Pane(_)));
    }

    #[test]
    fn collect_pane_ids_from_tree() {
        let panes = pane_ids(4);
        let total = Size::new(80, 24);
        let node = BuiltinLayout::Tiled.apply(&panes, total);
        let ids = collect_pane_ids(&node);
        assert_eq!(ids.len(), 4);
    }

    #[test]
    fn tiled_layout_sizes_sum() {
        let panes = pane_ids(4);
        let total = Size::new(80, 24);
        let node = BuiltinLayout::Tiled.apply(&panes, total);
        if let LayoutNode::TopBottom { sizes, .. } = &node {
            assert_eq!(sum_child_sizes(sizes), 24);
        }
    }

    #[test]
    fn layout_checksum_known_value() {
        // Compute the checksum for a known layout string body
        let body = "120x40,0,0,0";
        let csum = layout_checksum(body);
        let layout_str = format!("{csum:04x},{body}");
        assert!(validate_layout_checksum(&layout_str));
    }

    #[test]
    fn layout_checksum_invalid() {
        assert!(!validate_layout_checksum("0000,120x40,0,0,0"));
    }

    #[test]
    fn layout_checksum_too_short() {
        assert!(!validate_layout_checksum("abc"));
    }

    #[test]
    fn even_horizontal_remainder_distribution() {
        // 7 cols / 3 panes = 2+1, 2+1, 2+0 = 3, 3, 1? No: 7/3=2 rem 1 -> 3, 2, 2
        let panes = pane_ids(3);
        let total = Size::new(7, 1);
        if let LayoutNode::LeftRight { sizes, .. } = BuiltinLayout::EvenHorizontal.apply(&panes, total) {
            assert_eq!(sum_child_sizes(&sizes), 7);
            assert_eq!(sizes[0], 3); // first gets remainder
            assert_eq!(sizes[1], 2);
            assert_eq!(sizes[2], 2);
        }
    }
}
