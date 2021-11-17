// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::ascii::Table;
use super::config::Config;
use super::curses::{Color, Window};
use super::editor::Document;
use std::collections::BTreeSet;
use unicode_segmentation::UnicodeSegmentation;

/// Document view.
pub struct View {
    /// Line width mode (fixed/dynamic).
    pub fixed_width: bool,
    /// ASCII characters table (None hides the field).
    pub ascii_table: Option<&'static Table>,

    /// Max offset (file size).
    pub max_offset: u64,

    /// Number of lines per page.
    pub lines: usize,
    /// Number of bytes per line.
    pub columns: usize,

    /// Workspace window (offsets and data).
    pub workspace: Window,
    /// Status bar window.
    statusbar: Window,
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
    /// Number of words per line in fixed mode.
    const FIXED_WIDTH: usize = 4;
    /// Length of a byte in hex representation.
    const HEX_LEN: usize = 2;
    /// Margin size between fields offset/hex/ascii.
    const FIELD_MARGIN: usize = 3;
    /// Margin between bytes in a word.
    const BYTE_MARGIN: usize = 1;
    /// Margin between word.
    const WORD_MARGIN: usize = 2;
    /// Number of bytes in a single word.
    const BYTES_IN_WORD: usize = 4;

    /// Min width of the screen.
    pub const MIN_WIDTH: usize = 30;
    /// Min height of the window (status bar and at least one line of data).
    pub const MIN_HEIGHT: usize = 2;

    /// Create new viewer instance.
    ///
    /// # Arguments
    ///
    /// * `config` - application config
    /// * `file_size` - file size
    ///
    /// # Return value
    ///
    /// Viewer instance.
    pub fn new(config: &Config, file_size: u64) -> Self {
        Self {
            fixed_width: config.fixed_width,
            ascii_table: config.ascii_table,
            max_offset: file_size,
            lines: 1,
            columns: 1,
            workspace: Window::new(0, 0, 0, 0, Color::HexNorm),
            statusbar: Window::new(0, 0, 0, 0, Color::Bar),
            offset_width: 0,
            hex_width: 0,
            offset: 0,
            data: Vec::new(),
            changes: BTreeSet::new(),
            differs: BTreeSet::new(),
        }
    }

    /// Reinitialization.
    pub fn reinit(&mut self) {
        let (width, height) = self.workspace.get_size();

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
            let mut free_space = width - self.offset_width - View::FIELD_MARGIN;
            if self.ascii_table.is_some() {
                free_space -= View::FIELD_MARGIN - View::WORD_MARGIN;
            } else {
                free_space += View::WORD_MARGIN;
            }

            // number of words per line
            free_space / word_width
        };
        debug_assert_ne!(words, 0); // window too small?

        self.lines = height;
        self.columns = words * View::BYTES_IN_WORD;

        // calculate hex field size
        let word_width =
            View::BYTES_IN_WORD * View::HEX_LEN + (View::BYTES_IN_WORD - 1) * View::BYTE_MARGIN;
        self.hex_width = words * word_width + (words - 1) * View::WORD_MARGIN;

        // increase the offset length if possible
        let data_width = View::FIELD_MARGIN
            + self.hex_width
            + self.offset_width
            + if self.ascii_table.is_some() {
                View::FIELD_MARGIN + self.columns
            } else {
                0
            };
        if data_width < width && self.offset_width < 8 {
            let free_space = width - data_width;
            let max_offset_len = 8 - self.offset_width;
            self.offset_width += free_space.min(max_offset_len);
        }
    }

    /// Window resize handler: recalculate the view scheme.
    ///
    /// # Arguments
    ///
    /// * `y` - start line on the screen
    /// * `width` - size of the viewer
    /// * `height` - size of the viewer
    pub fn resize(&mut self, y: usize, width: usize, height: usize) {
        debug_assert!(width >= View::MIN_WIDTH);
        debug_assert!(height >= View::MIN_HEIGHT);

        self.statusbar.resize(width, 1);
        self.statusbar.set_pos(0, y);
        self.workspace.resize(width, height - 1);
        self.workspace.set_pos(0, y + 1);
        self.reinit();
    }

    /// Draw the document.
    ///
    /// # Arguments
    ///
    /// * `doc` - document to render
    pub fn draw(&self, doc: &Document) {
        // draw statusbar
        self.statusbar.clear();
        self.draw_statusbar(doc);
        self.statusbar.refresh();

        // draw workspace
        self.workspace.clear();
        self.draw_offset(doc);
        self.draw_hex(doc);
        if self.ascii_table.is_some() {
            self.draw_ascii(doc);
        }
        self.highlight(doc);
        self.workspace.refresh();
    }

    /// Print the status bar.
    ///
    /// # Arguments
    ///
    /// * `doc` - document to render
    fn draw_statusbar(&self, doc: &Document) {
        let (width, _) = self.statusbar.get_size();

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
            stat = format!(" \u{2502} {}", table.id);
        };
        stat += &format!(
            " \u{2502} 0x{offset:04x} = 0x{value:02x} {value:<3} 0{value:<3o} {value:08b} \u{2502} {percent:>3}%",
            offset = doc.cursor.offset,
            value = value,
            percent = percent
        );
        let stat_len = stat.graphemes(true).count();

        // left part: path to the file and modification status (at least 5 chars)
        let path_min = 5;
        let path_max = if stat_len < width - path_min {
            width - stat_len
        } else {
            path_min
        };
        let mut path = doc.file.path.clone();
        if doc.file.is_modified() {
            path.push('*');
        }
        let path_len = path.graphemes(true).count();
        if path_len > path_max {
            let cut_start = 3;
            let cut_end = path_len - path_max + cut_start + 1 /*delimiter*/;
            let (start, _) = path.grapheme_indices(true).nth(cut_start).unwrap();
            let (end, _) = path.grapheme_indices(true).nth(cut_end).unwrap();
            path.replace_range(start..end, "\u{2026}");
        }

        // compose and print the final status bar line
        let mut statusbar = format!("{:<width$}{}", path, stat, width = path_max);
        if let Some((max, _)) = statusbar.grapheme_indices(true).nth(width) {
            statusbar.truncate(max);
        }
        self.statusbar.print(0, 0, &statusbar);
    }

    /// Print the offset column.
    ///
    /// # Arguments
    ///
    /// * `doc` - document to render
    fn draw_offset(&self, doc: &Document) {
        self.workspace.color_on(Color::Offset);
        let cursor_y = (doc.cursor.offset - self.offset) as usize / self.columns;

        for y in 0..=self.lines {
            let offset = self.offset + (y * self.columns) as u64;
            if offset > doc.file.size {
                break;
            }
            if cursor_y == y {
                self.workspace.color_on(Color::OffsetHi);
            }
            let line = format!("{:0width$x}", offset, width = self.offset_width);
            self.workspace.print(0, y, &line);
            if cursor_y == y {
                self.workspace.color_on(Color::Offset);
            }
        }
    }

    /// Print the hex field.
    ///
    /// # Arguments
    ///
    /// * `doc` - document to render
    fn draw_hex(&self, doc: &Document) {
        self.workspace.color_on(Color::HexNorm);

        let cursor_x = (doc.cursor.offset % self.columns as u64) as usize;
        let cursor_y = (doc.cursor.offset - self.offset) as usize / self.columns;
        let left_pos = self.offset_width + View::FIELD_MARGIN;

        for y in 0..=self.lines {
            let offset = self.offset + (y * self.columns) as u64;
            if offset >= doc.file.size {
                break;
            }
            // fill with hex dump
            let mut text = String::with_capacity(self.hex_width);
            for x in 0..self.columns {
                if !text.is_empty() {
                    text.push(' '); // byte delimiter
                    if x % View::BYTES_IN_WORD == 0 {
                        text.push(' '); // word delimiter
                    }
                }
                if let Some(&byte) = self.data.get((offset + x as u64 - self.offset) as usize) {
                    text.push_str(&format!("{:02x}", byte));
                } else {
                    text.push_str("  "); // fill with spaces for highlighting
                }
            }

            if cursor_y == y {
                self.workspace.color_on(Color::HexNormHi);
            }
            self.workspace.print(left_pos, y, &text);
            if cursor_y == y {
                self.workspace.color_on(Color::HexNorm);
            } else {
                // highlight current column
                let col_x = left_pos
                    + cursor_x * (View::BYTES_IN_WORD - 1)
                    + cursor_x / View::BYTES_IN_WORD;
                self.workspace
                    .color(col_x, y, View::HEX_LEN, Color::HexNormHi);
            }
        }
    }

    /// Print the ascii field.
    ///
    /// # Arguments
    ///
    /// * `doc` - document to render
    fn draw_ascii(&self, doc: &Document) {
        self.workspace.color_on(Color::AsciiNorm);

        let cursor_x = (doc.cursor.offset % self.columns as u64) as usize;
        let cursor_y = (doc.cursor.offset - self.offset) as usize / self.columns;
        let left_pos = self.offset_width + self.hex_width + View::FIELD_MARGIN * 2;

        let ascii_table = self.ascii_table.unwrap();

        for y in 0..=self.lines {
            let offset = self.offset + (y * self.columns) as u64;
            if offset >= doc.file.size {
                break;
            }
            let text = (0..self.columns)
                .map(|i| {
                    let index = (offset + i as u64 - self.offset) as usize;
                    if let Some(&byte) = self.data.get(index) {
                        ascii_table.charset[byte as usize]
                    } else {
                        ' '
                    }
                })
                .collect::<String>();

            if cursor_y == y {
                self.workspace.color_on(Color::AsciiNormHi);
            }
            self.workspace.print(left_pos, y, &text);
            if cursor_y == y {
                self.workspace.color_on(Color::AsciiNorm);
            } else {
                // highlight current column
                let col_x = left_pos + cursor_x;
                self.workspace.color(col_x, y, 1, Color::AsciiNormHi);
            }
        }
    }

    /// Highlight changes and diffs.
    ///
    /// # Arguments
    ///
    /// * `doc` - document to render
    fn highlight(&self, doc: &Document) {
        // calculate cursor position (indexes within the page data)
        let cursor_x = (doc.cursor.offset % self.columns as u64) as usize;
        let cursor_y = (doc.cursor.offset - self.offset) as usize / self.columns;

        // highlight diff
        for &offset in &self.differs {
            let cx = offset as usize % self.columns;
            let cy = (offset - self.offset) as usize / self.columns;
            if let Some((x, y)) = self.get_position(offset, true) {
                let color = if cx == cursor_x || cy == cursor_y {
                    Color::HexDiffHi
                } else {
                    Color::HexDiff
                };
                self.workspace.color(x, y, View::HEX_LEN, color);
            }
            if self.ascii_table.is_some() {
                if let Some((x, y)) = self.get_position(offset, false) {
                    let color = if cx == cursor_x || cy == cursor_y {
                        Color::AsciiDiffHi
                    } else {
                        Color::AsciiDiff
                    };
                    self.workspace.color(x, y, 1, color);
                }
            }
        }

        // highlight changes
        for &offset in &self.changes {
            let cx = offset as usize % self.columns;
            let cy = (offset - self.offset) as usize / self.columns;
            if let Some((x, y)) = self.get_position(offset, true) {
                let color = if cx == cursor_x || cy == cursor_y {
                    Color::HexModHi
                } else {
                    Color::HexMod
                };
                self.workspace.color(x, y, View::HEX_LEN, color);
            }
            if self.ascii_table.is_some() {
                if let Some((x, y)) = self.get_position(offset, false) {
                    let color = if cx == cursor_x || cy == cursor_y {
                        Color::AsciiModHi
                    } else {
                        Color::AsciiMod
                    };
                    self.workspace.color(x, y, 1, color);
                }
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

        let y = (offset - self.offset) as usize / self.columns;
        if y >= self.lines {
            return None;
        }

        let column = (offset % self.columns as u64) as usize;
        let mut x = self.offset_width + View::FIELD_MARGIN;
        if hex {
            x += column * (View::BYTES_IN_WORD - 1) + column / View::BYTES_IN_WORD;
        } else {
            x += self.hex_width + View::FIELD_MARGIN + column;
        }

        Some((x, y))
    }
}
