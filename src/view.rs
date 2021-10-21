// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::ascii::AsciiTable;
use super::curses::{Color, Curses, Window};
use super::cursor::*;
use super::document::Document;
use super::page::Page;
use unicode_segmentation::UnicodeSegmentation;

/// Document view.
pub struct View {
    /// Line width mode (fixed/dynamic).
    pub fixed_width: bool,
    /// ASCII characters table (None hides the field).
    pub ascii_table: Option<&'static AsciiTable>,
    /// File size.
    pub file_size: u64,
    /// Number of lines per page.
    pub lines: usize,
    /// Number of bytes per line.
    pub columns: usize,
    /// Window for status bar.
    pub wnd_statusbar: Window,
    /// Window for drawing offsets.
    pub wnd_offset: Window,
    /// Window for drawing the hex field.
    pub wnd_hex: Window,
    /// Window for drawing the ASCII field.
    pub wnd_ascii: Window,
}

impl View {
    const FIXED_WIDTH: usize = 4; // number of words per line in fixed mode
    const HEX_LEN: usize = 2; // length of a byte in hex representation
    const FIELD_MARGIN: usize = 3; // margin size between fields offset/hex/ascii
    const BYTE_MARGIN: usize = 1; // margin between bytes in a word
    const WORD_MARGIN: usize = 2; // margin between word
    const BYTES_IN_WORD: usize = 4; // number of bytes in a single word

    /// Create new viewer instance.
    pub fn new(file_size: u64) -> Self {
        Self {
            fixed_width: true,
            ascii_table: None,
            file_size,
            lines: 0,
            columns: 0,
            wnd_statusbar: Window {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
            wnd_offset: Window {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
            wnd_hex: Window {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
            wnd_ascii: Window {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
        }
    }

    /// Recalculate the view scheme.
    pub fn resize(&mut self, parent: &Window) {
        // define size of offset field
        let mut offset_width: usize = 4; // minimum offset as for u16
        for i in (2..8).rev() {
            if u64::max_value() << (i * 8) & self.file_size != 0 {
                offset_width = (i + 1) * 2;
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
            let mut free_space = parent.width - offset_width - View::FIELD_MARGIN;
            if self.ascii_table.is_some() {
                free_space -= View::FIELD_MARGIN - View::WORD_MARGIN;
            } else {
                free_space += View::WORD_MARGIN;
            }

            // number of words per line
            free_space / word_width
        };

        self.lines = parent.height - 1 /* status bar */;
        self.columns = words * View::BYTES_IN_WORD;

        // calculate hex field size
        let word_width =
            View::BYTES_IN_WORD * View::HEX_LEN + (View::BYTES_IN_WORD - 1) * View::BYTE_MARGIN;
        let hex_width = words * word_width + (words - 1) * View::WORD_MARGIN;

        // increase the offset length if possible
        if offset_width < 8 {
            let mut free_space = parent.width - offset_width - View::FIELD_MARGIN - hex_width;
            if self.ascii_table.is_some() {
                free_space -= View::FIELD_MARGIN + self.columns;
            }
            offset_width += free_space.min(8 - offset_width);
        }

        // calculate subwindows size and position
        self.wnd_statusbar.x = parent.x;
        self.wnd_statusbar.y = parent.y;
        self.wnd_statusbar.width = parent.width;
        self.wnd_statusbar.height = 1;
        self.wnd_offset.x = parent.x;
        self.wnd_offset.y = self.wnd_statusbar.y + self.wnd_statusbar.height;
        self.wnd_offset.width = offset_width;
        self.wnd_offset.height = self.lines;
        self.wnd_hex.x = self.wnd_offset.x + self.wnd_offset.width + View::FIELD_MARGIN;
        self.wnd_hex.y = self.wnd_statusbar.y + self.wnd_statusbar.height;
        self.wnd_hex.width = hex_width;
        self.wnd_hex.height = self.lines;
        self.wnd_ascii.x = self.wnd_hex.x + self.wnd_hex.width + View::FIELD_MARGIN;
        self.wnd_ascii.y = self.wnd_statusbar.y + self.wnd_statusbar.height;
        self.wnd_ascii.width = self.columns;
        self.wnd_ascii.height = self.lines;
    }

    /// Render the document.
    pub fn draw(&self, doc: &Document) {
        self.draw_statusbar(doc);

        // calculate cursor position (indexes within the page data)
        let cursor_x = doc.cursor.offset as usize % self.columns;
        let cursor_y = (doc.cursor.offset - doc.page.offset) as usize / self.columns;

        // draw subwindows
        self.draw_offsets(&doc.page, cursor_y);
        self.draw_hex(&doc.page, cursor_x, cursor_y);
        if self.ascii_table.is_some() {
            self.draw_ascii(&doc.page, cursor_x, cursor_y);
        }
    }

    /// Draw status bar.
    fn draw_statusbar(&self, doc: &Document) {
        // right part: charset, position, etc
        let mut stat = String::new();
        let (value, _) = doc.page.get(doc.cursor.offset).unwrap();
        let percent = (doc.cursor.offset * 100 / if doc.size > 1 { doc.size - 1 } else { 1 }) as u8;
        if let Some(table) = self.ascii_table {
            stat = format!(" │ {}", table.id);
        };
        stat += &format!(
            " │ 0x{offset:04x} = 0x{:02x} {value:<3} 0{value:<3o} {value:08b} │ {percent:>3}%",
            offset = doc.cursor.offset,
            value = value,
            percent = percent
        );

        let right_len = stat.graphemes(true).count();
        let left_len = self.wnd_statusbar.width - right_len;

        // left part: path to the file and modifcation status
        let mut path = doc.path.clone();
        if doc.changes.has_changes() {
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
        self.wnd_statusbar.print(0, 0, &statusbar);
        self.wnd_statusbar
            .color(0, 0, self.wnd_statusbar.width, Color::StatusBar);
    }

    /// Print the offsets field.
    fn draw_offsets(&self, page: &Page, cursor_y: usize) {
        Curses::color_on(Color::OffsetNormal);
        for y in 0..self.wnd_offset.height {
            let offset = page.offset + (y * self.columns) as u64;
            if offset > self.file_size {
                break;
            }
            self.wnd_offset
                .print(0, y, &format!("{:0w$x}", offset, w = self.wnd_offset.width));
            if y == cursor_y {
                self.wnd_offset
                    .color(0, y, self.wnd_offset.width, Color::OffsetHi);
            }
        }
    }

    /// Print the hex field.
    fn draw_hex(&self, page: &Page, cursor_x: usize, cursor_y: usize) {
        for y in 0..self.wnd_hex.height {
            // line background
            let color = if y == cursor_y {
                Color::HexHi
            } else {
                Color::HexNormal
            };
            self.wnd_hex.color(0, y, self.wnd_hex.width, color);

            for x in 0..self.columns {
                let offset = page.offset + (y * self.columns + x) as u64;
                let (text, color) = if let Some((byte, state)) = page.get(offset) {
                    (
                        format!("{:02x}", byte),
                        if state & Page::CHANGED != 0 {
                            if y == cursor_y || x == cursor_x {
                                Color::HexModifiedHi
                            } else {
                                Color::HexModified
                            }
                        } else if y == cursor_y || x == cursor_x {
                            Color::HexHi
                        } else {
                            Color::HexNormal
                        },
                    )
                } else {
                    (
                        String::from("  "),
                        if y == cursor_y || x == cursor_x {
                            Color::HexHi
                        } else {
                            Color::HexNormal
                        },
                    )
                };
                let pos_x = x * (View::BYTES_IN_WORD - 1) + x / View::BYTES_IN_WORD;
                self.wnd_hex.print(pos_x, y, &text);
                self.wnd_hex.color(pos_x, y, View::HEX_LEN, color);
            }
        }
    }

    /// Print the ASCII field.
    fn draw_ascii(&self, page: &Page, cursor_x: usize, cursor_y: usize) {
        for y in 0..self.wnd_ascii.height {
            for x in 0..self.wnd_ascii.width {
                let offset = page.offset + (y * self.wnd_ascii.width + x) as u64;
                let (chr, color) = if let Some((byte, state)) = page.get(offset) {
                    (
                        self.ascii_table.unwrap().charset[byte as usize],
                        if state & Page::CHANGED != 0 {
                            if y == cursor_y || x == cursor_x {
                                Color::AsciiModifiedHi
                            } else {
                                Color::AsciiModified
                            }
                        } else if y == cursor_y || x == cursor_x {
                            Color::AsciiHi
                        } else {
                            Color::AsciiNormal
                        },
                    )
                } else {
                    (
                        ' ',
                        if y == cursor_y || x == cursor_x {
                            Color::AsciiHi
                        } else {
                            Color::AsciiNormal
                        },
                    )
                };
                let text = format!("{}", chr);
                self.wnd_ascii.print(x, y, &text);
                self.wnd_ascii.color(x, y, 1, color);
            }
        }
    }

    /// Get position of the byte at specified offset, returns absolute display coordinates.
    pub fn get_position(&self, base: u64, cursor: &Cursor) -> (usize, usize) {
        let col = cursor.offset as usize % self.columns;
        let line = (cursor.offset - base) as usize / self.columns;
        debug_assert!(line < self.lines);

        if cursor.place == Place::Ascii {
            (self.wnd_ascii.x + col, self.wnd_ascii.y + line)
        } else {
            (
                self.wnd_hex.x
                    + col * (View::BYTES_IN_WORD - 1)
                    + col / View::BYTES_IN_WORD
                    + if cursor.half == HalfByte::Left { 0 } else { 1 },
                self.wnd_hex.y + line,
            )
        }
    }
}
