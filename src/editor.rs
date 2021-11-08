// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::config::Config;
use super::curses::{Color, Curses, Event, Key, KeyPress, Window};
use super::cursor::{Direction, HalfByte, Place};
use super::document::Document;
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
use std::collections::BTreeSet;

/// Editor: implements business logic of a hex editor.
pub struct Editor {
    /// Editable documents.
    documents: Vec<Document>,
    /// Index of currently selected document.
    current: usize,

    /// Keybar window.
    keybar: Window,

    /// Search history.
    search_history: Vec<Vec<u8>>,
    /// Last used search direction.
    search_backward: bool,
    /// Address history.
    goto_history: Vec<u64>,
    /// Last pattern used in fill/insert operations.
    pattern: Vec<u8>,
}

impl Editor {
    /// Create new editor instance.
    pub fn new(
        files: &[String],
        offset: Option<u64>,
        config: &Config,
    ) -> Result<Self, std::io::Error> {
        let history = History::default();
        let keybar = Window::default();

        // create document instances
        let mut documents = Vec::with_capacity(files.len());
        for file in files {
            documents.push(Document::new(file, config)?);
        }

        // create instance
        let mut instance = Self {
            documents,
            current: 0,
            keybar,
            search_history: history.get_search(),
            search_backward: false,
            goto_history: history.get_goto(),
            pattern: vec![0],
        };
        instance.resize();

        // define and apply initial offset
        let mut initial_offset = 0;
        if let Some(offset) = offset {
            initial_offset = offset;
        } else {
            for doc in &instance.documents {
                if let Some(offset) = history.get_filepos(&doc.file.path) {
                    initial_offset = offset;
                    break;
                }
            }
        }
        instance.move_cursor(&Direction::Absolute(initial_offset));

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
                        if !self.key_input_common(&key) {
                            if self.documents[self.current].cursor.place == Place::Hex {
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
                Editor::help();
                true
            }
            Key::F(2) => {
                if key.modifier == KeyPress::NONE {
                    Editor::save(&mut self.documents[self.current]);
                } else if key.modifier == KeyPress::SHIFT {
                    Editor::save_as(&mut self.documents[self.current]);
                }
                true
            }
            Key::F(3) => {
                self.goto();
                true
            }
            Key::F(5) => {
                if key.modifier == KeyPress::SHIFT {
                    self.find_closest(self.search_backward);
                } else if key.modifier == KeyPress::ALT {
                    self.find_closest(!self.search_backward);
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
                self.setup();
                true
            }
            Key::Tab => {
                self.switch_field(key.modifier != KeyPress::SHIFT);
                true
            }
            Key::Left => {
                if key.modifier == KeyPress::NONE {
                    self.move_cursor(&Direction::PrevByte);
                } else if key.modifier == KeyPress::SHIFT {
                    self.move_cursor(&Direction::PrevWord);
                } else if key.modifier == KeyPress::ALT {
                    self.closest_change(false);
                } else if key.modifier == KeyPress::CTRL
                    && self.documents[self.current].cursor.place == Place::Ascii
                {
                    self.documents
                        .iter_mut()
                        .for_each(|doc| doc.cursor.set_place(Place::Hex));
                }
                true
            }
            Key::Right => {
                if key.modifier == KeyPress::NONE {
                    self.move_cursor(&Direction::NextByte);
                } else if key.modifier == KeyPress::SHIFT {
                    self.move_cursor(&Direction::NextWord);
                } else if key.modifier == KeyPress::ALT {
                    self.closest_change(true);
                } else if key.modifier == KeyPress::CTRL
                    && self.documents[self.current].view.ascii_table.is_some()
                    && self.documents[self.current].cursor.place == Place::Hex
                {
                    self.documents
                        .iter_mut()
                        .for_each(|doc| doc.cursor.set_place(Place::Ascii));
                }
                true
            }
            Key::Up => {
                if key.modifier == KeyPress::NONE {
                    self.move_cursor(&Direction::LineUp);
                } else if key.modifier == KeyPress::SHIFT {
                    self.move_cursor(&Direction::ScrollUp);
                } else if key.modifier == KeyPress::ALT {
                    self.closest_change(false);
                } else if key.modifier == KeyPress::CTRL && self.current > 0 {
                    self.current -= 1;
                }
                true
            }
            Key::Down => {
                if key.modifier == KeyPress::NONE {
                    self.move_cursor(&Direction::LineDown);
                } else if key.modifier == KeyPress::SHIFT {
                    self.move_cursor(&Direction::ScrollDown);
                } else if key.modifier == KeyPress::ALT {
                    self.closest_change(true);
                } else if key.modifier == KeyPress::CTRL && self.current + 1 < self.documents.len()
                {
                    self.current += 1;
                }
                true
            }
            Key::Home => {
                if key.modifier == KeyPress::NONE {
                    self.move_cursor(&Direction::LineBegin);
                } else if key.modifier == KeyPress::CTRL {
                    self.move_cursor(&Direction::FileBegin);
                }
                true
            }
            Key::End => {
                if key.modifier == KeyPress::NONE {
                    self.move_cursor(&Direction::LineEnd);
                } else if key.modifier == KeyPress::CTRL {
                    self.move_cursor(&Direction::FileEnd);
                }
                true
            }
            Key::PageUp => {
                self.move_cursor(&Direction::PageUp);
                true
            }
            Key::PageDown => {
                self.move_cursor(&Direction::PageDown);
                true
            }
            Key::Char('z') => {
                if key.modifier == KeyPress::CTRL {
                    self.documents[self.current].undo();
                    true
                } else {
                    false
                }
            }
            Key::Char('r' | 'y') => {
                if key.modifier == KeyPress::CTRL {
                    self.documents[self.current].redo();
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
                self.move_cursor(&Direction::PrevHalf);
            }
            Key::Char('G') => {
                self.move_cursor(&Direction::FileEnd);
            }
            Key::Char('g') => {
                self.move_cursor(&Direction::FileBegin);
            }
            Key::Char(':') => {
                self.goto();
            }
            Key::Char('/') => {
                self.find();
            }
            Key::Char('n') => {
                self.find_closest(self.search_backward);
            }
            Key::Char('N') => {
                self.find_closest(!self.search_backward);
            }
            Key::Char('h') => {
                self.move_cursor(&Direction::PrevByte);
            }
            Key::Char('l') => {
                self.move_cursor(&Direction::NextByte);
            }
            Key::Char('j') => {
                self.move_cursor(&Direction::LineUp);
            }
            Key::Char('k') => {
                self.move_cursor(&Direction::LineDown);
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
                        let (value, mask) =
                            if self.documents[self.current].cursor.half == HalfByte::Left {
                                (half << 4, 0xf0)
                            } else {
                                (half, 0x0f)
                            };
                        self.documents[self.current].modify_cur(value, mask);
                        self.move_cursor(&Direction::NextHalf);
                    }
                }
            }
            Key::Char('u') => {
                self.documents[self.current].undo();
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
                    self.documents[self.current].modify_cur(chr as u8, 0xff);
                    self.move_cursor(&Direction::NextByte);
                }
            }
        }
    }

    /// Move cursor.
    ///
    /// # Arguments
    ///
    /// * `dir` - move direction
    fn move_cursor(&mut self, dir: &Direction) {
        self.documents[self.current].move_cursor(dir);
        let offset = self.documents[self.current].cursor.offset;
        for (index, doc) in self.documents.iter_mut().enumerate() {
            if self.current != index {
                doc.move_cursor(dir);
                if doc.cursor.offset != offset {
                    doc.move_cursor(&Direction::Absolute(offset));
                }
            }
        }

        // update diff
        if self.documents.len() > 1 {
            self.update_diff();
        }
    }

    /// Jump to the closest change or diff byte.
    ///
    /// # Arguments
    ///
    /// * `forward` - search direction
    fn closest_change(&mut self, forward: bool) {
        let doc = &self.documents[self.current];

        // offset of the closest changed byte
        let changed = if forward {
            if let Some((offset, _)) = doc
                .file
                .changes
                .range((doc.cursor.offset + 1)..u64::MAX)
                .min()
            {
                Some(offset)
            } else {
                None
            }
        } else if let Some((offset, _)) = doc.file.changes.range(0..doc.cursor.offset).max() {
            Some(offset)
        } else {
            None
        };

        if let Some(&offset) = changed {
            self.move_cursor(&Direction::Absolute(offset));
        }
    }

    /// Calculate diff between opened files.
    fn update_diff(&mut self) {
        for index in 0..self.documents.len() {
            let mut diff = BTreeSet::new();
            let offset = self.documents[index].view.offset;
            let size = self.documents[index].view.lines * self.documents[index].view.columns;
            let data_l = self.documents[index].file.read(offset, size).unwrap();
            for (_, doc_r) in self
                .documents
                .iter_mut()
                .enumerate()
                .filter(|(i, _)| *i != index)
            {
                let data_r = if offset >= doc_r.file.size {
                    vec![]
                } else {
                    doc_r.file.read(offset, size).unwrap()
                };
                for (index, byte_l) in data_l.iter().enumerate() {
                    let mut equal = false;
                    if let Some(byte_r) = data_r.get(index) {
                        equal = byte_l == byte_r;
                    }
                    if !equal {
                        diff.insert(offset + index as u64);
                    }
                }
            }
            self.documents[index].view.differs = diff;
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
        for i in 0_usize..10 {
            fn_line += &format!(
                "{:>2}{:<width$}",
                i + 1,
                titles[i as usize],
                width = fn_width - 2
            );
        }
        self.keybar.print(0, 0, &fn_line);
        self.keybar.color(0, 0, width, Color::KeyBarTitle);
        for i in 0..10 {
            self.keybar.color(i * fn_width, 0, 2, Color::KeyBarId);
        }
        self.keybar.refresh();

        // draw documents
        self.documents.iter().for_each(|doc| doc.view.draw(doc));
        // show cursor for current document
        self.documents[self.current].show_cursor();
    }

    /// Screen resize handler.
    fn resize(&mut self) {
        let (scr_width, scr_height) = Curses::screen_size();
        self.keybar.resize(scr_width, 1);
        self.keybar.set_pos(0, scr_height - 1);

        let workspace_height = scr_height - 1; // key bar
        let view_height = workspace_height / self.documents.len();
        let last_index = self.documents.len() - 1;
        for (index, doc) in self.documents.iter_mut().enumerate() {
            let y = index * view_height;
            // enlarge last window to fit the screen
            let height = if index == last_index {
                workspace_height - view_height * last_index
            } else {
                view_height
            };
            doc.view.resize(y, scr_width, height);
        }
    }

    /// Switch focus between documents and fields:
    /// current hex -> current ascii -> next hex -> next ascii
    ///
    /// # Arguments
    ///
    /// * `forward` - switch direction
    fn switch_field(&mut self, forward: bool) {
        let current = &self.documents[self.current];
        let has_ascii = current.view.ascii_table.is_some();

        #[allow(clippy::collapsible_else_if)]
        if forward {
            if current.cursor.place == Place::Hex && has_ascii {
                self.documents
                    .iter_mut()
                    .for_each(|doc| doc.cursor.set_place(Place::Ascii));
            } else {
                self.current += 1;
                if self.current == self.documents.len() {
                    self.current = 0;
                }
                if current.cursor.place == Place::Ascii {
                    self.documents
                        .iter_mut()
                        .for_each(|doc| doc.cursor.set_place(Place::Hex));
                }
            }
        } else {
            if current.cursor.place == Place::Ascii {
                self.documents
                    .iter_mut()
                    .for_each(|doc| doc.cursor.set_place(Place::Hex));
            } else {
                if self.current > 0 {
                    self.current -= 1;
                } else {
                    self.current = self.documents.len() - 1;
                }
                if current.cursor.place == Place::Hex && has_ascii {
                    self.documents
                        .iter_mut()
                        .for_each(|doc| doc.cursor.set_place(Place::Ascii));
                }
            }
        }
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
    fn save(doc: &mut Document) -> bool {
        if !doc.file.is_modified() {
            return true;
        }
        loop {
            match doc.save() {
                Ok(()) => {
                    return true;
                }
                Err(err) => {
                    if let Some(button) = MessageBox::write_error(
                        &doc.file.path,
                        &err,
                        &[
                            (StandardButton::Retry, true),
                            (StandardButton::Cancel, false),
                        ],
                    ) {
                        if button != StandardButton::Retry {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
            }
        }
    }

    /// Save current file with new name, returns false if operation failed.
    fn save_as(doc: &mut Document) -> bool {
        if let Some(new_name) = SaveAsDialog::show(doc.file.path.clone()) {
            loop {
                //todo: let mut progress = ProgressDialog::new("Save as...");
                match doc.save_as(&new_name) {
                    Ok(()) => {
                        return true;
                    }
                    Err(err) => {
                        if let Some(button) = MessageBox::write_error(
                            &doc.file.path,
                            &err,
                            &[
                                (StandardButton::Retry, true),
                                (StandardButton::Cancel, false),
                            ],
                        ) {
                            if button != StandardButton::Retry {
                                return false;
                            }
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
        if let Some(offset) = GotoDialog::show(
            &self.goto_history,
            self.documents[self.current].cursor.offset,
        ) {
            self.goto_history.retain(|o| o != &offset);
            self.goto_history.insert(0, offset);
            self.move_cursor(&Direction::Absolute(offset));
        }
    }

    /// Find position of the sequence.
    fn find(&mut self) {
        if let Some((seq, bkg)) = SearchDialog::show(&self.search_history, self.search_backward) {
            self.search_backward = bkg;
            self.search_history.retain(|s| s != &seq);
            self.search_history.insert(0, seq);
            self.draw();
            self.find_closest(self.search_backward);
        }
    }

    /// Find next/previous position of the sequence.
    fn find_closest(&mut self, backward: bool) {
        if self.search_history.is_empty() {
            self.search_backward = backward;
            self.find();
        } else {
            let mut progress = ProgressDialog::new("Searching...");
            let doc = &mut self.documents[self.current];
            if let Some(offset) = doc.file.find(
                doc.cursor.offset,
                &self.search_history[0],
                backward,
                &mut progress,
            ) {
                self.move_cursor(&Direction::Absolute(offset));
            } else if !progress.canceled {
                progress.hide();
                MessageBox::show(
                    DialogType::Error,
                    "Search",
                    &["Sequence not found in file", &doc.file.path],
                    &[(StandardButton::OK, true)],
                );
            }
        }
    }

    /// Fill range.
    fn fill(&mut self) {
        let doc = &mut self.documents[self.current];
        if let Some((range, pattern)) =
            FillDialog::show(doc.cursor.offset, doc.file.size, &self.pattern)
        {
            self.pattern = pattern;
            let mut pattern_pos = 0;
            for offset in range.start..range.end {
                doc.modify_at(offset, self.pattern[pattern_pos]);
                pattern_pos += 1;
                if pattern_pos == self.pattern.len() {
                    pattern_pos = 0;
                }
            }
            doc.update();
            self.move_cursor(&Direction::Absolute(range.end));
        }
    }

    /// Insert bytes.
    fn insert(&mut self) {
        let doc = &self.documents[self.current];
        if doc.file.is_modified() {
            MessageBox::show(
                DialogType::Error,
                "Insert bytes",
                &[
                    &doc.file.path,
                    "was modified.",
                    "Please save or undo your changes first.",
                ],
                &[(StandardButton::OK, true)],
            );
            return;
        }
        if let Some((mut offset, size, pattern)) =
            InsertDialog::show(doc.cursor.offset, &self.pattern)
        {
            self.pattern = pattern;
            if offset > doc.file.size {
                offset = doc.file.size;
            }
            let mut progress = ProgressDialog::new("Write file...");
            let doc = &mut self.documents[self.current];
            if let Err(err) = doc.file.insert(offset, size, &self.pattern, &mut progress) {
                progress.hide();
                MessageBox::write_error(&doc.file.path, &err, &[(StandardButton::Cancel, true)]);
            }
            doc.on_file_changed(offset + size as u64);
        }
    }

    /// Cut out range.
    fn cut(&mut self) {
        let doc = &self.documents[self.current];
        if doc.file.is_modified() {
            MessageBox::show(
                DialogType::Error,
                "Cut range",
                &[
                    &doc.file.path,
                    "was modified.",
                    "Please save or undo your changes first.",
                ],
                &[(StandardButton::OK, true)],
            );
            return;
        }
        if let Some(range) = CutDialog::show(doc.cursor.offset, doc.file.size) {
            let mut progress = ProgressDialog::new("Write file...");
            let doc = &mut self.documents[self.current];
            if let Err(err) = doc.file.cut(&range, &mut progress) {
                progress.hide();
                MessageBox::write_error(&doc.file.path, &err, &[(StandardButton::Cancel, true)]);
            }
            doc.on_file_changed(range.end);
        }
    }

    /// Setup via GUI.
    fn setup(&mut self) {
        if SetupDialog::show(&mut self.documents[self.current].view) {
            let fixed_width = self.documents[self.current].view.fixed_width;
            let ascii_table = self.documents[self.current].view.ascii_table;
            for doc in &mut self.documents {
                doc.view.fixed_width = fixed_width;
                doc.view.ascii_table = ascii_table;
                if doc.view.ascii_table.is_none() {
                    doc.cursor.set_place(Place::Hex);
                }
            }
            self.resize();
        }
    }

    /// Exit from editor.
    fn exit(&mut self) -> bool {
        for doc in &mut self.documents {
            if !doc.file.is_modified() {
                continue;
            }
            if let Some(button) = MessageBox::show(
                DialogType::Error,
                "Exit",
                &[&doc.file.path, "was modified.", "Save before exit?"],
                &[
                    (StandardButton::Yes, false),
                    (StandardButton::No, false),
                    (StandardButton::Cancel, true),
                ],
            ) {
                match button {
                    StandardButton::Yes => {
                        if !Editor::save(doc) {
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
        let mut history = History::default();
        history.set_goto(&self.goto_history);
        history.set_search(&self.search_history);
        self.documents
            .iter()
            .for_each(|doc| history.add_filepos(&doc.file.path, doc.cursor.offset));
        history.save();

        true
    }
}
