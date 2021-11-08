// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::super::curses::{Color, Key, KeyPress, Window};
use unicode_segmentation::UnicodeSegmentation;

/// Typed widget.
#[derive(PartialEq)]
pub enum WidgetType {
    Separator,
    StaticText(String),
    CheckBox(CheckBox),
    ListBox(ListBox),
    ProgressBar(u8),
    Button(Button),
    Edit(InputLine),
}

impl WidgetType {
    /// Draw widget.
    ///
    /// # Arguments
    ///
    /// * `wnd` - window (canvas)
    /// * `ctx` - widget context
    ///
    /// # Return value
    ///
    /// Cursor coordinates on the window.
    pub fn draw(&self, wnd: &Window, ctx: &WidgetContext) -> Option<(usize, usize)> {
        match self {
            WidgetType::Separator => {
                let text = format!("\u{255f}{:\u{2500}^1$}\u{2562}", "", ctx.width - 2);
                wnd.print(ctx.x, ctx.y, &text);
            }
            WidgetType::StaticText(text) => {
                wnd.print(ctx.x, ctx.y, text);
            }
            WidgetType::CheckBox(widget) => {
                widget.draw(wnd, ctx);
            }
            WidgetType::ListBox(widget) => {
                widget.draw(wnd, ctx);
            }
            WidgetType::ProgressBar(percent) => {
                debug_assert!(*percent <= 100);
                let text = format!(" {:>3}%", percent);
                let barsz = ctx.width - text.len();
                let fill = *percent as usize * barsz / 100;
                let bar = "\u{2588}".repeat(fill) + &"\u{2591}".repeat(barsz - fill) + &text;
                wnd.print(ctx.x, ctx.y, &bar);
            }
            WidgetType::Button(widget) => {
                widget.draw(wnd, ctx);
            }
            WidgetType::Edit(widget) => {
                return widget.draw(wnd, ctx);
            }
        };
        None
    }

    /// Keyboard input handler.
    ///
    /// # Arguments
    ///
    /// * `key` - pressed key
    ///
    /// # Return value
    ///
    /// `true` if key was handled.
    pub fn key_press(&mut self, key: &KeyPress) -> bool {
        match self {
            WidgetType::Edit(widget) => widget.key_press(key),
            WidgetType::CheckBox(widget) => widget.key_press(key),
            WidgetType::ListBox(widget) => widget.key_press(key),
            _ => false,
        }
    }

    /// Check if widget is focusable.
    ///
    /// # Return value
    ///
    /// `true` if widget can take focus
    pub fn focusable(&self) -> bool {
        matches!(
            self,
            WidgetType::CheckBox(_)
                | WidgetType::Button(_)
                | WidgetType::ListBox(_)
                | WidgetType::Edit(_)
        )
    }

    /// Set focus handler.
    pub fn focus_set(&mut self) {
        if let WidgetType::Edit(edit) = self {
            edit.on_focus_set();
        }
    }
}

/// Widget context: linkage with a dialog.
pub struct WidgetContext {
    /// Focus flag.
    pub focused: bool,
    /// State.
    pub enabled: bool,
    /// Start column on the dialog window.
    pub x: usize,
    /// Line number on the dialog window.
    pub y: usize,
    /// Size of the widget.
    pub width: usize,
}

impl Default for WidgetContext {
    fn default() -> Self {
        Self {
            focused: false,
            enabled: true,
            x: 0,
            y: 0,
            width: 0,
        }
    }
}

/// Check box control.
#[derive(PartialEq)]
pub struct CheckBox {
    pub state: bool,
    pub title: String,
}
impl CheckBox {
    /// Draw widget.
    ///
    /// # Arguments
    ///
    /// * `wnd` - window (canvas)
    /// * `ctx` - widget context
    pub fn draw(&self, wnd: &Window, ctx: &WidgetContext) {
        let text = &format!("[{}] {}", if self.state { 'x' } else { ' ' }, self.title);
        wnd.print(ctx.x, ctx.y, text);
        if ctx.focused {
            wnd.color(ctx.x, ctx.y, 3, Color::ItemFocused);
        } else if !ctx.enabled {
            wnd.color(ctx.x, ctx.y, 3, Color::ItemDisabled);
        }
    }

    /// Keyboard input handler.
    ///
    /// # Arguments
    ///
    /// * `key` - pressed key
    ///
    /// # Return value
    ///
    /// `true` if key was handled.
    pub fn key_press(&mut self, key: &KeyPress) -> bool {
        if key.key == Key::Char(' ') {
            self.state = !self.state;
            true
        } else {
            false
        }
    }
}

/// List box control.
#[derive(PartialEq)]
pub struct ListBox {
    pub list: Vec<String>,
    pub current: usize,
}
impl ListBox {
    /// Draw widget.
    ///
    /// # Arguments
    ///
    /// * `wnd` - window (canvas)
    /// * `ctx` - widget context
    pub fn draw(&self, wnd: &Window, ctx: &WidgetContext) {
        debug_assert!(self.current < self.list.len());

        let text = format!(
            "{: ^width$}",
            self.list[self.current],
            width = ctx.width - 2
        );

        wnd.print(ctx.x, ctx.y, "\u{25c4}");
        wnd.print(ctx.x + ctx.width - 1, ctx.y, "\u{25ba}");
        wnd.print(ctx.x + 1, ctx.y, &text);

        if ctx.focused {
            wnd.color(ctx.x, ctx.y, ctx.width, Color::ItemFocused);
        } else if !ctx.enabled {
            wnd.color(ctx.x, ctx.y, ctx.width, Color::ItemDisabled);
        }
    }

    /// Keyboard input handler.
    ///
    /// # Arguments
    ///
    /// * `key` - pressed key
    ///
    /// # Return value
    ///
    /// `true` if key was handled.
    pub fn key_press(&mut self, key: &KeyPress) -> bool {
        match key.key {
            Key::Left => {
                if self.current > 0 {
                    self.current -= 1;
                }
            }
            Key::Right => {
                if self.current < self.list.len() - 1 {
                    self.current += 1;
                }
            }
            _ => {
                return false;
            }
        };
        true
    }
}

/// Button.
#[derive(PartialEq)]
pub struct Button {
    /// Text representation.
    pub text: String,
    /// Default button in group.
    pub default: bool,
}
impl Button {
    /// Draw widget.
    ///
    /// # Arguments
    ///
    /// * `wnd` - window (canvas)
    /// * `ctx` - widget context
    pub fn draw(&self, wnd: &Window, ctx: &WidgetContext) {
        wnd.print(ctx.x, ctx.y, &self.text);
        if ctx.focused {
            wnd.color(ctx.x, ctx.y, ctx.width, Color::ItemFocused);
        } else if !ctx.enabled {
            wnd.color(ctx.x, ctx.y, ctx.width, Color::ItemDisabled);
        }
    }
}

/// Standard buttons.
#[derive(Clone, Copy, PartialEq)]
pub enum StandardButton {
    OK,
    Cancel,
    Retry,
    Yes,
    No,
}
impl StandardButton {
    /// Get text representation of the button.
    ///
    /// # Arguments
    ///
    /// * `default` - default button in group
    ///
    /// # Return value
    ///
    /// Button text.
    pub fn text(&self, default: bool) -> String {
        let title = match self {
            StandardButton::OK => "OK",
            StandardButton::Cancel => "Cancel",
            StandardButton::Retry => "Retry",
            StandardButton::Yes => "Yes",
            StandardButton::No => "No",
        };
        format!(
            "{} {} {}",
            if default { '{' } else { '[' },
            title,
            if default { '}' } else { ']' }
        )
    }
}

/// Single line input widget.
#[derive(PartialEq)]
pub struct InputLine {
    /// Editing value.
    value: String,
    /// Value format.
    format: InputFormat,
    /// Size of the input field.
    width: usize,
    /// Selection flag.
    selection: bool,
    /// Cursor position in value string.
    cursor: usize,
    /// First visible character of value string.
    start: usize,
    /// Input history.
    history: Vec<String>,
    /// Current index in history.
    current: usize,
}
impl InputLine {
    /// Create new widget instance.
    ///
    /// # Arguments
    ///
    /// * `value` - default value
    /// * `format` - value format
    /// * `history` - input history
    /// * `width` - size of the input field
    ///
    /// # Return value
    ///
    /// Windget instance.
    pub fn new(value: String, format: InputFormat, history: Vec<String>, width: usize) -> Self {
        Self {
            value,
            format,
            width,
            selection: false,
            cursor: 0,
            start: 0,
            history,
            current: 0,
        }
    }

    /// Set new value.
    ///
    /// # Arguments
    ///
    /// * `value` - new value
    pub fn set_value(&mut self, value: String) {
        self.value = value;
        self.move_cursor(isize::MIN);
    }

    /// Get the current value.
    ///
    /// # Return value
    ///
    /// Current value.
    pub fn get_value(&self) -> &str {
        &self.value
    }

    /// Draw widget.
    ///
    /// # Arguments
    ///
    /// * `wnd` - window (canvas)
    /// * `ctx` - widget context
    pub fn draw(&self, wnd: &Window, ctx: &WidgetContext) -> Option<(usize, usize)> {
        // get substring to display
        let visible_end = self.start + ctx.width.min(self.length() - self.start);
        let start = self.char2byte(self.start);
        let end = self.char2byte(visible_end);
        let mut substr = self.value[start..end].to_string();

        // erase line up to the end
        let len = substr.graphemes(true).count();
        if len < self.width {
            substr += &" ".repeat(self.width - len);
        }

        // draw
        wnd.print(ctx.x, ctx.y, &substr);
        if !self.history.is_empty() {
            wnd.print(ctx.x + ctx.width - 1, ctx.y, "\u{25bc}");
        }
        let color = if ctx.focused {
            Color::EditFocused
        } else {
            Color::EditNormal
        };
        wnd.color(ctx.x, ctx.y, ctx.width, color);

        if ctx.focused {
            if self.selection {
                wnd.color(ctx.x, ctx.y, self.cursor - self.start, Color::EditSelection);
            }
            Some((ctx.x + self.cursor - self.start, ctx.y))
        } else {
            None
        }
    }

    /// Keyboard input handler.
    ///
    /// # Arguments
    ///
    /// * `key` - pressed key
    ///
    /// # Return value
    ///
    /// `true` if key was handled.
    pub fn key_press(&mut self, key: &KeyPress) -> bool {
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
            _ => {
                return false;
            }
        };
        true
    }

    /// Focus set handler.
    pub fn on_focus_set(&mut self) {
        self.move_cursor(isize::max_value());
        self.selection = true;
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
            InputFormat::Any => true,
            InputFormat::HexStream => self.value.len() < max_stream && ch.is_ascii_hexdigit(),
            InputFormat::HexSigned => {
                (self.value.len() < max_hex && ch.is_ascii_hexdigit())
                    || ((self.cursor == 0 || self.selection) && (ch == '-' || ch == '+'))
            }
            InputFormat::HexUnsigned => self.value.len() <= max_hex && ch.is_ascii_hexdigit(),
            InputFormat::DecSigned => {
                (self.value.len() < max_dec && ch.is_ascii_digit())
                    || ((self.cursor == 0 || self.selection) && (ch == '-' || ch == '+'))
            }
            InputFormat::DecUnsigned => self.value.len() <= max_dec && ch.is_ascii_digit(),
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
        if forward && self.current + 1 < self.history.len() {
            self.current += 1;
        } else if !forward && self.current > 0 {
            self.current -= 1;
        } else {
            return;
        }
        self.value = self.history[self.current].clone();
        self.move_cursor(isize::MAX);
        self.selection = true;
    }

    /// Get length of the string in visual characters (grapheme).
    fn length(&self) -> usize {
        self.value.graphemes(true).count()
    }

    /// Convert char (grapheme) position to byte offset inside the value.
    fn char2byte(&self, char_pos: usize) -> usize {
        let mut byte_pos = 0;
        if char_pos > 0 {
            let (i, gr) = self
                .value
                .grapheme_indices(true)
                .nth(char_pos - 1)
                .expect("Invalid position");
            byte_pos = i + gr.len();
        }
        byte_pos
    }
}

/// Value formats.
#[derive(Clone, PartialEq)]
pub enum InputFormat {
    Any,
    HexStream,
    HexSigned,
    HexUnsigned,
    DecSigned,
    DecUnsigned,
}
