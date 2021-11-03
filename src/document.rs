// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::changes::ChangeList;
use super::config::Config;
use super::curses::Window;
use super::cursor::*;
use super::file::File;
use super::view::View;
use std::io;

/// Editable document.
pub struct Document {
    /// Editable file.
    pub file: File,
    /// Change list.
    pub changes: ChangeList,
    /// Cursor position within a page.
    pub cursor: Cursor,
    /// View of the document.
    pub view: View,
}

impl Document {
    /// Create new document instance.
    pub fn new(path: &str, config: &Config) -> io::Result<Self> {
        let file = File::open(path)?;
        let file_size = file.size;

        Ok(Self {
            file,
            changes: ChangeList::default(),
            cursor: Cursor::default(),
            view: View::new(config, file_size),
        })
    }

    /// File change handler.
    pub fn on_file_changed(&mut self, offset: u64) {
        self.view.max_offset = self.file.size;
        if !self.move_cursor(&Direction::Absolute(offset)) {
            self.update();
        }
    }

    /// Write changes to the file.
    pub fn save(&mut self) -> io::Result<()> {
        self.file.write()?;

        // reset undo/redo buffer
        self.changes.reset();

        self.update();

        Ok(())
    }

    /// Save current file with the new name.
    pub fn save_as(&mut self, path: String) -> io::Result<()> {
        self.file.write_to(path)?;

        // reset undo/redo buffer
        self.changes.reset();

        self.update();

        Ok(())
    }

    /// Move cursor.
    pub fn move_cursor(&mut self, dir: &Direction) -> bool {
        let new_base = self.cursor.move_to(dir, &self.view);
        let base_changed = new_base != self.view.offset;
        if base_changed {
            self.view.offset = new_base;
            self.update();
        }
        base_changed
    }

    /// Undo last change.
    pub fn undo(&mut self) {
        if let Some(change) = self.changes.undo() {
            if !self.move_cursor(&Direction::Absolute(change.offset)) {
                self.update();
            }
        }
    }

    /// Redo (opposite to Undo).
    pub fn redo(&mut self) {
        if let Some(change) = self.changes.redo() {
            if !self.move_cursor(&Direction::Absolute(change.offset)) {
                self.update();
            }
        }
    }

    /// Change data: replace byte value at the current cursor position.
    pub fn modify_cur(&mut self, value: u8, mask: u8) {
        let index = (self.cursor.offset - self.view.offset) as usize;
        let old = self.view.data[index];
        let new = (old & !mask) | (value & mask);

        self.changes.set(self.cursor.offset, old, new);
        self.update();
    }

    /// Change data: replace byte value at the specified position.
    pub fn modify_at(&mut self, offset: u64, value: u8) {
        let old = self.file.read(offset, 1).unwrap();
        let old = old[0];
        if old != value {
            self.changes.set(offset, old, value);
        }
    }

    /// Resize view.
    ///
    /// # Arguments
    ///
    /// * `parent` - parent window
    pub fn resize(&mut self, parent: Window) {
        self.view.resize(parent);
        if !self.move_cursor(&Direction::Absolute(self.cursor.offset)) {
            self.update();
        }
    }

    /// Update currently displayed page.
    pub fn update(&mut self) {
        debug_assert!(self.view.lines != 0 && self.view.columns != 0); // not initialized yet?

        self.file.changes = self.changes.get();

        self.view.data = self
            .file
            .read(self.view.offset, self.view.lines * self.view.columns)
            .unwrap();
        self.view.changes = self
            .file
            .changes
            .keys()
            .filter(|&o| {
                (self.view.offset..self.view.offset + (self.view.lines * self.view.columns) as u64)
                    .contains(o)
            })
            .cloned()
            .collect();
    }
}
