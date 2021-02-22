// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::cui::*;
use super::cursor::*;
use std::collections::BTreeMap;

/// Data page
pub struct Page {
    pub offset: u64,    // start address
    pub data: Vec<u8>,  // raw data
    pub state: Vec<u8>, // byte states
}

impl Page {
    pub const DEFAULT: u8 = 0;
    pub const CHANGED: u8 = 1;

    /// Create instance
    pub fn new(offset: u64, data: Vec<u8>) -> Self {
        let state = vec![Page::DEFAULT; data.len()];
        Self {
            offset,
            data,
            state,
        }
    }

    /// Check if offset is visible (belongs to the page)
    pub fn visible(&self, offset: u64) -> bool {
        offset >= self.offset && offset < self.offset + self.data.len() as u64
    }

    /// Find sequence inside the page
    pub fn find(&self, seq: &[u8], start: u64) -> Option<u64> {
        if start < self.offset || start + seq.len() as u64 > self.offset + self.data.len() as u64 {
            return None;
        }
        let skip = (start - self.offset) as usize;
        if let Some(pos) = self.data[skip..]
            .windows(seq.len())
            .position(|window| window == seq)
        {
            Some(self.offset + (skip + pos) as u64)
        } else {
            None
        }
    }

    /// Get byte value with state
    pub fn get(&self, offset: u64) -> Option<(u8, u8)> {
        if !self.visible(offset) {
            None
        } else {
            let index = (offset - self.offset) as usize;
            Some((self.data[index], self.state[index]))
        }
    }

    /// Set byte value and state
    pub fn set(&mut self, offset: u64, value: u8, state: u8) {
        debug_assert!(offset >= self.offset && offset < self.offset + self.data.len() as u64);
        let index = (offset - self.offset) as usize;
        self.data[index] = value;
        self.state[index] = state;
    }

    /// Update page with changed data
    pub fn update(&mut self, changes: &BTreeMap<u64, u8>) {
        for index in 0..self.data.len() {
            let offset = self.offset + index as u64;
            if let Some(value) = changes.get(&offset) {
                self.set(offset, *value, Page::CHANGED);
            } else {
                self.state[index] = Page::DEFAULT;
            }
        }
    }
}

/// View of data page
pub struct PageView;
impl PageView {
    const MARGIN: usize = 3; // margin size around hex area
    const COLS: usize = 0x10; // the only supported scheme
    const OFFSET_LEN: usize = 8; // todo
    const HEX_LEN: usize = 2; // length of a byte in hex representation

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

    pub fn size(lines: usize) -> (usize, usize) {
        (lines, PageView::COLS)
    }

    /// Print page view (offset/hex/ascii), returns screen position of cursor
    pub fn print(canvas: &Canvas, page: &Page, cursor: &Cursor) -> (usize, usize) {
        // current position of cursor inside page
        let cursor_x = cursor.offset as usize % PageView::COLS;
        let cursor_y = (cursor.offset - page.offset - cursor.offset % PageView::COLS as u64)
            as usize
            / PageView::COLS;

        let offset = Canvas {
            cui: canvas.cui,
            x: canvas.x,
            y: canvas.y,
            width: PageView::OFFSET_LEN,
            height: canvas.height,
        };
        PageView::print_offsets(&offset, page, cursor_y);

        let hex = Canvas {
            cui: canvas.cui,
            x: offset.x + offset.width + PageView::MARGIN,
            y: canvas.y,
            width: PageView::hex_width(),
            height: canvas.height,
        };
        PageView::print_hexdump(&hex, page, cursor.place == Place::Hex, cursor_x, cursor_y);

        let ascii = Canvas {
            cui: canvas.cui,
            x: hex.x + hex.width + PageView::MARGIN,
            y: canvas.y,
            width: PageView::COLS,
            height: canvas.height,
        };
        PageView::print_ascii(
            &ascii,
            page,
            cursor.place == Place::Ascii,
            cursor_x,
            cursor_y,
        );

        // convert cursor position to screen coordiantes
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

    /// Print offsets of page content
    fn print_offsets(canvas: &Canvas, page: &Page, cursor_y: usize) {
        for y in 0..canvas.height {
            let offset = page.offset + (y * PageView::COLS) as u64;
            if offset > page.offset + page.data.len() as u64 {
                break;
            }
            let text = &format!("{:0w$x}", offset, w = PageView::OFFSET_LEN);
            let color = if y == cursor_y {
                Color::Active
            } else {
                Color::Passive
            };
            canvas.print(0, y, text);
            canvas.color(0, y, canvas.width, color);
        }
    }

    /// Print page content as hex dump text
    fn print_hexdump(canvas: &Canvas, page: &Page, active: bool, cursor_x: usize, cursor_y: usize) {
        debug_assert!(canvas.width == PageView::hex_width());
        for y in 0..canvas.height {
            // line background
            let color = if active && y == cursor_y {
                Color::ActiveHi
            } else {
                Color::Passive
            };
            canvas.color(0, y, canvas.width, color);

            for x in 0..PageView::COLS {
                let offset = page.offset + (y * PageView::COLS + x) as u64;
                let (text, color) = if let Some((byte, state)) = page.get(offset) {
                    (
                        format!("{:02x}", byte),
                        if state & Page::CHANGED != 0 {
                            if active && (cursor_y == y || cursor_x == x) {
                                Color::ChangedHi
                            } else {
                                Color::Changed
                            }
                        } else if cursor_y == y || cursor_x == x {
                            if active {
                                Color::ActiveHi
                            } else {
                                Color::Active
                            }
                        } else if active {
                            Color::Active
                        } else {
                            Color::Passive
                        },
                    )
                } else {
                    (
                        String::from("  "),
                        if active && (cursor_y == y || cursor_x == x) {
                            Color::ActiveHi
                        } else {
                            Color::Active
                        },
                    )
                };
                let pos = PageView::hex_x(x);
                canvas.print(pos, y, &text);
                canvas.color(pos, y, PageView::HEX_LEN, color);
            }
        }
    }

    /// Print page content as ASCII text
    fn print_ascii(canvas: &Canvas, page: &Page, active: bool, cursor_x: usize, cursor_y: usize) {
        debug_assert!(canvas.width == PageView::COLS);
        for y in 0..canvas.height {
            for x in 0..PageView::COLS {
                let offset = page.offset + (y * PageView::COLS + x) as u64;
                let (chr, color) = if let Some((byte, state)) = page.get(offset) {
                    (
                        PageView::CP437[byte as usize],
                        if state & Page::CHANGED != 0 {
                            if active && (cursor_y == y || cursor_x == x) {
                                Color::ChangedHi
                            } else {
                                Color::Changed
                            }
                        } else if cursor_y == y || cursor_x == x {
                            if active {
                                Color::ActiveHi
                            } else {
                                Color::Active
                            }
                        } else if active {
                            Color::Active
                        } else {
                            Color::Passive
                        },
                    )
                } else {
                    (
                        ' ',
                        if active && (cursor_y == y || cursor_x == x) {
                            Color::ActiveHi
                        } else {
                            Color::Active
                        },
                    )
                };
                let text = format!("{}", chr);
                canvas.print(x, y, &text);
                canvas.color(x, y, 1, color);
            }
        }
    }

    /// Return horizontal position of specified column
    fn hex_x(column: usize) -> usize {
        // "00 11 22 33  44 55 66 77  88 99 aa bb  cc dd ee ff"
        column * 3 + column / 4
    }

    /// Return length of a single line
    fn hex_width() -> usize {
        // "00 11 22 33  44 55 66 77  88 99 aa bb  cc dd ee ff"
        PageView::hex_x(PageView::COLS - 1) + PageView::HEX_LEN
    }
}
