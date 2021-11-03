// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::config::Config;
use super::curses::*;
use super::cursor::*;
use super::document::Document;
use super::history::History;
use super::ui::cut::CutDlg;
use super::ui::dialog::DialogType;
use super::ui::fill::FillDlg;
use super::ui::goto::GotoDlg;
use super::ui::insert::InsertDlg;
use super::ui::messagebox::MessageBox;
use super::ui::progress::ProgressDlg;
use super::ui::saveas::SaveAsDlg;
use super::ui::search::SearchDlg;
use super::ui::setup::SetupDlg;
use super::ui::widget::StdButton;
use std::collections::BTreeSet;

/// Editor: implements business logic of a hex editor.
#[allow(dead_code)]
pub struct Editor {
    /// Editable documents.
    documents: Vec<Document>,
    /// Index of currently selected document.
    current: usize,

    /// "Goto" configuration dialog.
    goto_dlg: GotoDlg,
    /// Search configuration dialog.
    search_dlg: SearchDlg,
    /// View mode setup dialog.
    setup_dlg: SetupDlg,

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
        let history = History::new();

        // create document instances
        let mut documents = Vec::with_capacity(files.len());
        for file in files {
            documents.push(Document::new(file, config)?);
        }

        // create instance
        let mut instance = Self {
            documents,
            current: 0,
            goto_dlg: GotoDlg::default(),
            search_dlg: SearchDlg::default(),
            setup_dlg: SetupDlg {
                fixed_width: config.fixed_width,
                ascii_table: config.ascii_table,
            },
            pattern: vec![0],
        };
        instance.goto_dlg.history = history.get_goto();
        instance.search_dlg.history = history.get_search();
        instance.resize();

        // define and apply initial offset
        let mut initial_offset = 0;
        if let Some(offset) = offset {
            initial_offset = offset;
        } else {
            for doc in instance.documents.iter() {
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
                        self.handle_key(key);
                    }
                },
            }
        }
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
                    self.find_next(self.search_dlg.backward);
                } else if key.modifier == KeyPress::ALT {
                    self.find_next(!self.search_dlg.backward);
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
                if key.modifier == KeyPress::NONE {
                    if self.documents[self.current].cursor.place == Place::Hex
                        && self.documents[self.current].view.ascii_table.is_some()
                    {
                        self.documents
                            .iter_mut()
                            .for_each(|doc| doc.cursor.set_place(Place::Ascii));
                    } else {
                        if self.documents[self.current].cursor.place == Place::Ascii {
                            self.current += 1;
                            if self.current == self.documents.len() {
                                self.current = 0;
                            }
                        }
                        self.documents
                            .iter_mut()
                            .for_each(|doc| doc.cursor.set_place(Place::Hex));
                    }
                } else if key.modifier == KeyPress::SHIFT {
                    if self.documents[self.current].cursor.place == Place::Ascii {
                        self.documents
                            .iter_mut()
                            .for_each(|doc| doc.cursor.set_place(Place::Hex));
                    } else {
                        if self.documents[self.current].cursor.place == Place::Hex {
                            if self.current > 0 {
                                self.current -= 1;
                            } else {
                                self.current = self.documents.len() - 1;
                            }
                        }
                        let place = if self.documents[self.current].view.ascii_table.is_some() {
                            Place::Ascii
                        } else {
                            Place::Hex
                        };
                        self.documents
                            .iter_mut()
                            .for_each(|doc| doc.cursor.set_place(place.clone()));
                    }
                }
                true
            }
            Key::Left => {
                if key.modifier == KeyPress::NONE {
                    self.move_cursor(&Direction::PrevByte);
                } else if key.modifier == KeyPress::SHIFT {
                    self.move_cursor(&Direction::PrevHalf);
                } else if key.modifier == KeyPress::ALT {
                    self.move_cursor(&Direction::PrevWord);
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
                    self.move_cursor(&Direction::NextHalf);
                } else if key.modifier == KeyPress::ALT {
                    self.move_cursor(&Direction::NextWord);
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
                } else if key.modifier == KeyPress::ALT {
                    self.move_cursor(&Direction::ScrollUp);
                } else if key.modifier == KeyPress::CTRL && self.current > 0 {
                    self.current -= 1;
                }
                true
            }
            Key::Down => {
                if key.modifier == KeyPress::NONE {
                    self.move_cursor(&Direction::LineDown);
                } else if key.modifier == KeyPress::ALT {
                    self.move_cursor(&Direction::ScrollDown);
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
            Key::Char('r') => {
                if key.modifier == KeyPress::CTRL {
                    self.documents[self.current].redo();
                    true
                } else {
                    false
                }
            }
            Key::Char('y') => {
                if key.modifier == KeyPress::CTRL {
                    self.documents[self.current].redo();
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

        if self.documents[self.current].cursor.place == Place::Ascii {
            // ascii mode specific
            if let Key::Char(' '..='~') = key.key {
                if key.modifier == KeyPress::NONE {
                    if let Key::Char(chr) = key.key {
                        self.documents[self.current].modify_cur(chr as u8, 0xff);
                        self.move_cursor(&Direction::NextByte);
                    }
                }
            }
        } else {
            // hex mode specific
            match key.key {
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
                    self.find_next(self.search_dlg.backward);
                }
                Key::Char('N') => {
                    self.find_next(!self.search_dlg.backward);
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
                Key::Char('a'..='f') | Key::Char('A'..='F') | Key::Char('0'..='9') => {
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

    /// Caclulate diff between opened files.
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

    /// Show mini help.
    fn help(&self) {
        MessageBox::new("XVI: Hex editor", DialogType::Normal)
            .left("Arrows, PgUp, PgDown: move cursor;")
            .left("Tab: switch between Hex/ASCII;")
            .left("u or Ctrl+z: undo;")
            .left("Ctrl+r or Ctrl+y: redo;")
            .left("F2: save file; Shift+F2: save as;")
            .left("Esc or F10: exit.")
            .left("")
            .center("Read `man xvi` for more info.")
            .button(StdButton::Ok, true)
            .show();
    }

    /// Save current file, returns false if operation failed.
    fn save(doc: &mut Document) -> bool {
        loop {
            match doc.save() {
                Ok(()) => {
                    return true;
                }
                Err(err) => {
                    if let Some(btn) = MessageBox::new("Error", DialogType::Error)
                        .center("Error writing file")
                        .center(&doc.file.path)
                        .center(&format!("{}", err))
                        .button(StdButton::Retry, true)
                        .button(StdButton::Cancel, false)
                        .show()
                    {
                        if btn != StdButton::Retry {
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
        if let Some(new_name) = SaveAsDlg::default().show(doc.file.path.clone()) {
            loop {
                //todo: let mut progress = ProgressDlg::new("Save as...");
                match doc.save_as(new_name.clone()) {
                    Ok(()) => {
                        return true;
                    }
                    Err(err) => {
                        if let Some(btn) = MessageBox::new("Error", DialogType::Error)
                            .center("Error writing file")
                            .center(&doc.file.path)
                            .center(&format!("{}", err))
                            .button(StdButton::Retry, true)
                            .button(StdButton::Cancel, false)
                            .show()
                        {
                            if btn != StdButton::Retry {
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
        let pos = self.documents[self.current].cursor.offset;
        if let Some(offset) = self.goto_dlg.show(pos) {
            self.move_cursor(&Direction::Absolute(offset));
        }
    }

    /// Find position of the sequence.
    fn find(&mut self) {
        if self.search_dlg.show() {
            self.draw();
            self.find_next(self.search_dlg.backward);
        }
    }

    /// Find next/previous position of the sequence.
    fn find_next(&mut self, backward: bool) {
        if let Some(sequence) = self.search_dlg.get_sequence() {
            let mut progress = ProgressDlg::new("Searching...");
            let doc = &mut self.documents[self.current];
            if let Some(offset) =
                doc.file
                    .find(doc.cursor.offset, &sequence, backward, &mut progress)
            {
                self.move_cursor(&Direction::Absolute(offset));
            } else if !progress.canceled {
                self.draw();
                MessageBox::new("Search", DialogType::Error)
                    .center("Sequence not found!")
                    .button(StdButton::Ok, true)
                    .show();
            }
        } else {
            self.search_dlg.backward = backward;
            self.find();
        }
    }

    /// Fill range.
    fn fill(&mut self) {
        let doc = &mut self.documents[self.current];
        if let Some((range, pattern)) =
            FillDlg::show(doc.cursor.offset, doc.file.size, &self.pattern)
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
            MessageBox::new("Insert bytes", DialogType::Error)
                .center(&doc.file.path)
                .center("was modified.")
                .center("Please save or undo your changes first.")
                .button(StdButton::Ok, true)
                .show();
            return;
        }
        if let Some((mut offset, size, pattern)) = InsertDlg::show(doc.cursor.offset, &self.pattern)
        {
            self.pattern = pattern;
            if offset > doc.file.size {
                offset = doc.file.size;
            }
            self.draw();
            let mut progress = ProgressDlg::new("Write file...");
            let doc = &mut self.documents[self.current];
            if let Err(err) = doc.file.insert(offset, size, &self.pattern, &mut progress) {
                MessageBox::new("Error", DialogType::Error)
                    .center("Error writing file")
                    .center(&doc.file.path)
                    .center(&format!("{}", err))
                    .button(StdButton::Cancel, true)
                    .show();
            }
            doc.on_file_changed(offset + size as u64);
        }
    }

    /// Cut out range.
    fn cut(&mut self) {
        let doc = &self.documents[self.current];
        if doc.file.is_modified() {
            MessageBox::new("Cut range", DialogType::Error)
                .center(&doc.file.path)
                .center("was modified.")
                .center("Please save or undo your changes first.")
                .button(StdButton::Ok, true)
                .show();
            return;
        }
        if let Some(range) = CutDlg::show(doc.cursor.offset, doc.file.size) {
            self.draw();
            let mut progress = ProgressDlg::new("Write file...");
            let doc = &mut self.documents[self.current];
            if let Err(err) = doc.file.cut(&range, &mut progress) {
                MessageBox::new("Error", DialogType::Error)
                    .center("Error writing file")
                    .center(&doc.file.path)
                    .center(&format!("{}", err))
                    .button(StdButton::Cancel, true)
                    .show();
            }
            doc.on_file_changed(range.end);
        }
    }

    /// Setup via GUI.
    fn setup(&mut self) {
        if self.setup_dlg.show() {
            for doc in self.documents.iter_mut() {
                doc.view.fixed_width = self.setup_dlg.fixed_width;
                doc.view.ascii_table = self.setup_dlg.ascii_table;
                if doc.view.ascii_table.is_none() {
                    doc.cursor.set_place(Place::Hex);
                }
            }
            self.resize();
        }
    }

    /// Exit from editor.
    fn exit(&mut self) -> bool {
        for doc in self.documents.iter_mut() {
            if !doc.file.is_modified() {
                continue;
            }
            if let Some(btn) = MessageBox::new("Exit", DialogType::Error)
                .center(&doc.file.path)
                .center("was modified.")
                .center("Save before exit?")
                .button(StdButton::Yes, false)
                .button(StdButton::No, false)
                .button(StdButton::Cancel, true)
                .show()
            {
                match btn {
                    StdButton::Yes => {
                        if !Editor::save(doc) {
                            return false;
                        }
                    }
                    StdButton::Cancel => {
                        return false;
                    }
                    _ => {}
                }
            } else {
                return false;
            }
        }

        // save history
        let mut history = History::new();
        history.set_goto(&self.goto_dlg.history);
        history.set_search(&self.search_dlg.history);
        self.documents
            .iter()
            .for_each(|doc| history.add_filepos(&doc.file.path, doc.cursor.offset));
        history.save();

        true
    }

    /// Draw editor.
    fn draw(&self) {
        // draw documents
        self.documents.iter().for_each(|doc| doc.view.draw(doc));

        // draw key bar (bottom Fn line).
        let screen = Curses::get_screen();
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
        let doc = &self.documents[self.current];
        if let Some((mut x, y)) = doc
            .view
            .get_position(doc.cursor.offset, doc.cursor.place == Place::Hex)
        {
            if doc.cursor.half == HalfByte::Right {
                x += 1;
            }
            Curses::show_cursor(doc.view.window.x + x, doc.view.window.y + y);
        }
    }

    /// Screen resize handler.
    fn resize(&mut self) {
        Curses::clear_screen();
        let mut screen = Curses::get_screen();
        screen.height -= 1; // key bar

        let height = screen.height / self.documents.len();
        let last_index = self.documents.len() - 1;
        for (index, doc) in self.documents.iter_mut().enumerate() {
            let wnd = Window {
                x: screen.x,
                y: index * height,
                width: screen.width,
                height: if index != last_index {
                    height
                } else {
                    // enlarge last window to fit the screen
                    screen.height - height * last_index
                },
            };

            doc.resize(wnd);
        }
    }
}
