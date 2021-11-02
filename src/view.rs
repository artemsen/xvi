// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::ascii::AsciiTable;
use super::config::Config;
use super::curses::{Color, Curses, Window};
use super::document::Document;
use std::collections::BTreeSet;
use unicode_segmentation::UnicodeSegmentation;

/// Document view.
pub struct View {
    /// Line width mode (fixed/dynamic).
    pub fixed_width: bool,
    /// ASCII characters table (None hides the field).
    pub ascii_table: Option<&'static AsciiTable>,

    /// Max offset (file size).
    pub max_offset: u64,

    /// Number of lines per page.
    pub lines: usize,
    /// Number of bytes per line.
    pub columns: usize,

    /// Window for the view.
    pub window: Window,
    /// Size of the offset field.
    pub offset_width: usize,
    /// Size of the hex field.
    pub hex_width: usize,

    /// Start address of currently displayed page.
    pub offset: u64,
    /// File data of currently displayed page.
    pub data: Vec<u8>,
    /// Addresses of changed values on the current page.
    pub changes: BTreeSet<u64>,
    /// Addresses of diff values on the current page.
    pub differs: BTreeSet<u64>,
}

impl View {
    const FIXED_WIDTH: usize = 4; // number of words per line in fixed mode
    const HEX_LEN: usize = 2; // length of a byte in hex representation
    const FIELD_MARGIN: usize = 3; // margin size between fields offset/hex/ascii
    const BYTE_MARGIN: usize = 1; // margin between bytes in a word
    const WORD_MARGIN: usize = 2; // margin between word
    const BYTES_IN_WORD: usize = 4; // number of bytes in a single word

    /// Create new viewer instance.
    pub fn new(config: &Config, file_size: u64) -> Self {
        Self {
            fixed_width: config.fixed_width,
            ascii_table: config.ascii_table,
            max_offset: file_size,
            lines: 0,
            columns: 0,
            window: Window::default(),
            offset_width: 0,
            hex_width: 0,
            offset: u64::MAX,
            data: Vec::new(),
            changes: BTreeSet::new(),
            differs: BTreeSet::new(),
        }
    }

    /// Recalculate the view scheme.
    ///
    /// # Arguments
    ///
    /// * `parent` - parent window
    pub fn resize(&mut self, parent: Window) {
        self.window = parent;

        // define size of the offset field
        self.offset_width = 4; // minimum 4 digits (u16)
        for i in (2..8).rev() {
            if u64::max_value() << (i * 8) & self.max_offset != 0 {
                self.offset_width = (i + 1) * 2;
                break;
            }
        }

        // calculate number of words per line
        let words = if self.fixed_width {
            View::FIXED_WIDTH
        } else {
            // calculate word width (number of chars per word)
            let hex_width = View::BYTES_IN_WORD * View::HEX_LEN
                + (View::BYTES_IN_WORD - 1) * View::BYTE_MARGIN
                + View::WORD_MARGIN;

            let ascii_width = if self.ascii_table.is_some() {
                View::BYTES_IN_WORD
            } else {
                0
            };
            let word_width = hex_width + ascii_width;

            // available space
            let mut free_space = self.window.width - self.offset_width - View::FIELD_MARGIN;
            if self.ascii_table.is_some() {
                free_space -= View::FIELD_MARGIN - View::WORD_MARGIN;
            } else {
                free_space += View::WORD_MARGIN;
            }

            // number of words per line
            free_space / word_width
        };

        self.lines = self.window.height - 1 /* status bar */;
        self.columns = words * View::BYTES_IN_WORD;

        // calculate hex field size
        let word_width =
            View::BYTES_IN_WORD * View::HEX_LEN + (View::BYTES_IN_WORD - 1) * View::BYTE_MARGIN;
        self.hex_width = words * word_width + (words - 1) * View::WORD_MARGIN;

        // increase the offset length if possible
        if self.offset_width < 8 {
            let mut free_space =
                self.window.width - self.offset_width - View::FIELD_MARGIN - self.hex_width;
            if self.ascii_table.is_some() {
                free_space -= View::FIELD_MARGIN + self.columns;
            }
            self.offset_width += free_space.min(8 - self.offset_width);
        }
    }

    /// Draw the document.
    ///
    /// # Arguments
    ///
    /// * `doc` - document to render
    pub fn draw(&self, doc: &Document) {
        self.draw_statusbar(doc);
        self.draw_text(doc);
        self.colorize(doc);
    }

    /// Print the status bar.
    ///
    /// # Arguments
    ///
    /// * `doc` - document to render
    fn draw_statusbar(&self, doc: &Document) {
        // right part: charset, position, etc
        let mut stat = String::new();
        let value = self.data[(doc.cursor.offset - self.offset) as usize];
        let percent = (doc.cursor.offset * 100
            / if doc.file.size > 1 {
                doc.file.size - 1
            } else {
                1
            }) as u8;
        if let Some(table) = self.ascii_table {
            stat = format!(" │ {}", table.id);
        };
        stat += &format!(
            " │ 0x{offset:04x} = 0x{value:02x} {value:<3} 0{value:<3o} {value:08b} │ {percent:>3}%",
            offset = doc.cursor.offset,
            value = value,
            percent = percent
        );

        let right_len = stat.graphemes(true).count();
        let left_len = self.window.width - right_len;

        // left part: path to the file and modifcation status
        let mut path = doc.file.path.clone();
        if doc.file.is_modified() {
            path.push('*');
        }
        let path_len = path.graphemes(true).count();
        if path_len > left_len {
            // shrink path string
            let cut_start = 3;
            let cut_end = cut_start + path_len - left_len + 1 /* delimiter */;
            let (index_start, grapheme_start) = path.grapheme_indices(true).nth(cut_start).unwrap();
            let (index_end, grapheme_end) = path.grapheme_indices(true).nth(cut_end).unwrap();
            let start = index_start + grapheme_start.len();
            let end = index_end + grapheme_end.len();
            path.replace_range(start..end, "…");
        }

        // draw status bar
        let statusbar = format!("{:<width$}{}", path, stat, width = left_len);
        self.window.print(0, 0, &statusbar);
        self.window.color(0, 0, self.window.width, Color::StatusBar);
    }

    /// Print the text representation of the current page.
    ///
    /// # Arguments
    ///
    /// * `doc` - document to render
    fn draw_text(&self, doc: &Document) {
        Curses::color_on(Color::HexNormal);
        let mut hex = String::with_capacity(self.columns * 3 + self.columns / View::BYTES_IN_WORD);
        let mut ascii = String::with_capacity(self.columns);

        for y in 0..self.lines {
            let offset = self.offset + (y * self.columns) as u64;
            let line = if offset >= doc.file.size {
                // fill with spaces to erase previous text
                (0..self.window.width).map(|_| ' ').collect::<String>()
            } else {
                // fill hex and ascii
                hex.clear();
                ascii.clear();
                for x in 0..self.columns {
                    if !hex.is_empty() {
                        hex.push(' '); // byte delimiter
                        if x % View::BYTES_IN_WORD == 0 {
                            hex.push(' '); // word delimiter
                        }
                    }
                    if let Some(&byte) = self.data.get((offset + x as u64 - self.offset) as usize) {
                        hex.push_str(&format!("{:02x}", byte));
                        if let Some(table) = self.ascii_table {
                            ascii.push(table.charset[byte as usize]);
                        }
                    } else {
                        hex.push_str("  ");
                        if self.ascii_table.is_some() {
                            ascii.push(' ');
                        }
                    }
                }
                // compose the final string
                let mut line = format!(
                    "{:0ow$x}{:fm$}{}",
                    offset,
                    "",
                    hex,
                    ow = self.offset_width,
                    fm = View::FIELD_MARGIN
                );
                if self.ascii_table.is_some() {
                    line.push_str(&format!("{:fm$}{}", "", ascii, fm = View::FIELD_MARGIN));
                }
                line
            };

            self.window.print(0, y + 1 /*status bar*/, &line);
        }
    }

    /// Colorize the view.
    ///
    /// # Arguments
    ///
    /// * `doc` - document to render
    fn colorize(&self, doc: &Document) {
        // calculate cursor position (indexes within the page data)
        let cursor_x = doc.cursor.offset as usize % self.columns;
        let cursor_y = (doc.cursor.offset - self.offset) as usize / self.columns;

        for y in 0..self.lines {
            if self.offset + (y * self.columns) as u64 >= doc.file.size {
                break;
            }
            let display_y = y + 1 /* status bar */;

            // colorize offset
            self.window.color(
                0,
                display_y,
                self.offset_width,
                if y == cursor_y {
                    Color::OffsetHi
                } else {
                    Color::OffsetNormal
                },
            );

            // highlight the current line in hex
            if y == cursor_y {
                self.window.color(
                    self.offset_width + View::FIELD_MARGIN,
                    display_y,
                    self.hex_width,
                    Color::HexHi,
                );
            }

            // colorize ascii
            if self.ascii_table.is_some() {
                self.window.color(
                    self.offset_width + self.hex_width + View::FIELD_MARGIN * 2,
                    display_y,
                    self.columns,
                    if y == cursor_y {
                        Color::AsciiHi
                    } else {
                        Color::AsciiNormal
                    },
                );
            }

            // highlight current column inside hex and ascii fields
            if y != cursor_y {
                let hex_x = self.offset_width
                    + View::FIELD_MARGIN
                    + cursor_x * (View::BYTES_IN_WORD - 1)
                    + cursor_x / View::BYTES_IN_WORD;
                self.window
                    .color(hex_x, display_y, View::HEX_LEN, Color::HexHi);

                let ascii_x =
                    self.offset_width + self.hex_width + View::FIELD_MARGIN * 2 + cursor_x;
                self.window.color(ascii_x, display_y, 1, Color::AsciiHi);
            }
        }

        // highlight diff
        for &offset in self
            .differs
            .range(self.offset..(self.offset + self.data.len() as u64))
        {
            let cx = offset as usize % self.columns;
            let cy = (offset - self.offset) as usize / self.columns;
            if let Some((x, y)) = self.get_position(offset, true) {
                let color = if cx == cursor_x || cy == cursor_y {
                    Color::HexDiffHi
                } else {
                    Color::HexDiff
                };
                self.window.color(x, y, View::HEX_LEN, color);
            }
            if let Some((x, y)) = self.get_position(offset, false) {
                let color = if cx == cursor_x || cy == cursor_y {
                    Color::AsciiDiffHi
                } else {
                    Color::AsciiDiff
                };
                self.window.color(x, y, 1, color);
            }
        }

        // highlight changes
        for &offset in self
            .changes
            .range(self.offset..(self.offset + self.data.len() as u64))
        {
            let cx = offset as usize % self.columns;
            let cy = (offset - self.offset) as usize / self.columns;
            if let Some((x, y)) = self.get_position(offset, true) {
                let color = if cx == cursor_x || cy == cursor_y {
                    Color::HexModifiedHi
                } else {
                    Color::HexModified
                };
                self.window.color(x, y, View::HEX_LEN, color);
            }
            if let Some((x, y)) = self.get_position(offset, false) {
                let color = if cx == cursor_x || cy == cursor_y {
                    Color::AsciiModifiedHi
                } else {
                    Color::AsciiModified
                };
                self.window.color(x, y, 1, color);
            }
        }
    }

    /// Get coordinates of specified offset inside the hex or ascii fields.
    ///
    /// # Arguments
    ///
    /// * `offset` - address of the byte
    /// * `hex` - field type, `true` for hex, `false` for ascii
    ///
    /// # Return value
    ///
    /// Coordinates of the byte relative to the view window.
    pub fn get_position(&self, offset: u64, hex: bool) -> Option<(usize, usize)> {
        if offset < self.offset {
            return None;
        }

        let line = (offset - self.offset) as usize / self.columns;
        if line >= self.lines {
            return None;
        }
        let y = line + 1 /* status bar */;

        let column = offset as usize % self.columns;
        let mut x = self.offset_width + View::FIELD_MARGIN;
        if hex {
            x += column * (View::BYTES_IN_WORD - 1) + column / View::BYTES_IN_WORD;
        } else {
            x += self.hex_width + View::FIELD_MARGIN + column;
        }

        Some((x, y))
    }
}
