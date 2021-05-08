// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::config::Config;
use super::inifile::IniFile;
use std::env;
use std::fs;
use std::path::PathBuf;

/// History of editor.
pub struct History {
    /// Last used "goto" address.
    pub last_goto: u64,
    /// Last used sequence to search.
    pub last_search: Vec<u8>,
    /// List of last position in recent files.
    file_pos: Vec<(String, u64)>,
}

impl History {
    const GENERAL: &'static str = "gn";
    const LASTGOTO: &'static str = "lg";
    const LASTSEARCH: &'static str = "ls";
    const FILEPOS: &'static str = "fp";
    const POSNAME: char = 'n';
    const POSADDR: char = 'a';

    /// Create instance: load history.
    pub fn new() -> Self {
        let mut instance = Self {
            file_pos: Vec::new(),
            last_goto: 0,
            last_search: Vec::new(),
        };
        if let Some(file) = History::ini_file() {
            if let Ok(ini) = IniFile::load(&file) {
                // last used "goto" address
                if let Some(hex) = ini.get(History::GENERAL, History::LASTGOTO) {
                    if let Ok(addr) = u64::from_str_radix(hex, 16) {
                        instance.last_goto = addr;
                    }
                }
                // last used search sequence
                if let Some(hex) = ini.get(History::GENERAL, History::LASTSEARCH) {
                    if hex.len() % 2 == 0 {
                        for i in (0..hex.len()).step_by(2) {
                            if let Ok(n) = u8::from_str_radix(&hex[i..i + 2], 16) {
                                instance.last_search.push(n);
                            } else {
                                break;
                            }
                        }
                    }
                }
                // list of last positions
                if let Some(section) = ini.sections.get(History::FILEPOS) {
                    for i in 0..Config::get().filepos {
                        if let Some(name) = section.get(&format!("{}{}", History::POSNAME, i)) {
                            if let Some(pos) = section.get(&format!("{}{}", History::POSADDR, i)) {
                                if let Ok(num) = u64::from_str_radix(pos, 16) {
                                    instance.file_pos.push((String::from(name), num));
                                }
                            }
                        }
                    }
                }
            }
        }
        instance
    }

    /// Get position for specified file.
    pub fn get_last_pos(&self, file: &str) -> Option<u64> {
        let path = History::abs_path(file);
        for (name, pos) in self.file_pos.iter() {
            if *name == path {
                return Some(*pos);
            }
        }
        None
    }

    /// Set position for specified file.
    pub fn set_last_pos(&mut self, file: &str, pos: u64) {
        let path = History::abs_path(file);
        for (index, (name, _)) in self.file_pos.iter().enumerate() {
            if *name == path {
                // remove old entry
                self.file_pos.remove(index);
                break;
            }
        }
        self.file_pos.insert(0, (path, pos));

        let max_pos = Config::get().filepos;
        if self.file_pos.len() > max_pos {
            self.file_pos.drain(max_pos..);
        }
    }

    /// Save history to the file.
    pub fn save(&self) {
        if let Some(file) = History::ini_file() {
            let mut ini = IniFile::new();

            // last used "goto" address
            ini.set(
                History::GENERAL,
                History::LASTGOTO,
                &format!("{:x}", self.last_goto),
            );

            // last used search sequence
            let mut seq = String::with_capacity(self.last_search.len() * 2);
            for byte in self.last_search.iter() {
                seq.push_str(&format!("{:02x}", byte));
            }
            ini.set(History::GENERAL, History::LASTSEARCH, &seq);

            // list of last positions
            for (index, (name, pos)) in self.file_pos.iter().enumerate() {
                ini.set(
                    History::FILEPOS,
                    &format!("{}{}", History::POSNAME, index),
                    name,
                );
                ini.set(
                    History::FILEPOS,
                    &format!("{}{}", History::POSADDR, index),
                    &format!("{:x}", pos),
                );
            }

            // create path
            if let Some(parent) = file.parent() {
                if !parent.exists() && fs::create_dir(parent).is_err() {
                    return;
                }
            }

            let _ = ini.save(&file);
        }
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

    /// Get absolute path to the specified file.
    fn abs_path(file: &str) -> String {
        if let Ok(path) = PathBuf::from(file).canonicalize() {
            if let Ok(path) = path.into_os_string().into_string() {
                return path;
            }
        }
        String::from(file)
    }
}
