// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io;
use std::io::{Error, ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

/// Editable file.
pub struct File {
    /// File handle
    pub file: std::fs::File,
    /// Full path to the file.
    pub path: String,
    /// File size.
    pub size: u64,
    /// Data cache.
    cache_data: Vec<u8>,
    /// Start address of data cache.
    cache_start: u64,
}

impl File {
    /// Size of internal cache.
    const CACHE_SIZE: usize = 4096;

    /// Open file.
    ///
    /// # Arguments
    ///
    /// * `file` - file to open
    ///
    /// # Return value
    ///
    /// Self instance.
    pub fn open(file: &str) -> io::Result<Self> {
        let path = File::abs_path(file);
        // open file in read only mode
        let file = OpenOptions::new().read(true).open(&path)?;
        let meta = file.metadata()?;
        if meta.len() == 0 {
            return Err(Error::new(ErrorKind::UnexpectedEof, "File is empty"));
        }
        Ok(Self {
            file,
            path,
            size: meta.len(),
            cache_data: Vec::with_capacity(File::CACHE_SIZE),
            cache_start: 0,
        })
    }

    /// Read up to `size` bytes from file.
    ///
    /// # Arguments
    ///
    /// * `offset` - start offset
    /// * `size` - number of bytes to read
    ///
    /// # Return value
    ///
    /// File data.
    pub fn read(&mut self, offset: u64, size: usize) -> io::Result<Vec<u8>> {
        debug_assert!(offset < self.size);

        // read up to the end of file
        let size = size.min((self.size - offset) as usize);

        // update cache if needed
        let cache_miss = offset < self.cache_start
            || offset + size as u64 >= self.cache_start + self.cache_data.len() as u64;
        if cache_miss {
            let cache_size = File::CACHE_SIZE.min((self.size - offset) as usize);
            self.cache_data.resize(cache_size, 0);
            self.cache_start = offset;
            self.file.seek(SeekFrom::Start(offset))?;
            self.file.read_exact(&mut self.cache_data)?;
        }

        let start = (offset - self.cache_start) as usize;
        let end = start + size;
        Ok(self.cache_data[start..end].to_vec())
    }

    /// Write changes to the current file.
    ///
    /// # Arguments
    ///
    /// * `changes` - map of changes
    pub fn write(&mut self, changes: &BTreeMap<u64, u8>) -> io::Result<()> {
        // reopen file with the write permission
        let mut file = OpenOptions::new().write(true).open(self.path.clone())?;
        // write changes
        for (&offset, &value) in changes.iter() {
            file.seek(SeekFrom::Start(offset))?;
            file.write_all(&[value])?;
        }

        // reset cache
        self.cache_data.clear();

        Ok(())
    }

    /// Copy current file and write changes to it.
    ///
    /// # Arguments
    ///
    /// * `path` - path to the new file
    /// * `changes` - map of changes
    pub fn write_copy(&mut self, path: String, changes: &BTreeMap<u64, u8>) -> io::Result<()> {
        // reopen file with the write permission
        let path = File::abs_path(&path);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path.clone())?;

        let mut offset = 0;
        loop {
            let mut data = self.read(offset, 512)?;

            // apply changes
            for (&ch_offset, &ch_value) in changes.range(offset..offset + data.len() as u64) {
                data[(ch_offset - offset) as usize] = ch_value;
            }

            file.write_all(&data)?;
            offset += data.len() as u64;
            if offset >= self.size {
                break; //eof
            }
        }

        self.file = file;
        self.path = path;

        // reset cache
        self.cache_data.clear();

        Ok(())
    }

    /// Get absolute path to the file.
    ///
    /// # Arguments
    ///
    /// * `file` - path to the file
    ///
    /// # Return value
    ///
    /// Absolute path.
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
}
