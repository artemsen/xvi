// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::buffer::*;
use super::cui::*;
use super::cursor::*;
use super::dialog::*;
use super::file::*;
use super::page::*;
use super::widget::*;

pub struct Editor {
    cui: Box<dyn Cui>,
    file: File,
    page: Page,
    buffer: Buffer,
    cursor: Cursor,
    last_goto: u64,
    last_find: Vec<u8>,
}

impl Editor {
    pub fn new(cui: Box<dyn Cui>, path: &str) -> Result<Self, std::io::Error> {
        let file = File::open(path)?;
        let cursor = Cursor {
            offset: u64::MAX,
            half: HalfByte::Left,
            place: Place::Hex,
        };
        Ok(Self {
            cursor,
            cui,
            file,
            page: Page::new(u64::MAX, Vec::new()),
            buffer: Buffer::new(),
            last_goto: 0,
            last_find: Vec::new(),
        })
    }

    pub fn run(&mut self, offset: u64) {
        self.move_cursor(Location::Absolute(offset));
        loop {
            // redraw
            self.draw();

            // handle next event
            match self.cui.poll_event() {
                Event::TerminalResize => {
                    self.cui.clear();
                    self.move_cursor(Location::Absolute(self.cursor.offset));
                }
                Event::KeyPress(key) => {
                    match key.key {
                        Key::Esc | Key::F(10) => {
                            if self.exit() {
                                return;
                            }
                        }
                        _ => {
                            self.handle_key(key);
                        }
                    };
                }
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyPress) {
        let handled = match key.key {
            Key::F(1) => {
                MessageBox::create("Help")
                    .add_multiline("See `man xvi` for more info")
                    .add_button(Button::OK, true)
                    .show(self.cui.as_ref());
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
                    self.find_next();
                } else {
                    self.find();
                }
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

        if self.cursor.place == Place::Hex {
            // hex mode specific
            match key.key {
                Key::Char(':') => {
                    self.goto();
                }
                Key::Char('/') => {
                    self.find();
                }
                Key::Char('a'..='f') | Key::Char('A'..='F') | Key::Char('0'..='9') => {
                    if key.modifier == KeyPress::NONE {
                        if let Key::Char(chr) = key.key {
                            self.modify(chr);
                        }
                    }
                }
                Key::Char('u') => {
                    self.undo();
                }
                _ => {}
            }
        } else if let Key::Char(' '..='~') = key.key {
            // ascii mode specific
            if key.modifier == KeyPress::NONE {
                if let Key::Char(chr) = key.key {
                    self.replace_byte(self.cursor.offset, chr as u8, 0xff);
                    self.move_cursor(Location::NextByte);
                }
            }
        }
    }

    fn move_cursor(&mut self, loc: Location) {
        let (_, height) = self.cui.size();
        let (lines, cols) = PageView::size(height - 2 /*skip bars*/);

        let new_base = self
            .cursor
            .move_to(loc, self.page.offset, self.file.size, lines, cols);
        let data = self.file.read(new_base, lines * cols).unwrap();
        self.page = Page::new(new_base, data);
        self.page.update(&self.buffer.get());
    }

    fn replace_byte(&mut self, offset: u64, value: u8, mask: u8) {
        debug_assert!(offset >= self.page.offset);
        debug_assert!(offset < self.page.offset + self.page.data.len() as u64);

        let index = (offset - self.page.offset) as usize;
        let old = self.page.data[index];
        let new = (old & !mask) | (value & mask);

        self.buffer.add(offset, old, new);
        self.page.update(&self.buffer.get());
    }

    fn modify(&mut self, chr: char) {
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

    fn goto(&mut self) {
        if let Some(offset) = GotoDialog::show(self.cui.as_ref(), self.last_goto) {
            self.move_cursor(Location::Absolute(offset));
            self.last_goto = self.cursor.offset;
        }
    }

    fn find_next(&mut self) {
        let start = self.cursor.offset + 1;
        if let Some(offset) = self.page.find(&self.last_find, start) {
            self.move_cursor(Location::Absolute(offset));
        } else {
            let step = 1000;
            let size = step + self.last_find.len() - 1;
            let mut offset = start;
            loop {
                if offset >= self.file.size {
                    break;
                }
                let data = self.file.read(offset, size).unwrap();
                if let Some(pos) = data
                    .windows(self.last_find.len())
                    .position(|window| window == self.last_find)
                {
                    self.move_cursor(Location::Absolute(offset + pos as u64));
                    break;
                }
                offset += step as u64;
            }
        }
    }

    fn find(&mut self) {
        if let Some(find) = FindDialog::show(self.cui.as_ref(), &self.last_find) {
            self.last_find = find;
            self.find_next();
        }
    }

    fn undo(&mut self) {
        if let Some(change) = self.buffer.undo() {
            if self.page.visible(change.offset) {
                self.page.set(change.offset, change.old, Page::DEFAULT);
                self.page.update(&self.buffer.get());
            }
            self.move_cursor(Location::Absolute(change.offset));
        }
    }

    fn redo(&mut self) {
        if let Some(change) = self.buffer.redo() {
            if self.page.visible(change.offset) {
                self.page.update(&self.buffer.get());
            }
            self.move_cursor(Location::Absolute(change.offset));
        }
    }

    /// Save current file, returns false if operation failed
    fn save(&mut self) -> bool {
        loop {
            match self.file.save(&self.buffer.get()) {
                Ok(()) => {
                    self.buffer.reset();
                    self.page.update(&self.buffer.get());
                    return true;
                }
                Err(err) => {
                    if let Some(btn) = MessageBox::create("Error")
                        .add_line("Error writing file")
                        .add_line(&self.file.name)
                        .add_line(&format!("{}", err))
                        .add_button(Button::RETRY, true)
                        .add_button(Button::CANCEL, false)
                        .show(self.cui.as_ref())
                    {
                        if btn != Button::RETRY {
                            return false;
                        }
                        self.draw();
                    }
                }
            }
        }
    }

    /// Save current file with new name, returns false if operation failed
    fn save_as(&mut self) -> bool {
        if let Some(new_name) = SaveAsDialog::show(self.cui.as_ref(), &self.file.name) {
            loop {
                match self.file.save_as(new_name.clone(), &self.buffer.get()) {
                    Ok(()) => {
                        self.buffer.reset();
                        self.page.update(&self.buffer.get());
                        return true;
                    }
                    Err(err) => {
                        if let Some(btn) = MessageBox::create("Error")
                            .add_line("Error writing file")
                            .add_line(&self.file.name)
                            .add_line(&format!("{}", err))
                            .add_button(Button::RETRY, true)
                            .add_button(Button::CANCEL, false)
                            .show(self.cui.as_ref())
                        {
                            if btn != Button::RETRY {
                                return false;
                            }
                            self.draw();
                        }
                    }
                }
            }
        }
        false
    }

    /// Exit from editor, returns true if editor can be gracefully closed
    fn exit(&mut self) -> bool {
        if self.buffer.get().is_empty() {
            return true;
        }
        return if let Some(btn) = MessageBox::create("Exit")
            .add_line(&self.file.name)
            .add_line("was modified.")
            .add_line("Save before exit?")
            .add_button(Button::YES, false)
            .add_button(Button::NO, false)
            .add_button(Button::CANCEL, true)
            .show(self.cui.as_ref())
        {
            match btn {
                Button::YES => {
                    self.draw();
                    self.save()
                }
                Button::NO => true,
                Button::CANCEL => false,
                _ => unreachable!(),
            }
        } else {
            false
        };
    }

    fn draw(&self) {
        let (width, height) = self.cui.size();

        let status_bar = Canvas {
            cui: self.cui.as_ref(),
            x: 0,
            y: 0,
            width,
            height: 1,
        };
        self.draw_status_bar(&status_bar);

        let key_bar = Canvas {
            cui: self.cui.as_ref(),
            x: 0,
            y: height - 1,
            width,
            height: 1,
        };
        self.draw_key_bar(&key_bar);

        let hex = Canvas {
            cui: self.cui.as_ref(),
            x: 0,
            y: 1,
            width,
            height: height - 2, // without status bar and key bar
        };
        let (x_cursor, y_cursor) = PageView::print(&hex, &self.page, &self.cursor);
        self.cui.show_cursor(x_cursor, y_cursor);
    }

    /// Draw status bar
    fn draw_status_bar(&self, canvas: &Canvas) {
        canvas.print(0, 0, &self.file.name);
        if !self.buffer.get().is_empty() {
            canvas.print(0, 0, "*");
        }
        let (value, _) = self.page.get(self.cursor.offset).unwrap();
        let percent = (self.cursor.offset * 100 / (self.file.size - 1)) as u8;

        let stat = format!(
            "[0x{:02x} {value:<3} 0{value:<3o} {value:08b}]     0x{offset:04x}   {percent:>3}%",
            value = value,
            offset = self.cursor.offset,
            percent = percent,
        );
        canvas.print(canvas.width - stat.len(), 0, &stat);
        canvas.color(0, 0, canvas.width, Color::StatusBar);
    }

    /// Draw key bar (bottom Fn line)
    fn draw_key_bar(&self, canvas: &Canvas) {
        let titles = &[
            "Help", // F1
            "Save", // F2
            "",     // F3
            "",     // F4
            "Goto", // F5
            "",     // F6
            "Find", // F7
            "",     // F8
            "",     // F9
            "Exit", // F10
        ];

        let fn_id_len: usize = 2; // function number length (f1-f0)
        let title_len: usize = 6; // minimal title length

        let mut width = canvas.width / 10;
        if width < fn_id_len + title_len {
            width = fn_id_len + title_len;
        }
        for i in 0..10 {
            let x_num = i * width;
            canvas.print(x_num, 0, &format!("{:>2}", i + 1));
            canvas.color(x_num, 0, fn_id_len, Color::KeyBarId);
            let x_label = x_num + fn_id_len;
            canvas.print(x_label, 0, titles[i as usize]);
            canvas.color(x_label, 0, width - fn_id_len, Color::KeyBarTitle);
        }
    }
}
