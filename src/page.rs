// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use std::collections::BTreeSet;

/// Page data buffer.
pub struct PageData {
    /// Page start address.
    pub offset: u64,
    /// Raw data.
    pub data: Vec<u8>,
    /// State map (changed, diff, etc).
    pub state: Vec<u8>,
}

impl PageData {
    pub const DEFAULT: u8 = 0;
    pub const CHANGED: u8 = 1;

    /// Create instance.
    pub fn new(offset: u64, data: Vec<u8>) -> Self {
        let state = vec![PageData::DEFAULT; data.len()];
        Self {
            offset,
            data,
            state,
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
    pub fn update(&mut self, changes: &BTreeSet<u64>) {
        for index in 0..self.data.len() {
            let offset = self.offset + index as u64;
            self.state[index] = if changes.contains(&offset) {
                PageData::CHANGED
            } else {
                PageData::DEFAULT
            };
        }
    }
}
