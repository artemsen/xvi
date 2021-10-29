// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::ascii::AsciiTable;
use super::curses::Color;
use super::inifile::IniFile;
use std::env;
use std::path::PathBuf;

/// Configuration (user settings).
pub struct Config {
    /// Line width mode (fixed/dynamic).
    pub fixed_width: bool,
    /// ASCII table identifier.
    pub ascii_table: Option<&'static AsciiTable>,
    /// Color scheme.
    pub colors: Vec<(Color, u8, u8)>,
}

impl Default for Config {
    fn default() -> Self {
        let mut colors = Vec::new();
        for &(id, fg, bg) in Config::DARK_THEME {
            colors.push((id, fg, bg));
        }
        Self {
            fixed_width: false,
            ascii_table: Some(AsciiTable::default()),
            colors,
        }
    }
}

impl Config {
    const VIEW: &'static str = "View";
    const COLORS: &'static str = "Colors";

    /// Load configuration from the default rc file.
    pub fn load() -> Self {
        let mut instance = Config::default();

        let dir = match env::var("XDG_CONFIG_HOME") {
            Ok(val) => PathBuf::from(val),
            Err(_) => match env::var("HOME") {
                Ok(val) => PathBuf::from(val).join(".config"),
                Err(_) => PathBuf::new(),
            },
        };
        let file = dir.join("xvi").join("config");

        if let Ok(ini) = IniFile::load(&file) {
            if let Some(val) = ini.get_boolval(Config::VIEW, "FixedWidth") {
                instance.fixed_width = val;
            }
            if let Some(val) = ini.get_strval(Config::VIEW, "Ascii") {
                if val == "none" {
                    instance.ascii_table = None;
                } else {
                    instance.ascii_table = AsciiTable::from_id(&val);
                }
            }
            if let Some(val) = ini.get_strval(Config::COLORS, "Theme") {
                match val.to_lowercase().as_str() {
                    "light" => {
                        instance.colors = Vec::from(Config::LIGHT_THEME);
                    }
                    "dark" => { /* already set by default */ }
                    _ => {}
                };
            }
            if let Some(section) = ini.sections.get(&Config::COLORS.to_lowercase()) {
                instance.parse_colors(section);
            }
        }

        instance
    }

    /// Parse color parameters.
    fn parse_colors(&mut self, config: &[String]) {
        for line in config.iter() {
            if let Some((key, val)) = IniFile::keyval(line) {
                let id = match key.as_str() {
                    "offsetnormal" => Color::OffsetNormal,
                    "offsethi" => Color::OffsetHi,
                    "hexnormal" => Color::HexNormal,
                    "hexhi" => Color::HexHi,
                    "hexmodified" => Color::HexModified,
                    "hexmodifiedhi" => Color::HexModifiedHi,
                    "hexdiff" => Color::HexDiff,
                    "hexdiffhi" => Color::HexDiffHi,
                    "asciinormal" => Color::AsciiNormal,
                    "asciihi" => Color::AsciiHi,
                    "asciimodified" => Color::AsciiModified,
                    "asciimodifiedhi" => Color::AsciiModifiedHi,
                    "asciidiff" => Color::AsciiDiff,
                    "asciidiffhi" => Color::AsciiDiffHi,
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
                            // replace color
                            let index = self.colors.iter().position(|c| c.0 == id).unwrap();
                            self.colors[index] = (id, fg, bg);
                        }
                    }
                }
            }
        }
    }

    /// Default color scheme for light theme (id, foreground, background).
    #[rustfmt::skip]
    const LIGHT_THEME: &'static [(Color, u8, u8)] = &[
        (Color::OffsetNormal,     7,  4),
        (Color::OffsetHi,         0, 12),
        (Color::HexNormal,        7,  4),
        (Color::HexHi,            0, 12),
        (Color::HexModified,     11,  4),
        (Color::HexModifiedHi,   11, 12),
        (Color::HexDiff,          1,  4),
        (Color::HexDiffHi,        1, 12),
        (Color::AsciiNormal,      7,  4),
        (Color::AsciiHi,          0, 12),
        (Color::AsciiModified,   11,  4),
        (Color::AsciiModifiedHi, 11, 12),
        (Color::AsciiDiff,        1,  4),
        (Color::AsciiDiffHi,      1, 12),
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

    /// Default color scheme for dark theme (id, foreground, background).
    #[rustfmt::skip]
    const DARK_THEME: &'static [(Color, u8, u8)] = &[
        (Color::OffsetNormal,    241, 233),
        (Color::OffsetHi,        250, 235),
        (Color::HexNormal,       247, 233),
        (Color::HexHi,           250, 235),
        (Color::HexModified,     220, 233),
        (Color::HexModifiedHi,   220, 235),
        (Color::HexDiff,         124, 233),
        (Color::HexDiffHi,       160, 235),
        (Color::AsciiNormal,     241, 233),
        (Color::AsciiHi,         250, 235),
        (Color::AsciiModified,   220, 233),
        (Color::AsciiModifiedHi, 220, 235),
        (Color::AsciiDiff,       124, 233),
        (Color::AsciiDiffHi,     160, 235),
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
