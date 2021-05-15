// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::curses::Color;
use super::inifile::IniFile;
use std::collections::BTreeMap;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Configuration (user settings).
pub struct Config {
    /// Max number of stored file positions.
    pub filepos: usize,
    /// Color scheme.
    pub colors: BTreeMap<Color, (u8, u8)>,
}

impl Default for Config {
    fn default() -> Self {
        let mut colors = BTreeMap::new();
        for &(id, fg, bg) in Config::DARK_THEME {
            colors.insert(id, (fg, bg));
        }
        Self {
            filepos: 10,
            colors,
        }
    }
}

thread_local! {
    static CONFIG: RwLock<Arc<Config>> = RwLock::new(Default::default());
}

impl Config {
    const HISTORY: &'static str = "history";
    const COLORS: &'static str = "colors";

    /// Get current configuration instance.
    pub fn get() -> Arc<Config> {
        CONFIG.with(|c| c.read().unwrap().clone())
    }

    /// Load configuration from the default rc file.
    pub fn load() {
        let dir = match env::var("XDG_CONFIG_HOME") {
            Ok(val) => PathBuf::from(val),
            Err(_) => match env::var("HOME") {
                Ok(val) => PathBuf::from(val).join(".config"),
                Err(_) => PathBuf::new(),
            },
        };
        let file = dir.join("xvi").join("xvirc");
        Config::load_file(&file);
    }

    /// Load configuration from specified rc file.
    pub fn load_file(file: &Path) {
        if let Ok(ini) = IniFile::load(file) {
            let mut cfg = Config::default();
            if let Some(val) = ini.get(Config::HISTORY, "filepos") {
                if let Ok(val) = val.parse::<usize>() {
                    cfg.filepos = val;
                }
            }
            if let Some(val) = ini.get(Config::COLORS, "theme") {
                match val.to_lowercase().as_str() {
                    "light" => {
                        for &(id, fg, bg) in Config::LIGHT_THEME {
                            cfg.colors.insert(id, (fg, bg));
                        }
                    }
                    "dark" => { /* already set by default */ }
                    _ => {}
                };
            }
            if let Some(section) = ini.sections.get(Config::COLORS) {
                cfg.parse_colors(section);
            }

            CONFIG.with(|c| *c.write().unwrap() = Arc::new(cfg));
        }
    }

    /// Parse color parameters.
    fn parse_colors(&mut self, section: &BTreeMap<String, String>) {
        for (key, val) in section.iter() {
            let id = match key.to_lowercase().as_str() {
                "offsetnormal" => Color::OffsetNormal,
                "offsethi" => Color::OffsetHi,
                "hexnormal" => Color::HexNormal,
                "hexhi" => Color::HexHi,
                "hexmodified" => Color::HexModified,
                "hexmodifiedhi" => Color::HexModifiedHi,
                "asciinormal" => Color::AsciiNormal,
                "asciihi" => Color::AsciiHi,
                "asciimodified" => Color::AsciiModified,
                "asciimodifiedhi" => Color::AsciiModifiedHi,
                "statusbar" => Color::StatusBar,
                "keybarid" => Color::KeyBarId,
                "keybartitle" => Color::KeyBarTitle,
                "dialognormal" => Color::DialogNormal,
                "dialogerror" => Color::DialogError,
                "dialogshadow" => Color::DialogShadow,
                "itemdisabled" => Color::ItemDisabled,
                "itemfocused" => Color::ItemFocused,
                "editnormal" => Color::EditNormal,
                "editfocused" => Color::EditFocused,
                "editselection" => Color::EditSelection,
                _ => {
                    continue;
                }
            };
            let split: Vec<&str> = val.splitn(2, ',').collect();
            if split.len() == 2 {
                if let Ok(fg) = split[0].trim().parse::<u8>() {
                    if let Ok(bg) = split[1].trim().parse::<u8>() {
                        self.colors.insert(id, (fg, bg));
                    }
                }
            }
        }
    }

    /// Default color shceme for light theme.
    #[rustfmt::skip]
    const LIGHT_THEME: &'static [(Color, u8, u8)] = &[
        (Color::OffsetNormal,     7,  4),
        (Color::OffsetHi,         0, 12),
        (Color::HexNormal,        7,  4),
        (Color::HexHi,            0, 12),
        (Color::HexModified,     11,  4),
        (Color::HexModifiedHi,   11, 12),
        (Color::AsciiNormal,      7,  4),
        (Color::AsciiHi,          0, 12),
        (Color::AsciiModified,   11,  4),
        (Color::AsciiModifiedHi, 11, 12),
        (Color::StatusBar,        0,  6),
        (Color::KeyBarId,         7,  0),
        (Color::KeyBarTitle,      0,  6),
        (Color::DialogNormal,     0,  7),
        (Color::DialogError,      15, 1),
        (Color::DialogShadow,     8,  0),
        (Color::ItemDisabled,     8,  7),
        (Color::ItemFocused,      0,  6),
        (Color::EditNormal,       0,  6),
        (Color::EditFocused,      0,  6),
        (Color::EditSelection,    15, 0),
    ];

    /// Default color shceme for dark theme.
    #[rustfmt::skip]
    const DARK_THEME: &'static [(Color, u8, u8)] = &[
        (Color::OffsetNormal,    241, 233),
        (Color::OffsetHi,        250, 235),
        (Color::HexNormal,       247, 233),
        (Color::HexHi,           250, 235),
        (Color::HexModified,     220, 233),
        (Color::HexModifiedHi,   220, 235),
        (Color::AsciiNormal,     241, 233),
        (Color::AsciiHi,         250, 235),
        (Color::AsciiModified,   220, 233),
        (Color::AsciiModifiedHi, 220, 235),
        (Color::StatusBar,       242, 236),
        (Color::KeyBarId,        242, 233),
        (Color::KeyBarTitle,     242, 236),
        (Color::DialogNormal,    235, 245),
        (Color::DialogError,     250, 88),
        (Color::DialogShadow,    237, 232),
        (Color::ItemDisabled,    239, 245),
        (Color::ItemFocused,     250, 238),
        (Color::EditNormal,      235, 243),
        (Color::EditFocused,     250, 238),
        (Color::EditSelection,   250, 235),
    ];
}
