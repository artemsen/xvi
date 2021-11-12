// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::changes::ChangeList;
use super::config::Config;
use super::cursor::{Cursor, Direction, HalfByte, Place};
use super::file::{File, ProgressHandler};
use super::view::View;
use std::collections::BTreeSet;
use std::io;
use std::ops::Range;
use std::path::Path;

/// Holder of editable documents, implements editor business logic.
pub struct Editor {
    /// Editable documents.
    documents: Vec<Document>,
    /// Index of currently selected document.
    current: usize,
}

impl Editor {
    /// Create group of documents.
    ///
    /// # Arguments
    ///
    /// * `files` - files to open
    /// * `config` - app configuration
    ///
    /// # Return value
    ///
    /// Group instance.
    pub fn new(files: &[String], config: &Config) -> io::Result<Self> {
        debug_assert!(!files.is_empty());

        // open documents
        let mut documents = Vec::with_capacity(files.len());
        for file in files {
            documents.push(Document::new(Path::new(file), config)?);
        }

        Ok(Self {
            documents,
            current: 0,
        })
    }

    /// Get currently focused document.
    pub fn current(&self) -> &Document {
        &self.documents[self.current]
    }

    /// Get number of opened documents.
    ///
    /// # Return value
    ///
    /// Number of opened documents.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Get current cursor offset and list of opened files.
    /// Used for history saving.
    ///
    /// # Return value
    ///
    /// Cursor offset and list with absolute path of currently edited files.
    pub fn get_files(&self) -> (u64, Vec<String>) {
        (
            self.documents[self.current].cursor.offset,
            self.documents
                .iter()
                .map(|doc| doc.file.path.to_string())
                .collect(),
        )
    }

    /// Resize the current workspace.
    ///
    /// # Arguments
    ///
    /// * `width` - new width of workspace
    /// * `height` - new height of workspace
    pub fn resize(&mut self, width: usize, height: usize) {
        // height of a single view (lines per document)
        let lpd = height / self.documents.len();
        let last = self.documents.len() - 1;

        // resize views
        for (index, doc) in self.documents.iter_mut().enumerate() {
            let y = index * lpd;
            // enlarge last window to fit the entire workspace
            let height = if index == last {
                height - lpd * index
            } else {
                lpd
            };
            doc.view.resize(y, width, height);
        }

        self.refresh();

        let current = &self.documents[self.current];
        let view_offset = current.view.offset;
        let cursor_offset = current.cursor.offset;
        self.move_cursor(&Direction::Absolute(cursor_offset, view_offset));
    }

    /// Draw documents in current workspace.
    pub fn draw(&self) {
        self.documents.iter().for_each(|doc| doc.view.draw(doc));

        // show cursor
        let current = &self.documents[self.current];
        if let Some((mut x, y)) = current
            .view
            .get_position(current.cursor.offset, current.cursor.place == Place::Hex)
        {
            if current.cursor.half == HalfByte::Right {
                x += 1;
            }
            current.view.window.show_cursor(x, y);
        }
    }

    /// Move cursor.
    ///
    /// # Arguments
    ///
    /// * `dir` - move direction
    pub fn move_cursor(&mut self, dir: &Direction) {
        // move cursor in the current document
        let current = &mut self.documents[self.current];
        let mut update = current.move_cursor(dir);
        let view_offset = current.view.offset;
        let cursor_offset = current.cursor.offset;

        // move cursor in other documents
        let index = self.current;
        for (_, doc) in self
            .documents
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i != index)
        {
            update |= doc.move_cursor(&Direction::Absolute(cursor_offset, view_offset));
        }

        // refresh view data
        if update {
            self.refresh();
        }
    }

    /// Switch focus between documents and fields:
    /// current hex -> current ascii -> next hex -> next ascii
    ///
    /// # Arguments
    ///
    /// * `dir` - switch direction
    pub fn switch_focus(&mut self, dir: &Focus) {
        let current = &self.documents[self.current];
        let has_ascii = current.view.ascii_table.is_some();

        match dir {
            Focus::NextField => {
                if current.cursor.place == Place::Hex && has_ascii {
                    self.documents
                        .iter_mut()
                        .for_each(|doc| doc.cursor.set_place(Place::Ascii));
                } else {
                    self.current += 1;
                    if self.current == self.documents.len() {
                        self.current = 0;
                    }
                    if current.cursor.place == Place::Ascii {
                        self.documents
                            .iter_mut()
                            .for_each(|doc| doc.cursor.set_place(Place::Hex));
                    }
                }
            }
            Focus::PreviousField => {
                if current.cursor.place == Place::Ascii {
                    self.documents
                        .iter_mut()
                        .for_each(|doc| doc.cursor.set_place(Place::Hex));
                } else {
                    if self.current > 0 {
                        self.current -= 1;
                    } else {
                        self.current = self.documents.len() - 1;
                    }
                    if current.cursor.place == Place::Hex && has_ascii {
                        self.documents
                            .iter_mut()
                            .for_each(|doc| doc.cursor.set_place(Place::Ascii));
                    }
                }
            }
            Focus::NextDocument => {
                if self.current + 1 < self.documents.len() {
                    self.current += 1;
                }
            }
            Focus::PreviousDocument => {
                if self.current > 0 {
                    self.current -= 1;
                }
            }
            Focus::DocumentIndex(index) => {
                debug_assert!(*index < self.documents.len());
                self.current = *index;
            }
        }
    }

    /// Change data in the currently focused document.
    ///
    /// # Arguments
    ///
    /// * `offset` - offset of the byte value to change
    /// * `value` - new value
    /// * `mask` - mask of the new value
    pub fn change(&mut self, offset: u64, value: u8, mask: u8) {
        self.documents[self.current].change(offset, value, mask);
        self.refresh();
    }

    /// Undo last change in the currently focused document.
    pub fn undo(&mut self) {
        if let Some(change) = self.documents[self.current].changes.undo() {
            let offset = self.documents[self.current].view.offset;
            self.move_cursor(&Direction::Absolute(change.offset, offset));
            self.refresh();
        }
    }

    /// Redo (opposite to Undo) for the currently focused document.
    pub fn redo(&mut self) {
        if let Some(change) = self.documents[self.current].changes.redo() {
            let offset = self.documents[self.current].view.offset;
            self.move_cursor(&Direction::Absolute(change.offset, offset));
            self.refresh();
        }
    }

    /// Jump to the closest change.
    ///
    /// # Arguments
    ///
    /// * `forward` - search direction
    pub fn closest_change(&mut self, forward: bool) {
        let current = &self.documents[self.current];

        // offset of the closest changed byte
        let changed = if forward {
            if let Some((offset, _)) = current
                .file
                .changes
                .range((current.cursor.offset + 1)..u64::MAX)
                .min()
            {
                Some(offset)
            } else {
                None
            }
        } else if let Some((offset, _)) = current.file.changes.range(0..current.cursor.offset).max()
        {
            Some(offset)
        } else {
            None
        };

        if let Some(&offset) = changed {
            let base_offset = current.view.offset;
            self.move_cursor(&Direction::Absolute(offset, base_offset));
        }
    }

    /// Save currently focused document.
    pub fn save(&mut self) -> io::Result<()> {
        let current = &mut self.documents[self.current];
        current.file.save()?;
        current.changes.reset();
        self.refresh();
        Ok(())
    }

    /// Save currently focused document with the new name.
    ///
    /// # Arguments
    ///
    /// * `file` - path to the new file
    /// * `progress` - long time operation handler
    pub fn save_as(&mut self, file: &Path, progress: &mut dyn ProgressHandler) -> io::Result<()> {
        let current = &mut self.documents[self.current];
        current.file.save_as(file, progress)?;
        current.changes.reset();
        self.refresh();
        Ok(())
    }

    /// Find sequence inside the currently focused document.
    ///
    /// # Arguments
    ///
    /// * `start` - start address
    /// * `sequence` - sequence to find
    /// * `backward` - search direction
    /// * `progress` - long time operation handler
    ///
    /// # Return value
    ///
    /// Operation status.
    pub fn find(
        &mut self,
        start: u64,
        sequence: &[u8],
        backward: bool,
        progress: &mut dyn ProgressHandler,
    ) -> io::Result<()> {
        let current = &mut self.documents[self.current];
        match current.file.find(start, sequence, backward, progress) {
            Ok(offset) => {
                if offset != u64::MAX {
                    let view_offset = current.view.offset;
                    self.move_cursor(&Direction::Absolute(offset, view_offset));
                    self.refresh();
                }
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    /// Fill range in the currently focused document.
    ///
    /// # Arguments
    ///
    /// * `range` - range to fill
    /// * `pattern` - pattern to use
    pub fn fill(&mut self, range: &Range<u64>, pattern: &[u8]) {
        debug_assert!(!range.is_empty());
        debug_assert!(!pattern.is_empty());

        let current = &mut self.documents[self.current];
        let mut pattern_pos = 0;
        for offset in range.start..range.end {
            current.change(offset, pattern[pattern_pos], 0xff);
            pattern_pos += 1;
            if pattern_pos == pattern.len() {
                pattern_pos = 0;
            }
        }

        let offset = current.view.offset;
        self.move_cursor(&Direction::Absolute(range.end, offset));
        self.refresh();
    }

    /// Insert bytes at specified offset.
    ///
    /// # Arguments
    ///
    /// * `offset` - start offset
    /// * `size` - number of bytes to insert
    /// * `pattern` - pattern to fill
    /// * `progress` - long time operation handler
    ///
    /// # Return value
    ///
    /// Operation status.
    pub fn insert(
        &mut self,
        offset: u64,
        size: u64,
        pattern: &[u8],
        progress: &mut dyn ProgressHandler,
    ) -> io::Result<()> {
        let current = &mut self.documents[self.current];
        debug_assert!(!current.file.is_modified());
        debug_assert!(offset <= current.file.size);

        current.file.insert(offset, size, pattern, progress)?;

        current.view.max_offset = current.file.size;

        let view_offset = current.view.offset;
        self.move_cursor(&Direction::Absolute(offset + size, view_offset));
        self.refresh();

        Ok(())
    }

    /// Cut out the specified range.
    ///
    /// # Arguments
    ///
    /// * `range` - range to cut out
    /// * `progress` - long time operation handler
    pub fn cut(
        &mut self,
        range: &Range<u64>,
        progress: &mut dyn ProgressHandler,
    ) -> io::Result<()> {
        let current = &mut self.documents[self.current];
        debug_assert!(!current.file.is_modified());

        current.file.cut(range, progress)?;

        current.view.max_offset = current.file.size;

        let offset = current.view.offset;
        self.move_cursor(&Direction::Absolute(range.start, offset));
        self.refresh();

        Ok(())
    }

    /// Setup via GUI.
    pub fn config_changed(&mut self, config: &Config) {
        for doc in &mut self.documents {
            doc.view.fixed_width = config.fixed_width;
            doc.view.ascii_table = config.ascii_table;
            if doc.view.ascii_table.is_none() {
                doc.cursor.set_place(Place::Hex);
            }
            doc.view.reinit();
        }
        let cursor = self.documents[self.current].cursor.offset;
        let base = self.documents[self.current].view.offset;
        self.move_cursor(&Direction::Absolute(cursor, base));
        self.refresh();
    }

    /// Refresh documents buffers: data cache, changed set, diff etc.
    fn refresh(&mut self) {
        // refresh buffer for all documents
        self.documents.iter_mut().for_each(|doc| doc.refresh());

        // update diff markers
        if self.documents.len() > 1 {
            for index in 0..self.documents.len() {
                let mut diff = BTreeSet::new();
                let doc = &mut self.documents[index];
                let offset = doc.view.offset;
                let size = doc.view.lines * doc.view.columns;
                let data_l = doc.file.read(offset, size).unwrap();
                for (_, doc_r) in self
                    .documents
                    .iter_mut()
                    .enumerate()
                    .filter(|(i, _)| *i != index)
                {
                    let data_r = if offset >= doc_r.file.size {
                        vec![]
                    } else {
                        doc_r.file.read(offset, size).unwrap()
                    };
                    for (index, byte_l) in data_l.iter().enumerate() {
                        let mut equal = false;
                        if let Some(byte_r) = data_r.get(index) {
                            equal = byte_l == byte_r;
                        }
                        if !equal {
                            diff.insert(offset + index as u64);
                        }
                    }
                }
                self.documents[index].view.differs = diff;
            }
        }
    }
}

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
    ///
    /// # Arguments
    ///
    /// * `path` - path to the file to open
    /// * `config` - app configuration
    ///
    /// # Return value
    ///
    /// Document instance.
    fn new(path: &Path, config: &Config) -> io::Result<Self> {
        let file = File::open(path)?;
        let file_size = file.size;

        Ok(Self {
            file,
            changes: ChangeList::default(),
            cursor: Cursor::default(),
            view: View::new(config, file_size),
        })
    }

    /// Move cursor.
    ///
    /// # Arguments
    ///
    /// * `dir` - move direction
    ///
    /// # Return value
    ///
    /// `true` if new base address was set
    fn move_cursor(&mut self, dir: &Direction) -> bool {
        let new_base = self.cursor.move_to(dir, &self.view);
        let base_changed = new_base != self.view.offset;
        if base_changed {
            self.view.offset = new_base;
            self.refresh();
        }
        base_changed
    }

    /// Update currently displayed page.
    fn refresh(&mut self) {
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

    /// Change data in the document.
    ///
    /// # Arguments
    ///
    /// * `offset` - offset of the byte value to change
    /// * `value` - new value
    /// * `mask` - mask of the new value
    fn change(&mut self, offset: u64, value: u8, mask: u8) {
        debug_assert!(mask == 0x0f || mask == 0xf0 || mask == 0xff);

        // get currently set value
        let old = if let Some(val) = self.changes.last(offset) {
            val
        } else {
            self.file.read(offset, 1).unwrap()[0]
        };

        // set new value
        let new = (old & !mask) | (value & mask);
        if old != value {
            self.changes.set(offset, old, new);
        }
    }
}

/// Focus switch direction.
pub enum Focus {
    NextField,
    PreviousField,
    NextDocument,
    PreviousDocument,
    DocumentIndex(usize),
}
