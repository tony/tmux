//! Binary-tree layout engine.
//!
//! 5 built-in layouts: even-horizontal, even-vertical, main-horizontal,
//! main-vertical, tiled. Plus custom layout string parsing.
//!
//! Reference: tmux layout.c and layout-set.c.

/// Layout orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// A node in the layout binary tree.
#[derive(Debug, Clone)]
pub enum LayoutNode {
    /// A leaf pane.
    Leaf {
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    },
    /// A split containing children.
    Split {
        orientation: Orientation,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        children: Vec<LayoutNode>,
    },
}

impl LayoutNode {
    /// Get the bounding rect of this node.
    #[must_use]
    pub fn rect(&self) -> (u16, u16, u16, u16) {
        match self {
            Self::Leaf {
                x,
                y,
                width,
                height,
            }
            | Self::Split {
                x,
                y,
                width,
                height,
                ..
            } => (*x, *y, *width, *height),
        }
    }
}

/// Built-in layout types matching tmux's layout-set.c.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinLayout {
    EvenHorizontal,
    EvenVertical,
    MainHorizontal,
    MainVertical,
    Tiled,
}

/// Compute an even-horizontal layout for `n` panes in the given dimensions.
#[must_use]
pub fn even_horizontal(n: usize, width: u16, height: u16) -> Vec<LayoutNode> {
    if n == 0 {
        return Vec::new();
    }
    let pane_width = width / n as u16;
    let remainder = width % n as u16;
    let mut nodes = Vec::with_capacity(n);
    let mut x = 0u16;

    for i in 0..n {
        let w = pane_width + if (i as u16) < remainder { 1 } else { 0 };
        nodes.push(LayoutNode::Leaf {
            x,
            y: 0,
            width: w,
            height,
        });
        x += w;
    }
    nodes
}

/// Compute an even-vertical layout for `n` panes.
#[must_use]
pub fn even_vertical(n: usize, width: u16, height: u16) -> Vec<LayoutNode> {
    if n == 0 {
        return Vec::new();
    }
    let pane_height = height / n as u16;
    let remainder = height % n as u16;
    let mut nodes = Vec::with_capacity(n);
    let mut y = 0u16;

    for i in 0..n {
        let h = pane_height + if (i as u16) < remainder { 1 } else { 0 };
        nodes.push(LayoutNode::Leaf {
            x: 0,
            y,
            width,
            height: h,
        });
        y += h;
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn even_horizontal_single() {
        let nodes = even_horizontal(1, 80, 24);
        assert_eq!(nodes.len(), 1);
        let (x, y, w, h) = nodes[0].rect();
        assert_eq!((x, y, w, h), (0, 0, 80, 24));
    }

    #[test]
    fn even_horizontal_two() {
        let nodes = even_horizontal(2, 80, 24);
        assert_eq!(nodes.len(), 2);
        let (_, _, w1, _) = nodes[0].rect();
        let (_, _, w2, _) = nodes[1].rect();
        assert_eq!(w1 + w2, 80);
    }

    #[test]
    fn even_horizontal_remainder() {
        let nodes = even_horizontal(3, 80, 24);
        let total_width: u16 = nodes.iter().map(|n| n.rect().2).sum();
        assert_eq!(total_width, 80);
    }

    #[test]
    fn even_vertical_single() {
        let nodes = even_vertical(1, 80, 24);
        assert_eq!(nodes.len(), 1);
        let (x, y, w, h) = nodes[0].rect();
        assert_eq!((x, y, w, h), (0, 0, 80, 24));
    }

    #[test]
    fn even_vertical_three() {
        let nodes = even_vertical(3, 80, 24);
        let total_height: u16 = nodes.iter().map(|n| n.rect().3).sum();
        assert_eq!(total_height, 24);
    }

    #[test]
    fn even_horizontal_empty() {
        let nodes = even_horizontal(0, 80, 24);
        assert!(nodes.is_empty());
    }

    #[test]
    fn layout_node_rect() {
        let leaf = LayoutNode::Leaf {
            x: 5,
            y: 3,
            width: 40,
            height: 12,
        };
        assert_eq!(leaf.rect(), (5, 3, 40, 12));
    }

    #[test]
    fn builtin_layout_equality() {
        assert_eq!(BuiltinLayout::Tiled, BuiltinLayout::Tiled);
        assert_ne!(BuiltinLayout::Tiled, BuiltinLayout::EvenHorizontal);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Layout dimension invariant: sum of child widths must equal parent width.
        #[test]
        fn even_horizontal_width_sum(
            n in 1usize..20,
            width in 20u16..500,
            height in 10u16..100
        ) {
            let nodes = even_horizontal(n, width, height);
            let total: u16 = nodes.iter().map(|n| n.rect().2).sum();
            prop_assert_eq!(total, width);
        }

        /// Layout dimension invariant: sum of child heights must equal parent height.
        #[test]
        fn even_vertical_height_sum(
            n in 1usize..20,
            width in 20u16..500,
            height in 10u16..100
        ) {
            let nodes = even_vertical(n, width, height);
            let total: u16 = nodes.iter().map(|n| n.rect().3).sum();
            prop_assert_eq!(total, height);
        }
    }
}
