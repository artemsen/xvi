// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::ascii::Table;
use super::curses::Color;
use super::inifile::IniFile;
use std::env;
use std::path::PathBuf;

/// App configuration.
pub struct Config {
    /// Line width mode (fixed/dynamic).
    pub fixed_width: bool,
    /// ASCII table identifier.
    pub ascii_table: Option<&'static Table>,
    /// Color scheme.
    pub colors: Vec<(Color, i16, i16)>,
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
                    instance.ascii_table = Table::from_id(&val);
                }
            }
            let mut palette = Palette::DARK.clone();
            if let Some(val) = ini.get_strval(Config::COLORS, "Theme") {
                if val.to_lowercase().as_str() == "light" {
                    palette = Palette::LIGHT.clone();
                }
            }
            if let Some(section) = ini.sections.get(&Config::COLORS.to_lowercase()) {
                palette.parse(section);
            }
            instance.colors = palette.colors();
        }

        instance
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fixed_width: false,
            ascii_table: Some(Table::default()),
            colors: Palette::DARK.colors(),
        }
    }
}

/// Color palette.
#[derive(Clone)]
struct Palette {
    general: (i16, i16),
    highlight: (i16, i16),
    offset: (i16, i16),
    ascii: (i16, i16),
    modified: (i16, i16),
    diff: (i16, i16),
    bar: (i16, i16),
    dialog: (i16, i16),
    error: (i16, i16),
    disabled: (i16, i16),
    focused: (i16, i16),
    input: (i16, i16),
    select: (i16, i16),
}

impl Palette {
    /// Default color palette for the dark theme.
    const DARK: &'static Palette = &Palette {
        general: (-1, -1),
        highlight: (-1, 235),
        offset: (241, -1),
        ascii: (241, -1),
        modified: (220, -1),
        diff: (124, -1),
        bar: (242, 236),
        dialog: (235, 245),
        error: (250, 88),
        disabled: (239, 245),
        focused: (250, 238),
        input: (235, 243),
        select: (250, 235),
    };

    /// Default color palette for the light theme.
    const LIGHT: &'static Palette = &Palette {
        general: (7, 4),
        highlight: (0, 12),
        offset: (7, 4),
        ascii: (7, 4),
        modified: (11, 4),
        diff: (1, 4),
        bar: (0, 6),
        dialog: (0, 7),
        error: (15, 1),
        disabled: (8, 7),
        focused: (0, 6),
        input: (0, 6),
        select: (15, 0),
    };

    /// Parse ini section with palette setup.
    ///
    /// # Arguments
    ///
    /// * `section` - ini section yo parse
    fn parse(&mut self, section: &[String]) {
        for line in section {
            if let Some((key, val)) = IniFile::keyval(line) {
                let split: Vec<&str> = val.splitn(2, ',').collect();
                if split.len() == 2 {
                    if let Ok(fg) = split[0].trim().parse::<i16>() {
                        if let Ok(bg) = split[1].trim().parse::<i16>() {
                            match key.as_str() {
                                "general" => {
                                    self.general = (fg, bg);
                                }
                                "highlight" => {
                                    self.highlight = (fg, bg);
                                }
                                "offset" => {
                                    self.offset = (fg, bg);
                                }
                                "ascii" => {
                                    self.ascii = (fg, bg);
                                }
                                "modified" => {
                                    self.modified = (fg, bg);
                                }
                                "diff" => {
                                    self.diff = (fg, bg);
                                }
                                "bar" => {
                                    self.bar = (fg, bg);
                                }
                                "dialog" => {
                                    self.dialog = (fg, bg);
                                }
                                "error" => {
                                    self.error = (fg, bg);
                                }
                                "disabled" => {
                                    self.disabled = (fg, bg);
                                }
                                "focused" => {
                                    self.focused = (fg, bg);
                                }
                                "input" => {
                                    self.input = (fg, bg);
                                }
                                "select" => {
                                    self.select = (fg, bg);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    /// Compose color table from palette.
    ///
    /// # Return value
    ///
    /// Color table.
    pub fn colors(&self) -> Vec<(Color, i16, i16)> {
        vec![
            (Color::HexNorm, self.general.0, self.general.1),
            (Color::HexMod, self.modified.0, self.modified.1),
            (Color::HexDiff, self.diff.0, self.diff.1),
            (Color::HexNormHi, self.highlight.0, self.highlight.1),
            (Color::HexModHi, self.modified.0, self.highlight.1),
            (Color::HexDiffHi, self.diff.0, self.highlight.1),
            (Color::AsciiNorm, self.ascii.0, self.ascii.1),
            (Color::AsciiMod, self.modified.0, self.modified.1),
            (Color::AsciiDiff, self.diff.0, self.diff.1),
            (Color::AsciiNormHi, self.highlight.0, self.highlight.1),
            (Color::AsciiModHi, self.modified.0, self.highlight.1),
            (Color::AsciiDiffHi, self.diff.0, self.highlight.1),
            (Color::Offset, self.offset.0, self.offset.1),
            (Color::OffsetHi, self.highlight.0, self.highlight.1),
            (Color::Bar, self.bar.0, self.bar.1),
            (Color::Dialog, self.dialog.0, self.dialog.1),
            (Color::Error, self.error.0, self.error.1),
            (Color::Disabled, self.disabled.0, self.disabled.1),
            (Color::Focused, self.focused.0, self.focused.1),
            (Color::Input, self.input.0, self.input.1),
            (Color::Select, self.select.0, self.select.1),
        ]
    }
}
