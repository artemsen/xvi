// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::inifile::IniFile;
use std::env;
use std::path::PathBuf;

/// History: loads, holds and saves editor history (offsets, searches, etc).
pub struct History {
    /// Last position in files.
    pub file_pos: Vec<(String, u64)>,
    /// Search history.
    pub search: Vec<Vec<u8>>,
    /// Last used search direction (volatile).
    pub search_backward: bool,
    /// Goto address history.
    pub goto: Vec<u64>,
    /// Last used pattern to fill (volatile).
    pub pattern: Vec<u8>,
}

impl History {
    // Max number of stored entries
    const MAX_FILE: usize = 10;
    const MAX_SEARCH: usize = 10;
    const MAX_GOTO: usize = 10;

    // INI sections names
    const SEC_FILE: &'static str = "file";
    const SEC_SEARCH: &'static str = "search";
    const SEC_GOTO: &'static str = "goto";

    /// Load history from the ini file.
    ///
    /// # Arguments
    ///
    /// * `ini` - ini file to process
    fn load(&mut self, ini: &IniFile) {
        // read recent file position history
        if let Some(section) = ini.sections.get(History::SEC_FILE) {
            self.file_pos.reserve(section.len().max(History::MAX_FILE));
            for line in section.iter().take(History::MAX_FILE) {
                // stored as `file:offset`
                let split: Vec<&str> = line.rsplitn(2, ':').collect();
                if split.len() == 2 {
                    if let Ok(offset) = u64::from_str_radix(split[0], 16) {
                        self.file_pos.push((split[1].to_string(), offset));
                    }
                }
            }
        }

        // read search history
        if let Some(section) = ini.sections.get(History::SEC_SEARCH) {
            self.search.reserve(section.len().max(History::MAX_SEARCH));
            for line in section.iter().take(History::MAX_SEARCH) {
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
                        self.search.push(seq);
                    }
                }
            }
        }

        // read "goto" history
        if let Some(section) = ini.sections.get(History::SEC_GOTO) {
            self.goto.reserve(section.len().max(History::MAX_GOTO));
            for line in section.iter().take(History::MAX_GOTO) {
                if let Ok(offset) = u64::from_str_radix(line, 16) {
                    self.goto.push(offset);
                }
            }
        }
    }

    /// Save history to the ini file.
    pub fn save(&self) {
        if let Some(file) = History::ini_file() {
            let mut ini = IniFile::new();

            // recent file position history
            ini.sections.insert(
                History::SEC_FILE.to_string(),
                self.file_pos
                    .iter()
                    .take(History::MAX_FILE)
                    .map(|(f, o)| format!("{}:{:x}", f, o))
                    .collect(),
            );

            // search history
            ini.sections.insert(
                History::SEC_SEARCH.to_string(),
                self.search
                    .iter()
                    .take(History::MAX_SEARCH)
                    .map(|s| s.iter().map(|b| format!("{:02x}", b)).collect())
                    .collect(),
            );

            // "goto" history
            ini.sections.insert(
                History::SEC_GOTO.to_string(),
                self.goto
                    .iter()
                    .take(History::MAX_GOTO)
                    .map(|o| format!("{:x}", o))
                    .collect(),
            );

            ini.save(&file).ok();
        }
    }

    /// Get last position for the specified file.
    pub fn get_filepos(&self, file: &str) -> Option<u64> {
        // get absolute path
        let path = if let Ok(path) = std::fs::canonicalize(file) {
            path.into_os_string().into_string().unwrap()
        } else {
            file.to_string()
        };
        // search in history
        if let Some((_, offset)) = self.file_pos.iter().find(|(f, _)| *f == path) {
            return Some(*offset);
        }
        None
    }

    /// Add last position for the specified file.
    pub fn add_filepos(&mut self, file: &str, offset: u64) {
        // get absolute path
        let path = if let Ok(path) = std::fs::canonicalize(file) {
            path.into_os_string().into_string().unwrap()
        } else {
            file.to_string()
        };
        // remove previous records and new one
        self.file_pos.retain(|(f, _)| *f == path);
        self.file_pos.insert(0, (path, offset));
        self.file_pos.truncate(History::MAX_FILE);
    }

    /// Add search sequence to history.
    pub fn add_search(&mut self, sequence: &[u8]) {
        self.search.retain(|s| s != sequence);
        self.search.insert(0, sequence.to_vec());
        self.search.truncate(History::MAX_SEARCH);
    }

    /// Add "goto" address to history.
    pub fn add_goto(&mut self, offset: u64) {
        self.goto.retain(|o| o != &offset);
        self.goto.insert(0, offset);
        self.goto.truncate(History::MAX_GOTO);
    }

    /// Get path to the history file.
    fn ini_file() -> Option<PathBuf> {
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
}

impl Default for History {
    fn default() -> Self {
        let mut instance = Self {
            file_pos: Vec::new(),
            search: Vec::new(),
            search_backward: false,
            goto: Vec::new(),
            pattern: vec![0],
        };

        if let Some(file) = History::ini_file() {
            if let Ok(ini) = IniFile::load(&file) {
                instance.load(&ini);
            }
        }

        instance
    }
}

#[test]
fn test_load() {
    let mut ini = IniFile::new();
    ini.sections.insert(
        History::SEC_FILE.to_string(),
        vec![
            "/path/to/file:1234abc".to_string(),
            "/path/to/file:invalid".to_string(),
        ],
    );
    ini.sections.insert(
        History::SEC_SEARCH.to_string(),
        vec![
            "abcdef1234567890".to_string(),
            "1234abc".to_string(), // not even
            "invalid".to_string(),
        ],
    );
    ini.sections.insert(
        History::SEC_GOTO.to_string(),
        vec!["abc".to_string(), "-1".to_string()],
    );

    let mut history = History {
        file_pos: Vec::new(),
        search: Vec::new(),
        search_backward: false,
        goto: Vec::new(),
        pattern: Vec::new(),
    };
    history.load(&ini);

    assert_eq!(
        history.file_pos,
        vec![("/path/to/file".to_string(), 0x1234abc_u64)]
    );
    assert_eq!(
        history.search,
        vec![vec![0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78, 0x90]]
    );
    assert_eq!(history.goto, vec![0xabc]);
}

#[test]
fn test_set_goto() {
    let mut history = History {
        file_pos: Vec::new(),
        search: Vec::new(),
        search_backward: false,
        goto: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
        pattern: Vec::new(),
    };

    history.add_goto(55);
    assert_eq!(history.goto, vec![55, 0, 1, 2, 3, 4, 5, 6, 7, 8],);

    history.add_goto(3);
    assert_eq!(history.goto, vec![3, 55, 0, 1, 2, 4, 5, 6, 7, 8],);
}
