// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use std::collections::BTreeMap;

/// Page data.
pub struct Page {
    /// Page start address.
    pub offset: u64,
    /// Number of lines per page.
    pub lines: usize,
    /// Number of bytes per line.
    pub columns: usize,
    /// Raw data to display.
    pub data: Vec<u8>,
    /// Byte states (changed, diff, etc).
    pub state: Vec<u8>,
}

impl Page {
    pub const DEFAULT: u8 = 0;
    pub const CHANGED: u8 = 1;

    /// Create new instance.
    pub fn new() -> Self {
        Self {
            offset: u64::MAX,
            lines: 0,
            columns: 0,
            data: Vec::new(),
            state: Vec::new(),
        }
    }

    /// Check if offset is visible (belongs to the page).
    pub fn visible(&self, offset: u64) -> bool {
        offset >= self.offset && offset < self.offset + self.data.len() as u64
    }

    /// Get byte value with state.
    pub fn get(&self, offset: u64) -> Option<(u8, u8)> {
        if !self.visible(offset) {
            None
        } else {
            let index = (offset - self.offset) as usize;
            Some((self.data[index], self.state[index]))
        }
    }

    /// Update page with changed data.
    pub fn update(&mut self, changes: &BTreeMap<u64, u8>) {
        self.state.resize(self.data.len(), Page::DEFAULT);
        for index in 0..self.data.len() {
            let offset = self.offset + index as u64;
            self.state[index] = if changes.contains_key(&offset) {
                Page::CHANGED
            } else {
                Page::DEFAULT
            };
        }
    }
}
