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
    const GOTO: &'static str = "goto";
    const SEARCH: &'static str = "search";
    const FILE: &'static str = "file";

    /// Create instance: load history file.
    pub fn new() -> Self {
        if let Some(file) = History::file() {
            if let Ok(ini) = IniFile::load(&file) {
                return Self { ini };
            }
        }
        Self {
            ini: IniFile::new(),
        }
    }

    /// Save history data to the file.
    pub fn save(&self) {
        if let Some(file) = History::file() {
            let _ = self.ini.save(&file);
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
    pub fn set_goto(&mut self, offsets: &[u64], max: usize) {
        let ini_list = offsets
            .iter()
            .take(max)
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
    pub fn set_search(&mut self, sequences: &[Vec<u8>], max: usize) {
        let mut ini_list = Vec::with_capacity(sequences.len());
        for seq in sequences.iter().take(max) {
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
    pub fn add_filepos(&mut self, file: &str, offset: u64, max: usize) {
        let section = &mut self
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
        section.truncate(max);
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

    /// Split the "file:offset" line into components.
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
