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
    file: std::fs::File,
    /// Full path to the file.
    pub path: String,
    /// File size.
    pub size: u64,
    /// Cached map of real changes (offset -> new byte value).
    pub changes: BTreeMap<u64, u8>,
    /// Data cache.
    cache: Cache,
}

impl File {
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
            changes: BTreeMap::new(),
            cache: Cache::new(),
        })
    }

    /// Check if file is modofied.
    pub fn is_modified(&self) -> bool {
        !self.changes.is_empty()
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
        if !self.cache.has(offset, size) {
            let cache_size = Cache::SIZE.min((self.size - offset) as usize);
            self.cache.data.resize(cache_size, 0);
            self.cache.start = offset;
            self.file.seek(SeekFrom::Start(offset))?;
            self.file.read_exact(&mut self.cache.data)?;
        }

        // get file data
        let start = (offset - self.cache.start) as usize;
        let end = start + size;
        let mut data = self.cache.data[start..end].to_vec();

        // apply changes
        for (&addr, &value) in self.changes.range(offset..offset + size as u64) {
            let index = (addr - offset) as usize;
            data[index] = value;
        }

        Ok(data)
    }

    /// Write changes to the current file.
    pub fn write(&mut self) -> io::Result<()> {
        // reopen file with the write permission
        let mut file = OpenOptions::new().write(true).open(self.path.clone())?;
        // write changes
        for (&offset, &value) in self.changes.iter() {
            file.seek(SeekFrom::Start(offset))?;
            file.write_all(&[value])?;
        }

        // reset
        self.cache.data.clear();
        self.changes.clear();

        Ok(())
    }

    /// Copy current file and write changes to it.
    ///
    /// # Arguments
    ///
    /// * `path` - path to the new file
    /// * `changes` - map of changes
    pub fn write_copy(&mut self, path: String) -> io::Result<()> {
        // reopen file with the write permission
        let path = File::abs_path(&path);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path.clone())?;

        let mut offset = 0;
        loop {
            let data = self.read(offset, 512)?;
            file.write_all(&data)?;
            offset += data.len() as u64;
            if offset >= self.size {
                break; //eof
            }
        }

        self.file = file;
        self.path = path;

        // reset
        self.cache.data.clear();
        self.changes.clear();

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

/// Data cache.
struct Cache {
    /// Cache buffer.
    data: Vec<u8>,
    /// Start address of the cache.
    start: u64,
}

impl Cache {
    /// Size of the cache.
    const SIZE: usize = 4096;

    /// Create new cache instance.
    fn new() -> Self {
        Self {
            data: Vec::with_capacity(Cache::SIZE),
            start: 0,
        }
    }

    /// Check if cache contains specified range.
    fn has(&self, offset: u64, size: usize) -> bool {
        offset >= self.start && offset + (size as u64) < self.start + self.data.len() as u64
    }
}
