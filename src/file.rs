// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::io;
use std::io::{Read, Seek, SeekFrom, Write};

/// Edited file.
pub struct File {
    /// Full path to the file.
    pub name: String,
    /// File size.
    pub size: u64,

    /// File handle.
    file: std::fs::File,

    /// Data cache.
    cache_data: Vec<u8>,
    /// Start address of data cache.
    cache_start: u64,

    /// Queue of changes.
    ch_queue: Vec<Change>,
    /// undo/redo position (index of the next change).
    ch_index: usize,
    /// Map of changes (offset -> new byte value).
    ch_map: BTreeMap<u64, u8>,
}

impl File {
    const CACHE_SIZE: usize = 4096;

    /// Open file.
    pub fn open(path: &str) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).open(&path)?;
        let meta = file.metadata()?;
        Ok(Self {
            name: String::from(path),
            size: meta.len(),
            file,
            cache_data: Vec::new(),
            cache_start: 0,
            ch_queue: Vec::new(),
            ch_index: 0,
            ch_map: BTreeMap::new(),
        })
    }

    /// Save file.
    pub fn save(&mut self) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(self.name.clone())?;
        for (&offset, &value) in self.ch_map.iter() {
            file.seek(SeekFrom::Start(offset))?;
            file.write_all(&[value])?;
        }

        self.ch_queue.clear();
        self.ch_index = 0;
        self.ch_map.clear();

        Ok(())
    }

    /// Save file with new name.
    pub fn save_as(&mut self, name: String) -> io::Result<()> {
        let mut buf = [0; 512];

        let mut new_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(name.clone())?;

        self.file.seek(SeekFrom::Start(0))?;
        let mut pos = 0;
        loop {
            // read next block
            let len = self.file.read(&mut buf)?;
            if len == 0 {
                break; //eof
            }

            // apply changes
            for (&offset, &value) in self.ch_map.range(pos..pos + len as u64) {
                buf[(offset - pos) as usize] = value;
            }

            // write
            new_file.write_all(&buf[0..len])?;

            pos += len as u64;
        }

        self.name = name;
        self.file = new_file;
        let meta = self.file.metadata()?;
        self.size = meta.len();

        self.ch_queue.clear();
        self.ch_index = 0;
        self.ch_map.clear();

        Ok(())
    }

    /// Find sequence inside file.
    pub fn find(&mut self, sequence: &[u8], start: u64, backward: bool) -> Option<u64> {
        let step = 1024;
        let size = step + sequence.len() as i64;
        let mut offset = start as i64;

        if !backward {
            offset += 1;
        } else {
            offset -= 1;
        }

        let mut round = false;

        loop {
            if !backward {
                // forward search
                if offset as u64 >= self.size {
                    offset = 0;
                    round = true;
                }
            } else {
                // backward search
                if round && (offset as u64) < start {
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

            let mut file_data = self.read(offset as u64, size as usize).unwrap();
            self.apply(offset as u64, &mut file_data);

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
                if round && offset as u64 >= start {
                    break;
                }
            }
        }

        None
    }

    /// Read up to `size` bytes from file.
    fn read(&mut self, offset: u64, size: usize) -> io::Result<Vec<u8>> {
        debug_assert!(offset < self.size);

        // read up to the end of file
        let size = std::cmp::min((self.size - offset) as usize, size);
        let mut data = vec![0; size];
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(&mut data)?;

        Ok(data)
    }

    /// Apply changes to array of raw data.
    fn apply(&self, offset: u64, data: &mut Vec<u8>) {
        let end_offset = offset + data.len() as u64;
        for (&addr, &value) in self.ch_map.range(offset..end_offset) {
            let index = (addr - offset) as usize;
            data[index] = value;
        }
    }

    /// Get addresses of modified bytes.
    pub fn get_modified(&self) -> BTreeSet<u64> {
        let mut offsets = BTreeSet::new();
        for &offset in self.ch_map.keys() {
            offsets.insert(offset);
        }
        offsets
    }

    /// Check if file is modified.
    pub fn is_modified(&self) -> bool {
        !self.ch_map.is_empty()
    }

    /// Get data from file.
    /// Returns array with applied changes.
    pub fn get(&mut self, offset: u64, size: usize) -> io::Result<Vec<u8>> {
        debug_assert!(offset < self.size);

        let size = std::cmp::min((self.size - offset) as usize, size);

        // update cache
        let cache_miss = offset < self.cache_start
            || offset + size as u64 >= self.cache_start + self.cache_data.len() as u64;
        if cache_miss {
            self.cache_data = self.read(offset, std::cmp::max(size, File::CACHE_SIZE))?;
            self.cache_start = offset;
        }

        let start = (offset - self.cache_start) as usize;
        let end = start + size;
        let mut data = self.cache_data[start..end].to_vec();
        self.apply(offset, &mut data);

        Ok(data)
    }

    /// Modify single byte.
    pub fn set(&mut self, offset: u64, old: u8, new: u8) {
        // try to update the last changed value if it in the same offset
        if let Some(last) = self.ch_queue.last_mut() {
            if last.offset == offset {
                last.new = new;
                self.refresh();
                return;
            }
        }

        // reset forward changes by removing the tail
        if self.ch_index != 0 {
            self.ch_queue.truncate(self.ch_index);
        }

        self.ch_queue.push(Change { offset, old, new });
        self.ch_index = self.ch_queue.len();
        self.refresh();
    }

    /// Undo the last byte change, returns offset of it.
    pub fn undo(&mut self) -> Option<Change> {
        if self.ch_queue.is_empty() || self.ch_index == 0 {
            None
        } else {
            self.ch_index -= 1;
            self.refresh();
            Some(self.ch_queue[self.ch_index])
        }
    }

    /// Redo the next byte change, returns offset of it.
    pub fn redo(&mut self) -> Option<Change> {
        if self.ch_queue.is_empty() || self.ch_index == self.ch_queue.len() {
            None
        } else {
            self.ch_index += 1;
            self.refresh();
            Some(self.ch_queue[self.ch_index - 1])
        }
    }

    /// Update map of actual changes.
    fn refresh(&mut self) {
        self.ch_map.clear();
        let mut origins = BTreeMap::new();
        for change in self.ch_queue[0..self.ch_index].iter() {
            origins.entry(change.offset).or_insert(change.old);
            self.ch_map.insert(change.offset, change.new);
        }
        // remove changes that restore origin values
        for (offset, origin) in origins.iter() {
            if origin == self.ch_map.get(offset).unwrap() {
                self.ch_map.remove(offset);
            }
        }
    }
}

/// Single change.
#[derive(Copy, Clone)]
pub struct Change {
    pub offset: u64,
    pub old: u8,
    pub new: u8,
}
