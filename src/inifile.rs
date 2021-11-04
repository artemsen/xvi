// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use std::collections::BTreeMap;
use std::fs::{create_dir_all, File};
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

/// INI file (DOS-like, very simple).
pub struct IniFile {
    pub sections: BTreeMap<String, Vec<String>>,
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
        let ini_file = File::open(file)?;

        let mut instance = IniFile::new();
        let mut last_section = String::new();

        for line in BufReader::new(ini_file).lines().flatten() {
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
            // section line
            instance
                .sections
                .entry(last_section.clone())
                .or_insert_with(Vec::new)
                .push(line.to_string());
        }

        Ok(instance)
    }

    /// Save configuration to the file.
    pub fn save(&self, file: &Path) -> io::Result<()> {
        create_dir_all(file.parent().unwrap())?;
        let mut ini = File::create(file)?;
        for (name, params) in self.sections.iter() {
            ini.write_all(format!("[{}]\n", name).as_bytes())?;
            for line in params.iter() {
                ini.write_all(format!("{}\n", line).as_bytes())?;
            }
        }
        Ok(())
    }

    /// Set value for specified key in the named section.
    #[allow(dead_code)]
    pub fn set_keyval(&mut self, section: &str, key: &str, value: &str) {
        let lkey = key.to_lowercase();
        let new_line = format!("{} = {}", lkey, value);
        let section = section.to_lowercase();
        let section = &mut self.sections.entry(section).or_insert_with(Vec::new);
        for (index, line) in section.iter().enumerate() {
            if let Some((ckey, _)) = IniFile::keyval(line) {
                if ckey == lkey {
                    section[index] = new_line;
                    return;
                }
            }
        }
        section.push(new_line);
    }

    /// Get string value for specified key in the named section.
    pub fn get_strval(&self, section: &str, key: &str) -> Option<String> {
        if let Some(section) = self.sections.get(&section.to_lowercase()) {
            let key = key.to_lowercase();
            for line in section.iter() {
                if let Some((ckey, val)) = IniFile::keyval(line) {
                    if ckey == key {
                        return Some(val);
                    }
                }
            }
        }
        None
    }

    /// Get numeric value for specified key in the named section.
    pub fn get_numval(&self, section: &str, key: &str) -> Option<usize> {
        if let Some(val) = self.get_strval(section, key) {
            if let Ok(val) = val.parse::<usize>() {
                return Some(val);
            }
        }
        None
    }

    /// Get boolean value for specified key in the named section.
    pub fn get_boolval(&self, section: &str, key: &str) -> Option<bool> {
        if let Some(val) = self.get_numval(section, key) {
            return Some(val != 0);
        }
        None
    }

    /// Parse and convert line to the Key/Value pair.
    pub fn keyval(line: &str) -> Option<(String, String)> {
        let split: Vec<&str> = line.splitn(2, '=').collect();
        return if split.len() == 2 {
            let key = String::from(split[0].trim()).to_lowercase();
            let value = String::from(split[1].trim());
            Some((key, value))
        } else {
            None
        };
    }
}

#[test]
fn test_load() {
    let tmp_file = std::env::temp_dir().join("xvi_test_ini.load");
    let ini_data = r#"#comment
[section1]
[Section2]
    # c o m m e n t
Section_line
Section_line
[seCTIon3]
  sectionLine1
sectionLine2  
sectionLine3"#;
    std::fs::write(&tmp_file, ini_data).unwrap();

    let ini = IniFile::load(&tmp_file).unwrap();
    assert_eq!(ini.sections.len(), 2);
    assert!(ini.sections.contains_key("section2"));
    assert_eq!(
        ini.sections["section2"],
        vec!["Section_line", "Section_line"]
    );
    assert!(ini.sections.contains_key("section3"));
    assert_eq!(
        ini.sections["section3"],
        vec!["sectionLine1", "sectionLine2", "sectionLine3"]
    );

    std::fs::remove_file(tmp_file).unwrap();
}

#[test]
fn test_save() {
    let mut ini = IniFile::new();
    ini.sections.insert(
        "section1".to_string(),
        vec!["line".to_string(), "line".to_string()],
    );
    ini.sections.insert(
        "section2".to_string(),
        vec![
            "line1".to_string(),
            "line2".to_string(),
            "line3".to_string(),
        ],
    );

    let tmp_file = std::env::temp_dir().join("xvi_test_ini.save");
    ini.save(&tmp_file).unwrap();

    let ini_data = std::fs::read_to_string(&tmp_file).unwrap();
    assert_eq!(
        ini_data,
        "[section1]\nline\nline\n[section2]\nline1\nline2\nline3\n"
    );

    std::fs::remove_file(tmp_file).unwrap();
}

#[test]
fn test_str_keyval() {
    let mut ini = IniFile::new();

    ini.set_keyval("Section1", "MyKey", "MyVal");
    assert_eq!(
        ini.get_strval("Section1", "MyKey"),
        Some("MyVal".to_string())
    );

    // case ignore
    assert_eq!(
        ini.get_strval("section1", "mykey"),
        Some("MyVal".to_string())
    );

    // not existing
    assert_eq!(ini.get_strval("section1", "mykey1"), None);
    assert_eq!(ini.get_strval("section2", "mykey"), None);

    // update
    ini.set_keyval("section1", "Mykey", "MyVal2");
    assert_eq!(ini.sections["section1"].len(), 1);
    assert_eq!(
        ini.get_strval("section1", "MyKey"),
        Some("MyVal2".to_string())
    );
}

#[test]
fn test_num_keyval() {
    let mut ini = IniFile::new();
    ini.set_keyval("Section1", "MyKey", "123456789");
    assert_eq!(ini.get_numval("Section1", "MyKey"), Some(123456789));
}

#[test]
fn test_bool_keyval() {
    let mut ini = IniFile::new();
    ini.set_keyval("Section1", "MyKey", "1");
    assert_eq!(ini.get_boolval("Section1", "MyKey"), Some(true));
    ini.set_keyval("Section1", "MyKey", "0");
    assert_eq!(ini.get_boolval("Section1", "MyKey"), Some(false));
}

#[test]
fn test_keyval() {
    assert_eq!(
        IniFile::keyval("mykey=myvalue"),
        Some(("mykey".to_string(), "myvalue".to_string()))
    );
    assert_eq!(
        IniFile::keyval("MyKey=MyValue"),
        Some(("mykey".to_string(), "MyValue".to_string()))
    );
    assert_eq!(
        IniFile::keyval(" mykey\t  =\tmyvalue\n\n"),
        Some(("mykey".to_string(), "myvalue".to_string()))
    );
    assert_eq!(
        IniFile::keyval("mykey = myva=lue"),
        Some(("mykey".to_string(), "myva=lue".to_string()))
    );
    assert_eq!(IniFile::keyval("mykeymyvalue"), None);
}
