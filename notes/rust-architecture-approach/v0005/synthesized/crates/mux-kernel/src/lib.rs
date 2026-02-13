//! Sans-IO kernel: entities, layout, copy mode, event-to-effect processing.
//!
//! The kernel owns ALL mutable multiplexer state. It runs on a single
//! dedicated thread and processes KernelEvents into KernelEffects.
//! The kernel NEVER performs I/O directly.

#![forbid(unsafe_code)]

pub mod copy_mode;
pub mod event;
pub mod layout;
pub mod pane;
pub mod session;
pub mod window;

pub use event::{KernelEffect, KernelEvent};

use std::collections::HashMap;
use mux_grapheme_arena::GraphemeArena;
use mux_grid::ChunkedGrid;
use mux_options::{OptionTable, builtin_defaults};
use mux_parser::VtParser;
use mux_time::Clock;
use mux_types::id::{IdGenerator, SessionId, WindowId, PaneId, ClientId};
use mux_types::geometry::Size;
use thiserror::Error;

/// Kernel errors.
#[derive(Debug, Error)]
pub enum KernelError {
    #[error("session not found: {0:?}")]
    SessionNotFound(SessionId),
    #[error("window not found: {0:?}")]
    WindowNotFound(WindowId),
    #[error("pane not found: {0:?}")]
    PaneNotFound(PaneId),
    #[error("duplicate session name: {0}")]
    DuplicateSessionName(String),
    #[error("no sessions")]
    NoSessions,
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

/// A pane entity.
#[derive(Debug)]
pub struct PaneState {
    pub id: PaneId,
    pub grid: ChunkedGrid,
    pub parser: VtParser,
    pub arena: GraphemeArena,
    pub exited: bool,
    pub exit_code: Option<i32>,
    pub size: Size,
    pub options: OptionTable,
}

impl PaneState {
    fn new(id: PaneId, size: Size) -> Self {
        Self {
            id,
            grid: ChunkedGrid::new(size.cols, size.rows, 2000),
            parser: VtParser::new(),
            arena: GraphemeArena::new(),
            exited: false,
            exit_code: None,
            size,
            options: OptionTable::new(),
        }
    }

    fn process_output(&mut self, data: &[u8]) {
        let actions = self.parser.process_bytes(data);
        for action in actions {
            match action {
                mux_parser::VtAction::Print(ch) => {
                    let id = self.arena.intern(&ch.to_string());
                    let width = if ch as u32 > 0x2E80 { 2 } else { 1 };
                    self.grid.write_char(id, width);
                }
                mux_parser::VtAction::Execute(b) => match b {
                    b'\n' | 0x0B | 0x0C => self.grid.line_feed(),
                    b'\r' => self.grid.carriage_return(),
                    0x08 => {
                        let (x, y) = self.grid.cursor();
                        if x > 0 { self.grid.set_cursor(x - 1, y); }
                    }
                    _ => {}
                },
                mux_parser::VtAction::CsiDispatch(params) => {
                    self.handle_csi(&params);
                }
                _ => {}
            }
        }
    }

    fn handle_csi(&mut self, params: &mux_parser::CsiParams) {
        match params.final_byte {
            b'A' => { // CUU - Cursor Up
                let n = params.get(0, 1);
                let (x, y) = self.grid.cursor();
                self.grid.set_cursor(x, y.saturating_sub(n));
            }
            b'B' => { // CUD - Cursor Down
                let n = params.get(0, 1);
                let (x, y) = self.grid.cursor();
                self.grid.set_cursor(x, y.saturating_add(n));
            }
            b'C' => { // CUF - Cursor Forward
                let n = params.get(0, 1);
                let (x, y) = self.grid.cursor();
                self.grid.set_cursor(x.saturating_add(n), y);
            }
            b'D' => { // CUB - Cursor Backward
                let n = params.get(0, 1);
                let (x, y) = self.grid.cursor();
                self.grid.set_cursor(x.saturating_sub(n), y);
            }
            b'H' | b'f' => { // CUP - Cursor Position
                let row = params.get(0, 1).saturating_sub(1);
                let col = params.get(1, 1).saturating_sub(1);
                self.grid.set_cursor(col, row);
            }
            b'J' => { // ED - Erase Display
                match params.get(0, 0) {
                    0 => self.grid.erase_display_below(),
                    2 => self.grid.erase_display_all(),
                    _ => {}
                }
            }
            b'K' => { // EL - Erase Line
                match params.get(0, 0) {
                    0 => self.grid.erase_line_right(),
                    _ => {}
                }
            }
            b'm' => { // SGR - Select Graphic Rendition
                self.handle_sgr(params);
            }
            b'r' => { // DECSTBM - Set Scroll Region
                let top = params.get(0, 1).saturating_sub(1);
                let bottom = params.get(1, self.size.rows).saturating_sub(1);
                self.grid.set_scroll_region(top, bottom);
            }
            _ => {}
        }
    }

    fn handle_sgr(&mut self, params: &mux_parser::CsiParams) {
        use mux_types::colour::Colour;

        if params.is_empty() {
            self.grid.reset_attrs();
            return;
        }

        let mut i = 0;
        while i < params.len() {
            let p = params.get_raw(i);
            match p {
                0 => self.grid.reset_attrs(),
                1 => {
                    let (x, y) = self.grid.cursor();
                    let _ = (x, y);
                    // Would set bold - simplified for scaffold
                }
                30..=37 => {
                    let _ = Colour::from_ansi((p - 30) as u8);
                }
                _ => {}
            }
            i += 1;
        }
    }
}

/// A window entity.
#[derive(Debug)]
pub struct WindowState {
    pub id: WindowId,
    pub name: String,
    pub panes: Vec<PaneId>,
    pub active_pane_idx: usize,
    pub layout: layout::LayoutNode,
    pub options: OptionTable,
}

/// A session entity.
#[derive(Debug)]
pub struct SessionState {
    pub id: SessionId,
    pub name: String,
    pub windows: Vec<WindowId>,
    pub active_window_idx: usize,
    pub options: OptionTable,
}

/// A client entity.
#[derive(Debug)]
pub struct ClientState {
    pub id: ClientId,
    pub attached_session: Option<SessionId>,
    pub size: Size,
}

/// The main kernel state machine.
#[derive(Debug)]
pub struct Kernel {
    pub id_gen: IdGenerator,
    pub sessions: HashMap<SessionId, SessionState>,
    pub windows: HashMap<WindowId, WindowState>,
    pub panes: HashMap<PaneId, PaneState>,
    pub clients: HashMap<ClientId, ClientState>,
    pub clock: Clock,
    pub server_options: OptionTable,
}

impl Kernel {
    /// Create a new kernel with defaults.
    pub fn new(clock: Clock) -> Self {
        Self {
            id_gen: IdGenerator::new(),
            sessions: HashMap::new(),
            windows: HashMap::new(),
            panes: HashMap::new(),
            clients: HashMap::new(),
            clock,
            server_options: builtin_defaults(),
        }
    }

    /// Process a kernel event and return effects.
    pub fn process_event(&mut self, event: KernelEvent) -> Vec<KernelEffect> {
        match event {
            KernelEvent::CreateSession { name, size } => self.create_session(&name, size),
            KernelEvent::CreateWindow { session_id, name } => self.create_window(session_id, &name),
            KernelEvent::SplitPane { window_id, size, vertical } => self.split_pane(window_id, size, vertical),
            KernelEvent::PtyOutput { pane_id, data } => self.pty_output(pane_id, &data),
            KernelEvent::PaneExited { pane_id, code } => self.pane_exited(pane_id, code),
            KernelEvent::ClientResize { client_id, size } => self.client_resize(client_id, size),
            KernelEvent::DestroySession { session_id } => self.destroy_session(session_id),
            KernelEvent::DestroyPane { pane_id } => self.destroy_pane(pane_id),
            KernelEvent::SelectWindow { session_id, index } => self.select_window(session_id, index),
            KernelEvent::SelectPane { window_id, index } => self.select_pane(window_id, index),
            KernelEvent::RenameSession { session_id, name } => self.rename_session(session_id, &name),
        }
    }

    fn create_session(&mut self, name: &str, size: Size) -> Vec<KernelEffect> {
        // Check for duplicate name
        if self.sessions.values().any(|s| s.name == name) {
            return vec![KernelEffect::Error(KernelError::DuplicateSessionName(name.to_owned()))];
        }

        let session_id = self.id_gen.next_session();
        let window_id = self.id_gen.next_window();
        let pane_id = self.id_gen.next_pane();

        let pane = PaneState::new(pane_id, size);
        self.panes.insert(pane_id, pane);

        let window = WindowState {
            id: window_id,
            name: name.to_owned(),
            panes: vec![pane_id],
            active_pane_idx: 0,
            layout: layout::LayoutNode::Leaf {
                pane_id,
                x: 0, y: 0,
                cols: size.cols, rows: size.rows,
            },
            options: OptionTable::new(),
        };
        self.windows.insert(window_id, window);

        let session = SessionState {
            id: session_id,
            name: name.to_owned(),
            windows: vec![window_id],
            active_window_idx: 0,
            options: OptionTable::new(),
        };
        self.sessions.insert(session_id, session);

        vec![
            KernelEffect::SessionCreated { session_id, window_id, pane_id },
            KernelEffect::SpawnChild { pane_id, size },
        ]
    }

    fn create_window(&mut self, session_id: SessionId, name: &str) -> Vec<KernelEffect> {
        let session = match self.sessions.get_mut(&session_id) {
            Some(s) => s,
            None => return vec![KernelEffect::Error(KernelError::SessionNotFound(session_id))],
        };

        let size = Size::new(80, 24); // Default
        let window_id = self.id_gen.next_window();
        let pane_id = self.id_gen.next_pane();

        let pane = PaneState::new(pane_id, size);
        self.panes.insert(pane_id, pane);

        let window = WindowState {
            id: window_id,
            name: name.to_owned(),
            panes: vec![pane_id],
            active_pane_idx: 0,
            layout: layout::LayoutNode::Leaf {
                pane_id,
                x: 0, y: 0,
                cols: size.cols, rows: size.rows,
            },
            options: OptionTable::new(),
        };
        self.windows.insert(window_id, window);
        session.windows.push(window_id);

        vec![
            KernelEffect::WindowCreated { session_id, window_id, pane_id },
            KernelEffect::SpawnChild { pane_id, size },
        ]
    }

    fn split_pane(&mut self, window_id: WindowId, size: Size, _vertical: bool) -> Vec<KernelEffect> {
        let window = match self.windows.get_mut(&window_id) {
            Some(w) => w,
            None => return vec![KernelEffect::Error(KernelError::WindowNotFound(window_id))],
        };

        let pane_id = self.id_gen.next_pane();
        let pane = PaneState::new(pane_id, size);
        self.panes.insert(pane_id, pane);
        window.panes.push(pane_id);

        vec![
            KernelEffect::PaneCreated { window_id, pane_id },
            KernelEffect::SpawnChild { pane_id, size },
        ]
    }

    fn pty_output(&mut self, pane_id: PaneId, data: &[u8]) -> Vec<KernelEffect> {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            pane.process_output(data);
            vec![KernelEffect::Render]
        } else {
            vec![]
        }
    }

    fn pane_exited(&mut self, pane_id: PaneId, code: i32) -> Vec<KernelEffect> {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            pane.exited = true;
            pane.exit_code = Some(code);
        }
        vec![KernelEffect::PaneClosed { pane_id, code }]
    }

    fn client_resize(&mut self, client_id: ClientId, size: Size) -> Vec<KernelEffect> {
        if let Some(client) = self.clients.get_mut(&client_id) {
            client.size = size;
        }
        vec![KernelEffect::Render]
    }

    fn destroy_session(&mut self, session_id: SessionId) -> Vec<KernelEffect> {
        if let Some(session) = self.sessions.remove(&session_id) {
            let mut effects = Vec::new();
            for wid in &session.windows {
                if let Some(window) = self.windows.remove(wid) {
                    for pid in &window.panes {
                        self.panes.remove(pid);
                    }
                }
            }
            effects.push(KernelEffect::SessionDestroyed { session_id });
            effects
        } else {
            vec![KernelEffect::Error(KernelError::SessionNotFound(session_id))]
        }
    }

    fn destroy_pane(&mut self, pane_id: PaneId) -> Vec<KernelEffect> {
        self.panes.remove(&pane_id);
        // Remove from windows
        for window in self.windows.values_mut() {
            window.panes.retain(|&p| p != pane_id);
        }
        vec![KernelEffect::PaneClosed { pane_id, code: 0 }]
    }

    fn select_window(&mut self, session_id: SessionId, index: usize) -> Vec<KernelEffect> {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            if index < session.windows.len() {
                session.active_window_idx = index;
            }
        }
        vec![KernelEffect::Render]
    }

    fn select_pane(&mut self, window_id: WindowId, index: usize) -> Vec<KernelEffect> {
        if let Some(window) = self.windows.get_mut(&window_id) {
            if index < window.panes.len() {
                window.active_pane_idx = index;
            }
        }
        vec![KernelEffect::Render]
    }

    fn rename_session(&mut self, session_id: SessionId, name: &str) -> Vec<KernelEffect> {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            session.name = name.to_owned();
            vec![]
        } else {
            vec![KernelEffect::Error(KernelError::SessionNotFound(session_id))]
        }
    }

    /// Get a reference to a pane's grid.
    pub fn pane_grid(&self, pane_id: PaneId) -> Option<&ChunkedGrid> {
        self.panes.get(&pane_id).map(|p| &p.grid)
    }

    /// Get a mutable reference to a pane's grid.
    pub fn pane_grid_mut(&mut self, pane_id: PaneId) -> Option<&mut ChunkedGrid> {
        self.panes.get_mut(&pane_id).map(|p| &mut p.grid)
    }

    /// Number of sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Number of total panes.
    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    /// Get session by name.
    pub fn session_by_name(&self, name: &str) -> Option<&SessionState> {
        self.sessions.values().find(|s| s.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_kernel() -> Kernel {
        Kernel::new(Clock::manual())
    }

    #[test]
    fn create_session() {
        let mut k = test_kernel();
        let effects = k.process_event(KernelEvent::CreateSession {
            name: "test".into(),
            size: Size::new(80, 24),
        });
        assert!(effects.iter().any(|e| matches!(e, KernelEffect::SessionCreated { .. })));
        assert_eq!(k.session_count(), 1);
    }

    #[test]
    fn duplicate_session_name_error() {
        let mut k = test_kernel();
        k.process_event(KernelEvent::CreateSession { name: "dev".into(), size: Size::new(80, 24) });
        let effects = k.process_event(KernelEvent::CreateSession { name: "dev".into(), size: Size::new(80, 24) });
        assert!(effects.iter().any(|e| matches!(e, KernelEffect::Error(_))));
    }

    #[test]
    fn create_window() {
        let mut k = test_kernel();
        let effects = k.process_event(KernelEvent::CreateSession { name: "s".into(), size: Size::new(80, 24) });
        let session_id = match &effects[0] {
            KernelEffect::SessionCreated { session_id, .. } => *session_id,
            _ => SessionId(0),
        };
        let effects = k.process_event(KernelEvent::CreateWindow { session_id, name: "w2".into() });
        assert!(effects.iter().any(|e| matches!(e, KernelEffect::WindowCreated { .. })));
    }

    #[test]
    fn split_pane() {
        let mut k = test_kernel();
        let effects = k.process_event(KernelEvent::CreateSession { name: "s".into(), size: Size::new(80, 24) });
        let window_id = match &effects[0] {
            KernelEffect::SessionCreated { window_id, .. } => *window_id,
            _ => WindowId(0),
        };
        let effects = k.process_event(KernelEvent::SplitPane { window_id, size: Size::new(40, 24), vertical: true });
        assert!(effects.iter().any(|e| matches!(e, KernelEffect::PaneCreated { .. })));
    }

    #[test]
    fn pty_output() {
        let mut k = test_kernel();
        let effects = k.process_event(KernelEvent::CreateSession { name: "s".into(), size: Size::new(80, 24) });
        let pane_id = match &effects[0] {
            KernelEffect::SessionCreated { pane_id, .. } => *pane_id,
            _ => PaneId(0),
        };
        let effects = k.process_event(KernelEvent::PtyOutput { pane_id, data: b"hello".to_vec() });
        assert!(effects.iter().any(|e| matches!(e, KernelEffect::Render)));
    }

    #[test]
    fn pane_exited() {
        let mut k = test_kernel();
        let effects = k.process_event(KernelEvent::CreateSession { name: "s".into(), size: Size::new(80, 24) });
        let pane_id = match &effects[0] {
            KernelEffect::SessionCreated { pane_id, .. } => *pane_id,
            _ => PaneId(0),
        };
        let effects = k.process_event(KernelEvent::PaneExited { pane_id, code: 0 });
        assert!(effects.iter().any(|e| matches!(e, KernelEffect::PaneClosed { .. })));
    }

    #[test]
    fn destroy_session() {
        let mut k = test_kernel();
        let effects = k.process_event(KernelEvent::CreateSession { name: "s".into(), size: Size::new(80, 24) });
        let session_id = match &effects[0] {
            KernelEffect::SessionCreated { session_id, .. } => *session_id,
            _ => SessionId(0),
        };
        k.process_event(KernelEvent::DestroySession { session_id });
        assert_eq!(k.session_count(), 0);
    }

    #[test]
    fn select_window() {
        let mut k = test_kernel();
        let effects = k.process_event(KernelEvent::CreateSession { name: "s".into(), size: Size::new(80, 24) });
        let session_id = match &effects[0] {
            KernelEffect::SessionCreated { session_id, .. } => *session_id,
            _ => SessionId(0),
        };
        k.process_event(KernelEvent::CreateWindow { session_id, name: "w2".into() });
        k.process_event(KernelEvent::SelectWindow { session_id, index: 1 });
        assert_eq!(k.sessions.get(&session_id).map(|s| s.active_window_idx), Some(1));
    }

    #[test]
    fn session_by_name() {
        let mut k = test_kernel();
        k.process_event(KernelEvent::CreateSession { name: "dev".into(), size: Size::new(80, 24) });
        assert!(k.session_by_name("dev").is_some());
        assert!(k.session_by_name("prod").is_none());
    }

    #[test]
    fn rename_session() {
        let mut k = test_kernel();
        let effects = k.process_event(KernelEvent::CreateSession { name: "old".into(), size: Size::new(80, 24) });
        let session_id = match &effects[0] {
            KernelEffect::SessionCreated { session_id, .. } => *session_id,
            _ => SessionId(0),
        };
        k.process_event(KernelEvent::RenameSession { session_id, name: "new".into() });
        assert!(k.session_by_name("new").is_some());
    }

    #[test]
    fn pane_grid_access() {
        let mut k = test_kernel();
        let effects = k.process_event(KernelEvent::CreateSession { name: "s".into(), size: Size::new(80, 24) });
        let pane_id = match &effects[0] {
            KernelEffect::SessionCreated { pane_id, .. } => *pane_id,
            _ => PaneId(0),
        };
        assert!(k.pane_grid(pane_id).is_some());
    }

    #[test]
    fn pty_output_applies_vt() {
        let mut k = test_kernel();
        let effects = k.process_event(KernelEvent::CreateSession { name: "s".into(), size: Size::new(80, 24) });
        let pane_id = match &effects[0] {
            KernelEffect::SessionCreated { pane_id, .. } => *pane_id,
            _ => PaneId(0),
        };
        k.process_event(KernelEvent::PtyOutput { pane_id, data: b"Hello\r\nWorld".to_vec() });
        let pane = k.panes.get(&pane_id);
        assert!(pane.is_some());
        let text = pane.map(|p| p.grid.line_text(0, &p.arena)).unwrap_or_default();
        assert_eq!(text, "Hello");
    }

    #[test]
    fn csi_cursor_movement() {
        let mut k = test_kernel();
        let effects = k.process_event(KernelEvent::CreateSession { name: "s".into(), size: Size::new(80, 24) });
        let pane_id = match &effects[0] {
            KernelEffect::SessionCreated { pane_id, .. } => *pane_id,
            _ => PaneId(0),
        };
        // Move cursor to position (5, 3) via CSI
        k.process_event(KernelEvent::PtyOutput { pane_id, data: b"\x1b[4;6H".to_vec() });
        let cursor = k.pane_grid(pane_id).map(ChunkedGrid::cursor);
        assert_eq!(cursor, Some((5, 3)));
    }

    #[test]
    fn erase_display() {
        let mut k = test_kernel();
        let effects = k.process_event(KernelEvent::CreateSession { name: "s".into(), size: Size::new(80, 24) });
        let pane_id = match &effects[0] {
            KernelEffect::SessionCreated { pane_id, .. } => *pane_id,
            _ => PaneId(0),
        };
        k.process_event(KernelEvent::PtyOutput { pane_id, data: b"ABCDE\x1b[2J".to_vec() });
        let pane = k.panes.get(&pane_id);
        let text = pane.map(|p| p.grid.line_text(0, &p.arena)).unwrap_or_default();
        assert_eq!(text, "");
    }

    #[test]
    fn pane_count() {
        let mut k = test_kernel();
        assert_eq!(k.pane_count(), 0);
        k.process_event(KernelEvent::CreateSession { name: "s".into(), size: Size::new(80, 24) });
        assert_eq!(k.pane_count(), 1);
    }

    #[test]
    fn select_pane() {
        let mut k = test_kernel();
        let effects = k.process_event(KernelEvent::CreateSession { name: "s".into(), size: Size::new(80, 24) });
        let window_id = match &effects[0] {
            KernelEffect::SessionCreated { window_id, .. } => *window_id,
            _ => WindowId(0),
        };
        k.process_event(KernelEvent::SplitPane { window_id, size: Size::new(40, 24), vertical: true });
        k.process_event(KernelEvent::SelectPane { window_id, index: 1 });
        assert_eq!(k.windows.get(&window_id).map(|w| w.active_pane_idx), Some(1));
    }

    #[test]
    fn destroy_pane() {
        let mut k = test_kernel();
        let effects = k.process_event(KernelEvent::CreateSession { name: "s".into(), size: Size::new(80, 24) });
        let pane_id = match &effects[0] {
            KernelEffect::SessionCreated { pane_id, .. } => *pane_id,
            _ => PaneId(0),
        };
        k.process_event(KernelEvent::DestroyPane { pane_id });
        assert_eq!(k.pane_count(), 0);
    }

    #[test]
    fn destroy_nonexistent_session() {
        let mut k = test_kernel();
        let effects = k.process_event(KernelEvent::DestroySession { session_id: SessionId(999) });
        assert!(effects.iter().any(|e| matches!(e, KernelEffect::Error(_))));
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn create_destroy_cycle(count in 1usize..20) {
                let mut k = test_kernel();
                let mut session_ids = Vec::new();
                for i in 0..count {
                    let effects = k.process_event(KernelEvent::CreateSession {
                        name: format!("s{i}"),
                        size: Size::new(80, 24),
                    });
                    if let Some(KernelEffect::SessionCreated { session_id, .. }) = effects.first() {
                        session_ids.push(*session_id);
                    }
                }
                prop_assert_eq!(k.session_count(), count);
                for sid in session_ids {
                    k.process_event(KernelEvent::DestroySession { session_id: sid });
                }
                prop_assert_eq!(k.session_count(), 0);
            }

            #[test]
            fn ids_monotonic(count in 1usize..10) {
                let mut k = test_kernel();
                let mut prev = 0u64;
                for i in 0..count {
                    let effects = k.process_event(KernelEvent::CreateSession {
                        name: format!("s{i}"),
                        size: Size::new(80, 24),
                    });
                    if let Some(KernelEffect::SessionCreated { session_id, .. }) = effects.first() {
                        prop_assert!(session_id.0 > prev);
                        prev = session_id.0;
                    }
                }
            }
        }
    }
}
