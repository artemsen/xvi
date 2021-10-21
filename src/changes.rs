// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use std::collections::BTreeMap;

/// Modification of single byte.
#[derive(Copy, Clone)]
pub struct ByteChange {
    pub offset: u64,
    pub old: u8,
    pub new: u8,
}

/// List of bytes modifications.
pub struct ChangeList {
    /// List of changes.
    changes: Vec<ByteChange>,
    /// Current position in the queue.
    index: usize,
    /// Cached map of real changes (offset -> new byte value).
    pub real: BTreeMap<u64, u8>,
}

impl ChangeList {
    /// Create new change list instance.
    pub fn new() -> Self {
        Self {
            changes: Vec::with_capacity(64),
            index: 0,
            real: BTreeMap::new(),
        }
    }

    /// Make change: set new value for the byte at specified position.
    ///
    /// # Arguments
    ///
    /// * `offset` - address of the byte to modify
    /// * `old` - origin value of the byte
    /// * `new` - new value of the byte
    pub fn set(&mut self, offset: u64, old: u8, new: u8) {
        // try to update the last changed value if it in the same offset
        if let Some(last) = self.changes.last_mut() {
            if last.offset == offset {
                last.new = new;
                self.refresh();
                return;
            }
        }

        // reset forward changes by removing the tail
        if self.index != 0 {
            self.changes.truncate(self.index);
        }

        self.changes.push(ByteChange { offset, old, new });
        self.index = self.changes.len();
        self.refresh();
    }

    /// Undo the last change.
    ///
    /// Returns description of the undone change.
    pub fn undo(&mut self) -> Option<ByteChange> {
        if self.changes.is_empty() || self.index == 0 {
            None
        } else {
            self.index -= 1;
            self.refresh();
            Some(self.changes[self.index])
        }
    }

    /// Redo the next change.
    ///
    /// Returns description of the applied change.
    pub fn redo(&mut self) -> Option<ByteChange> {
        if self.changes.is_empty() || self.index == self.changes.len() {
            None
        } else {
            self.index += 1;
            self.refresh();
            Some(self.changes[self.index - 1])
        }
    }

    /// Check if the queue modifies source data.
    ///
    /// Returns `true` if source data was modified.
    pub fn has_changes(&self) -> bool {
        !self.real.is_empty()
    }

    /// Reset changes.
    pub fn reset(&mut self) {
        self.changes.clear();
        self.index = 0;
        self.real.clear();
    }

    /// Refresh the map of real changes.
    fn refresh(&mut self) {
        self.real.clear();
        let mut origins = BTreeMap::new();
        for change in self.changes[0..self.index].iter() {
            origins.entry(change.offset).or_insert(change.old);
            self.real.insert(change.offset, change.new);
        }
        // remove changes that restore origin values
        for (offset, origin) in origins.iter() {
            if origin == self.real.get(offset).unwrap() {
                self.real.remove(offset);
            }
        }
    }
}

#[test]
fn test_changesqueue() {
    let mut ch = ChangeList::new();

    ch.set(0x1234, 1, 2);
    ch.set(0x1235, 3, 4);
    ch.set(0x1235, 4, 5);
    ch.set(0x1235, 5, 6);
    assert_eq!(ch.real.len(), 2);
    assert_eq!(*ch.real.get(&0x1234).unwrap(), 2);
    assert_eq!(*ch.real.get(&0x1235).unwrap(), 6);

    ch.set(0x1234, 2, 1); // restore origin
    assert_eq!(ch.real.len(), 1);
    assert_eq!(*ch.real.get(&0x1235).unwrap(), 6);

    ch.undo();
    assert_eq!(ch.real.len(), 2);
    assert!(ch.has_changes());
    ch.undo();
    assert_eq!(ch.real.len(), 1);
    ch.undo();
    assert_eq!(ch.real.len(), 0);
    assert!(!ch.has_changes());
    ch.undo();

    ch.redo();
    assert!(ch.has_changes());
    assert_eq!(ch.real.len(), 1);

    ch.reset();
    assert!(!ch.has_changes());
}
