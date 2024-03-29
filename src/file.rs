// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::Path;

/// Editable file.
pub struct File {
    /// File handle
    file: std::fs::File,
    /// Absolute path to the file.
    pub path: String,
    /// File size.
    pub size: u64,
    /// Cached map of real changes (offset -> new byte value).
    pub changes: BTreeMap<u64, u8>,
    /// Data cache.
    cache: Cache,
}

impl File {
    /// Size of the block for read/write operations.
    const BLOCK_SIZE: usize = Cache::SIZE;

    /// Open file.
    ///
    /// # Arguments
    ///
    /// * `file` - file to open
    ///
    /// # Return value
    ///
    /// Self instance.
    pub fn open(file: &Path) -> Result<Self> {
        let path = std::fs::canonicalize(file)?;
        if !path.is_file() {
            return Err(Error::new(ErrorKind::InvalidData, "Not a file"));
        }

        // open file in read only mode
        let file = OpenOptions::new().read(true).open(&path)?;
        let meta = file.metadata()?;
        if meta.len() == 0 {
            return Err(Error::new(ErrorKind::UnexpectedEof, "File is empty"));
        }
        Ok(Self {
            file,
            path: path.into_os_string().into_string().unwrap(),
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
    pub fn read(&mut self, offset: u64, size: usize) -> Result<Vec<u8>> {
        debug_assert!(offset < self.size);

        // read up to the end of file
        #[allow(clippy::cast_possible_truncation)]
        let max_size = (self.size - offset) as usize;
        let size = size.min(max_size);

        // update cache if needed
        if !self.cache.has(offset, size) {
            let cache_size = Cache::SIZE.min(max_size).max(size);
            self.cache.data.resize(cache_size, 0);
            self.cache.start = offset;
            self.file.seek(SeekFrom::Start(offset))?;
            self.file.read_exact(&mut self.cache.data)?;
        }

        // get file data
        #[allow(clippy::cast_possible_truncation)]
        let start = (offset - self.cache.start) as usize;
        let end = start + size;
        let mut data = self.cache.data[start..end].to_vec();

        // apply changes
        for (&addr, &value) in self.changes.range(offset..offset + size as u64) {
            #[allow(clippy::cast_possible_truncation)]
            let index = (addr - offset) as usize;
            data[index] = value;
        }

        Ok(data)
    }

    /// Write changes to the current file.
    pub fn save(&mut self) -> Result<()> {
        // reopen file with the write permission
        let mut file = OpenOptions::new().write(true).open(self.path.clone())?;

        // write changes
        for (&offset, &value) in &self.changes {
            file.seek(SeekFrom::Start(offset))?;
            file.write_all(&[value])?;
        }

        // reset
        self.cache.data.clear();
        self.changes.clear();

        Ok(())
    }

    /// Create copy of the file and write the current changes to it (save as).
    ///
    /// # Arguments
    ///
    /// * `file` - path to the new file
    /// * `progress` - long time operation handler
    pub fn save_as(&mut self, file: &Path, progress: &mut dyn ProgressHandler) -> Result<()> {
        // create new file
        let mut new_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&file)?;
        new_file.set_len(0)?;

        let mut offset = 0;
        loop {
            // update progress info
            let percent = (100.0 / self.size as f64) * offset as f64;
            if !progress.update(percent as u8) {
                return Err(Error::new(ErrorKind::Interrupted, "Aborted by user"));
            }

            // read and write
            let data = self.read(offset, File::BLOCK_SIZE)?;
            new_file.write_all(&data)?;
            offset += data.len() as u64;
            if offset >= self.size {
                break; //eof
            }
        }

        self.file = new_file;

        let path = std::fs::canonicalize(file)?;
        self.path = path.into_os_string().into_string().unwrap();

        // reset
        self.cache.data.clear();
        self.changes.clear();

        Ok(())
    }

    /// Find sequence inside the current file from the specified position.
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
    /// Offset of the next sequence entry.
    pub fn find(
        &mut self,
        start: u64,
        sequence: &[u8],
        backward: bool,
        progress: &mut dyn ProgressHandler,
    ) -> Result<u64> {
        debug_assert!(start <= self.size);
        debug_assert!(!sequence.is_empty());
        debug_assert!(File::BLOCK_SIZE > sequence.len());

        let mut offset = start;
        if backward {
            if offset == 0 {
                offset = self.size;
            }
            offset -= 1;
        } else {
            offset += 1;
        }

        let mut round = false;
        let mut handled = 0;

        loop {
            // update progress info
            let percent = (100.0 / self.size as f64) * handled as f64;
            if !progress.update(percent as u8) {
                return Err(Error::new(ErrorKind::Interrupted, "Aborted by user"));
            }

            if backward {
                // backward search
                if round && offset < start {
                    break;
                }
                if offset >= File::BLOCK_SIZE as u64 {
                    offset -= File::BLOCK_SIZE as u64;
                } else {
                    if self.size < File::BLOCK_SIZE as u64 {
                        offset = 0;
                    } else {
                        offset = self.size - File::BLOCK_SIZE as u64;
                    }
                    round = true;
                }
            } else {
                // forward search
                if offset as u64 >= self.size {
                    offset = 0;
                    round = true;
                }
            }

            let file_data = self.read(offset as u64, File::BLOCK_SIZE)?;
            let mut window = file_data.windows(sequence.len());
            if !backward {
                if let Some(pos) = window.position(|wnd| wnd == sequence) {
                    return Ok(offset as u64 + pos as u64);
                }
            } else if let Some(pos) = window.rposition(|wnd| wnd == sequence) {
                return Ok(offset as u64 + pos as u64);
            }

            if !backward {
                offset += File::BLOCK_SIZE as u64;
                if round && offset as u64 >= start {
                    break;
                }
            }

            handled += file_data.len() as u64;
        }

        Err(Error::new(ErrorKind::NotFound, "Sequence not found"))
    }

    /// Insert bytes at the specified position in the file.
    ///
    /// # Arguments
    ///
    /// * `offset` - start position of bytes to insert
    /// * `length` - number of bytes to insert
    /// * `pattern` - pattern to fill the added range
    /// * `progress` - long time operation handler
    pub fn insert(
        &mut self,
        offset: u64,
        length: u64,
        pattern: &[u8],
        progress: &mut dyn ProgressHandler,
    ) -> Result<()> {
        debug_assert!(self.changes.is_empty());
        debug_assert!(offset <= self.size);
        debug_assert!(length > 0);
        debug_assert!(!pattern.is_empty());

        // reopen file with the write permission
        let mut file = OpenOptions::new().read(true).write(true).open(&self.path)?;

        // extend the file
        file.set_len(self.size + length)?;

        let mut handled = 0;
        let mut buffer = vec![0; File::BLOCK_SIZE];

        // move (copy) data to the end of file
        let mut back_offset = self.size;
        while back_offset > offset {
            // update progress info
            let percent = (100.0 / (self.size - offset + length) as f64) * handled as f64;
            if !progress.update(percent as u8) {
                return Err(Error::new(ErrorKind::Interrupted, "Aborted by user"));
            }

            // calculate size and position of the next block
            let mut size = buffer.len();
            #[allow(clippy::cast_possible_truncation)]
            if back_offset - (size as u64).min(back_offset) <= offset {
                size = (back_offset - offset) as usize;
            }
            let read_offset = back_offset - size as u64;
            let write_offset = back_offset + length - size as u64;

            // read data
            file.seek(SeekFrom::Start(read_offset))?;
            file.read_exact(&mut buffer[..size])?;

            // write data
            file.seek(SeekFrom::Start(write_offset))?;
            file.write_all(&buffer[..size])?;

            //dst_end -= size as u64;
            back_offset -= size as u64;
            handled += size as u64;
        }

        // fill with pattern
        let max_offset = offset + length;
        let mut fill_offset = offset;
        let mut pattern_pos = 0;
        while fill_offset < max_offset {
            // update progress info
            let percent = (100.0 / (self.size - offset + length) as f64) * handled as f64;
            if !progress.update(percent as u8) {
                return Err(Error::new(ErrorKind::Interrupted, "Aborted by user"));
            }

            // calculate size of the next block
            let mut size = buffer.len();
            #[allow(clippy::cast_possible_truncation)]
            if fill_offset + (size as u64) > max_offset {
                size = (max_offset - fill_offset) as usize;
            }

            // apply pattern
            for byte in buffer.iter_mut().take(size) {
                *byte = pattern[pattern_pos];
                pattern_pos += 1;
                if pattern_pos == pattern.len() {
                    pattern_pos = 0;
                }
            }

            // write data
            file.seek(SeekFrom::Start(fill_offset))?;
            file.write_all(&buffer[..size])?;

            fill_offset += size as u64;
            handled += size as u64;
        }

        file.sync_all()?;

        self.size += length;

        // reset cache
        self.cache.data.clear();

        Ok(())
    }

    /// Cut out the specified range from the file.
    ///
    /// # Arguments
    ///
    /// * `range` - range to cut out
    /// * `progress` - long time operation handler
    pub fn cut(&mut self, range: &Range<u64>, progress: &mut dyn ProgressHandler) -> Result<()> {
        debug_assert!(self.changes.is_empty());
        debug_assert!(!range.is_empty());
        debug_assert!(range.end <= self.size);

        let range_len = range.end - range.start;

        // reopen file with the write permission
        let mut file = OpenOptions::new().read(true).write(true).open(&self.path)?;

        let mut offset = range.start;
        let mut buffer = vec![0; File::BLOCK_SIZE];
        loop {
            // update progress info
            let percent = (100.0 / (self.size - range.end) as f64) * (offset - range.start) as f64;
            if !progress.update(percent as u8) {
                return Err(Error::new(ErrorKind::Interrupted, "Aborted by user"));
            }

            // read data
            file.seek(SeekFrom::Start(offset + range_len))?;
            let size = file.read(&mut buffer)?;
            if size == 0 {
                break; //end of file
            }

            // write data
            file.seek(SeekFrom::Start(offset))?;
            file.write_all(&buffer[..size])?;

            offset += size as u64;
        }

        self.size -= range_len;

        // truncate the file
        file.set_len(self.size)?;
        file.sync_all()?;

        // reset cache
        self.cache.data.clear();

        Ok(())
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

/// Progress handler interface for long time operations.
pub trait ProgressHandler {
    /// Update progress and check for interrupt.
    ///
    /// # Arguments
    ///
    /// * `percent` - current percent of completion
    ///
    /// # Return value
    ///
    /// `false` if operation aborted by user.
    fn update(&mut self, percent: u8) -> bool;
}

#[cfg(test)]
struct ProgressTest {}

#[cfg(test)]
impl ProgressHandler for ProgressTest {
    fn update(&mut self, percent: u8) -> bool {
        assert!(percent <= 100);
        true
    }
}

#[test]
fn test_find() {
    let path = std::env::temp_dir().join("xvi_test_file.find");
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .unwrap();
    file.set_len(0).unwrap();
    file.write_all(&vec![11; 4]).unwrap();
    file.write_all(&vec![22, 33, 33, 33, 44]).unwrap();
    file.write_all(&vec![55, 5]).unwrap();

    let mut progress = ProgressTest {};

    let mut file = File::open(&path).unwrap();
    assert_eq!(
        file.find(0, &vec![42], false, &mut progress)
            .unwrap_err()
            .kind(),
        ErrorKind::NotFound
    );

    assert_eq!(
        file.find(0, &vec![33, 33], false, &mut progress).unwrap(),
        5
    );
    assert_eq!(
        file.find(5, &vec![33, 33], false, &mut progress).unwrap(),
        6
    );
    assert_eq!(
        file.find(file.size, &vec![33, 33], true, &mut progress)
            .unwrap(),
        6
    );

    std::fs::remove_file(path).unwrap();
}

#[test]
fn test_cut() {
    let path = std::env::temp_dir().join("xvi_test_file.cut");
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .unwrap();
    file.set_len(0).unwrap();
    file.write_all(&vec![11; 4]).unwrap();
    file.write_all(&vec![22, 33, 44, 55]).unwrap();
    file.write_all(&vec![66; 4]).unwrap();

    let mut progress = ProgressTest {};

    let mut file = File::open(&path).unwrap();
    file.cut(&(2..5), &mut progress).unwrap();
    assert_eq!(
        file.read(0, 255).unwrap(),
        vec![11, 11, 33, 44, 55, 66, 66, 66, 66]
    );

    std::fs::remove_file(path).unwrap();
}

#[test]
fn test_insert() {
    let path = std::env::temp_dir().join("xvi_test_file.insert");
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .unwrap();
    file.set_len(0).unwrap();
    file.write_all(&vec![11, 22, 33, 44, 55, 66, 77]).unwrap();

    let mut progress = ProgressTest {};

    let mut file = File::open(&path).unwrap();
    file.insert(1, 4, &vec![88, 99], &mut progress).unwrap();
    assert_eq!(
        file.read(0, 255).unwrap(),
        vec![11, 88, 99, 88, 99, 22, 33, 44, 55, 66, 77]
    );

    std::fs::remove_file(path).unwrap();
}
