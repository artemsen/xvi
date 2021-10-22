// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use std::collections::BTreeMap;
use std::collections::BTreeSet;

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
    /// Addresses of changed bytes.
    pub changed: BTreeSet<u64>,
}

impl Page {
    /// Create new instance.
    pub fn new() -> Self {
        Self {
            offset: u64::MAX,
            lines: 0,
            columns: 0,
            data: Vec::new(),
            state: Vec::new(),
            changed: BTreeSet::new(),
        }
    }

    /// Get byte value with state.
    pub fn get_data(&self, offset: u64) -> Option<&u8> {
        let index = offset - self.offset;
        self.data.get(index as usize)
    }

    /// Update page with changed data.
    pub fn update(&mut self, changes: &BTreeMap<u64, u8>) {
        self.changed = changes.keys().cloned().collect();
    }
}
