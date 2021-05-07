// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::cui::*;
use super::cursor::*;
use super::file::*;
use super::page::*;

/// Document view.
pub struct View {
    /// Line width mode (fixed/dynamic).
    pub fixed_width: bool,
    /// Show/hide ascii field.
    pub ascii: bool,
    /// Show/hide status bar.
    pub statusbar: bool,
    /// Show/hide key bar.
    pub keybar: bool,
}

impl View {
    const FIXED_WIDTH: usize = 4; // number of words per line in fixed mode
    const HEX_LEN: usize = 2; // length of a byte in hex representation
    const HEX_MARGIN: usize = 3; // margin size around hex area
    const BYTE_MARGIN: usize = 1; // margin between bytes in a word
    const WORD_MARGIN: usize = 2; // margin between word
    const BYTES_IN_WORD: usize = 4; // number of bytes in a single word
    const WORD_LENGTH: usize = View::BYTES_IN_WORD * View::HEX_LEN
        + (View::BYTES_IN_WORD - 1) * View::BYTE_MARGIN
        + View::WORD_MARGIN;

    /// Get the viewer scheme.
    /// Returns the tuple with:
    /// - number of rows;
    /// - number of columns (bytes per line);
    /// - length of the offset field;
    pub fn get_scheme(&self, width: usize, height: usize, file_size: u64) -> (usize, usize, usize) {
        // define size of offset field
        let mut offset: usize = 4; // minimum offset as for u16
        for i in (2..8).rev() {
            if u64::max_value() << (i * 8) & file_size != 0 {
                offset = (i + 1) * 2;
                break;
            }
        }

        // length of a single displayed word
        let word_len = View::WORD_LENGTH
            //+ View::WORD_MARGIN
            + if self.ascii { View::BYTES_IN_WORD } else { 0 };
        // free space in line
        let free_space = width + View::WORD_MARGIN
            - offset
            - View::HEX_MARGIN
            - if self.ascii { View::HEX_MARGIN } else { 0 };
        // number of words per line
        let words = if self.fixed_width {
            View::FIXED_WIDTH
        } else {
            // based on max number of words per line
            free_space / word_len
        };
        let columns = words * View::BYTES_IN_WORD;

        let rows = height - if self.statusbar { 1 } else { 0 } - if self.keybar { 1 } else { 0 };

        // increase the offset length if possible
        let free_space = free_space - words * word_len;
        if offset < 8 && free_space >= 8 {
            offset = 8;
        }

        (rows, columns, offset)
    }

    /// Print page view (offset/hex/ascii), returns screen position of cursor.
    pub fn draw(
        &self,
        canvas: &Canvas,
        page: &PageData,
        cursor: &Cursor,
        file: &File,
    ) -> (usize, usize) {
        let (rows, columns, offsets) = self.get_scheme(canvas.width, canvas.height, file.size);

        // cursor coordinates inside hex/ascii field
        let cursor_x = cursor.offset as usize % columns;
        let cursor_y = (cursor.offset - page.offset) as usize / columns;

        // status bar
        if self.statusbar {
            let bar = Canvas {
                cui: canvas.cui,
                x: canvas.x,
                y: canvas.y,
                width: canvas.width,
                height: 1,
            };
            self.draw_statusbar(&bar, page, file, cursor);
            //bar.print(0, 0, &format!("{:x} {} {}x{}", page.offset, cursor.offset - page.offset, cursor_x, cursor_y));
        }

        // offsets (addresses)
        let offset = Canvas {
            cui: canvas.cui,
            x: canvas.x,
            y: canvas.y + if self.statusbar { 1 } else { 0 },
            width: offsets,
            height: rows,
        };
        self.draw_offsets(&offset, page.offset, file.size, columns, cursor.offset);

        // hex dump
        let hex = Canvas {
            cui: canvas.cui,
            x: offset.x + offset.width + View::HEX_MARGIN,
            y: canvas.y + if self.statusbar { 1 } else { 0 },
            width: columns / View::BYTES_IN_WORD * View::WORD_LENGTH - View::WORD_MARGIN,
            height: rows,
        };
        self.draw_hexdump(&hex, page, cursor_x, cursor_y);

        // cursor coordinates on the main window
        let mut cr_wnd = (
            hex.x
                + cursor_x * (View::BYTES_IN_WORD - 1)
                + cursor_x / View::BYTES_IN_WORD
                + if cursor.half == HalfByte::Right { 1 } else { 0 },
            hex.y + cursor_y,
        );

        // ascii data
        if self.ascii {
            let ascii = Canvas {
                cui: canvas.cui,
                x: hex.x + hex.width + View::HEX_MARGIN,
                y: canvas.y + if self.statusbar { 1 } else { 0 },
                width: columns,
                height: rows,
            };
            self.draw_ascii(&ascii, page, cursor_x, cursor_y);
            if cursor.place == Place::Ascii {
                cr_wnd = (ascii.x + cursor_x, ascii.y + cursor_y);
            }
        }

        // key bar
        if self.keybar {
            let bar = Canvas {
                cui: canvas.cui,
                x: canvas.x,
                y: canvas.height - 1,
                width: canvas.width,
                height: 1,
            };
            self.draw_keybar(&bar);
        }

        cr_wnd
    }

    /// Print offsets.
    fn draw_offsets(&self, canvas: &Canvas, start: u64, end: u64, step: usize, current: u64) {
        canvas.color_on(Color::OffsetNormal);
        for y in 0..canvas.height {
            let offset = start + (y * step) as u64;
            if offset > end {
                break;
            }
            canvas.print(0, y, &format!("{:0w$x}", offset, w = canvas.width));
            if current >= offset && current < offset + step as u64 {
                canvas.color(0, y, canvas.width, Color::OffsetHi);
            }
        }
    }

    /// Print hex dump.
    fn draw_hexdump(&self, canvas: &Canvas, page: &PageData, cursor_x: usize, cursor_y: usize) {
        let columns = (canvas.width + View::WORD_MARGIN) / View::WORD_LENGTH * View::BYTES_IN_WORD;
        for y in 0..canvas.height {
            // line background
            let color = if y == cursor_y {
                Color::HexHi
            } else {
                Color::HexNormal
            };
            canvas.color(0, y, canvas.width, color);

            for x in 0..columns {
                let offset = page.offset + (y * columns + x) as u64;
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
                let pos_x = x * (View::BYTES_IN_WORD - 1) + x / View::BYTES_IN_WORD;
                canvas.print(pos_x, y, &text);
                canvas.color(pos_x, y, View::HEX_LEN, color);
            }
        }
    }

    /// Print ASCII text.
    fn draw_ascii(&self, canvas: &Canvas, page: &PageData, cursor_x: usize, cursor_y: usize) {
        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let offset = page.offset + (y * canvas.width + x) as u64;
                let (chr, color) = if let Some((byte, state)) = page.get(offset) {
                    (
                        View::CP437[byte as usize],
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

    /// Draw status bar.
    fn draw_statusbar(&self, canvas: &Canvas, page: &PageData, file: &File, cursor: &Cursor) {
        // right part: position, current value, etc
        let (value, _) = page.get(cursor.offset).unwrap();
        let percent = (cursor.offset * 100 / (file.size - 1)) as u8;
        let stat = format!(
            " {ch} [0x{:02x} {value:<3} 0{value:<3o} {value:08b}]     0x{offset:04x}   {percent:>3}%",
            value = value,
            offset = cursor.offset,
            percent = percent,
            ch = if file.is_modified() {'*'} else {' '}
        );
        canvas.print(canvas.width - stat.len(), 0, &stat);

        // left part: file name
        let max_len = canvas.width - stat.len();
        if file.name.len() <= max_len {
            canvas.print(0, 0, &file.name);
        } else {
            let mut name = String::from(&file.name[..3]);
            name.push('…');
            let vs = file.name.len() - max_len + 4;
            name.push_str(&file.name[vs..]);
            canvas.print(0, 0, &name);
        }

        canvas.color(0, 0, canvas.width, Color::StatusBar);
    }

    /// Draw key bar (bottom Fn line).
    fn draw_keybar(&self, canvas: &Canvas) {
        let titles = &[
            "Help",                                           // F1
            "Save",                                           // F2
            if self.fixed_width { "UnWrap" } else { "Wrap" }, // F3
            "",                                               // F4
            "Goto",                                           // F5
            "",                                               // F6
            "Find",                                           // F7
            "",                                               // F8
            "",                                               // F9
            "Exit",                                           // F10
        ];

        let fn_id_len: usize = 2; // function number length (f1-f0)
        let width = canvas.width / 10;
        for i in 0..10 {
            let x_num = i * width;
            canvas.print(x_num, 0, &format!("{:>2}", i + 1));
            canvas.color(x_num, 0, fn_id_len, Color::KeyBarId);
            let x_label = x_num + fn_id_len;
            canvas.print(x_label, 0, titles[i as usize]);
            canvas.color(x_label, 0, width - fn_id_len, Color::KeyBarTitle);
        }
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
}
