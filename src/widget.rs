// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::cui::*;
use unicode_segmentation::UnicodeSegmentation;

/// Widget interface
pub trait Widget {
    /// Draw widget, returns cursor position
    fn draw(&self, focused: bool, canvas: &Canvas) -> Option<usize>;

    /// Check if widget is focusable
    fn focusable(&self) -> bool {
        false
    }

    fn has_input(&self) -> bool {
        false
    }

    /// Keyboard input handler
    fn keypress(&mut self, _key: KeyPress) -> Option<usize> {
        None
    }
}

/// Static text line
pub struct Text {
    pub text: String,
}
impl Text {
    pub fn new(text: &str) -> Self {
        Self {
            text: String::from(text),
        }
    }
}
impl Widget for Text {
    fn draw(&self, _focused: bool, canvas: &Canvas) -> Option<usize> {
        canvas.print(0, 0, &self.text);
        canvas.color(0, 0, canvas.width, Color::DialogNormal);
        None
    }
}

/// Static border with title.
pub struct Border {
    pub title: String,
}
impl Border {
    pub fn new(title: &str) -> Self {
        Self {
            title: format!(" {} ", title),
        }
    }
}
impl Widget for Border {
    fn draw(&self, _focused: bool, canvas: &Canvas) -> Option<usize> {
        // top
        let border = format!("╔{:═^1$}╗", self.title, canvas.width - 2);
        canvas.print(0, 0, &border);
        canvas.color(0, 0, canvas.width, Color::DialogNormal);
        // bottom
        //
        let line = (0..canvas.width - 2).map(|_| "═").collect::<String>();
        let border = String::from("╚") + &line + "╝";
        canvas.print(0, canvas.height - 1, &border);
        canvas.color(0, canvas.height - 1, canvas.width, Color::DialogNormal);
        // left/right
        for y in 1..canvas.height - 1 {
            canvas.print(0, y, "║");
            canvas.color(0, y, 1, Color::DialogNormal);
            canvas.print(canvas.width - 1, y, "║");
            canvas.color(canvas.width - 1, y, 1, Color::DialogNormal);
        }
        None
    }
}

/// Static separator (horizontal line)
pub struct Separator {
    pub title: String,
}
impl Separator {
    pub fn new(title: &str) -> Self {
        Self {
            title: if title.is_empty() {
                String::new()
            } else {
                format!(" {} ", title)
            },
        }
    }
}
impl Widget for Separator {
    fn draw(&self, _focused: bool, canvas: &Canvas) -> Option<usize> {
        let line = format!("╟{:─^1$}╢", self.title, canvas.width - 2);
        canvas.print(0, 0, &line);
        canvas.color(0, 0, canvas.width, Color::DialogNormal);
        None
    }
}

/// Button
pub struct Button {
    pub text: String,
    pub id: usize,
    pub default: bool,
    pub enabled: bool,
}
impl Button {
    // standard buttons
    pub const OK: usize = 0x01;
    pub const CANCEL: usize = 0x02;
    pub const RETRY: usize = 0x04;
    pub const YES: usize = 0x08;
    pub const NO: usize = 0x10;

    pub fn new(title: &str, id: usize, default: bool) -> Self {
        let text = format!(
            "{} {} {}",
            if default { '{' } else { '[' },
            title,
            if default { '}' } else { ']' }
        );
        Self {
            text,
            id,
            default,
            enabled: true,
        }
    }

    pub fn std(button: usize, default: bool) -> Self {
        let text = match button {
            Button::OK => "OK",
            Button::CANCEL => "Cancel",
            Button::RETRY => "Retry",
            Button::YES => "Yes",
            Button::NO => "No",
            _ => unreachable!(),
        };
        Button::new(text, button, default)
    }
}
impl Widget for Button {
    fn draw(&self, focused: bool, canvas: &Canvas) -> Option<usize> {
        canvas.print(0, 0, &self.text);
        canvas.color(
            0,
            0,
            self.text.len(),
            if focused {
                Color::ButtonFocused
            } else if !self.enabled {
                Color::ButtonDisabled
            } else {
                Color::Button
            },
        );
        None
    }

    fn focusable(&self) -> bool {
        true
    }
    /// Keyboard input handler
    fn keypress(&mut self, key: KeyPress) -> Option<usize> {
        match key.key {
            Key::Enter | Key::Char(' ') => Some(self.id),
            _ => None,
        }
    }
}

/// Single line editor
pub struct Edit {
    pub value: String,
    format: EditFormat, // value format
    cursor: usize,      // cursor position in value string
    start: usize,       // first visible character of value string
    width: usize,
}
#[allow(dead_code)]
pub enum EditFormat {
    Any,
    Hex,
}
impl Edit {
    pub fn new(width: usize, value: &str, format: EditFormat) -> Self {
        Self {
            value: String::from(value),
            format,
            cursor: 0,
            start: 0,
            width,
        }
    }

    /// Move cursor inside the edit string
    fn move_cursor(&mut self, step: isize) {
        debug_assert!(step != 0);

        // change cursor position
        let length = self.length() as isize;
        self.cursor = if step > length || self.cursor as isize + step > length {
            length as usize
        } else if step < 0 && self.cursor as isize + step < 0 {
            0
        } else {
            (self.cursor as isize + step) as usize
        };

        // change start position (first visible character)
        if self.cursor < self.start {
            self.start = self.cursor;
        } else if self.cursor >= self.start + self.width {
            self.start = self.cursor - self.width + 1;
        }
    }

    /// Insert character to current cursor position
    fn insert_char(&mut self, ch: char) {
        let allow = match self.format {
            EditFormat::Any => true,
            EditFormat::Hex => ch.is_ascii_hexdigit(),
        };
        if allow {
            let byte_pos = self.char2byte(self.cursor);
            self.value.insert(byte_pos, ch);
            self.move_cursor(1);
        }
    }

    /// Delete character from current cursor position
    fn delete_char(&mut self, count: isize) {
        debug_assert!(count != 0);

        let length = self.length();
        if count > 0 && self.cursor < length {
            // delete to the right
            let remove = std::cmp::min(length - self.cursor, count as usize);
            let byte_start = self.char2byte(self.cursor);
            let byte_end = self.char2byte(self.cursor + remove);
            self.value.drain(byte_start..byte_end);
        } else if count < 0 && self.cursor > 0 {
            // delete to the left (backspace)
            let remove = std::cmp::min(self.cursor, count.abs() as usize);
            let byte_start = self.char2byte(self.cursor - remove);
            let byte_end = self.char2byte(self.cursor);
            self.value.drain(byte_start..byte_end);
            self.move_cursor(count);
        }
    }

    /// Get length of the string in visual characters (graphemes)
    fn length(&self) -> usize {
        UnicodeSegmentation::graphemes(&self.value as &str, true).count()
    }

    /// Convert char (grapheme) position to byte offset inside the value
    fn char2byte(&self, char_pos: usize) -> usize {
        let mut byte_pos = 0;
        if char_pos > 0 {
            let (i, gr) = UnicodeSegmentation::grapheme_indices(&self.value as &str, true)
                .nth(char_pos - 1)
                .expect("Invalid position");
            byte_pos = i + gr.len();
        }
        byte_pos
    }
}

impl Widget for Edit {
    fn draw(&self, focused: bool, canvas: &Canvas) -> Option<usize> {
        // get substring to display
        let visible_end = self.start + std::cmp::min(self.length() - self.start, self.width);
        let start = self.char2byte(self.start);
        let end = self.char2byte(visible_end);
        let substr = &self.value[start..end];

        canvas.print(0, 0, substr);
        canvas.color(
            0,
            0,
            self.width,
            if focused {
                Color::EditFocused
            } else {
                Color::Edit
            },
        );
        if focused {
            Some(self.cursor - self.start)
        } else {
            None
        }
    }

    fn focusable(&self) -> bool {
        true
    }

    fn has_input(&self) -> bool {
        true
    }

    /// Keyboard input handler
    fn keypress(&mut self, key: KeyPress) -> Option<usize> {
        match key.key {
            Key::Enter => {
                return Some(Button::OK);
            }
            Key::Home => {
                self.move_cursor(isize::MIN);
            }
            Key::End => {
                self.move_cursor(isize::MAX);
            }
            Key::Left => {
                self.move_cursor(-1);
            }
            Key::Right => {
                self.move_cursor(1);
            }
            Key::Delete => {
                self.delete_char(1);
            }
            Key::Backspace => {
                self.delete_char(-1);
            }
            Key::Char(ch) => {
                self.insert_char(ch);
            }
            _ => {}
        };
        None
    }
}
