// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

/// Console UI interface.
pub trait Cui {
    /// Print text at the specified position.
    fn print(&self, x: usize, y: usize, text: &str);
    /// Colorize line.
    fn color(&self, x: usize, y: usize, width: usize, color: Color);
    /// Enable color for all further prints.
    fn color_on(&self, color: Color);
    /// Clear screen.
    fn clear(&self);
    /// Get screen size (width, height).
    fn size(&self) -> (usize, usize);
    /// Show cursor at specified position.
    fn show_cursor(&self, x: usize, y: usize);
    /// Hide cursor.
    fn hide_cursor(&self);
    /// Poll next event.
    fn poll_event(&self) -> Event;
}

/// Color identifiers.
#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum Color {
    OffsetNormal = 1,
    OffsetHi,
    HexNormal,
    HexHi,
    HexModified,
    HexModifiedHi,
    AsciiNormal,
    AsciiHi,
    AsciiModified,
    AsciiModifiedHi,
    StatusBar,
    KeyBarId,
    KeyBarTitle,
    DialogNormal,
    DialogError,
    DialogShadow,
    ItemDisabled,
    ItemFocused,
    EditNormal,
    EditFocused,
    EditSelection,
}

/// External event.
pub enum Event {
    /// Terminal window was resized.
    TerminalResize,
    /// Key pressed.
    KeyPress(KeyPress),
}

/// Key press event data: code with modifiers.
pub struct KeyPress {
    pub key: Key,
    pub modifier: u8,
}

impl KeyPress {
    pub const NONE: u8 = 0b000;
    pub const SHIFT: u8 = 0b001;
    pub const CTRL: u8 = 0b010;
    pub const ALT: u8 = 0b100;

    pub fn new(key: Key, modifier: u8) -> Self {
        Self { key, modifier }
    }
}

/// Key types.
#[derive(PartialEq)]
pub enum Key {
    // alphanumeric
    Char(char),
    // functional buttons (F1, F2, ...)
    F(u8),
    // special buttons
    Left,
    Right,
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    Tab,
    Backspace,
    Delete,
    Enter,
    Esc,
}

/// Window for drawing.
pub struct Window<'a> {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub cui: &'a dyn Cui,
}

impl<'a> Window<'a> {
    /// Print text on the window
    pub fn print(&self, x: usize, y: usize, text: &str) {
        debug_assert!(x <= self.width);
        debug_assert!(y <= self.height);
        self.cui.print(self.x + x, self.y + y, text);
    }

    /// Colorize area
    pub fn color(&self, x: usize, y: usize, width: usize, color: Color) {
        debug_assert!(x <= self.width);
        debug_assert!(y <= self.height);
        debug_assert!(x + width <= self.width);
        self.cui.color(self.x + x, self.y + y, width, color);
    }

    /// Enable color for all further prints.
    pub fn color_on(&self, color: Color) {
        self.cui.color_on(color);
    }

    /// Show cursor at specified position.
    pub fn show_cursor(&self, x: usize, y: usize) {
        debug_assert!(x <= self.width);
        debug_assert!(y <= self.height);
        self.cui.show_cursor(self.x + x, self.y + y);
    }
}
