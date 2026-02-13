use mux_protocol::{Cell, Color};

pub struct Surface {
    pub width: u16,
    pub height: u16,
    pub cells: Vec<Cell>,
}

impl Surface {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::default(); (width as usize) * (height as usize)],
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.cells = vec![Cell::default(); (width as usize) * (height as usize)];
    }

    pub fn set(&mut self, x: u16, y: u16, cell: Cell) {
        if x < self.width && y < self.height {
            let idx = (y as usize) * (self.width as usize) + (x as usize);
            self.cells[idx] = cell;
        }
    }
}

pub struct RenderState {
    pub cursor_pos: Option<(u16, u16)>,
    // In a real implementation, this would hold Arc<Grid> references
    // For now, we stub it to demonstrate the architecture
}

pub struct Renderer {
    pub front: Surface, // What is currently on screen
    pub back: Surface,  // What we are building
}

impl Renderer {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            front: Surface::new(width, height),
            back: Surface::new(width, height),
        }
    }

    pub fn compose(&mut self, _state: &RenderState) {
        // 1. Clear back buffer
        // 2. Iterate Panes from State
        // 3. Blit Pane grids into back buffer at correct offsets
        // (Stub for this pass)
    }

    // The Critical Path: Dirty Line Diffing
    pub fn diff(&mut self) -> String {
        let mut output = String::with_capacity(4096);
        let mut last_fg = Color(7);
        let mut last_bg = Color(0);
        let mut cursor_x = 0;
        let mut cursor_y = 0;

        // Reset cursor to home
        output.push_str("\x1b[H");

        for y in 0..self.back.height {
            let row_offset = (y as usize) * (self.back.width as usize);
            let mut dirty_run = false;
            
            for x in 0..self.back.width {
                let idx = row_offset + (x as usize);
                let new_cell = &self.back.cells[idx];
                let old_cell = &self.front.cells[idx];

                // Simplistic cell equality check
                if new_cell != old_cell {
                    if !dirty_run {
                        // Move cursor if we weren't already writing sequentially
                        if y != cursor_y || x != cursor_x {
                            output.push_str(&format!("\x1b[{};{}H", y + 1, x + 1));
                        }
                        dirty_run = true;
                    }

                    // Update styles if needed
                    if new_cell.fg != last_fg {
                        output.push_str(&format!("\x1b[38;5;{}m", new_cell.fg.0));
                        last_fg = new_cell.fg;
                    }
                    if new_cell.bg != last_bg {
                        output.push_str(&format!("\x1b[48;5;{}m", new_cell.bg.0));
                        last_bg = new_cell.bg;
                    }

                    output.push(new_cell.char);
                    cursor_x = x + 1;
                    cursor_y = y;
                } else {
                    dirty_run = false;
                }
            }
        }
        
        // Finalize: Swap buffers
        std::mem::swap(&mut self.front, &mut self.back);
        output
    }
}
