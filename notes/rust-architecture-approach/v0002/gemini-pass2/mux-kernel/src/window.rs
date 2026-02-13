use crate::pane::Pane;
use mux_layout::LayoutTree;
use mux_types::Size;

pub struct Window {
    pub id: u32,
    pub name: String,
    pub layout: LayoutTree,
    pub panes: Vec<Pane>,
    pub size: Size,
}

impl Window {
    pub fn new(id: u32, name: &str, size: Size) -> Self {
        // Default layout?
        Self {
            id,
            name: name.to_string(),
            layout: LayoutTree::Leaf(mux_layout::LayoutCell {
                id: 1,
                rect: mux_types::Rect::new(0, 0, size.width, size.height),
            }),
            panes: Vec::new(),
            size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_creation() {
        let w = Window::new(1, "bash", Size { width: 100, height: 50 });
        assert_eq!(w.id, 1);
        assert_eq!(w.name, "bash");
        assert_eq!(w.size.width, 100);
        assert!(w.panes.is_empty());
    }

    #[test]
    fn test_window_layout_init() {
        let w = Window::new(1, "test", Size { width: 10, height: 10 });
        if let LayoutTree::Leaf(cell) = w.layout {
            assert_eq!(cell.rect.size.width, 10);
        } else {
            panic!("Expected leaf layout");
        }
    }
}
