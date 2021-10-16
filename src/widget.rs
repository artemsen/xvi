// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::curses::{Color, Key, KeyPress, Window};
use unicode_segmentation::UnicodeSegmentation;

/// Widget interface.
pub trait Widget {
    /// Draw widget, returns cursor position in the line.
    fn draw(&self, focused: bool, enabled: bool, wnd: &Window) -> Option<usize>;

    /// Check if widget is focusable.
    fn focusable(&self) -> bool {
        false
    }
    /// Set focus handler.
    fn focus(&mut self) {}

    /// Keyboard input handler, returns true if key was handled.
    fn keypress(&mut self, _key: &KeyPress) -> bool {
        false
    }

    /// Get data from widget.
    fn set_data(&mut self, _data: WidgetData) {}
    /// Set data for widget.
    fn get_data(&self) -> WidgetData {
        WidgetData::Bool(false)
    }
}

/// Widget data.
#[derive(Clone, PartialEq)]
pub enum WidgetData {
    Text(String),
    Number(usize),
    Bool(bool),
}

/// Static text line.
pub struct Text {
    text: String,
}

impl Text {
    /// Create new widget instance.
    pub fn new(text: &str) -> Box<Self> {
        Box::new(Self {
            text: String::from(text),
        })
    }
}

impl Widget for Text {
    fn draw(&self, _focused: bool, _enabled: bool, wnd: &Window) -> Option<usize> {
        wnd.print(0, 0, &self.text);
        None
    }

    fn set_data(&mut self, data: WidgetData) {
        if let WidgetData::Text(text) = data {
            self.text = text;
        }
    }

    fn get_data(&self) -> WidgetData {
        WidgetData::Text(self.text.clone())
    }
}

/// Static border with title.
pub struct Border {
    title: String,
}

impl Border {
    /// Create new widget instance.
    pub fn new(title: &str) -> Box<Self> {
        Box::new(Self {
            title: format!(" {} ", title),
        })
    }
}

impl Widget for Border {
    fn draw(&self, _focused: bool, _enabled: bool, wnd: &Window) -> Option<usize> {
        // top
        let border = format!("╔{:═^1$}╗", &self.title, wnd.width - 2);
        wnd.print(0, 0, &border);
        // bottom
        let line = (0..wnd.width - 2).map(|_| "═").collect::<String>();
        let border = String::from("╚") + &line + "╝";
        wnd.print(0, wnd.height - 1, &border);
        // left/right
        for y in 1..wnd.height - 1 {
            wnd.print(0, y, "║");
            wnd.print(wnd.width - 1, y, "║");
        }
        None
    }
}

/// Static separator (horizontal line).
pub struct Separator {
    title: String,
}

impl Separator {
    /// Create new widget instance.
    pub fn new(title: Option<&str>) -> Box<Self> {
        let title = if let Some(title) = title {
            format!(" {} ", title)
        } else {
            String::new()
        };
        Box::new(Self { title })
    }
}

impl Widget for Separator {
    fn draw(&self, _focused: bool, _enabled: bool, wnd: &Window) -> Option<usize> {
        let line = format!("╟{:─^1$}╢", &self.title, wnd.width - 2);
        wnd.print(0, 0, &line);
        None
    }
}

/// Standard buttons.
#[derive(Copy, Clone, PartialEq)]
pub enum StdButton {
    Ok,
    Cancel,
    Retry,
    Yes,
    No,
}

/// Button.
#[derive(Clone)]
pub struct Button {
    pub text: String,
    pub default: bool,
}

impl Button {
    /// Create custom button instance.
    pub fn new(title: &str, default: bool) -> Box<Self> {
        let text = format!(
            "{} {} {}",
            if default { '{' } else { '[' },
            title,
            if default { '}' } else { ']' }
        );
        Box::new(Self { text, default })
    }

    /// Create standard button instance.
    pub fn std(button: StdButton, default: bool) -> Box<Self> {
        let text = match button {
            StdButton::Ok => "OK",
            StdButton::Cancel => "Cancel",
            StdButton::Retry => "Retry",
            StdButton::Yes => "Yes",
            StdButton::No => "No",
        };
        Button::new(text, default)
    }
}

impl Widget for Button {
    fn draw(&self, focused: bool, enabled: bool, wnd: &Window) -> Option<usize> {
        wnd.print(0, 0, &self.text);
        if focused {
            wnd.color(0, 0, self.text.len(), Color::ItemFocused);
        } else if !enabled {
            wnd.color(0, 0, self.text.len(), Color::ItemDisabled);
        }
        None
    }

    fn focusable(&self) -> bool {
        true
    }
}

/// Checkbox.
pub struct Checkbox {
    pub title: String,
    pub state: bool,
}

impl Checkbox {
    /// Create new widget instance.
    pub fn new(title: &str, state: bool) -> Box<Self> {
        Box::new(Self {
            title: String::from(title),
            state,
        })
    }
}

impl Widget for Checkbox {
    fn draw(&self, focused: bool, enabled: bool, wnd: &Window) -> Option<usize> {
        let text = format!("[{}] {}", if self.state { 'X' } else { ' ' }, self.title);
        wnd.print(0, 0, &text);
        if focused {
            wnd.color(0, 0, 3, Color::ItemFocused);
        } else if !enabled {
            wnd.color(0, 0, 3, Color::ItemDisabled);
        }
        if focused {
            Some(1)
        } else {
            None
        }
    }

    fn focusable(&self) -> bool {
        true
    }

    fn keypress(&mut self, key: &KeyPress) -> bool {
        if key.key == Key::Char(' ') {
            self.state = !self.state;
            true
        } else {
            false
        }
    }

    fn set_data(&mut self, data: WidgetData) {
        if let WidgetData::Bool(state) = data {
            self.state = state;
        }
    }

    fn get_data(&self) -> WidgetData {
        WidgetData::Bool(self.state)
    }
}

/// Progress bar.
pub struct ProgressBar {
    pub percent: usize,
}

impl ProgressBar {
    /// Create new widget instance.
    pub fn new() -> Box<Self> {
        Box::new(Self {
            percent: usize::MAX,
        })
    }
}

impl Widget for ProgressBar {
    fn draw(&self, _focused: bool, _enabled: bool, wnd: &Window) -> Option<usize> {
        let text = format!("{:>3}%", self.percent);
        let bar_len = wnd.width - text.len() - 1;
        let fill = self.percent as usize * bar_len / 100;
        let mut bar = (0..fill).map(|_| "▓").collect::<String>();
        bar += &(fill..bar_len).map(|_| "░").collect::<String>();
        wnd.print(0, 0, &bar);
        wnd.print(bar_len + 1, 0, &text);
        None
    }

    fn set_data(&mut self, data: WidgetData) {
        if let WidgetData::Number(n) = data {
            debug_assert!(n <= 100);
            self.percent = n;
        }
    }

    fn get_data(&self) -> WidgetData {
        WidgetData::Number(self.percent)
    }
}

/// Edit formats.
#[derive(Clone, PartialEq)]
pub enum EditFormat {
    Any,
    HexStream,
    HexSigned,
    HexUnsigned,
    DecSigned,
    DecUnsigned,
}

/// Single line editor.
pub struct Edit {
    pub value: String,
    pub history: Vec<String>,
    history_index: usize,
    format: EditFormat, // value format
    selection: bool,
    cursor: usize, // cursor position in value string
    start: usize,  // first visible character of value string
    pub width: usize,
}

impl Edit {
    /// Create new widget instance.
    pub fn new(width: usize, value: String, format: EditFormat) -> Box<Self> {
        Box::new(Self {
            value,
            history: Vec::new(),
            history_index: 0,
            format,
            selection: false,
            cursor: 0,
            start: 0,
            width,
        })
    }

    /// Move cursor inside the edit string.
    fn move_cursor(&mut self, step: isize) {
        debug_assert!(step != 0);

        self.selection = false;

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

    /// Insert character to current cursor position.
    fn insert_char(&mut self, ch: char) {
        let max_hex = 16;
        let max_dec = 20;
        let max_stream = 256 * 2;
        let allow = match self.format {
            EditFormat::Any => true,
            EditFormat::HexStream => self.value.len() < max_stream && ch.is_ascii_hexdigit(),
            EditFormat::HexSigned => {
                (self.value.len() < max_hex && ch.is_ascii_hexdigit())
                    || ((self.cursor == 0 || self.selection) && (ch == '-' || ch == '+'))
            }
            EditFormat::HexUnsigned => self.value.len() <= max_hex && ch.is_ascii_hexdigit(),
            EditFormat::DecSigned => {
                (self.value.len() < max_dec && ch.is_ascii_digit())
                    || ((self.cursor == 0 || self.selection) && (ch == '-' || ch == '+'))
            }
            EditFormat::DecUnsigned => self.value.len() <= max_dec && ch.is_ascii_digit(),
        };
        if allow {
            if self.selection {
                // delete the entire selection
                self.selection = false;
                self.value.clear();
                self.move_cursor(isize::min_value());
            }

            let byte_pos = self.char2byte(self.cursor);
            self.value.insert(byte_pos, ch);
            self.move_cursor(1);
        }
    }

    /// Delete character from current cursor position.
    fn delete_char(&mut self, count: isize) {
        debug_assert!(count != 0);

        if self.selection {
            // delete the entire selection
            self.selection = false;
            self.value.clear();
            self.move_cursor(isize::min_value());
        }

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

    /// Move through history.
    fn from_history(&mut self, forward: bool) {
        if forward && self.history_index + 1 < self.history.len() {
            self.history_index += 1;
        } else if !forward && self.history_index > 0 {
            self.history_index -= 1;
        } else {
            return;
        }
        self.value = self.history[self.history_index].clone();
        self.move_cursor(isize::max_value());
        self.selection = true;
    }

    /// Get length of the string in visual characters (grapheme).
    fn length(&self) -> usize {
        UnicodeSegmentation::graphemes(&self.value as &str, true).count()
    }

    /// Convert char (grapheme) position to byte offset inside the value.
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
    fn draw(&self, focused: bool, _enabled: bool, wnd: &Window) -> Option<usize> {
        // get substring to display
        let visible_end = self.start + self.width.min(self.length() - self.start);
        let start = self.char2byte(self.start);
        let end = self.char2byte(visible_end);
        let substr = &self.value[start..end];

        wnd.print(0, 0, substr);
        if !self.history.is_empty() {
            wnd.print(self.width - 1, 0, "▼");
        }
        wnd.color(
            0,
            0,
            self.width,
            if focused {
                Color::EditFocused
            } else {
                Color::EditNormal
            },
        );
        if focused {
            if self.selection {
                wnd.color(0, 0, self.cursor, Color::EditSelection);
            }
            Some(self.cursor - self.start)
        } else {
            None
        }
    }

    fn focusable(&self) -> bool {
        true
    }

    fn focus(&mut self) {
        self.move_cursor(isize::max_value());
        self.selection = true;
    }

    fn keypress(&mut self, key: &KeyPress) -> bool {
        match key.key {
            Key::Up => {
                if key.modifier & KeyPress::CTRL != 0 {
                    self.from_history(false);
                    return true;
                }
                return false;
            }
            Key::Down => {
                if key.modifier & KeyPress::CTRL != 0 {
                    self.from_history(true);
                    return true;
                }
                return false;
            }
            Key::Home => {
                self.move_cursor(isize::min_value());
            }
            Key::End => {
                self.move_cursor(isize::max_value());
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
            _ => {
                return false;
            }
        };
        true
    }

    fn set_data(&mut self, data: WidgetData) {
        if let WidgetData::Text(text) = data {
            self.value = text;
            self.move_cursor(isize::min_value());
        }
    }

    fn get_data(&self) -> WidgetData {
        WidgetData::Text(self.value.clone())
    }
}
