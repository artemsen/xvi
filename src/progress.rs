// SPDX-License-Identifier: MIT
// Copyright (C) 2021 Artem Senichev <artemsen@gmail.com>

use super::curses::{Curses, Event, Key};
use super::dialog::*;
use super::widget::*;

/// Progress dialog.
pub struct Progress {
    min: u64,
    max: u64,
    dlg: Dialog,
    did: ItemId,
}

impl Progress {
    /// Create new progress window.
    pub fn new(title: &str, min: u64, max: u64) -> Self {
        let mut dlg = Dialog::new(50, 5, DialogType::Normal, title);
        let did = dlg.add_next(ProgressBar::new());
        dlg.focus = dlg.add_button(Button::std(StdButton::Cancel, true));
        Self { min, max, dlg, did }
    }

    /// Show progress dialog.
    pub fn update(&mut self, value: u64) -> bool {
        debug_assert!(value >= self.min && value <= self.max);
        let percent = ((value - self.min) * 100)
            / if self.max == self.min {
                1
            } else {
                self.max - self.min
            };
        // update only if value was changed
        if let WidgetData::Number(curr) = self.dlg.get(self.did) {
            if curr != percent as usize {
                self.dlg.set(self.did, WidgetData::Number(percent as usize));
                self.dlg.draw();
                Curses::refresh_screen();
            }
        }
        // check for user interrupt
        if let Some(Event::KeyPress(key)) = Curses::peek_event() {
            return !matches!(key.key, Key::Esc | Key::Enter | Key::Char(' '));
        }
        true
    }
}
