// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::ascii::*;
use super::config;
use super::curses::{Color, Curses, Window};
use super::dialog::*;
use super::file::File;
use super::page::PageData;
use super::widget::*;
use unicode_segmentation::UnicodeSegmentation;

/// Hex document view.
pub struct View<'a> {
    pub scheme: &'a Scheme,
    pub config: &'a Config,
    pub page: &'a PageData,
    pub file: &'a File,
    pub offset: u64,
}
impl<'a> View<'a> {
    /// Print page view (offset/hex/ascii).
    pub fn draw(&self) {
        if self.config.statusbar {
            self.draw_statusbar();
        }
        self.draw_offsets();
        self.draw_hexdump();
        if self.config.ascii.is_some() {
            self.draw_ascii();
        }
        if self.config.keybar {
            self.draw_keybar();
        }
    }

    /// Print offsets.
    fn draw_offsets(&self) {
        let curr_y = (self.offset - self.page.offset) as usize / self.scheme.columns;
        Curses::color_on(Color::OffsetNormal);
        for y in 0..self.scheme.offset.height {
            let offset = self.page.offset + (y * self.scheme.columns) as u64;
            if offset > self.file.size {
                break;
            }
            self.scheme.offset.print(
                0,
                y,
                &format!("{:0w$x}", offset, w = self.scheme.offset.width),
            );
            if y == curr_y {
                self.scheme
                    .offset
                    .color(0, y, self.scheme.offset.width, Color::OffsetHi);
            }
        }
    }

    /// Print hex dump.
    fn draw_hexdump(&self) {
        let curr_x = self.offset as usize % self.scheme.columns;
        let curr_y = (self.offset - self.page.offset) as usize / self.scheme.columns;
        for y in 0..self.scheme.hex.height {
            // line background
            let color = if y == curr_y {
                Color::HexHi
            } else {
                Color::HexNormal
            };
            self.scheme.hex.color(0, y, self.scheme.hex.width, color);

            for x in 0..self.scheme.columns {
                let offset = self.page.offset + (y * self.scheme.columns + x) as u64;
                let (text, color) = if let Some((byte, state)) = self.page.get(offset) {
                    (
                        format!("{:02x}", byte),
                        if state & PageData::CHANGED != 0 {
                            if y == curr_y || x == curr_x {
                                Color::HexModifiedHi
                            } else {
                                Color::HexModified
                            }
                        } else if y == curr_y || x == curr_x {
                            Color::HexHi
                        } else {
                            Color::HexNormal
                        },
                    )
                } else {
                    (
                        String::from("  "),
                        if y == curr_y || x == curr_x {
                            Color::HexHi
                        } else {
                            Color::HexNormal
                        },
                    )
                };
                let pos_x = x * (Scheme::BYTES_IN_WORD - 1) + x / Scheme::BYTES_IN_WORD;
                self.scheme.hex.print(pos_x, y, &text);
                self.scheme.hex.color(pos_x, y, Scheme::HEX_LEN, color);
            }
        }
    }

    /// Print ASCII text.
    fn draw_ascii(&self) {
        let curr_x = self.offset as usize % self.scheme.columns;
        let curr_y = (self.offset - self.page.offset) as usize / self.scheme.columns;
        for y in 0..self.scheme.ascii.height {
            for x in 0..self.scheme.ascii.width {
                let offset = self.page.offset + (y * self.scheme.ascii.width + x) as u64;
                let (chr, color) = if let Some((byte, state)) = self.page.get(offset) {
                    (
                        self.config.ascii.unwrap().table[byte as usize],
                        if state & PageData::CHANGED != 0 {
                            if y == curr_y || x == curr_x {
                                Color::AsciiModifiedHi
                            } else {
                                Color::AsciiModified
                            }
                        } else if y == curr_y || x == curr_x {
                            Color::AsciiHi
                        } else {
                            Color::AsciiNormal
                        },
                    )
                } else {
                    (
                        ' ',
                        if y == curr_y || x == curr_x {
                            Color::AsciiHi
                        } else {
                            Color::AsciiNormal
                        },
                    )
                };
                let text = format!("{}", chr);
                self.scheme.ascii.print(x, y, &text);
                self.scheme.ascii.color(x, y, 1, color);
            }
        }
    }

    /// Draw status bar.
    fn draw_statusbar(&self) {
        // right part: charset, position, etc
        let (value, _) = self.page.get(self.offset).unwrap();
        let percent = (self.offset * 100
            / if self.file.size > 1 {
                self.file.size - 1
            } else {
                1
            }) as u8;
        let mut stat = String::new();
        if let Some(table) = self.config.ascii {
            stat = format!(" │ {}", table.id);
        };
        stat += &format!(
            " │ 0x{offset:04x} = 0x{:02x} {value:<3} 0{value:<3o} {value:08b} │ {percent:>3}%",
            offset = self.offset,
            value = value,
            percent = percent
        );
        let stat_len = stat.graphemes(true).count();
        self.scheme
            .statusbar
            .print(self.scheme.statusbar.width - stat_len, 0, &stat);

        // left part: file name
        let mut name: String;
        let max_len = self.scheme.statusbar.width - stat_len;
        let name_len = self.file.name.graphemes(true).count();
        if name_len <= max_len {
            name = self.file.name.clone();
        } else {
            // range replaced by a dilimeter
            let cut_start = 3;
            let cut_end = name_len - max_len + cut_start + 1 /* delimiter */;
            let (index, grapheme) = self
                .file
                .name
                .grapheme_indices(true)
                .nth(cut_start)
                .expect("Invalid position");
            let pos = index + grapheme.len();
            name = String::from(&self.file.name[..pos]);
            name.push('…'); // delimiter between file path parts
            let (index, grapheme) = self
                .file
                .name
                .grapheme_indices(true)
                .nth(cut_end)
                .expect("Invalid position");
            let pos = index + grapheme.len();
            name.push_str(&self.file.name[pos..]);
        }
        if self.file.is_modified() {
            name.push('*');
        }
        self.scheme.statusbar.print(0, 0, &name);

        self.scheme
            .statusbar
            .color(0, 0, self.scheme.statusbar.width, Color::StatusBar);
    }

    /// Draw key bar (bottom Fn line).
    fn draw_keybar(&self) {
        let titles = &[
            "Help", // F1
            "Save", // F2
            "",     // F3
            "",     // F4
            "Goto", // F5
            "",     // F6
            "Find", // F7
            "",     // F8
            "Mode", // F9
            "Exit", // F10
        ];

        let width = self.scheme.keybar.width / 10;
        for i in 0..10 {
            let x = i * width;
            let text = format!("{:>2}{}", i + 1, titles[i as usize]);
            self.scheme.keybar.print(x, 0, &text);
        }
        self.scheme
            .keybar
            .color(0, 0, self.scheme.keybar.width, Color::KeyBarTitle);
        for i in 0..10 {
            self.scheme.keybar.color(i * width, 0, 2, Color::KeyBarId);
        }
    }
}

/// Configuration of the view.
#[derive(Clone)]
pub struct Config {
    /// Line width mode (fixed/dynamic).
    pub fixed_width: bool,
    /// ASCII characters table.
    pub ascii: Option<&'static AsciiTable>,
    /// Show/hide status bar.
    pub statusbar: bool,
    /// Show/hide key bar.
    pub keybar: bool,
}
impl Config {
    pub fn new() -> Self {
        let cfg = config::Config::get();
        Self {
            fixed_width: cfg.fixed_width,
            ascii: cfg.ascii_charset,
            statusbar: cfg.show_statusbar,
            keybar: cfg.show_keybar,
        }
    }

    /// Show view's configuration dialog.
    pub fn setup(&mut self) -> bool {
        let mut dlg = Dialog::new(31, 10, DialogType::Normal, "View mode");
        let fixed = dlg.add_next(Checkbox::new("Fixed width (16 bytes)", self.fixed_width));
        let statusbar = dlg.add_next(Checkbox::new("Show status bar", self.statusbar));
        let keybar = dlg.add_next(Checkbox::new("Show key bar", self.keybar));
        dlg.add_separator();
        dlg.add_next(Text::new("ASCII field:"));

        let mut select = 0;
        let mut tables = Vec::with_capacity(ASCII_TABLES.len() + 1 /* None */);
        tables.push("None (hide)".to_string());
        for (index, table) in ASCII_TABLES.iter().enumerate() {
            tables.push(table.name.to_string());
            if let Some(current) = self.ascii {
                if current.id == table.id {
                    select = index + 1 /* "None (hide)" */;
                }
            }
        }
        let ascii = dlg.add_next(Listbox::new(tables, select));

        dlg.add_button(Button::std(StdButton::Ok, true));
        let btn_cancel = dlg.add_button(Button::std(StdButton::Cancel, false));
        dlg.cancel = btn_cancel;

        if let Some(id) = dlg.run() {
            if id != btn_cancel {
                if let WidgetData::Bool(value) = dlg.get(fixed) {
                    self.fixed_width = value;
                }
                if let WidgetData::Number(value) = dlg.get(ascii) {
                    self.ascii = if value == 0 {
                        None
                    } else {
                        ASCII_TABLES.get(value - 1)
                    }
                }
                if let WidgetData::Bool(value) = dlg.get(statusbar) {
                    self.statusbar = value;
                }
                if let WidgetData::Bool(value) = dlg.get(keybar) {
                    self.keybar = value;
                }
                return true;
            }
        }
        false
    }
}

/// Scheme of the view.
pub struct Scheme {
    /// Total number of lines.
    pub rows: usize,
    /// Total number of bytes per line.
    pub columns: usize,
    /// Length (width) of the offset field.
    pub offlen: usize,
    // size and position of internal windows
    pub statusbar: Window,
    pub keybar: Window,
    pub offset: Window,
    pub hex: Window,
    pub ascii: Window,
}
impl Scheme {
    const FIXED_WIDTH: usize = 4; // number of words per line in fixed mode
    const HEX_LEN: usize = 2; // length of a byte in hex representation
    const FIELD_MARGIN: usize = 3; // margin size between fields offset/hex/ascii
    const BYTE_MARGIN: usize = 1; // margin between bytes in a word
    const WORD_MARGIN: usize = 2; // margin between word
    const BYTES_IN_WORD: usize = 4; // number of bytes in a single word

    /// Create new viewer instance.
    pub fn new(wnd: &Window, config: &Config, offmax: u64) -> Self {
        // define size of offset field
        let mut offlen: usize = 4; // minimum offset as for u16
        for i in (2..8).rev() {
            if u64::max_value() << (i * 8) & offmax != 0 {
                offlen = (i + 1) * 2;
                break;
            }
        }

        // calculate number of words per line
        let words = if config.fixed_width {
            Scheme::FIXED_WIDTH
        } else {
            // calculate word width (number of chars per word)
            let hex_width = Scheme::BYTES_IN_WORD * Scheme::HEX_LEN
                + (Scheme::BYTES_IN_WORD - 1) * Scheme::BYTE_MARGIN
                + Scheme::WORD_MARGIN;

            let ascii_width = if config.ascii.is_some() {
                Scheme::BYTES_IN_WORD
            } else {
                0
            };
            let word_width = hex_width + ascii_width;

            // available space
            let mut free_space = wnd.width - offlen - Scheme::FIELD_MARGIN;
            if config.ascii.is_some() {
                free_space -= Scheme::FIELD_MARGIN - Scheme::WORD_MARGIN;
            } else {
                free_space += Scheme::WORD_MARGIN;
            }

            // number of words per line
            free_space / word_width
        };

        let columns = words * Scheme::BYTES_IN_WORD;
        let rows =
            wnd.height - if config.statusbar { 1 } else { 0 } - if config.keybar { 1 } else { 0 };

        // calculate hex field size
        let word_width = Scheme::BYTES_IN_WORD * Scheme::HEX_LEN
            + (Scheme::BYTES_IN_WORD - 1) * Scheme::BYTE_MARGIN;
        let hex_width = words * word_width + (words - 1) * Scheme::WORD_MARGIN;

        // increase the offset length if possible
        if offlen < 8 {
            let mut free_space = wnd.width - offlen - Scheme::FIELD_MARGIN - hex_width;
            if config.ascii.is_some() {
                free_space -= Scheme::FIELD_MARGIN + columns;
            }
            offlen += std::cmp::min(8 - offlen, free_space);
        }

        // calculate windows size and position
        let statusbar = Window {
            x: wnd.x,
            y: wnd.y,
            width: wnd.width,
            height: 1,
        };
        let keybar = Window {
            x: wnd.x,
            y: wnd.height - 1,
            width: wnd.width,
            height: 1,
        };
        let offset = Window {
            x: wnd.x,
            y: wnd.y + if config.statusbar { 1 } else { 0 },
            width: offlen,
            height: rows,
        };
        let hex = Window {
            x: offset.x + offset.width + Scheme::FIELD_MARGIN,
            y: wnd.y + if config.statusbar { 1 } else { 0 },
            width: hex_width,
            height: rows,
        };
        let ascii = Window {
            x: hex.x + hex.width + Scheme::FIELD_MARGIN,
            y: wnd.y + if config.statusbar { 1 } else { 0 },
            width: columns,
            height: rows,
        };

        Self {
            rows,
            columns,
            offlen,
            statusbar,
            keybar,
            offset,
            hex,
            ascii,
        }
    }

    /// Get position of the byte at specified offset, returns absolute display coordinates.
    pub fn position(&self, base: u64, offset: u64, hex: bool, lhb: bool) -> (usize, usize) {
        let col = offset as usize % self.columns;
        let row = (offset - base) as usize / self.columns;
        debug_assert!(row < self.rows);

        if hex {
            (
                self.hex.x
                    + col * (Scheme::BYTES_IN_WORD - 1)
                    + col / Scheme::BYTES_IN_WORD
                    + if lhb { 0 } else { 1 },
                self.hex.y + row,
            )
        } else {
            (self.ascii.x + col, self.ascii.y + row)
        }
    }
}
