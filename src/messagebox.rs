// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::dialog::*;
use super::widget::*;
use std::collections::BTreeMap;

/// Message box dialog.
pub struct MessageBox {
    /// Message text: string and align flag.
    message: Vec<(String, bool)>,
    /// Buttons: type and default state.
    buttons: Vec<(StdButton, bool)>,
    /// Message type.
    dtype: DialogType,
}

impl MessageBox {
    /// Create new message box.
    pub fn new(title: &str, dtype: DialogType) -> Self {
        Self {
            message: vec![(String::from(title), true)],
            buttons: Vec::new(),
            dtype,
        }
    }

    /// Add text line (left aligned).
    pub fn left(&mut self, text: &str) -> &mut Self {
        self.message.push((String::from(text), false));
        self
    }

    /// Add text line (centered).
    pub fn center(&mut self, text: &str) -> &mut Self {
        self.message.push((String::from(text), true));
        self
    }

    /// Add one of the standard button.
    pub fn button(&mut self, button: StdButton, default: bool) -> &mut Self {
        self.buttons.push((button, default));
        self
    }

    /// Show message box dialog.
    pub fn show(&self) -> Option<StdButton> {
        let mut dlg = Dialog::new(self.dtype);

        // calculate buttons line width
        let mut buttons_width = 0;
        for (button, default) in self.buttons.iter() {
            if buttons_width != 0 {
                buttons_width += 1; // space between buttons
            }
            buttons_width += Button::std(*button, *default).text.len();
        }

        // calculate min width
        let mut width = buttons_width;
        for (line, _) in self.message.iter() {
            if width < line.len() {
                width = line.len();
            }
        }

        let mut y = 0;

        // border
        let height = self.message.len() - 1 + if self.buttons.is_empty() { 2 } else { 4 };
        dlg.add(0, 0, width + 4, height, Border::new(&self.message[0].0));

        // message text
        y += 1;
        for (line, center) in self.message.iter().skip(1) {
            let mut x = 2;
            if *center {
                x += (width - line.len()) / 2;
            }
            dlg.add(x, y, 0, 1, Text::new(line));
            y += 1;
        }

        // separator between message and buttons
        if !self.buttons.is_empty() {
            dlg.add(0, y, width + 4, 1, Separator::new(None));
            y += 1;
        }

        // buttons line
        let mut button_ids = BTreeMap::new();
        let mut x = 2 + width / 2 - buttons_width / 2;
        for (button, default) in self.buttons.iter() {
            let btn = Button::std(*button, *default);
            let width = btn.text.len();
            let id = dlg.add(x, y, width, 1, btn);
            button_ids.insert(id, *button);
            if *default {
                dlg.focus = id;
            }
            x += width + 1;
        }

        if let Some(id) = dlg.run() {
            return Some(*button_ids.get(&id).unwrap());
        }
        None
    }
}
