// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::cui::*;
use super::cursor::*;
use std::collections::BTreeSet;

/// Page data buffer.
pub struct PageData {
    /// Page start address.
    pub offset: u64,
    /// Raw data.
    pub data: Vec<u8>,
    /// State map (changed, diff, etc).
    pub state: Vec<u8>,
}

impl PageData {
    pub const DEFAULT: u8 = 0;
    pub const CHANGED: u8 = 1;

    /// Create instance.
    pub fn new(offset: u64, data: Vec<u8>) -> Self {
        let state = vec![PageData::DEFAULT; data.len()];
        Self {
            offset,
            data,
            state,
        }
    }

    /// Check if offset is visible (belongs to the page).
    pub fn visible(&self, offset: u64) -> bool {
        offset >= self.offset && offset < self.offset + self.data.len() as u64
    }

    /// Get byte value with state.
    pub fn get(&self, offset: u64) -> Option<(u8, u8)> {
        if !self.visible(offset) {
            None
        } else {
            let index = (offset - self.offset) as usize;
            Some((self.data[index], self.state[index]))
        }
    }

    /// Set byte value and state.
    pub fn set(&mut self, offset: u64, value: u8, state: u8) {
        debug_assert!(offset >= self.offset && offset < self.offset + self.data.len() as u64);
        let index = (offset - self.offset) as usize;
        self.data[index] = value;
        self.state[index] = state;
    }

    /// Update page with changed data.
    pub fn update(&mut self, changes: &BTreeSet<u64>) {
        for index in 0..self.data.len() {
            let offset = self.offset + index as u64;
            self.state[index] = if changes.contains(&offset) {
                PageData::CHANGED
            } else {
                PageData::DEFAULT
            };
        }
    }
}

/// View of data page.
pub struct PageView {
    /// Max number or digits in the offset field.
    offset_max: usize,
    /// Current number of offset's digits
    offset_sz: usize,
    /// Wrap mode (false for dynamic line width).
    pub wrap: bool,
    /// Line width: number of showed bytes per line.
    pub columns: usize,
    /// Number of lines per page (height).
    pub lines: usize,
}

impl PageView {
    const WORD_SIZE: usize = 4; // size of the single word (in bytes)
    const MARGIN: usize = 3; // margin size around hex area
    const HEX_LEN: usize = 2; // length of a byte in hex representation

    pub fn new(fsize: u64, width: usize, height: usize) -> Self {
        // define size of offset field
        let mut offset_max: usize = 4; // minimum offset as u16
        for i in (2..8).rev() {
            if u64::max_value() << (i * 8) & fsize != 0 {
                offset_max = (i + 1) * 2;
                break;
            }
        }
        let mut instance = Self {
            offset_max,
            offset_sz: 0,
            wrap: false,
            columns: 0,
            lines: 0,
        };
        instance.resize(width, height);
        instance
    }

    /// Resize page (UI window resize handler).
    pub fn resize(&mut self, width: usize, height: usize) {
        self.lines = height;
        self.offset_sz = self.offset_max;

        let max_sz = width - self.offset_max - PageView::MARGIN * 2;
        let word_sz = PageView::WORD_SIZE * PageView::HEX_LEN +
                       3 /*margins between bytes*/ +
                       2 /*margins between blocks*/ +
                       PageView::WORD_SIZE /*ascii data*/;
        let max_words = max_sz / word_sz;

        if self.wrap {
            self.columns = 0x10;
        } else {
            self.columns = max_words * PageView::WORD_SIZE;
        }

        self.offset_sz = self.offset_max;
        if self.offset_sz < 8 && max_words * word_sz + 8 + PageView::MARGIN * 2 < width {
            self.offset_sz = 8;
        }
    }

    /// Print page view (offset/hex/ascii), returns screen position of cursor.
    pub fn print(&self, canvas: &Canvas, page: &PageData, cursor: &Cursor) -> (usize, usize) {
        // current position of cursor inside page
        let cursor_x = cursor.offset as usize % self.columns;
        let cursor_y = (cursor.offset - page.offset - cursor.offset % self.columns as u64) as usize
            / self.columns;

        let offset = Canvas {
            cui: canvas.cui,
            x: canvas.x,
            y: canvas.y,
            width: self.offset_sz,
            height: canvas.height,
        };
        self.print_offsets(&offset, page, cursor_y);

        let hex = Canvas {
            cui: canvas.cui,
            x: offset.x + offset.width + PageView::MARGIN,
            y: canvas.y,
            width: self.hex_width(),
            height: canvas.height,
        };
        self.print_hexdump(&hex, page, cursor_x, cursor_y);

        let ascii = Canvas {
            cui: canvas.cui,
            x: hex.x + hex.width + PageView::MARGIN,
            y: canvas.y,
            width: self.columns,
            height: canvas.height,
        };
        self.print_ascii(&ascii, page, cursor_x, cursor_y);

        // convert cursor position to screen coordinates
        if cursor.place == Place::Hex {
            let mut x_scr = hex.x + PageView::hex_x(cursor_x);
            if cursor.half == HalfByte::Right {
                x_scr += 1;
            }
            let y_scr = hex.y + cursor_y;
            (x_scr, y_scr)
        } else {
            (ascii.x + cursor_x, ascii.y + cursor_y)
        }
    }

    /// Print offsets of page content.
    fn print_offsets(&self, canvas: &Canvas, page: &PageData, cursor_y: usize) {
        canvas.color_on(Color::OffsetNormal);
        for y in 0..canvas.height {
            let offset = page.offset + (y * self.columns) as u64;
            if offset > page.offset + page.data.len() as u64 {
                break;
            }
            canvas.print(0, y, &format!("{:0w$x}", offset, w = canvas.width));
            if y == cursor_y {
                canvas.color(0, y, canvas.width, Color::OffsetHi);
            }
        }
    }

    /// Print page content as hex dump text.
    fn print_hexdump(&self, canvas: &Canvas, page: &PageData, cursor_x: usize, cursor_y: usize) {
        debug_assert!(canvas.width == self.hex_width());
        for y in 0..canvas.height {
            // line background
            let color = if y == cursor_y {
                Color::HexHi
            } else {
                Color::HexNormal
            };
            canvas.color(0, y, canvas.width, color);

            for x in 0..self.columns {
                let offset = page.offset + (y * self.columns + x) as u64;
                let (text, color) = if let Some((byte, state)) = page.get(offset) {
                    (
                        format!("{:02x}", byte),
                        if state & PageData::CHANGED != 0 {
                            if cursor_y == y || cursor_x == x {
                                Color::HexModifiedHi
                            } else {
                                Color::HexModified
                            }
                        } else if cursor_y == y || cursor_x == x {
                            Color::HexHi
                        } else {
                            Color::HexNormal
                        },
                    )
                } else {
                    (
                        String::from("  "),
                        if cursor_y == y || cursor_x == x {
                            Color::HexHi
                        } else {
                            Color::HexNormal
                        },
                    )
                };
                let pos = PageView::hex_x(x);
                canvas.print(pos, y, &text);
                canvas.color(pos, y, PageView::HEX_LEN, color);
            }
        }
    }

    /// Print page content as ASCII text.
    fn print_ascii(&self, canvas: &Canvas, page: &PageData, cursor_x: usize, cursor_y: usize) {
        debug_assert!(canvas.width == self.columns);
        for y in 0..canvas.height {
            for x in 0..self.columns {
                let offset = page.offset + (y * self.columns + x) as u64;
                let (chr, color) = if let Some((byte, state)) = page.get(offset) {
                    (
                        PageView::CP437[byte as usize],
                        if state & PageData::CHANGED != 0 {
                            if cursor_y == y || cursor_x == x {
                                Color::AsciiModifiedHi
                            } else {
                                Color::AsciiModified
                            }
                        } else if cursor_y == y || cursor_x == x {
                            Color::AsciiHi
                        } else {
                            Color::AsciiNormal
                        },
                    )
                } else {
                    (
                        ' ',
                        if cursor_y == y || cursor_x == x {
                            Color::AsciiHi
                        } else {
                            Color::AsciiNormal
                        },
                    )
                };
                let text = format!("{}", chr);
                canvas.print(x, y, &text);
                canvas.color(x, y, 1, color);
            }
        }
    }

    /// Return horizontal position of specified column.
    fn hex_x(column: usize) -> usize {
        column * 3 + column / PageView::WORD_SIZE
    }

    /// Return length of a single line.
    fn hex_width(&self) -> usize {
        PageView::hex_x(self.columns - 1) + PageView::HEX_LEN
    }

    /// ASCII view table (Code page 437).
    #[rustfmt::skip]
    const CP437: &'static [char] = &[
        ' ', '☺', '☻', '♥', '♦', '♣', '♠', '•', '◘', '○', '◙', '♂', '♀', '♪', '♫', '☼',
        '►', '◄', '↕', '‼', '¶', '§', '▬', '↨', '↑', '↓', '→', '←', '∟', '↔', '▲', '▼',
        ' ', '!', '"', '#', '$', '%', '&', '\'', '(', ')', '*', '+', ',', '-', '.', '/',
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', ':', ';', '<', '=', '>', '?',
        '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O',
        'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '[', '\\', ']', '^', '_',
        '`', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o',
        'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '{', '|', '}', '~', '⌂',
        'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å',
        'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ',
        'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»',
        '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕', '╣', '║', '╗', '╝', '╜', '╛', '┐',
        '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦', '╠', '═', '╬', '╧',
        '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐', '▀',
        'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩',
        '≡', '±', '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', ' ',
    ];

    /// ASCII view table (symbolic names).
    #[allow(dead_code)]
    #[rustfmt::skip]
    const CPSYM: &'static [char] = &[
        '␀', '␁', '␂', '␃', '␄', '␅', '␆', '␇', '␈', '␉', '␊', '␋', '␌', '␍', '␎', '␏',
        '␐', '␑', '␒', '␓', '␔', '␕', '␖', '␗', '␘', '␙', '␚', '␛', '␜', '␝', '␞', '␟',
        ' ', '!', '"', '#', '$', '%', '&', '\'', '(', ')', '*', '+', ',', '-', '.', '/',
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', ':', ';', '<', '=', '>', '?',
        '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O',
        'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '[', '\\', ']', '^', '_',
        '`', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o',
        'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '{', '|', '}', '~', '␡',
        '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·',
        '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·',
        '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·',
        '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·',
        '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·',
        '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·',
        '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·',
        '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·', '·',
    ];
}
