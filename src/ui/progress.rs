// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::super::curses::{Curses, Event, Key};
use super::super::file::ProgressHandler;
use super::dialog::{Dialog, DialogType, ItemId};
use super::widget::{StandardButton, WidgetType};

/// Progress dialog.
pub struct ProgressDialog {
    dlg: Dialog,
    bar: ItemId,
    pub canceled: bool,
}

impl ProgressDialog {
    /// Create new progress window.
    pub fn new(title: &str) -> Self {
        let mut dlg = Dialog::new(50, 1, DialogType::Normal, title);
        let bar = dlg.add_line(WidgetType::ProgressBar(0));
        dlg.add_button(StandardButton::Cancel, true);

        let mut instance = Self {
            dlg,
            bar,
            canceled: false,
        };
        instance.update(0);

        instance
    }

    /// Hide progress window.
    pub fn hide(&self) {
        self.dlg.hide();
    }
}

impl ProgressHandler for ProgressDialog {
    fn update(&mut self, percent: u8) -> bool {
        debug_assert!(percent <= 100);

        if let WidgetType::ProgressBar(current) = self.dlg.get_widget_mut(self.bar) {
            *current = percent;
            self.dlg.draw();
        }

        // check for user interrupt
        if let Some(Event::KeyPress(key)) = Curses::peek_event() {
            self.canceled = matches!(key.key, Key::Esc | Key::Enter | Key::Char(' '));
        }
        !self.canceled
    }
}
