// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::config::*;
use super::curses::*;
use super::cursor::*;
use super::dialog::*;
use super::file::*;
use super::goto::*;
use super::history::*;
use super::messagebox::*;
use super::page::*;
use super::saveas::*;
use super::search::*;
use super::view;
use super::widget::*;

/// Editor: implements business logic of a hex editor.
pub struct Editor {
    /// Edited file.
    file: File,
    /// Currently loaded and edited data.
    page: PageData,
    /// Configuration of the view mode.
    view_cfg: view::Config,
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
    pub fn new(path: &str) -> Result<Self, std::io::Error> {
        let file = File::open(path)?;
        let cursor = Cursor {
            offset: u64::MAX,
            half: HalfByte::Left,
            place: Place::Hex,
        };

        let history = History::new();
        let mut last_goto = history.get_goto();
        if last_goto.is_empty() {
            last_goto.push(0);
        }
        let mut last_search = history.get_search();
        if last_search.is_empty() {
            last_search.push(Vec::new());
        }

        let mut instance = Self {
            cursor,
            file,
            page: PageData::new(u64::MAX, Vec::new()),
            view_cfg: view::Config::new(),
            last_goto: last_goto[0],
            search: Search {
                data: last_search[0].clone(),
                backward: false,
            },
            exit: false,
        };

        instance.move_cursor(Location::Absolute(
            if let Some(offset) = history.get_filepos(&instance.file.name) {
                offset
            } else {
                0
            },
        ));

        Ok(instance)
    }

    /// Run editor.
    pub fn run(&mut self, offset: Option<u64>) {
        if let Some(offset) = offset {
            self.move_cursor(Location::Absolute(offset));
        }
        while !self.exit {
            // redraw
            self.draw();

            // handle next event
            match Curses::wait_event() {
                Event::TerminalResize => {
                    Curses::clear_screen();
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
            Key::F(9) => {
                if self.view_cfg.setup() {
                    if !self.view_cfg.ascii {
                        self.cursor.place = Place::Hex;
                    }
                    self.move_cursor(Location::Absolute(self.cursor.offset));
                }
                true
            }
            Key::Esc | Key::F(10) => {
                self.exit();
                true
            }
            Key::Tab => {
                if self.view_cfg.ascii {
                    self.cursor.switch_place();
                }
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
            .show();
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
                        .show()
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
        if let Some(new_name) = SaveAsDialog::show(self.file.name.clone()) {
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
                            .show()
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
        if let Some(offset) = GotoDialog::show(self.last_goto, self.cursor.offset) {
            self.move_cursor(Location::Absolute(offset));
            self.last_goto = offset;
        }
    }

    /// Find position of the sequence.
    fn find(&mut self) {
        if self.search.configure() {
            self.draw();
            self.find_next(self.search.backward);
        }
    }

    /// Find next/previous position of the sequence.
    fn find_next(&mut self, backward: bool) {
        if self.search.data.is_empty() {
            self.search.backward = backward;
            self.find();
        } else if let Some(offset) = self.search.find(&mut self.file, self.cursor.offset) {
            self.move_cursor(Location::Absolute(offset));
        }
        Curses::clear_screen();
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
            .show()
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
        if self.exit {
            let config = Config::get();
            let mut history = History::new();
            history.set_goto(&[self.last_goto], config.last_goto);
            history.set_search(&[self.search.data.clone()], config.last_search);
            history.add_filepos(&self.file.name, self.cursor.offset, config.last_filepos);
            //history.last_goto = self.last_goto;
            //history.last_search = self.search.data.clone();
            history.save();
        }
    }

    /// Move cursor.
    fn move_cursor(&mut self, loc: Location) {
        let (width, height) = Curses::screen_size();

        let scheme = view::Scheme::new(
            &Window {
                x: 0,
                y: 0,
                width,
                height,
            },
            &self.view_cfg,
            self.file.size,
        );
        let new_base = self.cursor.move_to(
            loc,
            self.page.offset,
            self.file.size,
            scheme.rows,
            scheme.columns,
        );
        let data = self
            .file
            .get(new_base, scheme.rows * scheme.columns)
            .unwrap();
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
        let (width, height) = Curses::screen_size();
        let wnd = Window {
            x: 0,
            y: 0,
            width,
            height,
        };
        let scheme = view::Scheme::new(&wnd, &self.view_cfg, self.file.size);
        let view = view::View {
            scheme: &scheme,
            config: &self.view_cfg,
            page: &self.page,
            file: &self.file,
            offset: self.cursor.offset,
        };

        view.draw();
        let (x_cursor, y_cursor) = scheme.position(
            self.page.offset,
            self.cursor.offset,
            self.cursor.place == Place::Hex,
            self.cursor.half == HalfByte::Left,
        );
        wnd.show_cursor(x_cursor, y_cursor);
    }
}
