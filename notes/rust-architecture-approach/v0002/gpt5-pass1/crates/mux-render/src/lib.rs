//! # mux-render
//!
//! Composition + dirty-line diff rendering.
//!
//! Strategy:
//! 1. Compose pane surfaces into a target framebuffer.
//! 2. Diff against previous framebuffer, line-by-line.
//! 3. Emit coalesced ANSI updates with per-frame byte budget.

#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};

use mux_types::{Cell, Line, Style};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone)]
pub struct PaneSurface {
    pub pane_id: u64,
    pub z_index: u16,
    pub rect: Rect,
    pub lines: Vec<Line>,
    pub dirty_rows: Vec<bool>,
    pub epoch: u64,
}

impl PaneSurface {
    #[must_use]
    pub fn dirty_row_count(&self) -> usize {
        self.dirty_rows.iter().filter(|dirty| **dirty).count()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FramePolicy {
    pub target_fps: u16,
    pub max_batch_bytes: usize,
    pub pane_row_quota: usize,
}

impl Default for FramePolicy {
    fn default() -> Self {
        Self {
            target_fps: 60,
            max_batch_bytes: 64 * 1024,
            pane_row_quota: 40,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FrameStats {
    pub changed_lines: usize,
    pub changed_spans: usize,
    pub emitted_bytes: usize,
    pub skipped_for_quota: usize,
}

#[derive(Debug, Clone, Default)]
pub struct FrameOutput {
    pub bytes: Vec<u8>,
    pub stats: FrameStats,
    pub deferred_panes: Vec<u64>,
}

#[derive(Debug, Clone)]
pub struct Compositor {
    rows: usize,
    cols: usize,
    policy: FramePolicy,
    previous: Vec<Line>,
    frame_id: u64,
    next_deadline_ms: u64,
    pane_stride_offset: HashMap<u64, usize>,
}

impl Compositor {
    #[must_use]
    pub fn new(rows: usize, cols: usize, policy: FramePolicy) -> Self {
        Self {
            rows,
            cols,
            policy,
            previous: vec![Line::new(cols); rows],
            frame_id: 0,
            next_deadline_ms: 0,
            pane_stride_offset: HashMap::new(),
        }
    }

    #[must_use]
    pub fn rows(&self) -> usize {
        self.rows
    }

    #[must_use]
    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.rows = rows;
        self.cols = cols;
        self.previous = vec![Line::new(cols); rows];
    }

    #[must_use]
    pub fn compose_frame(
        &mut self,
        now_ms: u64,
        panes: &[PaneSurface],
        force: bool,
    ) -> Option<FrameOutput> {
        if !force && now_ms < self.next_deadline_ms {
            return None;
        }

        let frame_interval = (1000u64 / u64::from(self.policy.target_fps.max(1))).max(1);
        self.next_deadline_ms = now_ms.saturating_add(frame_interval);
        self.frame_id = self.frame_id.saturating_add(1);

        let mut framebuffer = vec![Line::new(self.cols); self.rows];
        let mut changed_rows = HashSet::new();
        let mut deferred = Vec::new();
        let mut skipped = 0usize;

        let mut panes_sorted = panes.to_vec();
        panes_sorted.sort_by_key(|p| (p.z_index, p.pane_id, p.epoch));

        for pane in panes_sorted {
            let mask = self.row_mask_for_pane(&pane);
            let mut pane_deferred = false;

            for local_row in 0..pane.rect.rows as usize {
                let global_row = pane.rect.y as usize + local_row;
                if global_row >= self.rows {
                    continue;
                }
                let dirty = pane.dirty_rows.get(local_row).copied().unwrap_or(true);
                if !dirty {
                    continue;
                }
                if !mask.get(local_row).copied().unwrap_or(true) {
                    pane_deferred = true;
                    skipped = skipped.saturating_add(1);
                    continue;
                }

                let Some(src_line) = pane.lines.get(local_row) else {
                    continue;
                };
                let dst_line = &mut framebuffer[global_row];
                blit_line(dst_line, src_line, pane.rect.x as usize, self.cols);
                changed_rows.insert(global_row);
            }

            if pane_deferred {
                deferred.push(pane.pane_id);
            }
        }

        let mut bytes = Vec::new();
        let mut changed_lines = 0usize;
        let mut changed_spans = 0usize;

        for row in 0..self.rows {
            if !changed_rows.contains(&row) && self.previous.get(row) == framebuffer.get(row) {
                continue;
            }

            let Some(prev) = self.previous.get(row) else {
                continue;
            };
            let Some(next) = framebuffer.get(row) else {
                continue;
            };
            let spans = coalesce_spans(&line_diff_spans(prev, next), 2);
            if spans.is_empty() {
                continue;
            }

            changed_lines = changed_lines.saturating_add(1);
            changed_spans = changed_spans.saturating_add(spans.len());

            for (start_col, end_col) in spans {
                let mut op = encode_span(row, start_col, end_col, next);
                if bytes.len().saturating_add(op.len()) > self.policy.max_batch_bytes {
                    break;
                }
                bytes.append(&mut op);
            }

            if bytes.len() >= self.policy.max_batch_bytes {
                break;
            }
        }

        self.previous = framebuffer;

        Some(FrameOutput {
            stats: FrameStats {
                changed_lines,
                changed_spans,
                emitted_bytes: bytes.len(),
                skipped_for_quota: skipped,
            },
            bytes,
            deferred_panes: deferred,
        })
    }

    fn row_mask_for_pane(&mut self, pane: &PaneSurface) -> Vec<bool> {
        let dirty_count = pane.dirty_row_count();
        if dirty_count <= self.policy.pane_row_quota {
            return pane.dirty_rows.clone();
        }

        let stride = ((dirty_count + self.policy.pane_row_quota - 1) / self.policy.pane_row_quota).max(1);
        let offset = self.pane_stride_offset.entry(pane.pane_id).or_insert(0);

        let mut out = vec![false; pane.dirty_rows.len()];
        let mut seen = 0usize;
        for (idx, dirty) in pane.dirty_rows.iter().copied().enumerate() {
            if !dirty {
                continue;
            }
            if seen % stride == *offset {
                out[idx] = true;
            }
            seen = seen.saturating_add(1);
        }

        *offset = (*offset + 1) % stride;
        out
    }
}

fn blit_line(dst: &mut Line, src: &Line, x_offset: usize, max_cols: usize) {
    for (idx, cell) in src.cells().iter().cloned().enumerate() {
        let col = x_offset + idx;
        if col >= max_cols {
            break;
        }
        dst.set(col, cell);
    }
}

#[must_use]
pub fn line_diff_spans(previous: &Line, next: &Line) -> Vec<(usize, usize)> {
    let max = previous.len().max(next.len());
    let mut spans = Vec::new();
    let mut start: Option<usize> = None;

    for idx in 0..max {
        let left = previous.get(idx);
        let right = next.get(idx);
        if left != right {
            if start.is_none() {
                start = Some(idx);
            }
        } else if let Some(s) = start.take() {
            spans.push((s, idx));
        }
    }

    if let Some(s) = start {
        spans.push((s, max));
    }

    spans
}

#[must_use]
pub fn coalesce_spans(spans: &[(usize, usize)], gap: usize) -> Vec<(usize, usize)> {
    if spans.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::with_capacity(spans.len());
    let mut current = spans[0];

    for span in spans.iter().copied().skip(1) {
        if span.0 <= current.1.saturating_add(gap) {
            current.1 = current.1.max(span.1);
        } else {
            out.push(current);
            current = span;
        }
    }

    out.push(current);
    out
}

fn encode_span(row: usize, start_col: usize, end_col: usize, line: &Line) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(format!("\x1b[{};{}H", row + 1, start_col + 1).as_bytes());

    let mut last_style = Style::default();
    let mut style_initialized = false;

    for col in start_col..end_col {
        let Some(cell) = line.get(col) else {
            continue;
        };
        if !style_initialized || *cell.style() != last_style {
            out.extend_from_slice(style_to_sgr(cell.style()).as_bytes());
            last_style = *cell.style();
            style_initialized = true;
        }
        write_cell(&mut out, cell);
    }

    out
}

fn write_cell(out: &mut Vec<u8>, cell: &Cell) {
    if cell.width() == 0 {
        return;
    }
    out.extend_from_slice(cell.grapheme().as_bytes());
}

fn style_to_sgr(style: &Style) -> String {
    let mut parts = vec!["0".to_string()];

    let bits = style.attrs.bits();
    if bits & mux_types::Attrs::BOLD != 0 {
        parts.push("1".to_string());
    }
    if bits & mux_types::Attrs::DIM != 0 {
        parts.push("2".to_string());
    }
    if bits & mux_types::Attrs::ITALIC != 0 {
        parts.push("3".to_string());
    }
    if bits & mux_types::Attrs::UNDERLINE != 0 {
        parts.push("4".to_string());
    }
    if bits & mux_types::Attrs::REVERSE != 0 {
        parts.push("7".to_string());
    }

    parts.extend(color_to_sgr(style.fg, true));
    parts.extend(color_to_sgr(style.bg, false));

    format!("\x1b[{}m", parts.join(";"))
}

fn color_to_sgr(color: mux_types::Color, fg: bool) -> Vec<String> {
    match color {
        mux_types::Color::Default => vec![if fg { "39" } else { "49" }.to_string()],
        mux_types::Color::Indexed(idx) => {
            vec![if fg {
                format!("38;5;{idx}")
            } else {
                format!("48;5;{idx}")
            }]
        }
        mux_types::Color::Rgb(r, g, b) => {
            vec![if fg {
                format!("38;2;{r};{g};{b}")
            } else {
                format!("48;2;{r};{g};{b}")
            }]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mux_types::Cell;

    fn mk_line(s: &str) -> Line {
        let mut line = Line::new(s.chars().count().max(1));
        for (idx, ch) in s.chars().enumerate() {
            line.set(idx, Cell::from_char(ch, Style::default()));
        }
        line
    }

    #[test]
    fn line_diff_detects_single_span() {
        let left = mk_line("hello");
        let right = mk_line("hallo");
        let spans = line_diff_spans(&left, &right);
        assert_eq!(spans, vec![(1, 2)]);
    }

    #[test]
    fn coalesce_span_merges_small_gap() {
        let out = coalesce_spans(&[(0, 1), (2, 3), (10, 11)], 1);
        assert_eq!(out, vec![(0, 3), (10, 11)]);
    }

    #[test]
    fn compositor_respects_frame_deadline() {
        let mut c = Compositor::new(2, 8, FramePolicy::default());
        let pane = PaneSurface {
            pane_id: 1,
            z_index: 0,
            rect: Rect::new(0, 0, 8, 1),
            lines: vec![mk_line("hello")],
            dirty_rows: vec![true],
            epoch: 1,
        };

        let first = c.compose_frame(0, &[pane.clone()], false);
        assert!(first.is_some());

        let second = c.compose_frame(1, &[pane], false);
        assert!(second.is_none());
    }

    #[test]
    fn compositor_limits_flooding_pane_rows() {
        let mut c = Compositor::new(
            20,
            10,
            FramePolicy {
                target_fps: 60,
                max_batch_bytes: 64 * 1024,
                pane_row_quota: 4,
            },
        );
        let lines = (0..20)
            .map(|_| mk_line("abcdefghij"))
            .collect::<Vec<_>>();
        let pane = PaneSurface {
            pane_id: 10,
            z_index: 0,
            rect: Rect::new(0, 0, 10, 20),
            lines,
            dirty_rows: vec![true; 20],
            epoch: 1,
        };

        let frame = c.compose_frame(0, &[pane], true);
        assert!(frame.is_some());
        let frame = frame.unwrap_or_default();
        assert!(frame.stats.skipped_for_quota > 0);
        assert_eq!(frame.deferred_panes, vec![10]);
    }

    #[test]
    fn compositor_emits_ansi_cursor_move() {
        let mut c = Compositor::new(2, 8, FramePolicy::default());
        let pane = PaneSurface {
            pane_id: 1,
            z_index: 0,
            rect: Rect::new(0, 0, 8, 1),
            lines: vec![mk_line("z")],
            dirty_rows: vec![true],
            epoch: 1,
        };

        let frame = c.compose_frame(0, &[pane], true);
        assert!(frame.is_some());
        let frame = frame.unwrap_or_default();
        let bytes = String::from_utf8(frame.bytes).ok().unwrap_or_default();
        assert!(bytes.contains("\x1b[1;1H"));
    }
}
