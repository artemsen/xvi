// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::config::*;
use super::cui::*;

use ncurses as nc;

/// Wrapper around ncurses.
pub struct Curses {
    wnd: nc::WINDOW,
}

impl Curses {
    /// Initialization.
    pub fn new() -> Self {
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
        for (&color, &(fg, bg)) in Config::get().colors.iter() {
            nc::init_pair(color as i16, fg as i16, bg as i16);
        }

        nc::wbkgdset(wnd, nc::COLOR_PAIR(Color::HexNormal as i16));
        nc::wclear(wnd);

        Self { wnd }
    }

    /// Close ncurses.
    pub fn close() {
        nc::endwin();
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
            _ => {
                // check for Fn range
                if (nc::KEY_F1..nc::KEY_F1 + 64).contains(&code) {
                    let fn_max = 12;
                    let fn_code = code - nc::KEY_F1;
                    let f = 1 + fn_code % fn_max;
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
                    return Some(KeyPress::new(Key::F(f as u8), m));
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
                    0x0a => Some(KeyPress::new(Key::Enter, KeyPress::NONE)),
                    0x0b => Some(KeyPress::new(Key::Char('k'), KeyPress::CTRL)),
                    0x0c => Some(KeyPress::new(Key::Char('l'), KeyPress::CTRL)),
                    0x0d => Some(KeyPress::new(Key::Enter, KeyPress::NONE)),
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

impl Drop for Curses {
    fn drop(&mut self) {
        Curses::close();
    }
}

impl Cui for Curses {
    fn print(&self, x: usize, y: usize, text: &str) {
        nc::mvwaddstr(self.wnd, y as i32, x as i32, text);
    }

    fn color(&self, x: usize, y: usize, width: usize, color: Color) {
        nc::mvwchgat(self.wnd, y as i32, x as i32, width as i32, 0, color as i16);
    }

    fn color_on(&self, color: Color) {
        nc::wattron(self.wnd, nc::COLOR_PAIR(color as i16));
    }

    fn clear(&self) {
        nc::wclear(self.wnd);
    }

    fn size(&self) -> (usize, usize) {
        (
            nc::getmaxx(self.wnd) as usize,
            nc::getmaxy(self.wnd) as usize,
        )
    }

    fn show_cursor(&self, x: usize, y: usize) {
        nc::wmove(self.wnd, y as i32, x as i32);
        nc::curs_set(nc::CURSOR_VISIBILITY::CURSOR_VISIBLE);
    }

    fn hide_cursor(&self) {
        nc::curs_set(nc::CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    }

    fn poll_event(&self) -> Event {
        loop {
            match nc::wget_wch(self.wnd) {
                Some(nc::WchResult::Char(chr)) => {
                    if chr == 0x1b {
                        // esc code, read next key - it can be alt+? combination
                        nc::wtimeout(self.wnd, 10);
                        let key = nc::wget_wch(self.wnd);
                        nc::wtimeout(self.wnd, -1);
                        if let Some(nc::WchResult::Char(chr)) = key {
                            if let Some(mut key) = Curses::key_from_char(chr) {
                                key.modifier |= KeyPress::ALT;
                                return Event::KeyPress(key);
                            }
                        }
                        return Event::KeyPress(KeyPress::new(Key::Esc, KeyPress::NONE));
                    }
                    if let Some(key) = Curses::key_from_char(chr) {
                        return Event::KeyPress(key);
                    }
                }
                Some(nc::WchResult::KeyCode(key)) => match key {
                    nc::KEY_RESIZE => {
                        return Event::TerminalResize;
                    }
                    _ => {
                        if let Some(key) = Curses::key_from_code(key) {
                            return Event::KeyPress(key);
                        } else {
                            //let name = match nc::keyname(key) {
                            //    Some(n) => n,
                            //    None => String::from("?"),
                            //};
                            //println!("Unknown key: {} = 0x{:x} = {}", key, key, name);
                        }
                    }
                },
                None => {}
            }
        }
    }
}
