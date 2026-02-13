use mux_protocol::{KernelEvent};
use crossbeam_channel::{Receiver};
use serde::{Deserialize, Serialize};

pub type PaneId = u32;

// --- Layout Engine (BSP) ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitType { Vertical, Horizontal }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeKind {
    Pane(PaneId),
    Split {
        dir: SplitType,
        ratio: f32, // 0.0 - 1.0 (relative split)
        left: Box<LayoutNode>,
        right: Box<LayoutNode>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutNode {
    pub id: u32,
    pub rect: Rect, // Computed absolute position
    pub kind: NodeKind,
}

impl LayoutNode {
    pub fn new_pane(id: u32, pane_id: PaneId, rect: Rect) -> Self {
        Self {
            id,
            rect,
            kind: NodeKind::Pane(pane_id),
        }
    }
    
    // Recursive resize logic (simplified)
    pub fn resize(&mut self, new_rect: Rect) {
        self.rect = new_rect;
        match &mut self.kind {
            NodeKind::Pane(_) => {}, // Leaf: accepts new size
            NodeKind::Split { dir, ratio, left, right } => {
                let (l_rect, r_rect) = match dir {
                    SplitType::Horizontal => {
                        let h = (new_rect.height as f32 * *ratio) as u16;
                        (
                            Rect { height: h, ..new_rect },
                            Rect { y: new_rect.y + h, height: new_rect.height - h, ..new_rect }
                        )
                    },
                    SplitType::Vertical => {
                        let w = (new_rect.width as f32 * *ratio) as u16;
                        (
                            Rect { width: w, ..new_rect },
                            Rect { x: new_rect.x + w, width: new_rect.width - w, ..new_rect }
                        )
                    }
                };
                left.resize(l_rect);
                right.resize(r_rect);
            }
        }
    }
}

// --- Copy Mode State Machine ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub col: u16,
    pub row: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyState {
    pub scroll_offset: usize, // Lines up from bottom
    pub cursor: Point,        // Visual cursor position
    pub selection_start: Option<Point>,
    pub search_query: Option<String>,
}

impl Default for CopyState {
    fn default() -> Self {
        Self {
            scroll_offset: 0,
            cursor: Point { col: 0, row: 0 },
            selection_start: None,
            search_query: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaneMode {
    Standard, // Terminal Input
    Copy(CopyState), // Vi/Emacs navigation
}

// --- Core Kernel ---

pub struct Session {
    pub layout_root: LayoutNode,
    pub active_pane: PaneId,
}

pub struct Kernel {
    pub session: Session,
    pub rx: Receiver<KernelEvent>,
}

impl Kernel {
    pub fn new(rx: Receiver<KernelEvent>) -> Self {
        let root = LayoutNode::new_pane(0, 1, Rect { x:0, y:0, width: 80, height: 24 });
        Self {
            session: Session { layout_root: root, active_pane: 1 },
            rx,
        }
    }

    pub fn run(&mut self) {
        while let Ok(event) = self.rx.recv() {
            match event {
                KernelEvent::Resize(w, h) => {
                    self.session.layout_root.resize(Rect { x:0, y:0, width: w, height: h });
                    // Propagate to Panes (TIOCSWINSZ)
                },
                KernelEvent::PaneExited(id, code) => {
                    println!("Pane {} exited with {}", id, code);
                    // Handle cleanup / respawn
                },
                _ => {}
            }
        }
    }
}
