// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use std::collections::BTreeMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

/// INI file (DOS-like, very simple).
pub struct IniFile {
    pub sections: BTreeMap<String, BTreeMap<String, String>>,
}

impl IniFile {
    /// Create instance.
    pub fn new() -> Self {
        Self {
            sections: BTreeMap::new(),
        }
    }

    /// Load configuration from the file.
    pub fn load(file: &Path) -> io::Result<Self> {
        let ini = File::open(file)?;

        let mut instance = IniFile::new();
        let mut last_section = String::new();

        for line in BufReader::new(ini).lines().flatten() {
            let line = line.trim();
            // skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // section name
            if line.starts_with('[') && line.ends_with(']') {
                last_section = String::from(&line[1..line.len() - 1]).to_lowercase();
                continue;
            }
            // key = value
            let split: Vec<&str> = line.splitn(2, '=').collect();
            if split.len() != 2 {
                println!("WARNING: Invalid config: {}", line);
            } else {
                let key = String::from(split[0].trim()).to_lowercase();
                let value = String::from(split[1].trim());
                instance
                    .sections
                    .entry(last_section.clone())
                    .or_insert_with(BTreeMap::new)
                    .insert(key, value);
            }
        }

        Ok(instance)
    }

    /// Save configuration to the file.
    pub fn save(&self, file: &Path) -> io::Result<()> {
        let mut ini = File::create(file)?;
        for (name, params) in self.sections.iter() {
            ini.write_all(format!("[{}]\n", name).as_bytes())?;
            for (key, val) in params.iter() {
                ini.write_all(format!("{}={}\n", key, val).as_bytes())?;
            }
        }
        Ok(())
    }

    /// Set value for specified key in the named section.
    pub fn set(&mut self, section: &str, key: &str, value: &str) {
        let section = section.to_lowercase();
        let key = key.to_lowercase();
        self.sections
            .entry(section)
            .or_insert_with(BTreeMap::new)
            .insert(key, String::from(value));
    }

    /// Get value for specified key in the named section.
    pub fn get(&self, section: &str, key: &str) -> Option<&String> {
        let section = section.to_lowercase();
        let key = key.to_lowercase();
        if let Some(section) = self.sections.get(&section) {
            return section.get(&key);
        }
        None
    }

    /// Get numeric value for specified key in the named section.
    pub fn get_num(&self, section: &str, key: &str) -> Option<usize> {
        if let Some(val) = self.get(section, key) {
            if let Ok(val) = val.parse::<usize>() {
                return Some(val);
            }
        }
        None
    }

    /// Get boolean value for specified key in the named section.
    pub fn get_bool(&self, section: &str, key: &str) -> Option<bool> {
        if let Some(val) = self.get_num(section, key) {
            return Some(val != 0);
        }
        None
    }
}
