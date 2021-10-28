// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::page::Page;

/// Cursor position and movement.
pub struct Cursor {
    /// Absolute offset (current position) of the cursor.
    pub offset: u64,
    /// Position inside a hex byte (left/right half byte).
    pub half: HalfByte,
    /// Edit mode (hex/ascii).
    pub place: Place,
}

impl Cursor {
    const WORD_SIZE: u64 = 4;

    /// Create new cursor instance.
    pub fn new() -> Self {
        Self {
            offset: u64::MAX,
            half: HalfByte::Left,
            place: Place::Hex,
        }
    }

    /// Set current place (ascii/hex).
    pub fn set_place(&mut self, place: Place) {
        self.place = place;
        self.half = HalfByte::Left;
    }

    /// Move cursor.
    ///
    /// # Arguments
    ///
    /// * `dir` - move direction
    /// * `page` - currently displayed page
    /// * `max` - maximum possible offset (file size)
    ///
    /// # Return value
    ///
    /// New page offset.
    pub fn move_to(&mut self, dir: &Direction, page: &Page, max: u64) -> u64 {
        let page_size = (page.lines * page.columns) as u64;
        let mut new_base = page.offset;

        match dir {
            Direction::PrevHalf => {
                if self.place == Place::Hex {
                    if self.half == HalfByte::Right {
                        self.half = HalfByte::Left;
                    } else if self.offset != 0 {
                        self.half = HalfByte::Right;
                        self.offset -= 1;
                    }
                } else if self.offset != 0 {
                    self.half = HalfByte::Left;
                    self.offset -= 1;
                }
                if self.offset < new_base {
                    new_base -= page.columns as u64;
                }
            }
            Direction::NextHalf => {
                if self.offset == max - 1
                    || (self.place == Place::Hex && self.half == HalfByte::Left)
                {
                    self.half = HalfByte::Right;
                } else {
                    self.half = HalfByte::Left;
                    self.offset += 1;
                    if self.offset >= new_base + page_size {
                        new_base += page.columns as u64;
                    }
                }
            }
            Direction::PrevByte => {
                self.half = HalfByte::Left;
                if self.offset != 0 {
                    self.offset -= 1;
                    if self.offset < new_base {
                        new_base -= page.columns as u64;
                    }
                }
            }
            Direction::NextByte => {
                self.half = HalfByte::Left;
                if self.offset < max - 1 {
                    self.offset += 1;
                    if self.offset >= new_base + page_size {
                        new_base += page.columns as u64;
                    }
                }
            }
            Direction::PrevWord => {
                if self.offset != 0 {
                    self.offset -= Cursor::WORD_SIZE - self.offset % Cursor::WORD_SIZE;
                    if self.offset < new_base {
                        new_base -= page.columns as u64;
                    }
                }
                self.half = HalfByte::Left;
            }
            Direction::NextWord => {
                self.offset += Cursor::WORD_SIZE - self.offset % Cursor::WORD_SIZE;
                if self.offset > max - 1 {
                    self.offset = max - 1;
                }
                if self.offset >= new_base + page_size {
                    new_base += page.columns as u64;
                }
                self.half = HalfByte::Left;
            }
            Direction::LineBegin => {
                self.offset -= self.offset % (page.columns as u64);
                self.half = HalfByte::Left;
            }
            Direction::LineEnd => {
                self.offset += page.columns as u64 - self.offset % (page.columns as u64) - 1;
                if self.offset > max - 1 {
                    self.offset = max - 1;
                }
                self.half = HalfByte::Left;
            }
            Direction::LineUp => {
                if self.offset >= page.columns as u64 {
                    self.offset -= page.columns as u64;
                    if self.offset < new_base {
                        new_base -= page.columns as u64;
                    }
                }
            }
            Direction::LineDown => {
                if self.offset + (page.columns as u64) < max {
                    self.offset += page.columns as u64;
                } else if self.offset + (page.columns as u64) - self.offset % (page.columns as u64)
                    < max
                {
                    self.offset = max - 1;
                    self.half = HalfByte::Left;
                }
                if self.offset >= new_base + page_size {
                    new_base += page.columns as u64;
                }
            }
            Direction::ScrollUp => {
                if new_base != 0 {
                    new_base -= page.columns as u64;
                    self.offset -= page.columns as u64;
                }
            }
            Direction::ScrollDown => {
                if new_base + page_size + 1 < max {
                    new_base += page.columns as u64;
                    self.offset += page.columns as u64;
                }
            }
            Direction::PageUp => {
                if new_base >= page_size {
                    new_base -= page_size;
                    self.offset -= page_size;
                } else {
                    new_base = 0;
                    self.offset = 0;
                    self.half = HalfByte::Left;
                }
            }
            Direction::PageDown => {
                if new_base + page_size * 2 < max {
                    new_base += page_size;
                    self.offset += page_size;
                } else {
                    if page_size > max {
                        new_base = 0;
                    } else {
                        new_base = max - page_size;
                        let align = max % page.columns as u64;
                        new_base -= align;
                        if align != 0 {
                            new_base += page.columns as u64;
                        }
                    }
                    self.offset = max - 1;
                    self.half = HalfByte::Left;
                }
            }
            Direction::FileBegin => {
                new_base = 0;
                self.offset = 0;
                self.half = HalfByte::Left;
            }
            Direction::FileEnd => {
                self.offset = max - 1;
                self.half = HalfByte::Left;
                if page_size > max {
                    new_base = 0;
                } else {
                    new_base = max - page_size;
                    let align = max % page.columns as u64;
                    new_base -= align;
                    if align != 0 {
                        new_base += page.columns as u64;
                    }
                }
            }
            Direction::Absolute(offset) => {
                self.offset = if offset < &max { *offset } else { max - 1 };
                self.half = HalfByte::Left;
                if self.offset < new_base || self.offset > new_base + page_size {
                    if self.offset > page_size / 3 {
                        new_base = self.offset - page_size / 3;
                    } else {
                        new_base = self.offset;
                    }
                    new_base -= new_base % page.columns as u64;
                }
                if new_base + page_size > max {
                    if page_size > max {
                        new_base = 0;
                    } else {
                        new_base = max - page_size;
                        let align = max % page.columns as u64;
                        new_base -= align;
                        if align != 0 {
                            new_base += page.columns as u64;
                        }
                    }
                }
            }
        };

        new_base - new_base % page.columns as u64
    }
}

/// Position inside a hex byte (left/right half byte).
#[derive(PartialEq)]
pub enum HalfByte {
    Left,
    Right,
}

/// Edit mode (hex/ascii).
#[derive(PartialEq, Clone)]
pub enum Place {
    Hex,
    Ascii,
}

/// Cursor direction to move.
pub enum Direction {
    /// Previous half of byte (hex mode only).
    PrevHalf,
    /// Next half of byte (hex mode only).
    NextHalf,
    /// Previous byte.
    PrevByte,
    /// Next byte.
    NextByte,
    /// Previous word (4 bytes).
    PrevWord,
    /// Next word (4 bytes).
    NextWord,
    /// First byte of current line.
    LineBegin,
    /// Last byte of current line.
    LineEnd,
    /// Previous line.
    LineUp,
    /// Next line.
    LineDown,
    /// Scroll one line up.
    ScrollUp,
    /// Scroll one line down.
    ScrollDown,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// File beginning.
    FileBegin,
    /// End of file.
    FileEnd,
    /// Absolute offset.
    Absolute(u64),
}
