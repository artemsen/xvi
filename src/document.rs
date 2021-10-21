// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::changes::ChangeList;
use super::cursor::*;
use super::page::Page;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::{Error, ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

/// Editable document.
pub struct Document {
    /// Editable file.
    pub file: File,
    /// Absolute path to the file.
    pub path: String,
    /// File size.
    pub size: u64,
    /// Change list.
    pub changes: ChangeList,
    /// Currently displayed page.
    pub page: Page,
    /// Cursor position within a page.
    pub cursor: Cursor,
}

impl Document {
    /// Create new document instance.
    pub fn new(path: &str) -> io::Result<Self> {
        let path = Document::abs_path(path);
        // open file in read only mode
        let file = OpenOptions::new().read(true).open(&path)?;
        let meta = file.metadata()?;
        if meta.len() == 0 {
            return Err(Error::new(ErrorKind::UnexpectedEof, "File is empty"));
        }
        // create document instance
        Ok(Self {
            file,
            path,
            size: meta.len(),
            changes: ChangeList::new(),
            page: Page::new(),
            cursor: Cursor::new(),
        })
    }

    /// Write changes to the file.
    pub fn save(&mut self) -> io::Result<()> {
        // reopen file with the write permission
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(self.path.clone())?;
        for (&offset, &value) in self.changes.real.iter() {
            file.seek(SeekFrom::Start(offset))?;
            file.write_all(&[value])?;
        }

        // reset undo/redo buffer
        self.changes.reset();

        self.update_page(self.page.offset);

        Ok(())
    }

    /// Save current file with the new name.
    pub fn save_as(&mut self, path: String) -> io::Result<()> {
        let mut new_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path.clone())?;
        let mut pos = 0;
        loop {
            let data = self.get_data(pos, 512)?;
            new_file.write_all(&data)?;
            pos += data.len() as u64;
            if pos >= self.size {
                break; //eof
            }
        }

        // update file info
        self.file = new_file;
        self.path = Document::abs_path(&path);

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
            let percent = 100.0 / (self.size as f32) * handled as f32;
            if !progress.update(percent as u8) {
                return None; // aborted by user
            }

            if !backward {
                // forward search
                if offset as u64 >= self.size {
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
                    if self.size < size as u64 {
                        offset = 0;
                    } else {
                        offset = self.size as i64 - size;
                    }
                    round = true;
                }
            }

            let file_data = self.get_data(offset as u64, size as usize).unwrap();
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
    pub fn move_cursor(&mut self, dir: Direction) {
        let new_base = self.cursor.move_to(dir, &self.page, self.size);
        if new_base != self.page.offset {
            self.update_page(new_base);
        }
    }

    /// Undo last change.
    pub fn undo(&mut self) {
        if let Some(change) = self.changes.undo() {
            self.move_cursor(Direction::Absolute(change.offset));
        }
    }

    /// Redo (opposite to Undo).
    pub fn redo(&mut self) {
        if let Some(change) = self.changes.redo() {
            self.move_cursor(Direction::Absolute(change.offset));
        }
    }

    /// Change data: replace byte value at the current cursor position.
    pub fn modify(&mut self, value: u8, mask: u8) {
        let index = (self.cursor.offset - self.page.offset) as usize;
        let old = self.page.data[index];
        let new = (old & !mask) | (value & mask);

        self.changes.set(self.cursor.offset, old, new);
        self.update_page(self.page.offset);
    }

    /// Get absolute path to the file.
    fn abs_path(file: &str) -> String {
        if let Ok(path) = PathBuf::from(file).canonicalize() {
            if let Ok(path) = path.into_os_string().into_string() {
                path
            } else {
                file.to_string()
            }
        } else {
            file.to_string()
        }
    }

    /// Get file data with applied local changes.
    fn get_data(&mut self, offset: u64, size: usize) -> io::Result<Vec<u8>> {
        debug_assert!(offset < self.size);

        // read up to the end of file
        let size = std::cmp::min((self.size - offset) as usize, size);
        let mut data = vec![0; size];
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(&mut data)?;

        // apply changes
        let end_offset = offset + data.len() as u64;
        for (&addr, &value) in self.changes.real.range(offset..end_offset) {
            let index = (addr - offset) as usize;
            data[index] = value;
        }

        Ok(data)
    }

    /// Resize page.
    ///
    /// # Arguments
    ///
    /// * `lines` - number of lines per page
    /// * `columns` - number of bytes per line
    pub fn resize_page(&mut self, lines: usize, columns: usize) {
        self.page.lines = lines;
        self.page.columns = columns;
        let dir = Direction::Absolute(self.cursor.offset);
        let base = self.cursor.move_to(dir, &self.page, self.size);
        self.update_page(base);
    }

    /// Update currently displayed page.
    fn update_page(&mut self, offset: u64) {
        debug_assert!(self.page.lines != 0 && self.page.columns != 0); // not initialized yet?

        self.page.offset = offset;
        self.page.data = self
            .get_data(offset, self.page.lines * self.page.columns)
            .unwrap();
        self.page.update(&self.changes.real);
    }
}

/// Progress handler interface for long time operations.
pub trait ProgressHandler {
    fn update(&mut self, percent: u8) -> bool;
}
