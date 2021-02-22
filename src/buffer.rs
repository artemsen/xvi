// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use std::collections::BTreeMap;

/// Buffer of changes with undo/redo support.
pub struct Buffer {
    /// Queue of changes.
    queue: Vec<Change>,
    /// undo/redo position (index of the next change).
    index: usize,
}

impl Buffer {
    /// Create new instance.
    pub fn new() -> Self {
        Self {
            queue: Vec::with_capacity(4096 / std::mem::size_of::<Change>()),
            index: 0,
        }
    }

    /// Reset buffer state.
    pub fn reset(&mut self) {
        self.queue.clear();
        self.index = 0;
    }

    /// Add single change.
    pub fn add(&mut self, offset: u64, old: u8, new: u8) {
        // try to update the last changed value if it in the same offset
        if let Some(last) = self.queue.last_mut() {
            if last.offset == offset {
                last.new = new;
                return;
            }
        }

        // reset forward changes by removing the tail
        if self.index != 0 {
            self.queue.truncate(self.index);
        }

        self.queue.push(Change { offset, old, new });
        self.index = self.queue.len();
    }

    /// Get map of actual changes: offset -> value.
    pub fn get(&self) -> BTreeMap<u64, u8> {
        let mut origins = BTreeMap::new();
        let mut changes = BTreeMap::new();
        for change in self.queue[0..self.index].iter() {
            origins.entry(change.offset).or_insert(change.old);
            changes.insert(change.offset, change.new);
        }
        // remove changes that restore origin values
        for (offset, origin) in origins.iter() {
            if origin == changes.get(offset).unwrap() {
                changes.remove(offset);
            }
        }
        changes
    }

    /// Undo the last change, returns offset of it.
    pub fn undo(&mut self) -> Option<Change> {
        if self.queue.is_empty() || self.index == 0 {
            None
        } else {
            self.index -= 1;
            Some(self.queue[self.index])
        }
    }

    /// Redo the next change, returns offset of it
    pub fn redo(&mut self) -> Option<Change> {
        if self.queue.is_empty() || self.index == self.queue.len() {
            None
        } else {
            self.index += 1;
            Some(self.queue[self.index - 1])
        }
    }
}

/// Single change
#[derive(Copy, Clone)]
pub struct Change {
    pub offset: u64,
    pub old: u8,
    pub new: u8,
}
