// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::config::Config;
use super::curses::*;
use super::cursor::*;
use super::document::Document;
use super::history::*;
use super::ui::dialog::*;
use super::ui::goto::GotoDlg;
use super::ui::messagebox::*;
use super::ui::progress::ProgressDlg;
use super::ui::saveas::*;
use super::ui::search::*;
use super::ui::setup::SetupDlg;
use super::ui::widget::*;
use super::view::View;

/// Editor: implements business logic of a hex editor.
pub struct Editor {
    /// Editable document.
    document: Document,
    /// View of the document.
    view: View,

    /// "Goto" configuration dialog.
    goto: GotoDlg,
    /// Search configuration dialog.
    search: SearchDlg,
    /// View mode setup dialog.
    setup: SetupDlg,
}

impl Editor {
    /// Create new editor instance.
    pub fn new(path: &str, offset: Option<u64>, config: &Config) -> Result<Self, std::io::Error> {
        let history = History::new();

        let document = Document::new(path)?;

        let mut view = View::new();
        view.fixed_width = config.fixed_width;
        view.ascii_table = config.ascii_table;

        let goto = GotoDlg {
            history: history.get_goto(),
        };
        let search = SearchDlg {
            history: history.get_search(),
            backward: false,
        };
        let setup = SetupDlg {
            fixed_width: config.fixed_width,
            ascii_table: config.ascii_table,
        };

        let mut instance = Self {
            document,
            view,
            goto,
            search,
            setup,
        };

        let initial_offset = if let Some(offset) = offset {
            offset
        } else if let Some(offset) = history.get_filepos(&instance.document.path) {
            offset
        } else {
            0
        };
        instance.document.cursor.offset = initial_offset;
        instance.resize();

        Ok(instance)
    }

    /// Run editor.
    pub fn run(&mut self) {
        loop {
            // redraw
            self.draw();

            // handle next event
            match Curses::wait_event() {
                Event::TerminalResize => {
                    self.resize();
                }
                Event::KeyPress(key) => match key.key {
                    Key::Esc | Key::F(10) => {
                        if self.exit() {
                            return;
                        }
                    }
                    _ => {
                        self.handle_key(key);
                    }
                },
            }
        }
    }

    /// Screen resize handler.
    fn resize(&mut self) {
        Curses::clear_screen();
        let mut screen = Curses::get_screen();
        screen.height -= 1; // key bar
        self.view.resize(screen, self.document.size);
        self.document
            .resize_page(self.view.lines, self.view.columns);
    }

    /// External event handler, called on key press.
    fn handle_key(&mut self, key: KeyPress) {
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
                if self.setup.show() {
                    self.view.fixed_width = self.setup.fixed_width;
                    self.view.ascii_table = self.setup.ascii_table;
                    if self.view.ascii_table.is_none() {
                        self.document.cursor.place = Place::Hex;
                    }
                    self.resize();
                }
                true
            }
            Key::Tab => {
                if self.view.ascii_table.is_some() {
                    self.document.cursor.switch_place();
                }
                true
            }
            Key::Left => {
                if key.modifier == KeyPress::NONE {
                    self.document.move_cursor(Direction::PrevByte);
                } else if key.modifier == KeyPress::SHIFT {
                    self.document.move_cursor(Direction::PrevHalf);
                } else if key.modifier == KeyPress::CTRL {
                    self.document.move_cursor(Direction::PrevWord);
                }
                true
            }
            Key::Right => {
                if key.modifier == KeyPress::NONE {
                    self.document.move_cursor(Direction::NextByte);
                } else if key.modifier == KeyPress::SHIFT {
                    self.document.move_cursor(Direction::NextHalf);
                } else if key.modifier == KeyPress::CTRL {
                    self.document.move_cursor(Direction::NextWord);
                }
                true
            }
            Key::Up => {
                if key.modifier == KeyPress::NONE {
                    self.document.move_cursor(Direction::LineUp);
                } else if key.modifier == KeyPress::CTRL {
                    self.document.move_cursor(Direction::ScrollUp);
                }
                true
            }
            Key::Down => {
                if key.modifier == KeyPress::NONE {
                    self.document.move_cursor(Direction::LineDown);
                } else if key.modifier == KeyPress::CTRL {
                    self.document.move_cursor(Direction::ScrollDown);
                }
                true
            }
            Key::Home => {
                if key.modifier == KeyPress::NONE {
                    self.document.move_cursor(Direction::LineBegin);
                } else if key.modifier == KeyPress::CTRL {
                    self.document.move_cursor(Direction::FileBegin);
                }
                true
            }
            Key::End => {
                if key.modifier == KeyPress::NONE {
                    self.document.move_cursor(Direction::LineEnd);
                } else if key.modifier == KeyPress::CTRL {
                    self.document.move_cursor(Direction::FileEnd);
                }
                true
            }
            Key::PageUp => {
                self.document.move_cursor(Direction::PageUp);
                true
            }
            Key::PageDown => {
                self.document.move_cursor(Direction::PageDown);
                true
            }
            Key::Char('z') => {
                if key.modifier == KeyPress::CTRL {
                    self.document.undo();
                    true
                } else {
                    false
                }
            }
            Key::Char('r') => {
                if key.modifier == KeyPress::CTRL {
                    self.document.redo();
                    true
                } else {
                    false
                }
            }
            Key::Char('y') => {
                if key.modifier == KeyPress::CTRL {
                    self.document.redo();
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

        if self.document.cursor.place == Place::Ascii {
            // ascii mode specific
            if let Key::Char(' '..='~') = key.key {
                if key.modifier == KeyPress::NONE {
                    if let Key::Char(chr) = key.key {
                        self.document.modify(chr as u8, 0xff);
                        self.document.move_cursor(Direction::NextByte);
                    }
                }
            }
        } else {
            // hex mode specific
            match key.key {
                Key::Char('G') => {
                    self.document.move_cursor(Direction::FileEnd);
                }
                Key::Char('g') => {
                    self.document.move_cursor(Direction::FileBegin);
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
                            let (value, mask) = if self.document.cursor.half == HalfByte::Left {
                                (half << 4, 0xf0)
                            } else {
                                (half, 0x0f)
                            };
                            self.document.modify(value, mask);
                            self.document.move_cursor(Direction::NextHalf);
                        }
                    }
                }
                Key::Char('u') => {
                    self.document.undo();
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
            match self.document.save() {
                Ok(()) => {
                    return true;
                }
                Err(err) => {
                    if let Some(btn) = MessageBox::new("Error", DialogType::Error)
                        .center("Error writing file")
                        .center(&self.document.path)
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
        if let Some(new_name) = SaveAsDlg::show(self.document.path.clone()) {
            loop {
                match self.document.save_as(new_name.clone()) {
                    Ok(()) => {
                        return true;
                    }
                    Err(err) => {
                        if let Some(btn) = MessageBox::new("Error", DialogType::Error)
                            .center("Error writing file")
                            .center(&self.document.path)
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
        if let Some(offset) = self.goto.show(self.document.cursor.offset) {
            self.document.move_cursor(Direction::Absolute(offset));
        }
    }

    /// Find position of the sequence.
    fn find(&mut self) {
        if self.search.show() {
            self.draw();
            self.find_next(self.search.backward);
        }
    }

    /// Find next/previous position of the sequence.
    fn find_next(&mut self, backward: bool) {
        if let Some(sequence) = self.search.get_sequence() {
            let mut progress = ProgressDlg::new("Searching...");
            if let Some(offset) = self.document.find(&sequence, backward, &mut progress) {
                self.document.move_cursor(Direction::Absolute(offset));
            } else {
                MessageBox::new("Search", DialogType::Error)
                    .center("Sequence not found!")
                    .button(StdButton::Ok, true)
                    .show();
            }
        } else {
            self.search.backward = backward;
            self.find();
        }
        Curses::clear_screen();
    }

    /// Exit from editor.
    fn exit(&mut self) -> bool {
        let can_exit = if !self.document.changes.has_changes() {
            true
        } else if let Some(btn) = MessageBox::new("Exit", DialogType::Error)
            .center(&self.document.path)
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

        if can_exit {
            let mut history = History::new();
            history.set_goto(&self.goto.history);
            history.set_search(&self.search.history);
            history.add_filepos(&self.document.path, self.document.cursor.offset);
            history.save();
        }

        can_exit
    }

    /// Draw editor.
    fn draw(&self) {
        // draw document
        self.view.draw(&self.document);

        // draw key bar (bottom Fn line).
        let screen = Curses::get_screen();
        let titles = &[
            "Help",  // F1
            "Save",  // F2
            "",      // F3
            "",      // F4
            "Goto",  // F5
            "",      // F6
            "Find",  // F7
            "",      // F8
            "Setup", // F9
            "Exit",  // F10
        ];
        let mut fn_line = String::new();
        let width = screen.width / 10;
        for i in 0..10 {
            fn_line += &format!(
                "{:>2}{:<width$}",
                i + 1,
                titles[i as usize],
                width = width - 2
            );
        }
        screen.print(0, screen.height - 1, &fn_line);
        screen.color(0, screen.height - 1, screen.width, Color::KeyBarTitle);
        for i in 0..10 {
            screen.color(i * width, screen.height - 1, 2, Color::KeyBarId);
        }

        // show cursor
        if let Some((mut x, y)) = self.view.get_position(
            self.document.page.offset,
            self.document.cursor.offset,
            self.document.cursor.place == Place::Hex,
        ) {
            if self.document.cursor.half == HalfByte::Right {
                x += 1;
            }
            Curses::show_cursor(self.view.window.x + x, self.view.window.y + y);
        }
    }
}
