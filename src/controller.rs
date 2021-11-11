// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::config::Config;
use super::curses::{Color, Curses, Event, Key, KeyPress, Window};
use super::cursor::{Direction, HalfByte, Place};
use super::editor::{Editor, Focus};
use super::history::History;
use super::ui::cut::CutDialog;
use super::ui::dialog::DialogType;
use super::ui::fill::FillDialog;
use super::ui::goto::GotoDialog;
use super::ui::insert::InsertDialog;
use super::ui::messagebox::MessageBox;
use super::ui::progress::ProgressDialog;
use super::ui::saveas::SaveAsDialog;
use super::ui::search::SearchDialog;
use super::ui::setup::SetupDialog;
use super::ui::widget::StandardButton;
use std::io::{ErrorKind, Result};
use std::path::Path;

/// Controller: accepts input and converts it to commands for editor.
pub struct Controller {
    /// Editor (business logic).
    editor: Editor,
    /// History (seach, goto, etc).
    history: History,
    /// App configuration.
    config: Config,
    /// Keybar window.
    keybar: Window,
}

impl Controller {
    /// Min width of the screen.
    pub const MIN_WIDTH: usize = 60;
    /// Min height of the screen.
    pub const MIN_HEIGHT: usize = 16;

    /// Run controller.
    ///
    /// # Arguments
    ///
    /// * `files` - files to open
    /// * `offset` - desirable initial offset
    /// * `config` - configuration
    pub fn run(files: &[String], offset: Option<u64>, config: Config) -> Result<()> {
        let history = History::default();

        // find initial offset
        let initial_offset = if let Some(offset) = offset {
            offset
        } else {
            let mut offset = 0;
            for file in files {
                if let Some(val) = history.get_filepos(file) {
                    offset = val;
                    break;
                }
            }
            offset
        };

        // create controller instance
        let mut instance = Self {
            editor: Editor::new(files, &config)?,
            keybar: Window::default(),
            history,
            config,
        };

        instance.resize();
        if initial_offset != 0 {
            instance
                .editor
                .move_cursor(&Direction::Absolute(initial_offset, 0));
        }
        instance.main_loop();

        Ok(())
    }

    /// Main loop.
    fn main_loop(&mut self) {
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
                        if !self.key_input_common(&key) {
                            if self.editor.current().cursor.place == Place::Hex {
                                self.key_input_hex(&key);
                            } else {
                                self.key_input_ascii(&key);
                            }
                        }
                    }
                },
            }
        }
    }

    /// Common keyboard input handler.
    ///
    /// # Arguments
    ///
    /// * `key` - pressed key
    ///
    /// # Return value
    ///
    /// true if key was handled
    fn key_input_common(&mut self, key: &KeyPress) -> bool {
        match key.key {
            Key::F(1) => {
                Controller::help();
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
                self.goto();
                true
            }
            Key::F(5) => {
                if key.modifier == KeyPress::SHIFT {
                    self.find_closest(self.history.search_backward);
                } else if key.modifier == KeyPress::ALT {
                    self.find_closest(!self.history.search_backward);
                } else {
                    self.find();
                }
                true
            }
            Key::F(6) => {
                self.fill();
                true
            }
            Key::F(7) => {
                self.insert();
                true
            }
            Key::F(8) => {
                self.cut();
                true
            }
            Key::F(9) => {
                if SetupDialog::show(&mut self.config) {
                    self.editor.config_changed(&self.config);
                }
                true
            }
            Key::Tab => {
                let dir = if key.modifier == KeyPress::SHIFT {
                    Focus::PreviousField
                } else {
                    Focus::NextField
                };
                self.editor.switch_focus(&dir);
                true
            }
            Key::Left => {
                if key.modifier == KeyPress::NONE {
                    self.editor.move_cursor(&Direction::PrevByte);
                } else if key.modifier == KeyPress::SHIFT {
                    self.editor.move_cursor(&Direction::PrevWord);
                } else if key.modifier == KeyPress::ALT {
                    self.editor.closest_change(false);
                } else if key.modifier == KeyPress::CTRL {
                    self.editor.switch_focus(&Focus::PreviousField);
                }
                true
            }
            Key::Right => {
                if key.modifier == KeyPress::NONE {
                    self.editor.move_cursor(&Direction::NextByte);
                } else if key.modifier == KeyPress::SHIFT {
                    self.editor.move_cursor(&Direction::NextWord);
                } else if key.modifier == KeyPress::ALT {
                    self.editor.closest_change(true);
                } else if key.modifier == KeyPress::CTRL {
                    self.editor.switch_focus(&Focus::NextField);
                }
                true
            }
            Key::Up => {
                if key.modifier == KeyPress::NONE {
                    self.editor.move_cursor(&Direction::LineUp);
                } else if key.modifier == KeyPress::SHIFT {
                    self.editor.move_cursor(&Direction::ScrollUp);
                } else if key.modifier == KeyPress::ALT {
                    self.editor.closest_change(false);
                } else if key.modifier == KeyPress::CTRL {
                    self.editor.switch_focus(&Focus::PreviousDocument);
                }
                true
            }
            Key::Down => {
                if key.modifier == KeyPress::NONE {
                    self.editor.move_cursor(&Direction::LineDown);
                } else if key.modifier == KeyPress::SHIFT {
                    self.editor.move_cursor(&Direction::ScrollDown);
                } else if key.modifier == KeyPress::ALT {
                    self.editor.closest_change(true);
                } else if key.modifier == KeyPress::CTRL {
                    self.editor.switch_focus(&Focus::NextDocument);
                }
                true
            }
            Key::Home => {
                if key.modifier == KeyPress::NONE {
                    self.editor.move_cursor(&Direction::LineBegin);
                } else if key.modifier == KeyPress::CTRL {
                    self.editor.move_cursor(&Direction::FileBegin);
                }
                true
            }
            Key::End => {
                if key.modifier == KeyPress::NONE {
                    self.editor.move_cursor(&Direction::LineEnd);
                } else if key.modifier == KeyPress::CTRL {
                    self.editor.move_cursor(&Direction::FileEnd);
                }
                true
            }
            Key::PageUp => {
                self.editor.move_cursor(&Direction::PageUp);
                true
            }
            Key::PageDown => {
                self.editor.move_cursor(&Direction::PageDown);
                true
            }
            Key::Char('z') => {
                if key.modifier == KeyPress::CTRL {
                    self.editor.undo();
                    true
                } else {
                    false
                }
            }
            Key::Char('r' | 'y') => {
                if key.modifier == KeyPress::CTRL {
                    self.editor.redo();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Keyboard input handler (HEX field focused).
    ///
    /// # Arguments
    ///
    /// * `key` - pressed key
    fn key_input_hex(&mut self, key: &KeyPress) {
        match key.key {
            Key::Backspace => {
                self.editor.move_cursor(&Direction::PrevHalf);
            }
            Key::Char('G') => {
                self.editor.move_cursor(&Direction::FileEnd);
            }
            Key::Char('g') => {
                self.editor.move_cursor(&Direction::FileBegin);
            }
            Key::Char(':') => {
                self.goto();
            }
            Key::Char('/') => {
                self.find();
            }
            Key::Char('n') => {
                self.find_closest(self.history.search_backward);
            }
            Key::Char('N') => {
                self.find_closest(!self.history.search_backward);
            }
            Key::Char('h') => {
                self.editor.move_cursor(&Direction::PrevByte);
            }
            Key::Char('l') => {
                self.editor.move_cursor(&Direction::NextByte);
            }
            Key::Char('j') => {
                self.editor.move_cursor(&Direction::LineUp);
            }
            Key::Char('k') => {
                self.editor.move_cursor(&Direction::LineDown);
            }
            Key::Char('a'..='f' | 'A'..='F' | '0'..='9') => {
                if key.modifier == KeyPress::NONE {
                    if let Key::Char(chr) = key.key {
                        let half = match chr {
                            'a'..='f' => chr as u8 - b'a' + 10,
                            'A'..='F' => chr as u8 - b'A' + 10,
                            '0'..='9' => chr as u8 - b'0',
                            _ => unreachable!(),
                        };
                        let cursor = &self.editor.current().cursor;
                        let offset = cursor.offset;
                        let (value, mask) = if cursor.half == HalfByte::Left {
                            (half << 4, 0xf0)
                        } else {
                            (half, 0x0f)
                        };
                        self.editor.change(offset, value, mask);
                        self.editor.move_cursor(&Direction::NextHalf);
                    }
                }
            }
            Key::Char('u') => {
                self.editor.undo();
            }
            _ => {}
        }
    }

    /// Keyboard input handler (ASCII field focused).
    ///
    /// # Arguments
    ///
    /// * `key` - pressed key
    fn key_input_ascii(&mut self, key: &KeyPress) {
        if let Key::Char(' '..='~') = key.key {
            if key.modifier == KeyPress::NONE {
                if let Key::Char(chr) = key.key {
                    let offset = self.editor.current().cursor.offset;
                    self.editor.change(offset, chr as u8, 0xff);
                    self.editor.move_cursor(&Direction::NextByte);
                }
            }
        }
    }

    /// Draw editor.
    fn draw(&self) {
        Window::hide_cursor();

        // draw key bar (bottom Fn line).
        let (width, _) = self.keybar.get_size();
        let titles = &[
            "Help",   // F1
            "Save",   // F2
            "Goto",   // F3
            "",       // F4
            "Find",   // F5
            "Fill",   // F6
            "Insert", // F7
            "Cut",    // F8
            "Setup",  // F9
            "Exit",   // F10
        ];
        let mut fn_line = String::new();
        let fn_width = width / 10;
        let max_name = fn_width - 2 /* fn number */;
        for (i, name) in titles.iter().enumerate() {
            let mut name = name.to_string();
            if name.len() > max_name {
                name.truncate(max_name);
            }
            fn_line += &format!("{:>2}{:<width$}", i + 1, name, width = max_name);
        }
        fn_line += &" ".repeat(width - fn_line.len());
        self.keybar.print(0, 0, &fn_line);
        self.keybar.color(0, 0, width, Color::KeyBarTitle);
        for i in 0..10 {
            self.keybar.color(i * fn_width, 0, 2, Color::KeyBarId);
        }
        self.keybar.refresh();

        // draw documents
        self.editor.draw();
    }

    /// Screen resize handler.
    fn resize(&mut self) {
        let (width, height) = Curses::screen_size();

        // check for minimal screen size
        if width < Controller::MIN_WIDTH || height < Controller::MIN_HEIGHT {
            eprintln!(
                "Screen size too small, need at least {}x{}",
                Controller::MIN_WIDTH,
                Controller::MIN_HEIGHT
            );
            return;
        }

        self.keybar.resize(width, 1);
        self.keybar.set_pos(0, height - 1);
        self.editor.resize(width, height - 1);
    }

    /// Show mini help.
    fn help() {
        MessageBox::show(
            DialogType::Normal,
            "XVI: Hex editor",
            &[
                "Arrows, PgUp, PgDown: move cursor;",
                "Tab: switch between Hex/ASCII;",
                "u or Ctrl+z: undo;",
                "Ctrl+r or Ctrl+y: redo;",
                "F2: save file; Shift+F2: save as;",
                "Esc or F10: exit.",
            ],
            &[(StandardButton::OK, true)],
        );
    }

    /// Save current file, returns false if operation failed.
    fn save(&mut self) -> bool {
        if !self.editor.current().file.is_modified() {
            return true;
        }
        loop {
            match self.editor.save() {
                Ok(()) => {
                    return true;
                }
                Err(err) => {
                    if err.kind() == ErrorKind::Interrupted
                        || !MessageBox::retry_write(&self.editor.current().file.path, &err)
                    {
                        return false;
                    }
                }
            }
        }
    }

    /// Save current file with new name.
    fn save_as(&mut self) {
        let name = self.editor.current().file.path.to_string();
        if let Some(name) = SaveAsDialog::show(name) {
            loop {
                let mut progress = ProgressDialog::new("Save as...", true);
                match self.editor.save_as(Path::new(&name), &mut progress) {
                    Ok(()) => {
                        break;
                    }
                    Err(err) => {
                        progress.hide();
                        if err.kind() == ErrorKind::Interrupted
                            || !MessageBox::retry_write(&self.editor.current().file.path, &err)
                        {
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Goto to specified address.
    fn goto(&mut self) {
        if let Some(offset) =
            GotoDialog::show(&self.history.goto, self.editor.current().cursor.offset)
        {
            self.history.add_goto(offset);
            self.editor.move_cursor(&Direction::Absolute(offset, 0));
        }
    }

    /// Find position of the sequence.
    fn find(&mut self) {
        if let Some((seq, bkg)) =
            SearchDialog::show(&self.history.search, self.history.search_backward)
        {
            self.history.search_backward = bkg;
            self.history.add_search(&seq);
            self.find_closest(self.history.search_backward);
        }
    }

    /// Find next/previous position of the sequence.
    fn find_closest(&mut self, backward: bool) {
        if self.history.search.is_empty() {
            self.history.search_backward = backward;
            self.find();
        } else {
            let mut progress = ProgressDialog::new("Searching...", false);
            match self.editor.find(
                self.editor.current().cursor.offset,
                &self.history.search[0],
                backward,
                &mut progress,
            ) {
                Ok(()) => {}
                Err(err) => {
                    progress.hide();
                    match err.kind() {
                        ErrorKind::Interrupted => { /*skip*/ }
                        ErrorKind::NotFound => {
                            MessageBox::show(
                                DialogType::Error,
                                "Search",
                                &[
                                    "Sequence not found in file",
                                    &self.editor.current().file.path,
                                ],
                                &[(StandardButton::OK, true)],
                            );
                        }
                        _ => {
                            MessageBox::error_read(
                                &self.editor.current().file.path,
                                &err,
                                &[(StandardButton::Cancel, true)],
                            );
                        }
                    }
                }
            }
        }
    }

    /// Fill range.
    fn fill(&mut self) {
        let current = self.editor.current();
        if let Some((range, pattern)) = FillDialog::show(
            current.cursor.offset,
            current.file.size,
            &self.history.pattern,
        ) {
            self.history.pattern = pattern;
            self.editor.fill(&range, &self.history.pattern);
        }
    }

    /// Insert bytes.
    fn insert(&mut self) {
        let file = &self.editor.current().file;
        if file.is_modified() {
            MessageBox::show(
                DialogType::Error,
                "Insert bytes",
                &[
                    &file.path,
                    "was modified.",
                    "Please save or undo your changes first.",
                ],
                &[(StandardButton::OK, true)],
            );
            return;
        }
        if let Some((mut offset, size, pattern)) =
            InsertDialog::show(self.editor.current().cursor.offset, &self.history.pattern)
        {
            self.history.pattern = pattern;
            if offset > file.size {
                offset = file.size;
            }
            let mut progress = ProgressDialog::new("Insert bytes...", true);
            if let Err(err) = self
                .editor
                .insert(offset, size, &self.history.pattern, &mut progress)
            {
                if err.kind() != ErrorKind::Interrupted {
                    progress.hide();
                    MessageBox::error_write(
                        &self.editor.current().file.path,
                        &err,
                        &[(StandardButton::Cancel, true)],
                    );
                }
            }
        }
    }

    /// Cut out range.
    fn cut(&mut self) {
        let file = &self.editor.current().file;
        if file.is_modified() {
            MessageBox::show(
                DialogType::Error,
                "Cut range",
                &[
                    &file.path,
                    "was modified.",
                    "Please save or undo your changes first.",
                ],
                &[(StandardButton::OK, true)],
            );
            return;
        }
        if let Some(range) = CutDialog::show(self.editor.current().cursor.offset, file.size) {
            let mut progress = ProgressDialog::new("Cutting out range...", true);
            if let Err(err) = self.editor.cut(&range, &mut progress) {
                if err.kind() != ErrorKind::Interrupted {
                    progress.hide();
                    MessageBox::error_write(
                        &self.editor.current().file.path,
                        &err,
                        &[(StandardButton::Cancel, true)],
                    );
                }
            }
        }
    }

    /// Exit from editor.
    fn exit(&mut self) -> bool {
        for index in 0..self.editor.len() {
            self.editor.switch_focus(&Focus::DocumentIndex(index));
            let current = self.editor.current();
            if !current.file.is_modified() {
                continue;
            }

            // ask for save
            if let Some(button) = MessageBox::show(
                DialogType::Error,
                "Exit",
                &[&current.file.path, "was modified.", "Save before exit?"],
                &[
                    (StandardButton::Yes, false),
                    (StandardButton::No, false),
                    (StandardButton::Cancel, true),
                ],
            ) {
                match button {
                    StandardButton::Yes => {
                        // save current document
                        if !self.save() {
                            return false;
                        }
                    }
                    StandardButton::Cancel => {
                        return false;
                    }
                    _ => {}
                }
            } else {
                return false;
            }
        }

        // save history
        let (offset, files) = self.editor.get_files();
        files
            .iter()
            .for_each(|f| self.history.add_filepos(f, offset));
        self.history.save();

        true
    }
}
