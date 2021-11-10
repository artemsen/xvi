// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::view::View;

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
    /// * `view` - view instance
    ///
    /// # Return value
    ///
    /// New page offset.
    #[allow(clippy::too_many_lines)]
    pub fn move_to(&mut self, dir: &Direction, view: &View) -> u64 {
        let page_size = (view.lines * view.columns) as u64;
        let mut new_base = view.offset;

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
                    new_base -= view.columns as u64;
                }
            }
            Direction::NextHalf => {
                if self.offset == view.max_offset - 1
                    || (self.place == Place::Hex && self.half == HalfByte::Left)
                {
                    self.half = HalfByte::Right;
                } else {
                    self.half = HalfByte::Left;
                    self.offset += 1;
                    if self.offset >= new_base + page_size {
                        new_base += view.columns as u64;
                    }
                }
            }
            Direction::PrevByte => {
                self.half = HalfByte::Left;
                if self.offset != 0 {
                    self.offset -= 1;
                    if self.offset < new_base {
                        new_base -= view.columns as u64;
                    }
                }
            }
            Direction::NextByte => {
                self.half = HalfByte::Left;
                if self.offset < view.max_offset - 1 {
                    self.offset += 1;
                    if self.offset >= new_base + page_size {
                        new_base += view.columns as u64;
                    }
                }
            }
            Direction::PrevWord => {
                if self.offset != 0 {
                    self.offset -= Cursor::WORD_SIZE - self.offset % Cursor::WORD_SIZE;
                    if self.offset < new_base {
                        new_base -= view.columns as u64;
                    }
                }
                self.half = HalfByte::Left;
            }
            Direction::NextWord => {
                self.offset += Cursor::WORD_SIZE - self.offset % Cursor::WORD_SIZE;
                if self.offset > view.max_offset - 1 {
                    self.offset = view.max_offset - 1;
                }
                if self.offset >= new_base + page_size {
                    new_base += view.columns as u64;
                }
                self.half = HalfByte::Left;
            }
            Direction::LineBegin => {
                self.offset -= self.offset % (view.columns as u64);
                self.half = HalfByte::Left;
            }
            Direction::LineEnd => {
                self.offset += view.columns as u64 - self.offset % (view.columns as u64) - 1;
                if self.offset > view.max_offset - 1 {
                    self.offset = view.max_offset - 1;
                }
                self.half = HalfByte::Left;
            }
            Direction::LineUp => {
                if self.offset >= view.columns as u64 {
                    self.offset -= view.columns as u64;
                    if self.offset < new_base {
                        new_base -= view.columns as u64;
                    }
                }
            }
            Direction::LineDown => {
                if self.offset + (view.columns as u64) < view.max_offset {
                    self.offset += view.columns as u64;
                } else if self.offset + (view.columns as u64) - self.offset % (view.columns as u64)
                    < view.max_offset
                {
                    self.offset = view.max_offset - 1;
                    self.half = HalfByte::Left;
                }
                if self.offset >= new_base + page_size {
                    new_base += view.columns as u64;
                }
            }
            Direction::ScrollUp => {
                if new_base != 0 {
                    new_base -= view.columns as u64;
                    self.offset -= view.columns as u64;
                }
            }
            Direction::ScrollDown => {
                if new_base + page_size + 1 < view.max_offset {
                    new_base += view.columns as u64;
                    self.offset += view.columns as u64;
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
                if new_base + page_size * 2 < view.max_offset {
                    new_base += page_size;
                    self.offset += page_size;
                } else {
                    if page_size > view.max_offset {
                        new_base = 0;
                    } else {
                        new_base = view.max_offset - page_size;
                        let align = view.max_offset % view.columns as u64;
                        new_base -= align;
                        if align != 0 {
                            new_base += view.columns as u64;
                        }
                    }
                    self.offset = view.max_offset - 1;
                    self.half = HalfByte::Left;
                }
            }
            Direction::FileBegin => {
                new_base = 0;
                self.offset = 0;
                self.half = HalfByte::Left;
            }
            Direction::FileEnd => {
                self.offset = view.max_offset - 1;
                self.half = HalfByte::Left;
                if page_size > view.max_offset {
                    new_base = 0;
                } else {
                    new_base = view.max_offset - page_size;
                    let align = view.max_offset % view.columns as u64;
                    new_base -= align;
                    if align != 0 {
                        new_base += view.columns as u64;
                    }
                }
            }
            Direction::Absolute(offset, base) => {
                self.offset = if offset < &view.max_offset {
                    *offset
                } else {
                    view.max_offset - 1
                };
                self.half = HalfByte::Left;

                // try to use desirable base offset
                new_base = *base;

                if self.offset < new_base || self.offset > new_base + page_size {
                    if self.offset > page_size / 3 {
                        new_base = self.offset - page_size / 3;
                    } else {
                        new_base = self.offset;
                    }
                    new_base -= new_base % view.columns as u64;
                }
                if new_base + page_size > view.max_offset {
                    if page_size > view.max_offset {
                        new_base = 0;
                    } else {
                        new_base = view.max_offset - page_size;
                        let align = view.max_offset % view.columns as u64;
                        new_base -= align;
                        if align != 0 {
                            new_base += view.columns as u64;
                        }
                    }
                }
            }
        };

        new_base - new_base % view.columns as u64
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            offset: u64::MAX,
            half: HalfByte::Left,
            place: Place::Hex,
        }
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
    /// Absolute offset (offset, desirable base offset).
    Absolute(u64, u64),
}
