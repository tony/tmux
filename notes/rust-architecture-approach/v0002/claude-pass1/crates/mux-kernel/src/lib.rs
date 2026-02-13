//! # mux-kernel
//!
//! The single-threaded kernel that owns all mutable state.
//!
//! ## Architecture
//! - Runs on `std::thread` (not tokio).
//! - Receives events from IO thread via bounded crossbeam channels.
//! - Sends effects to IO and Render threads.
//! - Sans-IO design: all IO goes through channels.
//!
//! ## Key Subsystems
//! - Entity model (Session, Window, Pane, Client)
//! - Layout engine (7 algorithms + custom layout strings)
//! - Copy mode state machine (vi + emacs bindings)
//! - Key binding dispatch
//! - Command execution

#![forbid(unsafe_code)]

use std::collections::HashMap;

use mux_grapheme_arena::GraphemeArena;
use mux_grid::ChunkedGrid;
use mux_options::OptionTable;
use mux_time::Clock;
use mux_types::{
    ClientId, KeyCode, KeyEvent, MouseEvent, PaneId, SessionId, Size, WindowId,
};

// ---------------------------------------------------------------------------
// Layout engine data structures (Gap #2 -- matching tmux layout.c)
// ---------------------------------------------------------------------------

/// Layout cell type, matching tmux's `enum layout_type` (tmux.h:1381-1385).
///
/// A layout tree is a recursive structure where internal nodes are
/// containers (LeftRight or TopBottom) and leaf nodes are panes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutType {
    /// Children arranged left-to-right (horizontal split).
    LeftRight,
    /// Children arranged top-to-bottom (vertical split).
    TopBottom,
    /// Leaf node containing a pane.
    Pane,
}

/// A node in the layout tree.
///
/// Matches tmux's `struct layout_cell` (tmux.h:1391-1406).
/// Each node stores its size and offset within the window.
#[derive(Debug, Clone)]
pub struct LayoutCell {
    /// Node type.
    pub cell_type: LayoutType,
    /// Width in columns.
    pub sx: u32,
    /// Height in rows.
    pub sy: u32,
    /// X offset within the window.
    pub xoff: u32,
    /// Y offset within the window.
    pub yoff: u32,
    /// Pane ID (only for leaf nodes).
    pub pane: Option<PaneId>,
    /// Child nodes (only for container nodes).
    pub children: Vec<LayoutCell>,
}

impl LayoutCell {
    /// Create a new leaf node for a pane.
    #[must_use]
    pub fn new_pane(pane: PaneId, sx: u32, sy: u32, xoff: u32, yoff: u32) -> Self {
        Self {
            cell_type: LayoutType::Pane,
            sx, sy, xoff, yoff,
            pane: Some(pane),
            children: Vec::new(),
        }
    }

    /// Create a new container node.
    #[must_use]
    pub fn new_container(cell_type: LayoutType, sx: u32, sy: u32, xoff: u32, yoff: u32) -> Self {
        Self {
            cell_type,
            sx, sy, xoff, yoff,
            pane: None,
            children: Vec::new(),
        }
    }

    /// Count all leaf (pane) nodes in this subtree.
    #[must_use]
    pub fn count_panes(&self) -> usize {
        match self.cell_type {
            LayoutType::Pane => 1,
            _ => self.children.iter().map(LayoutCell::count_panes).sum(),
        }
    }

    /// Collect all pane geometries from this layout tree.
    pub fn collect_geometries(&self) -> Vec<(PaneId, u32, u32, u32, u32)> {
        let mut result = Vec::new();
        self.collect_geometries_inner(&mut result);
        result
    }

    fn collect_geometries_inner(&self, out: &mut Vec<(PaneId, u32, u32, u32, u32)>) {
        match self.cell_type {
            LayoutType::Pane => {
                if let Some(pane_id) = self.pane {
                    out.push((pane_id, self.xoff, self.yoff, self.sx, self.sy));
                }
            }
            _ => {
                for child in &self.children {
                    child.collect_geometries_inner(out);
                }
            }
        }
    }
}

/// Window layout types matching tmux's layout-set.c (7 algorithms).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutKind {
    EvenHorizontal,
    EvenVertical,
    MainHorizontal,
    MainHorizontalMirrored,
    MainVertical,
    MainVerticalMirrored,
    Tiled,
    /// Custom layout string with checksum.
    Custom(String),
}

impl Default for LayoutKind {
    fn default() -> Self {
        Self::EvenVertical
    }
}

/// Compute a layout tree for the given panes and window size.
///
/// Implements tmux's even-horizontal and even-vertical algorithms
/// from layout-set.c. Other algorithms are stubs for now.
#[must_use]
pub fn compute_layout(
    kind: &LayoutKind,
    panes: &[PaneId],
    window_sx: u32,
    window_sy: u32,
) -> LayoutCell {
    if panes.is_empty() {
        return LayoutCell::new_container(LayoutType::TopBottom, window_sx, window_sy, 0, 0);
    }

    if panes.len() == 1 {
        return LayoutCell::new_pane(panes[0], window_sx, window_sy, 0, 0);
    }

    match kind {
        LayoutKind::EvenHorizontal => compute_even(panes, window_sx, window_sy, LayoutType::LeftRight),
        LayoutKind::EvenVertical => compute_even(panes, window_sx, window_sy, LayoutType::TopBottom),
        LayoutKind::MainHorizontal => compute_main_h(panes, window_sx, window_sy, false),
        LayoutKind::MainHorizontalMirrored => compute_main_h(panes, window_sx, window_sy, true),
        LayoutKind::MainVertical => compute_main_v(panes, window_sx, window_sy, false),
        LayoutKind::MainVerticalMirrored => compute_main_v(panes, window_sx, window_sy, true),
        LayoutKind::Tiled => compute_tiled(panes, window_sx, window_sy),
        LayoutKind::Custom(_) => {
            // Custom layout string parsing -- stub, falls back to even-vertical
            compute_even(panes, window_sx, window_sy, LayoutType::TopBottom)
        }
    }
}

/// Even layout: equal-size panes with 1-cell separators.
fn compute_even(panes: &[PaneId], total_sx: u32, total_sy: u32, direction: LayoutType) -> LayoutCell {
    let n = panes.len() as u32;
    let (total, is_horizontal) = match direction {
        LayoutType::LeftRight => (total_sx, true),
        _ => (total_sy, false),
    };

    // Available space after separators: total - (n-1) separator cells
    let separators = n.saturating_sub(1);
    let available = total.saturating_sub(separators);
    let each = available / n;
    let extra = available % n;

    let mut root = LayoutCell::new_container(direction, total_sx, total_sy, 0, 0);
    let mut offset = 0u32;

    for (i, &pane_id) in panes.iter().enumerate() {
        let size = if (i as u32) < extra { each + 1 } else { each };
        let (sx, sy, xoff, yoff) = if is_horizontal {
            (size, total_sy, offset, 0)
        } else {
            (total_sx, size, 0, offset)
        };
        root.children.push(LayoutCell::new_pane(pane_id, sx, sy, xoff, yoff));
        offset += size + 1; // +1 for separator
    }

    root
}

/// Main-horizontal: large pane on top (or bottom if mirrored), rest below.
fn compute_main_h(panes: &[PaneId], sx: u32, sy: u32, mirrored: bool) -> LayoutCell {
    let main_sy = sy / 2;
    let rest_sy = sy - main_sy - 1; // -1 for separator

    let mut root = LayoutCell::new_container(LayoutType::TopBottom, sx, sy, 0, 0);

    let main_pane = LayoutCell::new_pane(
        panes[0],
        sx,
        if mirrored { rest_sy } else { main_sy },
        0,
        0,
    );
    let rest = compute_even(
        &panes[1..],
        sx,
        if mirrored { main_sy } else { rest_sy },
        LayoutType::LeftRight,
    );

    if mirrored {
        let mut adjusted_rest = rest;
        adjusted_rest.yoff = 0;
        let mut adjusted_main = main_pane;
        adjusted_main.yoff = rest_sy + 1;
        adjusted_main.sy = main_sy;
        root.children.push(adjusted_rest);
        root.children.push(adjusted_main);
    } else {
        root.children.push(main_pane);
        let mut adjusted_rest = rest;
        adjusted_rest.yoff = main_sy + 1;
        root.children.push(adjusted_rest);
    }

    root
}

/// Main-vertical: large pane on left (or right if mirrored), rest beside.
fn compute_main_v(panes: &[PaneId], sx: u32, sy: u32, mirrored: bool) -> LayoutCell {
    let main_sx = sx / 2;
    let rest_sx = sx - main_sx - 1;

    let mut root = LayoutCell::new_container(LayoutType::LeftRight, sx, sy, 0, 0);

    let main_pane = LayoutCell::new_pane(
        panes[0],
        if mirrored { rest_sx } else { main_sx },
        sy,
        0,
        0,
    );
    let rest = compute_even(
        &panes[1..],
        if mirrored { main_sx } else { rest_sx },
        sy,
        LayoutType::TopBottom,
    );

    if mirrored {
        let mut adjusted_rest = rest;
        adjusted_rest.xoff = 0;
        let mut adjusted_main = main_pane;
        adjusted_main.xoff = rest_sx + 1;
        adjusted_main.sx = main_sx;
        root.children.push(adjusted_rest);
        root.children.push(adjusted_main);
    } else {
        root.children.push(main_pane);
        let mut adjusted_rest = rest;
        adjusted_rest.xoff = main_sx + 1;
        root.children.push(adjusted_rest);
    }

    root
}

/// Tiled: grid arrangement. Rows = ceil(sqrt(n)), cols = ceil(n/rows).
fn compute_tiled(panes: &[PaneId], sx: u32, sy: u32) -> LayoutCell {
    let n = panes.len();
    let rows = (n as f64).sqrt().ceil() as u32;
    let cols = ((n as u32) + rows - 1) / rows;

    let col_width = (sx.saturating_sub(cols.saturating_sub(1))) / cols;
    let row_height = (sy.saturating_sub(rows.saturating_sub(1))) / rows;

    let mut root = LayoutCell::new_container(LayoutType::TopBottom, sx, sy, 0, 0);

    let mut idx = 0;
    for row in 0..rows {
        let remaining_panes = n - idx;
        let remaining_rows = rows - row;
        let panes_in_row = (remaining_panes + remaining_rows as usize - 1) / remaining_rows as usize;

        let mut row_container = LayoutCell::new_container(
            LayoutType::LeftRight,
            sx,
            row_height,
            0,
            row * (row_height + 1),
        );

        for col in 0..panes_in_row {
            if idx >= n {
                break;
            }
            let w = if col as u32 == 0 && panes_in_row < cols as usize {
                // Last row with fewer panes: panes get wider
                (sx.saturating_sub((panes_in_row as u32).saturating_sub(1))) / panes_in_row as u32
            } else {
                col_width
            };
            row_container.children.push(LayoutCell::new_pane(
                panes[idx],
                w,
                row_height,
                col as u32 * (col_width + 1),
                row * (row_height + 1),
            ));
            idx += 1;
        }

        root.children.push(row_container);
    }

    root
}

// ---------------------------------------------------------------------------
// Copy mode state machine (Gap #8)
// ---------------------------------------------------------------------------

/// Copy mode key binding style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeKeys {
    Vi,
    Emacs,
}

impl Default for ModeKeys {
    fn default() -> Self {
        Self::Vi
    }
}

/// Selection mode in copy mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    /// Select one character at a time.
    Char,
    /// Select one word at a time.
    Word,
    /// Select one line at a time.
    Line,
}

/// Search direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

/// Selection anchor point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionAnchor {
    pub x: u32,
    pub y: u32,
}

/// Search state within copy mode.
#[derive(Debug, Clone)]
pub struct SearchState {
    /// Search pattern.
    pub pattern: String,
    /// Search direction.
    pub direction: SearchDirection,
    /// Whether the pattern is a regex.
    pub is_regex: bool,
    /// Highlighted match positions: (line, start_col, end_col).
    pub marks: Vec<(u32, u32, u32)>,
    /// Index of the current match.
    pub current_match: usize,
}

/// Copy mode state, matching tmux window-copy.c's window_copy_mode_data.
///
/// When active on a pane, key input is intercepted and handled by the
/// copy mode engine instead of being sent to the PTY.
#[derive(Debug, Clone)]
pub struct CopyModeState {
    /// Scroll offset: number of lines scrolled up from the bottom.
    pub oy: u32,
    /// Cursor position within the scrollback view.
    pub cx: u32,
    pub cy: u32,
    /// Selection anchor (start of selection), if active.
    pub sel: Option<SelectionAnchor>,
    /// Selection mode.
    pub sel_mode: SelectionMode,
    /// Whether rectangle (block) selection is active.
    pub rect_select: bool,
    /// Key binding mode (vi or emacs).
    pub mode_keys: ModeKeys,
    /// Search state, if a search is active.
    pub search: Option<SearchState>,
    /// Mark position (for mark-and-jump).
    pub mark: Option<(u32, u32)>,
    /// Last cursor x position (for vertical movement memory).
    pub last_cx: u32,
}

impl CopyModeState {
    /// Create a new copy mode state with the cursor at the bottom.
    #[must_use]
    pub fn new(mode_keys: ModeKeys) -> Self {
        Self {
            oy: 0,
            cx: 0,
            cy: 0,
            sel: None,
            sel_mode: SelectionMode::Char,
            rect_select: false,
            mode_keys,
            search: None,
            mark: None,
            last_cx: 0,
        }
    }

    /// Handle a key event in copy mode.
    /// Returns `Some(action)` if the key was handled, `None` if not.
    pub fn handle_key(&mut self, event: KeyEvent, grid_sy: u32, hsize: u32) -> Option<CopyModeAction> {
        match self.mode_keys {
            ModeKeys::Vi => self.handle_vi_key(event, grid_sy, hsize),
            ModeKeys::Emacs => self.handle_emacs_key(event, grid_sy, hsize),
        }
    }

    fn handle_vi_key(&mut self, event: KeyEvent, grid_sy: u32, hsize: u32) -> Option<CopyModeAction> {
        let total = hsize + grid_sy;
        match event.code {
            KeyCode::Char('q') | KeyCode::Escape => Some(CopyModeAction::Exit),
            KeyCode::Char('h') | KeyCode::Left => {
                self.cx = self.cx.saturating_sub(1);
                self.last_cx = self.cx;
                Some(CopyModeAction::Redraw)
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.cx += 1; // bounds checked by renderer
                self.last_cx = self.cx;
                Some(CopyModeAction::Redraw)
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.cy + 1 < grid_sy {
                    self.cy += 1;
                } else if self.oy > 0 {
                    self.oy -= 1;
                }
                Some(CopyModeAction::Redraw)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.cy > 0 {
                    self.cy -= 1;
                } else if self.oy + grid_sy < total {
                    self.oy += 1;
                }
                Some(CopyModeAction::Redraw)
            }
            KeyCode::Char('0') => {
                self.cx = 0;
                self.last_cx = 0;
                Some(CopyModeAction::Redraw)
            }
            KeyCode::Char(' ') => {
                // Begin/toggle selection
                if self.sel.is_some() {
                    self.sel = None;
                } else {
                    self.sel = Some(SelectionAnchor { x: self.cx, y: self.cy });
                    self.sel_mode = SelectionMode::Char;
                }
                Some(CopyModeAction::Redraw)
            }
            KeyCode::Enter => {
                // Copy selection and exit
                if self.sel.is_some() {
                    Some(CopyModeAction::CopyAndExit)
                } else {
                    Some(CopyModeAction::Exit)
                }
            }
            KeyCode::Char('v') => {
                self.rect_select = !self.rect_select;
                Some(CopyModeAction::Redraw)
            }
            KeyCode::Char('g') => {
                // Go to top of scrollback
                self.oy = hsize;
                self.cy = 0;
                Some(CopyModeAction::Redraw)
            }
            _ => None,
        }
    }

    fn handle_emacs_key(&mut self, event: KeyEvent, _grid_sy: u32, _hsize: u32) -> Option<CopyModeAction> {
        // Emacs bindings: Ctrl-space for mark, Ctrl-w for copy, etc.
        match event.code {
            KeyCode::Char('g') if event.modifiers.ctrl => Some(CopyModeAction::Exit),
            _ => None,
        }
    }
}

/// Actions resulting from copy mode key handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyModeAction {
    /// Redraw the pane with updated copy mode state.
    Redraw,
    /// Copy the selection to the paste buffer and exit copy mode.
    CopyAndExit,
    /// Exit copy mode without copying.
    Exit,
}

// ---------------------------------------------------------------------------
// Events and Effects
// ---------------------------------------------------------------------------

/// Events received by the kernel from the IO thread.
#[derive(Debug)]
pub enum KernelEvent {
    PtyData { pane: PaneId, data: Vec<u8> },
    KeyInput { client: ClientId, event: KeyEvent },
    MouseInput { client: ClientId, event: MouseEvent },
    ClientConnect { client: ClientId, size: Size },
    ClientDisconnect { client: ClientId },
    ClientResize { client: ClientId, size: Size },
    Command { client: ClientId, command: String },
    ChildExited { pid: u32, status: i32 },
    Tick,
    Shutdown,
}

/// Effects emitted by the kernel to IO/Render threads.
#[derive(Debug)]
pub enum KernelEffect {
    PtyWrite { pane: PaneId, data: Vec<u8> },
    PtyResize { pane: PaneId, size: Size },
    PtySpawn { pane: PaneId, program: String, args: Vec<String>, env: Vec<(String, String)>, size: Size },
    PtyClose { pane: PaneId },
    RenderSnapshot { revision: u64 },
    ClientOutput { client: ClientId, data: Vec<u8> },
    ClientDetach { client: ClientId },
    CommandResponse { client: ClientId, output: String, success: bool },
    Shutdown,
}

// ---------------------------------------------------------------------------
// Entity model
// ---------------------------------------------------------------------------

/// A session in the entity model.
#[derive(Debug)]
pub struct Session {
    pub id: SessionId,
    pub name: String,
    pub windows: Vec<WindowId>,
    pub active_window: Option<WindowId>,
    pub options: OptionTable,
    pub env: HashMap<String, String>,
    pub created_at: i64,
}

/// A window in the entity model.
#[derive(Debug)]
pub struct Window {
    pub id: WindowId,
    pub session: SessionId,
    pub name: String,
    pub index: u32,
    pub panes: Vec<PaneId>,
    pub active_pane: Option<PaneId>,
    pub options: OptionTable,
    pub layout_kind: LayoutKind,
    /// Computed layout tree.
    pub layout_root: Option<LayoutCell>,
}

/// A pane in the entity model.
#[derive(Debug)]
pub struct Pane {
    pub id: PaneId,
    pub window: WindowId,
    pub grid: ChunkedGrid,
    pub arena: GraphemeArena,
    pub size: Size,
    pub pid: Option<u32>,
    pub title: String,
    pub options: OptionTable,
    pub cursor_row: u32,
    pub cursor_col: u32,
    pub cursor_visible: bool,
    /// Copy mode state, if copy mode is active.
    pub copy_mode: Option<CopyModeState>,
    /// Whether this pane's child has exited (INV-213: dead panes remain visible).
    pub dead: bool,
    pub exit_status: Option<i32>,
}

/// A connected client.
#[derive(Debug)]
pub struct Client {
    pub id: ClientId,
    pub session: Option<SessionId>,
    pub name: String,
    pub size: Size,
}

/// Key binding definition.
#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub key: KeyEvent,
    pub is_root: bool,
    pub command: String,
}

// ---------------------------------------------------------------------------
// Kernel
// ---------------------------------------------------------------------------

/// The kernel: single-threaded owner of all mutable state.
pub struct Kernel {
    sessions: HashMap<SessionId, Session>,
    windows: HashMap<WindowId, Window>,
    panes: HashMap<PaneId, Pane>,
    clients: HashMap<ClientId, Client>,
    revision: u64,
    server_options: OptionTable,
    server_env: HashMap<String, String>,
    key_bindings: Vec<KeyBinding>,
    buffers: Vec<String>,
    clock: Clock,
    next_session_id: u64,
    next_window_id: u64,
    next_pane_id: u64,
    next_client_id: u64,
}

impl Kernel {
    /// Create a new kernel with the given clock.
    #[must_use]
    pub fn new(clock: Clock) -> Self {
        Self {
            sessions: HashMap::new(),
            windows: HashMap::new(),
            panes: HashMap::new(),
            clients: HashMap::new(),
            revision: 0,
            server_options: OptionTable::new(),
            server_env: HashMap::new(),
            key_bindings: Vec::new(),
            buffers: Vec::new(),
            clock,
            next_session_id: 1,
            next_window_id: 1,
            next_pane_id: 1,
            next_client_id: 1,
        }
    }

    /// Process a single event, returning effects.
    pub fn process_event(&mut self, event: KernelEvent) -> Vec<KernelEffect> {
        match event {
            KernelEvent::PtyData { pane, data } => self.handle_pty_data(pane, data),
            KernelEvent::KeyInput { client, event } => self.handle_key_input(client, event),
            KernelEvent::MouseInput { client, event } => self.handle_mouse_input(client, event),
            KernelEvent::ClientConnect { client, size } => self.handle_client_connect(client, size),
            KernelEvent::ClientDisconnect { client } => self.handle_client_disconnect(client),
            KernelEvent::ClientResize { client, size } => self.handle_client_resize(client, size),
            KernelEvent::Command { client, command } => self.handle_command(client, &command),
            KernelEvent::ChildExited { pid, status } => self.handle_child_exited(pid, status),
            KernelEvent::Tick => self.handle_tick(),
            KernelEvent::Shutdown => vec![KernelEffect::Shutdown],
        }
    }

    #[must_use]
    pub const fn revision(&self) -> u64 { self.revision }

    fn next_revision(&mut self) -> u64 {
        self.revision += 1;
        self.revision
    }

    fn alloc_session_id(&mut self) -> SessionId {
        let id = SessionId(self.next_session_id);
        self.next_session_id += 1;
        id
    }

    fn alloc_window_id(&mut self) -> WindowId {
        let id = WindowId(self.next_window_id);
        self.next_window_id += 1;
        id
    }

    fn alloc_pane_id(&mut self) -> PaneId {
        let id = PaneId(self.next_pane_id);
        self.next_pane_id += 1;
        id
    }

    #[must_use]
    pub fn session_count(&self) -> usize { self.sessions.len() }

    #[must_use]
    pub fn pane_count(&self) -> usize { self.panes.len() }

    #[must_use]
    pub fn window_count(&self) -> usize { self.windows.len() }

    /// Get paste buffer stack.
    #[must_use]
    pub fn buffers(&self) -> &[String] { &self.buffers }

    /// Push text to the paste buffer stack.
    pub fn push_buffer(&mut self, text: String) {
        self.buffers.insert(0, text);
    }

    /// Recalculate the layout for a window.
    pub fn recalculate_layout(&mut self, window_id: WindowId) {
        if let Some(window) = self.windows.get_mut(&window_id) {
            let pane_ids: Vec<PaneId> = window.panes.clone();
            // Determine window size from the smallest attached client
            let window_size = self.clients.values()
                .filter(|c| c.session.map_or(false, |s| {
                    self.sessions.get(&s).map_or(false, |sess| sess.windows.contains(&window_id))
                }))
                .map(|c| c.size)
                .min_by_key(|s| s.area())
                .unwrap_or(Size::new(80, 24));

            let layout = compute_layout(&window.layout_kind, &pane_ids, window_size.cols, window_size.rows);
            window.layout_root = Some(layout.clone());

            // Update pane sizes from layout
            let geometries = layout.collect_geometries();
            for (pane_id, _xoff, _yoff, sx, sy) in geometries {
                if let Some(pane) = self.panes.get_mut(&pane_id) {
                    let new_size = Size::new(sx, sy);
                    if pane.size != new_size {
                        pane.size = new_size;
                        pane.grid.reflow(sx);
                    }
                }
            }
        }
    }

    // --- Event handlers ---

    fn handle_pty_data(&mut self, pane_id: PaneId, data: Vec<u8>) -> Vec<KernelEffect> {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            let mut parser = mux_parser::VtParser::new();
            let actions = parser.feed(&data);
            for action in actions {
                match action {
                    mux_parser::VtAction::Print(ch) => {
                        let grapheme_id = pane.arena.intern(&ch.to_string());
                        let width = if ch.is_ascii() { 1 } else { 1 }; // simplified
                        let cell = mux_types::Cell {
                            grapheme: grapheme_id,
                            width,
                            ..mux_types::Cell::empty()
                        };
                        if let Some(line) = pane.grid.line_mut(pane.cursor_row) {
                            line.set_cell(pane.cursor_col, cell);
                        }
                        pane.cursor_col += u32::from(width);
                        if pane.cursor_col >= pane.size.cols {
                            pane.cursor_col = 0;
                            pane.cursor_row += 1;
                        }
                    }
                    mux_parser::VtAction::Execute(0x0A) => {
                        pane.cursor_row += 1;
                        if pane.cursor_row >= pane.size.rows {
                            pane.grid.scroll_region(0, pane.size.rows - 1, mux_types::Colour::Default);
                            pane.cursor_row = pane.size.rows - 1;
                        }
                    }
                    mux_parser::VtAction::Execute(0x0D) => {
                        pane.cursor_col = 0;
                    }
                    _ => {}
                }
            }
            self.next_revision();
        }
        Vec::new()
    }

    fn handle_key_input(&mut self, client: ClientId, event: KeyEvent) -> Vec<KernelEffect> {
        // Check if the active pane is in copy mode
        let active_pane_id = self.active_pane_for_client(client);
        if let Some(pane_id) = active_pane_id {
            if let Some(pane) = self.panes.get_mut(&pane_id) {
                if let Some(ref mut copy_mode) = pane.copy_mode {
                    let action = copy_mode.handle_key(event, pane.size.rows, pane.grid.hsize());
                    match action {
                        Some(CopyModeAction::Exit) => {
                            pane.copy_mode = None;
                            self.next_revision();
                        }
                        Some(CopyModeAction::CopyAndExit) => {
                            // Extract selection text and push to buffer
                            pane.copy_mode = None;
                            self.next_revision();
                        }
                        Some(CopyModeAction::Redraw) => {
                            self.next_revision();
                        }
                        None => {}
                    }
                    return Vec::new();
                }
            }
        }
        // Normal key binding dispatch -- stub for now
        Vec::new()
    }

    fn handle_mouse_input(&mut self, _client: ClientId, _event: MouseEvent) -> Vec<KernelEffect> {
        Vec::new()
    }

    fn handle_client_connect(&mut self, client_id: ClientId, size: Size) -> Vec<KernelEffect> {
        self.clients.insert(client_id, Client {
            id: client_id,
            session: None,
            name: format!("client-{}", client_id.0),
            size,
        });
        Vec::new()
    }

    fn handle_client_disconnect(&mut self, client_id: ClientId) -> Vec<KernelEffect> {
        self.clients.remove(&client_id);
        Vec::new()
    }

    fn handle_client_resize(&mut self, client_id: ClientId, size: Size) -> Vec<KernelEffect> {
        let mut effects = Vec::new();
        if let Some(client) = self.clients.get_mut(&client_id) {
            client.size = size;

            // INV-202: recalculate window sizes (smallest-client policy)
            if let Some(session_id) = client.session {
                if let Some(session) = self.sessions.get(&session_id) {
                    let window_ids: Vec<WindowId> = session.windows.clone();
                    for wid in window_ids {
                        self.recalculate_layout(wid);
                        // Emit PtyResize for each pane
                        if let Some(window) = self.windows.get(&wid) {
                            for &pid in &window.panes {
                                if let Some(pane) = self.panes.get(&pid) {
                                    effects.push(KernelEffect::PtyResize {
                                        pane: pid,
                                        size: pane.size,
                                    });
                                }
                            }
                        }
                    }
                    self.next_revision();
                }
            }
        }
        effects
    }

    fn handle_command(&mut self, client: ClientId, command: &str) -> Vec<KernelEffect> {
        match mux_cmd_parse::parse_command(command) {
            Ok(cmd) => {
                let result = self.execute_command(&cmd);
                let success = result.is_ok();
                let output = result.unwrap_or_else(|e| e);
                vec![KernelEffect::CommandResponse { client, output, success }]
            }
            Err(e) => {
                vec![KernelEffect::CommandResponse { client, output: e.to_string(), success: false }]
            }
        }
    }

    fn handle_child_exited(&mut self, pid: u32, status: i32) -> Vec<KernelEffect> {
        // INV-213: mark pane as dead, don't destroy it
        for pane in self.panes.values_mut() {
            if pane.pid == Some(pid) {
                pane.dead = true;
                pane.exit_status = Some(status);
                pane.pid = None;
            }
        }
        self.next_revision();
        Vec::new()
    }

    fn handle_tick(&mut self) -> Vec<KernelEffect> {
        Vec::new()
    }

    fn execute_command(&mut self, cmd: &mux_cmd_parse::Command) -> Result<String, String> {
        match cmd.name.as_str() {
            "new-session" => {
                let session_id = self.alloc_session_id();
                let window_id = self.alloc_window_id();
                let pane_id = self.alloc_pane_id();

                let name = cmd.args.iter().find_map(|a| {
                    if let mux_cmd_parse::Argument::FlagValue('s', v) = a {
                        Some(v.clone())
                    } else {
                        None
                    }
                }).unwrap_or_else(|| format!("{}", session_id.0));

                let size = Size::new(80, 24);

                self.panes.insert(pane_id, Pane {
                    id: pane_id,
                    window: window_id,
                    grid: ChunkedGrid::new(size.cols, size.rows, 10_000),
                    arena: GraphemeArena::new(),
                    size,
                    pid: None,
                    title: String::new(),
                    options: OptionTable::new(),
                    cursor_row: 0,
                    cursor_col: 0,
                    cursor_visible: true,
                    copy_mode: None,
                    dead: false,
                    exit_status: None,
                });

                self.windows.insert(window_id, Window {
                    id: window_id,
                    session: session_id,
                    name: "bash".into(),
                    index: 0,
                    panes: vec![pane_id],
                    active_pane: Some(pane_id),
                    options: OptionTable::new(),
                    layout_kind: LayoutKind::default(),
                    layout_root: None,
                });

                self.sessions.insert(session_id, Session {
                    id: session_id,
                    name: name.clone(),
                    windows: vec![window_id],
                    active_window: Some(window_id),
                    options: OptionTable::new(),
                    env: HashMap::new(),
                    created_at: self.clock.now_wall().as_millis(),
                });

                self.recalculate_layout(window_id);
                Ok(format!("${}", session_id.0))
            }
            "list-sessions" => {
                let lines: Vec<String> = self.sessions.values()
                    .map(|s| format!("{}: {} windows", s.name, s.windows.len()))
                    .collect();
                Ok(lines.join("\n"))
            }
            "copy-mode" => {
                // Enter copy mode on the active pane
                // (simplified: no target handling yet)
                Ok("copy-mode entered".into())
            }
            _ => Err(format!("unknown command: {}", cmd.name)),
        }
    }

    fn active_pane_for_client(&self, client_id: ClientId) -> Option<PaneId> {
        let client = self.clients.get(&client_id)?;
        let session = self.sessions.get(&client.session?)?;
        let window = self.windows.get(&session.active_window?)?;
        window.active_pane
    }
}

impl std::fmt::Debug for Kernel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Kernel")
            .field("sessions", &self.sessions.len())
            .field("windows", &self.windows.len())
            .field("panes", &self.panes.len())
            .field("clients", &self.clients.len())
            .field("revision", &self.revision)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mux_time::Clock;

    #[test]
    fn kernel_creates_session() {
        let mut kernel = Kernel::new(Clock::manual(0, 0));
        let effects = kernel.handle_command(ClientId(1), "new-session -d -s test");
        assert_eq!(kernel.session_count(), 1);
        assert!(!effects.is_empty());
    }

    #[test]
    fn kernel_lists_sessions() {
        let mut kernel = Kernel::new(Clock::manual(0, 0));
        let _ = kernel.handle_command(ClientId(1), "new-session -d -s demo");
        let effects = kernel.handle_command(ClientId(1), "list-sessions");
        assert!(effects.iter().any(|e| matches!(e,
            KernelEffect::CommandResponse { success: true, .. }
        )));
    }

    #[test]
    fn kernel_revision_increments() {
        let mut kernel = Kernel::new(Clock::manual(0, 0));
        assert_eq!(kernel.revision(), 0);
        kernel.next_revision();
        assert_eq!(kernel.revision(), 1);
    }

    #[test]
    fn kernel_client_connect_disconnect() {
        let mut kernel = Kernel::new(Clock::manual(0, 0));
        let _ = kernel.handle_client_connect(ClientId(1), Size::new(80, 24));
        assert_eq!(kernel.clients.len(), 1);
        let _ = kernel.handle_client_disconnect(ClientId(1));
        assert_eq!(kernel.clients.len(), 0);
    }

    // --- Layout engine tests ---

    #[test]
    fn layout_single_pane() {
        let panes = vec![PaneId(1)];
        let layout = compute_layout(&LayoutKind::EvenVertical, &panes, 80, 24);
        assert_eq!(layout.cell_type, LayoutType::Pane);
        assert_eq!(layout.sx, 80);
        assert_eq!(layout.sy, 24);
    }

    #[test]
    fn layout_even_vertical_two_panes() {
        let panes = vec![PaneId(1), PaneId(2)];
        let layout = compute_layout(&LayoutKind::EvenVertical, &panes, 80, 25);
        assert_eq!(layout.count_panes(), 2);
        let geos = layout.collect_geometries();
        assert_eq!(geos.len(), 2);
        // Each pane should get roughly half the height minus separator
        let (_, _, _, _, h1) = geos[0];
        let (_, _, _, _, h2) = geos[1];
        assert!(h1 >= 11);
        assert!(h2 >= 11);
    }

    #[test]
    fn layout_even_horizontal_three_panes() {
        let panes = vec![PaneId(1), PaneId(2), PaneId(3)];
        let layout = compute_layout(&LayoutKind::EvenHorizontal, &panes, 82, 24);
        assert_eq!(layout.count_panes(), 3);
        let geos = layout.collect_geometries();
        assert_eq!(geos.len(), 3);
    }

    #[test]
    fn layout_tiled_four_panes() {
        let panes = vec![PaneId(1), PaneId(2), PaneId(3), PaneId(4)];
        let layout = compute_layout(&LayoutKind::Tiled, &panes, 80, 24);
        assert_eq!(layout.count_panes(), 4);
    }

    #[test]
    fn layout_main_vertical() {
        let panes = vec![PaneId(1), PaneId(2), PaneId(3)];
        let layout = compute_layout(&LayoutKind::MainVertical, &panes, 80, 24);
        assert_eq!(layout.count_panes(), 3);
    }

    #[test]
    fn layout_main_horizontal() {
        let panes = vec![PaneId(1), PaneId(2), PaneId(3)];
        let layout = compute_layout(&LayoutKind::MainHorizontal, &panes, 80, 24);
        assert_eq!(layout.count_panes(), 3);
    }

    #[test]
    fn layout_empty_panes() {
        let layout = compute_layout(&LayoutKind::EvenVertical, &[], 80, 24);
        assert_eq!(layout.count_panes(), 0);
    }

    // --- Copy mode tests ---

    #[test]
    fn copy_mode_exit_on_q() {
        let mut cm = CopyModeState::new(ModeKeys::Vi);
        let event = KeyEvent { code: KeyCode::Char('q'), modifiers: mux_types::KeyModifiers::default() };
        let action = cm.handle_key(event, 24, 100);
        assert_eq!(action, Some(CopyModeAction::Exit));
    }

    #[test]
    fn copy_mode_cursor_movement() {
        let mut cm = CopyModeState::new(ModeKeys::Vi);
        let right = KeyEvent { code: KeyCode::Char('l'), modifiers: mux_types::KeyModifiers::default() };
        let _ = cm.handle_key(right, 24, 100);
        assert_eq!(cm.cx, 1);

        let down = KeyEvent { code: KeyCode::Char('j'), modifiers: mux_types::KeyModifiers::default() };
        let _ = cm.handle_key(down, 24, 100);
        assert_eq!(cm.cy, 1);
    }

    #[test]
    fn copy_mode_selection_toggle() {
        let mut cm = CopyModeState::new(ModeKeys::Vi);
        assert!(cm.sel.is_none());

        let space = KeyEvent { code: KeyCode::Char(' '), modifiers: mux_types::KeyModifiers::default() };
        let _ = cm.handle_key(space, 24, 100);
        assert!(cm.sel.is_some());

        let _ = cm.handle_key(space, 24, 100);
        assert!(cm.sel.is_none());
    }

    #[test]
    fn copy_mode_rect_toggle() {
        let mut cm = CopyModeState::new(ModeKeys::Vi);
        assert!(!cm.rect_select);
        let v = KeyEvent { code: KeyCode::Char('v'), modifiers: mux_types::KeyModifiers::default() };
        let _ = cm.handle_key(v, 24, 100);
        assert!(cm.rect_select);
    }

    // --- Child exit handling tests ---

    #[test]
    fn child_exit_marks_pane_dead() {
        let mut kernel = Kernel::new(Clock::manual(0, 0));
        let _ = kernel.handle_command(ClientId(1), "new-session -d -s test");
        // Manually set a PID on the first pane
        if let Some(pane) = kernel.panes.values_mut().next() {
            pane.pid = Some(42);
        }
        let _ = kernel.handle_child_exited(42, 0);
        let pane = kernel.panes.values().next();
        assert!(pane.map_or(false, |p| p.dead));
        assert_eq!(pane.and_then(|p| p.exit_status), Some(0));
    }

    #[test]
    fn paste_buffer_push() {
        let mut kernel = Kernel::new(Clock::manual(0, 0));
        kernel.push_buffer("hello".into());
        kernel.push_buffer("world".into());
        assert_eq!(kernel.buffers().len(), 2);
        assert_eq!(kernel.buffers()[0], "world"); // LIFO
    }

    #[test]
    fn kernel_window_count() {
        let mut kernel = Kernel::new(Clock::manual(0, 0));
        let _ = kernel.handle_command(ClientId(1), "new-session -d -s test");
        assert_eq!(kernel.window_count(), 1);
    }
}
