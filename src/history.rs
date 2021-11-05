// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::inifile::IniFile;
use std::env;
use std::path::PathBuf;

/// History of editor.
pub struct History {
    /// INI file with history data.
    ini: IniFile,
}

impl History {
    // Max number of stored entries
    const MAX_GOTO: usize = 10;
    const MAX_SEARCH: usize = 10;
    const MAX_FILE: usize = 10;

    // INI sections names
    const GOTO: &'static str = "goto";
    const SEARCH: &'static str = "search";
    const FILE: &'static str = "file";

    /// Save history data to the file.
    pub fn save(&self) {
        if let Some(file) = History::file() {
            self.ini.save(&file).ok();
        }
    }

    /// Get list of the last used "goto" addresses.
    pub fn get_goto(&self) -> Vec<u64> {
        if let Some(section) = self.ini.sections.get(History::GOTO) {
            let mut offsets = Vec::with_capacity(section.len());
            for line in section.iter() {
                if let Ok(offset) = u64::from_str_radix(line, 16) {
                    offsets.push(offset);
                }
            }
            return offsets;
        }
        Vec::new()
    }

    /// Set list of the last used "goto" addresses.
    pub fn set_goto(&mut self, offsets: &[u64]) {
        let ini_list = offsets
            .iter()
            .take(History::MAX_GOTO)
            .map(|o| format!("{:x}", o))
            .collect();
        self.ini
            .sections
            .insert(History::GOTO.to_string(), ini_list);
    }

    /// Get list of the last used search sequences.
    pub fn get_search(&self) -> Vec<Vec<u8>> {
        if let Some(section) = self.ini.sections.get(History::SEARCH) {
            let mut searches = Vec::with_capacity(section.len());
            for line in section.iter() {
                if line.len() % 2 == 0 {
                    let mut seq = Vec::with_capacity(line.len() / 2);
                    for i in (0..line.len()).step_by(2) {
                        if let Ok(n) = u8::from_str_radix(&line[i..i + 2], 16) {
                            seq.push(n);
                        } else {
                            break;
                        }
                    }
                    if !seq.is_empty() {
                        searches.push(seq);
                    }
                }
            }
            return searches;
        }
        Vec::new()
    }

    /// Set list of the last used search sequences.
    pub fn set_search(&mut self, sequences: &[Vec<u8>]) {
        let mut ini_list = Vec::with_capacity(sequences.len());
        for seq in sequences.iter().take(History::MAX_SEARCH) {
            ini_list.push(seq.iter().map(|b| format!("{:02x}", b)).collect());
        }
        self.ini
            .sections
            .insert(History::SEARCH.to_string(), ini_list);
    }

    /// Get last position for the specified file.
    pub fn get_filepos(&self, file: &str) -> Option<u64> {
        if let Some(section) = self.ini.sections.get(History::FILE) {
            for line in section.iter() {
                if let Some((path, offset)) = History::filepos(line) {
                    if path == file {
                        return Some(offset);
                    }
                }
            }
        }
        None
    }

    /// Add last position for the specified file.
    pub fn add_filepos(&mut self, file: &str, offset: u64) {
        let section = self
            .ini
            .sections
            .entry(History::FILE.to_string())
            .or_insert_with(Vec::new);

        // remove previous offset info
        section.retain(|l| {
            if let Some((p, _)) = History::filepos(l) {
                p != file
            } else {
                false
            }
        });

        // insert new record
        section.insert(0, format!("{}:{:x}", file, offset));
        section.truncate(History::MAX_FILE);
    }

    /// Get path to the history file.
    fn file() -> Option<PathBuf> {
        let dir;
        match env::var("XDG_DATA_HOME") {
            Ok(val) => dir = PathBuf::from(val),
            Err(_) => match env::var("HOME") {
                Ok(val) => dir = PathBuf::from(val).join(".local").join("share"),
                Err(_) => return None,
            },
        };
        Some(dir.join("xvi").join("history"))
    }

    /// Split the `file:offset` line into components.
    fn filepos(line: &str) -> Option<(&str, u64)> {
        let split: Vec<&str> = line.rsplitn(2, ':').collect();
        if split.len() == 2 {
            if let Ok(offset) = u64::from_str_radix(split[0], 16) {
                return Some((split[1], offset));
            }
        }
        None
    }
}

impl Default for History {
    fn default() -> Self {
        if let Some(file) = History::file() {
            if let Ok(ini) = IniFile::load(&file) {
                return Self { ini };
            }
        }
        Self {
            ini: IniFile::new(),
        }
    }
}

#[test]
fn test_set_goto() {
    let ini = IniFile::new();
    let mut history = History { ini };

    let mut offsets = Vec::new();
    for i in 0..History::MAX_GOTO + 2 {
        offsets.push(i as u64);
    }
    history.set_goto(&offsets);

    let offsets = history.get_goto();
    assert_eq!(offsets.len(), History::MAX_GOTO);
    assert_eq!(offsets[0], 0);
    assert_eq!(
        offsets[History::MAX_GOTO - 1],
        (History::MAX_GOTO - 1) as u64
    );
}

#[test]
fn test_get_search() {
    let mut ini = IniFile::new();
    ini.sections.insert(
        History::SEARCH.to_string(),
        vec![
            "abcdef1234567890".to_string(),
            "112233".to_string(),
            "1234abc".to_string(), // not even
            "wtf?".to_string(),    // invalid hex
        ],
    );
    let history = History { ini };
    assert_eq!(
        history.get_search(),
        vec![
            vec![0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78, 0x90],
            vec![0x11, 0x22, 0x33]
        ]
    );
}

#[test]
fn test_set_search() {
    let ini = IniFile::new();
    let mut history = History { ini };
    history.set_search(&vec![vec![0x00]]);
    assert_eq!(history.get_search(), vec![vec![0x00]]);
    history.set_search(&vec![vec![0x12, 0x34], vec![0xab]]);
    assert_eq!(history.get_search(), vec![vec![0x12, 0x34], vec![0xab]]);
    assert_eq!(history.ini.sections[History::SEARCH], vec!["1234", "ab"]);
}

#[test]
fn test_get_filepos() {
    let mut ini = IniFile::new();
    ini.sections.insert(
        History::FILE.to_string(),
        vec![
            "/path/to/file:123ab".to_string(),
            "/path/to/fi:le:cdef".to_string(),
        ],
    );
    let history = History { ini };
    assert_eq!(history.get_filepos("/path/to/file"), Some(0x123ab));
    assert_eq!(history.get_filepos("/path/to/fi:le"), Some(0xcdef));
    assert_eq!(history.get_filepos("/does/not/exist"), None);
}

#[test]
fn test_add_filepos() {
    let ini = IniFile::new();
    let mut history = History { ini };

    for i in 0..History::MAX_FILE + 2 {
        history.add_filepos(&format!("/path/to/file{}", i), i as u64);
    }

    assert_eq!(history.get_filepos("/path/to/file1"), None);
    assert_eq!(history.get_filepos("/path/to/file2"), Some(2));
}
