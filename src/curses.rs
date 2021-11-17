// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use ncurses as nc;

/// Wrapper around ncurses.
pub struct Curses;
impl Curses {
    /// Initialization.
    pub fn initialize(colors: &[(Color, i16, i16)]) {
        // setup locale to get UTF-8 support
        nc::setlocale(nc::LcCategory::all, "");

        // setup ncurses
        let wnd = nc::initscr();
        nc::raw();
        nc::noecho();
        nc::keypad(wnd, true);
        nc::set_escdelay(0);

        // setup colors
        nc::start_color();
        nc::use_default_colors();
        for &(color, fg, bg) in colors.iter() {
            nc::init_pair(color as i16, fg, bg);
        }

        nc::bkgdset(nc::COLOR_PAIR(Color::HexNorm as i16));
        nc::clear();
    }

    /// Close ncurses.
    pub fn close() {
        nc::endwin();
    }

    /// Get screen size.
    ///
    /// # Return value
    ///
    /// Screen size (width, height).
    pub fn screen_size() -> (usize, usize) {
        let window = nc::stdscr();
        (
            nc::getmaxx(window).unsigned_abs() as usize,
            nc::getmaxy(window).unsigned_abs() as usize,
        )
    }

    /// Pass screen resize event.
    /// Used by dialogs to notify the controller about screen resize.
    pub fn screen_resize() {
        let window = nc::stdscr();
        let height = nc::getmaxy(window);
        let width = nc::getmaxx(window);
        ncurses::resizeterm(height, width);
    }

    /// Read next event.
    ///
    /// # Return value
    ///
    /// Event.
    fn read_event() -> Option<Event> {
        match nc::get_wch() {
            Some(nc::WchResult::Char(chr)) => {
                if chr == 0x1b {
                    // esc code, read next key - it can be alt+? combination
                    nc::timeout(10);
                    let key = nc::get_wch();
                    nc::timeout(-1);
                    if let Some(nc::WchResult::Char(chr)) = key {
                        if let Some(mut key) = Curses::key_from_char(chr) {
                            key.modifier |= KeyPress::ALT;
                            return Some(Event::KeyPress(key));
                        }
                    }
                    return Some(Event::KeyPress(KeyPress::new(Key::Esc, KeyPress::NONE)));
                }
                if let Some(key) = Curses::key_from_char(chr) {
                    return Some(Event::KeyPress(key));
                }
            }
            Some(nc::WchResult::KeyCode(key)) => match key {
                nc::KEY_RESIZE => {
                    nc::refresh();
                    return Some(Event::TerminalResize);
                }
                _ => {
                    if let Some(key) = Curses::key_from_code(key) {
                        return Some(Event::KeyPress(key));
                    }
                }
            },
            None => {}
        }
        None
    }

    /// Read next event (blocking).
    ///
    /// # Return value
    ///
    /// Event.
    pub fn wait_event() -> Event {
        loop {
            if let Some(event) = Curses::read_event() {
                return event;
            }
        }
    }

    /// Read next event (non blocking).
    ///
    /// # Return value
    ///
    /// Event.
    pub fn peek_event() -> Option<Event> {
        nc::timeout(0);
        let event = Curses::read_event();
        nc::timeout(-1);
        event
    }

    /// Create instance from ncurses code.
    fn key_from_code(code: i32) -> Option<KeyPress> {
        match code {
            nc::KEY_LEFT => Some(KeyPress::new(Key::Left, KeyPress::NONE)),
            nc::KEY_SLEFT => Some(KeyPress::new(Key::Left, KeyPress::SHIFT)),
            0x220 => Some(KeyPress::new(Key::Left, KeyPress::ALT)),
            0x221 => Some(KeyPress::new(Key::Left, KeyPress::ALT | KeyPress::SHIFT)),
            0x222 => Some(KeyPress::new(Key::Left, KeyPress::CTRL)),
            0x223 => Some(KeyPress::new(Key::Left, KeyPress::CTRL | KeyPress::SHIFT)),
            0x224 => Some(KeyPress::new(Key::Left, KeyPress::ALT | KeyPress::CTRL)),
            nc::KEY_RIGHT => Some(KeyPress::new(Key::Right, KeyPress::NONE)),
            nc::KEY_SRIGHT => Some(KeyPress::new(Key::Right, KeyPress::SHIFT)),
            0x22f => Some(KeyPress::new(Key::Right, KeyPress::ALT)),
            0x230 => Some(KeyPress::new(Key::Right, KeyPress::ALT | KeyPress::SHIFT)),
            0x231 => Some(KeyPress::new(Key::Right, KeyPress::CTRL)),
            0x232 => Some(KeyPress::new(Key::Right, KeyPress::CTRL | KeyPress::SHIFT)),
            0x233 => Some(KeyPress::new(Key::Right, KeyPress::ALT | KeyPress::CTRL)),
            nc::KEY_UP => Some(KeyPress::new(Key::Up, KeyPress::NONE)),
            nc::KEY_SR => Some(KeyPress::new(Key::Up, KeyPress::SHIFT)),
            0x235 => Some(KeyPress::new(Key::Up, KeyPress::ALT)),
            0x236 => Some(KeyPress::new(Key::Up, KeyPress::ALT | KeyPress::SHIFT)),
            0x237 => Some(KeyPress::new(Key::Up, KeyPress::CTRL)),
            0x238 => Some(KeyPress::new(Key::Up, KeyPress::CTRL | KeyPress::SHIFT)),
            0x239 => Some(KeyPress::new(Key::Up, KeyPress::ALT | KeyPress::CTRL)),
            nc::KEY_DOWN => Some(KeyPress::new(Key::Down, KeyPress::NONE)),
            nc::KEY_SF => Some(KeyPress::new(Key::Down, KeyPress::SHIFT)),
            0x20c => Some(KeyPress::new(Key::Down, KeyPress::ALT)),
            0x20d => Some(KeyPress::new(Key::Down, KeyPress::ALT | KeyPress::SHIFT)),
            0x20e => Some(KeyPress::new(Key::Down, KeyPress::CTRL)),
            0x20f => Some(KeyPress::new(Key::Down, KeyPress::CTRL | KeyPress::SHIFT)),
            0x210 => Some(KeyPress::new(Key::Down, KeyPress::ALT | KeyPress::CTRL)),
            nc::KEY_PPAGE => Some(KeyPress::new(Key::PageUp, KeyPress::NONE)),
            nc::KEY_NPAGE => Some(KeyPress::new(Key::PageDown, KeyPress::NONE)),
            nc::KEY_HOME => Some(KeyPress::new(Key::Home, KeyPress::NONE)),
            nc::KEY_SHOME => Some(KeyPress::new(Key::Home, KeyPress::SHIFT)),
            0x216 => Some(KeyPress::new(Key::Home, KeyPress::ALT)),
            0x217 => Some(KeyPress::new(Key::Home, KeyPress::ALT | KeyPress::SHIFT)),
            0x218 => Some(KeyPress::new(Key::Home, KeyPress::CTRL)),
            0x219 => Some(KeyPress::new(Key::Home, KeyPress::CTRL | KeyPress::SHIFT)),
            0x21a => Some(KeyPress::new(Key::Home, KeyPress::ALT | KeyPress::CTRL)),
            nc::KEY_END => Some(KeyPress::new(Key::End, KeyPress::NONE)),
            nc::KEY_SEND => Some(KeyPress::new(Key::End, KeyPress::SHIFT)),
            0x211 => Some(KeyPress::new(Key::End, KeyPress::ALT)),
            0x212 => Some(KeyPress::new(Key::End, KeyPress::ALT | KeyPress::SHIFT)),
            0x213 => Some(KeyPress::new(Key::End, KeyPress::CTRL)),
            0x214 => Some(KeyPress::new(Key::End, KeyPress::CTRL | KeyPress::SHIFT)),
            0x215 => Some(KeyPress::new(Key::End, KeyPress::ALT | KeyPress::CTRL)),
            nc::KEY_BTAB => Some(KeyPress::new(Key::Tab, KeyPress::SHIFT)),
            nc::KEY_DC => Some(KeyPress::new(Key::Delete, KeyPress::NONE)),
            nc::KEY_BACKSPACE => Some(KeyPress::new(Key::Backspace, KeyPress::NONE)),
            _ => {
                // check for Fn range
                if (nc::KEY_F1..nc::KEY_F1 + 64).contains(&code) {
                    let fn_max = 12;
                    let fn_code = code - nc::KEY_F1;
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let fn_num = 1 + (fn_code % fn_max) as u8;
                    let m = if fn_code >= fn_max * 4 {
                        KeyPress::ALT
                    } else if fn_code >= fn_max * 3 {
                        KeyPress::CTRL | KeyPress::SHIFT
                    } else if fn_code >= fn_max * 2 {
                        KeyPress::CTRL
                    } else if fn_code >= fn_max {
                        KeyPress::SHIFT
                    } else {
                        KeyPress::NONE
                    };
                    return Some(KeyPress::new(Key::F(fn_num), m));
                }
                None
            }
        }
    }

    /// Create instance from an ASCII character.
    fn key_from_char(chr: u32) -> Option<KeyPress> {
        if let Some(chr) = std::char::from_u32(chr) {
            if chr.is_ascii() {
                return match chr as u8 {
                    // check for control codes
                    0x01 => Some(KeyPress::new(Key::Char('a'), KeyPress::CTRL)),
                    0x02 => Some(KeyPress::new(Key::Char('b'), KeyPress::CTRL)),
                    0x03 => Some(KeyPress::new(Key::Char('c'), KeyPress::CTRL)),
                    0x04 => Some(KeyPress::new(Key::Char('d'), KeyPress::CTRL)),
                    0x05 => Some(KeyPress::new(Key::Char('e'), KeyPress::CTRL)),
                    0x06 => Some(KeyPress::new(Key::Char('f'), KeyPress::CTRL)),
                    0x07 => Some(KeyPress::new(Key::Char('g'), KeyPress::CTRL)),
                    0x08 => Some(KeyPress::new(Key::Char('h'), KeyPress::CTRL)),
                    0x09 => Some(KeyPress::new(Key::Tab, KeyPress::NONE)),
                    0x0a | 0x0d => Some(KeyPress::new(Key::Enter, KeyPress::NONE)),
                    0x0b => Some(KeyPress::new(Key::Char('k'), KeyPress::CTRL)),
                    0x0c => Some(KeyPress::new(Key::Char('l'), KeyPress::CTRL)),
                    0x0e => Some(KeyPress::new(Key::Char('n'), KeyPress::CTRL)),
                    0x0f => Some(KeyPress::new(Key::Char('o'), KeyPress::CTRL)),
                    0x10 => Some(KeyPress::new(Key::Char('p'), KeyPress::CTRL)),
                    0x11 => Some(KeyPress::new(Key::Char('q'), KeyPress::CTRL)),
                    0x12 => Some(KeyPress::new(Key::Char('r'), KeyPress::CTRL)),
                    0x13 => Some(KeyPress::new(Key::Char('s'), KeyPress::CTRL)),
                    0x14 => Some(KeyPress::new(Key::Char('t'), KeyPress::CTRL)),
                    0x15 => Some(KeyPress::new(Key::Char('u'), KeyPress::CTRL)),
                    0x16 => Some(KeyPress::new(Key::Char('v'), KeyPress::CTRL)),
                    0x17 => Some(KeyPress::new(Key::Char('w'), KeyPress::CTRL)),
                    0x18 => Some(KeyPress::new(Key::Char('x'), KeyPress::CTRL)),
                    0x19 => Some(KeyPress::new(Key::Char('y'), KeyPress::CTRL)),
                    0x1a => Some(KeyPress::new(Key::Char('z'), KeyPress::CTRL)),
                    0x1b => Some(KeyPress::new(Key::Esc, KeyPress::NONE)),
                    0x1c => Some(KeyPress::new(Key::Char('\\'), KeyPress::CTRL)),
                    0x1d => Some(KeyPress::new(Key::Char(']'), KeyPress::CTRL)),
                    0x1e => Some(KeyPress::new(Key::Char('^'), KeyPress::CTRL)),
                    0x1f => Some(KeyPress::new(Key::Char('_'), KeyPress::CTRL)),
                    0x7f => Some(KeyPress::new(Key::Backspace, KeyPress::NONE)),
                    // all other is an ascii char
                    _ => Some(KeyPress::new(Key::Char(chr), KeyPress::NONE)),
                };
            }
            // wide char
            return Some(KeyPress::new(Key::Char(chr), KeyPress::NONE));
        }
        None
    }
}

/// Color identifiers.
#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum Color {
    HexNorm = 1,
    HexMod,
    HexDiff,
    HexNormHi,
    HexModHi,
    HexDiffHi,
    AsciiNorm,
    AsciiMod,
    AsciiDiff,
    AsciiNormHi,
    AsciiModHi,
    AsciiDiffHi,
    Offset,
    OffsetHi,
    Bar,
    Dialog,
    Error,
    Disabled,
    Focused,
    Input,
    Select,
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

/// Curses window.
pub struct Window {
    /// Curses window.
    window: nc::WINDOW,
    /// Curses panel.
    panel: nc::PANEL,
    /// Window width.
    width: usize,
    /// Window height.
    height: usize,
}

impl Window {
    /// Create new window.
    ///
    /// # Arguments
    ///
    /// * `x` - window position: column number
    /// * `y` - window position: line number
    /// * `width` - window width
    /// * `height` - window height
    /// * `color` - background color of the window
    ///
    /// # Return value
    ///
    /// Window instance.
    pub fn new(x: usize, y: usize, width: usize, height: usize, color: Color) -> Self {
        // create curses entities
        let window = nc::newwin(height as i32, width as i32, y as i32, x as i32);
        let panel = nc::new_panel(window);
        nc::update_panels();

        // default background
        nc::wbkgdset(window, nc::COLOR_PAIR(color as i16));
        nc::werase(window);

        Self {
            window,
            panel,
            width,
            height,
        }
    }

    /// Create new centered window.
    ///
    /// # Arguments
    ///
    /// * `width` - window width
    /// * `height` - window height
    /// * `color` - background color of the window
    ///
    /// # Return value
    ///
    /// Window instance.
    pub fn new_centered(width: usize, height: usize, color: Color) -> Self {
        debug_assert!(width > 0);
        debug_assert!(height > 0);

        // get screen size
        let screen = nc::stdscr();
        let screen_width = nc::getmaxx(screen).unsigned_abs() as usize;
        let screen_height = nc::getmaxy(screen).unsigned_abs() as usize;

        // calculate window position, center of the screen
        let x = if width >= screen_width {
            0
        } else {
            screen_width / 2 - width / 2
        };
        let y = if height >= screen_height {
            0
        } else {
            (screen_height as f32 / 2.2) as usize - height / 2
        };

        Window::new(x, y, width, height, color)
    }

    /// Hide the window.
    pub fn hide(&self) {
        nc::hide_panel(self.panel);
    }

    /// Get window size.
    ///
    /// # Return value
    ///
    /// Size of the window (width,height).
    pub fn get_size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// Resize the window.
    ///
    /// # Arguments
    ///
    /// * `width` - window width
    /// * `height` - window height
    pub fn resize(&mut self, width: usize, height: usize) {
        debug_assert!(width > 0);
        debug_assert!(height > 0);

        self.width = width;
        self.height = height;
        nc::wresize(self.window, height as i32, width as i32);
    }

    /// Move window.
    ///
    /// # Arguments
    ///
    /// * `x` - absolute screen coordinates: column number
    /// * `y` - absolute screen coordinates: line number
    pub fn set_pos(&self, x: usize, y: usize) {
        let status = nc::mvwin(self.window, y as i32, x as i32);
        debug_assert_eq!(status, nc::OK);
    }

    /// Clear the window.
    pub fn clear(&self) {
        nc::werase(self.window);
    }

    /// Print text on the window.
    ///
    /// # Arguments
    ///
    /// * `x` - start column of the text
    /// * `y` - line number
    /// * `text` - text to print
    pub fn print(&self, x: usize, y: usize, text: &str) {
        debug_assert!(x <= self.width);
        debug_assert!(y <= self.height);
        nc::mvwaddstr(self.window, y as i32, x as i32, text);
    }

    /// Colorize the specified range.
    ///
    /// # Arguments
    ///
    /// * `x` - start column
    /// * `y` - line number
    /// * `width` - number of characters to colorize
    /// * `color` - color to set
    pub fn color(&self, x: usize, y: usize, width: usize, color: Color) {
        debug_assert!(x <= self.width);
        debug_assert!(y <= self.height);
        debug_assert!(width + x <= self.width);
        nc::mvwchgat(
            self.window,
            y as i32,
            x as i32,
            width as i32,
            0,
            color as i16,
        );
    }

    /// Set color for further prints.
    ///
    /// # Arguments
    ///
    /// * `color` - color to set
    pub fn color_on(&self, color: Color) {
        nc::wattron(self.window, nc::COLOR_PAIR(color as i16));
    }

    /// Refresh the window, flushes all changes to the screen.
    pub fn refresh(&self) {
        nc::wrefresh(self.window);
    }

    /// Show cursor at specified position.
    ///
    /// # Arguments
    ///
    /// * `x` - column number
    /// * `y` - line number
    pub fn show_cursor(&self, x: usize, y: usize) {
        // wmove doesn't work, use absolute coordinates
        let x = nc::getbegx(self.window) + x as i32;
        let y = nc::getbegy(self.window) + y as i32;
        nc::mv(y, x);
        nc::curs_set(nc::CURSOR_VISIBILITY::CURSOR_VISIBLE);
    }

    /// Hide cursor.
    pub fn hide_cursor() {
        nc::curs_set(nc::CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        nc::del_panel(self.panel);
        nc::update_panels();
        nc::delwin(self.window);
    }
}
