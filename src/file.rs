// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};

pub struct File {
    file: std::fs::File,
    pub name: String,
    pub size: u64,
}

impl File {
    /// Open file for editing
    pub fn open(path: &str) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new().read(true).write(true).open(&path)?;
        let meta = file.metadata()?;
        Ok(Self {
            file,
            name: String::from(path),
            size: meta.len(),
        })
    }

    /// Read up to max_size bytes from file
    pub fn read(&mut self, offset: u64, max_size: usize) -> Result<Vec<u8>, std::io::Error> {
        debug_assert!(offset < self.size);

        let size = std::cmp::min((self.size - offset) as usize, max_size);
        let mut data = vec![0; size];
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(&mut data)?;

        Ok(data)
    }

    /// Save file
    pub fn save(&mut self, changes: &BTreeMap<u64, u8>) -> Result<(), std::io::Error> {
        for (offset, value) in changes.iter() {
            self.file.seek(SeekFrom::Start(*offset))?;
            self.file.write_all(&[*value])?;
        }
        Ok(())
    }

    /// Save as
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
            // read nex block
            let len = self.file.read(&mut buf)?;
            if len == 0 {
                break; //eof
            }

            // make changes
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
}
