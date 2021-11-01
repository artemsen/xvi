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
        // calculate dialog width
        let mut buttons_width = 0;
        for (button, default) in self.buttons.iter() {
            if buttons_width != 0 {
                buttons_width += 1; // space between buttons
            }
            buttons_width += Button::std(*button, *default).text.len();
        }
        let mut width = buttons_width;
        for (line, _) in self.message.iter() {
            if width < line.len() {
                width = line.len();
            }
        }
        width += Dialog::PADDING_X * 2;

        // calculate dialog height
        let height = self.message.len() - 1 +
            Dialog::PADDING_Y * 2 +
            2 /* buttons with separator */;

        // construct dialog
        let mut dlg = Dialog::new(width, height, self.dtype, &self.message.first().unwrap().0);
        for (text, center) in self.message.iter().skip(1) {
            let widget = Text::new(text);
            if *center {
                dlg.add_center(dlg.last_line, text.len(), widget);
                dlg.last_line += 1;
            } else {
                dlg.add_next(widget);
            }
        }

        // buttons line
        let mut button_ids = BTreeMap::new();
        for &(button, default) in self.buttons.iter() {
            let btn = Button::std(button, default);
            let id = dlg.add_button(btn);
            button_ids.insert(id, button);
            if default {
                dlg.focus = id;
            }
        }

        if let Some(id) = dlg.run_simple() {
            return Some(*button_ids.get(&id).unwrap());
        }
        None
    }
}
