use crate::row::Row;
use std::collections::VecDeque;

pub struct ScrollbackBuffer {
    history: VecDeque<Row>,
    max_lines: usize,
}

impl ScrollbackBuffer {
    pub fn new(max_lines: usize) -> Self {
        Self {
            history: VecDeque::new(),
            max_lines,
        }
    }

    pub fn push(&mut self, row: Row) {
        if self.history.len() >= self.max_lines {
            self.history.pop_front();
        }
        self.history.push_back(row);
    }
}
