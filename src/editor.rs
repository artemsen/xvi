// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::cui::*;
use super::cursor::*;
use super::dialog::*;
use super::file::*;
use super::goto::*;
use super::history::*;
use super::messagebox::*;
use super::page::*;
use super::saveas::*;
use super::search::*;
use super::view::*;
use super::widget::*;

/// Editor: implements business logic of a hex editor.
pub struct Editor {
    /// Console UI (curses).
    cui: Box<dyn Cui>,
    /// Edited file.
    file: File,
    /// Currently loaded and edited data.
    page: PageData,
    /// View of the currently edited (visible) data.
    view: View,
    /// Cursor position.
    cursor: Cursor,
    /// Last used "goto" address.
    last_goto: u64,
    /// Search data.
    search: Search,
    /// Exit flag.
    exit: bool,
}

impl Editor {
    /// Create new editor instance.
    pub fn new(cui: Box<dyn Cui>, path: &str) -> Result<Self, std::io::Error> {
        let file = File::open(path)?;
        let cursor = Cursor {
            offset: u64::MAX,
            half: HalfByte::Left,
            place: Place::Hex,
        };
        let history = History::new();
        Ok(Self {
            cursor,
            cui,
            file,
            page: PageData::new(u64::MAX, Vec::new()),
            view: View {
                fixed_width: false,
                ascii: true,
                statusbar: true,
                keybar: true,
            },
            last_goto: history.last_goto,
            search: Search::new(history.last_search),
            exit: false,
        })
    }

    /// Run editor.
    pub fn run(&mut self, offset: u64) {
        self.move_cursor(Location::Absolute(offset));
        while !self.exit {
            // redraw
            self.draw();

            // handle next event
            match self.cui.poll_event() {
                Event::TerminalResize => {
                    self.cui.clear();
                    self.move_cursor(Location::Absolute(self.cursor.offset));
                }
                Event::KeyPress(key) => {
                    self.handle_key(key);
                }
            }
        }
    }

    /// External event handler, called on key press.
    pub fn handle_key(&mut self, key: KeyPress) {
        let handled = match key.key {
            Key::F(1) => {
                self.help();
                true
            }
            Key::F(2) => {
                if key.modifier == KeyPress::NONE {
                    self.save();
                } else if key.modifier == KeyPress::SHIFT {
                    self.save_as();
                }
                true
            }
            Key::F(3) => {
                self.cui.clear();
                self.view.fixed_width = !self.view.fixed_width;
                self.move_cursor(Location::Absolute(self.cursor.offset));
                true
            }
            Key::F(5) => {
                self.goto();
                true
            }
            Key::F(7) => {
                if key.modifier == KeyPress::SHIFT {
                    self.find_next(self.search.backward);
                } else if key.modifier == KeyPress::ALT {
                    self.find_next(!self.search.backward);
                } else {
                    self.find();
                }
                true
            }
            Key::Esc | Key::F(10) => {
                self.exit();
                true
            }
            Key::Tab => {
                self.cursor.switch_place();
                true
            }
            Key::Left => {
                if key.modifier == KeyPress::NONE {
                    self.move_cursor(Location::PrevByte);
                } else if key.modifier == KeyPress::SHIFT {
                    self.move_cursor(Location::PrevHalf);
                } else if key.modifier == KeyPress::CTRL {
                    self.move_cursor(Location::PrevWord);
                }
                true
            }
            Key::Right => {
                if key.modifier == KeyPress::NONE {
                    self.move_cursor(Location::NextByte);
                } else if key.modifier == KeyPress::SHIFT {
                    self.move_cursor(Location::NextHalf);
                } else if key.modifier == KeyPress::CTRL {
                    self.move_cursor(Location::NextWord);
                }
                true
            }
            Key::Up => {
                if key.modifier == KeyPress::NONE {
                    self.move_cursor(Location::LineUp);
                } else if key.modifier == KeyPress::CTRL {
                    self.move_cursor(Location::ScrollUp);
                }
                true
            }
            Key::Down => {
                if key.modifier == KeyPress::NONE {
                    self.move_cursor(Location::LineDown);
                } else if key.modifier == KeyPress::CTRL {
                    self.move_cursor(Location::ScrollDown);
                }
                true
            }
            Key::Home => {
                if key.modifier == KeyPress::NONE {
                    self.move_cursor(Location::LineBegin);
                } else if key.modifier == KeyPress::CTRL {
                    self.move_cursor(Location::FileBegin);
                }
                true
            }
            Key::End => {
                if key.modifier == KeyPress::NONE {
                    self.move_cursor(Location::LineEnd);
                } else if key.modifier == KeyPress::CTRL {
                    self.move_cursor(Location::FileEnd);
                }
                true
            }
            Key::PageUp => {
                self.move_cursor(Location::PageUp);
                true
            }
            Key::PageDown => {
                self.move_cursor(Location::PageDown);
                true
            }
            Key::Char('z') => {
                if key.modifier == KeyPress::CTRL {
                    self.undo();
                    true
                } else {
                    false
                }
            }
            Key::Char('r') => {
                if key.modifier == KeyPress::CTRL {
                    self.redo();
                    true
                } else {
                    false
                }
            }
            Key::Char('y') => {
                if key.modifier == KeyPress::CTRL {
                    self.redo();
                    true
                } else {
                    false
                }
            }
            _ => false,
        };

        if handled {
            return;
        }

        if self.cursor.place == Place::Ascii {
            // ascii mode specific
            if let Key::Char(' '..='~') = key.key {
                if key.modifier == KeyPress::NONE {
                    if let Key::Char(chr) = key.key {
                        self.replace_byte(self.cursor.offset, chr as u8, 0xff);
                        self.move_cursor(Location::NextByte);
                    }
                }
            }
        } else {
            // hex mode specific
            match key.key {
                Key::Char('q') => {
                    self.exit();
                }
                Key::Char('G') => {
                    self.move_cursor(Location::FileEnd);
                }
                Key::Char('g') => {
                    self.move_cursor(Location::FileBegin);
                }
                Key::Char(':') => {
                    self.goto();
                }
                Key::Char('/') => {
                    self.find();
                }
                Key::Char('n') => {
                    self.find_next(self.search.backward);
                }
                Key::Char('N') => {
                    self.find_next(!self.search.backward);
                }
                Key::Char('a'..='f') | Key::Char('A'..='F') | Key::Char('0'..='9') => {
                    if key.modifier == KeyPress::NONE {
                        if let Key::Char(chr) = key.key {
                            let half = match chr {
                                'a'..='f' => chr as u8 - b'a' + 10,
                                'A'..='F' => chr as u8 - b'A' + 10,
                                '0'..='9' => chr as u8 - b'0',
                                _ => unreachable!(),
                            };
                            let (value, mask) = if self.cursor.half == HalfByte::Left {
                                (half << 4, 0xf0)
                            } else {
                                (half, 0x0f)
                            };
                            self.replace_byte(self.cursor.offset, value, mask);
                            self.move_cursor(Location::NextHalf);
                        }
                    }
                }
                Key::Char('u') => {
                    self.undo();
                }
                _ => {}
            }
        }
    }

    /// Show mini help.
    fn help(&self) {
        MessageBox::new("XVI: Hex editor", DialogType::Normal)
            .left("Arrows, PgUp, PgDown: move cursor;")
            .left("Tab: switch between Hex/ASCII mode;")
            .left("u or Ctrl+z: undo;")
            .left("Ctrl+r or Ctrl+y: redo;")
            .left("F2: save file;")
            .left("Shift+F2: save file with new name;")
            .left("Esc, F10 or q: exit.")
            .left("")
            .center("Read `man xvi` for more info.")
            .button(StdButton::Ok, true)
            .show(self.cui.as_ref());
    }

    /// Save current file, returns false if operation failed.
    fn save(&mut self) -> bool {
        loop {
            match self.file.save() {
                Ok(()) => {
                    self.page.update(&self.file.get_modified());
                    return true;
                }
                Err(err) => {
                    if let Some(btn) = MessageBox::new("Error", DialogType::Error)
                        .center("Error writing file")
                        .center(&self.file.name)
                        .center(&format!("{}", err))
                        .button(StdButton::Retry, true)
                        .button(StdButton::Cancel, false)
                        .show(self.cui.as_ref())
                    {
                        if btn != StdButton::Retry {
                            return false;
                        }
                        self.draw();
                    } else {
                        return false;
                    }
                }
            }
        }
    }

    /// Save current file with new name, returns false if operation failed.
    fn save_as(&mut self) -> bool {
        if let Some(new_name) = SaveAsDialog::show(self.cui.as_ref(), self.file.name.clone()) {
            loop {
                match self.file.save_as(new_name.clone()) {
                    Ok(()) => {
                        self.page.update(&self.file.get_modified());
                        return true;
                    }
                    Err(err) => {
                        if let Some(btn) = MessageBox::new("Error", DialogType::Error)
                            .center("Error writing file")
                            .center(&self.file.name)
                            .center(&format!("{}", err))
                            .button(StdButton::Retry, true)
                            .button(StdButton::Cancel, false)
                            .show(self.cui.as_ref())
                        {
                            if btn != StdButton::Retry {
                                return false;
                            }
                            self.draw();
                        } else {
                            return false;
                        }
                    }
                }
            }
        }
        false
    }

    /// Goto to specified address.
    fn goto(&mut self) {
        if let Some(offset) =
            GotoDialog::show(self.cui.as_ref(), self.last_goto, self.cursor.offset)
        {
            self.move_cursor(Location::Absolute(offset));
            self.last_goto = offset;
        }
    }

    /// Find position of the sequence.
    fn find(&mut self) {
        if self.search.dialog(self.cui.as_ref()) {
            self.draw();
            self.find_next(self.search.backward);
        }
    }

    /// Find next/previous position of the sequence.
    fn find_next(&mut self, backward: bool) {
        if self.search.data.is_empty() {
            self.search.backward = backward;
            self.find();
        } else if let Some(offset) = self
            .file
            .find(&self.search.data, self.cursor.offset, backward)
        {
            self.move_cursor(Location::Absolute(offset));
        } else {
            MessageBox::new("Search", DialogType::Error)
                .center("Sequence not found!")
                .button(StdButton::Ok, true)
                .show(self.cui.as_ref());
        }
    }

    /// Exit from editor.
    fn exit(&mut self) {
        self.exit = if !self.file.is_modified() {
            true
        } else if let Some(btn) = MessageBox::new("Exit", DialogType::Error)
            .center(&self.file.name)
            .center("was modified.")
            .center("Save before exit?")
            .button(StdButton::Yes, false)
            .button(StdButton::No, false)
            .button(StdButton::Cancel, true)
            .show(self.cui.as_ref())
        {
            match btn {
                StdButton::Yes => {
                    self.draw();
                    self.save()
                }
                StdButton::No => true,
                StdButton::Cancel => false,
                _ => unreachable!(),
            }
        } else {
            false
        };
        if self.exit && self.cursor.offset != 0 {
            let mut history = History::new();
            history.set_last_pos(&self.file.name, self.cursor.offset);
            history.last_goto = self.last_goto;
            history.last_search = self.search.data.clone();
            history.save();
        }
    }

    /// Move cursor.
    fn move_cursor(&mut self, loc: Location) {
        let (width, height) = self.cui.size();
        let (rows, columns, _) = self.view.get_scheme(width, height, self.file.size);
        let new_base = self
            .cursor
            .move_to(loc, self.page.offset, self.file.size, rows, columns);
        let new_base = new_base - new_base % columns as u64;
        let data = self.file.get(new_base, rows * columns).unwrap();
        self.page = PageData::new(new_base, data);
        self.page.update(&self.file.get_modified());
    }

    /// Undo last modification.
    fn undo(&mut self) {
        if let Some(change) = self.file.undo() {
            self.move_cursor(Location::Absolute(change.offset));
        }
    }

    /// Redo (opposite to Undo).
    fn redo(&mut self) {
        if let Some(change) = self.file.redo() {
            self.move_cursor(Location::Absolute(change.offset));
        }
    }

    /// Change data: replace byte at specified offset.
    fn replace_byte(&mut self, offset: u64, value: u8, mask: u8) {
        debug_assert!(offset >= self.page.offset);
        debug_assert!(offset < self.page.offset + self.page.data.len() as u64);

        let index = (offset - self.page.offset) as usize;
        let old = self.page.data[index];
        let new = (old & !mask) | (value & mask);

        self.file.set(offset, old, new);
        self.page.update(&self.file.get_modified());
    }

    /// Draw editor.
    fn draw(&self) {
        let (width, height) = self.cui.size();
        let canvas = Canvas {
            cui: self.cui.as_ref(),
            x: 0,
            y: 0,
            width,
            height,
        };
        let (x_cursor, y_cursor) = self
            .view
            .draw(&canvas, &self.page, &self.cursor, &self.file);
        canvas.show_cursor(x_cursor, y_cursor);
    }
}
