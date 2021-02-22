// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

#[derive(PartialEq)]
pub enum HalfByte {
    Left,
    Right,
}

#[derive(PartialEq)]
pub enum Place {
    Hex,
    Ascii,
}

pub enum Location {
    PrevHalf,
    NextHalf,
    PrevByte,
    NextByte,
    PrevWord,
    NextWord,
    LineBegin,
    LineEnd,
    LineUp,
    LineDown,
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    FileBegin,
    FileEnd,
    Absolute(u64),
}

pub struct Cursor {
    pub offset: u64,    // absolute offset
    pub half: HalfByte, // position inside a hex byte
    pub place: Place,   // cursor place
}

impl Cursor {
    const WORD_STEP: u64 = 4;

    pub fn switch_place(&mut self) {
        self.place = if self.place == Place::Hex {
            Place::Ascii
        } else {
            Place::Hex
        };
        self.half = HalfByte::Left;
    }

    pub fn move_to(
        &mut self,
        loc: Location,
        base: u64,
        max: u64,
        lines: usize,
        cols: usize,
    ) -> u64 {
        let page_size = (lines * cols) as u64;
        let mut new_base = base;

        match loc {
            Location::PrevHalf => {
                if self.offset == 0 || (self.place == Place::Hex && self.half == HalfByte::Right) {
                    self.half = HalfByte::Left;
                } else {
                    self.half = HalfByte::Left;
                    self.offset -= 1;
                    if self.offset < new_base {
                        new_base -= cols as u64;
                    }
                }
            }
            Location::NextHalf => {
                if self.offset == max - 1
                    || (self.place == Place::Hex && self.half == HalfByte::Left)
                {
                    self.half = HalfByte::Right;
                } else {
                    self.half = HalfByte::Left;
                    self.offset += 1;
                    if self.offset >= new_base + page_size {
                        new_base += cols as u64;
                    }
                }
            }
            Location::PrevByte => {
                self.half = HalfByte::Left;
                if self.offset != 0 {
                    self.offset -= 1;
                    if self.offset < new_base {
                        new_base -= cols as u64;
                    }
                }
            }
            Location::NextByte => {
                self.half = HalfByte::Left;
                if self.offset < max - 1 {
                    self.offset += 1;
                    if self.offset >= new_base + page_size {
                        new_base += cols as u64;
                    }
                }
            }
            Location::PrevWord => {
                if self.offset != 0 {
                    self.offset -= Cursor::WORD_STEP - self.offset % Cursor::WORD_STEP;
                    if self.offset < new_base {
                        new_base -= cols as u64;
                    }
                }
                self.half = HalfByte::Left;
            }
            Location::NextWord => {
                self.offset += Cursor::WORD_STEP - self.offset % Cursor::WORD_STEP;
                if self.offset > max - 1 {
                    self.offset = max - 1;
                }
                if self.offset >= new_base + page_size {
                    new_base += cols as u64;
                }
                self.half = HalfByte::Left;
            }
            Location::LineBegin => {
                self.offset -= self.offset % (cols as u64);
                self.half = HalfByte::Left;
            }
            Location::LineEnd => {
                self.offset += cols as u64 - self.offset % (cols as u64) - 1;
                if self.offset > max - 1 {
                    self.offset = max - 1;
                }
                self.half = HalfByte::Left;
            }
            Location::LineUp => {
                if self.offset >= cols as u64 {
                    self.offset -= cols as u64;
                    if self.offset < new_base {
                        new_base -= cols as u64;
                    }
                }
            }
            Location::LineDown => {
                if self.offset + (cols as u64) < max {
                    self.offset += cols as u64;
                } else if self.offset + (cols as u64) - self.offset % (cols as u64) < max {
                    self.offset = max - 1;
                    self.half = HalfByte::Left;
                }
                if self.offset >= new_base + page_size {
                    new_base += cols as u64;
                }
            }
            Location::ScrollUp => {
                if new_base != 0 {
                    new_base -= cols as u64;
                    self.offset -= cols as u64;
                }
            }
            Location::ScrollDown => {
                if new_base + page_size + 1 < max {
                    new_base += cols as u64;
                    self.offset += cols as u64;
                }
            }
            Location::PageUp => {
                if new_base >= page_size {
                    new_base -= page_size;
                    self.offset -= page_size;
                } else {
                    new_base = 0;
                    self.offset = 0;
                    self.half = HalfByte::Left;
                }
            }
            Location::PageDown => {
                if new_base + page_size * 2 < max {
                    new_base += page_size;
                    self.offset += page_size;
                } else {
                    if page_size > max {
                        new_base = 0;
                    } else {
                        new_base = max - page_size;
                        new_base -= max % cols as u64;
                        new_base += cols as u64;
                    }
                    self.offset = max - 1;
                    self.half = HalfByte::Left;
                }
            }
            Location::FileBegin => {
                new_base = 0;
                self.offset = 0;
                self.half = HalfByte::Left;
            }
            Location::FileEnd => {
                new_base = max - page_size;
                let align = max % cols as u64;
                new_base -= align;
                if align != 0 {
                    new_base += cols as u64;
                }
                self.offset = max - 1;
                self.half = HalfByte::Left;
            }
            Location::Absolute(offset) => {
                self.offset = if offset < max { offset } else { max - 1 };
                self.half = HalfByte::Left;
                if self.offset < new_base || self.offset > new_base + page_size {
                    if self.offset > page_size / 3 {
                        new_base = self.offset - page_size / 3;
                    } else {
                        new_base = self.offset;
                    }
                    new_base -= new_base % cols as u64;
                }
                if new_base + page_size > max {
                    if page_size > max {
                        new_base = 0;
                    } else {
                        new_base = max - page_size;
                        new_base -= max % cols as u64;
                        new_base += cols as u64;
                    }
                }
            }
        };

        new_base
    }
}
