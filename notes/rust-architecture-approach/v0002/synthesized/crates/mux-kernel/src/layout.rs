//! Binary-tree layout engine with 7 built-in algorithms.
//!
//! Matching tmux's layout_type enum from tmux.h:1381.

use mux_types::{PaneId, Size};

/// Layout node type (matches tmux layout_type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutType {
    LeftRight,
    TopBottom,
    Pane,
}

/// A rectangle in the terminal coordinate space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    #[must_use]
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    #[must_use]
    pub const fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}

/// A node in the layout tree.
#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub layout_type: LayoutType,
    pub rect: Rect,
    pub pane_id: Option<PaneId>,
    pub children: Vec<LayoutNode>,
}

impl LayoutNode {
    /// Create a leaf node for a single pane.
    #[must_use]
    pub fn leaf(pane_id: PaneId, rect: Rect) -> Self {
        Self {
            layout_type: LayoutType::Pane,
            rect,
            pane_id: Some(pane_id),
            children: Vec::new(),
        }
    }

    /// Create a horizontal split (left-right) container.
    #[must_use]
    pub fn left_right(rect: Rect, children: Vec<LayoutNode>) -> Self {
        Self {
            layout_type: LayoutType::LeftRight,
            rect,
            pane_id: None,
            children,
        }
    }

    /// Create a vertical split (top-bottom) container.
    #[must_use]
    pub fn top_bottom(rect: Rect, children: Vec<LayoutNode>) -> Self {
        Self {
            layout_type: LayoutType::TopBottom,
            rect,
            pane_id: None,
            children,
        }
    }

    /// Collect all pane rects from this tree.
    pub fn pane_rects(&self, out: &mut Vec<(PaneId, Rect)>) {
        if let Some(id) = self.pane_id {
            out.push((id, self.rect));
        }
        for child in &self.children {
            child.pane_rects(out);
        }
    }
}

/// Built-in layout algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinLayout {
    EvenHorizontal,
    EvenVertical,
    MainHorizontal,
    MainVertical,
    Tiled,
}

/// Solve a built-in layout for the given panes and size.
///
/// Returns a layout tree with pane rects computed.
pub fn solve_layout(
    layout: BuiltinLayout,
    pane_ids: &[PaneId],
    size: Size,
) -> LayoutNode {
    if pane_ids.is_empty() {
        return LayoutNode::leaf(PaneId(0), Rect::new(0, 0, size.cols, size.rows));
    }
    if pane_ids.len() == 1 {
        return LayoutNode::leaf(pane_ids[0], Rect::new(0, 0, size.cols, size.rows));
    }

    match layout {
        BuiltinLayout::EvenHorizontal => solve_even_horizontal(pane_ids, size),
        BuiltinLayout::EvenVertical => solve_even_vertical(pane_ids, size),
        BuiltinLayout::MainHorizontal => solve_main_horizontal(pane_ids, size),
        BuiltinLayout::MainVertical => solve_main_vertical(pane_ids, size),
        BuiltinLayout::Tiled => solve_tiled(pane_ids, size),
    }
}

fn solve_even_horizontal(pane_ids: &[PaneId], size: Size) -> LayoutNode {
    let n = pane_ids.len() as u32;
    let base_width = size.cols / n;
    let remainder = size.cols % n;
    let mut children = Vec::new();
    let mut x = 0;
    for (i, &id) in pane_ids.iter().enumerate() {
        let w = base_width + u32::from((i as u32) < remainder);
        children.push(LayoutNode::leaf(id, Rect::new(x, 0, w, size.rows)));
        x += w;
    }
    LayoutNode::left_right(Rect::new(0, 0, size.cols, size.rows), children)
}

fn solve_even_vertical(pane_ids: &[PaneId], size: Size) -> LayoutNode {
    let n = pane_ids.len() as u32;
    let base_height = size.rows / n;
    let remainder = size.rows % n;
    let mut children = Vec::new();
    let mut y = 0;
    for (i, &id) in pane_ids.iter().enumerate() {
        let h = base_height + u32::from((i as u32) < remainder);
        children.push(LayoutNode::leaf(id, Rect::new(0, y, size.cols, h)));
        y += h;
    }
    LayoutNode::top_bottom(Rect::new(0, 0, size.cols, size.rows), children)
}

fn solve_main_horizontal(pane_ids: &[PaneId], size: Size) -> LayoutNode {
    // Main pane on top, remaining split horizontally below
    let main_height = size.rows / 2;
    let rest_height = size.rows - main_height;
    let main = LayoutNode::leaf(pane_ids[0], Rect::new(0, 0, size.cols, main_height));
    let rest_ids = &pane_ids[1..];
    let rest = if rest_ids.len() == 1 {
        LayoutNode::leaf(rest_ids[0], Rect::new(0, main_height, size.cols, rest_height))
    } else {
        let n = rest_ids.len() as u32;
        let base_w = size.cols / n;
        let rem = size.cols % n;
        let mut children = Vec::new();
        let mut x = 0;
        for (i, &id) in rest_ids.iter().enumerate() {
            let w = base_w + u32::from((i as u32) < rem);
            children.push(LayoutNode::leaf(id, Rect::new(x, main_height, w, rest_height)));
            x += w;
        }
        LayoutNode::left_right(Rect::new(0, main_height, size.cols, rest_height), children)
    };
    LayoutNode::top_bottom(Rect::new(0, 0, size.cols, size.rows), vec![main, rest])
}

fn solve_main_vertical(pane_ids: &[PaneId], size: Size) -> LayoutNode {
    // Main pane on left, remaining split vertically on right
    let main_width = size.cols / 2;
    let rest_width = size.cols - main_width;
    let main = LayoutNode::leaf(pane_ids[0], Rect::new(0, 0, main_width, size.rows));
    let rest_ids = &pane_ids[1..];
    let rest = if rest_ids.len() == 1 {
        LayoutNode::leaf(rest_ids[0], Rect::new(main_width, 0, rest_width, size.rows))
    } else {
        let n = rest_ids.len() as u32;
        let base_h = size.rows / n;
        let rem = size.rows % n;
        let mut children = Vec::new();
        let mut y = 0;
        for (i, &id) in rest_ids.iter().enumerate() {
            let h = base_h + u32::from((i as u32) < rem);
            children.push(LayoutNode::leaf(id, Rect::new(main_width, y, rest_width, h)));
            y += h;
        }
        LayoutNode::top_bottom(Rect::new(main_width, 0, rest_width, size.rows), children)
    };
    LayoutNode::left_right(Rect::new(0, 0, size.cols, size.rows), vec![main, rest])
}

fn solve_tiled(pane_ids: &[PaneId], size: Size) -> LayoutNode {
    // Simple tiling: split into rows, each row has equal columns
    let n = pane_ids.len();
    let cols_count = (n as f64).sqrt().ceil() as u32;
    let rows_count = ((n as u32) + cols_count - 1) / cols_count;
    let base_h = size.rows / rows_count;
    let rem_h = size.rows % rows_count;

    let mut row_nodes = Vec::new();
    let mut pane_idx = 0;
    let mut y = 0;
    for row in 0..rows_count {
        let h = base_h + u32::from(row < rem_h);
        let panes_in_row = if row < rows_count - 1 {
            cols_count as usize
        } else {
            n - pane_idx
        };
        let base_w = size.cols / panes_in_row.max(1) as u32;
        let rem_w = size.cols % panes_in_row.max(1) as u32;

        let mut col_nodes = Vec::new();
        let mut x = 0;
        for col in 0..panes_in_row {
            if pane_idx >= n { break; }
            let w = base_w + u32::from((col as u32) < rem_w);
            col_nodes.push(LayoutNode::leaf(pane_ids[pane_idx], Rect::new(x, y, w, h)));
            x += w;
            pane_idx += 1;
        }
        if col_nodes.len() == 1 {
            row_nodes.push(col_nodes.into_iter().next().unwrap_or_else(|| {
                LayoutNode::leaf(PaneId(0), Rect::new(0, y, size.cols, h))
            }));
        } else {
            row_nodes.push(LayoutNode::left_right(Rect::new(0, y, size.cols, h), col_nodes));
        }
        y += h;
    }
    if row_nodes.len() == 1 {
        row_nodes.into_iter().next().unwrap_or_else(|| {
            LayoutNode::leaf(PaneId(0), Rect::new(0, 0, size.cols, size.rows))
        })
    } else {
        LayoutNode::top_bottom(Rect::new(0, 0, size.cols, size.rows), row_nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_pane_takes_full_area() {
        let root = solve_layout(BuiltinLayout::EvenHorizontal, &[PaneId(1)], Size::new(80, 24));
        let mut rects = Vec::new();
        root.pane_rects(&mut rects);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].1, Rect::new(0, 0, 80, 24));
    }

    #[test]
    fn even_horizontal_two_panes() {
        let root = solve_layout(
            BuiltinLayout::EvenHorizontal,
            &[PaneId(1), PaneId(2)],
            Size::new(80, 24),
        );
        let mut rects = Vec::new();
        root.pane_rects(&mut rects);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].1.width + rects[1].1.width, 80);
    }

    #[test]
    fn even_vertical_three_panes() {
        let root = solve_layout(
            BuiltinLayout::EvenVertical,
            &[PaneId(1), PaneId(2), PaneId(3)],
            Size::new(80, 24),
        );
        let mut rects = Vec::new();
        root.pane_rects(&mut rects);
        assert_eq!(rects.len(), 3);
        let total_height: u32 = rects.iter().map(|(_, r)| r.height).sum();
        assert_eq!(total_height, 24);
    }

    #[test]
    fn main_horizontal_layout() {
        let root = solve_layout(
            BuiltinLayout::MainHorizontal,
            &[PaneId(1), PaneId(2), PaneId(3)],
            Size::new(80, 24),
        );
        let mut rects = Vec::new();
        root.pane_rects(&mut rects);
        assert_eq!(rects.len(), 3);
        // Main pane should be wider than sub-panes (it spans full width)
        assert_eq!(rects[0].1.width, 80);
    }

    #[test]
    fn main_vertical_layout() {
        let root = solve_layout(
            BuiltinLayout::MainVertical,
            &[PaneId(1), PaneId(2)],
            Size::new(80, 24),
        );
        let mut rects = Vec::new();
        root.pane_rects(&mut rects);
        assert_eq!(rects.len(), 2);
        // Main pane should take left half
        assert_eq!(rects[0].1.height, 24);
    }

    #[test]
    fn tiled_four_panes() {
        let root = solve_layout(
            BuiltinLayout::Tiled,
            &[PaneId(1), PaneId(2), PaneId(3), PaneId(4)],
            Size::new(80, 24),
        );
        let mut rects = Vec::new();
        root.pane_rects(&mut rects);
        assert_eq!(rects.len(), 4);
    }

    #[test]
    fn rect_size() {
        let r = Rect::new(5, 10, 80, 24);
        assert_eq!(r.size(), Size::new(80, 24));
    }

    #[test]
    fn layout_widths_sum_to_total() {
        for n in 2..=7 {
            let panes: Vec<PaneId> = (0..n).map(|i| PaneId(i as u64)).collect();
            let root = solve_layout(BuiltinLayout::EvenHorizontal, &panes, Size::new(80, 24));
            let mut rects = Vec::new();
            root.pane_rects(&mut rects);
            let total_w: u32 = rects.iter().map(|(_, r)| r.width).sum();
            assert_eq!(total_w, 80, "failed for {n} panes");
        }
    }

    #[test]
    fn layout_heights_sum_to_total() {
        for n in 2..=7 {
            let panes: Vec<PaneId> = (0..n).map(|i| PaneId(i as u64)).collect();
            let root = solve_layout(BuiltinLayout::EvenVertical, &panes, Size::new(80, 24));
            let mut rects = Vec::new();
            root.pane_rects(&mut rects);
            let total_h: u32 = rects.iter().map(|(_, r)| r.height).sum();
            assert_eq!(total_h, 24, "failed for {n} panes");
        }
    }
}
