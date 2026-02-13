//! # mux-kernel
//!
//! Sans-IO kernel: deterministic reducer, layout solver, and copy-mode state.

#![forbid(unsafe_code)]

use std::collections::{HashMap, VecDeque};

// --- Slot Map / GenSlotMap ---

pub type Generation = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotKey {
    pub index: u32,
    pub generation: Generation,
}

impl SlotKey {
    #[must_use]
    pub fn new(index: u32, generation: Generation) -> Self {
        Self { index, generation }
    }
}

#[derive(Debug, Clone)]
enum Slot<T> {
    Occupied { value: T, generation: Generation },
    Vacant { generation: Generation },
}

#[derive(Debug, Clone)]
pub struct GenSlotMap<T> {
    slots: Vec<Slot<T>>,
    free_list: Vec<u32>,
    count: usize,
}

impl<T> GenSlotMap<T> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_list: Vec::new(),
            count: 0,
        }
    }

    pub fn insert(&mut self, value: T) -> SlotKey {
        if let Some(index) = self.free_list.pop() {
            let slot = &mut self.slots[index as usize];
            let generation = match slot {
                Slot::Occupied { generation, .. } | Slot::Vacant { generation } => {
                    generation.saturating_add(1)
                }
            };
            *slot = Slot::Occupied { value, generation };
            self.count += 1;
            SlotKey::new(index, generation)
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(Slot::Occupied {
                value,
                generation: 0,
            });
            self.count += 1;
            SlotKey::new(index, 0)
        }
    }

    #[must_use]
    pub fn get(&self, key: SlotKey) -> Option<&T> {
        match self.slots.get(key.index as usize)? {
            Slot::Occupied { value, generation } if *generation == key.generation => Some(value),
            _ => None,
        }
    }

    #[must_use]
    pub fn get_mut(&mut self, key: SlotKey) -> Option<&mut T> {
        match self.slots.get_mut(key.index as usize)? {
            Slot::Occupied { value, generation } if *generation == key.generation => Some(value),
            _ => None,
        }
    }

    pub fn remove(&mut self, key: SlotKey) -> Option<T> {
        let slot = self.slots.get_mut(key.index as usize)?;
        match slot {
            Slot::Occupied { generation, .. } if *generation == key.generation => {
                let generation = *generation;
                let prior = std::mem::replace(slot, Slot::Vacant { generation });
                self.free_list.push(key.index);
                self.count = self.count.saturating_sub(1);
                match prior {
                    Slot::Occupied { value, .. } => Some(value),
                    Slot::Vacant { .. } => None,
                }
            }
            _ => None,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.count
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

impl<T> Default for GenSlotMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

// --- Layout Engine ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub cols: u16,
    pub rows: u16,
}

impl Rect {
    #[must_use]
    pub fn new(x: u16, y: u16, cols: u16, rows: u16) -> Self {
        Self { x, y, cols, rows }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinLayout {
    EvenHorizontal,
    EvenVertical,
    MainHorizontal,
    MainVertical,
    Tiled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutNode {
    Leaf { pane: SlotKey, min_size: u16 },
    Split {
        axis: Axis,
        children: Vec<LayoutNode>,
        weights: Vec<u16>,
        main_index: Option<usize>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowLayout {
    pub root: LayoutNode,
    pub encoded: String,
}

impl WindowLayout {
    #[must_use]
    pub fn single_pane(pane: SlotKey) -> Self {
        Self {
            root: LayoutNode::Leaf { pane, min_size: 1 },
            encoded: "00,leaf".to_string(),
        }
    }

    #[must_use]
    pub fn checksum(payload: &str) -> u8 {
        payload
            .as_bytes()
            .iter()
            .fold(0u8, |acc, byte| acc.wrapping_add(*byte))
    }

    #[must_use]
    pub fn parse_custom(spec: &str, panes: &[SlotKey]) -> Option<Self> {
        let (checksum_hex, payload) = spec.split_once(',')?;
        let expected = u8::from_str_radix(checksum_hex, 16).ok()?;
        if expected != Self::checksum(payload) {
            return None;
        }
        let layout = match payload {
            "even-horizontal" => built_in_layout(BuiltinLayout::EvenHorizontal, panes),
            "even-vertical" => built_in_layout(BuiltinLayout::EvenVertical, panes),
            "main-horizontal" => built_in_layout(BuiltinLayout::MainHorizontal, panes),
            "main-vertical" => built_in_layout(BuiltinLayout::MainVertical, panes),
            "tiled" => built_in_layout(BuiltinLayout::Tiled, panes),
            _ => return None,
        };
        Some(layout)
    }
}

#[must_use]
pub fn built_in_layout(layout: BuiltinLayout, panes: &[SlotKey]) -> WindowLayout {
    let pane_list = if panes.is_empty() {
        Vec::new()
    } else {
        panes.to_vec()
    };

    let payload = match layout {
        BuiltinLayout::EvenHorizontal => "even-horizontal",
        BuiltinLayout::EvenVertical => "even-vertical",
        BuiltinLayout::MainHorizontal => "main-horizontal",
        BuiltinLayout::MainVertical => "main-vertical",
        BuiltinLayout::Tiled => "tiled",
    };

    if pane_list.is_empty() {
        return WindowLayout {
            root: LayoutNode::Split {
                axis: Axis::Horizontal,
                children: Vec::new(),
                weights: Vec::new(),
                main_index: None,
            },
            encoded: format!("{:02x},{}", WindowLayout::checksum(payload), payload),
        };
    }

    if pane_list.len() == 1 {
        return WindowLayout {
            root: LayoutNode::Leaf {
                pane: pane_list[0],
                min_size: 1,
            },
            encoded: format!("{:02x},{}", WindowLayout::checksum(payload), payload),
        };
    }

    let root = match layout {
        BuiltinLayout::EvenHorizontal => LayoutNode::Split {
            axis: Axis::Vertical,
            children: pane_list
                .into_iter()
                .map(|pane| LayoutNode::Leaf { pane, min_size: 1 })
                .collect(),
            weights: vec![1; panes.len()],
            main_index: None,
        },
        BuiltinLayout::EvenVertical => LayoutNode::Split {
            axis: Axis::Horizontal,
            children: pane_list
                .into_iter()
                .map(|pane| LayoutNode::Leaf { pane, min_size: 1 })
                .collect(),
            weights: vec![1; panes.len()],
            main_index: None,
        },
        BuiltinLayout::MainHorizontal => {
            let main = LayoutNode::Leaf {
                pane: pane_list[0],
                min_size: 3,
            };
            let tail = LayoutNode::Split {
                axis: Axis::Horizontal,
                children: pane_list[1..]
                    .iter()
                    .copied()
                    .map(|pane| LayoutNode::Leaf { pane, min_size: 1 })
                    .collect(),
                weights: vec![1; pane_list.len().saturating_sub(1)],
                main_index: None,
            };
            LayoutNode::Split {
                axis: Axis::Vertical,
                children: vec![main, tail],
                weights: vec![3, 1],
                main_index: Some(0),
            }
        }
        BuiltinLayout::MainVertical => {
            let main = LayoutNode::Leaf {
                pane: pane_list[0],
                min_size: 3,
            };
            let tail = LayoutNode::Split {
                axis: Axis::Vertical,
                children: pane_list[1..]
                    .iter()
                    .copied()
                    .map(|pane| LayoutNode::Leaf { pane, min_size: 1 })
                    .collect(),
                weights: vec![1; pane_list.len().saturating_sub(1)],
                main_index: None,
            };
            LayoutNode::Split {
                axis: Axis::Horizontal,
                children: vec![main, tail],
                weights: vec![3, 1],
                main_index: Some(0),
            }
        }
        BuiltinLayout::Tiled => {
            let mid = (pane_list.len() + 1) / 2;
            let top = LayoutNode::Split {
                axis: Axis::Horizontal,
                children: pane_list[..mid]
                    .iter()
                    .copied()
                    .map(|pane| LayoutNode::Leaf { pane, min_size: 1 })
                    .collect(),
                weights: vec![1; mid],
                main_index: None,
            };
            let bottom_children = pane_list[mid..]
                .iter()
                .copied()
                .map(|pane| LayoutNode::Leaf { pane, min_size: 1 })
                .collect::<Vec<_>>();
            let bottom = if bottom_children.is_empty() {
                LayoutNode::Split {
                    axis: Axis::Horizontal,
                    children: Vec::new(),
                    weights: Vec::new(),
                    main_index: None,
                }
            } else {
                LayoutNode::Split {
                    axis: Axis::Horizontal,
                    children: bottom_children,
                    weights: vec![1; pane_list.len().saturating_sub(mid)],
                    main_index: None,
                }
            };
            LayoutNode::Split {
                axis: Axis::Vertical,
                children: vec![top, bottom],
                weights: vec![1, 1],
                main_index: None,
            }
        }
    };

    WindowLayout {
        root,
        encoded: format!("{:02x},{}", WindowLayout::checksum(payload), payload),
    }
}

#[must_use]
pub fn solve_layout(root: &LayoutNode, area: Rect) -> HashMap<SlotKey, Rect> {
    let mut out = HashMap::new();
    solve_node(root, area, &mut out);
    out
}

fn solve_node(node: &LayoutNode, area: Rect, out: &mut HashMap<SlotKey, Rect>) {
    match node {
        LayoutNode::Leaf { pane, .. } => {
            out.insert(*pane, area);
        }
        LayoutNode::Split {
            axis,
            children,
            weights,
            ..
        } => {
            if children.is_empty() {
                return;
            }

            let mins = children
                .iter()
                .map(|child| match child {
                    LayoutNode::Leaf { min_size, .. } => *min_size,
                    LayoutNode::Split { .. } => 1,
                })
                .collect::<Vec<_>>();

            let lengths = distribute_lengths(
                if *axis == Axis::Horizontal {
                    area.cols
                } else {
                    area.rows
                },
                weights,
                &mins,
            );

            let mut cursor_x = area.x;
            let mut cursor_y = area.y;
            for (child, length) in children.iter().zip(lengths.iter().copied()) {
                let child_area = if *axis == Axis::Horizontal {
                    Rect::new(cursor_x, area.y, length, area.rows)
                } else {
                    Rect::new(area.x, cursor_y, area.cols, length)
                };
                solve_node(child, child_area, out);
                if *axis == Axis::Horizontal {
                    cursor_x = cursor_x.saturating_add(length);
                } else {
                    cursor_y = cursor_y.saturating_add(length);
                }
            }
        }
    }
}

#[must_use]
pub fn distribute_lengths(total: u16, weights: &[u16], minimums: &[u16]) -> Vec<u16> {
    let count = weights.len().max(minimums.len());
    if count == 0 {
        return Vec::new();
    }

    let safe_weights = (0..count)
        .map(|idx| {
            let weight = weights.get(idx).copied().unwrap_or(1);
            if weight == 0 { 1 } else { weight }
        })
        .collect::<Vec<_>>();

    let mins = (0..count)
        .map(|idx| minimums.get(idx).copied().unwrap_or(1).max(1))
        .collect::<Vec<_>>();

    let min_sum = mins
        .iter()
        .fold(0u16, |acc, val| acc.saturating_add(*val));

    if total <= count as u16 {
        return vec![1; count];
    }

    if min_sum >= total {
        let mut out = vec![1u16; count];
        let mut remaining = total.saturating_sub(count as u16);
        let mut idx = 0usize;
        while remaining > 0 {
            out[idx] = out[idx].saturating_add(1);
            idx = (idx + 1) % count;
            remaining = remaining.saturating_sub(1);
        }
        return out;
    }

    let mut out = mins;
    let extra = total - min_sum;
    let weight_sum = safe_weights.iter().fold(0u32, |acc, w| acc + *w as u32).max(1);

    let mut assigned = 0u16;
    for idx in 0..count {
        let share = ((extra as u32 * safe_weights[idx] as u32) / weight_sum) as u16;
        out[idx] = out[idx].saturating_add(share);
        assigned = assigned.saturating_add(share);
    }

    let mut remaining = extra.saturating_sub(assigned);
    let mut idx = 0usize;
    while remaining > 0 {
        out[idx] = out[idx].saturating_add(1);
        idx = (idx + 1) % count;
        remaining = remaining.saturating_sub(1);
    }

    out
}

// --- Copy Mode ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyTable {
    Vi,
    Emacs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    Character,
    Line,
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CopyCursor {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopySelection {
    pub anchor: CopyCursor,
    pub focus: CopyCursor,
    pub mode: SelectionMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    pub needle: String,
    pub case_sensitive: bool,
    pub direction: SearchDirection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyModeState {
    pub enabled: bool,
    pub key_table: KeyTable,
    pub cursor: CopyCursor,
    pub scroll_offset: usize,
    pub selection: Option<CopySelection>,
    pub search: Option<SearchQuery>,
    pub buffer_ring: VecDeque<String>,
}

impl CopyModeState {
    #[must_use]
    pub fn new(key_table: KeyTable) -> Self {
        Self {
            enabled: true,
            key_table,
            cursor: CopyCursor { row: 0, col: 0 },
            scroll_offset: 0,
            selection: None,
            search: None,
            buffer_ring: VecDeque::new(),
        }
    }

    pub fn move_cursor(&mut self, delta_row: isize, delta_col: isize, max_rows: usize, max_cols: usize) {
        let row = self.cursor.row as isize + delta_row;
        let col = self.cursor.col as isize + delta_col;
        let max_row = max_rows.saturating_sub(1) as isize;
        let max_col = max_cols.saturating_sub(1) as isize;
        self.cursor.row = row.clamp(0, max_row.max(0)) as usize;
        self.cursor.col = col.clamp(0, max_col.max(0)) as usize;

        if let Some(selection) = &mut self.selection {
            selection.focus = self.cursor;
        }
    }

    pub fn begin_selection(&mut self, mode: SelectionMode) {
        let cursor = self.cursor;
        self.selection = Some(CopySelection {
            anchor: cursor,
            focus: cursor,
            mode,
        });
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    #[must_use]
    pub fn yank_selection(&mut self, viewport: &[String]) -> Option<String> {
        let selection = self.selection.clone()?;
        let top = selection.anchor.row.min(selection.focus.row);
        let bottom = selection.anchor.row.max(selection.focus.row);
        let left = selection.anchor.col.min(selection.focus.col);
        let right = selection.anchor.col.max(selection.focus.col);

        let mut out = String::new();
        for row in top..=bottom {
            let line = viewport.get(row)?;
            match selection.mode {
                SelectionMode::Character => {
                    let chunk = slice_chars(line, left, right.saturating_add(1));
                    out.push_str(&chunk);
                }
                SelectionMode::Line => {
                    out.push_str(line.trim_end());
                }
                SelectionMode::Block => {
                    let chunk = slice_chars(line, left, right.saturating_add(1));
                    out.push_str(&chunk);
                }
            }
            if row != bottom {
                out.push('\n');
            }
        }

        self.buffer_ring.push_front(out.clone());
        while self.buffer_ring.len() > 64 {
            let _ = self.buffer_ring.pop_back();
        }
        Some(out)
    }

    #[must_use]
    pub fn search_next(&mut self, haystack: &[String], query: SearchQuery) -> Option<CopyCursor> {
        if query.needle.is_empty() {
            return None;
        }
        let needle = if query.case_sensitive {
            query.needle.clone()
        } else {
            query.needle.to_lowercase()
        };

        let scan = |line: &str| {
            if query.case_sensitive {
                line.to_string()
            } else {
                line.to_lowercase()
            }
        };

        let found = match query.direction {
            SearchDirection::Forward => {
                let start_row = self.cursor.row;
                (start_row..haystack.len()).find_map(|row| {
                    let line = scan(&haystack[row]);
                    line.find(&needle).map(|col| CopyCursor { row, col })
                })
            }
            SearchDirection::Backward => {
                let mut row = self.cursor.row.min(haystack.len().saturating_sub(1));
                let mut result = None;
                loop {
                    let line = scan(&haystack[row]);
                    if let Some(col) = line.rfind(&needle) {
                        result = Some(CopyCursor { row, col });
                        break;
                    }
                    if row == 0 {
                        break;
                    }
                    row -= 1;
                }
                result
            }
        };

        self.search = Some(query);
        if let Some(cursor) = found {
            self.cursor = cursor;
        }
        found
    }
}

fn slice_chars(s: &str, start: usize, end: usize) -> String {
    s.chars().skip(start).take(end.saturating_sub(start)).collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyModeInput {
    Move { dr: isize, dc: isize },
    BeginSelection(SelectionMode),
    ClearSelection,
    Search(SearchQuery),
    Yank,
    Exit,
}

// --- Server Graph and Kernel ---

#[derive(Debug, Clone)]
pub struct Session {
    pub name: String,
    pub windows: Vec<SlotKey>,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub name: String,
    pub session: SlotKey,
    pub panes: Vec<SlotKey>,
    pub layout: WindowLayout,
}

#[derive(Debug, Clone)]
pub struct Pane {
    pub window: SlotKey,
    pub pty_id: u32,
}

#[derive(Debug, Clone)]
pub struct ServerGraph {
    pub sessions: GenSlotMap<Session>,
    pub windows: GenSlotMap<Window>,
    pub panes: GenSlotMap<Pane>,
}

impl ServerGraph {
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: GenSlotMap::new(),
            windows: GenSlotMap::new(),
            panes: GenSlotMap::new(),
        }
    }

    pub fn create_session(&mut self, name: &str) -> SlotKey {
        self.sessions.insert(Session {
            name: name.to_string(),
            windows: Vec::new(),
        })
    }

    pub fn create_window(&mut self, session: SlotKey, name: &str) -> Option<SlotKey> {
        let key = self.windows.insert(Window {
            name: name.to_string(),
            session,
            panes: Vec::new(),
            layout: WindowLayout {
                root: LayoutNode::Split {
                    axis: Axis::Horizontal,
                    children: Vec::new(),
                    weights: Vec::new(),
                    main_index: None,
                },
                encoded: "00,empty".to_string(),
            },
        });
        let sess = self.sessions.get_mut(session)?;
        sess.windows.push(key);

        if self.windows.get(key).is_none() {
            let _ = self.windows.remove(key);
            return None;
        }
        Some(key)
    }

    pub fn create_pane(&mut self, window: SlotKey, pty_id: u32) -> Option<SlotKey> {
        let pane_key = self.panes.insert(Pane { window, pty_id });
        let win = self.windows.get_mut(window)?;
        win.panes.push(pane_key);
        win.layout = built_in_layout(BuiltinLayout::Tiled, &win.panes);
        Some(pane_key)
    }
}

impl Default for ServerGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelEvent {
    ClientInput { client_id: u64, bytes: Vec<u8> },
    PtyOutput { pane: SlotKey, bytes: Vec<u8> },
    PtyExited { pane: SlotKey, status: i32 },
    Tick,
    SigWinch,
    SigChild,
    Resize { client_id: u64, rows: u16, cols: u16 },
    CreateSession { name: String },
    CreateWindow { session: SlotKey, name: String },
    CreatePane { window: SlotKey, pty_id: u32 },
    EnterCopyMode { pane: SlotKey, key_table: KeyTable },
    CopyModeInput { pane: SlotKey, input: CopyModeInput },
    ApplyLayout { window: SlotKey, layout: BuiltinLayout },
    Crash { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelEffect {
    WritePty { pane: SlotKey, bytes: Vec<u8> },
    SendClient { client_id: u64, bytes: Vec<u8> },
    SpawnPty { pane: SlotKey },
    ResizePty { pane: SlotKey, rows: u16, cols: u16 },
    RecomputeWindowLayout { window: SlotKey, area: Rect },
    RenderNow,
    ReapChildren,
    QueryTerminalSize,
    CopyBufferReady { pane: SlotKey, text: String },
    SessionCreated { session: SlotKey },
    WindowCreated { window: SlotKey },
    PaneCreated { pane: SlotKey },
    Log { message: String },
    CrashReportFrame { message: String },
    Shutdown { code: i32 },
}

#[derive(Debug, Clone)]
pub struct Kernel {
    pub graph: ServerGraph,
    pub active_session: Option<SlotKey>,
    pub client_sizes: HashMap<u64, (u16, u16)>,
    pub copy_mode: HashMap<SlotKey, CopyModeState>,
    pub last_geometry: HashMap<SlotKey, HashMap<SlotKey, Rect>>,
}

impl Kernel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            graph: ServerGraph::new(),
            active_session: None,
            client_sizes: HashMap::new(),
            copy_mode: HashMap::new(),
            last_geometry: HashMap::new(),
        }
    }

    pub fn step(&mut self, event: KernelEvent) -> Vec<KernelEffect> {
        match event {
            KernelEvent::CreateSession { name } => {
                let session = self.graph.create_session(&name);
                self.active_session.get_or_insert(session);
                vec![KernelEffect::SessionCreated { session }]
            }
            KernelEvent::CreateWindow { session, name } => {
                if let Some(window) = self.graph.create_window(session, &name) {
                    vec![KernelEffect::WindowCreated { window }]
                } else {
                    vec![KernelEffect::Log {
                        message: "create_window: unknown session".to_string(),
                    }]
                }
            }
            KernelEvent::CreatePane { window, pty_id } => {
                if let Some(pane) = self.graph.create_pane(window, pty_id) {
                    vec![KernelEffect::PaneCreated { pane }, KernelEffect::SpawnPty { pane }]
                } else {
                    vec![KernelEffect::Log {
                        message: "create_pane: unknown window".to_string(),
                    }]
                }
            }
            KernelEvent::ApplyLayout { window, layout } => {
                if let Some(win) = self.graph.windows.get_mut(window) {
                    win.layout = built_in_layout(layout, &win.panes);
                    vec![KernelEffect::RenderNow]
                } else {
                    vec![KernelEffect::Log {
                        message: "apply_layout: unknown window".to_string(),
                    }]
                }
            }
            KernelEvent::SigWinch => vec![KernelEffect::QueryTerminalSize],
            KernelEvent::Resize {
                client_id,
                rows,
                cols,
            } => {
                self.client_sizes.insert(client_id, (rows, cols));
                let mut effects = Vec::new();
                let Some(session) = self.active_session else {
                    return vec![KernelEffect::RenderNow];
                };
                let Some(sess) = self.graph.sessions.get(session) else {
                    return vec![KernelEffect::RenderNow];
                };

                for window in &sess.windows {
                    let area = Rect::new(0, 0, cols, rows.saturating_sub(1));
                    effects.push(KernelEffect::RecomputeWindowLayout {
                        window: *window,
                        area,
                    });
                    if let Some(win) = self.graph.windows.get(*window) {
                        let geometry = solve_layout(&win.layout.root, area);
                        self.last_geometry.insert(*window, geometry.clone());
                        for (pane, rect) in geometry {
                            effects.push(KernelEffect::ResizePty {
                                pane,
                                rows: rect.rows,
                                cols: rect.cols,
                            });
                        }
                    }
                }
                effects.push(KernelEffect::RenderNow);
                effects
            }
            KernelEvent::SigChild => vec![KernelEffect::ReapChildren],
            KernelEvent::PtyExited { pane, status } => {
                let _ = self.copy_mode.remove(&pane);
                vec![KernelEffect::Log {
                    message: format!("pane {:?} exited with {status}", pane),
                }]
            }
            KernelEvent::PtyOutput { pane, bytes } => {
                if self.copy_mode.contains_key(&pane) {
                    vec![KernelEffect::Log {
                        message: "pty output buffered while copy mode active".to_string(),
                    }]
                } else {
                    vec![KernelEffect::SendClient {
                        client_id: 0,
                        bytes,
                    }]
                }
            }
            KernelEvent::ClientInput { client_id: _, bytes } => vec![KernelEffect::WritePty {
                pane: SlotKey::new(0, 0),
                bytes,
            }],
            KernelEvent::EnterCopyMode { pane, key_table } => {
                self.copy_mode.insert(pane, CopyModeState::new(key_table));
                vec![KernelEffect::RenderNow]
            }
            KernelEvent::CopyModeInput { pane, input } => {
                let Some(state) = self.copy_mode.get_mut(&pane) else {
                    return vec![KernelEffect::Log {
                        message: "copy mode input ignored: not in copy mode".to_string(),
                    }];
                };
                match input {
                    CopyModeInput::Move { dr, dc } => {
                        state.move_cursor(dr, dc, 4_096, 4_096);
                        vec![KernelEffect::RenderNow]
                    }
                    CopyModeInput::BeginSelection(mode) => {
                        state.begin_selection(mode);
                        vec![KernelEffect::RenderNow]
                    }
                    CopyModeInput::ClearSelection => {
                        state.clear_selection();
                        vec![KernelEffect::RenderNow]
                    }
                    CopyModeInput::Search(query) => {
                        let _ = state.search_next(&[], query);
                        vec![KernelEffect::RenderNow]
                    }
                    CopyModeInput::Yank => {
                        let viewport = vec!["".to_string()];
                        if let Some(text) = state.yank_selection(&viewport) {
                            vec![KernelEffect::CopyBufferReady { pane, text }]
                        } else {
                            vec![KernelEffect::Log {
                                message: "yank requested with no active selection".to_string(),
                            }]
                        }
                    }
                    CopyModeInput::Exit => {
                        let _ = self.copy_mode.remove(&pane);
                        vec![KernelEffect::RenderNow]
                    }
                }
            }
            KernelEvent::Crash { reason } => vec![
                KernelEffect::CrashReportFrame {
                    message: reason.clone(),
                },
                KernelEffect::Shutdown { code: 101 },
            ],
            KernelEvent::Tick => vec![KernelEffect::RenderNow],
        }
    }
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_keys(count: usize) -> Vec<SlotKey> {
        (0..count)
            .map(|idx| SlotKey::new(idx as u32, 0))
            .collect::<Vec<_>>()
    }

    #[test]
    fn genslotmap_rejects_stale_key() {
        let mut sm = GenSlotMap::new();
        let k1 = sm.insert(10u32);
        let removed = sm.remove(k1);
        assert_eq!(removed, Some(10));
        let k2 = sm.insert(11u32);
        assert_ne!(k1, k2);
        assert_eq!(sm.get(k1), None);
        assert_eq!(sm.get(k2), Some(&11u32));
    }

    #[test]
    fn layout_checksum_validation() {
        let panes = make_keys(2);
        let payload = "even-vertical";
        let encoded = format!("{:02x},{}", WindowLayout::checksum(payload), payload);
        let parsed = WindowLayout::parse_custom(&encoded, &panes);
        assert!(parsed.is_some());

        let bad = format!("00,{}", payload);
        assert!(WindowLayout::parse_custom(&bad, &panes).is_none());
    }

    #[test]
    fn distribute_lengths_respects_total() {
        let out = distribute_lengths(80, &[1, 1, 1], &[10, 10, 10]);
        assert_eq!(out.iter().fold(0u16, |a, n| a + *n), 80);
        assert!(out.iter().all(|n| *n >= 10));
    }

    #[test]
    fn solve_even_vertical_produces_full_width() {
        let panes = make_keys(3);
        let layout = built_in_layout(BuiltinLayout::EvenVertical, &panes);
        let out = solve_layout(&layout.root, Rect::new(0, 0, 120, 40));
        assert_eq!(out.len(), 3);
        let width_sum = panes
            .iter()
            .filter_map(|pane| out.get(pane))
            .fold(0u16, |acc, rect| acc.saturating_add(rect.cols));
        assert_eq!(width_sum, 120);
    }

    #[test]
    fn solve_even_horizontal_produces_full_height() {
        let panes = make_keys(3);
        let layout = built_in_layout(BuiltinLayout::EvenHorizontal, &panes);
        let out = solve_layout(&layout.root, Rect::new(0, 0, 100, 30));
        assert_eq!(out.len(), 3);
        let height_sum = panes
            .iter()
            .filter_map(|pane| out.get(pane))
            .fold(0u16, |acc, rect| acc.saturating_add(rect.rows));
        assert_eq!(height_sum, 30);
    }

    #[test]
    fn copy_mode_selection_and_yank() {
        let mut state = CopyModeState::new(KeyTable::Vi);
        state.begin_selection(SelectionMode::Character);
        state.move_cursor(0, 4, 10, 10);
        let text = state.yank_selection(&["hello world".to_string()]);
        assert_eq!(text, Some("hello".to_string()));
        assert_eq!(state.buffer_ring.front().cloned(), Some("hello".to_string()));
    }

    #[test]
    fn copy_mode_search_forward() {
        let mut state = CopyModeState::new(KeyTable::Emacs);
        state.cursor = CopyCursor { row: 0, col: 0 };
        let found = state.search_next(
            &["alpha".to_string(), "beta gamma".to_string()],
            SearchQuery {
                needle: "gamma".to_string(),
                case_sensitive: true,
                direction: SearchDirection::Forward,
            },
        );
        assert_eq!(found, Some(CopyCursor { row: 1, col: 5 }));
    }

    #[test]
    fn kernel_resize_emits_recompute_and_resize_pty() {
        let mut kernel = Kernel::new();
        let session = kernel.graph.create_session("s");
        kernel.active_session = Some(session);
        let window = kernel.graph.create_window(session, "w");
        assert!(window.is_some());
        let win = window.unwrap_or(SlotKey::new(0, 0));
        let pane = kernel.graph.create_pane(win, 42);
        assert!(pane.is_some());

        let effects = kernel.step(KernelEvent::Resize {
            client_id: 1,
            rows: 40,
            cols: 120,
        });

        assert!(effects.iter().any(|e| matches!(
            e,
            KernelEffect::RecomputeWindowLayout { window: _, area: _ }
        )));
        assert!(effects.iter().any(|e| matches!(
            e,
            KernelEffect::ResizePty {
                pane: _,
                rows: _,
                cols: _
            }
        )));
        assert!(effects.iter().any(|e| matches!(e, KernelEffect::RenderNow)));
    }

    #[test]
    fn kernel_sigwinch_sequence_starts_with_query() {
        let mut kernel = Kernel::new();
        let effects = kernel.step(KernelEvent::SigWinch);
        assert_eq!(effects, vec![KernelEffect::QueryTerminalSize]);
    }

    #[test]
    fn crash_emits_report_before_shutdown() {
        let mut kernel = Kernel::new();
        let effects = kernel.step(KernelEvent::Crash {
            reason: "fatal".to_string(),
        });
        assert_eq!(effects.len(), 2);
        assert!(matches!(effects[0], KernelEffect::CrashReportFrame { .. }));
        assert!(matches!(effects[1], KernelEffect::Shutdown { .. }));
    }
}
