// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::changes::ChangeList;
use super::config::Config;
use super::curses::Window;
use super::cursor::*;
use super::file::File;
use super::page::Page;
use super::view::View;
use std::io;

/// Editable document.
pub struct Document {
    /// Editable file.
    pub file: File,
    /// Change list.
    pub changes: ChangeList,
    /// Currently displayed page.
    pub page: Page,
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
            changes: ChangeList::new(),
            page: Page::new(),
            cursor: Cursor::new(),
            view: View::new(config, file_size),
        })
    }

    /// Write changes to the file.
    pub fn save(&mut self) -> io::Result<()> {
        self.file.write()?;

        // reset undo/redo buffer
        self.changes.reset();

        self.update_page(self.page.offset);

        Ok(())
    }

    /// Save current file with the new name.
    pub fn save_as(&mut self, path: String) -> io::Result<()> {
        self.file.write_copy(path)?;

        // reset undo/redo buffer
        self.changes.reset();

        self.update_page(self.page.offset);

        Ok(())
    }

    /// Find sequence inside the current file from the current position.
    pub fn find(
        &mut self,
        sequence: &[u8],
        backward: bool,
        progress: &mut dyn ProgressHandler,
    ) -> Option<u64> {
        let mut handled = 0;

        let step = 1024;
        let size = step + sequence.len() as i64;
        let mut offset = self.cursor.offset as i64;

        if backward {
            offset -= 1;
        } else {
            offset += 1;
        }

        let mut round = false;

        loop {
            // update progress info
            handled += step as u64;
            let percent = 100.0 / (self.file.size as f32) * handled as f32;
            if !progress.update(percent as u8) {
                return None; // aborted by user
            }

            if !backward {
                // forward search
                if offset as u64 >= self.file.size {
                    offset = 0;
                    round = true;
                }
            } else {
                // backward search
                if round && (offset as u64) < self.cursor.offset {
                    break;
                }
                offset -= size;
                if offset < 0 {
                    if self.file.size < size as u64 {
                        offset = 0;
                    } else {
                        offset = self.file.size as i64 - size;
                    }
                    round = true;
                }
            }

            let file_data = self.file.read(offset as u64, size as usize).unwrap();
            let mut window = file_data.windows(sequence.len());
            if !backward {
                if let Some(pos) = window.position(|wnd| wnd == sequence) {
                    return Some(offset as u64 + pos as u64);
                }
            } else if let Some(pos) = window.rposition(|wnd| wnd == sequence) {
                return Some(offset as u64 + pos as u64);
            }

            if !backward {
                offset += step;
                if round && offset as u64 >= self.cursor.offset {
                    break;
                }
            }
        }

        None
    }

    /// Move cursor.
    pub fn move_cursor(&mut self, dir: &Direction) {
        let new_base = self.cursor.move_to(dir, &self.page, self.file.size);
        if new_base != self.page.offset {
            self.update_page(new_base);
        }
    }

    /// Undo last change.
    pub fn undo(&mut self) {
        if let Some(change) = self.changes.undo() {
            self.update_page(self.page.offset);
            self.move_cursor(&Direction::Absolute(change.offset));
        }
    }

    /// Redo (opposite to Undo).
    pub fn redo(&mut self) {
        if let Some(change) = self.changes.redo() {
            self.update_page(self.page.offset);
            self.move_cursor(&Direction::Absolute(change.offset));
        }
    }

    /// Change data: replace byte value at the current cursor position.
    pub fn modify(&mut self, value: u8, mask: u8) {
        let index = (self.cursor.offset - self.page.offset) as usize;
        let old = self.page.data[index];
        let new = (old & !mask) | (value & mask);

        self.file.changes = self.changes.set(self.cursor.offset, old, new);
        self.update_page(self.page.offset);
    }

    /// Resize view and page.
    ///
    /// # Arguments
    ///
    /// * `parent` - parent window
    pub fn resize(&mut self, parent: Window) {
        self.view.resize(parent);
        self.page.lines = self.view.lines;
        self.page.columns = self.view.columns;
        let dir = Direction::Absolute(self.cursor.offset);
        let base = self.cursor.move_to(&dir, &self.page, self.file.size);
        self.update_page(base);
    }

    /// Update currently displayed page.
    fn update_page(&mut self, offset: u64) {
        debug_assert!(self.page.lines != 0 && self.page.columns != 0); // not initialized yet?
        debug_assert!(offset < self.file.size);

        self.page.offset = offset;
        self.page.data = self
            .file
            .read(offset, self.page.lines * self.page.columns)
            .unwrap();
        self.page.changed = self.file.changes.keys().cloned().collect();
    }
}

/// Progress handler interface for long time operations.
pub trait ProgressHandler {
    fn update(&mut self, percent: u8) -> bool;
}
