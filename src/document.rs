// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::changes::ChangeList;
use super::config::Config;
use super::cursor::{Cursor, Direction, HalfByte, Place};
use super::file::File;
use super::view::View;
use std::io;
use std::path::Path;

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
        let file = File::open(Path::new(path))?;
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
    pub fn save_as(&mut self, path: &str) -> io::Result<()> {
        self.file.write_to(Path::new(&path))?;

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
        #[allow(clippy::cast_possible_truncation)]
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

    /// Show cursor.
    pub fn show_cursor(&self) {
        if let Some((mut x, y)) = self
            .view
            .get_position(self.cursor.offset, self.cursor.place == Place::Hex)
        {
            if self.cursor.half == HalfByte::Right {
                x += 1;
            }
            self.view.window.show_cursor(x, y);
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
            .copied()
            .collect();
    }
}
