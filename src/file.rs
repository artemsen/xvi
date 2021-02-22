// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};

/// Edited file.
pub struct File {
    file: std::fs::File,
    pub name: String,
    pub size: u64,
}

impl File {
    /// Open file.
    pub fn open(path: &str) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new().read(true).open(&path)?;
        let meta = file.metadata()?;
        Ok(Self {
            file,
            name: String::from(path),
            size: meta.len(),
        })
    }

    /// Read up to max_size bytes from file.
    pub fn read(&mut self, offset: u64, max_size: usize) -> Result<Vec<u8>, std::io::Error> {
        debug_assert!(offset < self.size);

        let size = std::cmp::min((self.size - offset) as usize, max_size);
        let mut data = vec![0; size];
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(&mut data)?;

        Ok(data)
    }

    /// Save file.
    pub fn save(&mut self, changes: &BTreeMap<u64, u8>) -> Result<(), std::io::Error> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(self.name.clone())?;
        for (offset, value) in changes.iter() {
            file.seek(SeekFrom::Start(*offset))?;
            file.write_all(&[*value])?;
        }
        Ok(())
    }

    /// Save file with new name.
    pub fn save_as(
        &mut self,
        name: String,
        changes: &BTreeMap<u64, u8>,
    ) -> Result<(), std::io::Error> {
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
            for (&offset, &value) in changes.range(pos..pos + len as u64) {
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

        Ok(())
    }

    /// Find sequence inside file.
    pub fn find(
        &mut self,
        sequence: &[u8],
        start: u64,
        backward: bool,
        changes: &BTreeMap<u64, u8>,
    ) -> Option<u64> {
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

            // apply changes
            for (&pos, &val) in changes.range((offset as u64)..((offset + size) as u64)) {
                file_data[(pos - offset as u64) as usize] = val;
            }

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
}
