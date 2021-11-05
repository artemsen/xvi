// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::super::curses::{Curses, Event, Key};
use super::super::file::ProgressHandler;
use super::dialog::{Dialog, DialogType, ItemId};
use super::widget::{Button, ProgressBar, StdButton, WidgetData};

/// Progress dialog.
pub struct ProgressDialog {
    dlg: Dialog,
    bar: ItemId,
    pub canceled: bool,
}

impl ProgressDialog {
    /// Create new progress window.
    pub fn new(title: &str) -> Self {
        let mut dlg = Dialog::new(50, 5, DialogType::Normal, title);
        let bar = dlg.add_next(ProgressBar::new());
        dlg.focus = dlg.add_button(Button::std(StdButton::Cancel, true));
        Self {
            dlg,
            bar,
            canceled: false,
        }
    }
}

impl ProgressHandler for ProgressDialog {
    fn update(&mut self, percent: u8) -> bool {
        debug_assert!(percent <= 100);

        // update only if value was changed
        if let WidgetData::Number(current) = self.dlg.get(self.bar) {
            if current != percent as usize {
                self.dlg.set(self.bar, WidgetData::Number(percent as usize));
                self.dlg.draw();
                Curses::refresh_screen();
            }
        }
        // check for user interrupt
        if let Some(Event::KeyPress(key)) = Curses::peek_event() {
            self.canceled = matches!(key.key, Key::Esc | Key::Enter | Key::Char(' '));
        }
        !self.canceled
    }
}
