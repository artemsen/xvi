// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::curses::{Color, Curses, Window};
use super::file::File;
use super::page::PageData;

/// Configuration of the view.
#[derive(Clone)]
pub struct Config {
    /// Line width mode (fixed/dynamic).
    pub fixed_width: bool,
    /// Show/hide ascii field.
    pub ascii: bool,
    /// Show/hide status bar.
    pub statusbar: bool,
    /// Show/hide key bar.
    pub keybar: bool,
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
    const HEX_MARGIN: usize = 3; // margin size around hex area
    const BYTE_MARGIN: usize = 1; // margin between bytes in a word
    const WORD_MARGIN: usize = 2; // margin between word
    const BYTES_IN_WORD: usize = 4; // number of bytes in a single word
    const WORD_LENGTH: usize = Scheme::BYTES_IN_WORD * Scheme::HEX_LEN
        + (Scheme::BYTES_IN_WORD - 1) * Scheme::BYTE_MARGIN
        + Scheme::WORD_MARGIN;

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

        // length of a single displayed word
        let word_len = Scheme::WORD_LENGTH
            //+ View::WORD_MARGIN
            + if config.ascii { Scheme::BYTES_IN_WORD } else { 0 };
        // free space in line
        let free_space = wnd.width + Scheme::WORD_MARGIN
            - offlen
            - Scheme::HEX_MARGIN
            - if config.ascii { Scheme::HEX_MARGIN } else { 0 };
        // number of words per line
        let words = if config.fixed_width {
            Scheme::FIXED_WIDTH
        } else {
            // based on max number of words per line
            free_space / word_len
        };
        let columns = words * Scheme::BYTES_IN_WORD;

        let rows =
            wnd.height - if config.statusbar { 1 } else { 0 } - if config.keybar { 1 } else { 0 };

        // increase the offset length if possible
        let free_space = free_space - words * word_len;
        if offlen < 8 && free_space >= 8 {
            offlen = 8;
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
            x: offset.x + offset.width + Scheme::HEX_MARGIN,
            y: wnd.y + if config.statusbar { 1 } else { 0 },
            width: columns / Scheme::BYTES_IN_WORD * Scheme::WORD_LENGTH - Scheme::WORD_MARGIN,
            height: rows,
        };
        let ascii = Window {
            x: hex.x + hex.width + Scheme::HEX_MARGIN,
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

/// Hex document view.
pub struct View<'a> {
    pub scheme: &'a Scheme,
    pub config: &'a Config,
    pub page: &'a PageData,
    pub file: &'a File,
    pub offset: u64,
}

impl<'a> View<'a> {
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

    /// Print page view (offset/hex/ascii).
    pub fn draw(&self) {
        if self.config.statusbar {
            self.draw_statusbar();
        }
        self.draw_offsets();
        self.draw_hexdump();
        if self.config.ascii {
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
                        View::CP437[byte as usize],
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
        // right part: position, current value, etc
        let (value, _) = self.page.get(self.offset).unwrap();
        let percent = (self.offset * 100 / (self.file.size - 1)) as u8;
        let stat = format!(
            " {ch} [0x{:02x} {value:<3} 0{value:<3o} {value:08b}]     0x{offset:04x}   {percent:>3}%",
            value = value,
            offset =self.offset,
            percent = percent,
            ch = if self.file.is_modified() {'*'} else {' '}
        );
        self.scheme
            .statusbar
            .print(self.scheme.statusbar.width - stat.len(), 0, &stat);

        // left part: file name
        let max_len = self.scheme.statusbar.width - stat.len();
        if self.file.name.len() <= max_len {
            self.scheme.statusbar.print(0, 0, &self.file.name);
        } else {
            let mut name = String::from(&self.file.name[..3]);
            name.push('…');
            let vs = self.file.name.len() - max_len + 4;
            name.push_str(&self.file.name[vs..]);
            self.scheme.statusbar.print(0, 0, &name);
        }

        self.scheme
            .statusbar
            .color(0, 0, self.scheme.statusbar.width, Color::StatusBar);
    }

    /// Draw key bar (bottom Fn line).
    fn draw_keybar(&self) {
        let titles = &[
            "Help", // F1
            "Save", // F2
            if self.config.fixed_width {
                "UnWrap"
            } else {
                "Wrap"
            }, // F3
            "",     // F4
            "Goto", // F5
            "",     // F6
            "Find", // F7
            "",     // F8
            "",     // F9
            "Exit", // F10
        ];

        let fn_id_len: usize = 2; // function number length (f1-f0)
        let width = self.scheme.keybar.width / 10;
        for i in 0..10 {
            let x_num = i * width;
            self.scheme.keybar.print(x_num, 0, &format!("{:>2}", i + 1));
            self.scheme
                .keybar
                .color(x_num, 0, fn_id_len, Color::KeyBarId);
            let x_label = x_num + fn_id_len;
            self.scheme.keybar.print(x_label, 0, titles[i as usize]);
            self.scheme
                .keybar
                .color(x_label, 0, width - fn_id_len, Color::KeyBarTitle);
        }
    }
}
